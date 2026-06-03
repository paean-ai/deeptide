//! OpenAI-protocol backend (`/v1/chat/completions`).
//!
//! Why this exists: the [`AgentBackend`] trait is protocol-agnostic, so the
//! whole agent loop, tool system, and TUI work unchanged regardless of how the
//! request is shaped on the wire. The original [`crate::api::AnthropicBackend`]
//! speaks Anthropic's `/v1/messages`; this module speaks the OpenAI
//! chat-completions dialect, which is the lingua franca of the open ecosystem:
//!
//!   * **DeepSeek** (`https://api.deepseek.com`),
//!   * **local inference engines** — ollama (`:11434/v1`), LM Studio
//!     (`:1234/v1`), vLLM / llama.cpp server (`:8000/v1`), mlx-lm — almost all
//!     expose an OpenAI-compatible endpoint,
//!   * OpenAI itself and the legion of OpenAI-compatible relays.
//!
//! It reuses [`crate::api::tool_schema_catalog`] (the single source of truth
//! for the tool catalog) and [`crate::api::classify_api_error`] (provider-
//! agnostic error formatting), so adding a tool or improving an error message
//! lights up on every protocol at once.
//!
//! This first cut is **non-streaming**: it requests a buffered completion and
//! assembles an [`AgentResponse`] identical in shape to the Anthropic path. A
//! streaming variant (SSE `delta` accumulation) layers on next without changing
//! this module's public surface.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::api::{ToolChoice, classify_api_error, tool_schema_catalog};
use crate::streaming::{StreamError, parse_openai_stream};
use crate::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ConversationMessage, MessageRole,
    StreamingHandler, ToolCall,
};

/// Configuration for an OpenAI-compatible chat-completions backend.
///
/// Deliberately a *separate* struct from `AnthropicConfig` rather than a shared
/// one: the two protocols diverge on enough fields (no `cache_control`, no
/// Anthropic `thinking` block, a different tool envelope) that a union type
/// would be a bag of `Option`s where half are always `None`. Keeping them
/// distinct keeps each backend honest about what it actually sends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenAiConfig {
    /// API root. The `/chat/completions` path is appended by
    /// [`chat_completions_url`], which tolerates roots that already include a
    /// `/v1` segment (the common shape for local engines).
    pub base_url: String,
    /// Bearer token. May be empty for keyless local servers (ollama, a bare
    /// llama.cpp `server`); when empty no `Authorization` header is sent.
    pub api_key: String,
    /// Model identifier passed through verbatim — `deepseek-chat`,
    /// `gpt-4o`, `qwen2.5-coder:7b`, an mlx repo id, etc.
    pub model: String,
    /// Output token ceiling. Sent as `max_tokens` (still accepted by every
    /// compatible server; the newer `max_completion_tokens` is OpenAI-only and
    /// rejected by most local engines, so we stay on the portable field).
    pub max_tokens: usize,
    /// System prompt. Empty / whitespace-only is treated as absent.
    pub system_prompt: Option<String>,
    /// How the model may pick tools. Reuses the Anthropic [`ToolChoice`] enum
    /// and maps it onto OpenAI's `tool_choice` dialect at send time.
    pub tool_choice: ToolChoice,
    /// Request `stream: true`. The non-streaming path still assembles the same
    /// [`AgentResponse`]; this flag is plumbed for the forthcoming SSE variant.
    pub enable_streaming: bool,
    /// Model retried once on a transient server-overload error. `None` disables.
    pub fallback_model: Option<String>,
}

impl OpenAiConfig {
    /// Construct with portable defaults (64K output budget, auto tool choice,
    /// non-streaming). Callers override fields directly — they're all public.
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            max_tokens: 65_536,
            system_prompt: None,
            tool_choice: ToolChoice::Auto,
            enable_streaming: false,
            fallback_model: None,
        }
    }

    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        let prompt = prompt.into();
        self.system_prompt = if prompt.trim().is_empty() {
            None
        } else {
            Some(prompt)
        };
        self
    }

    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.enable_streaming = enabled;
        self
    }

    pub fn with_fallback_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        self.fallback_model = if model.trim().is_empty() {
            None
        } else {
            Some(model)
        };
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: usize) -> Self {
        self.max_tokens = max_tokens;
        self
    }
}

