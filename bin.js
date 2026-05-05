#!/usr/bin/env node
// DeepTide CLI launcher — thin redirect to @paean-ai/zero-cli with the
// brand env var set so the underlying binary renders as DeepTide.
//
// All CLI source lives in https://github.com/a8e-ai/zero-cli. This file is
// the only meaningful code in the `deeptide` npm package; everything else
// is metadata.
process.env.ZERO_CLI_INVOKED_AS = 'tide'
process.env.NODE_ENV ??= 'production'
await import('@paean-ai/zero-cli/dist/_cli.js')
