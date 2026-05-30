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

/// Prefix used on synthetic `MessageDelta.stop_reason` values that the
/// API layer emits to signal in-band events to the UI (currently just
/// stream-cut retries; future signals like model fallback can reuse the
/// same namespace). The colon separator keeps it parseable.
pub const STREAM_RETRY_SIGNAL_PREFIX: &str = "deeptide:stream-retry:";

/// Decoded payload of a [`STREAM_RETRY_SIGNAL_PREFIX`] sentinel.
///
/// The wire format is `deeptide:stream-retry:N/M (reason)`, where `N` is
/// the 1-indexed attempt that just failed and `M` is the total retry
/// budget. Both numbers come straight from the API layer's retry loop;
/// `reason` is the truncation error message we're about to retry past.
///
/// Stored as `usize` (not `u32`) because the call sites work in `usize`
/// for vector indexing and we want to avoid a cast at every render site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamRetryNotice {
    /// 1-indexed: the attempt that just failed.
    pub attempt: usize,
    /// Total attempts the API layer is willing to make (initial + retries).
    pub max_attempts: usize,
    /// User-facing reason for the retry (truncation error message).
    pub reason: String,
}

/// Try to decode a `MessageDelta.stop_reason` as a [`StreamRetryNotice`].
///
/// Returns `None` for any value that isn't one of our synthetic in-band
/// signals — including real model-emitted reasons like `"end_turn"` or
/// `"tool_use"`. The strict prefix-check keeps the sentinel namespace
/// from colliding with future Anthropic stop_reason values.
///
/// Format-fault tolerant: if a future API version emits the prefix with
/// a malformed payload (missing slash, non-numeric, etc.), we return
/// `None` rather than panicking, so the REPL silently falls back to
/// ignoring the event.
pub fn parse_stream_retry_signal(stop_reason: &str) -> Option<StreamRetryNotice> {
    let rest = stop_reason.strip_prefix(STREAM_RETRY_SIGNAL_PREFIX)?;
    // Split on the first space to separate the "N/M" header from the
    // free-form "(reason)" suffix. The reason is optional so a future
    // signal without a payload still parses cleanly.
    let (numeric, reason) = match rest.split_once(' ') {
        Some((numeric, tail)) => (
            numeric,
            tail.trim()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .to_owned(),
        ),
        None => (rest, String::new()),
    };
    let (attempt_str, max_str) = numeric.split_once('/')?;
    let attempt = attempt_str.parse::<usize>().ok()?;
    let max_attempts = max_str.parse::<usize>().ok()?;
    if attempt == 0 || max_attempts == 0 || attempt > max_attempts {
        return None;
    }
    Some(StreamRetryNotice {
        attempt,
        max_attempts,
        reason,
    })
}

/// Categorical reason a streaming response failed to assemble.
///
/// The agent loop uses this to decide whether to retry, escalate, or
/// surface the error to the user verbatim. Each variant carries the
/// formatted user-facing message, so callers can still `to_string()` for
/// display while inspecting the variant for routing decisions.
///
/// The distinction matters because the recourse is different per case:
///   * [`StreamError::Truncated`] — upstream connection dropped before
///     `content_block_stop`/`message_stop`. The HTTP request itself was
///     idempotent (same body would produce the same answer modulo
///     sampling) and the cause is transient (network blip, proxy 504,
///     model server hiccup). **Safe to retry automatically.**
///   * [`StreamError::UpstreamCorruption`] — the assembled JSON contains
///     U+FFFD replacement characters, indicating a hop in the chain
///     lossily decoded mid-multibyte-sequence chunks. Retry *might*
///     help if the chunk boundaries land elsewhere, but the underlying
///     bug is in the upstream streamer.
///   * [`StreamError::Malformed`] — the model emitted JSON we cannot
///     parse and the stream closed cleanly. Retrying is unlikely to
///     help; surface to the user.
///   * [`StreamError::EmptyStream`] — connection opened but produced no
///     content blocks. Usually a credential / model-not-found issue.
///   * [`StreamError::Protocol`] — malformed SSE framing or unparseable
///     control event. Indicates a serious upstream bug.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamError {
    /// SSE stream ended without `content_block_stop` for an in-flight
    /// block (or without `message_stop` overall). Retry-safe.
    Truncated(String),
    /// Stream completed but its payload contains U+FFFD — upstream
    /// re-encoded chunked UTF-8 lossily.
    UpstreamCorruption(String),
    /// Stream completed but the assembled tool input JSON is unparseable
    /// for reasons unrelated to truncation or UTF-8 corruption.
    Malformed(String),
    /// Stream closed without producing any content blocks at all.
    EmptyStream(String),
    /// SSE framing or control event was unparseable; serious upstream
    /// issue, not retriable.
    Protocol(String),
}

