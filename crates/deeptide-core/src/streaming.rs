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
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// Token-level delta of model *reasoning* / chain-of-thought, distinct from
    /// the user-facing answer. Emitted by reasoning models on the OpenAI
    /// protocol (DeepSeek-reasoner's `reasoning_content`, and the `reasoning`
    /// alias other OpenAI-compatible providers use). UIs render this dimmed and
    /// separately from the answer; it is deliberately NOT folded into the
    /// response `content` (DeepSeek requires reasoning is not echoed back on the
    /// next request, and consumers price/replay it differently).
    ThinkingDelta { delta: String },
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
    /// Stream completed cleanly but the model ran out of its
    /// `max_tokens` budget mid-content — Anthropic still emits
    /// `content_block_stop` + `message_stop` in this case, just with
    /// `stop_reason: max_tokens` on the terminal `message_delta`. The
    /// partial tool input JSON is necessarily incomplete (closing
    /// quote / brace missing) because the model didn't get to finish.
    /// Not retriable with the same budget — the next attempt would
    /// hit the same wall — so we surface a distinct error pointing
    /// the user at `--max-tokens` rather than burning another call.
    TokenBudgetExceeded(String),
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
    /// The user cancelled (Ctrl-C) while the stream was in flight. NOT a retry
    /// candidate — the agent loop maps the resulting error to an
    /// [`crate::AgentTerminalEvent::Interrupted`] terminal.
    Interrupted,
}

impl StreamError {
    /// User-facing message. Equivalent to `to_string()`.
    pub fn message(&self) -> &str {
        match self {
            StreamError::Interrupted => "stream cancelled by user",
            StreamError::Truncated(m)
            | StreamError::TokenBudgetExceeded(m)
            | StreamError::UpstreamCorruption(m)
            | StreamError::Malformed(m)
            | StreamError::EmptyStream(m)
            | StreamError::Protocol(m) => m,
        }
    }

