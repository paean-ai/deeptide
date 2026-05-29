# Benchmarks

## `cache_memory_bench.py` — DeepSeek prompt-cache alignment

Measures, against the **live DeepSeek API**, how memory/system-prompt layout
affects DeepSeek's automatic prefix cache (a cache hit is billed at ~10% of a
miss, so this is the dominant cost/latency lever for an agentic coding loop that
re-sends a large prefix every turn).

### Run

```sh
DEEPSEEK_API_KEY=sk-... python3 benchmarks/cache_memory_bench.py --turns 5
```

Cost is negligible (`max_tokens=8`; only the `usage` field is read).

### What it compares

Three memory layouts over a simulated multi-turn agentic session:

| Layout | Description |
|--------|-------------|
| **NAIVE — volatile line first** | A per-turn-changing line (date / turn counter / token count) at the **front** of the system prompt. |
| **NAIVE — memory reordered each turn** | Memory entries first but re-ordered every turn (not byte-stable). |
| **ALIGNED — stable prefix, volatile last** | System + memory block byte-identical and **first**; volatile content pushed to the **end** of the request. |

Metric: `prompt_cache_hit_tokens / prompt_tokens` per turn (warm = turns 2+).

### Measured result (2026-05-29, deepseek-chat, 5 turns)

```
NAIVE (volatile line first)      warm hit rate:  0.0%
NAIVE (memory reordered)         warm hit rate: 67.9%
ALIGNED (stable prefix)          warm hit rate: 94.9%   (peaked 97.5% at turn 4)
```

**A single volatile token at the front of the prefix collapses cache-hit from
~95% to 0% — a ~10× input-cost regression.** Re-ordering the memory block is
less catastrophic (the stable section before it still caches) but still leaves
~30 points on the table and is non-deterministic.

### Design rules this establishes (enforced in code)

1. **No volatile content (date, clock, turn counter) in the system prompt.**
   Guarded by `prompt::tests::system_prompt_has_no_volatile_date_or_clock` and
   `…::system_prompt_is_deterministic`.
2. **The memory block must be byte-stable across turns**, and appending new
   entries must not perturb the retained head. Enforced by
   `truncate_memory` (line-aligned, prefix-stable cap) and its
   `memory::cache_alignment_tests`. Overflow is relocated to on-demand
   `MemorySearch`, not silently dropped (two-tier memory: cached core + retrieved
   long tail).