impl StreamError {
    /// User-facing message. Equivalent to `to_string()`.
    pub fn message(&self) -> &str {
        match self {
            StreamError::Truncated(m)
            | StreamError::UpstreamCorruption(m)
            | StreamError::Malformed(m)
            | StreamError::EmptyStream(m)
            | StreamError::Protocol(m) => m,
        }
    }

    /// `true` if the agent loop should silently retry the same request.
    ///
    /// Currently only `Truncated` qualifies. `UpstreamCorruption` is
    /// _potentially_ retriable but we leave that to a future iteration
    /// to avoid wasting tokens spinning on a persistent server bug.
    pub fn is_transient_retry(&self) -> bool {
        matches!(self, StreamError::Truncated(_))
    }
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

impl std::error::Error for StreamError {}

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
) -> Result<AgentResponse, StreamError> {
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
                let parsed: StreamingErrorEnvelope =
                    serde_json::from_str(&event.data).map_err(|e| {
                        StreamError::Protocol(format!("malformed streaming error event: {e}"))
                    })?;
                return Err(StreamError::Protocol(format!(
                    "streaming error ({}): {}",
                    parsed.error.error_type, parsed.error.message
                )));
            }
            "message_start" => {
                let payload: MessageStartEvent =
                    serde_json::from_str(&event.data).map_err(|e| {
                        StreamError::Protocol(format!("invalid message_start event: {e}"))
                    })?;
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
                let payload: ContentBlockStartEvent =
                    serde_json::from_str(&event.data).map_err(|e| {
                        StreamError::Protocol(format!("invalid content_block_start event: {e}"))
                    })?;
                ensure_index(&mut blocks, payload.index);
                match payload.content_block {
                    StreamContentBlock::Text { text } => {
                        blocks[payload.index] = BlockAccumulator::Text {
                            text,
                            stop_seen: false,
                        };
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
                            stop_seen: false,
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
                            stop_seen: false,
                        };
                    }
                }
            }
            "content_block_delta" => {
                let payload: ContentBlockDeltaEvent =
                    serde_json::from_str(&event.data).map_err(|e| {
                        StreamError::Protocol(format!("invalid content_block_delta event: {e}"))
                    })?;
                ensure_index(&mut blocks, payload.index);
                match payload.delta {
                    StreamDelta::TextDelta { text } => {
                        if let BlockAccumulator::Text { text: buf, .. } = &mut blocks[payload.index]
                        {
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
                let payload: IndexEvent = serde_json::from_str(&event.data).map_err(|e| {
                    StreamError::Protocol(format!("invalid content_block_stop event: {e}"))
                })?;
                if let Some(slot) = blocks.get_mut(payload.index) {
                    match slot {
                        BlockAccumulator::Text { stop_seen, .. }
                        | BlockAccumulator::ToolUse { stop_seen, .. } => *stop_seen = true,
                        BlockAccumulator::Empty => {}
                    }
                }
                if let Some(handler) = handler {
                    handler(&StreamingEvent::BlockStop {
                        index: payload.index,
                    });
                }
            }
            "message_delta" => {
                let payload: MessageDeltaEvent =
                    serde_json::from_str(&event.data).map_err(|e| {
                        StreamError::Protocol(format!("invalid message_delta event: {e}"))
                    })?;
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
        return Err(StreamError::EmptyStream(String::from(
            "streaming response ended before any content was received",
        )));
    }

    let mut text_parts: Vec<String> = Vec::new();
    let mut tool_calls: Vec<ToolCall> = Vec::new();
    // Snapshot a useful char-count BEFORE consuming `blocks`, so the
    // fallback truncation message at the bottom can report how much
    // content we did manage to assemble without needing to re-walk
    // (the per-tool detailed error below also re-uses these strings).
    let total_chars: usize = blocks
        .iter()
        .map(|block| match block {
            BlockAccumulator::Text { text, .. } => text.chars().count(),
            BlockAccumulator::ToolUse { partial_input, .. } => partial_input.chars().count(),
            BlockAccumulator::Empty => 0,
        })
        .sum();
    let block_count = blocks.len();

    for block in blocks {
        match block {
            BlockAccumulator::Empty => {}
            BlockAccumulator::Text { text, .. } => {
                if !text.is_empty() {
                    text_parts.push(text);
                }
            }
            BlockAccumulator::ToolUse {
                id,
                name,
                partial_input,
                stop_seen,
            } => {
                let input: serde_json::Value = if partial_input.trim().is_empty() {
                    serde_json::json!({})
                } else {
                    serde_json::from_str(&partial_input).map_err(|e| {
                        classify_tool_input_error(&name, &id, &partial_input, &e, stop_seen)
                    })?
                };
                tool_calls.push(ToolCall::new(id, name, input));
            }
        }
    }

    // Last-resort truncation guard: every block parsed cleanly (so the
    // detailed per-tool errors above had no quarrel) but `message_stop`
    // never arrived. This is the "text answer cut mid-paragraph" case
    // — no per-block error fires because text blocks just hold whatever
    // arrived, but the agent loop must still treat it as transient and
    // retry. (Detailed per-tool truncation errors are emitted above and
    // take precedence so the user gets the most actionable message.)
    if !saw_message_stop {
        return Err(StreamError::Truncated(format!(
            "streaming response cut before message_stop ({total_chars} chars assembled across {block_count} block(s)). \
             Retry the prompt, or run with --no-stream if the issue persists."
        )));
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
///
/// `stop_seen` tracks whether the corresponding `content_block_stop` event
/// arrived. When the SSE stream is interrupted mid-flight (network blip,
/// upstream model server hiccup, proxy disconnect) we exit the loop with
/// `stop_seen == false`, and the partial buffer becomes a broken JSON
/// object that we surface as a *truncation* error rather than an opaque
/// "EOF while parsing a string" — much easier for the user to diagnose
/// and retry.
enum BlockAccumulator {
    Empty,
    Text {
        text: String,
        stop_seen: bool,
    },
    ToolUse {
        id: String,
        name: String,
        partial_input: String,
        stop_seen: bool,
    },
}

fn ensure_index(blocks: &mut Vec<BlockAccumulator>, index: usize) {
    while blocks.len() <= index {
        blocks.push(BlockAccumulator::Empty);
    }
}

/// Categorise a tool-input JSON failure into the right [`StreamError`]
/// variant and format the user-facing message.
///
/// We distinguish three failure modes so the user gets a useful next step
/// instead of an opaque parse error:
///
/// 1. **Mid-stream truncation** (`!stop_seen`) — the upstream server cut
///    the SSE connection before sending `content_block_stop` for this
///    block. The partial JSON is necessarily incomplete (no closing
///    quote / brace). Returned as [`StreamError::Truncated`] so the API
///    layer can transparently retry.
/// 2. **UTF-8 corruption** (`partial_input` contains U+FFFD) — some hop
///    in the chain (proxy, load balancer, model server's own streamer)
///    lossily decoded a chunked UTF-8 sequence and replaced the bad
///    bytes with `\u{FFFD}`. The JSON string then either fails to
///    decode or, if salvageable, holds garbage CJK / emoji. Returned as
///    [`StreamError::UpstreamCorruption`] — _not_ auto-retried, because
///    the cause is upstream and a retry may just yield the same bad
///    chunks at different offsets.
/// 3. **Otherwise** — genuinely malformed JSON. Returned as
///    [`StreamError::Malformed`]; the user has to retry by hand.
///
/// `partial_input` is truncated to a reasonable preview length so a 12 KB
/// HTML write call doesn't dump its entire body into the error message.
fn classify_tool_input_error(
    name: &str,
    id: &str,
    partial_input: &str,
    parse_err: &serde_json::Error,
    stop_seen: bool,
) -> StreamError {
    const PREVIEW_BYTES: usize = 240;

    let has_replacement = partial_input.contains('\u{FFFD}');
    let total_chars = partial_input.chars().count();
    let preview = truncate_preview(partial_input, PREVIEW_BYTES);

    if !stop_seen {
        let utf8_note = if has_replacement {
            " (also contains U+FFFD replacement chars — upstream UTF-8 corruption)"
        } else {
            ""
        };
        StreamError::Truncated(format!(
            "tool_use {name}#{id} stream truncated before content_block_stop ({total_chars} chars assembled){utf8_note}. \
             Retry the prompt, or run with --no-stream if the issue persists. Partial input preview: {preview}"
        ))
    } else if has_replacement {
        StreamError::UpstreamCorruption(format!(
            "tool_use {name}#{id} input JSON contains U+FFFD replacement characters — upstream UTF-8 corruption in the streaming payload. \
             Retry the prompt; if it recurs the upstream API is mis-chunking multibyte sequences. Underlying parse error: {parse_err}. Preview: {preview}"
        ))
    } else {
        StreamError::Malformed(format!(
            "tool_use {name}#{id} input JSON did not assemble cleanly: {parse_err}; partial={preview}"
        ))
    }
}

/// Truncate a UTF-8 string to roughly `max_bytes` bytes for inclusion in
/// an error message. Respects char boundaries so the preview stays valid
/// UTF-8 even when the cut falls mid-multibyte-sequence, and appends an
/// `…` marker plus the total length so the reader knows it's not the
/// whole payload.
fn truncate_preview(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_owned();
    }
    let mut cut = max_bytes;
    while cut > 0 && !text.is_char_boundary(cut) {
        cut -= 1;
    }
    format!(
        "{}… ({} more chars)",
        &text[..cut],
        text.chars().count() - text[..cut].chars().count()
    )
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
fn read_sse_event<R: BufRead>(reader: &mut R) -> Result<Option<SseEvent>, StreamError> {
    let mut event_type: Option<String> = None;
    let mut data_parts: Vec<String> = Vec::new();
    let mut saw_any_line = false;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line).map_err(|e| {
            // I/O errors on the SSE stream mid-flight (TCP RST,
            // connection reset, proxy timeout) are exactly the
            // transient class we want to retry. Tag as Truncated so
            // the API layer treats it as such.
            StreamError::Truncated(format!("failed to read streaming response: {e}"))
        })?;
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
    ///
    /// Returns the typed [`StreamError`] on failure so individual tests
    /// can assert on the variant; for convenience the test body usually
    /// matches on the variant *and* checks the formatted message.
    fn drive(
        payload: &str,
    ) -> (
        Result<AgentResponse, StreamError>,
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
        assert!(matches!(err, StreamError::Protocol(_)));
        let msg = err.message();
        assert!(
            msg.contains("overloaded_error") && msg.contains("Try later"),
            "unexpected error surface: {msg}"
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
        assert!(matches!(err, StreamError::EmptyStream(_)));
        assert!(
            err.message()
                .contains("ended before any content was received")
        );
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
        // Stream closed cleanly (saw content_block_stop + message_stop)
        // but the JSON inside is truly malformed — should NOT be
        // classified as retriable truncation.
        assert!(matches!(err, StreamError::Malformed(_)));
        assert!(!err.is_transient_retry());
        let msg = err.message();
        assert!(msg.contains("tool_use") && msg.contains("Read"));
    }

    #[test]
    fn tool_stream_truncated_before_block_stop_yields_distinct_error() {
        // Real-world failure mode: the upstream server (or a proxy) cuts
        // the SSE connection mid-tool-call. We saw the `tool_use` start,
        // received some `input_json_delta`s, but never got the matching
        // `content_block_stop` or any `message_stop`.
        //
        // Old behaviour: opaque "EOF while parsing a string at column N"
        //   leaking the partial JSON into the user's terminal.
        // New behaviour: structured "stream truncated before
        //   content_block_stop" with a chars-assembled count and a
        //   bounded preview so the user knows it's an upstream issue
        //   they can retry.
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_x\",\"name\":\"Write\",\"input\":{}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"file_path\\\":\\\"a.html\\\",\\\"content\\\":\\\"<html>...\"}}\n\n",
            // ← intentionally no content_block_stop, no message_stop
        );
        let (result, _) = drive(payload);
        let err = result.expect_err("truncated tool stream must error");
        // Variant must be Truncated so the API layer's auto-retry
        // loop fires; the formatted message must still carry the
        // human-actionable framing.
        assert!(
            matches!(err, StreamError::Truncated(_)),
            "expected Truncated variant, got {err:?}"
        );
        assert!(err.is_transient_retry());
        let msg = err.message();
        assert!(
            msg.contains("stream truncated before content_block_stop")
                || msg.contains("cut before message_stop"),
            "expected truncation framing, got: {msg}"
        );
        assert!(msg.contains("Write") && msg.contains("toolu_x"));
    }

    #[test]
    fn tool_input_with_replacement_chars_is_flagged_as_utf8_corruption() {
        // Upstream UTF-8 mis-chunking: the server replaced bad bytes
        // with U+FFFD inside the streamed JSON. The block DID close
        // cleanly (stop_seen=true) so this is distinct from the
        // truncation case — we surface a different diagnosis because
        // the user can't fix it by waiting longer.
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_y\",\"name\":\"Write\",\"input\":{}}}\n\n",
            "event: content_block_delta\n",
            // Three U+FFFD chars inside the `content` value → still
            // valid JSON syntactically (FFFD is a legal Unicode scalar)
            // but the trailing `,` makes it unparsable, so we still
            // route through `format_tool_input_error`.
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"content\\\":\\\"\u{FFFD}\u{FFFD}\u{FFFD}\\\",\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (result, _) = drive(payload);
        let err = result.expect_err("FFFD-tainted JSON must error");
        // Clean stream close + FFFD corruption → UpstreamCorruption,
        // NOT Truncated. The retry loop must not fire because the
        // problem is upstream's lossy decoder, not a transient cut.
        assert!(
            matches!(err, StreamError::UpstreamCorruption(_)),
            "expected UpstreamCorruption variant, got {err:?}"
        );
        assert!(!err.is_transient_retry());
        let msg = err.message();
        assert!(
            msg.contains("U+FFFD replacement characters"),
            "expected FFFD framing, got: {msg}"
        );
        assert!(
            msg.contains("upstream UTF-8 corruption"),
            "error must blame upstream, not the user: {msg}"
        );
    }

    #[test]
    fn truncation_with_replacement_chars_mentions_both_in_diagnosis() {
        // The pathological case the user actually hit: BOTH upstream
        // UTF-8 corruption AND mid-stream truncation. We prioritise
        // the truncation framing (more actionable — retry might fix
        // it) but tack on a note that UTF-8 mojibake is also present.
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_z\",\"name\":\"Write\",\"input\":{}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"content\\\":\\\"无尽地\u{FFFD}\u{FFFD}\u{FFFD}\"}}\n\n",
            // No stop events.
        );
        let (result, _) = drive(payload);
        let err = result.expect_err("truncated+corrupt must error");
        // Truncation wins — it's the more actionable framing, and
        // the upstream UTF-8 mojibake gets a parenthetical note.
        assert!(
            matches!(err, StreamError::Truncated(_)),
            "expected Truncated variant, got {err:?}"
        );
        let msg = err.message();
        assert!(
            msg.contains("stream truncated") || msg.contains("cut before message_stop"),
            "got: {msg}"
        );
        assert!(
            msg.contains("U+FFFD"),
            "should also mention UTF-8 corruption: {msg}"
        );
    }

    #[test]
    fn large_partial_input_is_previewed_not_dumped_into_error_message() {
        // A 12 KB HTML write (the screenshot's failure size) must NOT
        // splat the whole body into the error string — that's how the
        // old failure mode wallpapered the terminal. Verify the error
        // stays small AND mentions that it's truncated.
        let mut huge_partial = String::from("{\\\"content\\\":\\\"");
        for _ in 0..3_000 {
            huge_partial.push_str("ABCD");
        }
        let payload = format!(
            concat!(
                "event: message_start\n",
                "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"model\":\"m\"}}}}\n\n",
                "event: content_block_start\n",
                "data: {{\"type\":\"content_block_start\",\"index\":0,\"content_block\":{{\"type\":\"tool_use\",\"id\":\"toolu_h\",\"name\":\"Write\",\"input\":{{}}}}}}\n\n",
                "event: content_block_delta\n",
                "data: {{\"type\":\"content_block_delta\",\"index\":0,\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":\"{}\"}}}}\n\n",
            ),
            huge_partial
        );
        let (result, _) = drive(&payload);
        let err = result.expect_err("must error");
        let msg = err.message();
        assert!(
            msg.len() < 2_000,
            "error message must stay bounded, got {} bytes",
            msg.len()
        );
        assert!(
            msg.contains("more chars"),
            "preview must show elision: {msg}"
        );
    }

    #[test]
    fn text_only_stream_cut_before_message_stop_classifies_as_truncated() {
        // Variant of the truncation case where the model never even
        // got to a tool call — pure text output cut mid-paragraph.
        // The block closed cleanly but `message_stop` never arrived,
        // so the agent loop must treat it as transient and retry.
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello, this is a partial answer that gets cut off mid-\"}}\n\n",
            // No content_block_stop, no message_stop — connection died.
        );
        let (result, _) = drive(payload);
        let err = result.expect_err("text stream cut must error");
        assert!(
            matches!(err, StreamError::Truncated(_)),
            "text truncation must be Truncated variant, got {err:?}"
        );
        assert!(err.is_transient_retry());
        let msg = err.message();
        assert!(
            msg.contains("cut before message_stop"),
            "expected top-level truncation framing, got: {msg}"
        );
        // The chars-assembled count comes from the partial text we did
        // receive — useful for the user to see how much was lost.
        assert!(
            msg.contains("chars assembled"),
            "must report assembled char count: {msg}"
        );
    }

    #[test]
    fn parse_stream_retry_signal_decodes_canonical_format() {
        let raw = "deeptide:stream-retry:2/3 (streaming response cut before message_stop)";
        let notice = parse_stream_retry_signal(raw).expect("canonical signal must parse");
        assert_eq!(notice.attempt, 2);
        assert_eq!(notice.max_attempts, 3);
        assert_eq!(notice.reason, "streaming response cut before message_stop");
    }

    #[test]
    fn parse_stream_retry_signal_handles_missing_reason_payload() {
        // Forward-compat: a future signal may omit the parenthesised
        // reason. We must accept it (empty reason) rather than reject
        // the whole signal.
        let notice = parse_stream_retry_signal("deeptide:stream-retry:1/3")
            .expect("payload-less signal must parse");
        assert_eq!(notice.attempt, 1);
        assert_eq!(notice.max_attempts, 3);
        assert_eq!(notice.reason, "");
    }

    #[test]
    fn parse_stream_retry_signal_rejects_real_stop_reasons() {
        // Real Anthropic stop_reasons must not be mistaken for our
        // sentinel — they share the field but not the prefix.
        assert!(parse_stream_retry_signal("end_turn").is_none());
        assert!(parse_stream_retry_signal("tool_use").is_none());
        assert!(parse_stream_retry_signal("max_tokens").is_none());
        assert!(parse_stream_retry_signal("stop_sequence").is_none());
        // Empty / whitespace cases.
        assert!(parse_stream_retry_signal("").is_none());
        assert!(parse_stream_retry_signal("   ").is_none());
    }

    #[test]
    fn parse_stream_retry_signal_rejects_malformed_payloads() {
        // Defense-in-depth: a malformed payload must never panic and
        // must never produce a misleading attempt count. Each of these
        // returns None and the REPL silently no-ops.
        assert!(parse_stream_retry_signal("deeptide:stream-retry:").is_none());
        assert!(parse_stream_retry_signal("deeptide:stream-retry:abc/3").is_none());
        assert!(parse_stream_retry_signal("deeptide:stream-retry:3/abc").is_none());
        assert!(parse_stream_retry_signal("deeptide:stream-retry:3-of-3").is_none());
        // attempt > max is nonsensical (the retry loop never emits this).
        assert!(parse_stream_retry_signal("deeptide:stream-retry:5/3").is_none());
        // attempt == 0 likewise (we're always at least on attempt 1).
        assert!(parse_stream_retry_signal("deeptide:stream-retry:0/3").is_none());
        // max == 0 is degenerate.
        assert!(parse_stream_retry_signal("deeptide:stream-retry:1/0").is_none());
    }

    #[test]
    fn parse_stream_retry_signal_round_trips_with_api_emit_format() {
        // The format the api.rs retry loop emits MUST decode cleanly.
        // This guards against the two call sites drifting apart — if
        // someone changes the format string in api.rs without updating
        // the parser, this test goes red.
        let attempt = 1_usize;
        let max = STREAM_TRUNCATION_MAX_ATTEMPTS_FOR_TESTS;
        let reason = "stream cut, retrying with backoff";
        let wire = format!(
            "{prefix}{attempt}/{max} ({reason})",
            prefix = STREAM_RETRY_SIGNAL_PREFIX
        );
        let decoded = parse_stream_retry_signal(&wire).expect("round-trip must parse");
        assert_eq!(decoded.attempt, attempt);
        assert_eq!(decoded.max_attempts, max);
        assert_eq!(decoded.reason, reason);
    }

    /// Mirror of `api::STREAM_TRUNCATION_MAX_ATTEMPTS` so this module's
    /// test can assert round-trip without taking a circular dep on api.
    const STREAM_TRUNCATION_MAX_ATTEMPTS_FOR_TESTS: usize = 3;

    #[test]
    fn is_transient_retry_only_fires_for_truncated_variant() {
        assert!(StreamError::Truncated("x".into()).is_transient_retry());
        assert!(!StreamError::UpstreamCorruption("x".into()).is_transient_retry());
        assert!(!StreamError::Malformed("x".into()).is_transient_retry());
        assert!(!StreamError::EmptyStream("x".into()).is_transient_retry());
        assert!(!StreamError::Protocol("x".into()).is_transient_retry());
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
