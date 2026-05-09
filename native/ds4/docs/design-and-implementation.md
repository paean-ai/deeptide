# ds4.c: Design and Implementation

This document describes how the **ds4** project is structured, what problems it optimizes for, and how the major components fit together. It complements the root [`README.md`](../README.md) and [`AGENT.md`](../AGENT.md).

## Purpose and scope

**ds4.c** is a **native, DeepSeek V4 Flash–specific** inference engine for Apple Silicon. It is deliberately **not** a generic GGUF runner:

- It binds **one fixed tensor layout** (layer counts, head geometry, MoE routing, compressed KV/indexer behavior, vocabulary size, etc.) and refuses GGUFs that do not match.
- It targets **Metal** as the production backend: a **whole-model Metal graph** executes prefill and decode on the GPU with tensor-resident activations and KV state.
- It ships **narrow HTTP/API glue** (`ds4-server`) and a **CLI** (`ds4`) that share the same engine boundary (`ds4_engine`, `ds4_session`).

The design thesis (see README) is that **compressed KV** plus **fast SSDs** make **disk-first KV caching** practical for long agent sessions: reuse expensive prefixes without recomputing full prefills on every stateless request.

## Architectural boundaries

Public inference state is split into two opaque handles declared in [`ds4.h`](../ds4.h):

| Concept | Role |
|--------|------|
| **`ds4_engine`** | Loaded model: mmap-backed weights, tokenizer, backend selection, fixed-shape validator. |
| **`ds4_session`** | One **mutable timeline**: live KV checkpoint position, logits buffer, graph/device buffers for that session. |

Callers (CLI or server) build token prefixes (`ds4_tokens`) via chat-encoding helpers, then call **`ds4_session_sync()`**: if the new prompt extends the current checkpoint, only the **suffix** is evaluated; otherwise the graph is refilled from scratch. Sampling uses **`ds4_session_sample()`** / greedy **`ds4_session_argmax()`** after sync.

This keeps HTTP and REPL code **free of tensor internals** while the implementation in [`ds4.c`](../ds4.c) owns GGUF parsing, layout, CPU reference paths, Metal scheduling, and disk-cache **payload** serialization.

## Repository layout

| Path | Responsibility |
|------|----------------|
| [`ds4.c`](../ds4.c) | GGUF loader (mmap), DS4 shape constants, tokenizer wiring, CPU reference kernels, Metal graph driver, `ds4_session_*`, disk payload read/write. |
| [`ds4.h`](../ds4.h) | Stable C API for engine, session, chat encoding, disk payload hooks. |
| [`ds4_metal.m`](../ds4_metal.m) / [`ds4_metal.h`](../ds4_metal.h) | Objective-C Metal runtime: buffers, command encoding, wrappers around kernels. |
| [`metal/*.metal`](../metal/) | Compute kernels (attention/KV, MoE, norms, flash-style paths, indexer helpers, etc.). |
| [`ds4_cli.c`](../ds4_cli.c) | `linenoise` REPL, one-shot mode, interactive transcript + single session. |
| [`ds4_server.c`](../ds4_server.c) | HTTP server, JSON parsing, OpenAI/Anthropic endpoint mapping, tool-call DSML bridging, **disk KV policy**, worker queue. |
| [`tests/ds4_test.c`](../tests/ds4_test.c) | C test runner (vectors, server smoke tests when enabled). |
| [`tests/test-vectors/`](../tests/test-vectors/) | Official API–derived logprob vectors for regression. |

Build logic is in [`Makefile`](../Makefile): on Darwin, `ds4` and `ds4-server` link Metal frameworks; `ds4_native` builds CPU-only objects for non-GPU debugging (`DS4_NO_METAL`).

## Model loading and memory

- **mmap**: Tensor data stays mapped; the loader validates metadata and tensor layout against fixed enums instead of copying the full model into RAM up front.
- **Metal**: Weight slices are exposed to the GPU as views over the mapping where appropriate; activations and KV live in **device-owned** tensors across command batches (see comments in [`ds4_metal.h`](../ds4_metal.h)).
- **Quantization**: The engine implements only the GGUF block types needed for this project’s published GGUFs (e.g. routed experts at 2-bit / 4-bit mixes per README; shared/projections per layout).

