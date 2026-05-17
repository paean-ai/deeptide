# DeepSeek V4 Flash IQ2_XXS Asymmetric Quant: A Dialectical Reading

## The artifact

```
DeepSeek-V4-Flash-IQ2XXS-w2Q2K-AProjQ8-SExpQ8-OutQ8-chat-v2.gguf    80.8 GiB
```

The file name encodes the quantization plan:

| Component | Precision | Notes |
|-----------|-----------|-------|
| MoE expert `gate` / `up` (`w1`, `w3`) | **IQ2_XXS** | The two FFN-input projections per expert |
| MoE expert `down` (`w2`) | **Q2_K** | The FFN-output projection per expert — k-quant, not plain 2-bit |
| Attention projections (`q_proj`, `k_proj`, `v_proj`, `o_proj`) | **Q8_0** | All four heads-collapsed matrices |
| Shared expert | **Q8_0** | The dense MoE bypass |
| Output head (`lm_head`) | **Q8_0** | Final token logits |
| Router (gating) | **F16** | The expert-selection MLP |
| Token embeddings | **F16** | Input lookup |
| DSA indexer | **F16** | Top-k routing for compressed attention |
| KV compressor | **F16** | The low-rank KV projection |
| Head-channel projection (`HC`) | **F16** | Position-modulated head input |
| LayerNorm weights / attention sinks / biases | **F32** | Tiny in bytes, large in numerical impact |

At 80.8 GiB the file fits comfortably on a 128 GB M3/M4 Max with room left over for runtime KV state, which is the entire point of the design. It is also the size sweet spot for a single 80 GB H100, or 2× consumer-grade 48 GB cards — three different deployment shapes the author clearly had in mind.

## What the design gets right

### Precision-budget allocation maps to actual sensitivity

Years of quant ablations have established a rough sensitivity ranking inside transformer-MoE blocks:

1. **Norms, sinks, biases** (tiny, dominate numerical stability) — *must* be high precision.
2. **Router** (mis-routing is unrecoverable; the wrong expert outputs are just noise the model can't fix) — *must* be high precision.
3. **Embeddings / output head** (every token passes through; output head sees the full vocab competition) — high precision pays off.
4. **Attention projections** (long-range dependency fidelity; Q/K dot products are the most precision-sensitive op in the network) — medium-high precision.
5. **Shared expert** (touched by every token in MoE; degradation is uniform across the input distribution) — medium-high precision.
6. **DSA indexer / KV compressor / HC** (these *are* the long-context machinery — degrade them and you degrade attention itself at scale) — high precision is cheap, do it.
7. **MoE expert FFN matrices** (each individual expert is sparsely activated; errors average across the routed mixture) — *here* is where you can finally compress aggressively.

The file's precision assignments match this ranking item for item. That isn't accidental — it's the result of someone who has actually read the V4 architecture paper and thought about which weights carry information density vs which weights carry redundancy.

### The asymmetric expert split (IQ2_XXS gate/up vs Q2_K down) is well-targeted

Inside one expert FFN, `gate` and `up` operate on the same input vector and their outputs go through a SwiGLU non-linearity before hitting `down`. The non-linearity is *forgiving*: small errors in `gate` get gated by the sigmoid, small errors in `up` get masked when `gate` happens to be near zero. The errors that survive into `down` are already filtered.

`down`, by contrast, projects the SwiGLU output back into the residual stream — anything wrong here gets *added* into the model's running state and propagates. Giving it Q2_K (which preserves k-quant outlier handling) while letting `gate`/`up` take the IQ2_XXS hit is asymmetric in the right direction.

### F16 on the long-context machinery is a load-bearing choice

DSA (DeepSeek Sparse Attention) on V4 separates the attention path into a low-rank compressor + an indexer that picks top-k tokens for each query. Two precision points dominate behaviour past ~64k tokens:

- **The indexer.** If its top-k scores are quantization-noisy, the attention sparsity pattern becomes random. Past-context recall collapses long before perplexity moves visibly.
- **The compressor.** The compressed KV is the *only* path back to tokens outside the sliding window. Quantization noise here leaks into every long-range dependency.

The author kept both at F16. They are also small in absolute size (compared to the experts), so the cost is low. This is the move that separates a competent V4 quant from one that nominally fits but practically doesn't work at long context.

### F32 norms / sinks are free correctness

LayerNorm weights, attention sinks, and biases occupy a trivial number of bytes but their numerical behaviour is highly non-linear — a `RMSNorm` weight quantized to a poor representation can blow up the activation scale unpredictably. F32 here costs roughly nothing and removes an entire class of subtle numerical failures.

### The 80 GiB target is engineering, not a coincidence

There are exactly three commodity inference shapes this size lands cleanly on:

- A 128 GB M3/M4 Max with the weights resident *and* runtime headroom.
- A single 80 GB H100 SXM (with some swapping for KV state).
- 2× 48 GB consumer GPUs (RTX 6000 Ada, RTX A6000, L40S) under tensor parallelism.

Fall to ~60 GiB (everything Q2) and you save deployment money but the experts collapse. Push to ~120 GiB (everything Q4) and you fall off the back of every shape above. 80 GiB is a deliberately-aimed point in the size/quality plane, not a random outcome.

## Where the design takes real risk

A dialectical reading has to argue back. The same configuration has weak points the file name cannot advertise.

### IQ2_XXS gate/up is the weakest link, and it's load-bearing

The argument that "SwiGLU filters errors so 2-bit gate/up is fine" works on average. It breaks on the long tail. Specific failure modes that have been observed in 2-bit MoE experts:

- **Dead activation patches.** Some input regions produce gate values near zero across the entire vocabulary; experts in those regions become functionally inactive. With Q4 the failure is subtle, with IQ2_XXS it can be sharp.
- **Knowledge blind spots.** Facts that happen to be encoded in low-importance singular directions of `gate`/`up` (which IQ2_XXS will discard) become unrecoverable. Perplexity barely moves; specific factual recall drops.
- **iMatrix calibration dependence.** IQ2_XXS requires an importance matrix derived from a calibration corpus. If that corpus is English-heavy and the deployment is Chinese-coding-heavy (or vice versa), the per-channel scales are systematically off. The file name does not record which iMatrix was used.

These failure modes are *invisible to standard benchmarks*. They show up as "the model is great on benchmarks but weirdly bad at my specific workflow."

### Router-expert precision mismatch is a known anti-pattern

Keeping the router at F16 while crushing the experts to IQ2_XXS / Q2_K creates a structural contradiction: the router has the precision to make fine-grained routing decisions across a large expert mixture, but the experts no longer differ from each other in fine-grained ways. The router learns to assign tokens to a "best" expert that, post-quantization, isn't meaningfully better than its neighbours.

The pathological case is **expert collapse under quantization**: a few experts that happened to remain well-conditioned after IQ2_XXS hog the routing distribution, and the remaining (now degraded) experts become dead. The model nominally has 256 experts; in practice it acts like one with 32. The MoE benefits — the entire reason for V4's architecture — quietly evaporate.

You don't see this in a single-shot perplexity number. You see it in *task diversity*: the model gets noticeably worse at off-distribution tasks because the experts that handled those distributions are the ones that died.

### Q8_0 attention is necessary but not sufficient

Q8_0 on Q/K/V/O is widely understood to preserve attention quality on its own. But attention does not run on its own — it operates on the residual stream, into which the IQ2_XXS expert outputs flow. Quantization noise from the FFN path *contaminates* the attention input on the next layer.

Concretely: even if Q8_0 attention has perfect numerics, if the residual stream is being mildly perturbed every layer by FFN quant error, by the time you get 60 layers deep the perturbation is no longer mild. Long-context attention at 1M tokens accumulates *layer-depth × sequence-length* worth of noise, not just one or the other.

This means the 1M context claim, which works on paper for the cloud (full precision) build, needs *empirical verification* on this quant. Q8_0 attention proj is a necessary condition for 1M working; it is not a sufficient one.

### "Smart asymmetric" cuts against future maintainability

The plan above is *exquisitely tuned to V4 Flash specifically*. When a V4 Pro variant ships, or V4.1, or a fine-tune that shifts the activation distribution — the same component-precision plan will not transfer. The asymmetric quant has effectively been hand-fit to one model checkpoint.

Compare to a uniform Q4_K_M: nobody calls it clever, but you can re-quantize a new V4-family checkpoint in an afternoon and trust the result. The IQ2_XXS plan above requires re-running ablations to verify each component is still in the right precision class. The cleverness has an ongoing maintenance cost.

### Mixed-precision kernel maturity is uneven

Q8_0 and Q4_K kernels on Apple Metal and CUDA are battle-tested. IQ2_XXS kernels are newer; their performance characteristics differ across:

- **GPU vendor** (NVIDIA fused dot products vs Metal's `simdgroup_matrix` paths).
- **GPU generation** (Ada vs Hopper, M2 vs M4).
- **Inference engine** (llama.cpp current vs the V4-specific dsedge backend that ships in deeptide).

The wallclock cost of an IQ2_XXS dot product is *not* always faster than the Q4_K dot product it replaces, even though IQ2_XXS uses fewer bits, because the unpacking step is more complex. Smaller weights with slower decode can be slower end-to-end. This is a deployment-time discovery, not something the file name predicts.

### F16 indexer is "F16 enough" — until it isn't

The DSA indexer being F16 is great for top-k *score precision*. It does not help with the *top-k selection itself* at extreme context lengths: F16 has ~3 decimal digits of mantissa, and at 1M tokens the indexer scores must distinguish among many tokens with very similar scores. Some recent V4 inference stacks have started pushing the indexer to F32 specifically for this reason. F16 is a reasonable default, not an obvious win at the long end of the context window.

## Verdict

This is, on net, one of the most pragmatic V4 quantizations published — *for a specific deployment shape*. It is not "the right quant for V4 Flash"; it is "the right quant for V4 Flash when your constraint is ~80 GB resident and your workload is short-to-medium context coding."

For the other two relevant deployment shapes:

- **128 GB Mac, long-context agent work** — the [Q4_K mixed-precision build](./deepseek-v4-flash-q2.md#the-q4_k-mixed-precision-local-build) (153 GiB) is the better choice if your machine can hold it. The IQ2_XXS file is a fallback when it can't.
- **128 GB Mac, "does the stack work" smoke-testing** — the [stock Q2 build](./deepseek-v4-flash-q2.md) (smaller, faster, ships with the dsedge defaults) is still the right choice. Don't pay 80 GB of RAM to validate that the server starts.

The asymmetric IQ2_XXS file sits between these — a real production option for hosts that have ~80 GB headroom but don't want to commit 150+ GB, and operators who care enough about quality to read the file name and parse what each suffix means.

## How to evaluate it on your workload

Trust the design philosophy, but verify the result against tasks that exercise the design's claimed weaknesses:

1. **Long-context recall.** Feed a 200k–500k-token prompt with a unique identifier mid-stream; ask the model to recall it at the end. Compare against the Q2 build and the cloud V4 baseline. The IQ2_XXS quant should clearly beat Q2 here and approach (but probably not match) cloud V4.
2. **Expert-diversity check.** Run a task batch spanning code, math, prose, structured data, and a non-English language. If the model degrades disproportionately on the off-distribution items, the expert-collapse risk is real for your iMatrix.
3. **Multi-turn agent stability.** A 30-turn coding agent session. Track whether tool-result blocks from turn 5 are still correctly recalled by turn 25. The FFN-residual-contamination risk shows up as the model "forgetting" things it has in context.
4. **Decode-rate measurement.** Wallclock tokens/second on your specific GPU/Metal stack — IQ2_XXS will not necessarily be faster than Q4_K_M despite the smaller bit count.

If the file passes those four checks on your hardware and your workload, use it. If it fails any of them, the failure mode tells you which component to revisit — go up one precision tier on that component, not on everything.

## Suggested next step for deeptide

This quant is not currently wired up as a `LocalAgentPolicy` profile (only the Q2 dsedge default and the Q4_K mixed-precision profile are). Adding it is straightforward — a new case in `LocalAgentPolicy.profile(for:)` analogous to the Q4_K entry, plus a `deepseek-v4-flash-iq2` family of aliases in `ModelAlias.swift`, plus a `ModelContextWindow.forModel` entry slotted in the same precedence-order region. The natural default context is **256k**: high enough to take real advantage of the F16 indexer/compressor, conservative enough to leave room for the IQ2_XXS expert tail to misbehave at the very long end of the window without invalidating the whole session. If empirical evaluation (see the four checks above) shows it holds at 1M, the cap can be lifted later.

Until that profile is added and validated, the file is documented but not yet supported as a first-class local model in deeptide.
