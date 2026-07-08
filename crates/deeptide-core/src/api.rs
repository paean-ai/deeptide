use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ConversationMessage, MessageRole,
    StreamingEvent, StreamingHandler, ToolCall, ToolResultBlock,
    streaming::{STREAM_RETRY_SIGNAL_PREFIX, parse_streaming_response},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnthropicConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: usize,
    pub auth_mode: AnthropicAuthMode,
    /// Optional system prompt. Empty/whitespace-only is treated as None on the wire.
    pub system_prompt: Option<String>,
    /// How the model is allowed to pick tools. Defaults to [`ToolChoice::Auto`].
    pub tool_choice: ToolChoice,
    /// Attach `cache_control: ephemeral` to the system block (when present) and
    /// the last tool schema so Anthropic can reuse cached tokens across turns.
    /// The bulk of every request is the ~50K-token tool schema; caching it
    /// turns a $0.50 request into a $0.05 cache-hit on follow-ups.
    pub enable_prompt_caching: bool,
    /// When true, the backend requests `stream: true` from `/v1/messages` and
    /// parses the SSE response, optionally invoking
    /// [`AnthropicBackend::with_streaming_handler`] for each delta. The
    /// assembled response is identical to the non-streaming path so callers
    /// of [`AgentBackend::respond`] don't see a behaviour change — only the
    /// wire shape changes. Required for proxies (openrouter, custom relays)
    /// that mandate streaming.
    pub enable_streaming: bool,
    /// Model to retry with once when the primary model returns a transient
    /// server-overload error (HTTP 529 or 503). `None` disables the fallback.
    /// Mirrors the Swift implementation's `fallback_model` behavior.
    pub fallback_model: Option<String>,
    /// Extended-thinking directive. `None` omits the field (provider default).
    pub thinking: Option<ThinkingConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnthropicAuthMode {
    ApiKey,
    BearerToken,
}

/// Maps onto Anthropic's [tool choice](https://docs.anthropic.com/en/api/messages#tool-choice)
/// directive. Defaults to [`Auto`](ToolChoice::Auto), matching the API default
/// when omitted, but we always send it on the wire to keep behaviour explicit
/// and stable across API revisions.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ToolChoice {
    /// Model decides whether to call a tool. API default when omitted.
    #[default]
    Auto,
    /// Model MUST call some tool, but may pick which one.
    Any,
    /// Model MUST call exactly the named tool.
    Tool(String),
}

/// Extended-thinking / reasoning-effort directive sent on the wire as
/// `{"type": "enabled"|"disabled", "budget_tokens": N}`. Mirrors the Swift
/// implementation's `ThinkingConfig`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(rename = "budget_tokens", skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<usize>,
}

impl ThinkingConfig {
    /// `low` reasoning budget (4 000 tokens).
    pub fn low() -> Self {
        Self::enabled(4_000)
    }

    /// `medium` reasoning budget (16 000 tokens). The default when a generic
    /// "enabled" value is requested.
    pub fn medium() -> Self {
        Self::enabled(16_000)
    }

    /// `high` reasoning budget (32 000 tokens).
    pub fn high() -> Self {
        Self::enabled(32_000)
    }

    fn enabled(budget_tokens: usize) -> Self {
        Self {
            kind: String::from("enabled"),
            budget_tokens: Some(budget_tokens),
        }
    }

    /// Thinking explicitly turned off.
    pub fn disabled() -> Self {
        Self {
            kind: String::from("disabled"),
            budget_tokens: None,
        }
    }

    /// Parse a `thinking`/`effort` label into a config, mirroring Swift's
    /// `ThinkingConfig.from`. Returns `None` for unset/`auto`/`default`, which
    /// means "omit the field and let the provider decide".
    pub fn from_label(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "auto" | "default" => None,
            "disabled" | "disable" | "off" | "none" | "false" => Some(Self::disabled()),
            "low" => Some(Self::low()),
            "high" => Some(Self::high()),
            // "medium"/"enabled"/"enable"/"on"/"true" and any other non-empty value.
            _ => Some(Self::medium()),
        }
    }

    /// Whether this directive actually enables thinking (vs. an explicit
    /// `disabled`).
    pub fn is_enabled(&self) -> bool {
        self.kind == "enabled"
    }
}

impl AnthropicConfig {
    pub fn new(
        base_url: impl Into<String>,
        api_key: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: api_key.into(),
            model: model.into(),
            // 64K matches the practical output cap of every modern
            // Anthropic-compatible backend we ship against:
            //
            //   * Claude 4.5 Sonnet:        64K output
            //   * DeepSeek-V4-Pro/Flash:    384K output  (1M context)
            //   * Paean (Anthropic-format): 64K+ output
            //   * Older Claude 3 / DeepSeek-V3.2: server-side clamped
            //     to their own limit (8K / 16K), so requesting 64K is
            //     safe — the API just returns whatever it supports.
            //
            // Defaulting to 64K closes the failure mode where a single
            // `Write` of a non-trivial HTML/code file (~7-10K tokens)
            // hits the budget mid-call and produces an unparseable
            // partial tool input. We don't go higher by default because
            // (a) 384K would be a footgun for billing on first-use and
            // (b) for outputs larger than 64K the right design is to
            // chunk via Edit / append-style tools rather than one-shot.
            max_tokens: 65_536,
            auth_mode: AnthropicAuthMode::ApiKey,
            system_prompt: None,
            tool_choice: ToolChoice::Auto,
            enable_prompt_caching: true,
            enable_streaming: false,
            fallback_model: None,
            thinking: None,
        }
    }

    pub fn new_with_bearer_token(
        base_url: impl Into<String>,
        token: impl Into<String>,
        model: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: token.into(),
            model: model.into(),
            // See `new` — same rationale, applies to bearer-auth backends too.
            max_tokens: 65_536,
            auth_mode: AnthropicAuthMode::BearerToken,
            system_prompt: None,
            tool_choice: ToolChoice::Auto,
            enable_prompt_caching: true,
            enable_streaming: false,
            fallback_model: None,
            thinking: None,
        }
    }

    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        let text = system_prompt.into();
        self.system_prompt = if text.trim().is_empty() {
            None
        } else {
            Some(text)
        };
        self
    }

    pub fn with_tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = choice;
        self
    }

    pub fn with_prompt_caching(mut self, enabled: bool) -> Self {
        self.enable_prompt_caching = enabled;
        self
    }

    pub fn with_streaming(mut self, enabled: bool) -> Self {
        self.enable_streaming = enabled;
        self
    }

    /// Set the model retried once on a transient server overload. An
    /// empty/whitespace-only value clears the fallback.
    pub fn with_fallback_model(mut self, model: impl Into<String>) -> Self {
        let model = model.into();
        self.fallback_model = if model.trim().is_empty() {
            None
        } else {
            Some(model)
        };
        self
    }

    /// Set the extended-thinking directive. `None` omits it from the request.
    pub fn with_thinking(mut self, thinking: Option<ThinkingConfig>) -> Self {
        self.thinking = thinking;
        self
    }
}

pub struct AnthropicBackend {
    config: AnthropicConfig,
    client: Client,
    /// Caller-supplied sink for live streaming deltas. Only invoked when
    /// `config.enable_streaming == true`. Wrapped in `Arc<dyn Fn>` so the
    /// backend can be cheaply cloned into sub-agent factories while keeping
    /// every sub-agent pointed at the same output stream.
    streaming_handler: Option<StreamingHandler>,
    /// Cooperative cancellation flag, shared with the agent loop / CLI Ctrl-C
    /// handler. Threaded into [`parse_streaming_response`] so an in-flight SSE
    /// stream can be dropped mid-generation. `None` = no cancellation wired up.
    interrupt: Option<Arc<AtomicBool>>,
}

impl AnthropicBackend {
    pub fn new(config: AnthropicConfig) -> Result<Self, String> {
        // Cancellation note: this reqwest build has no per-read timeout on the
        // blocking client, so we can't wake a blocked SSE read on a timer.
        // Instead, the parser checks the interrupt flag *between* SSE events
        // (see `parse_streaming_response`). During active token generation,
        // events arrive continuously, so a Ctrl-C is observed within one event
        // (tens of ms). The only non-instant window is the model's pre-first-
        // token "thinking" pause, where the read blocks with no bytes; that
        // cancellation lands as soon as the first token arrives. Tools and the
        // between-step checks cancel immediately regardless.
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

    /// Attach the cooperative cancellation flag so an in-flight streaming
    /// request can be aborted mid-generation by a Ctrl-C.
    pub fn with_interrupt_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.interrupt = Some(flag);
        self
    }

    pub fn config(&self) -> &AnthropicConfig {
        &self.config
    }

    /// Attach a sink for live streaming deltas. Only invoked when the
    /// underlying config has `enable_streaming = true`.
    pub fn with_streaming_handler(mut self, handler: StreamingHandler) -> Self {
        self.streaming_handler = Some(handler);
        self
    }
}

impl AgentBackend for AnthropicBackend {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String> {
        // AgentRequest::system overrides AnthropicConfig::system_prompt when set,
        // allowing per-session prompts (built from CWD) to take precedence over
        // the static prompt injected via CLI flags.
        let effective_system = request
            .system
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .or(self.config.system_prompt.as_deref())
            .map(ToOwned::to_owned);

        // Prefer the per-request model — this is how a runtime model switch
        // (e.g. the REPL `/model` command, via AgentLoop::set_model) reaches the
        // wire — falling back to the configured default when unset.
        let primary = if request.model.trim().is_empty() {
            self.config.model.clone()
        } else {
            request.model.clone()
        };
        // Same precedence story for the thinking directive: a per-
        // request override (from `/think`) wins over whatever was
        // baked into the backend at construction time, but absence of
        // an override falls back to the construction-time default.
        // Implemented by temporarily swapping `self.config.thinking`
        // for the duration of this call — cheap because `ThinkingConfig`
        // is small, and it keeps `try_model_with_stream_retry` (and the
        // rest of the request-building plumbing) unchanged.
        let baseline_thinking = self.config.thinking.clone();
        if let Some(override_cfg) = request.thinking.clone() {
            self.config.thinking = Some(override_cfg);
        }
        let outcome =
            self.try_model_with_stream_retry(&primary, &request, effective_system.as_deref());
        self.config.thinking = baseline_thinking;
        match outcome {
            Ok(response) => Ok(response),
            Err(failure) => {
                // On a transient server overload, retry once with the
                // configured fallback model before surfacing the error.
                let fallback = self.config.fallback_model.clone();
                if let Some(fallback) = fallback
                    && fallback != primary
                    && is_retryable_overload(failure.status)
                {
                    return self
                        .try_model_with_stream_retry(
                            &fallback,
                            &request,
                            effective_system.as_deref(),
                        )
                        .map_err(|failure| failure.message);
                }
                Err(failure.message)
            }
        }
    }
}

/// Maximum number of SSE-truncation retries before bubbling the error.
/// Three attempts (initial + 2 retries) is the same budget zero-cli uses
/// and matches the empirical observation that transient stream cuts
/// almost always recover by the second attempt.
const STREAM_TRUNCATION_MAX_ATTEMPTS: u32 = 3;

/// Compute the backoff sleep between attempt N and N+1. We start at
/// 200 ms and double (200 → 400 → 800 …) so the first retry is fast
/// (likely-transient cut), and any subsequent retry signals a fatter
/// problem and waits longer to avoid hammering the upstream.
fn stream_retry_backoff(attempt: u32) -> Duration {
    let base_ms: u64 = 200;
    Duration::from_millis(base_ms.saturating_mul(1_u64 << attempt.min(4)))
}

/// A failed `/v1/messages` attempt, tagging the formatted error with the HTTP
/// status so the caller can decide whether to retry with a fallback model.
struct ApiFailure {
    /// HTTP status code, or `0` for transport-level errors (no response).
    status: u16,
    /// Set when the failure was a streaming SSE truncation — the HTTP
    /// request was idempotent and the cause is a transient network
    /// or upstream-proxy hiccup, so the caller may safely re-run the
    /// same request without changing inputs.
    transient_truncation: bool,
    message: String,
}

/// Server-side overload statuses worth retrying with a different model.
/// `529` is Anthropic's "overloaded"; `503` is a generic upstream outage.
fn is_retryable_overload(status: u16) -> bool {
    matches!(status, 503 | 529)
}

