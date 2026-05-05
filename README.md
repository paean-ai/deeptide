# DeepTide

A modern AI coding agent. Two flavors, same team.

- **DeepTide for macOS** — native Mac app with a tuned GUI.
- **DeepTide CLI** — cross-platform terminal version, powered by the
  open-source [Zero CLI](https://github.com/a8e-ai/zero-cli). Available
  as `deeptide` or `tide` from any shell on Linux, Windows, and macOS.

This repository is the community front door for both: docs, FAQ, issue
tracking, and the npm redirect package that ships the CLI.

---

## Install (CLI)

> **Prerequisite:** [Bun](https://bun.com/) must be installed and on
> PATH. The CLI runtime requires it (matches the underlying
> [Zero CLI](https://github.com/a8e-ai/zero-cli)). Bun does not
> replace your Node install — it sits alongside.

```bash
# bun (recommended, fastest install)
bun add -g deeptide

# npm (works too; bun is still required at runtime)
npm install -g deeptide

# pnpm
pnpm add -g deeptide
```

Then:

```bash
tide                          # interactive REPL
deeptide                      # same thing
tide -p "explain this repo"   # one-shot mode
tide --help                   # all options
```

`deeptide` and `tide` are both available after install. Pick whichever
your fingers prefer.

## Install (macOS native app)

The macOS native app is distributed separately. See
[Releases](https://github.com/paean-ai/deeptide/releases) (or the
project homepage when announced) for the latest build.

## Quick start (CLI)

The CLI talks to Anthropic-protocol-compatible LLM APIs. The fastest
path on first run:

```bash
tide login                    # browser-based OAuth (Paean AI default)
tide                          # start an interactive session
```

For BYOK (bring-your-own-key) flows and other providers (DeepSeek, Zhipu
GLM, Volcengine, etc.), see the [Zero CLI README](https://github.com/a8e-ai/zero-cli#readme),
which is the upstream and authoritative configuration reference.

## Documentation

- [Shared interface contract (`tide-spec`)](https://github.com/a8e-ai/zero-cli/blob/main/docs/tide-spec.md)
  — tool catalog, slash commands, CLI flags, hook env vars, model
  aliases. Both DeepTide for macOS and the CLI conform to this.
- [Zero CLI docs](https://github.com/a8e-ai/zero-cli/tree/main/docs)
  — comprehensive reference for the CLI engine.

## Reporting issues

Please use the [issue tracker](https://github.com/paean-ai/deeptide/issues).
The new-issue form will route you to the right template:

- **CLI bug or feature** — typically forwarded to
  [`a8e-ai/zero-cli`](https://github.com/a8e-ai/zero-cli) where the CLI
  code lives, but feel free to start here if you're not sure.
- **macOS native app** — report here. Please **redact** any sensitive
  paths, file names, or model output before pasting logs.
- **Documentation, install, or general questions** — start here.

⚠️ **Privacy reminder:** crash logs and console output may contain
project paths, file names, environment variable values, or model
output. Review before pasting into a public issue. If a report
genuinely needs sensitive data, contact the maintainers privately
instead.

## About this npm package

The `deeptide` npm package is an intentionally thin redirect to
[`@paean-ai/zero-cli`](https://www.npmjs.com/package/@paean-ai/zero-cli),
which contains all CLI source code. We publish under both names so users
can install whichever feels natural; both resolve to the same binary
with the DeepTide brand surface. There is exactly one source of truth
([a8e-ai/zero-cli](https://github.com/a8e-ai/zero-cli)) — no duplicated
implementation.

## License

MIT. See [`LICENSE`](./LICENSE) and [`NOTICE`](./NOTICE).

DeepTide is an independent project and is not affiliated with,
endorsed by, or sponsored by Anthropic, Inc.
