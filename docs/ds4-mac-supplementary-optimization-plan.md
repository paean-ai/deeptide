# DS4 Mac Supplementary Optimization Plan

## Executive Summary

本文档是对 `ds4-mac-optimization-plan.md` 的补充改进计划，聚焦于原计划未覆盖但投入产出比高的方向。核心思路是在不改变单 Metal worker、C API、固定模型布局等架构约束的前提下，从 KV cache 压缩、前缀命中、内核预取、内存压力感知 4 个维度进一步降低 VM 压力、减少首 token 延迟方差、提升长对话场景下的 cache 命中率。

所有改进默认 opt-in，提供完整的环境变量逃逸通道，可独立验证和回滚。

## 补充优化方向

---

## 1. KV Cache 量化压缩 (P0)

### 动机

当前 KV cache 以模型推理精度（fp16/bf16）存储，对于 q2 模型跑 128k 长上下文：

- 单层 KV cache（含 sliding window + compressed KV）在 128k tokens 下可占用数 GB 的 unified memory
- Disk KV 持久化 payload 大小随之膨胀，导致 Section 3（异步持久化）的 snapshot copy 时间和 file write 时间仍然可观
- 量化 KV cache 可将内存占用降低 2-4x，同时减少 disk I/O 体积，与 Section 3/4 形成乘数效应

### 目标

- KV cache 运行时内存占用降低 40-60%
- Disk KV payload 大小降低对应比例
- Token 输出精度损失控制在 `perplexity delta < 0.5%` 或 `logprob cosine similarity > 0.995`

### 实现方案

#### 1.1 量化格式选择

首选 **fp8_e4m3**（M3/M4 GPU 原生支持），备选 int8 per-channel symmetric：

| 格式 | Metal 原生支持 | 精度损失 | 实现复杂度 |
|------|---------------|---------|-----------|
| fp8_e4m3 | M3+ 原生 | 极低 | 低（Metal 内置转换） |
| int8 per-channel | 所有 GPU | 低 | 中（需手动 scale/zero） |
| int8 per-token | 所有 GPU | 中 | 低 |

推荐路径：**先实现 int8 per-channel 作为基线**（兼容性最好），再添加 fp8_e4m3 加速路径（M3+ 专用）。

#### 1.2 量化时机与位置

```
[Attention Output] → [量化到 int8/fp8] → [写入 KV cache tensor] → [Disk持久化时直接读取已量化数据]
                                                      ↓
                                              [Decode时反量化回 fp16] → [Attention计算]
```

- **量化点**：KV 写入 cache 时（RMS norm + RoPE 之后）
- **反量化点**：从 cache 读取进行 attention 计算时
- **持久化**：直接保存量化后的 buffer，无需额外转换
- **不量化对象**：当前 step 的 active KV（仍在 fp16 计算）、logits、token upload buffer

#### 1.3 新增环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DS4_METAL_KV_QUANT=off` | `off` | `off` / `int8_pc` / `fp8_e4m3` |
| `DS4_METAL_KV_QUANT_SCALE_BITS=8` | `8` | per-channel scale 的位宽（8 或 16） |

#### 1.4 数据布局

```
// int8 per-channel 布局
// shape: [num_layers, 2, num_kv_heads, seq_len, head_dim]
// 每个 head_dim 维度: 128 个 int8 值 + 1 个 fp16 scale
// 物理存储: int8[N] + fp16*scale[N/128]
```

对于 sliding-window KV 和 compressed KV 分别量化，保持独立的 scale 参数。

#### 1.5 Metal Shader 改造

**新增 shader**：

- `quantize_kv_int8_pc`: KV 写入时量化，输入 fp16，输出 int8 + scale
- `dequantize_kv_int8_pc`: attention 前反量化，输入 int8 + scale，输出 fp16
- （可选）`quantize_kv_fp8_e4m3`: M3+ 原生 fp8 路径

**改动点**：

- 在现有的 fused KV finalization kernel 后插入量化 pass
- 在 indexed attention / sliding window attention 的 KV 读取阶段插入反量化 pass
- 改动范围：`native/ds4/metal/` 下的 KV 相关 shader

