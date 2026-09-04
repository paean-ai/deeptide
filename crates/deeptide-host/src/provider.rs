//! Provider presets — the user-facing front door to multi-protocol support.
//!
//! A *provider* bundles three decisions that otherwise the user would have to
//! wire by hand: which wire **protocol** to speak (`anthropic` vs the OpenAI
//! `/v1/chat/completions` dialect), a sensible default **base URL**, and a
//! default **model**. `--provider deepseek` or `--provider ollama` is far less
//! error-prone than remembering DeepSeek's Anthropic-proxy URL or ollama's
//! `:11434/v1` endpoint.
//!
//! Resolution is intentionally pure and override-friendly: a preset only
//! *supplies defaults*. An explicit `--base-url` / `--model` / `--api-key` (or
//! their env vars) always wins, so a preset never boxes the user in. Unknown
//! provider names resolve to a custom OpenAI-compatible endpoint, which is the
//! right default for the long tail of local servers and relays.

/// The wire protocol a backend speaks. The agent loop is identical regardless;
/// this only selects how requests/responses are shaped on the socket. The enum
/// is the extension point for "other protocol" support — adding a variant +
/// a backend is all a new protocol needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Anthropic `/v1/messages` (the original backend).
    Anthropic,
    /// OpenAI-compatible `/v1/chat/completions` — DeepSeek, ollama, LM Studio,
    /// vLLM, llama.cpp, mlx-lm, OpenAI itself, and most relays.
    OpenAi,
    /// Google Gemini `generateContent` / `streamGenerateContent`.
    Gemini,
}

/// A resolved provider preset: the protocol plus default endpoint/model. The
/// `requires_key` flag drives whether a missing credential is an error (cloud)
/// or fine (keyless local servers like ollama / a bare llama.cpp `server`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderPreset {
    /// Canonical lowercase id (e.g. `"deepseek"`, `"ollama"`, `"custom"`).
    pub id: String,
    pub protocol: Protocol,
    /// Default base URL when the user doesn't override it.
    pub base_url: String,
    /// Default model when the user doesn't override it.
    pub model: String,
    /// Whether a credential is mandatory. Local engines set this `false` so a
    /// keyless run still works.
    pub requires_key: bool,
    /// One-line human label for `--list-providers` / status display.
    pub label: String,
}

impl ProviderPreset {
    fn new(
        id: &str,
        protocol: Protocol,
        base_url: &str,
        model: &str,
        requires_key: bool,
        label: &str,
    ) -> Self {
        Self {
            id: id.to_owned(),
            protocol,
            base_url: base_url.to_owned(),
            model: model.to_owned(),
            requires_key,
            label: label.to_owned(),
        }
    }
}

