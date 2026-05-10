# DS4 Mac Optimization Plan

## Executive Summary

DS4 is already a narrow, DeepSeek V4 Flash-specific inference engine with a strong Apple Silicon focus. Its main performance strategy is to avoid general-purpose runtime overhead: it validates one fixed model layout, runs the production path as a Metal graph, keeps activations and KV state tensor-resident, maps GGUF weights into Metal without copying the model, and uses disk KV checkpoints to make long stateless agent sessions practical.

This plan focuses on further Mac-specific optimization for M3/M4 Max machines with 128 GB of unified memory, q2 weights, and local coding-agent workloads. The priorities are lower VM pressure, lower first-request latency variance, more predictable long-context prefill, and less blocking from disk KV persistence. The plan does not target the CPU backend, generic GGUF support, or multi-session parallel inference.

## Current DS4 Optimizations

- Fixed DeepSeek V4 Flash model shape: DS4 binds the expected tensor layout, attention geometry, MoE routing, compressed KV/indexer behavior, quantization mix, and vocabulary shape instead of acting as a generic GGUF runner.
- Whole-model Metal execution: production inference runs through the Metal graph path, with chunked prefill for long prompts and decode steps that update the live graph/KV state.
- No-copy model mapping: GGUF tensor data is mmap-backed and wrapped as overlapping shared Metal buffers, avoiding eager full-model copies while respecting Metal buffer length limits.
- Tensor-resident runtime state: activations, scratch buffers, raw sliding-window KV, compressed KV, indexer cache, and frontier state live in Metal tensors across command batches.
- DS4-specific fused kernels: the Metal path already includes fused Q/KV RMS norm, fused KV finalization, HC split/sum/expand helpers, routed-MoE Swiglu paths, shared expert fusion, and specialized indexed attention paths.
- Chunked long-context prefill: normal prompts can run whole-batch, while longer prompts default to clean 2048-token chunks that align with compression ratios and keep graph work bounded.
- Disk KV cache: the server persists reusable session payloads keyed by SHA1 of exact token IDs, allowing stateless clients that resend long conversations to resume from saved prefixes.
- Single Metal worker: the server serializes inference through one worker-owned session, keeping graph mutation and live KV reuse simple and deterministic.

## Optimization Priorities

### 1. Add a Mac Benchmark Matrix

Create a repeatable benchmark script, for example `native/ds4/tools/bench_mac.sh`, that runs existing binaries without changing inference behavior. The script should record:

- Short prompt prefill and decode throughput.
- Long prompt prefill and decode throughput, including around 12k-token prompts from existing test vectors where possible.
- Disk KV miss, cold save, disk KV hit, and continued checkpoint timings.
- First-request warmup time, Metal residency/warmup time, process RSS/VM size, and generated token throughput.
- Results for `DS4_METAL_PREFILL_CHUNK` values such as `1024`, `1536`, `2048`, and `3072`.
- Results for `DS4_METAL_RESUME_PREFILL_MIN` values such as `16`, `32`, `64`, and `128`.

The benchmark output should be plain text or JSONL so future changes can be compared without additional tooling.

### 2. Tune Mac Profile Defaults

Use benchmark results to add an optional Mac device profile in the runtime. The profile should choose conservative defaults for M3/M4 Max 128 GB machines while preserving existing environment overrides.

Recommended interface:

- `DS4_METAL_DEVICE_PROFILE=auto|mmax128|ultra|off`
- `auto` detects the host class when feasible and falls back to existing defaults when detection is uncertain.
- `off` preserves the current behavior exactly.

Candidate knobs:

- Prefill chunk size.
- Resume-prefill crossover threshold.
- Model warmup stride.
- Any future memory pressure guardrails.

Startup logs should print the selected profile and effective values.

### 3. Reduce Disk KV Persistence Blocking

Current disk KV persistence copies Metal tensor payloads through fixed-size CPU buffers and writes synchronously from the worker path. This is simple and avoids more mmap pressure, but continued checkpoints can still block the only inference worker.

Keep the non-mmap cache design, but split lower-risk persistence work:

- Keep `evict` and `shutdown` saves synchronous for durability.
- For `continued` saves, copy the exact snapshot payload into bounded host chunks and hand file writing to a background writer.
- Ensure the background writer never mutates the live session and never observes graph state directly after the snapshot boundary.
- Keep existing atomic temp-file-plus-rename behavior.
- Add logs for snapshot copy time, file write time, total save time, and skipped saves when a previous background save is still active.

This should reduce user-visible decode or prefill stalls during long agent sessions without changing the cache file format.

### 4. Evaluate Private Metal Runtime Tensors

Most runtime tensors are allocated as `MTLResourceStorageModeShared`, which simplifies CPU reads/writes for logits, debugging, and payload serialization. On Apple Silicon this is legal and practical, but some tensors that are never directly CPU-read may benefit from `MTLResourceStorageModePrivate`.

Add an experimental opt-in flag:

- `DS4_METAL_PRIVATE_TENSORS=1`

Initial scope should be conservative:

- Keep logits, token upload buffers, disk payload staging, and any tensor exposed through `ds4_metal_tensor_contents()` in shared storage.
- Move only clearly GPU-only scratch/runtime tensors to private storage.
- Use explicit blit or staging only where a CPU read/write is required.

This must remain disabled by default until benchmark data shows a clear win and correctness tests pass.

### 5. Refine Startup Warmup and Residency Strategy

DS4 already uses Metal residency sets on macOS 15+ and a coarse model-view warmup pass to reduce first-use stalls. For 128 GB machines, startup behavior should balance lower first-token variance against avoiding excessive page pressure.

Recommended changes:

- Make warmup stride profile-aware, with a conservative default for `mmax128`.
- Keep `DS4_METAL_NO_RESIDENCY` and `DS4_METAL_NO_MODEL_WARMUP` as full escape hatches.
- Log model view count, mapped GiB, residency time, warmup stride, warmup time, and any failure reason.
- Avoid dense full-model prefetch by default on 128 GB machines.

## Implementation Notes

- Preserve the public C API: `ds4_engine`, `ds4_session`, `ds4_session_sync()`, and disk payload loading/saving should remain source-compatible.
- Do not change model semantics or add permanent semantic variants behind flags.
- Treat official-vector correctness as mandatory before accepting a speedup.
- Keep risky optimizations opt-in until measured on target hardware.
- Do not optimize the CPU backend for Mac production use; it remains a reference/debug path.
- Do not introduce C++ or external runtime dependencies.

## Validation Plan

Required checks:

- `make -C native/ds4`
- `make -C native/ds4 test`
- `./native/ds4/ds4_test --logprob-vectors` when model files and vectors are available.

Mac benchmark acceptance criteria:

- Default decode throughput must not regress versus the current baseline.
- Long-prompt prefill should improve or remain within 3% of the current baseline.
- Disk KV hit should reduce time-to-first-token versus a cold prefill.
- Continued checkpoint saves should not introduce visible long stalls in generation progress logs.
- Any private-tensor experiment must pass at least short-vector and long-vector correctness checks before being considered for default use.

Rollback requirements:

- Each optimization must have an environment-variable escape hatch or be limited to benchmark tooling.
- Disabling new flags should restore current behavior.

## Assumptions

- Primary target hardware is M3/M4 Max with 128 GB unified memory.
- Primary model is the q2 DeepSeek V4 Flash GGUF distributed for DS4.
- Primary workload is a local coding agent that repeatedly sends long, mostly shared prompts to `ds4-server`.
- The server continues to use one live Metal session and one inference worker.
- The objective is practical local reliability and latency stability, not maximum multi-user throughput.
