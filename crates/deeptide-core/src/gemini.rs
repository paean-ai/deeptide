//! Gemini-protocol backend (`generateContent` / `streamGenerateContent`).
//!
//! The third wire protocol behind the protocol-agnostic [`AgentBackend`] trait,
//! alongside Anthropic (`api.rs`) and OpenAI (`openai.rs`). Google's Gemini API
//! diverges the most of the three, so this module does the most translation:
//!
//!   * **Endpoint**: `{base}/v1beta/models/{model}:generateContent` (buffered)
//!     or `:streamGenerateContent?alt=sse` (streamed) — the model is in the URL,
//!     not the body.
//!   * **Auth**: `x-goog-api-key: <key>` header (not a bearer token).
//!   * **Messages**: `contents[].parts[]` with role `user` / **`model`** (not
//!     `assistant`); the system prompt is a top-level `systemInstruction`.
//!   * **Tools**: `tools[].functionDeclarations[]`. A call is a
//!     `functionCall{name, args}` part where `args` is a JSON **object** (like
//!     Anthropic, unlike OpenAI's stringified args). A result is a
//!     `functionResponse{name, response}` part — keyed by **name, not id**
//!     (Gemini has no tool-call id), so we map our internal `tool_use_id` back
//!     to its tool name via the preceding `model` turn.
//!
//! It reuses [`crate::api::tool_schema_catalog`] (the single tool source of
//! truth) and emits the SAME [`StreamingEvent`]s as the other backends so the
//! TUI renders Gemini identically (live markdown, `↓ N tokens`, tool subjects).

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::api::tool_schema_catalog;
use crate::streaming::{StreamError, read_sse_data_event};
use crate::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ConversationMessage, MessageRole,
    StreamingEvent, StreamingHandler, ToolCall,
};

/// Default Gemini API root (the public Generative Language endpoint). Vertex AI
/// uses a different host shape; users point `--base-url` there when needed.
pub const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com";

/// Configuration for a Gemini `generateContent` backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeminiConfig {
    pub base_url: String,
    /// API key sent as the `x-goog-api-key` header.
    pub api_key: String,
    /// Model id, e.g. `gemini-2.0-flash` / `gemini-2.5-pro`. Goes in the URL.
    pub model: String,
    /// Output token ceiling (`generationConfig.maxOutputTokens`).
    pub max_tokens: usize,
    pub system_prompt: Option<String>,
    pub enable_streaming: bool,
    pub fallback_model: Option<String>,
}

impl GeminiConfig {
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

/// An [`AgentBackend`] speaking Google's Gemini `generateContent` protocol.
pub struct GeminiBackend {
    config: GeminiConfig,
    client: Client,
    streaming_handler: Option<StreamingHandler>,
    interrupt: Option<Arc<AtomicBool>>,
}

impl GeminiBackend {
    pub fn new(config: GeminiConfig) -> Result<Self, String> {
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

    pub fn config(&self) -> &GeminiConfig {
        &self.config
    }

    pub fn with_streaming_handler(mut self, handler: StreamingHandler) -> Self {
        self.streaming_handler = Some(handler);
        self
    }

    pub fn with_interrupt_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.interrupt = Some(flag);
        self
    }

    fn streaming_active(&self) -> bool {
        self.config.enable_streaming && self.streaming_handler.is_some()
    }