impl AnthropicBackend {
    /// Wrap [`Self::try_model`] in an SSE-truncation retry loop.
    ///
    /// Real-world failure mode (PR #96, user-reported): a long Write
    /// tool call gets cut mid-payload because the upstream proxy / load
    /// balancer / model server hiccuped before emitting `message_stop`.
    /// The HTTP request itself was idempotent — the same body produces
    /// equivalent output modulo sampling — so retrying transparently
    /// is the right behaviour, exactly mirroring how zero-cli handles
    /// the same class of error.
    ///
    /// We retry only when [`ApiFailure::transient_truncation`] is set:
    /// other failures (HTTP 4xx, model-rejected, malformed JSON with a
    /// clean stream close, U+FFFD upstream-corruption) are *not* retried
    /// because the recourse is human, not mechanical. Backoff doubles
    /// between attempts and we cap at [`STREAM_TRUNCATION_MAX_ATTEMPTS`].
    ///
    /// Each retry emits a synthetic streaming notice through the user's
    /// installed `StreamingHandler` so the UI can surface "stream cut,
    /// retrying…" instead of going silent for a full second.
    fn try_model_with_stream_retry(
        &self,
        model: &str,
        request: &AgentRequest,
        effective_system: Option<&str>,
    ) -> Result<AgentResponse, ApiFailure> {
        let mut last_err: Option<ApiFailure> = None;
        for attempt in 0..STREAM_TRUNCATION_MAX_ATTEMPTS {
            match self.try_model(model, request, effective_system) {
                Ok(response) => return Ok(response),
                Err(failure) => {
                    if !failure.transient_truncation {
                        return Err(failure);
                    }
                    // Last attempt: don't sleep, just surface.
                    if attempt + 1 >= STREAM_TRUNCATION_MAX_ATTEMPTS {
                        last_err = Some(failure);
                        break;
                    }
                    // Tell the UI we're retrying so the user sees
                    // forward progress rather than a frozen spinner.
                    if let Some(handler) = self.streaming_handler.as_ref() {
                        handler(&StreamingEvent::MessageDelta {
                            stop_reason: Some(format!(
                                "{prefix}{}/{} ({})",
                                attempt + 1,
                                STREAM_TRUNCATION_MAX_ATTEMPTS,
                                failure.message,
                                prefix = STREAM_RETRY_SIGNAL_PREFIX,
                            )),
                            output_tokens: None,
                        });
                    }
                    std::thread::sleep(stream_retry_backoff(attempt));
                    last_err = Some(failure);
                }
            }
        }
        Err(last_err.expect("retry loop must have populated last_err on exit"))
    }

    /// Issue a single `/v1/messages` request with the given model, returning
    /// the assembled response or a status-tagged failure.
    fn try_model(
        &self,
        model: &str,
        request: &AgentRequest,
        effective_system: Option<&str>,
    ) -> Result<AgentResponse, ApiFailure> {
        let started = Instant::now();
        let mut body = build_messages_request(
            model,
            &request.messages,
            self.config.max_tokens,
            effective_system,
            &self.config.tool_choice,
            self.config.enable_prompt_caching,
        );
        // A restricted sub-agent only advertises the tools it may call, so the
        // model is not offered (and cannot waste a turn on) a forbidden tool.
        if let Some(allowed) = request.allowed_tools.as_deref() {
            body.tools = filter_wire_tools(
                std::mem::take(&mut body.tools),
                allowed,
                self.config.enable_prompt_caching,
            );
        }
        apply_thinking(&mut body, self.config.thinking.as_ref());
        body.stream = self.config.enable_streaming;
        let url = messages_url(&self.config.base_url);

        let req = self
            .client
            .post(url)
            .header("anthropic-version", "2023-06-01");
        let req = if self.config.enable_streaming {
            // Some proxies require the explicit Accept header to switch from
            // buffered JSON responses to SSE. Anthropic itself doesn't require
            // it, but setting it costs nothing and improves portability.
            req.header("accept", "text/event-stream")
        } else {
            req
        };
        let req = req.json(&body);
        let req = match self.config.auth_mode {
            AnthropicAuthMode::ApiKey => req.header("x-api-key", &self.config.api_key),
            AnthropicAuthMode::BearerToken => req.bearer_auth(&self.config.api_key),
        };
        let response = req.send().map_err(|error| {
            // Some `reqwest` send-time errors stem from a mid-stream
            // reset where the connection was established but never got
            // a clean response. Heuristically these errors stringify
            // with "decode" / "body" hints or fail at connect/timeout —
            // treat them as transient so the outer retry loop gives
            // them another try.
            let msg = error.to_string();
            let transient = error.is_timeout()
                || error.is_connect()
                || msg.contains("connection")
                || msg.contains("reset")
                || msg.contains("decode")
                || msg.contains("body");
            ApiFailure {
                status: 0,
                transient_truncation: transient,
                message: format!("connection error: {error}"),
            }
        })?;
        let status = response.status();

        if !status.is_success() {
            // Capture the server's retry hint before the body is consumed, so a
            // 429/503/529 error message can name the concrete wait time instead
            // of a generic "retry later".
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let text = response.text().map_err(|error| ApiFailure {
                status: status.as_u16(),
                transient_truncation: false,
                message: format!("failed to read response body: {error}"),
            })?;
            return Err(ApiFailure {
                status: status.as_u16(),
                transient_truncation: false,
                message: classify_error(status.as_u16(), &text, retry_after.as_deref()),
            });
        }

        if self.config.enable_streaming {
            // The reqwest blocking Response impls `Read`, which the SSE
            // parser consumes chunk-by-chunk — no need to buffer the whole
            // payload up front, so live deltas flow to the handler as the
            // model produces them.
            parse_streaming_response(
                response,
                self.streaming_handler.as_ref(),
                started.elapsed(),
                self.interrupt.as_deref(),
            )
            .map_err(|stream_err| ApiFailure {
                status: status.as_u16(),
                transient_truncation: stream_err.is_transient_retry(),
                message: stream_err.to_string(),
            })
        } else {
            let text = response.text().map_err(|error| ApiFailure {
                status: status.as_u16(),
                transient_truncation: false,
                message: format!("failed to read response body: {error}"),
            })?;
            parse_messages_response(&text, started.elapsed()).map_err(|message| ApiFailure {
                status: status.as_u16(),
                transient_truncation: false,
                message,
            })
        }
    }
}

#[derive(Debug, Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: usize,
    messages: Vec<WireMessage>,
    tools: Vec<WireTool>,
    stream: bool,
    /// Anthropic accepts either a plain string OR an array of text blocks for
    /// `system`. We always emit the array form (when set) so a single
    /// `cache_control` marker can be attached to the last block — that marker
    /// caches the entire system prefix server-side.
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Vec<WireSystemBlock>>,
    /// Direct passthrough of [`ToolChoice`]. Always emitted when tools are
    /// declared so behaviour is explicit and stable across API defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<WireToolChoice>,
    /// Extended-thinking directive. Omitted unless configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    thinking: Option<ThinkingConfig>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct WireMessage {
    role: &'static str,
    content: Vec<WireContentBlock>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct WireSystemBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<WireCacheControl>,
}

/// Marks a content block / tool schema as eligible for Anthropic's ephemeral
/// prompt cache. The 1h TTL flavour is intentionally not exposed yet — only
/// allowlisted accounts can use it and it's an easy way to silently bust
/// caching for everyone else (zero-cli gates 1h behind a GrowthBook flag for
/// exactly this reason).
#[derive(Debug, Serialize, PartialEq, Eq, Clone)]
struct WireCacheControl {
    #[serde(rename = "type")]
    cache_type: &'static str,
}

impl WireCacheControl {
    fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral",
        }
    }
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireToolChoice {
    Auto,
    Any,
    Tool { name: String },
}

impl WireToolChoice {
    fn from(choice: &ToolChoice) -> Self {
        match choice {
            ToolChoice::Auto => Self::Auto,
            ToolChoice::Any => Self::Any,
            ToolChoice::Tool(name) => Self::Tool { name: name.clone() },
        }
    }
}

/// Anthropic content block, serialised with `type` discriminator. Covers every
/// kind we actually round-trip: free-form `text`, assistant-issued `tool_use`
/// invocations, and the user-side `tool_result` blocks that answer them.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WireContentBlock {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<WireCacheControl>,
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<WireCacheControl>,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "is_false")]
        is_error: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<WireCacheControl>,
    },
}

impl WireContentBlock {
    /// Mutable handle to this block's cache marker slot, regardless of variant.
    /// Used to stamp the rolling conversation cache breakpoint on the tail
    /// block without caring what kind of block it is.
    fn cache_control_slot(&mut self) -> &mut Option<WireCacheControl> {
        match self {
            WireContentBlock::Text { cache_control, .. } => cache_control,
            WireContentBlock::ToolUse { cache_control, .. } => cache_control,
            WireContentBlock::ToolResult { cache_control, .. } => cache_control,
        }
    }
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Serialize)]
struct WireTool {
    name: &'static str,
    description: &'static str,
    input_schema: serde_json::Value,
    /// Optional ephemeral cache marker. Anthropic uses this on a single tool
    /// (typically the last one) as a cache breakpoint covering the whole tool
    /// schema prefix — for us that's ~50K tokens of schema definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<WireCacheControl>,
}

/// Keep only the tools named in `allowed`, preserving declaration order, and
/// re-attach the cache breakpoint to the new last tool. Used to advertise a
/// restricted sub-agent only the tools it is permitted to call.
fn filter_wire_tools(
    tools: Vec<WireTool>,
    allowed: &[String],
    enable_caching: bool,
) -> Vec<WireTool> {
    let mut filtered: Vec<WireTool> = tools
        .into_iter()
        .filter(|tool| allowed.iter().any(|name| name == tool.name))
        .map(|mut tool| {
            tool.cache_control = None;
            tool
        })
        .collect();
    if enable_caching && let Some(last) = filtered.last_mut() {
        last.cache_control = Some(WireCacheControl::ephemeral());
    }
    filtered
}

fn build_messages_request<'a>(
    model: &'a str,
    messages: &[ConversationMessage],
    max_tokens: usize,
    system_prompt: Option<&str>,
    tool_choice: &ToolChoice,
    enable_prompt_caching: bool,
) -> MessagesRequest<'a> {
    let mut tools = tool_schemas();
    // Attach the cache breakpoint to the LAST declared tool. Anthropic uses
    // that marker as the cache prefix boundary, so everything up through (and
    // including) the entire tool schema becomes one cached chunk. Doing this
    // unconditionally when caching is enabled keeps the prefix bytes stable
    // turn-to-turn — the #1 thing that breaks prompt caching is moving the
    // marker around between requests.
    if enable_prompt_caching && let Some(last) = tools.last_mut() {
        last.cache_control = Some(WireCacheControl::ephemeral());
    }

    let system = system_prompt
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| {
            vec![WireSystemBlock {
                block_type: "text",
                text: text.to_owned(),
                cache_control: if enable_prompt_caching {
                    Some(WireCacheControl::ephemeral())
                } else {
                    None
                },
            }]
        });

    let mut wire_messages: Vec<WireMessage> = messages.iter().map(wire_message_from).collect();

    // Rolling conversation cache breakpoint. The system prompt and tool schemas
    // already carry static breakpoints above; the conversation itself grows
    // every turn and — without a marker — Anthropic would reprocess the entire
    // history uncached on each request. Stamping one ephemeral marker on the
    // LAST block of the LAST message makes this turn WRITE a cache of the whole
    // prefix; the next turn (which shares that prefix verbatim, since the
    // history only ever appends) gets a cache READ for everything up to here.
    //
    // Cache READS use the longest matching previously-cached prefix regardless
    // of where the *current* breakpoint sits, so a single tail marker per turn
    // is the canonical incremental pattern — and keeps us at 3 of Anthropic's
    // 4 allowed breakpoints (tools + system + conversation).
    if enable_prompt_caching
        && let Some(last_block) = wire_messages
            .last_mut()
            .and_then(|message| message.content.last_mut())
    {
        *last_block.cache_control_slot() = Some(WireCacheControl::ephemeral());
    }

    MessagesRequest {
        model,
        max_tokens,
        messages: wire_messages,
        tools,
        stream: false,
        system,
        tool_choice: Some(WireToolChoice::from(tool_choice)),
        thinking: None,
    }
}

/// Apply a thinking directive to a request, bumping `max_tokens` above the
/// thinking budget. Anthropic requires `max_tokens > thinking.budget_tokens`,
/// so we reserve the configured output budget on top of the thinking budget
/// (capped to a sane ceiling) when thinking is enabled.
fn apply_thinking(body: &mut MessagesRequest<'_>, thinking: Option<&ThinkingConfig>) {
    let Some(thinking) = thinking else {
        return;
    };
    if thinking.is_enabled()
        && let Some(budget) = thinking.budget_tokens
    {
        body.max_tokens = budget.saturating_add(body.max_tokens).min(64_000);
    }
    body.thinking = Some(thinking.clone());
}