/// An [`AgentBackend`] that speaks the OpenAI chat-completions protocol.
pub struct OpenAiBackend {
    config: OpenAiConfig,
    client: Client,
    /// Live-delta sink. When present AND `config.enable_streaming`, the backend
    /// requests `stream: true` and feeds [`parse_openai_stream`], so the TUI
    /// renders OpenAI/DeepSeek/local-model output token-by-token exactly like
    /// the Anthropic path. `None` ⇒ buffered request even if streaming is on.
    streaming_handler: Option<StreamingHandler>,
    /// Cooperative cancellation flag (shared with the agent loop / Ctrl-C
    /// handler). The streaming path checks it between SSE frames; the buffered
    /// path can only observe it before the blocking request returns. `None` =
    /// no cancellation wired up.
    interrupt: Option<Arc<AtomicBool>>,
}

impl OpenAiBackend {
    pub fn new(config: OpenAiConfig) -> Result<Self, String> {
        // 300s ceiling matches the Anthropic client and accommodates slow local
        // inference (a quantized 30B on CPU can take a minute to first token).
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            config,
            client,
            streaming_handler: None,
            interrupt: None,
        })
    }

    pub fn config(&self) -> &OpenAiConfig {
        &self.config
    }

    /// Attach a live-delta sink. Only used when `config.enable_streaming` is
    /// also set (the REPL sets both together).
    pub fn with_streaming_handler(mut self, handler: StreamingHandler) -> Self {
        self.streaming_handler = Some(handler);
        self
    }

    /// Attach the cooperative cancellation flag.
    pub fn with_interrupt_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.interrupt = Some(flag);
        self
    }

    /// Whether this request should stream: both the config flag and an
    /// installed handler are required (a handler is the only thing that makes
    /// streaming observable).
    fn streaming_active(&self) -> bool {
        self.config.enable_streaming && self.streaming_handler.is_some()
    }

    /// `true` once a cancellation has been requested for the current turn.
    fn is_interrupted(&self) -> bool {
        self.interrupt
            .as_ref()
            .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
    }

    /// Issue a single chat-completions request and assemble the response.
    fn try_model(
        &self,
        model: &str,
        request: &AgentRequest,
        effective_system: Option<&str>,
    ) -> Result<AgentResponse, ApiFailure> {
        let started = Instant::now();
        let streaming = self.streaming_active();
        let body = build_chat_request(
            model,
            &request.messages,
            self.config.max_tokens,
            effective_system,
            &self.config.tool_choice,
            request.allowed_tools.as_deref(),
            streaming,
        );
        let url = chat_completions_url(&self.config.base_url);

        let mut req = self.client.post(&url);
        // Keyless local servers (ollama, bare llama.cpp) reject or ignore an
        // empty bearer; only send the header when we actually have a token.
        if !self.config.api_key.trim().is_empty() {
            req = req.bearer_auth(&self.config.api_key);
        }
        if streaming {
            // Some proxies need the explicit Accept to switch to SSE framing.
            req = req.header("accept", "text/event-stream");
        }
        let response = req.json(&body).send().map_err(|error| {
            let msg = error.to_string();
            let transient = error.is_timeout()
                || error.is_connect()
                || msg.contains("connection")
                || msg.contains("reset");
            ApiFailure {
                status: 0,
                transient,
                message: format!("connection error: {error}"),
            }
        })?;

        let status = response.status();
        if !status.is_success() {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let text = response.text().map_err(|error| ApiFailure {
                status: status.as_u16(),
                transient: false,
                message: format!("failed to read response body: {error}"),
            })?;
            return Err(ApiFailure {
                status: status.as_u16(),
                transient: is_retryable_overload(status.as_u16()),
                message: classify_api_error(status.as_u16(), &text, retry_after.as_deref()),
            });
        }

        if streaming {
            // The blocking Response impls Read; feed the SSE parser directly so
            // deltas flow to the handler as the model produces them.
            return parse_openai_stream(
                response,
                self.streaming_handler.as_ref(),
                started.elapsed(),
                self.interrupt.as_deref(),
            )
            .map_err(|err| {
                // A cancelled stream is reported as interrupted (not a model
                // error) so the agent loop's interrupt terminal wins.
                let transient = matches!(err, StreamError::Truncated(_));
                ApiFailure {
                    status: status.as_u16(),
                    transient,
                    message: err.to_string(),
                }
            });
        }

        let text = response.text().map_err(|error| ApiFailure {
            status: status.as_u16(),
            transient: false,
            message: format!("failed to read response body: {error}"),
        })?;
        parse_chat_response(&text, started.elapsed()).map_err(|message| ApiFailure {
            status: status.as_u16(),
            transient: false,
            message,
        })
    }
}

