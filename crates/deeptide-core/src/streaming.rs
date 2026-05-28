//! Anthropic [Messages streaming](https://docs.anthropic.com/en/api/messages-streaming)
//! SSE protocol parser.
//!
//! When `stream: true` is sent on a `/v1/messages` request, Anthropic
//! replies with a `text/event-stream` made of these event kinds:
//!
//! ```text
//! event: message_start         → message envelope (id, model, role, partial usage)
//! event: content_block_start   → new content block at index N (text or tool_use)
//! event: content_block_delta   → incremental delta (text_delta or input_json_delta)
//! event: content_block_stop    → block N is complete
//! event: message_delta         → final stop_reason + output_tokens
//! event: message_stop          → stream is complete
//! event: ping                  → keepalive; ignore
//! event: error                 → terminal error; surface to caller
//! ```
//!
//! On the wire each event is two lines, terminated by a blank line:
//! `event: NAME\ndata: <json>\n\n`. Multi-line `data:` fields are
//! concatenated with `\n` per the SSE spec, though Anthropic does not
//! currently use that form.
//!
//! This module owns:
//!   * [`StreamingEvent`] / [`StreamingHandler`] — caller-facing observation
//!     surface used to print live text or tool deltas in the REPL.
//!   * [`parse_streaming_response`] — the only public entry point. Reads
//!     SSE off any `Read`, dispatches handler callbacks, and returns the
//!     fully reassembled [`AgentResponse`] for the parent agent loop.
//!
//! The parser is deliberately tolerant of unknown event types: Anthropic
//! ships new event variants (e.g. `connector_text_delta` for the Connectors
//! beta) without bumping the API version. Unknown events are ignored rather
//! than failing the request — same policy zero-cli's parser uses.

use std::io::{BufRead, BufReader, Read};
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;

use crate::{AgentResponse, AgentUsage, ToolCall};

/// Caller-facing snapshot of one Anthropic streaming delta. Subset of the
/// raw protocol — only the things a UI actually needs to render in real
/// time. The full event JSON is intentionally not exposed because that
/// surface is unstable across Anthropic API revisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamingEvent {
    /// Initial message envelope. Carries the message id and the prompt-side
    /// token counts that arrive immediately at stream start.
    MessageStart {
        message_id: String,
        model: String,
        input_tokens: usize,
        cache_read_tokens: usize,
        cache_creation_tokens: usize,
    },
    /// A new text content block opened at this index. UIs typically start
    /// a fresh paragraph or clear a placeholder cursor here.
    TextBlockStart { index: usize },
    /// Token-level text delta for an open text block.
    TextDelta { index: usize, delta: String },
    /// A new tool_use block opened at this index. The model has committed
    /// to calling `name` but hasn't sent any arguments yet.
    ToolUseStart {
        index: usize,
        id: String,
        name: String,
    },
    /// Incremental fragment of the tool_use input JSON. Multiple deltas
    /// must be concatenated before they parse as JSON — Anthropic emits
    /// the JSON one chunk at a time and only the final concatenation is
    /// guaranteed to be well-formed.
    ToolUseInputDelta { index: usize, partial_json: String },
    /// Content block at this index is complete.
    BlockStop { index: usize },
    /// Final usage and stop reason, delivered right before `message_stop`.
    MessageDelta {
        stop_reason: Option<String>,
        output_tokens: Option<usize>,
    },
    /// Stream complete. No more events will arrive.
    MessageStop,
}

/// Shared callback for live streaming deltas. `Arc` rather than `Box`
/// because the REPL may share a single sink across nested sub-agent
/// backends, and `Send + Sync` so the handler is safe across the thread
/// reqwest uses for the blocking client.
pub type StreamingHandler = Arc<dyn Fn(&StreamingEvent) + Send + Sync>;

