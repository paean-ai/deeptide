//! Protocol-neutral backend assembly, shared by the CLI and the GUI.
//!
//! Given a [`BackendParams`] (resolved from clap flags by the CLI, or from
//! `settings.json` by the GUI), [`build_backend`] resolves the provider preset,
//! short-circuits to the local-echo backend when no credential is available for
//! a cloud provider, and otherwise constructs the right protocol backend
//! (Anthropic / OpenAI / Gemini) — installing the streaming handler and
//! interrupt flag. Both front-ends call this one path so they can never drift.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use deeptide_core::{
    AgentBackend, AnthropicBackend, AnthropicConfig, GeminiBackend, GeminiConfig, LocalEchoBackend,
    OpenAiBackend, OpenAiConfig, StreamingHandler, ThinkingConfig,
};

use crate::provider::{self, Protocol};

/// A resolved credential for a cloud provider. Local keyless engines (ollama,
/// llama.cpp, mlx…) use `None`.
#[derive(Debug, Clone)]
pub enum CloudCredential {
    /// Anthropic `x-api-key` / OpenAI+Gemini bearer-or-key string.
    ApiKey(String),
    /// Anthropic OAuth-style bearer token.
    BearerToken(String),
}

/// The fully-resolved, front-end-neutral inputs to [`build_backend`]. The CLI
/// populates this from clap flags + env + layered config; the GUI populates it
/// from the shared `ConfigStore`. Keeping it `Cli`-free is the whole point — it
/// is the seam that lets both front-ends share one assembly path.
#[derive(Debug, Clone)]
pub struct BackendParams {
    /// Provider preset id (e.g. `"anthropic"`, `"deepseek"`, `"ollama"`).
    pub provider: String,
    /// Explicit base-URL override (flag/env). `None` ⇒ the preset default.
    pub base_url_override: Option<String>,
    /// Explicit model override (flag/env). `None` ⇒ the preset default.
    pub model_override: Option<String>,
    /// Resolved credential, or `None` for keyless local engines.
    pub credential: Option<CloudCredential>,
    /// Output-token ceiling.
    pub max_output_tokens: usize,
    /// Whether to send Anthropic prompt-cache markers.
    pub enable_prompt_caching: bool,
    /// Whether the user asked for streaming (`--stream`). Streaming also turns
    /// on automatically whenever a streaming handler is installed.
    pub stream: bool,
    /// Fallback model id used when the primary is overloaded.
    pub fallback_model: Option<String>,
    /// Extended-thinking directive label (`low`/`medium`/`high`/…), already
    /// resolved from `--thinking` falling back to `--effort`. `ThinkingConfig`
    /// parsing happens here so the front-ends don't repeat it.
    pub thinking_label: Option<String>,
    /// Resolved system prompt to install on the backend, if any.
    pub system_prompt: Option<String>,
}

/// The result of assembling a backend: the boxed backend, the effective model
/// id, whether a real provider was configured (vs. the echo fallback), and the
/// protocol-tagged config used to spawn sub-agents on the same endpoint.
pub struct ConfiguredBackend {
    pub backend: Box<dyn AgentBackend>,
    pub model: String,
    pub is_configured: bool,
    pub subagent_config: Option<SubagentConfig>,
}

/// Protocol-tagged backend config used to spawn sub-agents (the `Agent` tool)
/// with the SAME protocol and endpoint as the main loop. Without this, a
/// session pointed at DeepSeek/ollama would silently spawn sub-agents against
/// Anthropic. The variant mirrors whichever backend the main loop is using.
#[derive(Clone)]
pub enum SubagentConfig {
    Anthropic(AnthropicConfig),
    OpenAi(OpenAiConfig),
    Gemini(GeminiConfig),
}

impl SubagentConfig {
    /// Borrow the Anthropic config when this session speaks the Anthropic
    /// protocol. Used by tests that assert on Anthropic-specific fields
    /// (thinking budget, etc.); returns `None` for other protocols.
    pub fn as_anthropic(&self) -> Option<&AnthropicConfig> {
        match self {
            SubagentConfig::Anthropic(config) => Some(config),
            SubagentConfig::OpenAi(_) | SubagentConfig::Gemini(_) => None,
        }
    }
}

fn echo_backend() -> ConfiguredBackend {
    ConfiguredBackend {
        backend: Box::<LocalEchoBackend>::default(),
        model: String::from("unconfigured"),
        is_configured: false,
        subagent_config: None,
    }
}

/// Build the configured backend from resolved [`BackendParams`], optionally
/// installing a streaming handler and interrupt flag.
///
/// When `streaming_handler` is `Some`, the backend is forced into streaming
/// mode regardless of `params.stream` (the handler is only useful when streaming
/// is active). A missing credential for a cloud (key-requiring) provider falls
/// back to the local-echo backend, exactly as the CLI did before.
pub fn build_backend(
    params: &BackendParams,
    streaming_handler: Option<StreamingHandler>,
    interrupt: Option<Arc<AtomicBool>>,
) -> Result<ConfiguredBackend, String> {
    let preset = provider::resolve_provider(&params.provider);

    // Keyless local engines (requires_key=false) are usable with no credential.
    // Cloud providers still require one — when absent, fall back to echo.
    if params.credential.is_none() && preset.requires_key {
        return Ok(echo_backend());
    }

    let base_url = params
        .base_url_override
        .clone()
        .unwrap_or_else(|| preset.base_url.clone());
    let model = params
        .model_override
        .clone()
        .unwrap_or_else(|| preset.model.clone());

    match preset.protocol {
        Protocol::Anthropic => {
            configure_anthropic_backend(params, base_url, model, streaming_handler, interrupt)
        }
        Protocol::OpenAi => {
            configure_openai_backend(params, base_url, model, streaming_handler, interrupt)
        }
        Protocol::Gemini => {
            configure_gemini_backend(params, base_url, model, streaming_handler, interrupt)
        }
    }
}

