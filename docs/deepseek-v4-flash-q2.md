# DeepSeek V4 Flash (Q2): Capability Limits and Usage Guidance

## TL;DR

The local **DeepSeek V4 Flash Q2** build that ships in DS4 / Deeptide local mode is a **technology-feasibility preview**, not a production coding model. It exists to prove that V4-class architectures (DSA attention, indexed top-k, compressed KV, MoE routing) can run end-to-end on a single Apple Silicon machine — and that goal it accomplishes. It is **not** a faithful approximation of cloud V4's behavior, and trying to use it as one will produce a long string of small disappointments before you give up.

If you want the model to actually help you finish work, use one of:

- **Cloud DeepSeek V4 / V4-Pro** (API, full precision, 1M context) — for general agent work.
- **Local V4 Flash Q4_K mixed-precision** (from [`opensota/deepseek-v4-gguf`](https://huggingface.co/opensota/deepseek-v4-gguf), file `DeepSeek-V4-Flash-Q4KExperts-F16HC-F16Compressor-F16Indexer-Q8Attn-Q8Shared-Q8Out-chat-v2.gguf`, ~153 GiB) — the in-architecture upgrade. Needs a host with the memory headroom (M3/M4 Ultra-class 192 GB+; will **not** fit on a 128 GB M3/M4 Max).
- **A local non-V4 Q4-class GGUF** such as Qwen3-Coder-Next UD-Q4_K_XL or GLM-4.7-Flash UD-Q6_K_XL — for offline coding agents on the same 128 GB envelope where the V4 Q4_K build can't load.

Use V4 Flash Q2 when the *thing you are validating* is "does the local stack work at all" — never when the thing you need is "a correct answer."

## Why Q2 specifically

Q2 represents each weight with only **4 quantization levels** (2 bits, so values in {−1.5σ, −0.5σ, +0.5σ, +1.5σ} after the per-block scale). That is a brutal compression ratio. It works surprisingly well on lightly-loaded paths — token embeddings, MLP layers with broad activation distributions — but it falls off a cliff on three specific paths that V4 *relies on heavily*:

| Path | What Q2 breaks |
|------|----------------|
| Attention output projection (`Wo`) | Quantization noise stacks linearly with sequence length. Attention scores become coarse — fine-grained "this token attends to that token" routing drops to "this token attends to roughly this region." Past ~32k tokens, perplexity rises visibly and recall on long-range references collapses. |
| DSA indexer top-k selection | The indexer learns a low-rank projection to score "which past tokens matter for this query." With 4 levels of weight resolution, the score distribution flattens and the top-k cutoff stops being meaningful. The model still returns *some* tokens, just often the wrong ones. |
| RoPE-dependent positional encoding | Position-modulated attention logits need at least 3-bit resolution to keep the sin/cos rotation phase information intact. At Q2 the rotation gets aliased; relative positions past a few thousand tokens become noisy. |

The cumulative effect: V4 Flash Q2 acts like a *much smaller* model on long contexts. It is not "the same model, slightly worse." It is qualitatively different.

By contrast:

- **Q3 (3-bit)** preserves most of the attention path's resolution. Long-context perplexity is acceptable up to roughly 100k tokens.
- **Q4_K_XL** (effectively 4.5–5 bits with unsloth-style mixed-precision groups) is the conventional "no apologies needed" floor for local serving. Outliers in activations are preserved; attention scores stay well-shaped well past 128k. This is the regime the Qwen3-Coder-Next and GLM-4.7-Flash profiles target.

## What this means for Deeptide local mode

Deeptide's local profile for V4 Flash is intentionally tuned to play *within* Q2's strengths:

1. **64k context cap.** The profile reports `contextWindow = 64_000` in both `LocalAgentPolicy` (the local-mode profile) and `ModelContextWindow.forModel` (the cloud-routing compaction lookup). Both must stay aligned — earlier versions of `ModelContextWindow` used a loose `contains("deepseek-v4")` substring match that incorrectly returned the cloud-V4 1M window for the local Flash build, which let the agent loop think it had headroom it does not.

2. **No subagent fanout, serial tool execution.** `InferenceProfile.local(...)` sets `disablesSubagents: true` and `serialToolExecution: true`. Parallel tool calls let Q2 fall behind quickly because tool-result blocks compound state across turns.

3. **Compact prompts, no Tide banners in system messages.** The system-prompt section for the V4 Flash profile is shorter than the qwen / glm sections — every kilotoken of system prompt reduces the budget for the actual task.

The other local profiles do not share these caps because they do not share Q2's quality cliff:

| Profile | Context | Why |
|---------|---------|-----|
| `deepseek-v4-flash` (Q2 dsedge) | **64k** | Q2 attention quality cliff |
| `deepseek-v4-flash-q4k` (Q4_K mixed-precision dsedge) | **1M** | Q4 experts + F16 indexer/HC/compressor preserves long-range attention |
| `qwen3-coder-next` (Q4_K_XL) | 131_072 | Q4 holds at 128k; this is the native window |
| `glm-4.7-flash` (Q6_K_XL) | 131_072 | Q6 has more headroom than the context needs |
| `qwen3.6-35b-a3b` (Q4_K_M) | 131_072 | Native window |
| Cloud `deepseek-v4*` (API) | 1_000_000 | Server-side; full precision |

If you are tempted to raise the V4 Flash context window past 64k, re-run the local-agent benchmark suite under `benchmarks/local-agent/` with prompts spanning 32k / 64k / 96k tokens **first**. The 96k bucket is where the regression usually appears, and it is not always visible in a single-shot perplexity check — it shows up as the agent hallucinating file paths, forgetting tool results from 20 turns ago, or producing diffs that no longer match the file it just read.

## The Q4_K mixed-precision local build

The `opensota/deepseek-v4-gguf` repository on Hugging Face publishes a higher-fidelity local V4 build that solves the Q2 quality cliff at the cost of a much larger memory footprint. The file is:

```
DeepSeek-V4-Flash-Q4KExperts-F16HC-F16Compressor-F16Indexer-Q8Attn-Q8Shared-Q8Out-chat-v2.gguf
```

The naming encodes a per-tensor-group quantization plan:

| Path | Precision | Why |
|------|-----------|-----|
| MoE experts | **Q4_K** | Experts dominate model size; Q4_K is the standard "no apologies" floor for MoE weights |
| Head-channel projection (`HC`) | **F16** | Tiny in absolute terms; quantization here directly aliases positional encoding, so we keep it full-precision |
| Compressed-KV projection | **F16** | Compressed KV is the only KV path past the sliding window — any precision loss here compounds across the entire long context |
| DSA indexer | **F16** | The top-k routing path. Q2 broke this; F16 makes it boringly correct |
| Attention `Q`/`K`/`V`/`Wo` | **Q8** | Q8 attention preserves head separation; Q8_0 dot-products are well-behaved on Metal |
| Shared expert | **Q8** | Hit on every token; F16 here would balloon size with no measurable quality gain over Q8 |
| Output head | **Q8** | Final projection — keep enough precision that the top-1 logit is stable |

Resident size is about **153 GiB**. The deeptide local profile loads this as `deepseek-v4-flash-q4k` (aliases: `flash-q4`, `flash-q4k`, `v4-flash-q4`, `v4-flash-q4k`). The profile's context window is set to **1M tokens**, matching the cloud V4 figure — the Q4_K + F16 indexer combination preserves long-range attention well enough that the architecture's native 1M window is actually usable, not aspirational.

### Who should use it

- Hosts with 192 GB+ unified memory (M3 Ultra / M4 Ultra-class). It will not fit on a 128 GB Max — `mmap` may succeed but the working set is too large to keep resident, and you will spend the entire session swapping.
- Workloads that actually need a long context (large-repo onboarding, multi-hour agent sessions, code review across a big diff). On short contexts the Q2 build is faster and the quality gap is smaller; the Q4_K build pays off where Q2's long-range attention falls down.
- Operators who want to keep all V4-specific architecture quirks (DSA, indexer, compressed KV) in their evaluation — switching to Qwen3-Coder-Next or GLM-4.7-Flash gives you Q4 quality but on a different architecture.

### Who should not

- Anyone on 128 GB or less. The Qwen Q4_K_XL or GLM Q6_K_XL profiles are the right local choice — they fit, hold 128k cleanly, and don't require V4-specific server code.
- Anyone where the bottleneck is decode latency. Q4_K experts decode faster than F16 experts but slower than Q2 experts; if you measured Q2 as "fast enough but wrong," Q4_K will be "slower and correct," not "as fast and correct."

### How to enable

1. Confirm the host has the memory headroom. `vm_stat | head -10` should show free + inactive comfortably above 160 GB before launch.
2. Download the GGUF from `opensota/deepseek-v4-gguf` to `~/Zero/models/deepseek-v4-flash-q4k/` (the path the bundled profile expects), or override with `deeptide config set local.model_path /your/path.gguf`.
3. Start the local server with the Q4_K alias: `deeptide local start --model flash-q4 --ctx 1000000`.
4. Run agent sessions normally — they pick up the new profile via `deeptide --local --model flash-q4`.

If you cannot fit the file, do not lower its context window as a workaround. The bottleneck is resident weight bytes, not KV cache; a smaller context will still fail to keep the model in memory. Use a different profile instead.

## When Q2 is actually fine

- Smoke-testing the local serving stack (DSEdge boot, Metal graph compile, KV-disk persistence).
- Walking through the agent loop on a small, well-scoped task (one or two files, a few hundred lines).
- Reproducing a server-side issue locally without consuming cloud credits.
- Demos that prioritize "this runs on my laptop" over "this answer is right."
- Anything where you are going to verify every line of output anyway.

## When Q2 will let you down

- Tasks that need to hold more than ~6–8 files in working context.
- Long-running agent sessions with many tool calls (Q2 KV degrades faster than Q4 KV).
- Anything requiring exact recall of a function signature or interface defined earlier in the session.
- Code review across a large diff.
- Tasks where a small inaccuracy compounds (refactors, migrations, API rename).

For any of the above: switch to cloud V4 (`deeptide --provider deepseek-cloud`), or to one of the Q4-class local profiles (`deeptide local --model qwen3-coder-next` or `--model glm-4.7-flash`).

## Implementation notes for maintainers

- `Sources/AgentLoop/LocalAgentPolicy.swift` is the source of truth for per-model local context windows. The Q2 default (`defaultContextWindow = 64_000`) and the Q4_K profile (`contextWindow: 1_000_000`) are deliberately separate constants — do not collapse them.
- `Sources/AgentLoop/CompactionManager.swift::ModelContextWindow.forModel` checks model ids in this order: `deepseek-v4-flash-q4` (1M) → `deepseek-v4-flash` (64k) → `deepseek-v4` (1M, cloud). The ordering matters because `String.contains` is a substring match, so a more specific id has to appear first. If you add another local V4 variant, slot it in *before* the bare `deepseek-v4-flash` check, not after.
- `Sources/Configuration/ModelAlias.swift` short aliases (`flash`, `fast`, `v4-flash` → Q2 build; `flash-q4`, `flash-q4k`, `v4-flash-q4`, `v4-flash-q4k` → Q4_K build) all currently resolve to the *local* canonical id. If you ever add a cloud Flash SKU, make sure these aliases do not silently route cloud traffic to a 64k-capped context.
- The pricing entry in `Sources/Configuration/CostTracker.swift` for `deepseek-v4-flash` is independent of the context cap and reflects the actual local cost (zero); leave it alone. The Q4_K id has no pricing entry, which is fine — local runs do not charge.