fn wire_message_from(message: &ConversationMessage) -> WireMessage {
    let role: &'static str = match message.role {
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
    };

    let mut blocks: Vec<WireContentBlock> = Vec::new();

    if !message.content.is_empty() {
        blocks.push(WireContentBlock::Text {
            text: message.content.clone(),
            cache_control: None,
        });
    }

    // Assistant turns carry the tool_use blocks the model emitted. We append
    // them after the text body so the order matches what the model produced
    // and what Anthropic expects to see echoed back on the next turn.
    for tool_call in &message.tool_calls {
        blocks.push(WireContentBlock::ToolUse {
            id: tool_call.id.clone(),
            name: tool_call.name.clone(),
            input: tool_call.input.clone(),
            cache_control: None,
        });
    }

    // User turns answering a tool batch carry one tool_result per executed
    // tool, keyed by the matching tool_use_id. Anthropic rejects the request
    // if any tool_use lacks a matching tool_result in the very next user
    // message, so the agent loop must produce them in the same order.
    for block in &message.tool_results {
        blocks.push(tool_result_block(block));
    }

    // Anthropic requires every message to contain at least one content block.
    // If a caller hands us a fully-empty message we emit a single-space text
    // block so the request stays valid rather than failing with a cryptic
    // `content: [] is too short` error.
    if blocks.is_empty() {
        blocks.push(WireContentBlock::Text {
            text: " ".to_owned(),
            cache_control: None,
        });
    }

    WireMessage {
        role,
        content: blocks,
    }
}

fn tool_result_block(block: &ToolResultBlock) -> WireContentBlock {
    WireContentBlock::ToolResult {
        tool_use_id: block.tool_use_id.clone(),
        content: block.content.clone(),
        is_error: block.is_error,
        cache_control: None,
    }
}

