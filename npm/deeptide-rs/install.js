#!/usr/bin/env node
// Postinstall script for the `deeptide-rs` npm package.
//
// Downloads the platform-matching prebuilt Rust binary from the GitHub
// Releases of the `paean-ai/deeptide` repo and places it next to bin.js.
//
// Design notes:
//
// * We deliberately do NOT use `optionalDependencies` (the
//   per-platform sub-package scheme that esbuild/swc use) for the
//   first release because that requires publishing N sibling
//   packages atomically. A downloader keeps the publish surface
//   to one package while we stabilize the release pipeline. Once
//   we ship a few versions we can migrate to optional deps and
//   drop the downloader without changing the public CLI name.
//
// * The script is **never fatal**. If the download fails (offline
//   install, no release published yet, sandboxed CI) we exit 0 and
//   print clear remediation steps. bin.js then surfaces an
//   actionable error at first run instead of breaking
//   `npm install -g deeptide-rs` outright. This matches the
//   convention `npm` itself uses for native-add-on packages.
//
// * The release artifact URL convention is:
//     https://github.com/paean-ai/deeptide/releases/download/
//       deeptide-rs-v<version>/deeptide-rs-<os>-<arch>.<ext>
//   where <ext> is `tar.gz` on Unix, `zip` on Windows. The
//   archive contains a single `deeptide` (or `deeptide.exe`) at
//   its root.
//
// * Users behind a proxy can bypass the download entirely:
//     DEEPTIDE_RS_SKIP_DOWNLOAD=1 npm install -g deeptide-rs
//   then drop a binary at $npm_config_prefix/lib/node_modules/
//   deeptide-rs/binary/ themselves.

import { createWriteStream, existsSync, mkdirSync, chmodSync, statSync } from 'node:fs'
import { rm } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { pipeline } from 'node:stream/promises'
import { spawnSync } from 'node:child_process'
import { tmpdir } from 'node:os'
import { createGunzip } from 'node:zlib'
import { Readable } from 'node:stream'

const __dirname = dirname(fileURLToPath(import.meta.url))
const PACKAGE_VERSION = readPackageVersion()
const TARGET = resolveTarget()
const BINARY_DIR = join(__dirname, 'binary')
const BINARY_NAME = TARGET.os === 'windows' ? 'deeptide.exe' : 'deeptide'
const BINARY_PATH = join(BINARY_DIR, BINARY_NAME)

async function main() {
  if (process.env.DEEPTIDE_RS_SKIP_DOWNLOAD === '1') {
    info('DEEPTIDE_RS_SKIP_DOWNLOAD=1 set; skipping binary download.')
    info(`Drop a binary at: ${BINARY_PATH}`)
    return
  }

  // Optional override useful for local development and air-gapped envs:
  //   DEEPTIDE_RS_BINARY=/path/to/built/deeptide npm install ...
  // Reuses that binary instead of downloading anything.
  const override = process.env.DEEPTIDE_RS_BINARY
  if (override && existsSync(override)) {
    info(`Using DEEPTIDE_RS_BINARY=${override}`)
    await copyBinary(override)
    return
  }

  if (!TARGET.supported) {
    warn(
      `unsupported platform: ${process.platform}/${process.arch}. ` +
      `Build from source with: cargo install --git ` +
      `https://github.com/paean-ai/deeptide --bin deeptide deeptide-cli`
    )
    return
  }

  const url = releaseUrl()
  info(`Downloading ${url}`)
  try {
    await downloadAndExtract(url)
  } catch (error) {
    warn(`could not fetch prebuilt binary: ${error.message}`)
    warn(
      'You can still use `deeptide-rs` if you build the binary locally and re-run\n' +
      '  node install.js\n' +
      'or by setting DEEPTIDE_RS_BINARY=/path/to/deeptide before npm install.'
    )
    // Never fail the install — bin.js will explain at first run.
  }
}

function readPackageVersion() {
  // Avoid importing JSON via assertion (varies across Node versions).
  // The script ships next to its own package.json so a sync read is fine.
  const pkg = require('node:fs').readFileSync(join(__dirname, 'package.json'), 'utf8')
  return JSON.parse(pkg).version
}

function resolveTarget() {
  const arch = {
    x64: 'x86_64',
    arm64: 'aarch64',
  }[process.arch]
  const os = {
    darwin: 'darwin',
    linux: 'linux',
    win32: 'windows',
  }[process.platform]
  if (!arch || !os) {
    return { supported: false, os: process.platform, arch: process.arch }
  }
  return { supported: true, os, arch }
}

function releaseUrl() {
  const ext = TARGET.os === 'windows' ? 'zip' : 'tar.gz'
  return (
    `https://github.com/paean-ai/deeptide/releases/download/` +
    `deeptide-rs-v${PACKAGE_VERSION}/` +
    `deeptide-rs-${TARGET.os}-${TARGET.arch}.${ext}`
  )
}

