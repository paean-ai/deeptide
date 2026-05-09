# dsgo

`dsgo` is the Mac-native local gateway for DeepSeek inference. It is intended to
be the stable process that local agents and API clients talk to, while specialized
backends do the actual inference:

- `ds4-server` for DeepSeek V4 Flash on Apple Silicon / Metal.
- `llama-server` for earlier DeepSeek GGUF models and smaller variants.

The product goal is a low-friction local runtime: one command starts the gateway,
optionally starts bundled backends, and exposes OpenAI-compatible and
Anthropic-compatible HTTP APIs on one local port.

## API surface

`dsgo` currently serves:

- `GET /health`
- `GET /v1/models`
- `GET /v1/models/{id}`
- `POST /v1/chat/completions`
- `POST /v1/completions`
- `POST /v1/messages`
- `POST /v1/messages/count_tokens`

Generation requests are routed by the `model` field. Streaming requests are
passed through as live upstream bytes instead of being buffered until completion.

## Default model routing

Defaults are intentionally useful without a config file:

| Model | Backend |
| --- | --- |
| `deepseek-v4-flash` | `ds4` |
| `ds4/deepseek-v4-flash` | `ds4` |
| `deepseek-reasoner` | `ds4` |
| `deepseek-chat` | `ds4` |
| `deepseek-coder-v2-lite` | `llama` |
| `llama/deepseek-coder-v2-lite` | `llama` |

Unknown model names route to `llama` unless they use the `ds4/` prefix.

## Quick start

Run as a gateway to already-running backends:

```sh
swift run dsgo \
  --ds4-upstream http://127.0.0.1:8000 \
  --llama-upstream http://127.0.0.1:18081
```

Run and spawn both backends:

```sh
swift run dsgo \
  --spawn-ds4 --ds4-bin /path/to/ds4-server --ds4-model /path/to/ds4flash.gguf \
  --ds4-arg --ctx --ds4-arg 100000 \
  --ds4-arg --kv-disk-dir --ds4-arg /tmp/dsgo-ds4-kv \
  --spawn-llama --llama-bin /path/to/llama-server \
  --llama-model ./models/DeepSeek-Coder-V2-Lite-Instruct.Q4_K_M.gguf
```

Use it as an OpenAI-compatible provider:

```sh
curl http://127.0.0.1:38442/v1/chat/completions \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer dsgo-local' \
  -d '{
    "model": "deepseek-v4-flash",
    "messages": [{"role": "user", "content": "Explain prefix KV caching briefly."}],
    "stream": true
  }'
```

Use it as an Anthropic-compatible provider:

```sh
export ANTHROPIC_BASE_URL=http://127.0.0.1:38442
export ANTHROPIC_AUTH_TOKEN=dsgo-local
export ANTHROPIC_MODEL=deepseek-v4-flash
```

## Config file

`--config dsgo.json` keeps installation and agent setup simple:

```json
{
  "listen": {
    "host": "127.0.0.1",
    "port": 38442
  },
  "upstreams": {
    "ds4": "http://127.0.0.1:8000",
    "llama": "http://127.0.0.1:18081"
  },
  "models": [
    {
      "id": "deepseek-v4-flash",
      "backend": "ds4",
      "aliases": ["ds4/deepseek-v4-flash", "deepseek-reasoner", "deepseek-chat"],
      "contextWindow": 100000,
      "maxOutputTokens": 384000,
      "reasoning": true
    },
    {
      "id": "deepseek-coder-v2-lite",
      "backend": "llama",
      "aliases": ["llama/deepseek-coder-v2-lite"],
      "contextWindow": 32768,
      "maxOutputTokens": 16384,
      "reasoning": false
    }
  ]
}
```

## Distribution direction

The preferred distribution shape is:

```text
dsgo/
  dsgo
  ds4-server
  llama-server
  models/
  examples/dsgo.json
```

When backend binaries are placed beside `dsgo`, `--spawn-ds4` and
`--spawn-llama` find them without PATH changes. This keeps the user-facing setup
close to: download bundle, put model files in `models/`, run one command.

## Notes

The current `Network.framework` HTTP listener is stable for local development,
but exact `--host` binding is not implemented yet; non-`0.0.0.0` host values are
reported in logs and should be treated as product intent until the listener is
moved to a POSIX socket acceptor.