/// Parse an Anthropic SSE stream off `reader`, invoke `handler` for each
/// observed delta (if any), and assemble the final non-streaming
/// [`AgentResponse`] the agent loop expects.
///
/// `elapsed` is the wall time from the start of the request — recorded into
/// [`AgentUsage::duration_ms`] for cost telemetry parity with the
/// non-streaming code path.
pub fn parse_streaming_response<R: Read>(
    reader: R,
    handler: Option<&StreamingHandler>,
    elapsed: Duration,
) -> Result<AgentResponse, String> {
    let mut reader = BufReader::new(reader);
    let mut blocks: Vec<BlockAccumulator> = Vec::new();
    let mut usage = UsageAccumulator::default();
    let mut model: Option<String> = None;
    let mut saw_message_stop = false;

    while let Some(event) = read_sse_event(&mut reader)? {
        match event.event_type.as_str() {
            "ping" => {}
            "error" => {
                // Errors mid-stream arrive as `event: error` with an
                // `error: {type, message}` object inside `data:`. Surface
                // the same shape as the non-streaming classifier.
                let parsed: StreamingErrorEnvelope = serde_json::from_str(&event.data)
                    .map_err(|e| format!("malformed streaming error event: {e}"))?;
                return Err(format!(
                    "streaming error ({}): {}",
                    parsed.error.error_type, parsed.error.message
                ));
            }
            "message_start" => {
                let payload: MessageStartEvent = serde_json::from_str(&event.data)
                    .map_err(|e| format!("invalid message_start event: {e}"))?;
                if let Some(message_usage) = payload.message.usage.as_ref() {
                    usage.merge_input(message_usage);
                }
                model = payload.message.model.clone();
                if let Some(handler) = handler {
                    handler(&StreamingEvent::MessageStart {
                        message_id: payload.message.id.clone().unwrap_or_default(),
                        model: payload.message.model.clone().unwrap_or_default(),
                        input_tokens: payload
                            .message
                            .usage
                            .as_ref()
                            .and_then(|u| u.input_tokens)
                            .unwrap_or(0),
                        cache_read_tokens: payload
                            .message
                            .usage
                            .as_ref()
                            .and_then(|u| u.cache_read_input_tokens)
                            .unwrap_or(0),
                        cache_creation_tokens: payload
                            .message
                            .usage
                            .as_ref()
                            .and_then(|u| u.cache_creation_input_tokens)
                            .unwrap_or(0),
                    });
                }
            }
            "content_block_start" => {
                let payload: ContentBlockStartEvent = serde_json::from_str(&event.data)
                    .map_err(|e| format!("invalid content_block_start event: {e}"))?;
                ensure_index(&mut blocks, payload.index);
                match payload.content_block {
                    StreamContentBlock::Text { text } => {
                        blocks[payload.index] = BlockAccumulator::Text { text };
                        if let Some(handler) = handler {
                            handler(&StreamingEvent::TextBlockStart {
                                index: payload.index,
                            });
                        }
                    }
                    StreamContentBlock::ToolUse { id, name, input } => {
                        // Anthropic occasionally inlines a partial `input`
                        // object on block_start. Treat it as the first
                        // chunk of the JSON-accumulating buffer.
                        let initial = input
                            .as_ref()
                            .and_then(|v| serde_json::to_string(v).ok())
                            .filter(|s| !s.is_empty() && s != "{}")
                            .unwrap_or_default();
                        blocks[payload.index] = BlockAccumulator::ToolUse {
                            id: id.clone(),
                            name: name.clone(),
                            partial_input: initial,
                        };
                        if let Some(handler) = handler {
                            handler(&StreamingEvent::ToolUseStart {
                                index: payload.index,
                                id,
                                name,
                            });
                        }
                    }
                    StreamContentBlock::Other => {
                        // Unknown block type — keep the slot reserved so
                        // later block_stop events on the same index don't
                        // shift the assembled output. Treat as empty text
                        // so it serialises to nothing on the final pass.
                        blocks[payload.index] = BlockAccumulator::Text {
                            text: String::new(),
                        };
                    }
                }
            }
            "content_block_delta" => {
                let payload: ContentBlockDeltaEvent = serde_json::from_str(&event.data)
                    .map_err(|e| format!("invalid content_block_delta event: {e}"))?;
                ensure_index(&mut blocks, payload.index);
                match payload.delta {
                    StreamDelta::TextDelta { text } => {
                        if let BlockAccumulator::Text { text: buf } = &mut blocks[payload.index] {
                            buf.push_str(&text);
                        }
                        if let Some(handler) = handler {
                            handler(&StreamingEvent::TextDelta {
                                index: payload.index,
                                delta: text,
                            });
                        }
                    }
                    StreamDelta::InputJsonDelta { partial_json } => {
                        if let BlockAccumulator::ToolUse { partial_input, .. } =
                            &mut blocks[payload.index]
                        {
                            partial_input.push_str(&partial_json);
                        }
                        if let Some(handler) = handler {
                            handler(&StreamingEvent::ToolUseInputDelta {
                                index: payload.index,
                                partial_json,
                            });
                        }
                    }
                    StreamDelta::Other => {}
                }
            }
            "content_block_stop" => {
                let payload: IndexEvent = serde_json::from_str(&event.data)
                    .map_err(|e| format!("invalid content_block_stop event: {e}"))?;
                if let Some(handler) = handler {
                    handler(&StreamingEvent::BlockStop {
                        index: payload.index,
                    });
                }
            }
            "message_delta" => {
                let payload: MessageDeltaEvent = serde_json::from_str(&event.data)
                    .map_err(|e| format!("invalid message_delta event: {e}"))?;
                if let Some(message_usage) = payload.usage.as_ref() {
                    usage.merge_output(message_usage);
                }
                if let Some(handler) = handler {
                    handler(&StreamingEvent::MessageDelta {
                        stop_reason: payload.delta.stop_reason.clone(),
                        output_tokens: payload.usage.as_ref().and_then(|u| u.output_tokens),
                    });
                }
            }
            "message_stop" => {
                saw_message_stop = true;
                if let Some(handler) = handler {
                    handler(&StreamingEvent::MessageStop);
                }
                break;
            }
            _ => {
                // Unknown event type — Anthropic adds new ones (e.g.
                // `connector_text_delta`) without bumping the API
                // version. Ignoring keeps forward compatibility.
            }
        }
    }

    if !saw_message_stop && blocks.is_empty() {
        return Err(String::from(
            "streaming response ended before any content was received",
        ));
    }

    let mut text_parts: Vec<String> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    for block in blocks {
        match block {
            BlockAccumulator::Empty => {}
            BlockAccumulator::Text { text } => {
                if !text.is_empty() {
                    text_parts.push(text);
                }
            }
            BlockAccumulator::ToolUse {
                id,
                name,
                partial_input,
            } => {
                let input: serde_json::Value = if partial_input.trim().is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::from_str(&partial_input).map_err(|e| {
                        format!(
                            "tool_use {name}#{id} input JSON did not assemble cleanly: {e}; partial={partial_input}"
                        )
                    })?
                };
                tool_calls.push(ToolCall::new(id, name, input));
            }
        }
    }

    let _ = model; // captured for completeness; not yet surfaced on AgentResponse.

    Ok(AgentResponse {
        content: text_parts.join("\n"),
        usage: Some(AgentUsage::new(
            usage.input_tokens,
            usage.output_tokens,
            usage.cache_creation_input_tokens,
            usage.cache_read_input_tokens,
            elapsed.as_millis().try_into().unwrap_or(usize::MAX),
        )),
        tool_calls,
    })
}

