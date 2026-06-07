<p align="center">
  <img src="./assets/logo.svg" alt="DeepTide logo" width="200">
</p>

<h1 align="center">DeepTide</h1>

<p align="center">
  <strong>Built by DeepSeek, for DeepSeek.</strong><br>
  An AI coding agent that flows through your codebase like a tide.
  <br><sub>The name: <strong>DeepSeek</strong> + <strong>tide</strong> (terminal IDE).</sub>
</p>

<p align="center">
  <a href="https://github.com/paean-ai/deeptide/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.npmjs.com/package/deeptide"><img src="https://img.shields.io/npm/v/deeptide.svg" alt="npm version"></a>
  <a href="https://github.com/paean-ai/deeptide/stargazers"><img src="https://img.shields.io/github/stars/paean-ai/deeptide?style=social" alt="GitHub stars"></a>
</p>

---

## Three flavors, same team

| | DeepTide for macOS | DeepTide CLI (`deeptide`) | DeepTide CLI Rust (`deeptide-rs`) |
|---|---|---|---|
| **Form factor** | Native macOS app | Cross-platform terminal CLI | Cross-platform terminal CLI **+ native desktop GUI** ([`--gui`](#desktop-gui-deeptide-rs)) |
| **Runtime** | Swift 6 native binary, ~15 MB idle | Bun, ~50 MB resident | Native Rust binary, ~10 MB on disk, no runtime |
| **Platforms** | macOS 15+ | Linux · Windows · macOS | Linux · Windows · macOS |
| **Install** | `curl -fsSL https://deeptide.sh/install.sh \| sh` | `bun add -g deeptide` (this package) | `npm install -g deeptide-rs` |
| **CLI name(s)** | DeepTide.app | `deeptide`, `tide` | `deeptide-rs` |
| **Lineage** | 100% authored by DeepSeek V4 | Powered by open-source [Zero CLI](https://github.com/a8e-ai/zero-cli) | Authored under [`crates/`](./crates) in this repo |
| **Best for** | macOS users who want a tuned native experience | Existing users; richest plugin surface | Headless CI, slow laptops, single-binary deploys |

This repository is the **community front door for all three** — docs, FAQ,
issue tracking — and is the home of two npm packages:

- [`deeptide`](./package.json) — the current TypeScript/Bun CLI (forwards to
  [`@paean-ai/zero-cli`](https://www.npmjs.com/package/@paean-ai/zero-cli))
- [`deeptide-rs`](./npm/deeptide-rs) — the Rust CLI (ships a native binary
  via GitHub Releases postinstall)

The two CLI packages **do not conflict** — they expose different binary
names (`deeptide`/`tide` vs `deeptide-rs`) so you can install both and
switch between them freely while we mature the Rust port.

The Rust port lives under [`crates/`](./crates). It is intended to grow
into the canonical cross-platform CLI over time, but the `deeptide` package
will remain available as long as users find value in it — there is no
forced migration.

It also contains the open-source native local inference runtime under
[`native/`](./native): a hard-forked `ds4` DeepSeek V4 Flash Metal engine plus
`dsgo`, the local OpenAI/Anthropic-compatible gateway intended to pair with
DeepTide.

---

## Install on macOS (recommended)

For Mac users, the recommended path is the native Deeptide build from
[deeptide.sh](https://deeptide.sh/). It downloads the signed build for your
Mac architecture and installs both `deeptide` and the shorter `tide` command:

```bash
curl -fsSL https://deeptide.sh/install.sh | sh
```

Then start with:

```bash
tide auth login   # Paean OAuth, multimodal-aware
tide login        # or save a DeepSeek API key directly
tide              # launch the REPL
tide doctor       # diagnose install + network
```

If you want a native Mac terminal to pair with Deeptide, also try
[Clide](https://clide.app/) — a modern macOS terminal with file explorer,
multi-pane layouts, drag-and-drop, and native voice input.

## Install on Linux / Windows (Zero CLI alias)

> **Prerequisite:** [Bun](https://bun.com/) must be installed and on
> PATH. The CLI runtime requires it (matches the underlying
> [Zero CLI](https://github.com/a8e-ai/zero-cli)). Bun does not
> replace your Node install — it sits alongside.

On non-Mac systems, this npm package is the cross-platform DeepTide-flavored
entrypoint powered by [Zero CLI](https://github.com/a8e-ai/zero-cli). It
installs the `deeptide` and `tide` aliases for a workflow close to Deeptide,
but it is not the Swift-native macOS build from `deeptide.sh`.

```bash
# bun (recommended, fastest install)
bun add -g deeptide

# npm (works too; bun is still required at runtime)
npm install -g deeptide

# pnpm
pnpm add -g deeptide
```

Two commands are installed; pick whichever your fingers prefer:

```bash
tide                          # interactive REPL (preferred — short)
deeptide                      # same thing, full name
tide -p "explain this repo"   # one-shot mode
tide --help                   # all options
```

You can also install the upstream package directly:

```bash
bun add -g @paean-ai/zero-cli
```

## Build the macOS native app from source

The macOS native build is open source at
[paean-ai/deeptide](https://github.com/paean-ai/deeptide) (Swift). Most users
should install from [deeptide.sh](https://deeptide.sh/), but source builders can
inspect and build from the source tree when they need to modify the native
runtime or local inference components.

---

## Quick start (CLI)

DeepTide CLI talks to the **DeepSeek API** by default (matching the
DeepTide native app), and can also drive any Anthropic-protocol-compatible
endpoint via BYOK — Zhipu GLM, Volcengine, Paean, Qwen, Moonshot,
self-hosted gateways, and so on.

```bash
# Default path — DeepSeek
export DEEPSEEK_API_KEY="sk-..."
tide

# BYOK to another provider
tide --base-url https://open.bigmodel.cn/api/anthropic --api-key <GLM_KEY>

# One-shot, non-interactive
tide -p "Explain the auth middleware"
```

For the full configuration surface (settings.json schema, hooks,
permissions, MCP servers, sub-agents, model aliases) see the upstream
[Zero CLI README](https://github.com/a8e-ai/zero-cli#readme), which is
the authoritative reference.

---

## Desktop GUI (`deeptide-rs`)

The Rust port ships a **native desktop app** — a single Rust binary (egui, no
Electron/webview) that is a thin window over the same engine the CLI uses. It
**fully shares the CLI's configuration, session history, and the entire tool
set** — there is no separate config to maintain:

- **Same config** — reads the identical `~/.config/tide/settings.json` (and
  project `.deeptide/settings.json`); set your provider/model/API key once and
  both the CLI and the GUI pick it up.
- **Same sessions** — conversations save to the same on-disk store, so a chat
  started in the GUI is `deeptide-rs --resume`-able from the terminal, and a CLI
  session shows up in the GUI's sidebar to resume with one click.
- **Same tools** — the full agent tool set (Read/Write/Edit, Bash, Glob/Grep,
  WebFetch, MCP servers, sub-agents, …) is available identically.

What it does: streaming chat with **markdown rendering**, live **reasoning**
("💭 thinking") and **tool-call cards**, **interactive tool approvals** (with a
coloured diff preview for Write/Edit), a **session sidebar** (resume past
chats), a **provider/model picker**, **New chat**, **Stop/interrupt**, and a
**cost/usage bar** (↑/↓ tokens · cache · $).

### Launch

```bash
# Via the CLI launcher (execs the desktop binary, sharing this config/cwd):
deeptide-rs --gui

# …or run the GUI binary directly:
deeptide-gui
```

### Build from source

```bash
# From the repo root (the Rust workspace under crates/):
cargo run -p deeptide-gui            # dev run
cargo build -p deeptide-gui --release # release binary at target/release/deeptide-gui
```

Configure a model the same way as the CLI — e.g. `export DEEPTIDE_API_KEY=…`
(or set it in `settings.json`) before launching. Without a credential the GUI
opens in a safe local-echo mode and shows a "no API key" banner. To point it at
a non-default provider, use the in-app picker or `export DEEPTIDE_PROVIDER=…`
(e.g. `deepseek`, `ollama`, `openai`, `gemini`).

---

## Built-in capabilities

DeepTide is an **agentic** coding assistant — the model plans, calls
tools, observes results, and adapts. Out of the box:

- **Multi-turn agentic loop** — plan → tool → observe → adapt
- **Streaming responses** — see the model think and act in real time
- **30+ built-in tools** — file I/O, shell, web, tasks, MCP, scheduling, sub-agents
- **25+ slash commands** — `/status`, `/cost`, `/diff`, `/init`, `/permission`, `/hooks`, …
- **Permission modes** — default · accept-edits · plan · bypass
- **Hooks engine** — pre/post tool, user-prompt, session, compaction shell hooks
- **Memory system** — persistent project memory across sessions
- **Sub-agents from markdown** — define custom agents in your project
- **Custom slash commands from markdown** — drop a `<name>.md` under
  `.deeptide/commands/` (or `~/.deeptide/commands/`) and run it as `/<name>`;
  the body becomes a prompt with `$ARGUMENTS` / `$1`…`$11` substitution. Built-in
  commands always take precedence.
- **Plan mode** — design before you code, get approval before execution
- **Web research** — `/deep-seek <question>` uses the model's built-in
  knowledge to propose likely official/canonical URLs, then verifies them with
  `WebFetch`. Optional `BRAVE_SEARCH_API_KEY` / `SERPER_API_KEY` credentials
  can improve source discovery, but are not required for URL-based research.
  Sources are labeled `[verified]`, `[known]`, or `[unverified]`.

The tool catalog, slash command set, hook event names, and model
aliases are kept aligned with the macOS native DeepTide app via the
[`tide-spec`](https://github.com/a8e-ai/zero-cli/blob/main/docs/tide-spec.md)
shared interface contract — your hooks and muscle memory port between
the two.

---

## Documentation

- [`tide-spec`](https://github.com/a8e-ai/zero-cli/blob/main/docs/tide-spec.md) — shared interface contract: tool catalog, slash commands, CLI flags, hook env vars, model aliases.
- [Zero CLI docs](https://github.com/a8e-ai/zero-cli/tree/main/docs) — comprehensive reference for the CLI engine.
- [DeepTide native source](https://github.com/paean-ai/deeptide) — Swift codebase for the macOS app.
- [`native/`](./native) — local DeepSeek runtime source: `ds4` inference engine and `dsgo` gateway.
- [`docs/deepseek-v4-flash-q2.md`](./docs/deepseek-v4-flash-q2.md) — what the stock local V4 Flash Q2 build is and is not good for, and why deeptide caps it at 64k context. Also covers the higher-fidelity [Q4_K mixed-precision build](./docs/deepseek-v4-flash-q2.md#the-q4_k-mixed-precision-local-build) (153 GiB) for hosts with 192 GB+ memory.
- [`docs/deepseek-v4-flash-iq2-asymmetric.md`](./docs/deepseek-v4-flash-iq2-asymmetric.md) — dialectical analysis of the asymmetric IQ2_XXS + Q2_K + Q8_0 + F16 quant (80.8 GiB), the size sweet spot for 128 GB Macs and single 80 GB H100 boxes. Walks through what the per-tensor-group precision plan gets right *and* the failure modes it takes on (expert collapse risk, FFN residual contamination, iMatrix dependence).

## Samples

- [`samples/pixel-roguelike`](./samples/pixel-roguelike) — a dependency-free
  HTML Canvas roguelike showcase with editable pixel-art sprites, procedural
  terrain, responsive mobile controls, combat effects, and a pixel-art preview
  helper.
- [`samples/pixel-backpack-roguelite`](./samples/pixel-backpack-roguelite) —
  a pixel backpack roguelite with spatial item planning, merging, artifact
  forging, autonomous TD-style combat waves, and infinite scaling rewards.
- [`samples/deeptide-deck-template`](./samples/deeptide-deck-template) — a
  responsive HTML presentation deck template for desktop demos, mobile reading,
  and polished browser-to-PDF export, using Deeptide as the reference product.
- [`samples/agent-workbench-template`](./samples/agent-workbench-template) —
  a polished coding-agent handoff template for plans, diffs, verification,
  risks, and PR-ready review notes.
- [`samples/agent-brief-template`](./samples/agent-brief-template) — a
  professional research and decision brief template for general-agent outputs,
  option comparisons, evidence matrices, and action plans.
- [`samples/void-descent`](./samples/void-descent) — a matching pixel-art
  turn-based dungeon crawler showcase with fog of war, room-and-corridor
  exploration, floor descent, persistent upgrades, items, and mobile swipes.

## Skills

- [`skills/`](./skills) — English, sample-derived agent skills for pixel-art
  coding, premium visual design, HTML decks, roguelike game loops, casual game
  systems, responsive Canvas game shells, coding-agent delivery, and research
  brief workflows.

## Local DeepSeek runtime

The native runtime source is managed in this repository without changing the npm
package install surface. `package.json` uses an explicit `files` allowlist, so
`native/` is available to GitHub users and source builders but is not packed into
the published npm wrapper.

Build both native components on macOS:

```bash
npm run build:native
```

Or build individually:

```bash
npm run build:ds4
npm run build:dsgo
```

See [`native/README.md`](./native/README.md), [`native/ds4/README.md`](./native/ds4/README.md),
and [`native/dsgo/README.md`](./native/dsgo/README.md) for runtime details.

---

## Reporting issues

Open an [issue](https://github.com/paean-ai/deeptide/issues). The new-
issue form will route you to the right template:

- **CLI bug or feature** — typically forwarded to
  [`a8e-ai/zero-cli`](https://github.com/a8e-ai/zero-cli) where the CLI
  code lives, but feel free to start here if you're not sure.
- **macOS native app** — report here. Please **redact** any sensitive
  paths, file names, or model output before pasting logs.
- **Documentation, install, or general questions** — start here.
- **Security vulnerabilities** — use [private vulnerability reporting](https://github.com/paean-ai/deeptide/security/advisories/new), not public issues.

⚠️ **Privacy reminder:** crash logs and console output may contain
project paths, file names, environment variable values, or model
output. Review before pasting into a public issue. If a report
genuinely needs sensitive data, contact the maintainers privately
instead.

---

## About this npm package

The `deeptide` npm package is an intentionally thin redirect to
[`@paean-ai/zero-cli`](https://www.npmjs.com/package/@paean-ai/zero-cli),
which contains all CLI source code. We publish under both names so users
can install whichever feels natural; both resolve to the same binary
with the DeepTide brand surface. There is exactly one source of truth
([`a8e-ai/zero-cli`](https://github.com/a8e-ai/zero-cli)) — no
duplicated implementation.

## License

MIT — see [`LICENSE`](./LICENSE) and [`NOTICE`](./NOTICE).

DeepTide is an independent project and is not affiliated with,
endorsed by, or sponsored by Anthropic, Inc. or DeepSeek (Hangzhou)
AI Co., Ltd.

---

<p align="center">
  <sub>Built with 🐳 by <a href="https://a8e.ai">a8e</a> — for DeepSeek, on every platform.</sub>
</p>
