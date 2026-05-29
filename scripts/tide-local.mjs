// `tide local` — one-command DeepSeek V4 Flash agentic local bootstrap.
//
// Invoked by bin.js when argv[2] === 'local'. Pure Node 18+ ESM, no runtime deps.
//
// Subcommands:
//   tide local                 full bootstrap: host probe → quant pick → build → download → service → self-test → settings
//   tide local start           same, accepting --model / --ctx / --port to pin choices
//   tide local stop            stop the LaunchAgent
//   tide local status          show service health, ports, model in use
//   tide local self-test       run only the streaming-tool-use verification against an already-running gateway
//   tide local doctor          host + toolchain readiness check (read-only)
//   tide local --dry-run       print the plan that `tide local` would execute, do nothing
//
// Profile surface mirrors docs/deepseek-v4-flash-q2.md:
//   flash       → deepseek-v4-flash      (IQ2_XXS, ~81 GiB,  64k ctx)   — for 96–160 GB hosts
//   flash-q4    → deepseek-v4-flash-q4k  (Q4_K mix,  ~153 GiB, 1M ctx)  — for 192 GB+ hosts

import { spawn, spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdir, readFile, rename, stat, writeFile } from 'node:fs/promises'
import { request as httpRequest } from 'node:http'
import { arch, homedir, platform as osPlatform, totalmem } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const HERE = dirname(fileURLToPath(import.meta.url))
const PKG_ROOT = resolve(HERE, '..')
const HOME = homedir()
const STATE_DIR = join(HOME, '.deeptide')
const LOG_DIR = join(STATE_DIR, 'log')
const SETTINGS_PATH = join(STATE_DIR, 'settings.json')
const LOCAL_STATE_PATH = join(STATE_DIR, 'local.json')
const LAUNCHD_LABEL = 'com.deeptide.local'
const LAUNCHD_PLIST = join(HOME, 'Library', 'LaunchAgents', `${LAUNCHD_LABEL}.plist`)
const DSGO_DEFAULT_PORT = 38442
const DS4_DEFAULT_PORT = 8000
const GIB = 1024 ** 3

const PROFILES = {
  flash: {
    canonicalId: 'deepseek-v4-flash',
    aliases: ['flash', 'fast', 'v4-flash', 'deepseek-v4-flash'],
    downloadTarget: 'q2',
    ggufFile: 'DeepSeek-V4-Flash-IQ2XXS-w2Q2K-AProjQ8-SExpQ8-OutQ8-chat-v2.gguf',
    sizeGiB: 81,
    contextWindow: 64_000,
    minHostRAMGiB: 96,
    notes: 'IQ2_XXS asymmetric — capped at 64k ctx (Q2 attention quality cliff)',
  },
  'flash-q4': {
    canonicalId: 'deepseek-v4-flash-q4k',
    aliases: ['flash-q4', 'flash-q4k', 'v4-flash-q4', 'v4-flash-q4k', 'deepseek-v4-flash-q4k'],
    downloadTarget: 'q4',
    ggufFile: 'DeepSeek-V4-Flash-Q4KExperts-F16HC-F16Compressor-F16Indexer-Q8Attn-Q8Shared-Q8Out-chat-v2.gguf',
    sizeGiB: 153,
    contextWindow: 1_000_000,
    minHostRAMGiB: 192,
    notes: 'Q4_K mixed-precision — 1M ctx, needs M3/M4 Ultra-class memory',
  },
}

// ---------- tiny logging ----------

const isTTY = process.stdout.isTTY
const c = (code) => (s) => isTTY ? `\x1b[${code}m${s}\x1b[0m` : s
const dim = c('2')
const bold = c('1')
const green = c('32')
const yellow = c('33')
const red = c('31')
const cyan = c('36')

function logStep(n, total, title) {
  process.stdout.write(`${cyan(`[${n}/${total}]`)} ${bold(title)}\n`)
}
function logLine(s) {
  process.stdout.write(`      ${s}\n`)
}
function logOk(s) {
  process.stdout.write(`      ${green('✓')} ${s}\n`)
}
function logWarn(s) {
  process.stderr.write(`      ${yellow('!')} ${s}\n`)
}
function fail(msg, exitCode = 1) {
  process.stderr.write(`${red('error:')} ${msg}\n`)
  process.exit(exitCode)
}