    fn is_interrupted(&self) -> bool {
        self.interrupt
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    fn try_model(
        &self,
        model: &str,
        request: &AgentRequest,
        effective_system: Option<&str>,
    ) -> Result<AgentResponse, GeminiFailure> {
        let started = Instant::now();
        let streaming = self.streaming_active();
        let body = build_generate_request(
            &request.messages,
            self.config.max_tokens,
            effective_system,
            request.allowed_tools.as_deref(),
        );
        let url = generate_url(&self.config.base_url, model, streaming);

        let response = self
            .client
            .post(&url)
            .header("x-goog-api-key", &self.config.api_key)
            .header(
                "accept",
                if streaming {
                    "text/event-stream"
                } else {
                    "application/json"
                },
            )
            .json(&body)
            .send()
            .map_err(|error| {
                let msg = error.to_string();
                let transient = error.is_timeout()
                    || error.is_connect()
                    || msg.contains("connection")
                    || msg.contains("reset");
                GeminiFailure {
                    transient,
                    message: format!("connection error: {error}"),
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let text = response
                .text()
                .unwrap_or_else(|e| format!("<failed to read error body: {e}>"));
            return Err(GeminiFailure {
                transient: is_retryable_overload(status.as_u16()),
                message: crate::api::classify_api_error(status.as_u16(), &text, None),
            });
        }

        if streaming {
            return parse_gemini_stream(
                response,
                self.streaming_handler.as_ref(),
                started.elapsed(),
                self.interrupt.as_deref(),
            )
            .map_err(|err| GeminiFailure {
                transient: matches!(err, StreamError::Truncated(_)),
                message: err.to_string(),
            });
        }

        let text = response.text().map_err(|error| GeminiFailure {
            transient: false,
            message: format!("failed to read response body: {error}"),
        })?;
        parse_generate_response(&text, started.elapsed()).map_err(|message| GeminiFailure {
            transient: false,
            message,
        })
    }
}

impl AgentBackend for GeminiBackend {
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
                if self.is_interrupted() {
                    return Err(String::from("stream cancelled by user"));
                }
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

struct GeminiFailure {
    transient: bool,
    message: String,
}

fn is_retryable_overload(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 529)
}

/// Build the `generateContent` / `streamGenerateContent` URL. The model and the
/// method are part of the path; streaming adds `?alt=sse` so the server emits
/// SSE framing instead of a JSON array.
fn generate_url(base_url: &str, model: &str, streaming: bool) -> String {
    let base = base_url.trim_end_matches('/');
    let method = if streaming {
        "streamGenerateContent"
    } else {
        "generateContent"
    };
    // Tolerate a base that already includes `/v1beta` (or `/v1`).
    let root = if base.ends_with("/v1beta") || base.ends_with("/v1") {
        base.to_owned()
    } else {
        format!("{base}/v1beta")
    };
    let suffix = if streaming { "?alt=sse" } else { "" };
    format!("{root}/models/{model}:{method}{suffix}")
}

// ─── Wire request types ──────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct GenerateRequest {
    contents: Vec<Content>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<SystemInstruction>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tools: Vec<GeminiToolBlock>,
    #[serde(rename = "generationConfig")]
    generation_config: GenerationConfig,
}

#[derive(Debug, Serialize)]
struct SystemInstruction {
    parts: Vec<Part>,
}

#[derive(Debug, Serialize)]
struct GenerationConfig {
    #[serde(rename = "maxOutputTokens")]
    max_output_tokens: usize,
}

#[derive(Debug, Serialize)]
struct Content {
    role: &'static str,
    parts: Vec<Part>,
}

/// One content part. Exactly one field is set; the rest are skipped. Serializes
/// to `{"text": …}`, `{"functionCall": …}`, or `{"functionResponse": …}`.
#[derive(Debug, Default, Serialize)]
struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    #[serde(rename = "functionCall", skip_serializing_if = "Option::is_none")]
    function_call: Option<FunctionCall>,
    #[serde(rename = "functionResponse", skip_serializing_if = "Option::is_none")]
    function_response: Option<FunctionResponse>,
}

#[derive(Debug, Serialize)]
struct FunctionCall {
    name: String,
    /// Arguments as a JSON **object** (Gemini, like Anthropic — unlike OpenAI's
    /// stringified args).
    args: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct FunctionResponse {
    name: String,
    /// Gemini wants the tool output wrapped in an object; we put the raw string
    /// under `result` (its conventional key).
    response: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct GeminiToolBlock {
    #[serde(rename = "functionDeclarations")]
    function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct FunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

fn build_generate_request(
    messages: &[ConversationMessage],
    max_tokens: usize,
    system_prompt: Option<&str>,
    allowed_tools: Option<&[String]>,
) -> GenerateRequest {
    // Gemini pairs a functionResponse to its functionCall by NAME, but our
    // ToolResultBlock carries only the tool_use_id. Walk the transcript once to
    // build id → name from every `model` (assistant) turn's tool_calls, so we
    // can recover the name when emitting tool results below.
    let mut id_to_name: HashMap<String, String> = HashMap::new();
    for message in messages {
        if message.role == MessageRole::Assistant {
            for call in &message.tool_calls {
                id_to_name.insert(call.id.clone(), call.name.clone());
            }
        }
    }

    let contents: Vec<Content> = messages
        .iter()
        .filter_map(|message| content_from(message, &id_to_name))
        .collect();

    let tools_decls: Vec<FunctionDeclaration> = tool_schema_catalog()
        .into_iter()
        .filter(|tool| match allowed_tools {
            Some(allowed) => allowed.iter().any(|name| name == tool.name),
            None => true,
        })
        .map(|tool| FunctionDeclaration {
            name: tool.name.to_owned(),
            description: tool.description.to_owned(),
            parameters: sanitize_schema(tool.input_schema),
        })
        .collect();

    let tools = if tools_decls.is_empty() {
        Vec::new()
    } else {
        vec![GeminiToolBlock {
            function_declarations: tools_decls,
        }]
    };

    let system_instruction = system_prompt
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|text| SystemInstruction {
            parts: vec![Part {
                text: Some(text.to_owned()),
                ..Default::default()
            }],
        });

    GenerateRequest {
        contents,
        system_instruction,
        tools,
        generation_config: GenerationConfig {
            max_output_tokens: max_tokens,
        },
    }
}

/// Convert one internal message into a Gemini `Content`, or `None` if it would
/// be empty. Roles map `Assistant → "model"`, `User → "user"`. Tool results
/// (which live on a `User` message in our model) become `functionResponse`
/// parts on a `user`-role content.
fn content_from(
    message: &ConversationMessage,
    id_to_name: &HashMap<String, String>,
) -> Option<Content> {
    let mut parts: Vec<Part> = Vec::new();

    if !message.content.is_empty() {
        parts.push(Part {
            text: Some(message.content.clone()),
            ..Default::default()
        });
    }

    match message.role {
        MessageRole::Assistant => {
            for call in &message.tool_calls {
                parts.push(Part {
                    function_call: Some(FunctionCall {
                        name: call.name.clone(),
                        args: call.input.clone(),
                    }),
                    ..Default::default()
                });
            }
            if parts.is_empty() {
                return None;
            }
            Some(Content {
                role: "model",
                parts,
            })
        }
        MessageRole::User => {
            for result in &message.tool_results {
                // Recover the tool name; fall back to the id if (somehow) the
                // matching call wasn't seen — Gemini still accepts an arbitrary
                // name, it just won't correlate as cleanly.
                let name = id_to_name
                    .get(&result.tool_use_id)
                    .cloned()
                    .unwrap_or_else(|| result.tool_use_id.clone());
                parts.push(Part {
                    function_response: Some(FunctionResponse {
                        name,
                        response: serde_json::json!({ "result": result.content }),
                    }),
                    ..Default::default()
                });
            }
            if parts.is_empty() {
                return None;
            }
            Some(Content {
                role: "user",
                parts,
            })
        }
    }
}

/// Gemini's function `parameters` accept only a subset of JSON Schema (an
/// OpenAPI 3.0 dialect) and reject unknown keywords like `$schema`,
/// `additionalProperties`, and `default`. Strip those recursively so our
/// Anthropic-shaped tool schemas are accepted. Conservative: only removes a
/// known-problematic keyword set, leaving everything else intact.
fn sanitize_schema(mut value: serde_json::Value) -> serde_json::Value {
    fn strip(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => {
                map.remove("$schema");
                map.remove("additionalProperties");
                map.remove("default");
                map.remove("$ref");
                for v in map.values_mut() {
                    strip(v);
                }
            }
            serde_json::Value::Array(items) => {
                for v in items {
                    strip(v);
                }
            }
            _ => {}
        }
    }
    strip(&mut value);
    value
}

// ─── Wire response types ─────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    candidates: Vec<Candidate>,
    #[serde(rename = "usageMetadata", default)]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    #[serde(default)]
    content: Option<ResponseContent>,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    #[serde(default)]
    parts: Vec<ResponsePart>,
}

#[derive(Debug, Deserialize)]
struct ResponsePart {
    #[serde(default)]
    text: Option<String>,
    /// Gemini thinking models flag a reasoning (chain-of-thought) part with
    /// `thought: true`; its `text` is the model's reasoning, NOT the answer.
    /// Surfaced live as `ThinkingDelta` and excluded from the answer content,
    /// mirroring the OpenAI `reasoning_content` path.
    #[serde(default)]
    thought: bool,
    #[serde(rename = "functionCall", default)]
    function_call: Option<ResponseFunctionCall>,
}

#[derive(Debug, Deserialize)]
struct ResponseFunctionCall {
    #[serde(default)]
    name: String,
    #[serde(default)]
    args: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    #[serde(rename = "promptTokenCount", default)]
    prompt_token_count: usize,
    #[serde(rename = "candidatesTokenCount", default)]
    candidates_token_count: usize,
    #[serde(rename = "cachedContentTokenCount", default)]
    cached_content_token_count: usize,
}

fn parse_generate_response(body: &str, elapsed: Duration) -> Result<AgentResponse, String> {
    let response: GenerateResponse =
        serde_json::from_str(body).map_err(|error| format!("invalid model response: {error}"))?;
    let (content, tool_calls) = assemble_parts(
        response
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content)
            .map(|c| c.parts)
            .unwrap_or_default(),
        0,
    );
    let usage = response.usage_metadata.map(|usage| {
        // `promptTokenCount` is the FULL prompt including the cached portion
        // (`cachedContentTokenCount`); subtract so the cost tracker prices the
        // cached tokens once at the cache rate, not also at the input rate.
        let uncached_input = usage
            .prompt_token_count
            .saturating_sub(usage.cached_content_token_count);
        AgentUsage::new(
            uncached_input,
            usage.candidates_token_count,
            0,
            usage.cached_content_token_count,
            elapsed.as_millis().try_into().unwrap_or(usize::MAX),
        )
    });
    Ok(AgentResponse {
        content,
        usage,
        tool_calls,
    })
}

/// Turn a list of response parts into (assembled text, tool_calls). `call_base`
/// seeds the synthetic id counter so streamed and buffered paths produce stable
/// ids. Gemini gives no call id, so we synthesize `gemini-call-N`.
fn assemble_parts(parts: Vec<ResponsePart>, call_base: usize) -> (String, Vec<ToolCall>) {
    let mut text = String::new();
    let mut tool_calls = Vec::new();
    for part in parts {
        if let Some(t) = part.text {
            // Reasoning parts (`thought: true`) are display-only; never let them
            // bleed into the assembled answer (buffered path has no live sink).
            if !part.thought {
                text.push_str(&t);
            }
        }
        if let Some(call) = part.function_call {
            let id = format!("gemini-call-{}", call_base + tool_calls.len());
            tool_calls.push(ToolCall::new(id, call.name, call.args));
        }
    }
    (text, tool_calls)
}

/// Parse a Gemini SSE stream (`:streamGenerateContent?alt=sse`). Each `data:`
/// line is a full `GenerateContentResponse` partial; text parts stream
/// incrementally, function-call parts arrive whole. Emits the same
/// [`StreamingEvent`]s as the other backends.
pub fn parse_gemini_stream<R: std::io::Read>(
    reader: R,
    handler: Option<&StreamingHandler>,
    elapsed: Duration,
    interrupt: Option<&AtomicBool>,
) -> Result<AgentResponse, StreamError> {
    let mut reader = std::io::BufReader::new(reader);
    let mut text = String::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    let mut input_tokens = 0usize;
    let mut output_tokens = 0usize;
    let mut cache_read = 0usize;
    let mut emitted_text_block = false;
    let mut saw_any = false;

    loop {
        if interrupt.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(StreamError::Interrupted);
        }
        let Some(data) = read_sse_data_event(&mut reader)? else {
            break;
        };
        let data = data.trim();
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let chunk: GenerateResponse = serde_json::from_str(data)
            .map_err(|e| StreamError::Protocol(format!("invalid Gemini stream chunk: {e}")))?;
        saw_any = true;

        if let Some(usage) = chunk.usage_metadata {
            output_tokens = usage.candidates_token_count;
            cache_read = usage.cached_content_token_count;
            // Subtract the cached portion from the full prompt count so cached
            // tokens aren't priced twice (mirrors the buffered path above).
            input_tokens = usage.prompt_token_count.saturating_sub(cache_read);
            if let Some(handler) = handler {
                handler(&StreamingEvent::MessageDelta {
                    stop_reason: None,
                    output_tokens: Some(output_tokens),
                });
            }
        }

        let parts = chunk
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content)
            .map(|c| c.parts)
            .unwrap_or_default();

