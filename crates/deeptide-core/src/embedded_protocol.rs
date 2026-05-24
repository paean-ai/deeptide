use std::fmt;

use serde::Serialize;
use serde_json::Value;

pub struct EmbeddedProtocol;

impl EmbeddedProtocol {
    pub fn parse_prompt(raw: &str) -> Result<String, EmbeddedProtocolError> {
        Ok(Self::parse(raw)?.prompt())
    }

    pub fn parse(raw: &str) -> Result<ParsedInput, EmbeddedProtocolError> {
        let mut parsed = ParsedInput::default();

        for (index, line) in raw.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let value: Value = serde_json::from_str(trimmed)
                .map_err(|_| EmbeddedProtocolError::InvalidLine(index + 1))?;
            let object = value
                .as_object()
                .ok_or(EmbeddedProtocolError::InvalidLine(index + 1))?;

            let kind = object.get("type").and_then(Value::as_str).unwrap_or("user");

            match kind {
                "user" | "user_text" => {
                    if should_skip_user_message(object) {
                        continue;
                    }
                    if let Some(text) = extract_text(object) {
                        parsed.user_texts.push(text);
                    }
                }
                "interrupt" => parsed.interrupt_requested = true,
                "resume" => {
                    parsed.resume_session_id = object
                        .get("session_id")
                        .or_else(|| object.get("sessionId"))
                        .and_then(Value::as_str)
                        .and_then(non_empty);
                }
                "permission_response" => {
                    parsed.permission_responses.push(PermissionResponse {
                        request_id: object
                            .get("request_id")
                            .or_else(|| object.get("requestId"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        tool_use_id: object
                            .get("tool_use_id")
                            .or_else(|| object.get("toolUseID"))
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        allowed: object
                            .get("allowed")
                            .or_else(|| object.get("approve"))
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        remember: object
                            .get("remember")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    });
                }
                "keep_alive" | "_spec" => {}
                _ => {}
            }
        }

        Ok(parsed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ParsedInput {
    pub user_texts: Vec<String>,
    pub interrupt_requested: bool,
    pub resume_session_id: Option<String>,
    pub permission_responses: Vec<PermissionResponse>,
}

impl ParsedInput {
    pub fn prompt(&self) -> String {
        self.user_texts.join("\n\n")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionResponse {
    pub request_id: Option<String>,
    pub tool_use_id: Option<String>,
    pub allowed: bool,
    pub remember: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmbeddedProtocolError {
    InvalidLine(usize),
}

impl fmt::Display for EmbeddedProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLine(line) => write!(f, "Invalid stream-json input at line {line}."),
        }
    }
}

impl std::error::Error for EmbeddedProtocolError {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EmbeddedProtocolSpec {
    #[serde(rename = "type")]
    kind: &'static str,
    version: &'static str,
    #[serde(rename = "input_format")]
    input_format: &'static str,
    #[serde(rename = "output_format")]
    output_format: &'static str,
    #[serde(rename = "supported_input_types")]
    supported_input_types: &'static [&'static str],
    #[serde(rename = "supported_output_types")]
    supported_output_types: &'static [&'static str],
}

impl EmbeddedProtocolSpec {
    pub const VERSION: &'static str = "deepclide.embedded.v1";
    pub const INPUT_FORMAT: &'static str = "stream-json";
    pub const OUTPUT_FORMAT: &'static str = "stream-json";
    pub const SUPPORTED_INPUT_TYPES: &'static [&'static str] = &[
        "user",
        "user_text",
        "interrupt",
        "resume",
        "permission_response",
        "keep_alive",
    ];
    pub const SUPPORTED_OUTPUT_TYPES: &'static [&'static str] = &[
        "_spec",
        "user",
        "assistant_delta",
        "assistant",
        "thinking_delta",
        "tool_progress",
        "tool_stream",
        "tool_diff",
        "tool_result",
        "tool_batch",
        "permission_request",
        "permission_mode",
        "retry",
        "compact",
        "terminal",
        "result",
        "error",
    ];
}

impl Default for EmbeddedProtocolSpec {
    fn default() -> Self {
        Self {
            kind: "_spec",
            version: Self::VERSION,
            input_format: Self::INPUT_FORMAT,
            output_format: Self::OUTPUT_FORMAT,
            supported_input_types: Self::SUPPORTED_INPUT_TYPES,
            supported_output_types: Self::SUPPORTED_OUTPUT_TYPES,
        }
    }
}

fn should_skip_user_message(object: &serde_json::Map<String, Value>) -> bool {
    object
        .get("isSynthetic")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || object
            .get("isReplay")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        || object.get("parent_tool_use_id").is_some()
        || object.get("tool_use_result").is_some()
}

fn extract_text(object: &serde_json::Map<String, Value>) -> Option<String> {
    object
        .get("prompt")
        .and_then(Value::as_str)
        .and_then(non_empty)
        .or_else(|| {
            object
                .get("text")
                .and_then(Value::as_str)
                .and_then(non_empty)
        })
        .or_else(|| object.get("content").and_then(extract_text_content))
        .or_else(|| {
            object
                .get("message")
                .and_then(Value::as_object)
                .and_then(|message| message.get("content"))
                .and_then(extract_text_content)
        })
}

fn extract_text_content(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => non_empty(text),
        Value::Array(blocks) => {
            let text = blocks
                .iter()
                .filter_map(|block| {
                    let object = block.as_object()?;
                    if object.get("type").and_then(Value::as_str) != Some("text") {
                        return None;
                    }
                    object
                        .get("text")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned)
                })
                .collect::<Vec<_>>()
                .join("\n");
            non_empty(&text)
        }
        _ => None,
    }
}