async function downloadAndExtract(url) {
  if (!existsSync(BINARY_DIR)) mkdirSync(BINARY_DIR, { recursive: true })

  const response = await fetchFollowingRedirects(url)
  if (!response.ok) {
    throw new Error(`HTTP ${response.status} ${response.statusText}`)
  }

  if (TARGET.os === 'windows') {
    // Write the archive to a temp file, then unzip via the system
    // `tar` (Windows 10+ ships bsdtar that understands `.zip`).
    const tmpZip = join(tmpdir(), `deeptide-rs-${Date.now()}.zip`)
    await pipeline(Readable.fromWeb(response.body), createWriteStream(tmpZip))
    const result = spawnSync('tar', ['-xf', tmpZip, '-C', BINARY_DIR], {
      stdio: 'inherit',
    })
    await rm(tmpZip, { force: true })
    if (result.status !== 0) {
      throw new Error('tar -xf failed; install bsdtar or extract the archive manually')
    }
  } else {
    // Unix: stream the gzipped tarball straight into `tar -x`. We pipe
    // through gunzip in Node so we don't depend on the platform tar's
    // gzip support (musl's busybox `tar` lacks `-z` in minimal images).
    const tar = spawnSync(
      tarBinary(),
      ['-x', '-C', BINARY_DIR],
      // Feed gunzipped bytes via stdin; sync read of the whole response
      // into a Buffer keeps install.js a single self-contained file.
      { input: await gunzipResponse(response), stdio: ['pipe', 'inherit', 'inherit'] }
    )
    if (tar.status !== 0) {
      throw new Error(`tar failed (status ${tar.status})`)
    }
  }

  if (!existsSync(BINARY_PATH)) {
    // The archive should contain a `deeptide` at its root. If the
    // upstream release packaging changes, surface a clear diagnostic
    // instead of a silent missing-binary.
    throw new Error(
      `archive did not contain ${BINARY_NAME} at its root; ` +
      `release artifact may be malformed`
    )
  }

  if (TARGET.os !== 'windows') chmodSync(BINARY_PATH, 0o755)
  const size = statSync(BINARY_PATH).size
  info(`Installed ${BINARY_PATH} (${(size / 1024 / 1024).toFixed(1)} MB)`)
}

async function gunzipResponse(response) {
  const gunzip = createGunzip()
  Readable.fromWeb(response.body).pipe(gunzip)
  const chunks = []
  for await (const chunk of gunzip) chunks.push(chunk)
  return Buffer.concat(chunks)
}

function tarBinary() {
  // macOS ships bsdtar at /usr/bin/tar; Linux ships GNU tar. Both
  // accept `-x -C <dir>` on stdin so we don't need to discriminate.
  return 'tar'
}

async function copyBinary(source) {
  if (!existsSync(BINARY_DIR)) mkdirSync(BINARY_DIR, { recursive: true })
  const fs = await import('node:fs/promises')
  await fs.copyFile(source, BINARY_PATH)
  if (TARGET.os !== 'windows') chmodSync(BINARY_PATH, 0o755)
  info(`Copied ${source} → ${BINARY_PATH}`)
}

async function fetchFollowingRedirects(url, depth = 0) {
  if (depth > 8) throw new Error('too many redirects')
  // Node 20+ has built-in fetch that follows redirects automatically
  // for normal requests, but GitHub Releases issue a 302 to S3 with
  // headers `fetch` strips on the next hop. Setting `redirect: 'manual'`
  // and walking ourselves makes the behaviour explicit and easy to
  // debug from a `curl --verbose` reproduction.
  const response = await fetch(url, { redirect: 'manual' })
  if (response.status >= 300 && response.status < 400 && response.headers.get('location')) {
    return fetchFollowingRedirects(response.headers.get('location'), depth + 1)
  }
  return response
}

// node-without-import-meta-resolve compat: use the legacy `require` to
// read JSON synchronously instead of an assert-using JSON import.
function require(spec) {
  if (require._cache?.[spec]) return require._cache[spec]
  const mod = createRequire(import.meta.url)
  return (require._cache ??= {})[spec] = mod(spec)
}
import { createRequire } from 'node:module'

function info(message) {
  // npm prefixes its own logs with `npm ` so prepending our package
  // name keeps install logs greppable: `npm install ... | grep deeptide-rs`.
  process.stdout.write(`[deeptide-rs] ${message}\n`)
}

function warn(message) {
  process.stderr.write(`[deeptide-rs] WARNING: ${message}\n`)
}

main().catch((error) => {
  warn(`install script failed: ${error.stack || error.message}`)
  // Soft-fail so npm install still succeeds and the user can recover
  // via manual fallback paths described in README.md.
  process.exit(0)
})