function assertPort(flag, raw) {
  const n = Number(raw)
  if (!Number.isInteger(n) || n < 1 || n > 65535) {
    fail(`${flag} requires an integer in 1..65535, got: ${JSON.stringify(raw)}`)
  }
  return n
}

function assertPositiveInt(flag, raw, max = Number.MAX_SAFE_INTEGER) {
  const n = Number(raw)
  const bounded = max < Number.MAX_SAFE_INTEGER
  if (!Number.isInteger(n) || n < 1 || n > max) {
    fail(`${flag} requires a positive integer${bounded ? ` ≤ ${max.toLocaleString('en-US')}` : ''}, got: ${JSON.stringify(raw)}`)
  }
  return n
}

// ---------- argv ----------

function parseArgs(argv) {
  const out = {
    sub: null,
    model: null,
    ctx: null,
    dsgoPort: DSGO_DEFAULT_PORT,
    ds4Port: DS4_DEFAULT_PORT,
    dryRun: false,
    yes: false,
    noBuild: false,
    noDownload: false,
    noService: false,
    noSelfTest: false,
    noSettings: false,
    help: false,
  }
  const positional = []
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i]
    const next = () => {
      if (i + 1 >= argv.length) fail(`${a} requires a value`)
      return argv[++i]
    }
    if (a === '-h' || a === '--help') out.help = true
    else if (a === '--dry-run') out.dryRun = true
    else if (a === '-y' || a === '--yes') out.yes = true
    else if (a === '--model') out.model = next()
    else if (a.startsWith('--model=')) out.model = a.slice('--model='.length)
    else if (a === '--ctx') out.ctx = assertPositiveInt('--ctx', next(), 10_000_000)
    else if (a.startsWith('--ctx=')) out.ctx = assertPositiveInt('--ctx', a.slice('--ctx='.length), 10_000_000)
    else if (a === '--port') out.dsgoPort = assertPort('--port', next())
    else if (a.startsWith('--port=')) out.dsgoPort = assertPort('--port', a.slice('--port='.length))
    else if (a === '--ds4-port') out.ds4Port = assertPort('--ds4-port', next())
    else if (a === '--no-build') out.noBuild = true
    else if (a === '--no-download') out.noDownload = true
    else if (a === '--no-service') out.noService = true
    else if (a === '--no-self-test') out.noSelfTest = true
    else if (a === '--no-settings') out.noSettings = true
    else if (a.startsWith('-')) fail(`unknown flag: ${a}`)
    else positional.push(a)
  }
  out.sub = positional[0] ?? 'start'
  return out
}

function usage() {
  process.stdout.write(`tide local — run DeepSeek V4 Flash agentically on your Mac, fully offline.

Usage:
  tide local [start]              full bootstrap (default): probe → build → download → service → self-test → settings
  tide local stop                 stop the local gateway LaunchAgent
  tide local status               show service health, ports, model in use
  tide local self-test            run only the streaming-tool-use verification
  tide local doctor               read-only host + toolchain readiness check
  tide local --dry-run            print the plan and exit
  tide local -h | --help          this message

Options for start:
  --model <alias>                 force a profile (${Object.keys(PROFILES).join(' | ')}); auto-picked from host RAM if omitted
  --ctx <N>                       override profile's documented context cap (advanced — read docs/deepseek-v4-flash-q2.md first)
  --port <N>                      dsgo gateway port (default ${DSGO_DEFAULT_PORT})
  --ds4-port <N>                  ds4-server port (default ${DS4_DEFAULT_PORT})
  --yes                           don't prompt for confirmation
  --no-build                      skip native build step (assume binaries present)
  --no-download                   skip weight download (assume gguf already on disk)
  --no-service                    don't write/load the LaunchAgent (run-as-foreground or external-supervisor case)
  --no-self-test                  skip the post-start streaming tool-use check
  --no-settings                   don't merge into ~/.deeptide/settings.json

Profiles (sync with docs/deepseek-v4-flash-q2.md):
  flash       deepseek-v4-flash      IQ2_XXS  ~81 GiB    64k ctx   ≥96 GB unified memory
  flash-q4    deepseek-v4-flash-q4k  Q4_K mix ~153 GiB   1M  ctx   ≥192 GB unified memory
`)
}