fn tool_schemas() -> Vec<WireTool> {
    vec![
        WireTool {
            name: "Read",
            description: "Reads a file from the local filesystem and returns its contents with line numbers.\n- The file_path may be absolute or relative; relative paths resolve against the current workspace.\n- By default reads up to 2000 lines from the start of the file. Use offset (1-based) and limit to read a specific range; when you already know the part you need, read only that range.\n- Results are returned in \"line number + tab\" format - everything after the tab is the actual file content. Never include the line-number prefix when constructing old_string for the Edit tool.\n- This tool reads text files only, not directories. To list a directory use Glob; to search inside files use Grep.\n- Output is self-bounded and never silently cropped: if a range is too large the tool returns a clear error - retry with a smaller offset/limit, or use Grep to locate content first. When the default 2000-line limit is reached, the result reports the next offset to continue from.\n- If the file does not exist, the error suggests a Glob pattern to locate it.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the file to read."},
                    "offset": {"type": "integer", "description": "Line number to start reading from. Defaults to 1."},
                    "limit": {"type": "integer", "description": "Number of lines to read."},
                    "pages": {"type": "string", "description": "Page range for PDF files, reserved for PDF-capable readers."}
                },
                "required": ["file_path"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "FileMetadata",
            description: "Inspect file metadata without reading file contents.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the file or directory. Relative paths resolve against the current workspace."},
                    "include_xattrs": {"type": "boolean", "description": "Whether to include extended attribute names where supported."},
                    "include_spotlight": {"type": "boolean", "description": "Whether to include Spotlight metadata where supported."}
                },
                "required": ["file_path"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "ReadFiles",
            description: "Reads multiple text files in one ordered result.\n- Provide paths as an array of file paths; relative paths resolve against the current workspace.\n- Up to 50 files per call; split larger sets into multiple calls.\n- Each file is returned under a \"===== path =====\" separator with line numbers, up to 2000 lines per file.\n- The combined output is capped (~60000 estimated tokens); once the cap is reached the remaining files are skipped with a notice.\n- Prefer this over many parallel Read calls when inspecting several known paths before acting.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "paths": {
                        "type": "array",
                        "description": "Ordered list of file paths to read. Relative paths resolve against the current workspace.",
                        "items": {"type": "string"}
                    }
                },
                "required": ["paths"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Glob",
            description: "Fast file pattern matching that works with any codebase size.\n- Supports glob patterns like \"**/*.rs\" or \"src/**/*.ts\".\n- Returns matching file paths sorted by modification time.\n- Use this when you need to find files by name patterns before reading them.\n- Results are capped at 100 files; use a more specific path or pattern if the output is truncated.\n- For open-ended exploration that may need multiple rounds of globbing and grepping, use the Agent tool instead.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Glob pattern, for example **/*.rs"},
                    "path": {"type": "string", "description": "Directory to search. Defaults to the workspace root."}
                },
                "required": ["pattern"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Grep",
            description: "A powerful content search tool built on regular expressions.\n- ALWAYS use Grep for content search; never invoke grep or rg through Bash - Grep is optimized for correct permissions and output formatting.\n- Supports full regex syntax (e.g. \"log.*Error\", \"function\\\\s+\\\\w+\").\n- Filter files with the glob parameter (e.g. \"*.rs\") or the type parameter (e.g. \"rust\", \"py\", \"ts\") - type is more efficient than glob for standard file types.\n- Output modes: \"content\" shows matching lines (supports -A/-B/-C context and line numbers), \"files_with_matches\" shows file paths only (default), \"count\" shows match counts per file.\n- Pagination: head_limit caps results (default 250, 0 for unlimited); offset skips entries to paginate.\n- Pattern syntax is regex, not POSIX grep - literal braces need escaping (e.g. \"interface\\\\{\\\\}\").\n- Multiline: patterns match within a single line by default; set multiline=true for cross-line patterns.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Regular expression pattern to search for."},
                    "path": {"type": "string", "description": "File or directory to search. Defaults to the workspace root."},
                    "glob": {"type": "string", "description": "Optional glob pattern to filter files."},
                    "type": {"type": "string", "description": "File type to search (e.g. js, py, rust, go, ts). More convenient than glob for standard types."},
                    "output_mode": {"type": "string", "description": "files_with_matches, content, or count."},
                    "-B": {"type": "integer", "description": "Lines to show before each match (content mode only)."},
                    "-A": {"type": "integer", "description": "Lines to show after each match (content mode only)."},
                    "-C": {"type": "integer", "description": "Context lines before and after each match (content mode only)."},
                    "context": {"type": "integer", "description": "Alias for -C."},
                    "-n": {"type": "boolean", "description": "Show line numbers in content mode (default true)."},
                    "-i": {"type": "boolean", "description": "Case insensitive search."},
                    "multiline": {"type": "boolean", "description": "Enable multiline mode where . matches newlines and patterns can span lines."},
                    "head_limit": {"type": "integer", "description": "Limit output to first N entries. Use 0 for unlimited."},
                    "offset": {"type": "integer", "description": "Skip first N entries before applying head_limit for pagination. Defaults to 0."}
                },
                "required": ["pattern"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "WebFetch",
            description: "Fetches content from a URL, converts HTML to readable text, and returns it with response diagnostics.\n- The url must be a fully-formed http or https URL; the prompt describes what to extract from the page.\n- This tool is read-only and does not modify files.\n- The result includes the HTTP status, the final URL after redirects, the content type, and selected response headers.\n- Non-2xx responses are returned as tool errors with status, headers, final URL, and a short diagnostic - use those details to diagnose failures rather than assuming the page was fetched.\n- When a URL redirects to a different host, the final URL is reported; make a new WebFetch call with it if you need the redirected content.\n- For GitHub URLs, prefer the gh CLI via Bash (e.g. gh pr view, gh issue view, gh api) - faster and structured.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "The URL to fetch content from."},
                    "prompt": {"type": "string", "description": "The prompt describing what to extract from the page."}
                },
                "required": ["url", "prompt"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "WebSearch",
            description: "Searches the web and returns results.\n- The query parameter is required and should be a descriptive search query.\n- Use allowed_domains and blocked_domains to restrict or exclude result domains.\n- Requires configured Brave Search or Serper credentials.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "The search query to use.", "minLength": 2},
                    "allowed_domains": {
                        "type": "array",
                        "description": "Only include search results from these domains.",
                        "items": {"type": "string"}
                    },
                    "blocked_domains": {
                        "type": "array",
                        "description": "Exclude search results from these domains.",
                        "items": {"type": "string"}
                    }
                },
                "required": ["query"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "ToolSearch",
            description: "Search the live tool registry by name, purpose, or capability.\n- query: search keywords (required).\n- max_results: cap on results (default 10).\n- Use select:Read,Edit,Grep when you know exact tool names and want their summaries without fuzzy ranking.\n- Prefix required terms with +, e.g. \"+macos permission\" or \"+mcp resource\".\n- Searches CamelCase tool names, one-line summaries, and capability synonyms such as screenshot, clipboard, OCR, Spotlight, crash, and LSP.\n- Returns names, read/write and parallel-safety traits, and one-line summaries. Use it before guessing between related tools.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Search keywords, for example file edit or git commit. Use select:Read,Edit for exact tool summaries."},
                    "max_results": {"type": "integer", "description": "Maximum results to return. Defaults to 10 and is clamped to 1..40."}
                },
                "required": ["query"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "AskUserQuestion",
            description: "Ask the user clarifying questions when progress is blocked by missing information.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "questions": {
                        "type": "array",
                        "description": "Questions to ask the user (1-4 questions).",
                        "minItems": 1,
                        "maxItems": 4,
                        "items": {
                            "type": "object",
                            "properties": {
                                "question": {"type": "string", "description": "The complete question to ask the user."},
                                "header": {"type": "string", "description": "Very short label displayed as a chip/tag (max 12 chars)."},
                                "options": {
                                    "type": "array",
                                    "description": "The available choices for this question (2-4 options).",
                                    "minItems": 2,
                                    "maxItems": 4,
                                    "items": {
                                        "type": "object",
                                        "properties": {
                                            "label": {"type": "string", "description": "The display text for this option (1-5 words)."},
                                            "description": {"type": "string", "description": "Explanation of what this option means."}
                                        },
                                        "required": ["label", "description"]
                                    }
                                },
                                "multiSelect": {"type": "boolean", "description": "Set to true to allow multiple answers."}
                            },
                            "required": ["question", "header", "options", "multiSelect"]
                        }
                    }
                },
                "required": ["questions"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "MemorySearch",
            description: "Search Deeptide project/global memory files for durable preferences, decisions, and project facts.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Keyword or phrase to search for in Deeptide memory files."},
                    "scope": {"type": "string", "description": "Memory scope: project, global, or all. Defaults to all.", "enum": ["project", "global", "all"]},
                    "max_results": {"type": "integer", "description": "Maximum memory files to return. Defaults to 10 and is clamped to 1..50."}
                },
                "required": ["query"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "MemoryWrite",
            description: "Persist a concise auditable Deeptide memory shard for future sessions.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string", "description": "Short stable title for the memory, 3-8 words.", "minLength": 3, "maxLength": 80},
                    "body": {"type": "string", "description": "Concise, factual, durable memory content.", "minLength": 8, "maxLength": 2000},
                    "scope": {"type": "string", "description": "Where to save the memory. Use project for repository facts and global for user preferences.", "enum": ["project", "global"], "default": "project"},
                    "type": {"type": "string", "description": "Memory type.", "enum": ["user", "feedback", "project", "reference"], "default": "project"},
                    "reason": {"type": "string", "description": "Why this is worth remembering, for auditability.", "minLength": 3, "maxLength": 240}
                },
                "required": ["title", "body", "reason"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Agent",
            description: "Launch a specialized sub-agent for multi-step exploration or planning.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "description": {"type": "string", "description": "Short 3-5 word description of the task."},
                    "prompt": {"type": "string", "description": "Task for the agent to perform."},
                    "subagent_type": {"type": "string", "description": "Specialized agent type: general-purpose, Explore, or Plan.", "enum": ["general-purpose", "Explore", "Plan"]},
                    "model": {"type": "string", "description": "Optional model override for this sub-agent."},
                    "run_in_background": {"type": "boolean", "description": "Run asynchronously when supported by the interactive host."},
                    "isolation": {"type": "string", "description": "Optional worktree isolation for parallel-safe execution.", "enum": ["worktree"]}
                },
                "required": ["description", "prompt"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "MCP",
            description: "Forward a JSON-RPC call to an MCP server configured under settings.mcp_servers.\n- server: the server key in settings.mcp_servers (or mcpServers).\n- method: JSON-RPC method such as tools/call, resources/list, or prompts/get.\n- params: free-form JSON object passed to the server.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "server": {"type": "string", "description": "Name of the server in settings.mcp_servers or mcpServers."},
                    "method": {"type": "string", "description": "JSON-RPC method such as tools/call, resources/list, or prompts/get."},
                    "params": {"type": "object", "description": "Free-form JSON-RPC params object."}
                },
                "required": ["server", "method"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "ListMcpResources",
            description: "List resources exposed by configured MCP servers.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "server": {"type": "string", "description": "Optional server name; omit to list across all configured servers."}
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "ReadMcpResource",
            description: "Read a resource from a configured MCP server by URI.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "server": {"type": "string", "description": "Configured MCP server name."},
                    "uri": {"type": "string", "description": "Resource URI from ListMcpResources."}
                },
                "required": ["server", "uri"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "ListMcpPrompts",
            description: "List prompt templates exposed by configured MCP servers.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "server": {"type": "string", "description": "Optional server name; omit to list across all configured servers."}
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "GetMcpPrompt",
            description: "Fetch a prompt template from a configured MCP server by name.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "server": {"type": "string", "description": "Configured MCP server name."},
                    "name": {"type": "string", "description": "Prompt name from ListMcpPrompts."},
                    "arguments": {"type": "object", "description": "Prompt arguments if required by the prompt template."}
                },
                "required": ["server", "name"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Brief",
            description: "Request a context compaction summary of the conversation so far.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            cache_control: None,
        },
        WireTool {
            name: "CtxInspect",
            description: "Inspect context window usage, estimated remaining capacity, and cache expectations.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "model": {"type": "string", "description": "Optional model name used to estimate the context window."},
                    "estimated_tokens": {"type": "integer", "description": "Optional current token estimate supplied by the host."},
                    "message_count": {"type": "integer", "description": "Optional active message count supplied by the host."}
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "Snip",
            description: "Request aggressive trimming of older conversation history when context is overloaded.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "keepLast": {"type": "integer", "description": "Number of most recent messages to keep. Defaults to 10 and is clamped to 1..100."},
                    "explanation": {"type": "string", "description": "Brief explanation of why history is being trimmed."}
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "EnterPlanMode",
            description: "Enter plan mode before significant code changes: explore, design, and ask for approval before editing.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            cache_control: None,
        },
        WireTool {
            name: "ExitPlanMode",
            description: "Exit plan mode and present the plan for user approval before implementation.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "plan": {
                        "type": "string",
                        "description": "Optional plan content to present for review. Hosts may inject this when the plan was edited or stored outside the active transcript."
                    },
                    "planFilePath": {
                        "type": "string",
                        "description": "Optional path to the plan file presented to the user."
                    },
                    "planWasEdited": {
                        "type": "boolean",
                        "description": "Whether the plan was edited before approval."
                    },
                    "allowedPrompts": {
                        "type": "array",
                        "description": "Categories of actions needed to implement the plan.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "tool": {"type": "string", "description": "The tool this prompt applies to.", "enum": ["Bash"]},
                                "prompt": {"type": "string", "description": "Semantic description of the action."}
                            },
                            "required": ["tool", "prompt"]
                        }
                    }
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "Clipboard",
            description: "Read from or write to the system clipboard. Finder selection is available on macOS.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "inspect, read, files, finder_selection, or write.",
                        "enum": ["inspect", "read", "files", "finder_selection", "write"]
                    },
                    "content": {"type": "string", "description": "Text to write to the clipboard. Required for write operation."}
                },
                "required": ["operation"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "AudioTranscribe",
            description: "Transcribe a local audio file with a configured local speech backend.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the audio file. Relative paths resolve against the current workspace."},
                    "language_hint": {"type": "string", "description": "Optional BCP-47 language code such as zh-CN or en-US."}
                },
                "required": ["file_path"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "VideoTranscribe",
            description: "Extract and transcribe the audio track from a local video file.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the video file. Relative paths resolve against the current workspace."},
                    "language_hint": {"type": "string", "description": "Optional BCP-47 language code such as zh-CN or en-US."},
                    "allow_server": {"type": "boolean", "description": "Accepted for Swift schema parity. Rust does not upload media by default."}
                },
                "required": ["file_path"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "SpotlightSearch",
            description: "Fast macOS file discovery using the Spotlight metadata index.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Keyword or mdfind predicate to search for in file names, content, and metadata."},
                    "scope": {"type": "string", "description": "Directory scope. Relative paths resolve against the current workspace. Defaults to the workspace root."},
                    "names_only": {"type": "boolean", "description": "Search by filename only using mdfind -name. Defaults to false."},
                    "max_results": {"type": "integer", "description": "Maximum results to return. Defaults to 30 and is clamped to 1..200."}
                },
                "required": ["query"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "ScreenCapture",
            description: "List visible apps or capture a macOS window screenshot.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "description": "list or capture.", "enum": ["list", "capture"]},
                    "app_name": {"type": "string", "description": "For capture: partial app name to match. The Rust fallback currently requires window_id for capture."},
                    "window_id": {"type": "integer", "description": "For capture: exact window ID from a list operation or platform tool."},
                    "include_ocr": {"type": "boolean", "description": "Accepted for Swift schema parity. OCR is not implemented in the Rust fallback."},
                    "auto_trim": {"type": "boolean", "description": "Accepted for Swift schema parity. Use ImagePreprocess on the returned file for trimming."},
                    "enhance_text": {"type": "boolean", "description": "Accepted for Swift schema parity. Use ImagePreprocess on the returned file for text enhancement."},
                    "max_dimension": {"type": "integer", "description": "Accepted for Swift schema parity and clamped to 256..4096."}
                },
                "required": ["operation"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "LSP",
            description: "Code intelligence via the Language Server Protocol.\n\nOperations:\n- goToDefinition - jump to where a symbol is defined (needs line + character)\n- findReferences - find all usages of a symbol across the codebase (needs line + character)\n- hover - type signature and doc comment at a position (needs line + character)\n- documentSymbol - list all functions, classes, and structs in a file (needs line only)\n\nParameters:\n- file_path - source file path; relative paths resolve against the current workspace\n- line - 1-based line number, as shown in editors\n- character - 1-based column offset, required for goToDefinition, findReferences, and hover\n- operation - one of the four operations above\n\nPrefer LSP over Grep for symbol navigation when you need an exact definition location (not just name matches), all references, type information without reading whole files, or a quick outline of a file's symbols.\n\nResolves a language server by file type: sourcekit-lsp for Swift/Objective-C/C/C++, pyright for Python, typescript-language-server for JS/TS, rust-analyzer for Rust, gopls for Go. Install the appropriate language server; the tool returns a clear error if none is found on PATH.\n\nTip: use documentSymbol first to understand a file's structure, then goToDefinition/findReferences on specific symbols.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "description": "goToDefinition, findReferences, hover, or documentSymbol.",
                        "enum": ["goToDefinition", "findReferences", "hover", "documentSymbol"]
                    },
                    "file_path": {"type": "string", "description": "Path to the source file. Relative paths resolve against the current workspace."},
                    "line": {"type": "integer", "description": "Line number, 1-based as shown in editors."},
                    "character": {"type": "integer", "description": "Character offset, 1-based. Required for goToDefinition, findReferences, and hover."}
                },
                "required": ["operation", "file_path", "line"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "ImagePreprocess",
            description: "Inspect and preprocess local image files before visual analysis.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the image file. Relative paths resolve against the current workspace."},
                    "operation": {
                        "type": "string",
                        "description": "inspect or preprocess.",
                        "enum": ["inspect", "preprocess"]
                    },
                    "max_dimension": {"type": "integer", "description": "Largest output side in pixels for preprocess. Defaults to 1600 and is clamped to 256..4096."},
                    "auto_trim": {"type": "boolean", "description": "Crop likely blank border/background before resize."},
                    "enhance_text": {"type": "boolean", "description": "Grayscale, increase contrast, and sharpen for screenshots/text."},
                    "crop": {"type": "object", "description": "Optional normalized crop rectangle with x, y, width, height in 0..1, origin at top-left."},
                    "format": {"type": "string", "description": "Output image format for preprocess: png or jpeg.", "enum": ["png", "jpeg"]}
                },
                "required": ["file_path", "operation"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Vision",
            description: "Analyze local images and PDFs with OCR, layout extraction, or classification.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to an image or PDF file. Relative paths resolve against the current workspace."},
                    "operation": {
                        "type": "string",
                        "description": "ocr, layout, or classify.",
                        "enum": ["ocr", "layout", "classify"]
                    },
                    "pages": {"description": "PDF page number or range like 1-3. Defaults to the first page and caps ranges to five pages."},
                    "language_hints": {"type": "array", "items": {"type": "string"}, "description": "Optional OCR language hints such as eng or jpn. Passed to local OCR backends when available."},
                    "min_confidence": {"type": "number", "description": "Minimum OCR layout confidence from 0.0 to 1.0. Defaults to 0.5."}
                },
                "required": ["file_path", "operation"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "CrashLog",
            description: "Inspect local macOS DiagnosticReports for crash, hang, spin, panic, and ips reports.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "operation": {"type": "string", "description": "list, latest, or read.", "enum": ["list", "latest", "read"]},
                    "app_name": {"type": "string", "description": "Optional app/process name filter."},
                    "file_path": {"type": "string", "description": "Specific report path. Required for operation=read."},
                    "limit": {"type": "integer", "description": "Maximum reports to list. Defaults to 20 and is clamped to 1..100."},
                    "max_lines": {"type": "integer", "description": "Maximum lines to include when reading a report. Defaults to 160 and is clamped to 1..1000."}
                },
                "required": ["operation"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "MacLog",
            description: "Search recent macOS Unified Logging entries using bounded filters.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "process": {"type": "string", "description": "Optional process name filter."},
                    "subsystem": {"type": "string", "description": "Optional unified logging subsystem filter."},
                    "category": {"type": "string", "description": "Optional unified logging category filter."},
                    "contains": {"type": "string", "description": "Optional case-insensitive eventMessage filter."},
                    "level": {"type": "string", "description": "Log level filter.", "enum": ["error_or_fault", "fault", "error", "default", "info", "debug", "all"]},
                    "last_minutes": {"type": "integer", "description": "Lookback window in minutes. Defaults to 15 and is clamped to 1..1440."},
                    "limit": {"type": "integer", "description": "Maximum output lines. Defaults to 80 and is clamped to 1..300."},
                    "timeout_ms": {"type": "integer", "description": "Maximum time to wait for log show. Defaults to 8000 and is clamped to 1000..30000."}
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "MacDiagnose",
            description: "Build a focused macOS-native diagnostic route for local failures.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "scenario": {"type": "string", "description": "Problem class to diagnose.", "enum": ["general", "crash", "permission", "screen", "audio", "keychain", "network", "install", "performance"]},
                    "app_name": {"type": "string", "description": "Optional app/process name to focus on. Defaults to deeptide."},
                    "include_live_signals": {"type": "boolean", "description": "Accepted for Swift parity; Rust currently renders guidance without live native rows."}
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "CronCreate",
            description: "Schedule a prompt using a 5-field cron expression.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "cron": {"type": "string", "description": "5-field cron expression: minute hour day-of-month month day-of-week."},
                    "prompt": {"type": "string", "description": "Prompt to enqueue at each fire time."},
                    "recurring": {"type": "boolean", "description": "true repeats on schedule; false is one-shot. Omit to infer repeating schedules."}
                },
                "required": ["cron", "prompt"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "CronList",
            description: "List all scheduled cron jobs with IDs, schedules, and prompts.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            cache_control: None,
        },
        WireTool {
            name: "CronDelete",
            description: "Cancel a previously scheduled cron job by ID.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Job ID returned by CronCreate."}
                },
                "required": ["id"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "ReviewArtifact",
            description: "Mark a workspace file as needing human review.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the file to flag for review. Relative paths resolve against the current workspace."},
                    "reason": {"type": "string", "description": "Short context to help the human reviewer."}
                },
                "required": ["file_path"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Skill",
            description: "Invoke a named built-in skill by expanding its reusable prompt template.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "skill": {"type": "string", "description": "The skill name.", "enum": ["commit", "simplify", "review-pr", "init", "batch", "publish", "update-config"]},
                    "args": {"type": "string", "description": "Optional arguments for the skill."}
                },
                "required": ["skill"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "DiscoverSkills",
            description: "List every built-in Deeptide skill (name, description, optional when-to-use). Read-only. Use BEFORE calling Skill to discover the surface; avoids wasted tool calls on unknown skill names.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {},
                "additionalProperties": false
            }),
            cache_control: None,
        },
        WireTool {
            name: "Publish",
            description: "Prepare, inspect, or publish a static frontend to Paean Apps Square and clide.app.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dir": {"type": "string", "description": "Optional publish directory. Omit to auto-detect dist/build/out/.output/public/public/root."},
                    "title": {"type": "string", "description": "Optional Paean Apps Square title. Defaults to package.json name or project directory."},
                    "summary": {"type": "string", "description": "Optional Paean Apps Square summary. Defaults to package.json description."},
                    "category": {"type": "string", "description": "Optional Paean Apps Square category. Defaults to custom."},
                    "tags": {"type": "array", "items": {"type": "string"}, "description": "Optional Paean Apps Square tags."},
                    "allow_secrets": {"type": "boolean", "description": "Bypass high-confidence secret scan only when the user explicitly accepts the risk."},
                    "handle": {"type": "string", "description": "Legacy direct-publish handle. Only valid with delete."},
                    "delete": {"type": "boolean", "description": "Set true only to delete/unpublish a legacy direct publish handle."},
                    "dry_run": {"type": "boolean", "description": "Inspect publish directory, metadata, files, bytes, ignore rules, and secret scan without uploading."},
                    "status": {"type": "boolean", "description": "Show saved .clide/publish.json state without contacting the publish API."}
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "Remix",
            description: "Download the source of one or more published Paean Apps Square games by hash and scaffold a new game that remixes them.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "sources": {"type": "array", "items": {"type": "string"}, "description": "Published Square app references to remix. Each must resolve to a Square hashKey: bare hashKey, hashKey.8x.gg, 8x.gg URL, or hashKey=role to tag the borrowed aspect (e.g. h1=gameplay, h2=art, h3=theme). A *.clide.app play URL is a deployed handle, not necessarily the Square hashKey."},
                    "dir": {"type": "string", "description": "Optional target directory for the new project. Auto-derived from the title when omitted."},
                    "title": {"type": "string", "description": "Optional title for the new game (seeds clide.json)."},
                    "summary": {"type": "string", "description": "Optional summary (seeds clide.json)."},
                    "category": {"type": "string", "description": "Optional category (seeds clide.json)."},
                    "license": {"type": "string", "description": "Optional SPDX license id for the new project. Defaults to MIT."},
                    "dry_run": {"type": "boolean", "description": "Resolve and preview each source's metadata, target directory, and remix graph without downloading or writing files."}
                },
                "required": ["sources"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "RemoteTrigger",
            description: "POST a JSON payload to a configured remote webhook endpoint.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "payload": {"type": "string", "description": "Free-form payload string. Sent as {\"payload\":\"...\"} JSON unless override_body is set."},
                    "override_body": {"type": "string", "description": "Optional raw JSON string to use as the request body verbatim."}
                },
                "required": ["payload"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "PushNotification",
            description: "Post a native desktop notification when the user should be alerted.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "Notification body. Required and limited to 500 characters."},
                    "title": {"type": "string", "description": "Notification title. Defaults to deeptide."},
                    "subtitle": {"type": "string", "description": "Optional subtitle or second line."},
                    "sound": {"type": "boolean", "description": "Play the default notification sound where supported. Defaults to true."}
                },
                "required": ["message"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "NotebookEdit",
            description: "Edit Jupyter notebook cells by id or insert new cells.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "notebook_path": {"type": "string", "description": "Path to a .ipynb notebook file."},
                    "cell_id": {"type": "string", "description": "Cell id to replace/delete, or the cell after which to insert."},
                    "new_source": {"type": "string", "description": "New source text for replace or insert."},
                    "cell_type": {"type": "string", "description": "Cell type for replace or insert.", "enum": ["code", "markdown"]},
                    "edit_mode": {"type": "string", "description": "replace, insert, or delete.", "enum": ["replace", "insert", "delete"]}
                },
                "required": ["notebook_path", "new_source"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "EnterWorktree",
            description: "Create an isolated git worktree for parallel work.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Optional worktree branch name."}
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "ExitWorktree",
            description: "Keep or remove a git worktree and its branch.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "description": "keep or remove.", "enum": ["keep", "remove"]},
                    "path": {"type": "string", "description": "Path to the worktree. Defaults to the current workspace."}
                },
                "required": ["action"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "VerifyPlanExecution",
            description: "Verify that planned file changes appear in git status.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "expected_files": {"type": "array", "items": {"type": "string"}, "description": "File paths that should appear in the current git diff or untracked set."}
                }
            }),
            cache_control: None,
        },
        WireTool {
            name: "Sleep",
            description: "Wait for a bounded duration without running a shell command.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "duration_ms": {"type": "number", "description": "Duration to sleep in milliseconds, clamped to 0..300000."}
                },
                "required": ["duration_ms"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Write",
            description: "Writes a file to the local filesystem.\n- Required input keys are exactly file_path and content; do not call Write with path, filename, target, or an empty object.\n- Relative file_path values resolve against the current workspace. Use the exact output filename the user requested (e.g. summary.json, Dockerfile, report.md). For deliverables a user or verifier will inspect, prefer a relative path over /tmp unless that path was explicitly requested.\n- This tool overwrites an existing file at the path. Read existing files first, and prefer the Edit tool for targeted changes - only use Write to create new files or for full rewrites.\n- NEVER create documentation files (*.md) or README files unless explicitly requested by the user.\n- Only add emojis if the user explicitly requests it.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the file to create or replace."},
                    "content": {"type": "string", "description": "Complete file contents to write."}
                },
                "required": ["file_path", "content"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "AppendFile",
            description: "Append UTF-8 text to a file, creating it (with parent directories) if it does not exist.\n- Use this for INCREMENTAL file construction when a single Write would exceed your output token budget: emit a skeleton with Write, then call AppendFile repeatedly to add each subsequent section. This is strictly better than risking a truncated one-shot Write — partial JSON cannot be repaired.\n- Required input keys are exactly file_path and content; do not call AppendFile with path, filename, target, or an empty object.\n- Relative file_path values resolve against the current workspace.\n- The chunk is appended verbatim after line-ending normalization (\\r\\n / lone \\r → \\n).\n- By default a single \\n separator is inserted between the existing file tail and the new chunk only when the file does not already end in a newline; pass ensure_trailing_newline=false to disable this for byte-exact concatenation.\n- AppendFile is O(chunk_size) and safe to call many times against a growing file.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the file to extend (created if missing)."},
                    "content": {"type": "string", "description": "Chunk to append verbatim after line-ending normalization."},
                    "ensure_trailing_newline": {"type": "boolean", "description": "Insert a single '\\n' separator when the existing file does not already end in a newline. Defaults to true."}
                },
                "required": ["file_path", "content"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Edit",
            description: "Performs exact string replacements in files.\n- Relative file_path values resolve against the current workspace.\n- Read the file first so old_string matches the current content exactly. When copying from Read output, preserve the exact indentation (tabs/spaces) as it appears AFTER the line-number prefix (line number + tab) and never include any part of that prefix in old_string or new_string.\n- ALWAYS prefer editing existing files; never write new files unless explicitly required.\n- For incremental construction of LARGE NEW files where a one-shot Write would exceed your output budget, use AppendFile instead of Edit.\n- Only add emojis if the user explicitly requests it.\n- The edit FAILS if old_string is not unique: provide a larger surrounding string to make it unique, or set replace_all to change every occurrence.\n- If you see \"old_string not found\", the file may have changed since your last Read - re-read it and retry; this is a normal recovery path, not a dead end.\n- Use replace_all to rename a symbol across the whole file.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "file_path": {"type": "string", "description": "Path to the file to modify."},
                    "old_string": {"type": "string", "description": "Exact text to replace."},
                    "new_string": {"type": "string", "description": "Replacement text."},
                    "replace_all": {"type": "boolean", "description": "Replace all occurrences. Defaults to false."}
                },
                "required": ["file_path", "old_string", "new_string"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Bash",
            description: "Executes a given shell command in the current workspace and returns its output.\n\nThe working directory persists between commands, but shell state does not. The shell environment is initialized from the user's profile (bash or zsh). The command field MUST be a single physical line - do not include newline characters, even for long git commits, heredocs, quoted strings, or multi-step workflows. Use && for dependent steps and ; for independent sequential steps.\n\nIMPORTANT: Avoid using this tool to run find, grep, cat, head, tail, sed, awk, or echo unless explicitly instructed, or after you have verified that a dedicated tool cannot accomplish the task. Prefer the dedicated tools - they give the user a better experience and make tool calls easier to review:\n- File search: use Glob (not find or ls)\n- Content search: use Grep (not grep or rg)\n- Read files: use Read (not cat/head/tail)\n- Edit files: use Edit (not sed/awk)\n- Write files: use Write (not echo > or cat <<EOF)\n- Communication: output text directly (not echo/printf)\n\nInstructions:\n- If the command creates new directories or files, first run ls to verify the parent directory exists and is the intended location.\n- Always quote file paths that contain spaces (e.g. cd \"path with spaces/file.txt\").\n- Maintain your working directory by using absolute paths and avoiding cd; never prepend cd <cwd> to a git command.\n- Set timeout (milliseconds, max 600000) for long-running builds.\n- With run_in_background=true the command is parked in the background and returns a shell_id; read its output later with BashOutput or stop it with KillBash.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Single-line shell command to execute."},
                    "timeout": {"type": "integer", "description": "Optional timeout in milliseconds, maximum 600000."},
                    "description": {"type": "string", "description": "Short description of what the command does."},
                    "run_in_background": {"type": "boolean", "description": "Start command and return immediately; output is captured and readable via BashOutput. Returns a shell_id for later BashOutput/KillBash calls."}
                },
                "required": ["command"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "BashOutput",
            description: "Read accumulated stdout/stderr of a background Bash invocation by shell_id. Use stdout_cursor/stderr_cursor from a previous BashOutput response to fetch only new output and avoid re-reading what you already saw.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "shell_id": {"type": "string", "description": "The shell_id returned by Bash with run_in_background=true."},
                    "stdout_cursor": {"type": "integer", "description": "Optional: return only stdout lines produced after this cursor. Pass back the stdout_cursor from your previous BashOutput response."},
                    "stderr_cursor": {"type": "integer", "description": "Optional: same as stdout_cursor but for stderr."}
                },
                "required": ["shell_id"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "KillBash",
            description: "SIGKILL a background Bash invocation by shell_id. Returns its final accumulated output. Idempotent — calling on an already-exited shell returns its recorded exit information without error.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "shell_id": {"type": "string", "description": "The shell_id returned by Bash with run_in_background=true."}
                },
                "required": ["shell_id"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Monitor",
            description: "Runs a long-running shell command and returns its recent output after a timeout or regex match.\n- command: shell command to run (required).\n- max_seconds: how long to monitor before returning (default 30, clamped to 5..300).\n- until: optional regular expression; return early when a stdout line matches.\n- Use this for log tailing, dev servers, and file watchers - situations where Bash's buffer-then-return model would block too long.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Shell command to run."},
                    "max_seconds": {"type": "integer", "description": "Max seconds to monitor before returning. Defaults to 30 and is clamped to 5..300."},
                    "until": {"type": "string", "description": "Optional regular expression. Return early when a stdout line matches."}
                },
                "required": ["command"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "TodoWrite",
            description: "Replace the complete todo list for the current task. Use this to track multi-step progress.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "todos": {
                        "type": "array",
                        "description": "The complete todo list to write. Replaces the previous list entirely.",
                        "items": {
                            "type": "object",
                            "properties": {
                                "content": {"type": "string", "description": "Todo item content."},
                                "status": {"type": "string", "description": "pending, in_progress, completed, or deleted."},
                                "activeForm": {"type": "string", "description": "Optional active phrasing for the current work."}
                            },
                            "required": ["content"]
                        }
                    }
                },
                "required": ["todos"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "TaskCreate",
            description: "Create one in-memory task with a subject and description.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "subject": {"type": "string", "description": "A brief title for the task."},
                    "description": {"type": "string", "description": "What needs to be done."},
                    "activeForm": {"type": "string", "description": "Present continuous form shown while active."},
                    "metadata": {"type": "object", "description": "Arbitrary metadata to attach to the task."}
                },
                "required": ["subject", "description"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "TaskList",
            description: "List the current in-memory todo tasks with status icons.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
            cache_control: None,
        },
        WireTool {
            name: "TaskGet",
            description: "Get full details for one in-memory todo task by ID. Use TaskList first to discover IDs.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "taskId": {"type": "string", "description": "The ID of the task to retrieve."}
                },
                "required": ["taskId"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "TaskUpdate",
            description: "Update an in-memory todo task by ID. Supports status, subject, description, and status=deleted.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "taskId": {"type": "string", "description": "The ID of the task to update."},
                    "status": {"type": "string", "description": "pending, in_progress, completed, or deleted."},
                    "subject": {"type": "string", "description": "New subject for the task."},
                    "description": {"type": "string", "description": "New description for the task."}
                },
                "required": ["taskId"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "TaskStop",
            description: "Stop a task by marking it completed.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "taskId": {"type": "string", "description": "The ID of the task to stop."},
                    "explanation": {"type": "string", "description": "Brief explanation of why the task is being stopped."}
                },
                "required": ["taskId"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "TaskOutput",
            description: "Retrieve recorded metadata and output for one task.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "task_id": {"type": "string", "description": "ID of the task whose output to fetch."},
                    "block": {"type": "boolean", "description": "Wait for completion. Currently accepted for forward compatibility."},
                    "timeout": {"type": "integer", "description": "Maximum wait time in milliseconds."}
                },
                "required": ["task_id"]
            }),
            cache_control: None,
        },
    ]
}