## Inference paths

### Metal graph (production)

The optimized path batches work into Metal commands: **chunked prefill** for long prompts and **decode steps** that advance KV and logits. Timing/logging hooks in `ds4.c` record encode vs execute vs read-back costs for debugging throughput.

### CPU backend (reference only)

A CPU path exists for correctness and tests. **README warning**: heavy CPU inference on recent macOS versions has triggered **kernel virtual-memory issues**; production use is **Metal-only** for the server.

### MTP / speculative decoding

Optional MTP weights can be loaded; speculative **`ds4_session_eval_speculative_argmax`** exists behind CLI/server flags. README states MTP is **experimental** and not a large win yet.

## Chat encoding and thinking modes

The engine exposes helpers to:

- Begin a chat transcript and append **system / user / assistant** segments with DeepSeek-style markers.
- Toggle **thinking**: none, normal, or **Think Max** (with a **minimum context threshold** so Think Max is not injected into tiny contexts—see `DS4_THINK_MAX_MIN_CONTEXT` and related logic in `ds4.c`).

The server maps OpenAI `reasoning_effort` / Anthropic-style controls onto these modes (details in README).

## Disk KV cache (server)

 Stateless HTTP clients typically **resend the entire conversation**. `ds4-server`:

1. Canonicalizes the request into token IDs.
2. Matches **longest prefix** against an in-memory/disk index (keyed by **SHA1 of token IDs**, not raw text—see README).
3. Calls **`ds4_session_sync`** so only new suffix tokens run through prefill.
4. Persists checkpoints to disk (ordinary read/write I/O, **not mmap**, to avoid extra VM pressure in an already large address space).

The **on-disk format** (KVC header + optional rendered text for humans + DS4 payload) and **when** checkpoints are written (cold / continued / evict / shutdown) are documented in the README. The engine implements **`ds4_session_save_payload` / `ds4_session_load_payload`**; the server owns eviction policy and directory quotas.

## Concurrency model (`ds4-server`)

- **Per-connection threads** parse HTTP and queue work.
- **One Metal worker** serializes inference: a single live `ds4_session` and graph checkpoint avoid cross-thread graph mutation. Concurrent requests **wait**; there is **no multi-request batching** today.

## HTTP surface

Supported routes include OpenAI-style **`/v1/chat/completions`**, **`/v1/completions`**, **`/v1/models`**, and Anthropic-style **`/v1/messages`**. Streaming uses SSE; tool calls round-trip through DeepSeek DSML formatting inside the server.

## Correctness and testing

- **Logprob vectors**: [`tests/test-vectors/`](../tests/test-vectors/) compares local **`--dump-logprobs`** output against slices captured from the **official DeepSeek V4 Flash API** (greedy, thinking off, bounded top logprobs—API does not expose full logits).
- **`make test`**: builds `ds4_test` and runs the C runner (`--logprob-vectors`, `--server`, etc., depending on build/environment).

## Operational constraints (from project docs)

- **Single-model lock**: A file lock prevents multiple huge model processes from stomping each other (see `g_ds4_lock_fd` in `ds4.c`).
- **Hardware class**: README targets **128 GB** unified memory machines for **q2** weights (~81 GB model footprint + headroom); **q4** expects **≥ 256 GB**. Benchmarks cite **MacBook Pro M3 Max (128 GB)** and **Mac Studio M3 Ultra (512 GB)**—these are reference machines for throughput tables, not minimum Apple SKU requirements for all future chips.

## Dependencies and credits

The project **does not link GGML**, but reuses/adapts MIT-licensed pieces (quant layouts, some kernels) per [`LICENSE`](../LICENSE) and README acknowledgements—**llama.cpp / GGML** are the conceptual parent ecosystem.

## Summary

**ds4.c** compresses the problem to: **one model**, **one Metal graph path**, **session sync for KV reuse**, and **disk-backed checkpoints for agent workloads**—with **official-vector regression** to keep logits aligned with the hosted model within the API-exposed surface.