impl AgentBackend for OpenAiBackend {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String> {
        let effective_system = request
            .system
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or(self.config.system_prompt.as_deref())
            .map(ToOwned::to_owned);

        let primary = if request.model.trim().is_empty() {
            self.config.model.clone()
        } else {
            request.model.clone()
        };

        match self.try_model(&primary, &request, effective_system.as_deref()) {
            Ok(response) => Ok(response),
            Err(failure) => {
                // A cancelled turn surfaces as a transport error; report it as
                // such so the agent loop's interrupt check (not a scary
                // connection-failure line) wins.
                if self.is_interrupted() {
                    return Err(String::from("stream cancelled by user"));
                }
                // Retry once with the fallback model on a transient overload.
                if let Some(fallback) = self.config.fallback_model.clone()
                    && fallback != primary
                    && failure.transient
                {
                    return self
                        .try_model(&fallback, &request, effective_system.as_deref())
                        .map_err(|failure| failure.message);
                }
                Err(failure.message)
            }
        }
    }
}

/// A failed chat-completions attempt tagged with enough state to decide on a
/// fallback retry. Mirrors `api::ApiFailure` but stays private to this module.
struct ApiFailure {
    #[allow(dead_code)]
    status: u16,
    transient: bool,
    message: String,
}

/// 429/500/502/503/529 are transient upstream conditions worth one fallback
/// retry. Anything else (auth, bad-request, model-not-found) won't improve on
/// a retry, so we surface it immediately.
fn is_retryable_overload(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 529)
}

/// Build the `/chat/completions` URL from a configured root, tolerating the
/// three shapes users actually paste:
///   * `https://api.openai.com/v1`        → `…/v1/chat/completions`
///   * `https://api.deepseek.com`         → `…/v1/chat/completions`
///   * `http://localhost:11434/v1/chat/completions` (already full) → unchanged
fn chat_completions_url(base_url: &str) -> String {
    let trimmed = base_url.trim_end_matches('/');
    if trimmed.ends_with("/chat/completions") {
        trimmed.to_owned()
    } else if trimmed.ends_with("/v1") {
        format!("{trimmed}/chat/completions")
    } else {
        format!("{trimmed}/v1/chat/completions")
    }
}

// ─── Wire request types ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    max_tokens: usize,
    stream: bool,
    /// `{include_usage: true}` so the final SSE chunk carries token counts —
    /// without it most servers omit usage entirely on streamed responses, and
    /// the `↓ N tokens` display would never get a real total. Only sent when
    /// streaming.
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_options: Option<StreamOptions>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<ChatTool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    /// OpenAI accepts `null` content on an assistant turn that only carries
    /// tool calls, so this is skipped when empty rather than sent as `""`.
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tool_calls: Vec<ChatToolCall>,
    /// Set only on `role: "tool"` messages, linking a result to its call.
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ChatToolCall {
    id: String,
    #[serde(rename = "type")]
    kind: &'static str,
    function: ChatFunctionCall,
}

#[derive(Debug, Serialize)]
struct ChatFunctionCall {
    name: String,
    /// OpenAI carries arguments as a JSON **string**, not an object — the key
    /// shape difference from Anthropic's `input` object.
    arguments: String,
}

#[derive(Debug, Serialize)]
struct ChatTool {
    #[serde(rename = "type")]
    kind: &'static str,
    function: ChatToolFunction,
}

#[derive(Debug, Serialize)]
struct ChatToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

