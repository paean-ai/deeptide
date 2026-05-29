# Benchmarks

## `auto_capture_bench.py` — automatic memory capture quality + cost

Auto-capture (`src/memory_capture.rs`) asks the model, at session end, to
extract durable facts worth remembering — so the user doesn't have to
`/remember` everything by hand. It's only worth running every session if it's
accurate *and* cheap. This benchmark plants durable facts among ephemeral
distractors across synthetic sessions and measures recall, false-positive rate,
and **real ¥ cost/session** against the live API.

```sh
DEEPSEEK_API_KEY=sk-... python3 benchmarks/auto_capture_bench.py
```

### Measured result (2026-05-29, deepseek-v4-flash, 3 sessions)

```
durable-fact recall : 7/7 = 100%
false-positive rate : 0/5 = 0%      (skipped "fix line 42", "restart staging", "2+2", "token expires today")
tokens              : in=649  out=530
est. cost           : ¥0.0010 / session
VERDICT: GOOD — accurate and cheap enough to run every session
```

Token counts are authoritative (from the API `usage` field); the ¥ figure uses
listed flash pricing. ~¥0.001/session is far below the ¥0.01 dashboard display
resolution, so it won't move the balance visibly — but it's real and trivially
affordable. Auto-capture runs on the cheap `deepseek-v4-flash` tier by design.

---


## `cache_memory_bench.py` — DeepSeek prompt-cache alignment

Measures, against the **live DeepSeek API**, how memory/system-prompt layout
affects DeepSeek's automatic prefix cache (a cache hit is billed at ~10% of a
miss, so this is the dominant cost/latency lever for an agentic coding loop that
re-sends a large prefix every turn).

### Run

```sh
DEEPSEEK_API_KEY=sk-... python3 benchmarks/cache_memory_bench.py --turns 5
# the account's real model ids (verified via GET /models):
DEEPSEEK_API_KEY=sk-... python3 benchmarks/cache_memory_bench.py --model deepseek-v4-flash
DEEPSEEK_API_KEY=sk-... python3 benchmarks/cache_memory_bench.py --model deepseek-v4-pro
```

> **Model ids.** `GET /models` returns `deepseek-v4-flash` and `deepseek-v4-pro`.
> `deepseek-chat` / `deepseek-reasoner` are legacy aliases that bill as
> `deepseek-v4-flash` — so calling them does **not** exercise the pro tier. Use
> `deepseek-v4-pro` for a real pro-tier run.

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

| Layout | `deepseek-v4-flash` | `deepseek-v4-pro` |
|--------|--------------------:|------------------:|
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