/// Accumulator state for one content block index. Slots stay `Empty` only
/// until we observe the corresponding `content_block_start`; we keep them
/// reserved so out-of-order indices stay distinct.
enum BlockAccumulator {
    Empty,
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        partial_input: String,
    },
}

fn ensure_index(blocks: &mut Vec<BlockAccumulator>, index: usize) {
    while blocks.len() <= index {
        blocks.push(BlockAccumulator::Empty);
    }
}

#[derive(Default)]
struct UsageAccumulator {
    input_tokens: usize,
    output_tokens: usize,
    cache_creation_input_tokens: usize,
    cache_read_input_tokens: usize,
}

impl UsageAccumulator {
    fn merge_input(&mut self, usage: &StreamUsage) {
        if let Some(value) = usage.input_tokens {
            self.input_tokens = value;
        }
        if let Some(value) = usage.cache_creation_input_tokens {
            self.cache_creation_input_tokens = value;
        }
        if let Some(value) = usage.cache_read_input_tokens {
            self.cache_read_input_tokens = value;
        }
        if let Some(value) = usage.output_tokens {
            self.output_tokens = value;
        }
    }

    fn merge_output(&mut self, usage: &StreamUsage) {
        if let Some(value) = usage.output_tokens {
            self.output_tokens = value;
        }
        // Anthropic re-sends final input/cache counts on message_delta as a
        // safety net for clients that missed message_start usage.
        if let Some(value) = usage.input_tokens {
            self.input_tokens = value;
        }
        if let Some(value) = usage.cache_read_input_tokens {
            self.cache_read_input_tokens = value;
        }
        if let Some(value) = usage.cache_creation_input_tokens {
            self.cache_creation_input_tokens = value;
        }
    }
}

#[derive(Debug)]
struct SseEvent {
    event_type: String,
    data: String,
}