#### 1.6 Disk KV 兼容性

- 在 disk KV file header 中增加 2 字节的 `kv_quant_format` 字段（预留 `0x00` 为无量化，`0x01` 为 int8_pc，`0x02` 为 fp8_e4m3）
- 旧格式（无量化）文件可直接读取：header 默认值 `0x00`，DS4 自动识别
- 不同量化格式的 cache 文件自然不共享（SHA1 已包含完整 token IDs，量化格式不影响 key 计算）

#### 1.7 验证标准

1. `ds4_test --logprob-vectors` 输出与未量化版本的 cosine similarity ≥ 0.995
2. 128k 上下文 prefill 时 KV 内存占用降低 ≥ 40%
3. `continued` checkpoint 的 snapshot copy 时间降低 ≥ 30%
4. Decode 吞吐量损失 ≤ 5%（反量化开销可接受）

#### 1.8 回滚

- 设置 `DS4_METAL_KV_QUANT=off` 恢复原始行为
- 量化路径完全隔离在条件分支内，不影响现有 fast path

---

## 2. 前缀树 / 部分命中 KV Cache 查找 (P0)

### 动机

当前 disk KV 查找使用完整 token sequence 的 SHA1 精确匹配。对于 coding agent 场景：

- 系统提示词（数万 tokens）长期不变
- 每次请求在原对话上追加 100-500 个新 token
- 精确匹配下，只要追加了 1 个新 token，整个前缀 cache 就 miss

实际 cache hit 率远低于理论上限。

### 目标

- 部分前缀命中时，复用命中部分的 KV 并仅 prefill 剩余 tokens
- 在典型 coding agent 多轮对话中，cache hit 率从接近 0% 提升到 80%+
- 前缀查找开销不成为新的延迟瓶颈

### 实现方案

#### 2.1 索引结构

使用 **内存 radix tree（压缩前缀树）** 索引已落盘 KV 文件的 token 序列。

```
结构：
- 全局 radix tree，key = token sequence（uint32[]），value = cache file path + 元数据
- 每个节点存储：
  - token 值（uint32）
  - 对应 cache file path（若非空，则该节点对应一个完整 KV 缓存）
  - 子节点指针（最多 65536 个，但实际极度稀疏，用 hash map）
  - 该节点对应序列的 token 长度
```

#### 2.2 索引生命周期

```
启动时 → 扫描 KV cache 目录 → 对每个 cache file 读 header 获取 token IDs → 插入 radix tree
运行时 → 新 cache save 成功后，异步插入 radix tree（不阻塞 worker）
关闭时 → radix tree 不持久化（下次启动重建，扫描开销可接受）
```

扫描开销：假设 1000 个 cache 文件，每个 header 256 bytes，扫描 ~256KB，重建索引 < 10ms。

#### 2.3 查找流程

```
1. 输入: 请求的完整 token sequence T[0..N]
2. 在 radix tree 中查找最长匹配前缀 T[0..K]，其中 K ≤ N
3. 若 K ≥ DS4_METAL_RESUME_PREFILL_MIN 且 cache file 有效：
   a. 从 cache file 加载前缀 T[0..K] 的 KV 状态
   b. 仅 prefill T[K+1..N] 的增量部分（K+1 到 N）
   c. 合并 KV 状态
4. 否则：全量 prefill（现有行为）
```

#### 2.4 Cache Key 设计

- 保持 SHA1 精确匹配路径不变
- 新增 `partial_match` 路径仅在 radix tree 命中时触发
- Cache file 内部 token IDs 存储在 header 中，确保加载时做二次校验：
  - 从 radix tree 找到 candidate cache file
  - 加载 header 读取实际 token IDs
  - 逐 token 比对前缀，确认无误后才使用

