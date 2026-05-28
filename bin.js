#!/usr/bin/env bun
// DeepTide CLI launcher — thin redirect to @paean-ai/zero-cli with the
// brand env var set so the underlying binary renders as DeepTide.
//
// All CLI source lives in https://github.com/a8e-ai/zero-cli. This file is
// the only meaningful code in the `deeptide` npm package; everything else
// is metadata — with one exception: `tide local`, the one-command bootstrap
// for running DeepSeek V4 Flash agentically on Apple Silicon, is intercepted
// here and handled by ./scripts/tide-local.mjs before any zero-cli code
// loads. See docs/deepseek-v4-flash-q2.md for the local-mode profile design.
//
// Runtime: `bun` is required for the zero-cli path, matching @paean-ai/zero-cli's
// dist/cli.js shebang. Several transitive deps (e.g. jsonc-parser) use
// extensionless ESM imports that bun resolves but strict Node ESM rejects.
// The `tide local` path uses only Node-builtin modules so it runs under bun too.
process.env.ZERO_CLI_INVOKED_AS = 'tide'
process.env.NODE_ENV ??= 'production'

if (process.argv[2] === 'local') {
  const mod = await import('./scripts/tide-local.mjs')
  await mod.run(process.argv.slice(3))
} else {
  await import('@paean-ai/zero-cli/dist/_cli.js')
}