/// Read one SSE event off the reader. Returns `Ok(None)` on EOF.
///
/// SSE framing: each event is a sequence of `field: value` lines
/// terminated by a blank line. Multiple `data:` lines within an event
/// are concatenated with `\n` per the SSE spec. Lines beginning with
/// `:` are comments (used for keepalives) and are ignored.
fn read_sse_event<R: BufRead>(reader: &mut R) -> Result<Option<SseEvent>, String> {
    let mut event_type: Option<String> = None;
    let mut data_parts: Vec<String> = Vec::new();
    let mut saw_any_line = false;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|e| format!("failed to read streaming response: {e}"))?;
        if bytes == 0 {
            // EOF before terminator. If we collected anything, surface it;
            // otherwise signal end-of-stream.
            if event_type.is_none() && data_parts.is_empty() && !saw_any_line {
                return Ok(None);
            }
            break;
        }
        saw_any_line = true;

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            // Blank line = event terminator.
            if event_type.is_some() || !data_parts.is_empty() {
                break;
            }
            // Empty line outside an event (e.g. after pings). Skip.
            continue;
        }

        if trimmed.starts_with(':') {
            continue;
        }

        let (field, value) = match trimmed.split_once(':') {
            Some((field, value)) => (field, value.strip_prefix(' ').unwrap_or(value)),
            None => continue,
        };

        match field {
            "event" => event_type = Some(value.to_owned()),
            "data" => data_parts.push(value.to_owned()),
            _ => {} // `id:`, `retry:`, etc. — unused here
        }
    }

    let event_type = event_type.unwrap_or_default();
    let data = data_parts.join("\n");
    if event_type.is_empty() && data.is_empty() {
        // Got nothing useful — let the caller continue reading.
        return read_sse_event(reader);
    }
    Ok(Some(SseEvent { event_type, data }))
}

#[derive(Debug, Deserialize)]
struct MessageStartEvent {
    message: StreamMessage,
}

#[derive(Debug, Deserialize)]
struct StreamMessage {
    id: Option<String>,
    model: Option<String>,
    usage: Option<StreamUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamUsage {
    input_tokens: Option<usize>,
    output_tokens: Option<usize>,
    cache_creation_input_tokens: Option<usize>,
    cache_read_input_tokens: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ContentBlockStartEvent {
    index: usize,
    content_block: StreamContentBlock,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum StreamContentBlock {
    #[serde(rename = "text")]
    Text {
        #[serde(default)]
        text: String,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: Option<serde_json::Value>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct ContentBlockDeltaEvent {
    index: usize,
    delta: StreamDelta,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum StreamDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct IndexEvent {
    index: usize,
}

#[derive(Debug, Deserialize)]
struct MessageDeltaEvent {
    delta: MessageDeltaInner,
    usage: Option<StreamUsage>,
}

#[derive(Debug, Deserialize)]
struct MessageDeltaInner {
    stop_reason: Option<String>,
    #[allow(dead_code)]
    stop_sequence: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamingErrorEnvelope {
    error: StreamingErrorBody,
}

#[derive(Debug, Deserialize)]
struct StreamingErrorBody {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Helper: drive `parse_streaming_response` with an in-memory SSE
    /// payload and capture every handler invocation in order.
    fn drive(
        payload: &str,
    ) -> (
        Result<AgentResponse, String>,
        Arc<Mutex<Vec<StreamingEvent>>>,
    ) {
        let observed: Arc<Mutex<Vec<StreamingEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&observed);
        let handler: StreamingHandler = Arc::new(move |event| {
            sink.lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(event.clone());
        });
        let result = parse_streaming_response(payload.as_bytes(), Some(&handler), Duration::ZERO);
        (result, observed)
    }

    #[test]
    fn assembles_text_response_from_multiple_deltas() {
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"model\":\"m\",\"usage\":{\"input_tokens\":11,\"output_tokens\":0,\"cache_read_input_tokens\":4,\"cache_creation_input_tokens\":7}}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\", world\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":2}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );

        let (result, observed) = drive(payload);
        let response = result.expect("stream should parse");
        assert_eq!(response.content, "Hello, world");
        assert!(response.tool_calls.is_empty());

        let usage = response.usage.expect("usage");
        assert_eq!(usage.input_tokens, 11);
        assert_eq!(usage.output_tokens, 2);
        assert_eq!(usage.cache_read, 4);
        assert_eq!(usage.cache_create, 7);

        // Observed handler events: MessageStart, TextBlockStart, 2x TextDelta,
        // BlockStop, MessageDelta, MessageStop.
        let observed = observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let kinds: Vec<&str> = observed
            .iter()
            .map(|event| match event {
                StreamingEvent::MessageStart { .. } => "MessageStart",
                StreamingEvent::TextBlockStart { .. } => "TextBlockStart",
                StreamingEvent::TextDelta { .. } => "TextDelta",
                StreamingEvent::ToolUseStart { .. } => "ToolUseStart",
                StreamingEvent::ToolUseInputDelta { .. } => "ToolUseInputDelta",
                StreamingEvent::BlockStop { .. } => "BlockStop",
                StreamingEvent::MessageDelta { .. } => "MessageDelta",
                StreamingEvent::MessageStop => "MessageStop",
            })
            .collect();
        assert_eq!(
            kinds,
            vec![
                "MessageStart",
                "TextBlockStart",
                "TextDelta",
                "TextDelta",
                "BlockStop",
                "MessageDelta",
                "MessageStop",
            ]
        );
    }

    #[test]
    fn reassembles_tool_use_input_from_json_chunks() {
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_2\",\"model\":\"m\",\"usage\":{\"input_tokens\":5,\"output_tokens\":0}}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"Read\",\"input\":{}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"file_path\\\":\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"\\\"README.md\\\"}\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":12}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );

        let (result, _) = drive(payload);
        let response = result.expect("stream should parse");
        assert_eq!(response.content, "");
        assert_eq!(response.tool_calls.len(), 1);
        let call = &response.tool_calls[0];
        assert_eq!(call.id, "toolu_1");
        assert_eq!(call.name, "Read");
        assert_eq!(call.input["file_path"], "README.md");
    }

    #[test]
    fn interleaved_text_and_tool_use_blocks_preserve_indices() {
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"I will read it.\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"Read\",\"input\":{}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"file_path\\\":\\\"x.md\\\"}\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );

        let (result, _) = drive(payload);
        let response = result.expect("stream should parse");
        assert_eq!(response.content, "I will read it.");
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].input["file_path"], "x.md");
    }

