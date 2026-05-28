use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ConversationMessage, MessageRole,
    StreamingHandler, ToolCall, ToolResultBlock, streaming::parse_streaming_response,
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
            max_tokens: 4096,
            auth_mode: AnthropicAuthMode::ApiKey,
            system_prompt: None,
            tool_choice: ToolChoice::Auto,
            enable_prompt_caching: true,
            enable_streaming: false,
            fallback_model: None,
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
            max_tokens: 4096,
            auth_mode: AnthropicAuthMode::BearerToken,
            system_prompt: None,
            tool_choice: ToolChoice::Auto,
            enable_prompt_caching: true,
            enable_streaming: false,
            fallback_model: None,
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
}

pub struct AnthropicBackend {
    config: AnthropicConfig,
    client: Client,
    /// Caller-supplied sink for live streaming deltas. Only invoked when
    /// `config.enable_streaming == true`. Wrapped in `Arc<dyn Fn>` so the
    /// backend can be cheaply cloned into sub-agent factories while keeping
    /// every sub-agent pointed at the same output stream.
    streaming_handler: Option<StreamingHandler>,
}

impl AnthropicBackend {
    pub fn new(config: AnthropicConfig) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self {
            config,
            client,
            streaming_handler: None,
        })
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

        let primary = self.config.model.clone();
        match self.try_model(&primary, &request, effective_system.as_deref()) {
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
                        .try_model(&fallback, &request, effective_system.as_deref())
                        .map_err(|failure| failure.message);
                }
                Err(failure.message)
            }
        }
    }
}

/// A failed `/v1/messages` attempt, tagging the formatted error with the HTTP
/// status so the caller can decide whether to retry with a fallback model.
struct ApiFailure {
    /// HTTP status code, or `0` for transport-level errors (no response).
    status: u16,
    message: String,
}

/// Server-side overload statuses worth retrying with a different model.
/// `529` is Anthropic's "overloaded"; `503` is a generic upstream outage.
fn is_retryable_overload(status: u16) -> bool {
    matches!(status, 503 | 529)
}

impl AnthropicBackend {
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
        let response = req.send().map_err(|error| ApiFailure {
            status: 0,
            message: format!("connection error: {error}"),
        })?;
        let status = response.status();

        if !status.is_success() {
            let text = response.text().map_err(|error| ApiFailure {
                status: status.as_u16(),
                message: format!("failed to read response body: {error}"),
            })?;
            return Err(ApiFailure {
                status: status.as_u16(),
                message: classify_error(status.as_u16(), &text),
            });
        }

        if self.config.enable_streaming {
            // The reqwest blocking Response impls `Read`, which the SSE
            // parser consumes chunk-by-chunk — no need to buffer the whole
            // payload up front, so live deltas flow to the handler as the
            // model produces them.
            parse_streaming_response(response, self.streaming_handler.as_ref(), started.elapsed())
                .map_err(|message| ApiFailure {
                    status: status.as_u16(),
                    message,
                })
        } else {
            let text = response.text().map_err(|error| ApiFailure {
                status: status.as_u16(),
                message: format!("failed to read response body: {error}"),
            })?;
            parse_messages_response(&text, started.elapsed()).map_err(|message| ApiFailure {
                status: status.as_u16(),
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
    },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "is_false")]
        is_error: bool,
    },
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

    MessagesRequest {
        model,
        max_tokens,
        messages: messages.iter().map(wire_message_from).collect(),
        tools,
        stream: false,
        system,
        tool_choice: Some(WireToolChoice::from(tool_choice)),
    }
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
    }
}