// ---------- host probe ----------

function sysctl(name) {
  const r = spawnSync('/usr/sbin/sysctl', ['-n', name], { encoding: 'utf8' })
  if (r.status !== 0) return null
  return r.stdout.trim()
}

function probeHost() {
  const plat = osPlatform()
  const cpuArch = arch()
  const totalRAMBytes = totalmem()
  const totalRAMGiB = Math.round(totalRAMBytes / GIB)
  const host = { platform: plat, arch: cpuArch, totalRAMGiB, isAppleSilicon: false, cpuBrand: null, macosVersion: null }
  if (plat === 'darwin') {
    host.cpuBrand = sysctl('machdep.cpu.brand_string') || null
    host.isAppleSilicon = cpuArch === 'arm64'
    const r = spawnSync('/usr/bin/sw_vers', ['-productVersion'], { encoding: 'utf8' })
    host.macosVersion = r.status === 0 ? r.stdout.trim() : null
  }
  return host
}

function describeHost(h) {
  const parts = []
  if (h.cpuBrand) parts.push(h.cpuBrand)
  parts.push(`${h.totalRAMGiB} GB unified memory`)
  if (h.macosVersion) parts.push(`macOS ${h.macosVersion}`)
  if (h.isAppleSilicon) parts.push('Apple Silicon')
  return parts.join(' · ')
}

function autoPickProfile(host) {
  if (host.totalRAMGiB >= PROFILES['flash-q4'].minHostRAMGiB) return resolveProfile('flash-q4')
  if (host.totalRAMGiB >= PROFILES.flash.minHostRAMGiB) return resolveProfile('flash')
  return null
}

function resolveProfile(name) {
  for (const [key, p] of Object.entries(PROFILES)) {
    if (key === name || p.aliases.includes(name)) return { key, ...p }
  }
  fail(`unknown profile alias: ${name} (known: ${Object.keys(PROFILES).join(', ')})`)
}

// ---------- paths ----------

function repoLayout() {
  const root = PKG_ROOT
  const ds4Dir = join(root, 'native', 'ds4')
  const dsgoDir = join(root, 'native', 'dsgo')
  if (!existsSync(ds4Dir) || !existsSync(dsgoDir)) {
    return null
  }
  return {
    root,
    ds4Dir,
    dsgoDir,
    ds4ServerBin: join(ds4Dir, 'ds4-server'),
    dsgoBin: join(dsgoDir, '.build', 'release', 'dsgo'),
    downloadScript: join(ds4Dir, 'download_model.sh'),
    ggufDir: join(ds4Dir, 'gguf'),
  }
}

// ---------- subprocess helpers ----------

function runStreaming(cmd, args, opts = {}) {
  return new Promise((resolveP, rejectP) => {
    const child = spawn(cmd, args, { stdio: 'inherit', ...opts })
    child.on('error', rejectP)
    child.on('close', (code) => {
      if (code === 0) resolveP()
      else rejectP(new Error(`${cmd} ${args.join(' ')} exited ${code}`))
    })
  })
}

function which(bin) {
  const r = spawnSync('/usr/bin/which', [bin], { encoding: 'utf8' })
  return r.status === 0 ? r.stdout.trim() : null
}

// ---------- build ----------

async function ensureNativeBuilt(layout, { dryRun }) {
  const ds4Ok = existsSync(layout.ds4ServerBin)
  const dsgoOk = existsSync(layout.dsgoBin)
  if (ds4Ok && dsgoOk) {
    logOk(`ds4-server present at ${layout.ds4ServerBin}`)
    logOk(`dsgo present at ${layout.dsgoBin}`)
    return
  }
  if (!which('make')) fail('make not found — install Xcode command line tools (xcode-select --install)')
  if (!which('swift')) fail('swift not found — install Xcode command line tools (xcode-select --install)')
  if (!ds4Ok) {
    logLine(`building ds4-server (make -C ${layout.ds4Dir})`)
    if (!dryRun) await runStreaming('make', [], { cwd: layout.ds4Dir })
  } else {
    logOk('ds4-server already built')
  }
  if (!dsgoOk) {
    logLine(`building dsgo (swift build -c release --package-path ${layout.dsgoDir})`)
    if (!dryRun) await runStreaming('swift', ['build', '-c', 'release', '--package-path', layout.dsgoDir])
  } else {
    logOk('dsgo already built')
  }
}

