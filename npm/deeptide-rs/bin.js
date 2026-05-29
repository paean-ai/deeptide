#!/usr/bin/env node
// Runtime entrypoint for `deeptide-rs`. Resolves the prebuilt Rust
// binary that install.js placed under `<pkg>/binary/` and execs it,
// forwarding argv and signals transparently.
//
// Why a Node shim instead of declaring the binary directly in
// package.json `bin`:
//   - npm's `bin` field only handles JS / shebang scripts on Unix
//     and shims via cmd/ps1 wrappers on Windows. Native binaries
//     work but you can't conditionally pick one per architecture.
//   - The shim is the natural place to print actionable error
//     messages when the install couldn't fetch a prebuilt (e.g.
//     `DEEPTIDE_RS_BINARY` override pointing at a stale path).
//
// Process model: we use `spawnSync(... stdio: 'inherit')` so the
// child inherits the controlling TTY directly. This matters for
// rustyline-backed line editing — using a piped stdio would break
// raw-mode terminal handling.

import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { platform } from 'node:os'

const __dirname = dirname(fileURLToPath(import.meta.url))
const BINARY_NAME = platform() === 'win32' ? 'deeptide.exe' : 'deeptide'
const BINARY_PATH =
  process.env.DEEPTIDE_RS_BINARY ||
  join(__dirname, 'binary', BINARY_NAME)

if (!existsSync(BINARY_PATH)) {
  // Diagnostic-friendly bailout. We surface the resolved path and
  // every env var that influences it so a user can copy-paste this
  // into a bug report verbatim.
  process.stderr.write(
    [
      `deeptide-rs: no native binary found at ${BINARY_PATH}`,
      ``,
      `The package's postinstall step normally downloads the binary from`,
      `GitHub Releases. If you saw a download error during 'npm install',`,
      `re-run it, OR fall back to one of:`,
      ``,
      `  1. Build from source:`,
      `       cargo install --git https://github.com/paean-ai/deeptide \\`,
      `         --bin deeptide deeptide-cli`,
      `     Then point us at it:`,
      `       DEEPTIDE_RS_BINARY=$(which deeptide) deeptide-rs ...`,
      ``,
      `  2. Re-run the installer manually:`,
      `       node ${join(__dirname, 'install.js')}`,
      ``,
      `Environment that influenced the resolved path:`,
      `  DEEPTIDE_RS_BINARY = ${process.env.DEEPTIDE_RS_BINARY || '(unset)'}`,
      `  process.platform   = ${process.platform}`,
      `  process.arch       = ${process.arch}`,
      ``,
    ].join('\n')
  )
  process.exit(127)
}

// Forward argv (skipping `node` and bin.js itself) plus the full env.
// `stdio: 'inherit'` is mandatory for TTY-using subcommands.
const result = spawnSync(BINARY_PATH, process.argv.slice(2), {
  stdio: 'inherit',
  // Pass through every env var, including ones the user set just
  // for this invocation (DEEPTIDE_API_KEY, ANTHROPIC_API_KEY, etc).
  env: process.env,
})

// Mirror the child's exit semantics. spawnSync returns `null` for
// `status` when the child was killed by a signal — translate that to
// 128+signal so shells render it as the standard "terminated by N".
if (result.error) {
  process.stderr.write(`deeptide-rs: failed to spawn ${BINARY_PATH}: ${result.error.message}\n`)
  process.exit(127)
}
if (result.signal) {
  // 128 + signal number is the conventional shell encoding. We don't
  // have the numeric signal directly but child_process.spawnSync
  // returns the signal name; for the most common termination signals
  // hardcode the numbers, otherwise fall back to 1.
  const signalCode = { SIGINT: 130, SIGTERM: 143, SIGHUP: 129, SIGQUIT: 131 }[result.signal] ?? 1
  process.exit(signalCode)
}
process.exit(result.status ?? 0)