/// Protocol-neutral view of one tool's schema: its name, human description,
/// and JSON-Schema input contract. The Anthropic wire layer ([`WireTool`])
/// and any other protocol backend (OpenAI function-calling, Gemini
/// `functionDeclarations`, …) all derive their own envelope from this, so the
/// ~900-line [`tool_schemas`] table stays the single source of truth for what
/// the agent can do.
#[derive(Debug, Clone)]
pub struct ToolSchema {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: serde_json::Value,
}

/// The full built-in tool catalog as protocol-neutral [`ToolSchema`]s, in
/// declaration order. Non-Anthropic backends call this instead of
/// re-declaring tools, so adding a tool in one place lights it up across every
/// protocol.
pub fn tool_schema_catalog() -> Vec<ToolSchema> {
    tool_schemas()
        .into_iter()
        .map(|tool| ToolSchema {
            name: tool.name,
            description: tool.description,
            input_schema: tool.input_schema,
        })
        .collect()
}

/// Format an HTTP error body into a user-facing message. Shared with
/// non-Anthropic backends because the JSON error shapes (`{error:{message,
/// type}}`, `{message}`, `{detail}`, `{errors:[…]}`) and the auth / rate-limit
/// / overload hints are provider-agnostic enough to reuse verbatim.
pub(crate) fn classify_api_error(status: u16, body: &str, retry_after: Option<&str>) -> String {
    classify_error(status, body, retry_after)
}