    #[test]
    fn ping_events_are_ignored() {
        let payload = concat!(
            ": this is an SSE comment keepalive\n\n",
            "event: ping\n",
            "data: {\"type\":\"ping\"}\n\n",
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: ping\n",
            "data: {\"type\":\"ping\"}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"hi\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );

        let (result, _) = drive(payload);
        let response = result.expect("stream should parse despite ping noise");
        assert_eq!(response.content, "hi");
    }

    #[test]
    fn streaming_error_event_is_surfaced() {
        let payload = concat!(
            "event: error\n",
            "data: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Try later\"}}\n\n",
        );
        let (result, _) = drive(payload);
        let err = result.expect_err("error event must surface");
        assert!(
            err.contains("overloaded_error") && err.contains("Try later"),
            "unexpected error surface: {err}"
        );
    }

    #[test]
    fn unknown_event_types_do_not_break_the_stream() {
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: brand_new_anthropic_beta_event\n",
            "data: {\"some\":\"future-shape\"}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (result, _) = drive(payload);
        let response = result.expect("unknown events must not fail the parse");
        assert_eq!(response.content, "ok");
    }

    #[test]
    fn unknown_content_block_type_keeps_index_slot_reserved() {
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"server_tool_use_v2\",\"id\":\"x\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"after\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (result, _) = drive(payload);
        let response = result.expect("unknown block types must not fail the parse");
        assert_eq!(response.content, "after");
        assert!(response.tool_calls.is_empty());
    }

    #[test]
    fn premature_stream_end_without_any_content_returns_error() {
        let result = parse_streaming_response(b"".as_ref(), None, Duration::ZERO);
        let err = result.expect_err("empty SSE stream must error");
        assert!(err.contains("ended before any content was received"));
    }

    #[test]
    fn malformed_tool_input_json_produces_actionable_error() {
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"Read\",\"input\":{}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"missing\\\":\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (result, _) = drive(payload);
        let err = result.expect_err("malformed JSON must surface");
        assert!(err.contains("tool_use") && err.contains("Read"));
    }

    #[test]
    fn multiline_data_field_is_joined_with_newlines() {
        // SSE spec joins multiple `data:` lines per event with `\n`.
        // Anthropic doesn't currently use this form, but we accept it.
        let payload = "event: message_start\n\
            data: {\"type\":\"message_start\",\n\
            data: \"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n\
            event: content_block_start\n\
            data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
            event: content_block_delta\n\
            data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"ok\"}}\n\n\
            event: content_block_stop\n\
            data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
            event: message_stop\n\
            data: {\"type\":\"message_stop\"}\n\n";
        let (result, _) = drive(payload);
        let response = result.expect("multiline data must parse");
        assert_eq!(response.content, "ok");
    }
}