    /// `true` if the agent loop should silently retry the same request.
    ///
    /// Currently only `Truncated` qualifies. `TokenBudgetExceeded` is
    /// deliberately *not* retriable: the same prompt with the same
    /// budget will hit the same wall, so retrying just doubles the
    /// bill. `UpstreamCorruption` is _potentially_ retriable but we
    /// leave that to a future iteration to avoid wasting tokens
    /// spinning on a persistent server bug.
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
    interrupt: Option<&AtomicBool>,
) -> Result<AgentResponse, StreamError> {
    let mut reader = BufReader::new(reader);
    let mut blocks: Vec<BlockAccumulator> = Vec::new();
    let mut usage = UsageAccumulator::default();
    let mut model: Option<String> = None;
    let mut saw_message_stop = false;
    // Last `stop_reason` seen on a `message_delta` event. When the
    // model hits its `max_tokens` budget mid-tool-call, Anthropic still
    // emits a clean `content_block_stop` + `message_stop`, just with
    // `stop_reason: max_tokens` on the terminal delta. We retain that
    // so the per-tool-input classifier can distinguish "budget
    // exhausted" (not retriable, fix by raising --max-tokens) from
    // "model emitted garbage JSON" (also not retriable, but different
    // recourse).
    let mut final_stop_reason: Option<String> = None;

    loop {
        // Cancelled mid-stream: drop the reader (closing the connection) and
        // bail. During active token generation, SSE events arrive
        // continuously, so this check fires within one event (tens of ms) of
        // the Ctrl-C. The only non-instant window is the model's pre-first-
        // token pause, where the blocking read sits with no bytes — that
        // cancellation lands when the first token arrives. (This reqwest build
        // has no per-read timeout on the blocking client, so the
        // between-events check is the cancellation mechanism.)
        if interrupt.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(StreamError::Interrupted);
        }
        let Some(event) = read_sse_event(&mut reader)? else {
            break;
        };
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
                    // Reasoning delta — emit for live display only. Not appended
                    // to any answer accumulator (the thinking block is a reserved
                    // Empty slot), so it never contaminates the response text.
                    StreamDelta::ThinkingDelta { thinking } => {
                        if let Some(handler) = handler {
                            handler(&StreamingEvent::ThinkingDelta { delta: thinking });
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
                // Latch the stop_reason. We deliberately overwrite on
                // every message_delta — Anthropic only emits one final
                // delta with a real stop_reason, but if a server emits
                // multiple (some compatible APIs do for streaming
                // progress) we want the last word.
                if let Some(reason) = payload.delta.stop_reason.as_ref() {
                    final_stop_reason = Some(reason.clone());
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
                        classify_tool_input_error(
                            &name,
                            &id,
                            &partial_input,
                            &e,
                            stop_seen,
                            final_stop_reason.as_deref(),
                        )
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
/// We distinguish four failure modes so the user gets a useful next step
/// instead of an opaque parse error:
///
/// 1. **Mid-stream truncation** (`!stop_seen`) — the upstream server cut
///    the SSE connection before sending `content_block_stop` for this
///    block. The partial JSON is necessarily incomplete (no closing
///    quote / brace). Returned as [`StreamError::Truncated`] so the API
///    layer can transparently retry.
/// 2. **`max_tokens` budget exhausted** (`stop_seen` + final
///    `stop_reason == "max_tokens"`) — the stream completed cleanly
///    but the model ran out of budget mid-content. The partial JSON
///    is just as broken as the truncation case, but retrying with
///    the same budget would hit the same wall, so we return
///    [`StreamError::TokenBudgetExceeded`] which is NOT
///    auto-retried. The message names `--max-tokens` so the user
///    knows the concrete knob to turn.
/// 3. **UTF-8 corruption** (`partial_input` contains U+FFFD) — some hop
///    in the chain (proxy, load balancer, model server's own streamer)
///    lossily decoded a chunked UTF-8 sequence and replaced the bad
///    bytes with `\u{FFFD}`. The JSON string then either fails to
///    decode or, if salvageable, holds garbage CJK / emoji. Returned as
///    [`StreamError::UpstreamCorruption`] — _not_ auto-retried, because
///    the cause is upstream and a retry may just yield the same bad
///    chunks at different offsets.
/// 4. **Otherwise** — genuinely malformed JSON. Returned as
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
    final_stop_reason: Option<&str>,
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
    } else if final_stop_reason == Some("max_tokens") {
        // Highest-priority "stop_seen=true" diagnosis: budget exhausted
        // takes precedence over UTF-8 corruption because the user's fix
        // is straightforward (raise --max-tokens), unambiguous, and
        // resolves the issue deterministically.
        let utf8_note = if has_replacement {
            " (payload also contains U+FFFD upstream-corruption marks)"
        } else {
            ""
        };
        StreamError::TokenBudgetExceeded(format!(
            "tool_use {name}#{id} hit the model's max_tokens budget mid-call \
             ({total_chars} chars produced before stop_reason=max_tokens){utf8_note}. \
             The partial JSON cannot be repaired by retrying with the same budget — \
             raise --max-tokens (default is 65536; DeepSeek V4 accepts up to 384K) \
             or break the request into smaller chunks using Edit/append-style edits. \
             Partial input preview: {preview}"
        ))
    } else if has_replacement {
        StreamError::UpstreamCorruption(format!(
            "tool_use {name}#{id} input JSON contains U+FFFD replacement characters — upstream UTF-8 corruption in the streaming payload. \
             Retry the prompt; if it recurs the upstream API is mis-chunking multibyte sequences. Underlying parse error: {parse_err}. Preview: {preview}"
        ))
    } else {
        // Cleanly-stopped stream with valid UTF-8 but unparseable JSON.
        // We've never observed this in the wild without a max_tokens
        // pretext, so most "Malformed" reports in practice are likely
        // a server that omits stop_reason on max_tokens. Point at
        // --max-tokens defensively but keep the variant distinct so
        // the user can recognise it.
        StreamError::Malformed(format!(
            "tool_use {name}#{id} input JSON did not assemble cleanly: {parse_err}; \
             this usually means the model truncated its own output (try raising --max-tokens \
             if the partial preview looks cut off mid-content). partial={preview}"
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

/// Read one SSE event and return just its `data` payload, or `Ok(None)` on EOF.
///
/// Convenience wrapper over [`read_sse_event`] for protocols whose framing
/// carries everything in the `data:` field with no `event:` type — OpenAI's
/// chat-completions and Gemini's `streamGenerateContent?alt=sse` both work this
/// way. Exposed at crate level so `gemini.rs` (and any future bare-`data:`
/// backend) can reuse the battle-tested SSE line reader instead of re-rolling
/// framing.
pub(crate) fn read_sse_data_event<R: BufRead>(
    reader: &mut R,
) -> Result<Option<String>, StreamError> {
    Ok(read_sse_event(reader)?.map(|event| event.data))
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
    /// Extended-thinking reasoning delta (when a thinking budget is enabled).
    /// Display-only: the thinking block lives at its own index and is never
    /// folded into the answer text. We surface it as `ThinkingDelta` for the
    /// dim live render, symmetric with the OpenAI/Gemini reasoning paths.
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
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

// ─── OpenAI chat-completions streaming ───────────────────────────────────────
//
// OpenAI's SSE dialect differs from Anthropic's in framing but maps cleanly
// onto the SAME [`StreamingEvent`] surface, so the TUI renders both protocols
// identically (live markdown, the `↓ N tokens` ticker, tool-call subjects).
//
// Wire differences handled here:
//   * **No `event:` field** — every frame is a bare `data: {json}` line, so we
//     dispatch on the JSON payload, not [`SseEvent::event_type`].
//   * **`data: [DONE]`** sentinel ends the stream (Anthropic uses
//     `event: message_stop`).
//   * **Tool calls stream as `choices[0].delta.tool_calls[]`** — each fragment
//     carries an `index`, optionally an `id`/`function.name` (first fragment),
//     and incremental `function.arguments` string chunks. We accumulate by
//     index exactly like Anthropic's `tool_use` block accumulator.
//   * Text streams as `choices[0].delta.content` string chunks.
//   * Usage (when present, often only on the final chunk with
//     `stream_options.include_usage`) lives at the top level.

/// One streamed chat-completions chunk (`object: "chat.completion.chunk"`).
#[derive(Debug, Deserialize)]
struct OpenAiChunk {
    #[serde(default)]
    choices: Vec<OpenAiChunkChoice>,
    #[serde(default)]
    usage: Option<OpenAiStreamUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChunkChoice {
    #[serde(default)]
    delta: OpenAiDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct OpenAiDelta {
    #[serde(default)]
    content: Option<String>,
    /// Reasoning-model chain-of-thought. DeepSeek-reasoner emits
    /// `reasoning_content`; some OpenAI-compatible providers use `reasoning`.
    /// Accept both so the live "thinking" stream works across vendors.
    #[serde(default, alias = "reasoning")]
    reasoning_content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OpenAiToolCallDelta>,
}

#[derive(Debug, Deserialize)]
struct OpenAiToolCallDelta {
    /// Position of this tool call within the assistant message. Stable across
    /// the fragments that build up one call.
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OpenAiFunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct OpenAiFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamUsage {
    #[serde(default)]
    prompt_tokens: usize,
    #[serde(default)]
    completion_tokens: usize,
    #[serde(default)]
    prompt_tokens_details: Option<OpenAiPromptDetails>,
}

#[derive(Debug, Deserialize)]
struct OpenAiPromptDetails {
    #[serde(default)]
    cached_tokens: usize,
}

/// Parse an OpenAI chat-completions SSE stream, invoking `handler` per delta
/// and assembling the final [`AgentResponse`]. Mirrors
/// [`parse_streaming_response`] (the Anthropic path) in shape and cancellation
/// behaviour so callers — and the TUI — see one uniform streaming contract.
///
/// `interrupt` is checked between frames; a raised flag drops the connection
/// and returns [`StreamError::Interrupted`], same as the Anthropic path.
pub fn parse_openai_stream<R: Read>(
    reader: R,
    handler: Option<&StreamingHandler>,
    elapsed: Duration,
    interrupt: Option<&AtomicBool>,
) -> Result<AgentResponse, StreamError> {
    let mut reader = BufReader::new(reader);
    let mut text = String::new();
    // Tool calls accumulate by their delta `index`. Each entry grows its
    // `arguments` string across fragments; `id`/`name` arrive on the first
    // fragment for that index.
    let mut tool_acc: Vec<OpenAiToolAccumulator> = Vec::new();
    let mut input_tokens = 0usize;
    let mut output_tokens = 0usize;
    let mut cache_read = 0usize;
    let mut emitted_text_block = false;
    let mut saw_done = false;

    loop {
        if interrupt.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            return Err(StreamError::Interrupted);
        }
        let Some(event) = read_sse_event(&mut reader)? else {
            break;
        };
        let data = event.data.trim();
        if data.is_empty() {
            continue;
        }
        // The `[DONE]` sentinel terminates the stream (no JSON to parse).
        if data == "[DONE]" {
            saw_done = true;
            if let Some(handler) = handler {
                handler(&StreamingEvent::MessageStop);
            }
            break;
        }

        // A server-side error can arrive mid-stream as a bare `data: {error:…}`
        // frame. Check for it FIRST — an error object also deserializes into
        // `OpenAiChunk` (every field is optional), so we'd otherwise silently
        // treat it as an empty chunk and lose the error.
        if let Some(message) = openai_stream_error(data) {
            return Err(StreamError::Protocol(format!("streaming error: {message}")));
        }
        let chunk: OpenAiChunk = match serde_json::from_str(data) {
            Ok(chunk) => chunk,
            Err(_) => {
                return Err(StreamError::Protocol(format!(
                    "invalid chat.completion.chunk: {data}"
                )));
            }
        };

        if let Some(usage) = chunk.usage {
            output_tokens = usage.completion_tokens;
            cache_read = usage
                .prompt_tokens_details
                .map(|d| d.cached_tokens)
                .unwrap_or(cache_read);
            // `prompt_tokens` INCLUDES the cached portion (OpenAI/DeepSeek
            // convention); subtract it so the cost tracker doesn't charge the
            // cached tokens twice. Mirrors the buffered path in openai.rs.
            input_tokens = usage.prompt_tokens.saturating_sub(cache_read);
            if let Some(handler) = handler {
                handler(&StreamingEvent::MessageDelta {
                    stop_reason: None,
                    output_tokens: Some(output_tokens),
                });
            }
        }

        for choice in chunk.choices {
            // Reasoning fragment (DeepSeek-reasoner et al.). Surface it live so
            // the user sees the model "thinking", but do NOT accumulate it into
            // `text` — reasoning is display-only and must not be echoed back as
            // assistant content on the next turn.
            if let Some(thought) = choice.delta.reasoning_content.filter(|s| !s.is_empty())
                && let Some(handler) = handler
            {
                handler(&StreamingEvent::ThinkingDelta { delta: thought });
            }

            // Text fragment.
            if let Some(piece) = choice.delta.content.filter(|s| !s.is_empty()) {
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

            // Tool-call fragments, keyed by index.
            for delta in choice.delta.tool_calls {
                let slot = ensure_tool_index(&mut tool_acc, delta.index);
                let first_fragment = !slot.started;
                if let Some(id) = delta.id.filter(|s| !s.is_empty()) {
                    slot.id = id;
                }
                if let Some(function) = delta.function {
                    if let Some(name) = function.name.filter(|s| !s.is_empty()) {
                        slot.name = name;
                    }
                    if let Some(args) = function.arguments {
                        slot.arguments.push_str(&args);
                        if let Some(handler) = handler {
                            handler(&StreamingEvent::ToolUseInputDelta {
                                // +1 so tool-call indices never collide with the
                                // text block at index 0 in the handler's view.
                                index: delta.index + 1,
                                partial_json: args,
                            });
                        }
                    }
                }
                // Announce the tool call the first time we see its slot, once we
                // have a name (the TUI shows "Preparing <name>").
                if first_fragment && !slot.name.is_empty() {
                    slot.started = true;
                    if let Some(handler) = handler {
                        handler(&StreamingEvent::ToolUseStart {
                            index: delta.index + 1,
                            id: slot.id.clone(),
                            name: slot.name.clone(),
                        });
                    }
                } else if !slot.name.is_empty() {
                    slot.started = true;
                }
            }

            // A finish_reason closes the message; OpenAI still sends `[DONE]`
            // after, but emit the terminal usage delta here so the UI's
            // token counter lands its final value promptly.
            if let (Some(reason), Some(handler)) = (choice.finish_reason, handler) {
                handler(&StreamingEvent::MessageDelta {
                    stop_reason: Some(reason),
                    output_tokens: Some(output_tokens),
                });
            }
        }
    }

    if !saw_done && text.is_empty() && tool_acc.is_empty() {
        return Err(StreamError::EmptyStream(String::from(
            "streaming response ended before any content was received",
        )));
    }

    // Emit BlockStop for each tool call so the TUI's args ticker tears down.
    if let Some(handler) = handler {
        for acc in &tool_acc {
            if acc.started {
                handler(&StreamingEvent::BlockStop {
                    index: acc.index + 1,
                });
            }
        }
    }

    let tool_calls: Vec<ToolCall> = tool_acc
        .into_iter()
        .filter(|acc| !acc.name.is_empty())
        .map(|acc| {
            let input = if acc.arguments.trim().is_empty() {
                serde_json::json!({})
            } else {
                serde_json::from_str(&acc.arguments).unwrap_or_else(|_| serde_json::json!({}))
            };
            ToolCall::new(acc.id, acc.name, input)
        })
        .collect();

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

/// Per-index accumulator for a streamed OpenAI tool call.
#[derive(Debug, Default)]
struct OpenAiToolAccumulator {
    index: usize,
    id: String,
    name: String,
    arguments: String,
    /// Whether we've already emitted a `ToolUseStart` for this slot.
    started: bool,
}

fn ensure_tool_index(
    acc: &mut Vec<OpenAiToolAccumulator>,
    index: usize,
) -> &mut OpenAiToolAccumulator {
    if let Some(pos) = acc.iter().position(|a| a.index == index) {
        return &mut acc[pos];
    }
    acc.push(OpenAiToolAccumulator {
        index,
        ..Default::default()
    });
    acc.last_mut().expect("just pushed")
}

/// Best-effort extraction of an error message from a mid-stream error frame.
fn openai_stream_error(data: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(data).ok()?;
    value
        .pointer("/error/message")
        .or_else(|| value.pointer("/message"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
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
        let result =
            parse_streaming_response(payload.as_bytes(), Some(&handler), Duration::ZERO, None);
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
                StreamingEvent::ThinkingDelta { .. } => "ThinkingDelta",
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
    fn extended_thinking_deltas_emit_thinking_without_polluting_the_answer() {
        // With a thinking budget enabled, Anthropic streams a `thinking` block
        // (own index) with `thinking_delta` reasoning, THEN the answer text in a
        // separate block. The reasoning must surface as ThinkingDelta events and
        // must NOT appear in the assembled answer.
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\",\"usage\":{\"input_tokens\":5,\"output_tokens\":0}}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"thinking_delta\",\"thinking\":\"Let me reason. \"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"abc\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"The answer.\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":3}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (result, observed) = drive(payload);
        let response = result.expect("stream parses");
        // Answer excludes the reasoning; the signature_delta is ignored.
        assert_eq!(response.content, "The answer.");

        let observed = observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let thoughts: Vec<&str> = observed
            .iter()
            .filter_map(|e| match e {
                StreamingEvent::ThinkingDelta { delta } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(thoughts, vec!["Let me reason. "]);
        assert!(observed.iter().all(|e| !matches!(
            e,
            StreamingEvent::TextDelta { delta, .. } if delta.contains("reason")
        )));
    }

    #[test]
    fn interrupt_flag_aborts_stream_before_reading() {
        // A pre-set interrupt flag makes the parser bail at the top of the
        // loop with `Interrupted`, even though the payload is a complete,
        // well-formed response. Mirrors a Ctrl-C that lands just as the
        // stream opens.
        let flag = AtomicBool::new(true);
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"x\",\"usage\":{\"input_tokens\":1,\"output_tokens\":0}}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let result =
            parse_streaming_response(payload.as_bytes(), None, Duration::ZERO, Some(&flag));
        assert!(
            matches!(result, Err(StreamError::Interrupted)),
            "a raised interrupt flag must abort the stream: {result:?}"
        );
        // `Interrupted` must not be a retry candidate — otherwise the API
        // layer's truncation-retry loop would re-issue the request the user
        // just cancelled.
        assert!(!StreamError::Interrupted.is_transient_retry());
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
        let result = parse_streaming_response(b"".as_ref(), None, Duration::ZERO, None);
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
    fn tool_input_truncated_by_max_tokens_classifies_as_token_budget_exceeded() {
        // The roguelike-HTML failure mode: model emits half a Write
        // tool call, then Anthropic terminates the message with
        // stop_reason=max_tokens. The block closes cleanly
        // (content_block_stop + message_stop both arrive) but the JSON
        // is necessarily incomplete because the model was cut off by
        // the budget.
        //
        // Old behaviour: classified as `Malformed`, which sent the
        //   user down the wrong recovery path (retry by hand, no
        //   guidance on what to change). Auto-retry didn't fire
        //   because the variant isn't retriable, but a retry would
        //   have hit the same wall anyway.
        // New behaviour: `TokenBudgetExceeded` variant with an
        //   explicit "raise --max-tokens" recommendation. Not
        //   auto-retried — same budget = same failure.
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_budget\",\"name\":\"Write\",\"input\":{}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"file_path\\\":\\\"a.html\\\",\\\"content\\\":\\\"<!DOCTYPE html>...\"}}\n\n",
            // Both stop events arrive cleanly — this is NOT a network
            // truncation. The message_delta carries stop_reason=max_tokens.
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"max_tokens\"},\"usage\":{\"output_tokens\":4096}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (result, _) = drive(payload);
        let err = result.expect_err("max_tokens mid-tool-call must error");
        // Variant must be TokenBudgetExceeded so the agent loop's
        // retry path bypasses this case (same budget = same failure)
        // and the user sees concrete recovery instructions.
        assert!(
            matches!(err, StreamError::TokenBudgetExceeded(_)),
            "expected TokenBudgetExceeded variant, got {err:?}"
        );
        assert!(
            !err.is_transient_retry(),
            "budget-exceeded must not auto-retry — same prompt + same budget = same failure"
        );
        let msg = err.message();
        assert!(
            msg.contains("max_tokens budget"),
            "message must name the cause: {msg}"
        );
        assert!(
            msg.contains("--max-tokens"),
            "message must name the CLI knob to turn: {msg}"
        );
        assert!(
            msg.contains("Write") && msg.contains("toolu_budget"),
            "message must identify the offending tool call: {msg}"
        );
    }

    #[test]
    fn max_tokens_diagnosis_only_fires_for_max_tokens_stop_reason() {
        // Defense-in-depth: a model that emits stop_reason=tool_use (the
        // normal "I'm done emitting this tool call" reason) followed by
        // genuinely malformed JSON must NOT be classified as a budget
        // overrun — that would point the user at the wrong knob.
        let payload = concat!(
            "event: message_start\n",
            "data: {\"type\":\"message_start\",\"message\":{\"id\":\"m\",\"model\":\"m\"}}\n\n",
            "event: content_block_start\n",
            "data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"t1\",\"name\":\"Read\",\"input\":{}}}\n\n",
            "event: content_block_delta\n",
            "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"file_path\\\":[\"}}\n\n",
            "event: content_block_stop\n",
            "data: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
            "event: message_delta\n",
            "data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":42}}\n\n",
            "event: message_stop\n",
            "data: {\"type\":\"message_stop\"}\n\n",
        );
        let (result, _) = drive(payload);
        let err = result.expect_err("malformed JSON must error");
        assert!(
            matches!(err, StreamError::Malformed(_)),
            "expected Malformed (not TokenBudgetExceeded), got {err:?}"
        );
        // The defensive --max-tokens hint is still included in the
        // Malformed message (since these often co-occur in practice)
        // but the variant is distinct.
        assert!(err.message().contains("did not assemble cleanly"));
    }

    #[test]
    fn is_transient_retry_excludes_token_budget_variant() {
        // Pin the retry-eligibility matrix: budget overrun must never
        // be retried because doing so would just burn another call
        // hitting the same limit. Test alongside the truncation case
        // so future variants can't accidentally flip this bit.
        assert!(StreamError::Truncated("x".into()).is_transient_retry());
        assert!(!StreamError::TokenBudgetExceeded("x".into()).is_transient_retry());
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

    // ─── OpenAI streaming ────────────────────────────────────────────────────

    fn drive_openai(
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
        let result = parse_openai_stream(payload.as_bytes(), Some(&handler), Duration::ZERO, None);
        (result, observed)
    }

    #[test]
    fn openai_stream_assembles_text_from_content_deltas() {
        let payload = "\
            data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n\
            data: {\"choices\":[{\"delta\":{\"content\":\", world\"}}]}\n\n\
            data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
            data: [DONE]\n\n";
        let (result, observed) = drive_openai(payload);
        let response = result.expect("stream should parse");
        assert_eq!(response.content, "Hello, world");
        assert!(response.tool_calls.is_empty());

        let events = observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // First content delta opens a text block, deltas flow, stream stops.
        assert!(matches!(
            events[0],
            StreamingEvent::TextBlockStart { index: 0 }
        ));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StreamingEvent::TextDelta { delta, .. } if delta == "Hello"))
        );
        assert!(matches!(events.last(), Some(StreamingEvent::MessageStop)));
    }

    #[test]
    fn openai_stream_surfaces_reasoning_content_as_thinking_without_polluting_answer() {
        // DeepSeek-reasoner streams `reasoning_content` deltas BEFORE the answer
        // `content`. The reasoning must surface as live ThinkingDelta events but
        // must NOT leak into the final answer text (it's display-only and must
        // not be echoed back to the model next turn).
        let payload = "\
            data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"Let me think. \"}}]}\n\n\
            data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"2+2=4.\"}}]}\n\n\
            data: {\"choices\":[{\"delta\":{\"content\":\"The answer is 4.\"}}]}\n\n\
            data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
            data: [DONE]\n\n";
        let (result, observed) = drive_openai(payload);
        let response = result.expect("stream should parse");
        // Answer is clean — reasoning is excluded.
        assert_eq!(response.content, "The answer is 4.");

        let events = observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let thoughts: Vec<&str> = events
            .iter()
            .filter_map(|e| match e {
                StreamingEvent::ThinkingDelta { delta } => Some(delta.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(thoughts, vec!["Let me think. ", "2+2=4."]);
        // No reasoning text bled into any TextDelta.
        assert!(events.iter().all(|e| !matches!(
            e,
            StreamingEvent::TextDelta { delta, .. } if delta.contains("think") || delta.contains("2+2")
        )));
    }

    #[test]
    fn openai_stream_accepts_reasoning_alias() {
        // Some OpenAI-compatible providers name the field `reasoning`.
        let payload = "\
            data: {\"choices\":[{\"delta\":{\"reasoning\":\"hmm\"}}]}\n\n\
            data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
            data: [DONE]\n\n";
        let (result, observed) = drive_openai(payload);
        assert_eq!(result.expect("parse").content, "hi");
        let events = observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StreamingEvent::ThinkingDelta { delta } if delta == "hmm"))
        );
    }

    #[test]
    fn openai_stream_accumulates_tool_call_across_fragments() {
        // id+name arrive on the first fragment; arguments stream in chunks.
        let payload = "\
            data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"call_1\",\"function\":{\"name\":\"Bash\",\"arguments\":\"{\\\"command\\\":\"}}]}}]}\n\n\
            data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"ls\\\"}\"}}]}}]}\n\n\
            data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n\
            data: [DONE]\n\n";
        let (result, observed) = drive_openai(payload);
        let response = result.expect("stream should parse");
        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].id, "call_1");
        assert_eq!(response.tool_calls[0].name, "Bash");
        assert_eq!(
            response.tool_calls[0].input,
            serde_json::json!({"command": "ls"})
        );

        let events = observed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // Tool index is offset by +1 so it never collides with text block 0.
        assert!(events.iter().any(|e| matches!(
            e,
            StreamingEvent::ToolUseStart { index: 1, name, .. } if name == "Bash"
        )));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StreamingEvent::ToolUseInputDelta { index: 1, .. }))
        );
        assert!(
            events
                .iter()
                .any(|e| matches!(e, StreamingEvent::BlockStop { index: 1 }))
        );
    }

    #[test]
    fn openai_stream_reads_usage_from_final_chunk() {
        let payload = "\
            data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n\
            data: {\"choices\":[],\"usage\":{\"prompt_tokens\":50,\"completion_tokens\":8,\"prompt_tokens_details\":{\"cached_tokens\":20}}}\n\n\
            data: [DONE]\n\n";
        let (result, _) = drive_openai(payload);
        let usage = result.expect("parse").usage.expect("usage present");
        // prompt_tokens (50) includes the 20 cached → uncached input is 30 so
        // the cached tokens aren't priced twice.
        assert_eq!(usage.input_tokens, 30);
        assert_eq!(usage.output_tokens, 8);
        assert_eq!(usage.cache_read, 20);
        assert_eq!(usage.input_tokens + usage.cache_read, 50);
    }

    #[test]
    fn openai_stream_surfaces_mid_stream_error_frame() {
        let payload =
            "data: {\"error\":{\"message\":\"model overloaded\",\"type\":\"server_error\"}}\n\n";
        let (result, _) = drive_openai(payload);
        let err = result.expect_err("error frame must surface");
        assert!(
            matches!(&err, StreamError::Protocol(m) if m.contains("model overloaded")),
            "got: {err:?}"
        );
    }

    #[test]
    fn openai_stream_honors_a_preset_interrupt_flag() {
        let flag = AtomicBool::new(true);
        let payload = "data: {\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n";
        let result = parse_openai_stream(payload.as_bytes(), None, Duration::ZERO, Some(&flag));
        assert!(matches!(result, Err(StreamError::Interrupted)));
    }

    #[test]
    fn openai_stream_empty_body_is_an_empty_stream_error() {
        let (result, _) = drive_openai("");
        assert!(matches!(result, Err(StreamError::EmptyStream(_))));
    }
}
