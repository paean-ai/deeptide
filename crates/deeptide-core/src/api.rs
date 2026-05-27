use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::{AgentBackend, AgentRequest, AgentResponse, AgentUsage, MessageRole, ToolCall};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnthropicConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub max_tokens: usize,
    pub auth_mode: AnthropicAuthMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnthropicAuthMode {
    ApiKey,
    BearerToken,
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
        }
    }
}

pub struct AnthropicBackend {
    config: AnthropicConfig,
    client: Client,
}

impl AnthropicBackend {
    pub fn new(config: AnthropicConfig) -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self { config, client })
    }

    pub fn config(&self) -> &AnthropicConfig {
        &self.config
    }
}

impl AgentBackend for AnthropicBackend {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String> {
        let started = Instant::now();
        let body = build_messages_request(
            &self.config.model,
            request.messages.iter().map(|message| WireMessage {
                role: match message.role {
                    MessageRole::User => "user",
                    MessageRole::Assistant => "assistant",
                },
                content: vec![WireContentBlock::text(&message.content)],
            }),
            self.config.max_tokens,
        );
        let url = messages_url(&self.config.base_url);

        let request = self
            .client
            .post(url)
            .header("anthropic-version", "2023-06-01")
            .json(&body);
        let request = match self.config.auth_mode {
            AnthropicAuthMode::ApiKey => request.header("x-api-key", &self.config.api_key),
            AnthropicAuthMode::BearerToken => request.bearer_auth(&self.config.api_key),
        };
        let response = request
            .send()
            .map_err(|error| format!("connection error: {error}"))?;
        let status = response.status();
        let text = response
            .text()
            .map_err(|error| format!("failed to read response body: {error}"))?;

        if !status.is_success() {
            return Err(classify_error(status.as_u16(), &text));
        }

        parse_messages_response(&text, started.elapsed())
    }
}

#[derive(Debug, Serialize)]
struct MessagesRequest<'a> {
    model: &'a str,
    max_tokens: usize,
    messages: Vec<WireMessage<'a>>,
    tools: Vec<WireTool>,
    stream: bool,
}

#[derive(Debug, Serialize)]
struct WireMessage<'a> {
    role: &'a str,
    content: Vec<WireContentBlock<'a>>,
}

#[derive(Debug, Serialize)]
struct WireContentBlock<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    text: &'a str,
}

#[derive(Debug, Serialize)]
struct WireTool {
    name: &'static str,
    description: &'static str,
    input_schema: serde_json::Value,
}

impl<'a> WireContentBlock<'a> {
    fn text(text: &'a str) -> Self {
        Self { kind: "text", text }
    }
}

fn build_messages_request<'a>(
    model: &'a str,
    messages: impl IntoIterator<Item = WireMessage<'a>>,
    max_tokens: usize,
) -> MessagesRequest<'a> {
    MessagesRequest {
        model,
        max_tokens,
        messages: messages.into_iter().collect(),
        tools: tool_schemas(),
        stream: false,
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
        },
        WireTool {
            name: "Brief",
            description: "Request a context compaction summary of the conversation so far.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
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
        },
        WireTool {
            name: "EnterPlanMode",
            description: "Enter plan mode before significant code changes: explore, design, and ask for approval before editing.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        WireTool {
            name: "ExitPlanMode",
            description: "Exit plan mode and present the plan for user approval before implementation.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
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
        },
        WireTool {
            name: "CronList",
            description: "List all scheduled cron jobs with IDs, schedules, and prompts.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
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
        },
        WireTool {
            name: "TaskList",
            description: "List the current in-memory todo tasks with status icons.",
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
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
    let message = serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .pointer("/error/message")
                .or_else(|| value.pointer("/message"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_else(|| body.trim().to_owned());

    if message.is_empty() {
        format!("API request failed with HTTP {status}")
    } else {
        format!("API request failed with HTTP {status}: {message}")
    }
}

#[cfg(test)]
mod tests {
    use super::{WireMessage, build_messages_request, messages_url, parse_messages_response};
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
    fn messages_request_declares_core_mutating_tool_schemas() {
        let request = build_messages_request(
            "test-model",
            [WireMessage {
                role: "user",
                content: vec![super::WireContentBlock::text("write a file")],
            }],
            100,
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
}