// ---------- download ----------

async function ensureWeightsDownloaded(layout, profile, { dryRun }) {
  await mkdir(layout.ggufDir, { recursive: true })
  const ggufPath = join(layout.ggufDir, profile.ggufFile)
  if (existsSync(ggufPath)) {
    try {
      const st = await stat(ggufPath)
      const gib = (st.size / GIB).toFixed(1)
      logOk(`${profile.ggufFile} present (${gib} GiB)`)
      return ggufPath
    } catch (e) {
      logWarn(`could not stat ${ggufPath}: ${e.message}`)
    }
  }
  logLine(`downloading ${profile.ggufFile} (~${profile.sizeGiB} GiB) via native/ds4/download_model.sh ${profile.downloadTarget}`)
  if (dryRun) return ggufPath
  await runStreaming('sh', [layout.downloadScript, profile.downloadTarget], { cwd: layout.ds4Dir })
  if (!existsSync(ggufPath)) fail(`download finished but ${ggufPath} not found`)
  return ggufPath
}

// ---------- LaunchAgent ----------

function plistXML({ dsgoBin, ds4ServerBin, ggufPath, dsgoPort, ds4Port, ctx }) {
  const args = [
    dsgoBin,
    '--host', '127.0.0.1',
    '--port', String(dsgoPort),
    '--spawn-ds4',
    '--ds4-bin', ds4ServerBin,
    '--ds4-port', String(ds4Port),
    '--ds4-model', ggufPath,
    '--ds4-arg', `--ctx`,
    '--ds4-arg', String(ctx),
  ]
  const escape = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  const argTags = args.map((a) => `    <string>${escape(a)}</string>`).join('\n')
  return `<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>${LAUNCHD_LABEL}</string>
  <key>ProgramArguments</key>
  <array>
${argTags}
  </array>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>${escape(join(LOG_DIR, 'dsgo.out.log'))}</string>
  <key>StandardErrorPath</key>
  <string>${escape(join(LOG_DIR, 'dsgo.err.log'))}</string>
  <key>ProcessType</key>
  <string>Interactive</string>
</dict>
</plist>
`
}

function gui(uid) {
  return `gui/${uid}`
}

async function writeAndLoadAgent({ dsgoBin, ds4ServerBin, ggufPath, dsgoPort, ds4Port, ctx, dryRun }) {
  await mkdir(LOG_DIR, { recursive: true })
  await mkdir(dirname(LAUNCHD_PLIST), { recursive: true })
  const xml = plistXML({ dsgoBin, ds4ServerBin, ggufPath, dsgoPort, ds4Port, ctx })
  if (dryRun) {
    logLine(`would write ${LAUNCHD_PLIST}`)
    return
  }
  await writeFile(LAUNCHD_PLIST, xml, 'utf8')
  logOk(`wrote ${LAUNCHD_PLIST}`)
  const uid = process.getuid?.() ?? 0
  // Idempotent reload: bootout (ignore failure if not loaded), then bootstrap.
  spawnSync('/bin/launchctl', ['bootout', gui(uid), LAUNCHD_PLIST], { stdio: 'ignore' })
  const r = spawnSync('/bin/launchctl', ['bootstrap', gui(uid), LAUNCHD_PLIST], { encoding: 'utf8' })
  if (r.status !== 0) {
    fail(`launchctl bootstrap failed (${r.status}): ${r.stderr || r.stdout}`)
  }
  logOk(`launchd: ${LAUNCHD_LABEL} bootstrapped into ${gui(uid)}`)
}

async function bootoutAgent({ dryRun }) {
  if (!existsSync(LAUNCHD_PLIST)) {
    logWarn(`no LaunchAgent installed at ${LAUNCHD_PLIST}`)
    return
  }
  if (dryRun) {
    logLine(`would launchctl bootout ${LAUNCHD_LABEL}`)
    return
  }
  const uid = process.getuid?.() ?? 0
  const r = spawnSync('/bin/launchctl', ['bootout', gui(uid), LAUNCHD_PLIST], { encoding: 'utf8' })
  if (r.status !== 0 && !/No such process/i.test(r.stderr || '')) {
    logWarn(`bootout exit ${r.status}: ${(r.stderr || '').trim()}`)
  } else {
    logOk(`launchd: ${LAUNCHD_LABEL} booted out`)
  }
}