#### 2.5 新增环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DS4_METAL_KV_PREFIX_INDEX=on` | `on` | 启用 radix tree 前缀索引（`off` 回退到纯 SHA1） |
| `DS4_METAL_PREFIX_INDEX_MAX_ENTRIES=4096` | `4096` | radix tree 最大节点数（0 = 无限制） |
| `DS4_METAL_RESUME_PREFILL_MIN=32` | `32` | 复用现有变量，最小前缀命中长度阈值 |

#### 2.6 实现文件

新增 `native/ds4/kv_prefix_index.h` 和 `native/ds4/kv_prefix_index.c`：

```c
// kv_prefix_index.h (精简 API)
typedef struct ds4_prefix_index ds4_prefix_index_t;

ds4_prefix_index_t* ds4_prefix_index_create(uint32_t max_entries);
void ds4_prefix_index_destroy(ds4_prefix_index_t* idx);

// 插入一个 KV 缓存对应的 token 序列
int ds4_prefix_index_insert(ds4_prefix_index_t* idx,
                            const uint32_t* tokens, uint32_t token_count,
                            const char* cache_path);

// 查找最长匹配前缀，返回匹配长度和对应的 cache path
int ds4_prefix_index_lookup(ds4_prefix_index_t* idx,
                            const uint32_t* tokens, uint32_t token_count,
                            uint32_t* matched_len, const char** cache_path);

// 移除指定 token 序列的索引条目（cache evict 时调用）
int ds4_prefix_index_remove(ds4_prefix_index_t* idx,
                            const uint32_t* tokens, uint32_t token_count);

// 启动时扫描 cache 目录并重建索引
int ds4_prefix_index_rebuild(ds4_prefix_index_t* idx, const char* cache_dir);
```

#### 2.7 验证标准

1. 使用 3 轮以上 coding agent 对话 trace，记录 cache hit/miss 日志
2. 前缀命中率 ≥ 80%（有意义的命中，即 K ≥ RESUME_PREFILL_MIN）
3. 索引重建时间 < 50ms（10000 个 cache 文件以内）
4. 前缀查找延迟 < 100μs
5. 全量 miss 时（无前缀匹配），行为与 `DS4_METAL_KV_PREFIX_INDEX=off` 完全一致

#### 2.8 回滚

- `DS4_METAL_KV_PREFIX_INDEX=off` 完全恢复到纯 SHA1 匹配

---

## 3. 模型权重分页预取 (P1)

### 动机

mmap + no-copy 设计避免了显式的全模型加载，但在首次 prefill 时，Metal 访问 mmap 映射的 GGUF tensor 会触发大量 cold page faults。每次缺页中断由 kernel 处理（从 NVMe 读取物理页），导致不可预测的延迟峰值，直接影响 first-request latency variance。

### 目标

- 首 token 延迟的 P99 降低 30-50%（消除缺页中断的尾延迟）
- 实现简单，不引入新依赖，不影响正确性

### 实现方案

#### 3.1 预取策略

在 warmup 阶段（`ds4_metal_warmup` 之后、首次 prefill 之前）按推理访问序列预取页面：

```
1. 遍历模型的所有 Metal buffer（mmap + Metal shared 映射的 GGUF tensor）
2. 对每个 buffer 按 warmup_stride 步长调用 madvise(MADV_WILLNEED)
3. 预取完成后调用 madvise(MADV_DONTNEED) 清理不参与推理的 trailing 页面
```

#### 3.2 实现细节

**预取函数**：

```c
// 对给定地址范围做分页预取
static void ds4_metal_prefetch_range(const void* addr, size_t len, size_t stride) {
    const size_t page_size = getpagesize();
    const uint8_t* base = (const uint8_t*)((uintptr_t)addr & ~(page_size - 1));
    size_t aligned_len = ((len + page_size - 1) / page_size) * page_size;

    // stride=0 表示预取所有页面
    if (stride == 0) {
        madvise((void*)base, aligned_len, MADV_WILLNEED);
        return;
    }

    // 按步长预取：每 stride 个页面预取 1 个
    size_t num_pages = aligned_len / page_size;
    for (size_t i = 0; i < num_pages; i += stride) {
        madvise((void*)(base + i * page_size), page_size, MADV_WILLNEED);
    }
}
```