fn thinking_for(params: &BackendParams) -> Option<ThinkingConfig> {
    params
        .thinking_label
        .as_deref()
        .and_then(ThinkingConfig::from_label)
}

fn configure_anthropic_backend(
    params: &BackendParams,
    base_url: String,
    model: String,
    streaming_handler: Option<StreamingHandler>,
    interrupt: Option<Arc<AtomicBool>>,
) -> Result<ConfiguredBackend, String> {
    // The Anthropic protocol always needs a credential; the keyless short-
    // circuit in build_backend only applies to local OpenAI engines, so a
    // missing credential here means the caller mis-set the provider — echo.
    let Some(credential) = params.credential.clone() else {
        return Ok(echo_backend());
    };
    let mut config = match credential {
        CloudCredential::ApiKey(api_key) => AnthropicConfig::new(base_url, api_key, model.clone()),
        CloudCredential::BearerToken(token) => {
            AnthropicConfig::new_with_bearer_token(base_url, token, model.clone())
        }
    };
    config.max_tokens = params.max_output_tokens;
    config.enable_prompt_caching = params.enable_prompt_caching;
    config.enable_streaming = params.stream || streaming_handler.is_some();
    config.fallback_model = params.fallback_model.clone();
    config.thinking = thinking_for(params);
    if let Some(system_prompt) = params.system_prompt.clone() {
        config = config.with_system_prompt(system_prompt);
    }
    let mut backend = AnthropicBackend::new(config.clone())?;
    if let Some(flag) = interrupt {
        backend = backend.with_interrupt_flag(flag);
    }
    if let Some(handler) = streaming_handler {
        backend = backend.with_streaming_handler(handler);
    }
    Ok(ConfiguredBackend {
        backend: Box::new(backend),
        model,
        is_configured: true,
        subagent_config: Some(SubagentConfig::Anthropic(config)),
    })
}

fn configure_openai_backend(
    params: &BackendParams,
    base_url: String,
    model: String,
    streaming_handler: Option<StreamingHandler>,
    interrupt: Option<Arc<AtomicBool>>,
) -> Result<ConfiguredBackend, String> {
    // OpenAI servers authenticate with a bearer token; both credential forms
    // collapse to the same string. A keyless local engine yields `None` → empty
    // key → no Authorization header (see OpenAiBackend::try_model).
    let api_key = match params.credential.clone() {
        Some(CloudCredential::ApiKey(key)) | Some(CloudCredential::BearerToken(key)) => key,
        None => String::new(),
    };
    let mut config = OpenAiConfig::new(base_url, api_key, model.clone());
    config.max_tokens = params.max_output_tokens;
    config.enable_streaming = params.stream || streaming_handler.is_some();
    config.fallback_model = params.fallback_model.clone();
    if let Some(system_prompt) = params.system_prompt.clone() {
        config = config.with_system_prompt(system_prompt);
    }
    let mut backend = OpenAiBackend::new(config.clone())?;
    if let Some(flag) = interrupt {
        backend = backend.with_interrupt_flag(flag);
    }
    if let Some(handler) = streaming_handler {
        backend = backend.with_streaming_handler(handler);
    }
    Ok(ConfiguredBackend {
        backend: Box::new(backend),
        model,
        is_configured: true,
        subagent_config: Some(SubagentConfig::OpenAi(config)),
    })
}

fn configure_gemini_backend(
    params: &BackendParams,
    base_url: String,
    model: String,
    streaming_handler: Option<StreamingHandler>,
    interrupt: Option<Arc<AtomicBool>>,
) -> Result<ConfiguredBackend, String> {
    let api_key = match params.credential.clone() {
        Some(CloudCredential::ApiKey(key)) | Some(CloudCredential::BearerToken(key)) => key,
        None => String::new(),
    };
    let mut config = GeminiConfig::new(base_url, api_key, model.clone());
    config.max_tokens = params.max_output_tokens;
    config.enable_streaming = params.stream || streaming_handler.is_some();
    config.fallback_model = params.fallback_model.clone();
    if let Some(system_prompt) = params.system_prompt.clone() {
        config = config.with_system_prompt(system_prompt);
    }
    let mut backend = GeminiBackend::new(config.clone())?;
    if let Some(flag) = interrupt {
        backend = backend.with_interrupt_flag(flag);
    }
    if let Some(handler) = streaming_handler {
        backend = backend.with_streaming_handler(handler);
    }
    Ok(ConfiguredBackend {
        backend: Box::new(backend),
        model,
        is_configured: true,
        subagent_config: Some(SubagentConfig::Gemini(config)),
    })
}
