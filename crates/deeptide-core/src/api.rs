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

        let response = self
            .client
            .post(url)
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
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
                    "limit": {"type": "integer", "description": "Number of lines to read."}
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
                    "head_limit": {"type": "integer", "description": "Limit output to first N entries. Use 0 for unlimited."}
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

        let read_files = request
            .tools
            .iter()
            .find(|tool| tool.name == "ReadFiles")
            .expect("ReadFiles tool schema should be declared");
        assert_eq!(
            read_files.input_schema["required"],
            serde_json::json!(["paths"])
        );

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

        let todo_write = request
            .tools
            .iter()
            .find(|tool| tool.name == "TodoWrite")
            .expect("TodoWrite tool schema should be declared");
        assert_eq!(
            todo_write.input_schema["required"],
            serde_json::json!(["todos"])
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
    }
}