**调用位置**：`ds4_metal_session_create` 中，在 residency set 和 warmup pass 之后，首次 `ds4_session_sync` 之前。

#### 3.3 步长策略

| Profile | Stride | 说明 |
|---------|--------|------|
| `mmax128` | 1 | 128GB 完全容纳 q2 模型，全量预取 |
| `auto` (≥ 64GB) | 1 | 内存充裕则全量预取 |
| `auto` (< 64GB) | 4 | 每 4 页预取 1 页，避免 VM 压力 |
| `off` | 0 | 不预取（0 = 跳过 madvise 调用） |

#### 3.4 新增环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DS4_METAL_PREFETCH_STRIDE=0` | `0` | 预取步长（0 = 不预取，1 = 全量，N = 每 N 页预取 1 页） |

Profile 系统内部会覆盖此值，用户直接设置该变量拥有最高优先级。

#### 3.5 日志

```
[ds4] metal prefetch: 89 buffers, 44.2 GiB mapped, stride=1, pages=11583488
[ds4] metal prefetch: done in 847 ms (52.2 GiB/s)
```

#### 3.6 验证标准

1. 首 token 延迟 P50/P99 在 cold start 下相较无预取版本有统计显著改善
2. `madvise` 调用不触发 Metal 错误或 MMU 异常
3. 小内存机器（< 32GB）上预取不导致 OOM 或 swap thrash
4. 设置 `DS4_METAL_PREFETCH_STRIDE=0` 行为与改动前完全一致

#### 3.7 回滚

- `DS4_METAL_PREFETCH_STRIDE=0` 跳过所有 madvise 调用

---

## 4. 内存压力感知与主动驱逐 (P1)

### 动机

原计划提及 "future memory pressure guardrails" 但未展开。macOS unified memory 是全局共享的——IDE、浏览器、编译器可能抢占内存。当 memory_pressure 进入 WARN 或 CRITICAL 状态时：

- Metal `newBuffer` / `newTexture` 可能返回 nil
- mmap 页面被 kernel 驱逐后，下次访问触发 swap-in 延迟
- 极端情况下 kernel_task 接管 GPU，导致 Metal 命令队列暂停

DS4 当前对此无感知，需要一个轻量级压力监控层。

### 目标

- 感知 macOS 全局内存压力状态
- 在压力升高时主动释放可重建资源，保障核心推理路径
- 压力解除后恢复默认行为

### 实现方案

#### 4.1 压力监控机制

macOS 提供了两层监控接口，优先使用 `sysctl`（无需链接额外 framework）：

```c
#include <sys/sysctl.h>

typedef enum {
    DS4_MEM_PRESSURE_NORMAL = 0,
    DS4_MEM_PRESSURE_WARN = 1,
    DS4_MEM_PRESSURE_CRITICAL = 2,
} ds4_mem_pressure_t;

static ds4_mem_pressure_t ds4_metal_get_memory_pressure(void) {
    // macOS 内存压力等级通过 vm.memory_pressure_level 暴露
    // 但该 sysctl 在用户态可能不可用；fallback 到可用内存比例判断
    int64_t page_size = getpagesize();
    int64_t free_pages = 0;
    size_t len = sizeof(free_pages);
    if (sysctlbyname("vm.page_free_count", &free_pages, &len, NULL, 0) == 0) {
        int64_t total_pages = 0;
        len = sizeof(total_pages);
        sysctlbyname("hw.memsize", &total_pages, &len, NULL, 0);
        total_pages /= page_size;

        double free_ratio = (double)free_pages / total_pages;
        if (free_ratio < 0.05) return DS4_MEM_PRESSURE_CRITICAL;
        if (free_ratio < 0.15) return DS4_MEM_PRESSURE_WARN;
    }
    return DS4_MEM_PRESSURE_NORMAL;
}
```

备选方案：使用 `dispatch_source` 监听 `DISPATCH_MEMORYPRESSURE` 事件（需要链接 libdispatch，DS4 已有 GCD 依赖则可直接使用）。