        for part in parts {
            // A reasoning part (`thought: true`) streams live as ThinkingDelta
            // but is kept out of the answer `text` — display-only, not echoed
            // back next turn (mirrors the OpenAI reasoning_content path).
            if part.thought {
                if let Some(piece) = part.text.filter(|s| !s.is_empty())
                    && let Some(handler) = handler
                {
                    handler(&StreamingEvent::ThinkingDelta { delta: piece });
                }
                continue;
            }
            if let Some(piece) = part.text.filter(|s| !s.is_empty()) {
                if !emitted_text_block {
                    emitted_text_block = true;
                    if let Some(handler) = handler {
                        handler(&StreamingEvent::TextBlockStart { index: 0 });
                    }
                }
                text.push_str(&piece);
                if let Some(handler) = handler {
                    handler(&StreamingEvent::TextDelta {
                        index: 0,
                        delta: piece,
                    });
                }
            }
            if let Some(call) = part.function_call {
                let index = tool_calls.len() + 1; // +1 so it never collides with text block 0
                let id = format!("gemini-call-{}", tool_calls.len());
                if let Some(handler) = handler {
                    handler(&StreamingEvent::ToolUseStart {
                        index,
                        id: id.clone(),
                        name: call.name.clone(),
                    });
                    // Gemini sends complete args, but emit one input delta so the
                    // TUI's args ticker has something to show, then close it.
                    if let Ok(json) = serde_json::to_string(&call.args) {
                        handler(&StreamingEvent::ToolUseInputDelta {
                            index,
                            partial_json: json,
                        });
                    }
                    handler(&StreamingEvent::BlockStop { index });
                }
                tool_calls.push(ToolCall::new(id, call.name, call.args));
            }
        }
    }

    if !saw_any && text.is_empty() && tool_calls.is_empty() {
        return Err(StreamError::EmptyStream(String::from(
            "streaming response ended before any content was received",
        )));
    }

    if let Some(handler) = handler {
        handler(&StreamingEvent::MessageStop);
    }

    let usage = if input_tokens > 0 || output_tokens > 0 {
        Some(AgentUsage::new(
            input_tokens,
            output_tokens,
            0,
            cache_read,
            elapsed.as_millis().try_into().unwrap_or(usize::MAX),
        ))
    } else {
        None
    };

    Ok(AgentResponse {
        content: text,
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
    fn generate_url_buffered_and_streaming_shapes() {
        assert_eq!(
            generate_url(
                "https://generativelanguage.googleapis.com",
                "gemini-2.0-flash",
                false
            ),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"
        );
        assert_eq!(
            generate_url(
                "https://generativelanguage.googleapis.com",
                "gemini-2.0-flash",
                true
            ),
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:streamGenerateContent?alt=sse"
        );
        // Base already carrying /v1beta is not doubled.
        assert_eq!(
            generate_url("https://host/v1beta", "m", false),
            "https://host/v1beta/models/m:generateContent"
        );
    }

    #[test]
    fn system_prompt_becomes_system_instruction_not_a_content() {
        let req = build_generate_request(&[user("hi")], 1000, Some("be terse"), None);
        let si = req.system_instruction.expect("system instruction present");
        assert_eq!(si.parts[0].text.as_deref(), Some("be terse"));
        // The user message is the only content; system is NOT a content entry.
        assert_eq!(req.contents.len(), 1);
        assert_eq!(req.contents[0].role, "user");
    }

    #[test]
    fn assistant_role_maps_to_model_and_tool_calls_use_object_args() {
        let assistant = ConversationMessage::assistant_with_tool_calls(
            "",
            vec![ToolCall::new(
                "id-1",
                "Read",
                serde_json::json!({"file_path": "a.rs"}),
            )],
        );
        let req = build_generate_request(&[assistant], 100, None, None);
        assert_eq!(req.contents.len(), 1);
        assert_eq!(req.contents[0].role, "model");
        let call = req.contents[0].parts[0]
            .function_call
            .as_ref()
            .expect("function call part");
        assert_eq!(call.name, "Read");
        // args is an OBJECT, not a string.
        assert_eq!(call.args, serde_json::json!({"file_path": "a.rs"}));
    }

    #[test]
    fn tool_results_recover_name_by_id_and_become_function_response() {
        // A model turn calls "Bash" with id "c1", then a user turn answers it.
        let assistant = ConversationMessage::assistant_with_tool_calls(
            "",
            vec![ToolCall::new("c1", "Bash", serde_json::json!({}))],
        );
        let results = ConversationMessage::tool_results(vec![crate::ToolResultBlock::new(
            "c1", "out", false,
        )]);
        let req = build_generate_request(&[assistant, results], 100, None, None);
        // contents: [model(functionCall), user(functionResponse)]
        assert_eq!(req.contents.len(), 2);
        let fr = req.contents[1].parts[0]
            .function_response
            .as_ref()
            .expect("function response");
        // Name recovered from the preceding call's id→name mapping.
        assert_eq!(fr.name, "Bash");
        assert_eq!(fr.response, serde_json::json!({"result": "out"}));
    }

    #[test]
    fn sanitize_schema_strips_unsupported_keywords_recursively() {
        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "x": {"type": "string", "default": "y", "$ref": "#/defs/z"}
            }
        });
        let cleaned = sanitize_schema(schema);
        assert!(cleaned.get("$schema").is_none());
        assert!(cleaned.get("additionalProperties").is_none());
        let x = &cleaned["properties"]["x"];
        assert!(x.get("default").is_none());
        assert!(x.get("$ref").is_none());
        // Untouched keys survive.
        assert_eq!(x["type"], "string");
    }

    #[test]
    fn parse_response_extracts_text_tool_calls_and_usage() {
        let body = r#"{
            "candidates": [{"content": {"role": "model", "parts": [
                {"text": "let me check"},
                {"functionCall": {"name": "Bash", "args": {"command": "ls"}}}
            ]}}],
            "usageMetadata": {"promptTokenCount": 100, "candidatesTokenCount": 12, "cachedContentTokenCount": 30}
        }"#;
        let response = parse_generate_response(body, Duration::from_millis(5)).unwrap();
        assert_eq!(response.content, "let me check");
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "Bash");
        assert_eq!(
            response.tool_calls[0].input,
            serde_json::json!({"command": "ls"})
        );
        // Synthetic id since Gemini provides none.
        assert_eq!(response.tool_calls[0].id, "gemini-call-0");
        let usage = response.usage.unwrap();
        // promptTokenCount (100) includes the 30 cached → uncached input is 70
        // so the cached tokens aren't double-charged.
        assert_eq!(usage.input_tokens, 70);
        assert_eq!(usage.output_tokens, 12);
        assert_eq!(usage.cache_read, 30);
        assert_eq!(usage.input_tokens + usage.cache_read, 100);
    }

    #[test]
    fn buffered_thought_parts_are_excluded_from_the_answer() {
        // A Gemini thinking model returns a `thought: true` reasoning part plus
        // the real answer part. Only the answer lands in `content`.
        let body = r#"{
            "candidates": [{"content": {"role": "model", "parts": [
                {"text": "I should add 2 and 2.", "thought": true},
                {"text": "The answer is 4."}
            ]}}]
        }"#;
        let response = parse_generate_response(body, Duration::from_millis(1)).expect("parse");
        assert_eq!(response.content, "The answer is 4.");
        assert!(
            !response.content.contains("should add"),
            "reasoning must not leak into the answer: {:?}",
            response.content
        );
    }

    #[test]
    fn streamed_thought_parts_emit_thinking_delta_not_answer_text() {
        use std::sync::{Arc, Mutex};
        // Two SSE frames: a reasoning part (thought) then the answer.
        let payload = concat!(
            "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"reasoning here\",\"thought\":true}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"role\":\"model\",\"parts\":[{\"text\":\"final answer\"}]}}]}\n\n",
        );
        let events: Arc<Mutex<Vec<StreamingEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&events);
        let handler: StreamingHandler = Arc::new(move |e| {
            sink.lock()
                .unwrap_or_else(|p| p.into_inner())
                .push(e.clone());
        });
        let response =
            parse_gemini_stream(payload.as_bytes(), Some(&handler), Duration::ZERO, None)
                .expect("stream parses");
        // Answer excludes the reasoning.
        assert_eq!(response.content, "final answer");

        let events = events.lock().unwrap_or_else(|p| p.into_inner());
        let thoughts: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                StreamingEvent::ThinkingDelta { delta } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(thoughts, vec!["reasoning here"]);
        // No TextDelta carried the reasoning text.
        assert!(events.iter().all(|e| !matches!(
            e,
            StreamingEvent::TextDelta { delta, .. } if delta.contains("reasoning")
        )));
    }

    #[test]
    fn config_builders_normalize_blank_inputs() {
        let cfg = GeminiConfig::new("http://x", "k", "m")
            .with_system_prompt("  ")
            .with_fallback_model("");
        assert!(cfg.system_prompt.is_none());
        assert!(cfg.fallback_model.is_none());
    }
}