/// Resolve a `--provider` value into a preset. Case-insensitive; surrounding
/// whitespace ignored. An unrecognized non-empty name becomes a `custom`
/// OpenAI-compatible provider pointed at localhost — the safe default for an
/// arbitrary local inference server — so users can do
/// `--provider whatever --base-url http://host:port` without us gatekeeping the
/// name.
pub fn resolve_provider(name: &str) -> ProviderPreset {
    match name.trim().to_ascii_lowercase().as_str() {
        // ─── Cloud, Anthropic protocol ──────────────────────────────────────
        "" | "anthropic" | "claude" => ProviderPreset::new(
            "anthropic",
            Protocol::Anthropic,
            "https://api.anthropic.com",
            "claude-sonnet-4-6",
            true,
            "Anthropic (Claude)",
        ),

        // ─── Cloud, OpenAI protocol ─────────────────────────────────────────
        "openai" | "gpt" => ProviderPreset::new(
            "openai",
            Protocol::OpenAi,
            "https://api.openai.com/v1",
            "gpt-4o",
            true,
            "OpenAI",
        ),
        // DeepSeek's native endpoint speaks the OpenAI protocol. (It also
        // offers an Anthropic-proxy at /anthropic, but native is the first-
        // class path with full usage/cache reporting.)
        "deepseek" => ProviderPreset::new(
            "deepseek",
            Protocol::OpenAi,
            "https://api.deepseek.com/v1",
            "deepseek-chat",
            true,
            "DeepSeek (official)",
        ),
        "moonshot" | "kimi" => ProviderPreset::new(
            "moonshot",
            Protocol::OpenAi,
            "https://api.moonshot.cn/v1",
            "kimi-k2-0711-preview",
            true,
            "Moonshot (Kimi)",
        ),
        "zhipu" | "glm" | "bigmodel" => ProviderPreset::new(
            "zhipu",
            Protocol::OpenAi,
            "https://open.bigmodel.cn/api/paas/v4",
            "glm-4-plus",
            true,
            "Zhipu (GLM)",
        ),
        "openrouter" => ProviderPreset::new(
            "openrouter",
            Protocol::OpenAi,
            "https://openrouter.ai/api/v1",
            "deepseek/deepseek-chat",
            true,
            "OpenRouter",
        ),
        "atlascloud" | "atlas" => ProviderPreset::new(
            "atlascloud",
            Protocol::OpenAi,
            "https://api.atlascloud.ai/v1",
            "deepseek-ai/deepseek-v4-pro",
            true,
            "Atlas Cloud",
        ),
        "groq" => ProviderPreset::new(
            "groq",
            Protocol::OpenAi,
            "https://api.groq.com/openai/v1",
            "llama-3.3-70b-versatile",
            true,
            "Groq",
        ),

        // ─── Cloud, Gemini protocol ─────────────────────────────────────────
        "gemini" | "google" => ProviderPreset::new(
            "gemini",
            Protocol::Gemini,
            "https://generativelanguage.googleapis.com",
            "gemini-2.0-flash",
            true,
            "Google Gemini",
        ),

        // ─── Local inference engines, OpenAI protocol, keyless ──────────────
        "ollama" => ProviderPreset::new(
            "ollama",
            Protocol::OpenAi,
            "http://localhost:11434/v1",
            "qwen2.5-coder:7b",
            false,
            "Ollama (local)",
        ),
        "lmstudio" | "lm-studio" => ProviderPreset::new(
            "lmstudio",
            Protocol::OpenAi,
            "http://localhost:1234/v1",
            "local-model",
            false,
            "LM Studio (local)",
        ),
        // vLLM / llama.cpp `server` (`llama-server`) / generic OpenAI server,
        // commonly :8000. The bare `llama` / `llama-server` aliases land here so
        // a user who just types `--provider llama` gets a working endpoint.
        "vllm" | "llamacpp" | "llama.cpp" | "llama-cpp" | "llama" | "llama-server" => {
            ProviderPreset::new(
                "vllm",
                Protocol::OpenAi,
                "http://localhost:8000/v1",
                "local-model",
                false,
                "vLLM / llama.cpp (local)",
            )
        }
        // mlx-lm's OpenAI server (Apple Silicon), commonly :8080.
        "mlx" | "mlx-lm" | "omlx" => ProviderPreset::new(
            "mlx",
            Protocol::OpenAi,
            "http://localhost:8080/v1",
            "local-model",
            false,
            "MLX (local, Apple Silicon)",
        ),
        // llamafile — single-file llama.cpp build, OpenAI server on :8080.
        "llamafile" => ProviderPreset::new(
            "llamafile",
            Protocol::OpenAi,
            "http://localhost:8080/v1",
            "local-model",
            false,
            "llamafile (local)",
        ),
        // KoboldCpp's OpenAI-compatible API, default :5001.
        "koboldcpp" | "kobold" => ProviderPreset::new(
            "koboldcpp",
            Protocol::OpenAi,
            "http://localhost:5001/v1",
            "local-model",
            false,
            "KoboldCpp (local)",
        ),
        // LocalAI — drop-in OpenAI replacement, default :8080.
        "localai" => ProviderPreset::new(
            "localai",
            Protocol::OpenAi,
            "http://localhost:8080/v1",
            "local-model",
            false,
            "LocalAI (local)",
        ),
        // Jan's local API server, default :1337.
        "jan" => ProviderPreset::new(
            "jan",
            Protocol::OpenAi,
            "http://localhost:1337/v1",
            "local-model",
            false,
            "Jan (local)",
        ),
        // text-generation-webui (oobabooga) OpenAI extension, default :5000.
        "oobabooga" | "textgen" | "text-generation-webui" => ProviderPreset::new(
            "oobabooga",
            Protocol::OpenAi,
            "http://localhost:5000/v1",
            "local-model",
            false,
            "text-generation-webui (local)",
        ),
        // GPT4All's local API server, default :4891.
        "gpt4all" => ProviderPreset::new(
            "gpt4all",
            Protocol::OpenAi,
            "http://localhost:4891/v1",
            "local-model",
            false,
            "GPT4All (local)",
        ),

        // ─── Unknown → custom OpenAI-compatible localhost endpoint ──────────
        other => ProviderPreset::new(
            other,
            Protocol::OpenAi,
            "http://localhost:8000/v1",
            "local-model",
            false,
            "Custom (OpenAI-compatible)",
        ),
    }
}