#### 4.2 分级响应策略

| 压力等级 | 响应动作 |
|---------|---------|
| `NORMAL` | 默认行为，无变化 |
| `WARN` | - 跳过 `continued` checkpoint（下一个 step 再补写）<br>- 将 warmup 步长加倍（减少预留页面）<br>- 日志记录 `[ds4] memory pressure: WARN` |
| `CRITICAL` | - 立即驱逐所有非活跃 disk KV 的 radix tree 条目<br>- 释放不参与当前 step 的 Metal scratch buffer<br>- 暂停 `continued` checkpoint<br>- 日志记录 `[ds4] memory pressure: CRITICAL, releasing resources` |

#### 4.3 检查频率与开销

- 在每次 `ds4_session_sync` 调用前做一次轻量级检查（仅 sysctl 调用，< 10μs）
- 不做定时器轮询（保持单 worker 确定性）
- 压力等级变化时才触发响应动作（避免重复释放）

#### 4.4 Scratch Buffer 可释放性标记

为 Metal 的 scratch/runtime buffer 增加元数据标记：

```c
typedef enum {
    DS4_BUF_PERSISTENT = 0,  // 不可释放（KV cache、模型权重映射）
    DS4_BUF_SCRATCH = 1,     // 可释放，按需重建
} ds4_buf_lifetime_t;
```

`CRITICAL` 时释放所有 `DS4_BUF_SCRATCH` buffer，下次使用时延迟重新分配。

#### 4.5 新增环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DS4_METAL_MEM_GUARD=on` | `on` | 启用内存压力感知 |
| `DS4_METAL_MEM_GUARD_WARN_THRESHOLD=0.15` | `0.15` | WARN 等级的可用内存比例阈值 |
| `DS4_METAL_MEM_GUARD_CRITICAL_THRESHOLD=0.05` | `0.05` | CRITICAL 等级阈值 |

#### 4.6 验证标准

1. 在内存压力正常的机器上，监控代码不引入可测量的性能开销
2. 模拟压力场景（例如用 `memory_pressure` 工具或并行分配大块内存），验证日志输出和资源释放行为
3. 压力解除后，后续推理正常运行，无泄漏或悬挂引用
4. `DS4_METAL_MEM_GUARD=off` 完全跳过监控逻辑

#### 4.7 回滚

- `DS4_METAL_MEM_GUARD=off` 关闭所有监控和响应

---

## 5. Disk KV 增量持久化 (P2)

### 动机

原计划 Section 3 将 `continued` save 的 file write 拆分到后台线程，解决 worker blocking 问题。但每次写入仍是完整 KV 快照。对于 128k 上下文的长对话，完整快照可能达数 GB，即使异步写入也占用大量 I/O 带宽。

增量持久化在异步化的基础之上，进一步减少实际写入的数据量。

### 目标

- `continued` checkpoint 的实际写入数据量降低 70-90%（只写增量）
- 不影响 `evict` 和 `shutdown` 的完整保存

### 实现方案

#### 5.1 文件布局

```
<sha1_of_full_tokens>.ds4kv/
├── base.kv      # 基础 KV 状态（前缀 tokens 的完整快照）
├── delta_N.kv   # 第 N 个增量子文件（最近 M 个 delta token 的 KV）
└── meta.json    # 元数据：各文件对应的 token 范围
```

`meta.json` 结构：

```json
{
  "version": 1,
  "token_count": 12345,
  "kv_quant_format": "int8_pc",
  "segments": [
    {"file": "base.kv", "token_offset": 0, "token_count": 12000},
    {"file": "delta_1.kv", "token_offset": 12000, "token_count": 345}
  ]
}
```

#### 5.2 写入策略

| 操作 | 行为 |
|------|------|
| 首次 save | 写入 `base.kv`（全量） |
| `continued` save | 仅写入新的 `delta_N.kv`，更新 `meta.json` |
| `continued` save（delta 累积超过阈值） | 合并 base + 所有 delta → 新 base，删除旧 delta |
| `evict` / `shutdown` save | 写入完整 `base.kv`，删除所有 delta |

