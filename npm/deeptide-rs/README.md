# deeptide-rs

> Rust implementation of the DeepTide CLI — a cross-platform native
> binary that ships beside the existing TypeScript `deeptide` package.
> Install either, both, or switch back and forth at any time.

```
npm install -g deeptide-rs
deeptide-rs --doctor   # diagnostic report; no API key required
deeptide-rs --help
tider --help           # `tider` is a shorter alias for `deeptide-rs`
```

## Why two packages?

| | `deeptide` (existing) | `deeptide-rs` (this one) |
|---|---|---|
| **Runtime** | Bun / Node | Native Rust binary |
| **Binary size** | ~50 MB resident (Bun + JS) | ~10 MB on disk, no runtime |
| **Startup** | ~120 ms | <50 ms warm, ~700 ms cold (one-time) |
| **CLI name** | `deeptide`, `tide` | `deeptide-rs`, `tider` |
| **Source** | Forwards to [@paean-ai/zero-cli](https://www.npmjs.com/package/@paean-ai/zero-cli) | This repo's `crates/` |
| **Best for** | Existing users; richest plugin surface | Headless CI, slow laptops, single-binary deploys |

Both packages can be installed **side by side** — they expose different
binary names so neither shadows the other. Use `deeptide` for daily
work and `deeptide-rs` for CI / SSH boxes / containers where Bun is
overkill.

## What's in the box

- 60 first-class tools (Bash, Read, Edit, Grep, WebFetch, MCP, Vision,
  Notebook, Skills, Cron, Publish, Background Bash, …)
- Full agent loop with subagents (`Explore`, `Plan`, `Implement`),
  permission modes, hooks, prompt cache, streaming, fallback model
- 40+ REPL slash commands (`/cost`, `/diff`, `/branch`, `/dream`,
  `/goal`, `/cron`, `/permission`, `/skills`, `/doctor`, …)
- `--doctor`, `--list-models`, `--list-sessions` — no-API-key
  diagnostics
- Cross-platform desktop notifications (macOS, Linux, Windows)
- Self-contained: no Node, Bun, or interpreter dependency at runtime

## Headless / scripting

`deeptide-rs` runs an interactive REPL by default, but it's built to be
driven non-interactively too. The headless surface is wire-compatible
with the Swift Deeptide app and `zero-cli` — the same
`--print` / `--output-format` / `--input-format` / `--embedded` contract
and `stream-json` event shapes — so existing `tide --print` scripts work
unchanged.

```sh
# One-shot prompt, plain text to stdout
deeptide-rs --print -p "explain src/main.rs"

# Pipe the prompt in; emit a single JSON result object
echo "summarise the staged diff" | deeptide-rs --print --output-format json

# Stream structured events (NDJSON) for live consumption
deeptide-rs --print --output-format stream-json -p "refactor the parser"

# Combine a fixed instruction with piped file contents
deeptide-rs --print --read-stdin -p "review this patch" < change.diff

# Full autonomy with a restricted toolset (good for CI)
deeptide-rs --print -y --allowed-tools Read,Edit,Bash -p "fix the failing test"

# Embedded NDJSON protocol, driven by a host application
deeptide-rs --embedded
```

Aliases are accepted for drop-in parity with the other CLIs:
`--no-tui` (≡ `--print`) and `--max-tokens` (≡ `--max-output-tokens`).
On-device local-model flags (`-L`/`--local` and friends) are macOS-only
and handled by the `tide local` launcher, not this binary. Run
`deeptide-rs --help` for the full grouped flag reference.

## Configuration

`deeptide-rs` reads the same `settings.json` scopes as `deeptide`:

- Global: `~/.config/tide/settings.json`
- Project: `<repo>/.deeptide/settings.json`
- Local: `<repo>/.deeptide/settings.local.json`

It honours the standard credential env vars in order:

```
DEEPTIDE_API_KEY > ZERO_API_KEY > ANTHROPIC_API_KEY
```

If you've already configured `deeptide`/`tide`, `deeptide-rs` will
pick up the same providers and models automatically.

Run `deeptide-rs --doctor` to print the full resolution chain — it
never reveals credential values, only their presence and length, so
the output is safe to paste into a bug report.

## Distribution model

`npm install -g deeptide-rs` runs a postinstall step that downloads
the matching Rust binary from GitHub Releases:

- `darwin-x86_64` / `darwin-aarch64`
- `linux-x86_64` / `linux-aarch64`
- `windows-x86_64`

If the download fails (offline install, no release published yet for
your platform), the install **does not fail** — it warns and the
shim prints actionable next-step instructions the first time you
run `deeptide-rs`. Manual fallbacks:

1. **Build from source** — requires the Rust toolchain:
   ```
   cargo install --git https://github.com/paean-ai/deeptide \
     --bin deeptide deeptide-cli
   DEEPTIDE_RS_BINARY=$(which deeptide) deeptide-rs --doctor
   ```

2. **Point at a locally-built binary** — set
   `DEEPTIDE_RS_BINARY=/path/to/deeptide` before running.

3. **Re-run the downloader** — `node $(npm root -g)/deeptide-rs/install.js`.

## Versioning

`deeptide-rs` versions are independent of the TypeScript `deeptide`
package's `0.11.x` line — they start at `0.1.0` and follow normal
semver. Pin them separately:

```
npm install -g deeptide@0.11.8
npm install -g deeptide-rs@0.1.0
```

## License

MIT. See LICENSE file. Same terms as the rest of the DeepTide repo.