fn non_empty(text: &str) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        EmbeddedProtocol, EmbeddedProtocolError, EmbeddedProtocolSpec, PermissionResponse,
    };

    #[test]
    fn parses_user_text_aliases_and_content_blocks() {
        let raw = concat!(
            "{\"type\":\"user_text\",\"text\":\"first\"}\n",
            "{\"type\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"second\"},{\"type\":\"image\",\"source\":\"ignored\"}]}\n",
            "{\"type\":\"user\",\"message\":{\"content\":\"third\"}}\n"
        );

        let parsed = EmbeddedProtocol::parse(raw)
            .unwrap_or_else(|error| panic!("embedded input should parse: {error}"));

        assert_eq!(parsed.user_texts, vec!["first", "second", "third"]);
        assert_eq!(parsed.prompt(), "first\n\nsecond\n\nthird");
    }

    #[test]
    fn skips_replay_synthetic_and_tool_result_messages() {
        let raw = concat!(
            "{\"type\":\"user\",\"text\":\"keep\"}\n",
            "{\"type\":\"user\",\"text\":\"skip synthetic\",\"isSynthetic\":true}\n",
            "{\"type\":\"user\",\"text\":\"skip replay\",\"isReplay\":true}\n",
            "{\"type\":\"user\",\"text\":\"skip tool result\",\"tool_use_result\":{\"ok\":true}}\n",
            "{\"type\":\"assistant\",\"content\":\"ignored\"}\n"
        );

        let parsed = EmbeddedProtocol::parse(raw)
            .unwrap_or_else(|error| panic!("embedded input should parse: {error}"));

        assert_eq!(parsed.user_texts, vec!["keep"]);
    }

    #[test]
    fn parses_control_messages_for_future_long_lived_host() {
        let raw = concat!(
            "{\"type\":\"keep_alive\"}\n",
            "{\"type\":\"interrupt\"}\n",
            "{\"type\":\"resume\",\"session_id\":\"session-a\"}\n",
            "{\"type\":\"permission_response\",\"request_id\":\"perm-1\",\"tool_use_id\":\"tool-1\",\"allowed\":true,\"remember\":true}\n"
        );

        let parsed = EmbeddedProtocol::parse(raw)
            .unwrap_or_else(|error| panic!("embedded input should parse: {error}"));

        assert!(parsed.interrupt_requested);
        assert_eq!(parsed.resume_session_id.as_deref(), Some("session-a"));
        assert_eq!(
            parsed.permission_responses,
            vec![PermissionResponse {
                request_id: Some("perm-1".to_owned()),
                tool_use_id: Some("tool-1".to_owned()),
                allowed: true,
                remember: true,
            }]
        );
    }

    #[test]
    fn rejects_malformed_ndjson_with_line_number() {
        let error = EmbeddedProtocol::parse("{")
            .expect_err("malformed embedded input should return a line-numbered error");
        assert_eq!(error, EmbeddedProtocolError::InvalidLine(1));
        assert_eq!(error.to_string(), "Invalid stream-json input at line 1.");
    }

    #[test]
    fn protocol_spec_declares_stable_version_and_core_types() {
        assert_eq!(EmbeddedProtocolSpec::VERSION, "deepclide.embedded.v1");
        assert!(EmbeddedProtocolSpec::SUPPORTED_INPUT_TYPES.contains(&"permission_response"));
        assert!(EmbeddedProtocolSpec::SUPPORTED_INPUT_TYPES.contains(&"interrupt"));
        assert!(EmbeddedProtocolSpec::SUPPORTED_OUTPUT_TYPES.contains(&"permission_request"));
        assert!(EmbeddedProtocolSpec::SUPPORTED_OUTPUT_TYPES.contains(&"assistant_delta"));
    }
}