fn build_chat_request<'a>(
    model: &'a str,
    messages: &[ConversationMessage],
    max_tokens: usize,
    system_prompt: Option<&str>,
    tool_choice: &ToolChoice,
    allowed_tools: Option<&[String]>,
    stream: bool,
) -> ChatRequest<'a> {
    let mut wire_messages: Vec<ChatMessage> = Vec::new();

    // System prompt becomes a leading `system` message (vs. Anthropic's
    // top-level `system` field).
    if let Some(system) = system_prompt.map(str::trim).filter(|s| !s.is_empty()) {
        wire_messages.push(ChatMessage {
            role: "system",
            content: Some(system.to_owned()),
            tool_calls: Vec::new(),
            tool_call_id: None,
        });
    }

    for message in messages {
        append_chat_messages(message, &mut wire_messages);
    }

    let tools: Vec<ChatTool> = tool_schema_catalog()
        .into_iter()
        .filter(|tool| match allowed_tools {
            Some(allowed) => allowed.iter().any(|name| name == tool.name),
            None => true,
        })
        .map(|tool| ChatTool {
            kind: "function",
            function: ChatToolFunction {
                name: tool.name.to_owned(),
                description: tool.description.to_owned(),
                parameters: tool.input_schema,
            },
        })
        .collect();

    let tool_choice = if tools.is_empty() {
        None
    } else {
        Some(tool_choice_value(tool_choice))
    };

    ChatRequest {
        model,
        messages: wire_messages,
        max_tokens,
        stream,
        stream_options: stream.then_some(StreamOptions {
            include_usage: true,
        }),
        tools,
        tool_choice,
    }
}

/// Map the shared [`ToolChoice`] enum onto OpenAI's `tool_choice` dialect:
/// `"auto"`, `"required"`, or `{type:"function", function:{name}}`.
fn tool_choice_value(choice: &ToolChoice) -> serde_json::Value {
    match choice {
        ToolChoice::Auto => serde_json::Value::String("auto".to_owned()),
        ToolChoice::Any => serde_json::Value::String("required".to_owned()),
        ToolChoice::Tool(name) => serde_json::json!({
            "type": "function",
            "function": { "name": name },
        }),
    }
}

/// Convert one internal [`ConversationMessage`] into its OpenAI wire
/// message(s). The asymmetry with Anthropic: a user turn that answers a tool
/// batch becomes **one `role:"tool"` message per result** (Anthropic packs them
/// into a single user message), and an assistant turn carries `tool_calls`
/// with stringified `arguments`.
fn append_chat_messages(message: &ConversationMessage, out: &mut Vec<ChatMessage>) {
    match message.role {
        MessageRole::Assistant => {
            let tool_calls: Vec<ChatToolCall> = message
                .tool_calls
                .iter()
                .map(|call| ChatToolCall {
                    id: call.id.clone(),
                    kind: "function",
                    function: ChatFunctionCall {
                        name: call.name.clone(),
                        arguments: serde_json::to_string(&call.input)
                            .unwrap_or_else(|_| "{}".to_owned()),
                    },
                })
                .collect();
            let content = if message.content.is_empty() {
                None
            } else {
                Some(message.content.clone())
            };
            // An assistant turn with neither text nor tool calls would be an
            // empty shell; skip it rather than send `{role:"assistant"}` with
            // nothing, which some servers reject.
            if content.is_some() || !tool_calls.is_empty() {
                out.push(ChatMessage {
                    role: "assistant",
                    content,
                    tool_calls,
                    tool_call_id: None,
                });
            }
        }
        MessageRole::User => {
            // Free-form user text (if any) comes first…
            if !message.content.is_empty() {
                out.push(ChatMessage {
                    role: "user",
                    content: Some(message.content.clone()),
                    tool_calls: Vec::new(),
                    tool_call_id: None,
                });
            }
            // …then one tool message per tool_result, each linked by id.
            for result in &message.tool_results {
                out.push(ChatMessage {
                    role: "tool",
                    content: Some(result.content.clone()),
                    tool_calls: Vec::new(),
                    tool_call_id: Some(result.tool_use_id.clone()),
                });
            }
        }
    }
}

// ─── Wire response types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<ChatUsage>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Debug, Deserialize)]
struct ChatResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ChatResponseToolCall>,
}

#[derive(Debug, Deserialize)]
struct ChatResponseToolCall {
    #[serde(default)]
    id: String,
    function: ChatResponseFunction,
}

