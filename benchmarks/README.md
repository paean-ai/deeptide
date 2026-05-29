# Benchmarks

## `cache_memory_bench.py` — DeepSeek prompt-cache alignment

Measures, against the **live DeepSeek API**, how memory/system-prompt layout
affects DeepSeek's automatic prefix cache (a cache hit is billed at ~10% of a
miss, so this is the dominant cost/latency lever for an agentic coding loop that
re-sends a large prefix every turn).

### Run

```sh
DEEPSEEK_API_KEY=sk-... python3 benchmarks/cache_memory_bench.py --turns 5
# pick the model; deepseek-reasoner is the "pro"/reasoning tier:
DEEPSEEK_API_KEY=sk-... python3 benchmarks/cache_memory_bench.py --model deepseek-reasoner
```

Cost is negligible (only the `usage` field is read).

> **Methodology — run isolation.** DeepSeek's cache is account-level and keyed
> on prefix *content*, so two runs with the same prefixes share cache: a second
> run hits the first run's warmed prefix and every layout looks ~equally cached,
> masking the real effect. Each run therefore mixes a unique `--salt` (random by
> default) into the stable prefix, forcing a cold start so only the *within-run*
> layout difference is measured. (Discovered the hard way: an early reasoner run
> without this showed a false "no difference" because it was hitting a prior
> chat run's cache.)

### What it compares

Three memory layouts over a simulated multi-turn agentic session:

| Layout | Description |
|--------|-------------|
| **NAIVE — volatile line first** | A per-turn-changing line (date / turn counter / token count) at the **front** of the system prompt. |
| **NAIVE — memory reordered each turn** | Memory entries first but re-ordered every turn (not byte-stable). |
| **ALIGNED — stable prefix, volatile last** | System + memory block byte-identical and **first**; volatile content pushed to the **end** of the request. |

Metric: `prompt_cache_hit_tokens / prompt_tokens` per turn (warm = turns 2+).

### Measured result (2026-05-29, 5 turns, fresh per-run salt)

Identical across the standard and the pro/reasoning tier — prefix caching is an
input-side mechanism, independent of which model generates the output:

| Layout | `deepseek-chat` | `deepseek-reasoner` (pro) |
|--------|----------------:|--------------------------:|
| NAIVE — volatile line first | **0.0%** | **0.0%** |
| NAIVE — memory reordered     | 67.6% | 67.6% |
| ALIGNED — stable prefix      | **95.5%** | **95.5%** |

**A single volatile token at the front of the prefix collapses cache-hit from
~95% to 0% — a ~10× input-cost regression**, on both tiers. Re-ordering the
memory block is less catastrophic (the stable section before it still caches)
but still leaves ~30 points on the table and is non-deterministic.

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