fn messages_url(base_url: &str) -> String {
    format!("{}/v1/messages", base_url.trim_end_matches('/'))
}

fn parse_messages_response(body: &str, elapsed: Duration) -> Result<AgentResponse, String> {
    let response: MessagesResponse =
        serde_json::from_str(body).map_err(|error| format!("invalid model response: {error}"))?;
    let mut tool_calls = Vec::new();
    let content = response
        .content
        .into_iter()
        .filter_map(|block| match block {
            ResponseContentBlock::Text { text, .. } => Some(text),
            ResponseContentBlock::ToolUse { id, name, input } => {
                tool_calls.push(ToolCall::new(id, name, input));
                None
            }
            ResponseContentBlock::Other => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    Ok(AgentResponse {
        content,
        usage: response.usage.map(|usage| {
            AgentUsage::new(
                usage.input_tokens.unwrap_or(0),
                usage.output_tokens.unwrap_or(0),
                usage.cache_creation_input_tokens.unwrap_or(0),
                usage.cache_read_input_tokens.unwrap_or(0),
                elapsed.as_millis().try_into().unwrap_or(usize::MAX),
            )
        }),
        tool_calls,
    })
}

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    content: Vec<ResponseContentBlock>,
    usage: Option<ResponseUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ResponseContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct ResponseUsage {
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
    cache_creation_input_tokens: Option<usize>,
    cache_read_input_tokens: Option<usize>,
}

fn classify_error(status: u16, body: &str, retry_after: Option<&str>) -> String {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
    let message = parsed
        .as_ref()
        .and_then(error_message_from_json)
        .unwrap_or_else(|| body.trim().to_owned());
    let error_type = parsed.as_ref().and_then(error_type_from_json);
    let hint = api_error_hint(status, error_type, retry_after);

    if message.is_empty() {
        format!("API request failed with HTTP {status}{hint}")
    } else if let Some(error_type) = error_type {
        format!("API request failed with HTTP {status} ({error_type}): {message}{hint}")
    } else {
        format!("API request failed with HTTP {status}: {message}{hint}")
    }
}

fn error_message_from_json(value: &serde_json::Value) -> Option<String> {
    value
        .pointer("/error/message")
        .or_else(|| value.pointer("/message"))
        .or_else(|| value.pointer("/detail"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|message| !message.is_empty())
        .map(ToOwned::to_owned)
        .or_else(|| {
            value
                .pointer("/errors/0/message")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|message| !message.is_empty())
                .map(ToOwned::to_owned)
        })
}

fn error_type_from_json(value: &serde_json::Value) -> Option<&str> {
    value
        .pointer("/error/type")
        .or_else(|| value.pointer("/type"))
        .or_else(|| value.pointer("/errors/0/type"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|error_type| !error_type.is_empty())
}

fn api_error_hint(status: u16, error_type: Option<&str>, retry_after: Option<&str>) -> String {
    let is_auth = matches!(status, 401 | 403)
        || matches!(
            error_type,
            Some("authentication_error" | "permission_error")
        );
    let is_rate_limit = status == 429 || error_type == Some("rate_limit_error");
    // 503 and 529 are both transient upstream-overload statuses (see
    // is_retryable_overload); give them the same actionable hint.
    let is_overload = matches!(status, 503 | 529) || error_type == Some("overloaded_error");

    // A server-provided Retry-After lets us name the concrete wait instead of a
    // vague "retry later". The header is either delta-seconds or an HTTP-date.
    let when = retry_after.map(format_retry_after);

    if is_auth {
        String::from(" Check provider credentials and authentication mode.")
    } else if is_rate_limit {
        match when {
            Some(when) => format!(" Retry after {when} or reduce request rate."),
            None => String::from(" Retry after quota resets or reduce request rate."),
        }
    } else if is_overload {
        match when {
            Some(when) => {
                format!(" Retry after {when} or switch to another available provider/model.")
            }
            None => String::from(" Retry later or switch to another available provider/model."),
        }
    } else {
        String::new()
    }
}

/// Render a `Retry-After` header value for humans. A bare integer is
/// delta-seconds (formatted as `90s` / `1m30s`); anything else (an HTTP-date)
/// is passed through trimmed.
fn format_retry_after(raw: &str) -> String {
    let trimmed = raw.trim();
    match trimmed.parse::<u64>() {
        Ok(secs) if secs >= 60 => {
            let minutes = secs / 60;
            let seconds = secs % 60;
            if seconds == 0 {
                format!("{minutes}m")
            } else {
                format!("{minutes}m{seconds}s")
            }
        }
        Ok(secs) => format!("{secs}s"),
        Err(_) => trimmed.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        WireContentBlock, apply_thinking, build_messages_request, classify_error,
        filter_wire_tools, messages_url, parse_messages_response, tool_schemas,
    };
    use crate::{ConversationMessage, ThinkingConfig, ToolCall, ToolChoice, ToolResultBlock};
    use std::time::Duration;

    #[test]
    fn registry_and_schemas_list_identical_tools() {
        use std::collections::BTreeSet;
        let registered: BTreeSet<&str> = crate::ToolRegistry::with_builtin_tools()
            .names()
            .into_iter()
            .collect();
        let advertised: BTreeSet<&str> = tool_schemas().iter().map(|tool| tool.name).collect();

        // Every registered tool must be advertised to the model, and every
        // advertised tool must be runnable — otherwise the model is offered a
        // tool it cannot call, or a registered tool it is never told about.
        let registered_only: Vec<&&str> = registered.difference(&advertised).collect();
        let advertised_only: Vec<&&str> = advertised.difference(&registered).collect();
        assert!(
            registered_only.is_empty(),
            "registered tools missing an API schema: {registered_only:?}"
        );
        assert!(
            advertised_only.is_empty(),
            "API schemas with no registered tool: {advertised_only:?}"
        );
    }

    #[test]
    fn filter_wire_tools_restricts_to_allowed_set() {
        use std::collections::BTreeSet;
        let allowed = vec![String::from("Read"), String::from("Grep")];
        let filtered = filter_wire_tools(tool_schemas(), &allowed, true);

        let names: BTreeSet<&str> = filtered.iter().map(|tool| tool.name).collect();
        assert_eq!(names, ["Grep", "Read"].into_iter().collect());

        // Exactly one cache breakpoint, on the last surviving tool.
        let cached = filtered
            .iter()
            .filter(|tool| tool.cache_control.is_some())
            .count();
        assert_eq!(cached, 1);
        assert!(
            filtered
                .last()
                .expect("filtered list is non-empty")
                .cache_control
                .is_some()
        );
    }

    #[test]
    fn filter_wire_tools_without_caching_sets_no_marker() {
        let filtered = filter_wire_tools(tool_schemas(), &[String::from("Read")], false);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "Read");
        assert!(filtered[0].cache_control.is_none());
    }

    #[test]
    fn tool_schemas_have_unique_names() {
        use std::collections::BTreeSet;
        let schemas = tool_schemas();
        let unique: BTreeSet<&str> = schemas.iter().map(|tool| tool.name).collect();
        assert_eq!(
            unique.len(),
            schemas.len(),
            "duplicate tool name in API schemas"
        );
    }

    #[test]
    fn tool_schemas_are_well_formed() {
        for tool in tool_schemas() {
            assert!(!tool.name.trim().is_empty(), "tool has an empty name");
            assert!(
                !tool.description.trim().is_empty(),
                "tool {} has no description",
                tool.name
            );
            assert_eq!(
                tool.input_schema["type"], "object",
                "tool {} input_schema must be an object",
                tool.name
            );
            assert!(
                tool.input_schema.get("properties").is_some(),
                "tool {} input_schema must declare properties",
                tool.name
            );
        }
    }

    #[test]
    fn core_tool_descriptions_carry_detailed_guidance() {
        let tools = tool_schemas();
        let describe = |name: &str| {
            tools
                .iter()
                .find(|tool| tool.name == name)
                .map(|tool| tool.description)
                .unwrap_or_else(|| panic!("{name} tool schema should be declared"))
        };
        // Cloud-mode Swift sends each tool's detailed prompt() as the wire
        // description; these assertions lock the ported guidance so the rich
        // text is not silently reverted to terse one-liners.
        assert!(describe("Bash").contains("single physical line"));
        assert!(describe("Bash").contains("use Glob (not find or ls)"));
        assert!(describe("Glob").contains("capped at 100 files"));
        assert!(describe("Grep").contains("head_limit caps results (default 250"));
        assert!(describe("Edit").contains("old_string not found"));
        assert!(describe("Write").contains("NEVER create documentation files"));
        assert!(describe("Read").contains("line number + tab"));
        assert!(describe("Read").contains("up to 2000 lines"));
        assert!(describe("ReadFiles").contains("Up to 50 files per call"));
        assert!(describe("WebFetch").contains("final URL after redirects"));
        assert!(describe("WebSearch").contains("allowed_domains and blocked_domains"));
        assert!(describe("LSP").contains("documentSymbol"));
        assert!(describe("LSP").contains("rust-analyzer for Rust"));
        assert!(describe("Monitor").contains("clamped to 5..300"));
        assert!(describe("ToolSearch").contains("Prefix required terms with +"));
        assert!(describe("MCP").contains("settings.mcp_servers"));
    }

    #[test]
    fn messages_url_normalizes_trailing_slash() {
        assert_eq!(
            messages_url("https://api.example.com/"),
            "https://api.example.com/v1/messages"
        );
    }

    #[test]
    fn parses_text_and_usage_from_messages_response() {
        let response = parse_messages_response(
            r#"{
                "id": "msg_1",
                "type": "message",
                "role": "assistant",
                "content": [
                    {"type": "text", "text": "hello"},
                    {"type": "text", "text": "world"}
                ],
                "usage": {
                    "input_tokens": 11,
                    "output_tokens": 7,
                    "cache_creation_input_tokens": 3,
                    "cache_read_input_tokens": 5
                }
            }"#,
            Duration::from_millis(42),
        )
        .expect("response should parse");

        assert_eq!(response.content, "hello\nworld");
        assert!(response.tool_calls.is_empty());
        let usage = response.usage.expect("usage should be present");
        assert_eq!(usage.input_tokens, 11);
        assert_eq!(usage.output_tokens, 7);
        assert_eq!(usage.cache_create, 3);
        assert_eq!(usage.cache_read, 5);
        assert_eq!(usage.duration_ms, 42);
    }

    #[test]
    fn ignores_non_text_response_blocks() {
        let response = parse_messages_response(
            r#"{
                "content": [
                    {"type": "thinking", "thinking": "hidden"},
                    {"type": "text", "text": "visible"}
                ]
            }"#,
            Duration::from_millis(0),
        )
        .expect("response should parse");

        assert_eq!(response.content, "visible");
        assert!(response.usage.is_none());
    }

    #[test]
    fn parses_tool_use_blocks_from_messages_response() {
        let response = parse_messages_response(
            r#"{
                "content": [
                    {"type": "text", "text": "I will inspect the file."},
                    {"type": "tool_use", "id": "toolu_1", "name": "Read", "input": {"file_path": "README.md", "limit": 5}}
                ]
            }"#,
            Duration::from_millis(0),
        )
        .expect("response should parse");

        assert_eq!(response.content, "I will inspect the file.");
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].id, "toolu_1");
        assert_eq!(response.tool_calls[0].name, "Read");
        assert_eq!(response.tool_calls[0].input["file_path"], "README.md");
        assert_eq!(response.tool_calls[0].input["limit"], 5);
    }

    #[test]
    fn classifies_api_errors_with_type_and_action_hint() {
        let error = classify_error(
            429,
            r#"{"type":"error","error":{"type":"rate_limit_error","message":"Quota exceeded"}}"#,
            None,
        );

        assert_eq!(
            error,
            "API request failed with HTTP 429 (rate_limit_error): Quota exceeded Retry after quota resets or reduce request rate."
        );
    }

    #[test]
    fn classifies_alternate_api_error_shapes() {
        let auth = classify_error(
            401,
            r#"{"errors":[{"type":"authentication_error","message":"bad token"}]}"#,
            None,
        );
        assert_eq!(
            auth,
            "API request failed with HTTP 401 (authentication_error): bad token Check provider credentials and authentication mode."
        );

        let plain = classify_error(529, "upstream unavailable", None);
        assert_eq!(
            plain,
            "API request failed with HTTP 529: upstream unavailable Retry later or switch to another available provider/model."
        );
    }

    #[test]
    fn classifies_rate_limit_with_retry_after_seconds() {
        let error = classify_error(
            429,
            r#"{"error":{"type":"rate_limit_error","message":"slow down"}}"#,
            Some("90"),
        );
        assert_eq!(
            error,
            "API request failed with HTTP 429 (rate_limit_error): slow down Retry after 1m30s or reduce request rate."
        );

        // A sub-minute delay stays in seconds; a 503 overload uses the same hint.
        let overload = classify_error(503, "upstream busy", Some("20"));
        assert_eq!(
            overload,
            "API request failed with HTTP 503: upstream busy Retry after 20s or switch to another available provider/model."
        );
    }

    #[test]
    fn retry_after_passes_through_http_date_and_formats_seconds() {
        assert_eq!(super::format_retry_after("45"), "45s");
        assert_eq!(super::format_retry_after("120"), "2m");
        assert_eq!(super::format_retry_after("3661"), "61m1s");
        // Non-numeric values (HTTP-date form) are passed through trimmed.
        assert_eq!(
            super::format_retry_after(" Wed, 21 Oct 2025 07:28:00 GMT "),
            "Wed, 21 Oct 2025 07:28:00 GMT"
        );
    }

    #[test]
    fn messages_request_declares_core_mutating_tool_schemas() {
        let request = build_messages_request(
            "test-model",
            &[ConversationMessage::user("write a file")],
            100,
            None,
            &ToolChoice::Auto,
            false,
        );

        let write = request
            .tools
            .iter()
            .find(|tool| tool.name == "Write")
            .expect("Write tool schema should be declared");
        assert_eq!(
            write.input_schema["required"],
            serde_json::json!(["file_path", "content"])
        );

        let edit = request
            .tools
            .iter()
            .find(|tool| tool.name == "Edit")
            .expect("Edit tool schema should be declared");
        assert_eq!(
            edit.input_schema["required"],
            serde_json::json!(["file_path", "old_string", "new_string"])
        );

        let bash = request
            .tools
            .iter()
            .find(|tool| tool.name == "Bash")
            .expect("Bash tool schema should be declared");
        assert_eq!(
            bash.input_schema["required"],
            serde_json::json!(["command"])
        );

        let monitor = request
            .tools
            .iter()
            .find(|tool| tool.name == "Monitor")
            .expect("Monitor tool schema should be declared");
        assert_eq!(
            monitor.input_schema["required"],
            serde_json::json!(["command"])
        );

        let read_files = request
            .tools
            .iter()
            .find(|tool| tool.name == "ReadFiles")
            .expect("ReadFiles tool schema should be declared");
        assert_eq!(
            read_files.input_schema["required"],
            serde_json::json!(["paths"])
        );

        let file_metadata = request
            .tools
            .iter()
            .find(|tool| tool.name == "FileMetadata")
            .expect("FileMetadata tool schema should be declared");
        assert_eq!(
            file_metadata.input_schema["required"],
            serde_json::json!(["file_path"])
        );

        let grep = request
            .tools
            .iter()
            .find(|tool| tool.name == "Grep")
            .expect("Grep tool schema should be declared");
        assert!(grep.input_schema["properties"].get("offset").is_some());

        let web_fetch = request
            .tools
            .iter()
            .find(|tool| tool.name == "WebFetch")
            .expect("WebFetch tool schema should be declared");
        assert_eq!(
            web_fetch.input_schema["required"],
            serde_json::json!(["url", "prompt"])
        );

        let web_search = request
            .tools
            .iter()
            .find(|tool| tool.name == "WebSearch")
            .expect("WebSearch tool schema should be declared");
        assert_eq!(
            web_search.input_schema["required"],
            serde_json::json!(["query"])
        );

        let tool_search = request
            .tools
            .iter()
            .find(|tool| tool.name == "ToolSearch")
            .expect("ToolSearch tool schema should be declared");
        assert_eq!(
            tool_search.input_schema["required"],
            serde_json::json!(["query"])
        );

        let ask_user = request
            .tools
            .iter()
            .find(|tool| tool.name == "AskUserQuestion")
            .expect("AskUserQuestion tool schema should be declared");
        assert_eq!(
            ask_user.input_schema["required"],
            serde_json::json!(["questions"])
        );

        let memory_search = request
            .tools
            .iter()
            .find(|tool| tool.name == "MemorySearch")
            .expect("MemorySearch tool schema should be declared");
        assert_eq!(
            memory_search.input_schema["required"],
            serde_json::json!(["query"])
        );

        let memory_write = request
            .tools
            .iter()
            .find(|tool| tool.name == "MemoryWrite")
            .expect("MemoryWrite tool schema should be declared");
        assert_eq!(
            memory_write.input_schema["required"],
            serde_json::json!(["title", "body", "reason"])
        );

        let exit_plan_mode = request
            .tools
            .iter()
            .find(|tool| tool.name == "ExitPlanMode")
            .expect("ExitPlanMode tool schema should be declared");
        assert_eq!(
            exit_plan_mode.input_schema["properties"]["plan"]["type"],
            serde_json::json!("string")
        );
        assert_eq!(
            exit_plan_mode.input_schema["properties"]["planFilePath"]["type"],
            serde_json::json!("string")
        );
        assert_eq!(
            exit_plan_mode.input_schema["properties"]["planWasEdited"]["type"],
            serde_json::json!("boolean")
        );
        assert_eq!(
            exit_plan_mode.input_schema["properties"]["allowedPrompts"]["items"]["properties"]["tool"]
                ["enum"],
            serde_json::json!(["Bash"]),
            "ExitPlanMode allowedPrompts.tool must be constrained to Bash to match Swift"
        );

        for name in [
            "Agent",
            "MCP",
            "ListMcpResources",
            "ReadMcpResource",
            "ListMcpPrompts",
            "GetMcpPrompt",
            "Brief",
            "CtxInspect",
            "Snip",
            "EnterPlanMode",
            "ExitPlanMode",
            "Clipboard",
            "AudioTranscribe",
            "VideoTranscribe",
            "SpotlightSearch",
            "ScreenCapture",
            "LSP",
            "ImagePreprocess",
            "Vision",
            "CrashLog",
            "MacLog",
            "MacDiagnose",
            "CronCreate",
            "CronList",
            "CronDelete",
            "ReviewArtifact",
            "Skill",
            "Publish",
            "Remix",
            "RemoteTrigger",
            "PushNotification",
            "NotebookEdit",
            "EnterWorktree",
            "ExitWorktree",
            "VerifyPlanExecution",
            "Sleep",
        ] {
            assert!(
                request.tools.iter().any(|tool| tool.name == name),
                "{name} tool schema should be declared"
            );
        }

        let todo_write = request
            .tools
            .iter()
            .find(|tool| tool.name == "TodoWrite")
            .expect("TodoWrite tool schema should be declared");
        assert_eq!(
            todo_write.input_schema["required"],
            serde_json::json!(["todos"])
        );

        let task_create = request
            .tools
            .iter()
            .find(|tool| tool.name == "TaskCreate")
            .expect("TaskCreate tool schema should be declared");
        assert_eq!(
            task_create.input_schema["required"],
            serde_json::json!(["subject", "description"])
        );

        assert!(
            request.tools.iter().any(|tool| tool.name == "TaskList"),
            "TaskList tool schema should be declared"
        );

        let task_get = request
            .tools
            .iter()
            .find(|tool| tool.name == "TaskGet")
            .expect("TaskGet tool schema should be declared");
        assert_eq!(
            task_get.input_schema["required"],
            serde_json::json!(["taskId"])
        );

        let task_update = request
            .tools
            .iter()
            .find(|tool| tool.name == "TaskUpdate")
            .expect("TaskUpdate tool schema should be declared");
        assert_eq!(
            task_update.input_schema["required"],
            serde_json::json!(["taskId"])
        );

        let task_stop = request
            .tools
            .iter()
            .find(|tool| tool.name == "TaskStop")
            .expect("TaskStop tool schema should be declared");
        assert_eq!(
            task_stop.input_schema["required"],
            serde_json::json!(["taskId"])
        );

        let task_output = request
            .tools
            .iter()
            .find(|tool| tool.name == "TaskOutput")
            .expect("TaskOutput tool schema should be declared");
        assert_eq!(
            task_output.input_schema["required"],
            serde_json::json!(["task_id"])
        );
    }

    #[test]
    fn assistant_tool_use_blocks_round_trip_through_wire_message() {
        let assistant = ConversationMessage::assistant_with_tool_calls(
            "I will inspect the file.",
            vec![ToolCall::new(
                "toolu_1",
                "Read",
                serde_json::json!({"file_path": "README.md", "limit": 5}),
            )],
        );

        let request = build_messages_request(
            "test-model",
            &[assistant],
            100,
            None,
            &ToolChoice::Auto,
            false,
        );
        let wire = request
            .messages
            .first()
            .expect("assistant message should be present");

        assert_eq!(wire.role, "assistant");
        assert_eq!(wire.content.len(), 2);

        match &wire.content[0] {
            WireContentBlock::Text { text, .. } => assert_eq!(text, "I will inspect the file."),
            other => panic!("expected leading text block, got {other:?}"),
        }
        match &wire.content[1] {
            WireContentBlock::ToolUse {
                id, name, input, ..
            } => {
                assert_eq!(id, "toolu_1");
                assert_eq!(name, "Read");
                assert_eq!(input["file_path"], "README.md");
                assert_eq!(input["limit"], 5);
            }
            other => panic!("expected tool_use block, got {other:?}"),
        }
    }

    #[test]
    fn assistant_message_with_only_tool_call_omits_empty_text_block() {
        let assistant = ConversationMessage::assistant_with_tool_calls(
            "",
            vec![ToolCall::new(
                "toolu_silent",
                "Read",
                serde_json::json!({"file_path": "README.md"}),
            )],
        );

        let request = build_messages_request(
            "test-model",
            &[assistant],
            100,
            None,
            &ToolChoice::Auto,
            false,
        );
        let wire = request.messages.first().expect("assistant message");

        assert_eq!(wire.content.len(), 1);
        assert!(matches!(wire.content[0], WireContentBlock::ToolUse { .. }));
    }

    #[test]
    fn tool_result_user_message_serialises_as_tool_result_blocks() {
        let user = ConversationMessage::tool_results(vec![
            ToolResultBlock::new("toolu_1", "alpha\n", false),
            ToolResultBlock::new("toolu_2", "boom", true),
        ]);

        let request =
            build_messages_request("test-model", &[user], 100, None, &ToolChoice::Auto, false);
        let wire = request.messages.first().expect("user message");
        assert_eq!(wire.role, "user");
        assert_eq!(wire.content.len(), 2);

        match &wire.content[0] {
            WireContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
                ..
            } => {
                assert_eq!(tool_use_id, "toolu_1");
                assert_eq!(content, "alpha\n");
                assert!(!*is_error);
            }
            other => panic!("expected tool_result block, got {other:?}"),
        }

        let serialised = serde_json::to_value(&wire.content[0]).expect("serialise");
        // is_error is skipped when false to match Anthropic's wire shape.
        assert!(serialised.get("is_error").is_none());

        match &wire.content[1] {
            WireContentBlock::ToolResult {
                tool_use_id,
                content,
                is_error,
                ..
            } => {
                assert_eq!(tool_use_id, "toolu_2");
                assert_eq!(content, "boom");
                assert!(*is_error);
            }
            other => panic!("expected tool_result block, got {other:?}"),
        }

        let error_serialised = serde_json::to_value(&wire.content[1]).expect("serialise");
        assert_eq!(error_serialised["is_error"], serde_json::json!(true));
    }

    #[test]
    fn empty_message_payload_falls_back_to_single_space_text_block() {
        let blank = ConversationMessage::assistant("");
        let request =
            build_messages_request("test-model", &[blank], 100, None, &ToolChoice::Auto, false);
        let wire = request.messages.first().expect("assistant message");
        assert_eq!(wire.content.len(), 1);
        match &wire.content[0] {
            WireContentBlock::Text { text, .. } => assert_eq!(text, " "),
            other => panic!("expected fallback text block, got {other:?}"),
        }
    }

    #[test]
    fn full_conversation_round_trip_preserves_tool_use_and_tool_result_pairing() {
        let messages = vec![
            ConversationMessage::user("read notes"),
            ConversationMessage::assistant_with_tool_calls(
                "",
                vec![ToolCall::new(
                    "toolu_a",
                    "Read",
                    serde_json::json!({"file_path": "notes.md"}),
                )],
            ),
            ConversationMessage::tool_results(vec![ToolResultBlock::new(
                "toolu_a",
                "file contents",
                false,
            )]),
            ConversationMessage::assistant("done"),
        ];

        let request =
            build_messages_request("test-model", &messages, 100, None, &ToolChoice::Auto, false);
        let roles: Vec<&str> = request.messages.iter().map(|msg| msg.role).collect();
        assert_eq!(roles, ["user", "assistant", "user", "assistant"]);

        // The synthesised tool-result turn must reference the same tool_use_id
        // that the preceding assistant turn declared. This is exactly the
        // constraint Anthropic enforces server-side.
        let assistant_call_id = match &request.messages[1].content[0] {
            WireContentBlock::ToolUse { id, .. } => id.clone(),
            other => panic!("expected tool_use in assistant turn, got {other:?}"),
        };
        let tool_result_id = match &request.messages[2].content[0] {
            WireContentBlock::ToolResult { tool_use_id, .. } => tool_use_id.clone(),
            other => panic!("expected tool_result in user turn, got {other:?}"),
        };
        assert_eq!(assistant_call_id, tool_result_id);
    }

    /// Anthropic only counts the LAST cache_control marker as the prefix
    /// boundary. We attach it to the final tool schema because tool schemas
    /// dwarf every other static block (~50K tokens). A wrong placement here
    /// would silently halve the cache hit rate, so guard it explicitly.
    #[test]
    fn prompt_caching_marks_only_last_tool_schema() {
        let request = build_messages_request(
            "test-model",
            &[ConversationMessage::user("hi")],
            100,
            None,
            &ToolChoice::Auto,
            true,
        );
        let last = request.tools.last().expect("at least one tool declared");
        assert!(
            last.cache_control.is_some(),
            "last tool should carry cache_control"
        );
        let marked = request
            .tools
            .iter()
            .filter(|t| t.cache_control.is_some())
            .count();
        assert_eq!(marked, 1, "exactly one cache breakpoint expected on tools");
    }

    #[test]
    fn prompt_caching_disabled_means_no_cache_control_markers() {
        let request = build_messages_request(
            "test-model",
            &[ConversationMessage::user("hi")],
            100,
            Some("you are a helpful assistant"),
            &ToolChoice::Auto,
            false,
        );
        assert!(request.tools.iter().all(|t| t.cache_control.is_none()));
        let system = request.system.as_ref().expect("system block emitted");
        assert!(system.iter().all(|block| block.cache_control.is_none()));
        // The conversation tail must also stay unmarked when caching is off.
        let tail = request.messages.last().expect("a message");
        assert!(
            tail.content.iter().all(|b| {
                serde_json::to_value(b).expect("serialise block")["cache_control"].is_null()
            }),
            "no message block may carry a breakpoint with caching disabled"
        );
    }

    #[test]
    fn prompt_caching_stamps_one_rolling_breakpoint_on_the_conversation_tail() {
        // A realistic multi-turn slice: user → assistant(tool_use) → user(tool_result).
        // The rolling conversation breakpoint must land on exactly ONE block —
        // the LAST block of the LAST message — so each turn writes a cache the
        // next turn reads. Earlier blocks/messages must stay unmarked (they're
        // covered by the read of the previous turn's cache).
        let messages = vec![
            ConversationMessage::user("read the file"),
            ConversationMessage::assistant_with_tool_calls(
                "",
                vec![ToolCall::new(
                    "toolu_1",
                    "Read",
                    serde_json::json!({"file_path": "a.txt"}),
                )],
            ),
            ConversationMessage::tool_results(vec![ToolResultBlock::new("toolu_1", "data", false)]),
        ];
        let request =
            build_messages_request("m", &messages, 100, Some("sys"), &ToolChoice::Auto, true);

        // Count every message content block that carries a breakpoint.
        let marked: usize = request
            .messages
            .iter()
            .flat_map(|m| &m.content)
            .filter(|b| {
                !serde_json::to_value(b).expect("serialise block")["cache_control"].is_null()
            })
            .count();
        assert_eq!(marked, 1, "exactly one rolling conversation breakpoint");

        // ...and it is specifically the final block of the final message.
        let last_msg = request.messages.last().expect("tail message");
        let last_block = last_msg.content.last().expect("tail block");
        let serialised = serde_json::to_value(last_block).expect("serialise");
        assert_eq!(
            serialised["cache_control"]["type"], "ephemeral",
            "tail block must hold the ephemeral marker: {serialised}"
        );

        // Total breakpoints across the whole request stay within Anthropic's 4:
        // tools(1) + system(1) + conversation(1) = 3.
        let tool_marks = request
            .tools
            .iter()
            .filter(|t| t.cache_control.is_some())
            .count();
        let system_marks = request
            .system
            .as_ref()
            .map(|s| s.iter().filter(|b| b.cache_control.is_some()).count())
            .unwrap_or(0);
        assert_eq!(tool_marks + system_marks + marked, 3, "<= 4 breakpoints");
    }

    #[test]
    fn system_prompt_emits_text_block_with_cache_control() {
        let request = build_messages_request(
            "test-model",
            &[ConversationMessage::user("hi")],
            100,
            Some("be concise"),
            &ToolChoice::Auto,
            true,
        );
        let system = request.system.as_ref().expect("system block emitted");
        assert_eq!(system.len(), 1);
        assert_eq!(system[0].block_type, "text");
        assert_eq!(system[0].text, "be concise");
        assert!(system[0].cache_control.is_some());

        let serialised = serde_json::to_value(&system[0]).expect("serialise system block");
        assert_eq!(serialised["type"], "text");
        assert_eq!(serialised["cache_control"]["type"], "ephemeral");
        assert!(
            serialised["cache_control"].get("ttl").is_none(),
            "1h TTL is intentionally not emitted in P2"
        );
    }

    #[test]
    fn whitespace_only_system_prompt_is_dropped() {
        let request = build_messages_request(
            "test-model",
            &[ConversationMessage::user("hi")],
            100,
            Some("   \n\t  "),
            &ToolChoice::Auto,
            true,
        );
        assert!(
            request.system.is_none(),
            "whitespace-only prompts must not produce a stale system block"
        );
    }

    #[test]
    fn missing_system_prompt_omits_system_field_entirely() {
        let request = build_messages_request(
            "test-model",
            &[ConversationMessage::user("hi")],
            100,
            None,
            &ToolChoice::Auto,
            true,
        );
        assert!(request.system.is_none());
        let payload = serde_json::to_value(&request).expect("serialise request");
        assert!(
            payload.get("system").is_none(),
            "absent system prompt should be omitted from JSON, not sent as null"
        );
    }

    #[test]
    fn tool_choice_round_trips_all_three_variants() {
        let cases = [
            (ToolChoice::Auto, serde_json::json!({"type": "auto"})),
            (ToolChoice::Any, serde_json::json!({"type": "any"})),
            (
                ToolChoice::Tool("Bash".to_owned()),
                serde_json::json!({"type": "tool", "name": "Bash"}),
            ),
        ];
        for (choice, expected) in cases {
            let request = build_messages_request(
                "test-model",
                &[ConversationMessage::user("go")],
                100,
                None,
                &choice,
                false,
            );
            let payload = serde_json::to_value(&request).expect("serialise request");
            assert_eq!(payload["tool_choice"], expected, "choice {choice:?}");
        }
    }

    #[test]
    fn anthropic_config_with_system_prompt_trims_blank() {
        use crate::AnthropicConfig;
        let cfg =
            AnthropicConfig::new("https://example.test", "key", "model").with_system_prompt("   ");
        assert!(cfg.system_prompt.is_none());

        let cfg = AnthropicConfig::new("https://example.test", "key", "model")
            .with_system_prompt("hello");
        assert_eq!(cfg.system_prompt.as_deref(), Some("hello"));
    }

    #[test]
    fn thinking_config_from_label_matches_swift_mapping() {
        assert_eq!(ThinkingConfig::from_label(""), None);
        assert_eq!(ThinkingConfig::from_label("auto"), None);
        assert_eq!(ThinkingConfig::from_label("DEFAULT"), None);
        assert_eq!(
            ThinkingConfig::from_label("off"),
            Some(ThinkingConfig::disabled())
        );
        assert_eq!(
            ThinkingConfig::from_label("none"),
            Some(ThinkingConfig::disabled())
        );
        assert_eq!(
            ThinkingConfig::from_label("low"),
            Some(ThinkingConfig::low())
        );
        assert_eq!(
            ThinkingConfig::from_label("Medium"),
            Some(ThinkingConfig::medium())
        );
        assert_eq!(
            ThinkingConfig::from_label("enabled"),
            Some(ThinkingConfig::medium())
        );
        assert_eq!(
            ThinkingConfig::from_label("high"),
            Some(ThinkingConfig::high())
        );
        // Unknown non-empty values default to medium, matching Swift.
        assert_eq!(
            ThinkingConfig::from_label("turbo"),
            Some(ThinkingConfig::medium())
        );

        assert_eq!(ThinkingConfig::low().budget_tokens, Some(4_000));
        assert_eq!(ThinkingConfig::medium().budget_tokens, Some(16_000));
        assert_eq!(ThinkingConfig::high().budget_tokens, Some(32_000));
        assert!(ThinkingConfig::high().is_enabled());
        assert!(!ThinkingConfig::disabled().is_enabled());
    }

    #[test]
    fn apply_thinking_sets_field_and_bumps_max_tokens() {
        let mut body = build_messages_request(
            "test-model",
            &[ConversationMessage::user("hi")],
            4_096,
            None,
            &ToolChoice::Auto,
            false,
        );
        apply_thinking(&mut body, Some(&ThinkingConfig::high()));

        // max_tokens must exceed the thinking budget (32000): 32000 + 4096.
        assert_eq!(body.max_tokens, 36_096);
        let payload = serde_json::to_value(&body).expect("serialise request");
        assert_eq!(payload["thinking"]["type"], "enabled");
        assert_eq!(payload["thinking"]["budget_tokens"], 32_000);
    }

    #[test]
    fn apply_thinking_disabled_keeps_max_tokens_and_omits_budget() {
        let mut body = build_messages_request(
            "test-model",
            &[ConversationMessage::user("hi")],
            4_096,
            None,
            &ToolChoice::Auto,
            false,
        );
        apply_thinking(&mut body, Some(&ThinkingConfig::disabled()));

        assert_eq!(body.max_tokens, 4_096);
        let payload = serde_json::to_value(&body).expect("serialise request");
        assert_eq!(payload["thinking"]["type"], "disabled");
        assert!(payload["thinking"].get("budget_tokens").is_none());
    }

    #[test]
    fn thinking_omitted_from_request_by_default() {
        let body = build_messages_request(
            "test-model",
            &[ConversationMessage::user("hi")],
            4_096,
            None,
            &ToolChoice::Auto,
            false,
        );
        let payload = serde_json::to_value(&body).expect("serialise request");
        assert!(
            payload.get("thinking").is_none(),
            "thinking must be omitted unless configured"
        );
    }
}