// ---------- HTTP helpers (Node-builtin, no fetch dep) ----------

function httpJSON({ host, port, path, body, headers = {}, timeoutMs = 10_000 }) {
  return new Promise((resolveP, rejectP) => {
    const payload = body == null ? undefined : Buffer.from(JSON.stringify(body))
    const req = httpRequest(
      {
        host,
        port,
        path,
        method: body == null ? 'GET' : 'POST',
        headers: {
          'content-type': 'application/json',
          ...(payload ? { 'content-length': payload.length } : {}),
          ...headers,
        },
      },
      (res) => {
        const chunks = []
        res.on('data', (c) => chunks.push(c))
        res.on('end', () => {
          const text = Buffer.concat(chunks).toString('utf8')
          let parsed
          try { parsed = text ? JSON.parse(text) : null } catch { parsed = text }
          resolveP({ status: res.statusCode, body: parsed })
        })
      }
    )
    req.setTimeout(timeoutMs, () => req.destroy(new Error(`http timeout after ${timeoutMs}ms`)))
    req.on('error', rejectP)
    if (payload) req.write(payload)
    req.end()
  })
}

function httpStreamSSE({ host, port, path, body, headers = {}, onEvent, timeoutMs = 60_000 }) {
  return new Promise((resolveP, rejectP) => {
    const payload = Buffer.from(JSON.stringify(body))
    const req = httpRequest(
      {
        host,
        port,
        path,
        method: 'POST',
        headers: {
          'content-type': 'application/json',
          'accept': 'text/event-stream',
          'content-length': payload.length,
          ...headers,
        },
      },
      (res) => {
        if (res.statusCode !== 200) {
          const chunks = []
          res.on('data', (c) => chunks.push(c))
          res.on('end', () => rejectP(new Error(`upstream HTTP ${res.statusCode}: ${Buffer.concat(chunks).toString('utf8').slice(0, 500)}`)))
          return
        }
        let buf = ''
        res.setEncoding('utf8')
        res.on('data', (chunk) => {
          buf += chunk
          let idx
          while ((idx = buf.indexOf('\n\n')) >= 0) {
            const raw = buf.slice(0, idx)
            buf = buf.slice(idx + 2)
            let event = 'message'
            let data = ''
            for (const line of raw.split('\n')) {
              if (line.startsWith('event:')) event = line.slice(6).trim()
              else if (line.startsWith('data:')) data += (data ? '\n' : '') + line.slice(5).trim()
            }
            if (!data) continue
            try { onEvent({ event, data: JSON.parse(data) }) } catch (e) { onEvent({ event, data, parseError: e }) }
          }
        })
        res.on('end', () => resolveP())
      }
    )
    req.setTimeout(timeoutMs, () => req.destroy(new Error(`SSE timeout after ${timeoutMs}ms`)))
    req.on('error', rejectP)
    req.write(payload)
    req.end()
  })
}

// ---------- readiness + self-test ----------

async function waitForReady({ host, port, timeoutMs = 30_000 }) {
  const deadline = Date.now() + timeoutMs
  let lastErr = null
  while (Date.now() < deadline) {
    try {
      const r = await httpJSON({ host, port, path: '/v1/models', timeoutMs: 2_000 })
      if (r.status && r.status < 500) return r
    } catch (e) { lastErr = e }
    await new Promise((r) => setTimeout(r, 500))
  }
  throw new Error(`gateway not ready on ${host}:${port} after ${timeoutMs}ms${lastErr ? `: ${lastErr.message}` : ''}`)
}