合并阈值：当 delta token 数量超过 base token 数量的 25% 时触发合并。

#### 5.3 加载流程

```
1. 读取 meta.json
2. 按 token_offset 顺序加载所有 segments
3. 重建完整 KV 状态：base 全部加载 + 逐个 delta 追加
4. 校验加载后的 token 数量与 meta 中的 token_count 一致
```

#### 5.4 与 Section 3 异步化的关系

- Section 3 的异步文件写入仍适用：delta 文件的后台写入方式与 base 一致
- delta 写入本身就是小文件（通常覆盖几百 tokens），本身已很快
- 合并操作（compaction）较重，应在 agent 空闲期或 shutdown 时进行，或在后台线程中异步合并

#### 5.5 新增环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DS4_METAL_KV_DELTA=off` | `off` | 启用增量持久化 |
| `DS4_METAL_KV_DELTA_MERGE_RATIO=0.25` | `0.25` | delta/base token 比例触发合并 |

#### 5.6 验证标准

1. 连续 10 轮 `continued` save，每次写入数据量 ≤ base 的 30%
2. Delta 增量加载的正确性通过 `--logprob-vectors` 验证
3. 合并后的 base 与从原始全量 save 中恢复的 KV 状态一致
4. 旧格式（单文件）cache 文件仍可正常加载

#### 5.7 回滚

- `DS4_METAL_KV_DELTA=off` 恢复完整快照行为

---

## 6. Thermal Throttling 监控 (P2)

### 动机

MacBook Pro/Max 在长时间推理下会触发热限制，Metal GPU 频率下降，导致 decode 吞吐量骤降。目前 DS4 对此无感知，用户只能通过风扇噪音和吞吐量变化间接判断。

### 目标

- 日志中明确标注 throttling 事件
- 为未来的自适应降频策略提供基础设施

### 实现方案

#### 6.1 检测方式

macOS 不直接暴露 GPU 频率，但可以通过以下间接方式判断 throttling：

1. **IOKit 查询**：通过 `IOServiceGetMatchingService` + `IOPSCopyPowerSourcesInfo` 检查电源状态
2. **pmset 日志**：解析 `pmset -g thermlog` 输出（需要 root，不推荐）
3. **性能采样**：在 decode step 之间测量实际 Metal 命令耗时，若连续 N 个 step 的耗时超过历史中位数的 1.5x 则标记为疑似 throttling

推荐方案：**方案 3（性能采样）**——零权限要求，纯用户态实现。

#### 6.2 实现

```c
typedef struct {
    double decode_time_ms_history[64];  // 环形缓冲区
    uint32_t history_idx;
    uint32_t history_count;
    double median_decode_time_ms;
    bool throttling_suspected;
} ds4_thermal_monitor_t;

static void ds4_thermal_monitor_record(ds4_thermal_monitor_t* tm, double step_time_ms) {
    tm->decode_time_ms_history[tm->history_idx % 64] = step_time_ms;
    tm->history_idx++;
    if (tm->history_count < 64) tm->history_count++;

    // 仅当有足够样本时才判断
    if (tm->history_count < 16) return;

    // 计算中位数（简化：取排序后的中间值）
    double sorted[64];
    memcpy(sorted, tm->decode_time_ms_history, sizeof(sorted));
    qsort(sorted, tm->history_count, sizeof(double), compare_double);
    double median = sorted[tm->history_count / 2];

    // 连续 8 个 step > 1.5x 中位数 → 疑似 throttling
    int slow_steps = 0;
    for (int i = 0; i < 8 && (int)tm->history_idx - 1 - i >= 0; i++) {
        uint32_t idx = (tm->history_idx - 1 - i) % 64;
        if (tm->decode_time_ms_history[idx] > median * 1.5) slow_steps++;
    }
    bool new_throttling = (slow_steps >= 8);

    if (new_throttling && !tm->throttling_suspected) {
        ds4_log("[ds4] thermal: throttling suspected (decode %.1fms vs median %.1fms)",
                step_time_ms, median);
    } else if (!new_throttling && tm->throttling_suspected) {
        ds4_log("[ds4] thermal: throttling resolved (decode %.1fms → median %.1fms)",
                step_time_ms, median);
    }
    tm->throttling_suspected = new_throttling;
    tm->median_decode_time_ms = median;
}
```