#[derive(Debug, Deserialize)]
struct ChatResponseFunction {
    #[serde(default)]
    name: String,
    #[serde(default)]
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct ChatUsage {
    #[serde(default)]
    prompt_tokens: usize,
    #[serde(default)]
    completion_tokens: usize,
    /// OpenAI/DeepSeek report cache hits here. Absent on most local engines.
    #[serde(default)]
    prompt_tokens_details: Option<PromptTokensDetails>,
}

#[derive(Debug, Deserialize)]
struct PromptTokensDetails {
    #[serde(default)]
    cached_tokens: usize,
}

fn parse_chat_response(body: &str, elapsed: Duration) -> Result<AgentResponse, String> {
    let response: ChatResponse =
        serde_json::from_str(body).map_err(|error| format!("invalid model response: {error}"))?;

    let choice = response
        .choices
        .into_iter()
        .next()
        .ok_or_else(|| String::from("model response contained no choices"))?;

    let content = choice.message.content.unwrap_or_default();
    let tool_calls: Vec<ToolCall> = choice
        .message
        .tool_calls
        .into_iter()
        .map(|call| {
            // OpenAI ships `arguments` as a JSON string. Parse it back into an
            // object so the rest of the agent (which works in
            // `serde_json::Value`) sees the same shape as the Anthropic path.
            // A blank string means "no arguments" → empty object.
            let input = if call.function.arguments.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(&call.function.arguments)
                    .unwrap_or_else(|_| serde_json::json!({}))
            };
            ToolCall::new(call.id, call.function.name, input)
        })
        .collect();

    let usage = response.usage.map(|usage| {
        let cache_read = usage
            .prompt_tokens_details
            .map(|details| details.cached_tokens)
            .unwrap_or(0);
        // OpenAI/DeepSeek report `prompt_tokens` as the FULL prompt size,
        // INCLUDING the cached portion — unlike Anthropic, where `input_tokens`
        // and `cache_read_input_tokens` are disjoint. Subtract the cached count
        // so the cost tracker prices each token exactly once (uncached at the
        // input rate, cached at the cheaper cache-read rate) instead of
        // double-charging the cached tokens.
        let uncached_input = usage.prompt_tokens.saturating_sub(cache_read);
        AgentUsage::new(
            uncached_input,
            usage.completion_tokens,
            // OpenAI has no cache-creation token concept; report 0.
            0,
            cache_read,
            elapsed.as_millis().try_into().unwrap_or(usize::MAX),
        )
    });

    Ok(AgentResponse {
        content,
        usage,
        tool_calls,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn user(text: &str) -> ConversationMessage {
        ConversationMessage::user(text)
    }

    #[test]
    fn chat_completions_url_handles_the_three_common_root_shapes() {
        assert_eq!(
            chat_completions_url("https://api.openai.com/v1"),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("https://api.deepseek.com"),
            "https://api.deepseek.com/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("http://localhost:11434/v1/chat/completions"),
            "http://localhost:11434/v1/chat/completions"
        );
        // Trailing slash is tolerated.
        assert_eq!(
            chat_completions_url("http://localhost:1234/v1/"),
            "http://localhost:1234/v1/chat/completions"
        );
    }

    #[test]
    fn system_prompt_becomes_a_leading_system_message() {
        let req = build_chat_request(
            "gpt-4o",
            &[user("hi")],
            1000,
            Some("be terse"),
            &ToolChoice::Auto,
            None,
            false,
        );
        assert_eq!(req.messages[0].role, "system");
        assert_eq!(req.messages[0].content.as_deref(), Some("be terse"));
        assert_eq!(req.messages[1].role, "user");
        assert_eq!(req.messages[1].content.as_deref(), Some("hi"));
    }

    #[test]
    fn assistant_tool_calls_serialize_arguments_as_a_json_string() {
        let assistant = ConversationMessage::assistant_with_tool_calls(
            "",
            vec![ToolCall::new(
                "call_1",
                "Read",
                serde_json::json!({"file_path": "a.rs"}),
            )],
        );
        let mut out = Vec::new();
        append_chat_messages(&assistant, &mut out);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].role, "assistant");
        // No text → content omitted.
        assert!(out[0].content.is_none());
        assert_eq!(out[0].tool_calls.len(), 1);
        let call = &out[0].tool_calls[0];
        assert_eq!(call.id, "call_1");
        assert_eq!(call.function.name, "Read");
        // Arguments are a JSON STRING, not an object.
        assert_eq!(call.function.arguments, r#"{"file_path":"a.rs"}"#);
    }

    #[test]
    fn tool_results_become_one_tool_message_each() {
        let results = ConversationMessage::tool_results(vec![
            crate::ToolResultBlock::new("call_1", "alpha", false),
            crate::ToolResultBlock::new("call_2", "beta", true),
        ]);
        let mut out = Vec::new();
        append_chat_messages(&results, &mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].role, "tool");
        assert_eq!(out[0].tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(out[0].content.as_deref(), Some("alpha"));
        assert_eq!(out[1].tool_call_id.as_deref(), Some("call_2"));
    }

    #[test]
    fn tool_choice_maps_onto_openai_dialect() {
        assert_eq!(
            tool_choice_value(&ToolChoice::Auto),
            serde_json::json!("auto")
        );
        assert_eq!(
            tool_choice_value(&ToolChoice::Any),
            serde_json::json!("required")
        );
        assert_eq!(
            tool_choice_value(&ToolChoice::Tool("Read".into())),
            serde_json::json!({"type": "function", "function": {"name": "Read"}})
        );
    }

    #[test]
    fn allowed_tools_filters_the_advertised_catalog() {
        let req = build_chat_request(
            "m",
            &[user("hi")],
            100,
            None,
            &ToolChoice::Auto,
            Some(&["Read".to_owned(), "Bash".to_owned()]),
            false,
        );
        let names: Vec<&str> = req.tools.iter().map(|t| t.function.name.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"Read"));
        assert!(names.contains(&"Bash"));
    }