async function selfTest({ host, port, profile }) {
  logLine(`POST http://${host}:${port}/v1/messages stream=true tools=[Echo]`)
  let sawToolUse = false
  let sawToolUseArgs = ''
  let sawText = ''
  let stopReason = null
  await httpStreamSSE({
    host,
    port,
    path: '/v1/messages',
    headers: { 'anthropic-version': '2023-06-01' },
    body: {
      model: profile.canonicalId,
      max_tokens: 200,
      stream: true,
      tools: [
        {
          name: 'Echo',
          description: 'Echo the given text back exactly.',
          input_schema: {
            type: 'object',
            properties: { text: { type: 'string', description: 'text to echo' } },
            required: ['text'],
          },
        },
      ],
      messages: [
        { role: 'user', content: "Call the Echo tool exactly once with text='deeptide local ok'. Do not output anything else." },
      ],
    },
    onEvent: ({ event, data }) => {
      if (event === 'content_block_start' && data?.content_block?.type === 'tool_use') sawToolUse = true
      if (event === 'content_block_delta' && data?.delta?.type === 'input_json_delta') sawToolUseArgs += data.delta.partial_json || ''
      if (event === 'content_block_delta' && data?.delta?.type === 'text_delta') sawText += data.delta.text || ''
      if (event === 'message_delta' && data?.delta?.stop_reason) stopReason = data.delta.stop_reason
    },
  })
  if (!sawToolUse) {
    throw new Error(`no tool_use content block in stream. stop_reason=${stopReason ?? '?'} text=${JSON.stringify(sawText.slice(0, 120))}`)
  }
  logOk(`tool_use streamed (args: ${sawToolUseArgs.slice(0, 80) || '?'}…, stop_reason=${stopReason ?? '?'})`)
}

// ---------- settings ----------

async function writeAgentSettings({ dsgoPort, profile, ctx, dryRun }) {
  await mkdir(STATE_DIR, { recursive: true })
  let prev = {}
  if (existsSync(SETTINGS_PATH)) {
    try { prev = JSON.parse(await readFile(SETTINGS_PATH, 'utf8')) } catch { prev = {} }
  }
  const merged = {
    ...prev,
    base_url: `http://127.0.0.1:${dsgoPort}`,
    model: profile.canonicalId,
    local: {
      ...(prev.local || {}),
      profile: profile.key,
      context_window: ctx,
      gateway_port: dsgoPort,
    },
  }
  if (dryRun) {
    logLine(`would merge into ${SETTINGS_PATH}: model=${profile.canonicalId} base_url=http://127.0.0.1:${dsgoPort}`)
    return
  }
  const tmp = `${SETTINGS_PATH}.tmp`
  await writeFile(tmp, JSON.stringify(merged, null, 2), 'utf8')
  await rename(tmp, SETTINGS_PATH)
  logOk(`merged into ${SETTINGS_PATH}`)
}

async function writeLocalState({ profile, ctx, ggufPath, dsgoPort, ds4Port, dryRun }) {
  if (dryRun) return
  await mkdir(STATE_DIR, { recursive: true })
  const state = {
    profile: profile.key,
    canonicalId: profile.canonicalId,
    contextWindow: ctx,
    ggufPath,
    dsgoPort,
    ds4Port,
    label: LAUNCHD_LABEL,
    updatedAt: new Date().toISOString(),
  }
  await writeFile(LOCAL_STATE_PATH, JSON.stringify(state, null, 2), 'utf8')
}

// ---------- subcommand orchestrators ----------