#### 6.3 新增环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `DS4_METAL_THERMAL_MONITOR=on` | `on` | 启用 thermal 监控 |
| `DS4_METAL_THERMAL_THRESHOLD_MULTIPLIER=1.5` | `1.5` | 慢 step 检测倍数阈值 |

#### 6.4 验证标准

1. 在冷却良好的 Mac Studio 上不产生假阳性
2. 在 MacBook 上长时间运行后，当用户主观感受到降速时，日志中有对应的 throttling 标记
3. 监控逻辑开销 < 0.1% decode 时间

#### 6.5 未来扩展

- 检测到 throttling 后自动增大 chunk size（减少 CPU 端调度开销）
- 提示用户考虑降低 `DS4_METAL_PREFILL_CHUNK` 或切换 profile

#### 6.6 回滚

- `DS4_METAL_THERMAL_MONITOR=off` 关闭监控

---

## Implementation Roadmap

```
Phase 1 (立即)                    Phase 2 (Phase 1 完成后)        Phase 3 (异步化完成后)
────────────────────────────────  ─────────────────────────────  ─────────────────────────
1. KV Cache 量化 (int8_pc)        1. 前缀树索引                   1. Disk KV 增量持久化
   - 新增 quantize/dequant shader    - radix tree 实现              - 文件布局 + segment merge
   - 修改 KV write/read 路径        - 启动扫描 + 重建               - 与异步写入适配
   - 修改 disk KV header            - 与现有 SHA1 查找整合          - 加载路径合并
                                 2. 模型权重预取                2. PF 监控完善
2. 前缀树索引                        - madvise 预取函数               - IOKit 温度查询备选路径
   - 核心 radix tree 数据结构        - 与 profile 系统整合
   - 索引构建 + 查找
   - Cache 目录扫描
```

---

## Design Principles

与主计划一致：

- **保持 C API 兼容**：所有新增功能通过环境变量控制，不改变 `ds4_engine`、`ds4_session`、`ds4_session_sync()`、disk payload 接口的签名和语义
- **默认 opt-in**：新功能默认 `off`，经过 benchmark 验证后在 profile 中逐步开启
- **可独立验证**：每个方向有独立的验证标准和回滚开关，不影响其他优化
- **不引入外部依赖**：所有实现仅依赖 C 标准库 + Metal.framework + macOS 系统调用
- **不优化 CPU backend**：CPU 路径保持参考/调试角色
- **不引入 C++**：仅 C99 + Metal Shading Language

---

## Validation Summary

| 方向 | 核心验证指标 | 逃逸开关 |
|------|------------|---------|
| KV Cache 量化 | cosine similarity ≥ 0.995, KV 内存 -40% | `DS4_METAL_KV_QUANT=off` |
| 前缀树查找 | coding agent trace hit rate ≥ 80% | `DS4_METAL_KV_PREFIX_INDEX=off` |
| 权重预取 | P99 first-token latency 降低 30%+ | `DS4_METAL_PREFETCH_STRIDE=0` |
| 内存压力感知 | 压力场景资源释放行为正确，无泄漏 | `DS4_METAL_MEM_GUARD=off` |
| 增量持久化 | continued save 数据量 ≤ 30% of base | `DS4_METAL_KV_DELTA=off` |
| Thermal 监控 | 不产生假阳性，throttling 事件可观测 | `DS4_METAL_THERMAL_MONITOR=off` |

---

## References

- 主改进计划：`docs/ds4-mac-optimization-plan.md`
- DS4 源码：`native/ds4/`
- Metal shader：`native/ds4/metal/`
- DS4 Server：`native/ds4/server/`