    #[test]
    fn parse_response_extracts_text_tool_calls_and_usage() {
        let body = r#"{
            "choices": [{
                "message": {
                    "content": "working on it",
                    "tool_calls": [{
                        "id": "call_9",
                        "type": "function",
                        "function": {"name": "Bash", "arguments": "{\"command\":\"ls\"}"}
                    }]
                }
            }],
            "usage": {
                "prompt_tokens": 120,
                "completion_tokens": 34,
                "prompt_tokens_details": {"cached_tokens": 64}
            }
        }"#;
        let response = parse_chat_response(body, Duration::from_millis(7)).unwrap();
        assert_eq!(response.content, "working on it");
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "Bash");
        // Stringified arguments are parsed back into an object.
        assert_eq!(
            response.tool_calls[0].input,
            serde_json::json!({"command": "ls"})
        );
        let usage = response.usage.unwrap();
        // prompt_tokens (120) INCLUDES the 64 cached; input is the uncached
        // remainder so cost isn't double-charged: 120 - 64 = 56.
        assert_eq!(usage.input_tokens, 56);
        assert_eq!(usage.output_tokens, 34);
        assert_eq!(usage.cache_read, 64);
        assert_eq!(usage.cache_create, 0);
        // Invariant: uncached input + cache_read == the provider's prompt total.
        assert_eq!(usage.input_tokens + usage.cache_read, 120);
    }

    #[test]
    fn parse_response_tolerates_missing_usage_and_empty_content() {
        let body = r#"{"choices": [{"message": {"tool_calls": []}}]}"#;
        let response = parse_chat_response(body, Duration::ZERO).unwrap();
        assert_eq!(response.content, "");
        assert!(response.tool_calls.is_empty());
        assert!(response.usage.is_none());
    }

    #[test]
    fn parse_response_errors_when_no_choices() {
        let body = r#"{"choices": []}"#;
        let err = parse_chat_response(body, Duration::ZERO).unwrap_err();
        assert!(err.contains("no choices"), "got: {err}");
    }

    #[test]
    fn blank_tool_arguments_decode_to_empty_object() {
        let body = r#"{"choices":[{"message":{"tool_calls":[
            {"id":"c","type":"function","function":{"name":"Read","arguments":""}}
        ]}}]}"#;
        let response = parse_chat_response(body, Duration::ZERO).unwrap();
        assert_eq!(response.tool_calls[0].input, serde_json::json!({}));
    }

    #[test]
    fn config_builders_normalize_blank_inputs() {
        let cfg = OpenAiConfig::new("http://x", "", "m")
            .with_system_prompt("   ")
            .with_fallback_model("");
        assert!(cfg.system_prompt.is_none());
        assert!(cfg.fallback_model.is_none());
        let cfg = cfg.with_system_prompt("real").with_fallback_model("fb");
        assert_eq!(cfg.system_prompt.as_deref(), Some("real"));
        assert_eq!(cfg.fallback_model.as_deref(), Some("fb"));
    }
}