fn tool_schemas() -> Vec<WireTool> {
    vec![
        WireTool {
            name: "Read",
            description: "Read a text file from the current workspace. Relative paths are resolved against the current working directory.",
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
            description: "Read multiple text files in one ordered result. Use this when inspecting several known paths.",
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
            description: "Find files by glob pattern. Use this when you need to discover file paths before reading.",
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
            description: "Search text files using a regular expression.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "pattern": {"type": "string", "description": "Regular expression pattern to search for."},
                    "path": {"type": "string", "description": "File or directory to search. Defaults to the workspace root."},
                    "glob": {"type": "string", "description": "Optional glob pattern to filter files."},
                    "output_mode": {"type": "string", "description": "files_with_matches, content, or count."},
                    "-i": {"type": "boolean", "description": "Case insensitive search."},
                    "head_limit": {"type": "integer", "description": "Limit output to first N entries. Use 0 for unlimited."},
                    "offset": {"type": "integer", "description": "Skip first N entries before applying head_limit for pagination. Defaults to 0."}
                },
                "required": ["pattern"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "WebFetch",
            description: "Fetch web content over HTTP or HTTPS and return readable text with response diagnostics.",
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
            description: "Search the web using configured Brave Search or Serper credentials.",
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
            description: "Search available tools by name or capability. Use select:ToolA,ToolB for exact summaries.",
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
            description: "Forward a JSON-RPC method to a configured MCP server.",
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
                                "tool": {"type": "string", "description": "The tool this prompt applies to."},
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
            description: "Code intelligence through a local Language Server Protocol server.",
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
            name: "Publish",
            description: "Prepare, inspect, or delete a static frontend publish on clide.app.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "dir": {"type": "string", "description": "Optional publish directory. Omit to auto-detect dist/build/out/.output/public/public/root."},
                    "handle": {"type": "string", "description": "Optional custom clide.app handle or remote publish handle."},
                    "random": {"type": "boolean", "description": "Set true only when the user explicitly asks for a new random handle."},
                    "delete": {"type": "boolean", "description": "Set true to delete/unpublish the saved or specified remote publish."},
                    "dry_run": {"type": "boolean", "description": "Inspect publish directory, handle, files, bytes, and ignore rules without uploading."},
                    "status": {"type": "boolean", "description": "Show saved .clide/publish.json state without contacting the publish API."}
                }
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
            description: "Write complete UTF-8 file contents to the current workspace. Use only when the user asked to create or replace a file.",
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
            name: "Edit",
            description: "Perform an exact string replacement in an existing file. Read the file first; old_string must match current contents exactly and be unique unless replace_all is true.",
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
            description: "Execute a single-line shell command in the current workspace. Prefer Read/Edit/Write/Glob/Grep for file work; use Bash for builds, tests, git, package managers, and shell-only operations.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Single-line shell command to execute."},
                    "timeout": {"type": "integer", "description": "Optional timeout in milliseconds, maximum 600000."},
                    "description": {"type": "string", "description": "Short description of what the command does."},
                    "run_in_background": {"type": "boolean", "description": "Start command and return immediately."}
                },
                "required": ["command"]
            }),
            cache_control: None,
        },
        WireTool {
            name: "Monitor",
            description: "Run a long command and return recent stdout/stderr after a timeout or regex match. Use for logs, watchers, and dev servers.",
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

fn classify_error(status: u16, body: &str) -> String {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok();
    let message = parsed
        .as_ref()
        .and_then(error_message_from_json)
        .unwrap_or_else(|| body.trim().to_owned());
    let error_type = parsed.as_ref().and_then(error_type_from_json);
    let hint = api_error_hint(status, error_type);

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

fn api_error_hint(status: u16, error_type: Option<&str>) -> &'static str {
    let Some(error_type) = error_type else {
        return match status {
            401 | 403 => " Check provider credentials and authentication mode.",
            429 => " Retry after quota resets or reduce request rate.",
            529 => " Retry later or switch to another available provider/model.",
            _ => "",
        };
    };

    match (status, error_type) {
        (401 | 403, _) | (_, "authentication_error" | "permission_error") => {
            " Check provider credentials and authentication mode."
        }
        (429, _) | (_, "rate_limit_error") => " Retry after quota resets or reduce request rate.",
        (529, _) | (_, "overloaded_error") => {
            " Retry later or switch to another available provider/model."
        }
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        WireContentBlock, build_messages_request, classify_error, messages_url,
        parse_messages_response,
    };
    use crate::{ConversationMessage, ToolCall, ToolChoice, ToolResultBlock};
    use std::time::Duration;

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
        );
        assert_eq!(
            auth,
            "API request failed with HTTP 401 (authentication_error): bad token Check provider credentials and authentication mode."
        );

        let plain = classify_error(529, "upstream unavailable");
        assert_eq!(
            plain,
            "API request failed with HTTP 529: upstream unavailable Retry later or switch to another available provider/model."
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
            WireContentBlock::Text { text } => assert_eq!(text, "I will inspect the file."),
            other => panic!("expected leading text block, got {other:?}"),
        }
        match &wire.content[1] {
            WireContentBlock::ToolUse { id, name, input } => {
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
            WireContentBlock::Text { text } => assert_eq!(text, " "),
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
}