/// The catalog of named presets for `--list-providers`, in display order.
/// (The `custom` fallback is intentionally omitted — it's the catch-all, not a
/// named entry.)
pub fn known_provider_ids() -> &'static [&'static str] {
    &[
        "anthropic",
        "openai",
        "deepseek",
        "moonshot",
        "zhipu",
        "openrouter",
        "atlascloud",
        "groq",
        "gemini",
        "ollama",
        "lmstudio",
        "vllm",
        "mlx",
        "llamafile",
        "koboldcpp",
        "localai",
        "jan",
        "oobabooga",
        "gpt4all",
    ]
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_anthropic_aliases_resolve_to_anthropic_protocol() {
        for name in ["", "anthropic", "Claude", "  ANTHROPIC  "] {
            let p = resolve_provider(name);
            assert_eq!(p.protocol, Protocol::Anthropic, "name={name:?}");
            assert_eq!(p.id, "anthropic");
            assert!(p.requires_key);
        }
    }

    #[test]
    fn deepseek_resolves_to_native_openai_endpoint() {
        let p = resolve_provider("deepseek");
        assert_eq!(p.protocol, Protocol::OpenAi);
        assert_eq!(p.base_url, "https://api.deepseek.com/v1");
        assert_eq!(p.model, "deepseek-chat");
        assert!(p.requires_key);
    }

    #[test]
    fn atlascloud_resolves_to_openai_compatible_endpoint() {
        for name in ["atlascloud", "atlas", "  AtlasCloud  "] {
            let p = resolve_provider(name);
            assert_eq!(p.id, "atlascloud", "name={name:?}");
            assert_eq!(p.protocol, Protocol::OpenAi);
            assert_eq!(p.base_url, "https://api.atlascloud.ai/v1");
            assert_eq!(p.model, "deepseek-ai/deepseek-v4-pro");
            assert!(p.requires_key);
        }
    }

    #[test]
    fn local_engines_are_openai_protocol_and_keyless() {
        for name in [
            "ollama",
            "lmstudio",
            "lm-studio",
            "vllm",
            "llama.cpp",
            "llama",
            "llama-server",
            "mlx",
            "omlx",
            "llamafile",
            "koboldcpp",
            "kobold",
            "localai",
            "jan",
            "oobabooga",
            "textgen",
            "gpt4all",
        ] {
            let p = resolve_provider(name);
            assert_eq!(p.protocol, Protocol::OpenAi, "name={name}");
            assert!(!p.requires_key, "local provider {name} must be keyless");
            assert!(
                p.base_url.starts_with("http://localhost"),
                "local provider {name} should default to localhost: {}",
                p.base_url
            );
        }
    }

    #[test]
    fn bare_llama_alias_resolves_to_the_llama_cpp_preset() {
        // The user-facing name people reach for is just "llama" — it must not
        // fall through to the generic `custom` catch-all.
        let p = resolve_provider("llama");
        assert_eq!(p.id, "vllm");
        assert_eq!(p.base_url, "http://localhost:8000/v1");
    }

    #[test]
    fn newly_added_local_engines_use_their_documented_default_ports() {
        // Each common local server listens on its own well-known port; a wrong
        // default would silently send requests into the void. Pin them.
        let cases = [
            ("llamafile", "http://localhost:8080/v1"),
            ("koboldcpp", "http://localhost:5001/v1"),
            ("localai", "http://localhost:8080/v1"),
            ("jan", "http://localhost:1337/v1"),
            ("oobabooga", "http://localhost:5000/v1"),
            ("gpt4all", "http://localhost:4891/v1"),
        ];
        for (name, expected_url) in cases {
            let p = resolve_provider(name);
            assert_eq!(p.base_url, expected_url, "wrong default port for {name}");
            assert_eq!(p.model, "local-model", "{name} default model");
        }
        // Aliases collapse to a canonical id.
        assert_eq!(resolve_provider("kobold").id, "koboldcpp");
        assert_eq!(resolve_provider("text-generation-webui").id, "oobabooga");
    }

    #[test]
    fn case_and_whitespace_are_normalized() {
        assert_eq!(resolve_provider("  DeepSeek ").id, "deepseek");
        assert_eq!(resolve_provider("OLLAMA").id, "ollama");
    }

    #[test]
    fn unknown_name_falls_back_to_custom_openai_localhost() {
        let p = resolve_provider("my-weird-server");
        assert_eq!(p.protocol, Protocol::OpenAi);
        assert_eq!(p.id, "my-weird-server");
        assert!(!p.requires_key);
        assert!(p.base_url.starts_with("http://localhost"));
    }

    #[test]
    fn aliases_collapse_to_canonical_ids() {
        assert_eq!(resolve_provider("gpt").id, "openai");
        assert_eq!(resolve_provider("kimi").id, "moonshot");
        assert_eq!(resolve_provider("glm").id, "zhipu");
        assert_eq!(resolve_provider("atlas").id, "atlascloud");
        assert_eq!(resolve_provider("google").id, "gemini");
    }

    #[test]
    fn gemini_resolves_to_gemini_protocol() {
        let p = resolve_provider("gemini");
        assert_eq!(p.protocol, Protocol::Gemini);
        assert_eq!(p.base_url, "https://generativelanguage.googleapis.com");
        assert!(p.model.starts_with("gemini-"));
        assert!(p.requires_key);
    }

    #[test]
    fn every_known_id_resolves_to_itself() {
        for id in known_provider_ids() {
            let p = resolve_provider(id);
            assert_eq!(&p.id, id, "{id} must resolve to its own canonical id");
        }
    }
}
