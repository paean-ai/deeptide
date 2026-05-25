use std::time::{Duration, Instant};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

use crate::{AgentBackend, AgentRequest, AgentResponse, AgentUsage, MessageRole};

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
        stream: false,
    }
}

fn messages_url(base_url: &str) -> String {
    format!("{}/v1/messages", base_url.trim_end_matches('/'))
}

fn parse_messages_response(body: &str, elapsed: Duration) -> Result<AgentResponse, String> {
    let response: MessagesResponse =
        serde_json::from_str(body).map_err(|error| format!("invalid model response: {error}"))?;
    let content = response
        .content
        .into_iter()
        .filter_map(|block| match block {
            ResponseContentBlock::Text { text, .. } => Some(text),
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
    use super::{messages_url, parse_messages_response};
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
}
