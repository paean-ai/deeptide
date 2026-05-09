# Native DeepTide Runtime

This directory contains the open-source native runtime components that can be
used with DeepTide and other local clients:

- `ds4/`: a hard-forked DeepSeek V4 Flash specific Metal inference engine.
- `dsgo/`: the local gateway that exposes OpenAI-compatible and
  Anthropic-compatible HTTP APIs while routing requests to `ds4-server` and
  `llama-server`.

The npm package remains a thin CLI wrapper. Native sources are kept in this
repository for source distribution, review, local builds, and future release
bundles, but they are not included in the published npm tarball because
`package.json` uses an explicit `files` allowlist.

## Build

Build both native components:

```sh
npm run build:native
```

Build individually:

```sh
npm run build:ds4
npm run build:dsgo
```

`ds4` is macOS/Apple Silicon/Metal oriented and expects Xcode command line
tools. It does not download model weights during build. Use
`native/ds4/download_model.sh` explicitly when you want to fetch supported
DeepSeek V4 Flash GGUFs.

`dsgo` is a SwiftPM executable:

```sh
swift build --package-path native/dsgo
```

## Runtime Shape

The intended local bundle layout is:

```text
dsgo/
  dsgo
  ds4-server
  llama-server
  models/
  examples/dsgo.json
```

`dsgo` is the process users and agents connect to. It can spawn colocated
`ds4-server` and `llama-server` binaries, or route to already-running backend
servers.

DeepTide's private agent harness is not part of this repository. The public
contract between the harness and the local runtime is the OpenAI/Anthropic HTTP
surface exposed by `dsgo`.