async function cmdStart(args) {
  if (osPlatform() !== 'darwin') {
    fail(`tide local currently requires macOS (Apple Silicon). detected platform: ${osPlatform()}. The cross-platform path is on the roadmap; for now use a cloud DeepSeek API key.`)
  }
  const layout = repoLayout()
  if (!layout) {
    fail(`tide local needs the native/ source tree. The npm package ships only the CLI wrapper. Clone https://github.com/paean-ai/deeptide and run \`tide local\` from inside the repo.`)
  }
  const host = probeHost()
  if (!host.isAppleSilicon) {
    fail(`tide local requires Apple Silicon. detected arch: ${host.arch}. Intel Macs cannot run the ds4 Metal engine.`)
  }
  const profile = args.model ? resolveProfile(args.model) : autoPickProfile(host)
  if (!profile) {
    fail(`host has ${host.totalRAMGiB} GB RAM, below the minimum for any local V4 Flash profile (${PROFILES.flash.minHostRAMGiB} GB). See docs/deepseek-v4-flash-q2.md.`)
  }
  if (host.totalRAMGiB < profile.minHostRAMGiB) {
    logWarn(`profile ${profile.key} expects ≥${profile.minHostRAMGiB} GB RAM, host has ${host.totalRAMGiB} GB — model may swap. consider --model flash (Q2) instead.`)
  }
  const ctx = args.ctx ?? profile.contextWindow

  const total = 6
  logStep(1, total, 'Host check')
  logLine(describeHost(host))

  logStep(2, total, 'Quant plan')
  logLine(`→ ${bold(profile.canonicalId)} · ${profile.notes}`)
  logLine(`weights: ${profile.ggufFile} (~${profile.sizeGiB} GiB)`)
  logLine(`context window: ${ctx.toLocaleString()} tokens`)
  if (!args.yes && !args.dryRun) {
    process.stdout.write(`      Continue? [Y/n] `)
    const yn = await readOneLine()
    if (yn && !/^y(es)?$/i.test(yn.trim())) {
      process.stdout.write('aborted.\n')
      process.exit(0)
    }
  }

  logStep(3, total, 'Build native runtime')
  if (args.noBuild) logLine('skipped (--no-build)')
  else await ensureNativeBuilt(layout, { dryRun: args.dryRun })

  logStep(4, total, 'Download weights')
  let ggufPath = join(layout.ggufDir, profile.ggufFile)
  if (args.noDownload) {
    if (!existsSync(ggufPath)) fail(`--no-download set but ${ggufPath} not present`)
    logLine(`skipped (--no-download); using ${ggufPath}`)
  } else {
    ggufPath = await ensureWeightsDownloaded(layout, profile, { dryRun: args.dryRun })
  }

  logStep(5, total, `Start services (LaunchAgent ${LAUNCHD_LABEL})`)
  if (args.noService) {
    logLine('skipped (--no-service)')
  } else {
    await writeAndLoadAgent({
      dsgoBin: layout.dsgoBin,
      ds4ServerBin: layout.ds4ServerBin,
      ggufPath,
      dsgoPort: args.dsgoPort,
      ds4Port: args.ds4Port,
      ctx,
      dryRun: args.dryRun,
    })
    if (!args.dryRun) {
      try {
        await waitForReady({ host: '127.0.0.1', port: args.dsgoPort, timeoutMs: 60_000 })
        logOk(`dsgo gateway ready at http://127.0.0.1:${args.dsgoPort}`)
      } catch (e) {
        logWarn(e.message)
      }
    }
  }

  logStep(6, total, 'Self-test (streaming tool_use round-trip)')
  if (args.noSelfTest || args.dryRun) {
    logLine(args.dryRun ? 'skipped (--dry-run)' : 'skipped (--no-self-test)')
  } else {
    try {
      await selfTest({ host: '127.0.0.1', port: args.dsgoPort, profile })
    } catch (e) {
      logWarn(`self-test failed: ${e.message}`)
      logWarn(`see logs: ${join(LOG_DIR, 'dsgo.err.log')}, ${join(LOG_DIR, 'dsgo.out.log')}`)
      process.exitCode = 2
    }
  }

  if (!args.noSettings) {
    await writeAgentSettings({ dsgoPort: args.dsgoPort, profile, ctx, dryRun: args.dryRun })
  }
  await writeLocalState({ profile, ctx, ggufPath, dsgoPort: args.dsgoPort, ds4Port: args.ds4Port, dryRun: args.dryRun })

  if (!args.dryRun) {
    process.stdout.write(
      `\n${green('Ready.')} ${profile.canonicalId} · local · ${host.totalRAMGiB} GB · tools ${process.exitCode === 2 ? red('✗') : green('✓')}\n` +
        `Run ${bold('tide')} to start an agent session.\n`
    )
  }
}

async function cmdStop(args) {
  await bootoutAgent({ dryRun: args.dryRun })
}

async function cmdStatus(args) {
  const installed = existsSync(LAUNCHD_PLIST)
  process.stdout.write(`LaunchAgent plist: ${installed ? LAUNCHD_PLIST : red('not installed')}\n`)
  let state = null
  if (existsSync(LOCAL_STATE_PATH)) {
    try { state = JSON.parse(await readFile(LOCAL_STATE_PATH, 'utf8')) } catch {}
  }
  if (state) {
    process.stdout.write(`profile: ${state.profile} (${state.canonicalId})\n`)
    process.stdout.write(`gguf: ${state.ggufPath}\n`)
    process.stdout.write(`context: ${state.contextWindow}\n`)
    process.stdout.write(`dsgo port: ${state.dsgoPort}\n`)
    process.stdout.write(`ds4 port: ${state.ds4Port}\n`)
  }
  if (installed) {
    const uid = process.getuid?.() ?? 0
    const r = spawnSync('/bin/launchctl', ['print', `${gui(uid)}/${LAUNCHD_LABEL}`], { encoding: 'utf8' })
    process.stdout.write(`\nlaunchctl print:\n${r.stdout || r.stderr || '(no output)'}\n`)
  }
  if (state?.dsgoPort) {
    try {
      const r = await httpJSON({ host: '127.0.0.1', port: state.dsgoPort, path: '/v1/models', timeoutMs: 2_000 })
      process.stdout.write(`\ngateway: ${green('reachable')} (HTTP ${r.status})\n`)
    } catch (e) {
      process.stdout.write(`\ngateway: ${red('unreachable')} — ${e.message}\n`)
    }
  }
  process.stdout.write(`\nlogs:\n  ${join(LOG_DIR, 'dsgo.out.log')}\n  ${join(LOG_DIR, 'dsgo.err.log')}\n`)
}

async function cmdSelfTest(args) {
  let state = null
  if (existsSync(LOCAL_STATE_PATH)) {
    try { state = JSON.parse(await readFile(LOCAL_STATE_PATH, 'utf8')) } catch {}
  }
  const port = args.dsgoPort || state?.dsgoPort || DSGO_DEFAULT_PORT
  const profileKey = args.model || state?.profile || 'flash'
  const profile = resolveProfile(profileKey)
  logStep(1, 1, `Self-test against http://127.0.0.1:${port}`)
  await selfTest({ host: '127.0.0.1', port, profile })
}

async function cmdDoctor() {
  const host = probeHost()
  process.stdout.write(`host: ${describeHost(host)} · ${host.platform}/${host.arch}\n`)
  const layout = repoLayout()
  process.stdout.write(`repo layout: ${layout ? 'found native/ in ' + layout.root : red('NOT FOUND') + ' — run from a cloned source repo'}\n`)
  if (layout) {
    process.stdout.write(`  ds4-server: ${existsSync(layout.ds4ServerBin) ? green('built') : yellow('not built')}\n`)
    process.stdout.write(`  dsgo: ${existsSync(layout.dsgoBin) ? green('built') : yellow('not built')}\n`)
  }
  process.stdout.write(`xcode tools: make=${which('make') ? green('ok') : red('missing')} swift=${which('swift') ? green('ok') : red('missing')}\n`)
  const auto = autoPickProfile(host)
  process.stdout.write(`auto-pick: ${auto ? `${auto.canonicalId} (${auto.key})` : red(`no profile for ${host.totalRAMGiB} GB host`)}\n`)
  process.stdout.write(`launchd plist: ${existsSync(LAUNCHD_PLIST) ? LAUNCHD_PLIST : yellow('not installed')}\n`)
  process.stdout.write(`settings: ${existsSync(SETTINGS_PATH) ? SETTINGS_PATH : yellow('absent')}\n`)
}

// ---------- tiny stdin readLine ----------

function readOneLine() {
  return new Promise((resolveP) => {
    let buf = ''
    const onData = (d) => {
      buf += d.toString('utf8')
      const nl = buf.indexOf('\n')
      if (nl >= 0) {
        process.stdin.off('data', onData)
        process.stdin.pause()
        resolveP(buf.slice(0, nl))
      }
    }
    process.stdin.resume()
    process.stdin.on('data', onData)
  })
}

// ---------- entrypoint ----------

export async function run(argv) {
  const args = parseArgs(argv)
  if (args.help) { usage(); return }
  switch (args.sub) {
    case 'start':
      await cmdStart(args)
      break
    case 'stop':
      await cmdStop(args)
      break
    case 'status':
      await cmdStatus(args)
      break
    case 'self-test':
      await cmdSelfTest(args)
      break
    case 'doctor':
      await cmdDoctor()
      break
    default:
      fail(`unknown subcommand: ${args.sub} (try 'tide local --help')`)
  }
}

// allow direct invocation: `node scripts/tide-local.mjs ...`
if (import.meta.url === `file://${process.argv[1]}`) {
  run(process.argv.slice(2)).catch((e) => {
    process.stderr.write(`${red('error:')} ${e.message}\n`)
    process.exit(1)
  })
}
