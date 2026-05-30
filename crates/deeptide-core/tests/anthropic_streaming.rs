//! End-to-end coverage for the `AnthropicBackend` streaming path: a real
//! `reqwest::blocking::Client` is pointed at a local TCP mock server that
//! speaks the Anthropic SSE shape. Validates that:
//!
//! 1. `stream: true` is sent on the wire.
//! 2. `Accept: text/event-stream` is set.
//! 3. The handler observes deltas in arrival order.
//! 4. The assembled `AgentResponse` exposes the same surface as the
//!    non-streaming path so the agent loop is unchanged.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use deeptide_core::{
    AgentBackend, AgentRequest, AnthropicBackend, AnthropicConfig, ConversationMessage,
    StreamingEvent, StreamingHandler,
};

const TEXT_STREAM: &str = "event: message_start\n\
    data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_e2e\",\"model\":\"m\",\"usage\":{\"input_tokens\":17,\"output_tokens\":0,\"cache_read_input_tokens\":3,\"cache_creation_input_tokens\":0}}}\n\n\
    event: content_block_start\n\
    data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n\
    event: content_block_delta\n\
    data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello, \"}}\n\n\
    event: content_block_delta\n\
    data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"world!\"}}\n\n\
    event: content_block_stop\n\
    data: {\"type\":\"content_block_stop\",\"index\":0}\n\n\
    event: message_delta\n\
    data: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"end_turn\"},\"usage\":{\"output_tokens\":4}}\n\n\
    event: message_stop\n\
    data: {\"type\":\"message_stop\"}\n\n";

/// Spin up a one-shot HTTP server that records the first POST body it sees
/// (request bytes) and responds with the supplied SSE payload. Returns the
/// base URL and a captured-request handle.
fn serve_sse_once(payload: &'static str) -> (String, Arc<Mutex<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind sse mock");
    let addr = listener.local_addr().expect("addr");
    let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = Arc::clone(&captured);

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");

        // Drain the full request before responding. The deeptide body is
        // ~50KB (tool schemas), and responding with Connection: close while
        // the client is still writing yields a TCP RST that surfaces as
        // "request or response body error" to reqwest.
        let mut buf = Vec::new();
        let mut chunk = [0_u8; 4096];
        let header_end = loop {
            let n = stream.read(&mut chunk).expect("read headers");
            if n == 0 {
                break buf.len();
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break pos + 4;
            }
        };

        let headers = String::from_utf8_lossy(&buf[..header_end]);
        let content_length = headers
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        let body_so_far = buf.len().saturating_sub(header_end);
        let mut remaining = content_length.saturating_sub(body_so_far);
        while remaining > 0 {
            let n = stream.read(&mut chunk).expect("read body");
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
            remaining = remaining.saturating_sub(n);
        }
        captured_clone
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .extend_from_slice(&buf);

        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            payload.len()
        );
        let _ = stream.write_all(header.as_bytes());
        let _ = stream.write_all(payload.as_bytes());
        let _ = stream.flush();
    });

    (format!("http://{addr}"), captured)
}

#[test]
fn streaming_backend_delivers_text_deltas_and_assembles_response() {
    let (base_url, captured_request) = serve_sse_once(TEXT_STREAM);

    let mut config = AnthropicConfig::new(base_url, "test-key", "test-model");
    config.enable_streaming = true;
    config.enable_prompt_caching = false;

    let observed: Arc<Mutex<Vec<StreamingEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&observed);
    let handler: StreamingHandler = Arc::new(move |event| {
        sink.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(event.clone());
    });

    let mut backend = AnthropicBackend::new(config)
        .expect("build backend")
        .with_streaming_handler(handler);

    let response = backend
        .respond(AgentRequest {
            messages: vec![ConversationMessage::user("hi")],
            model: "test-model".to_owned(),
            step: 0,
            max_turns: 1,
            system: None,
            allowed_tools: None,
            thinking: None,
        })
        .expect("streaming respond");

    assert_eq!(response.content, "Hello, world!");
    assert!(response.tool_calls.is_empty());
    let usage = response.usage.expect("usage");
    assert_eq!(usage.input_tokens, 17);
    assert_eq!(usage.output_tokens, 4);
    assert_eq!(usage.cache_read, 3);

    let events = observed
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let text_deltas: Vec<&str> = events
        .iter()
        .filter_map(|event| match event {
            StreamingEvent::TextDelta { delta, .. } => Some(delta.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(text_deltas, vec!["Hello, ", "world!"]);

    // The first event observed must be MessageStart so callers can render a
    // status line before any tokens arrive.
    assert!(matches!(events[0], StreamingEvent::MessageStart { .. }));
    // The last event observed must be MessageStop so callers can close their
    // streaming sink and flush output.
    assert!(matches!(events.last(), Some(StreamingEvent::MessageStop)));

    // Wire shape: request body must declare stream:true and Accept SSE.
    let raw = captured_request
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let request = String::from_utf8_lossy(&raw);
    assert!(
        request.contains("Accept: text/event-stream")
            || request.contains("accept: text/event-stream"),
        "missing Accept header for SSE: {request:.500}"
    );
    let body_idx = request.find("\r\n\r\n").expect("body separator");
    let body = &request[body_idx + 4..];
    assert!(
        body.contains("\"stream\":true"),
        "request body should set stream:true: {body:.500}"
    );
}

/// Multi-shot variant: serves a sequence of SSE payloads, one per
/// incoming connection. Used to validate the auto-retry path where the
/// first connection truncates mid-stream and the second one succeeds.
///
/// Each connection is independent (the agent's reqwest client opens a
/// fresh socket per retry because the first one was closed by the
/// mock). The `handled` counter lets tests assert how many attempts the
/// backend actually made before giving up or succeeding.
fn serve_sse_sequence(payloads: Vec<&'static str>) -> (String, Arc<Mutex<usize>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind sse sequence mock");
    let addr = listener.local_addr().expect("addr");
    let attempts: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
    let attempts_clone = Arc::clone(&attempts);

    thread::spawn(move || {
        for (idx, payload) in payloads.into_iter().enumerate() {
            let (mut stream, _) = match listener.accept() {
                Ok(pair) => pair,
                Err(_) => break,
            };
            *attempts_clone
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = idx + 1;

            // Drain the request fully (same protocol as serve_sse_once
            // — the deeptide tool-schema body is ~50KB, and closing the
            // socket while the client is still writing yields a TCP RST
            // that surfaces as "body error" to reqwest before we ever
            // get to write the response).
            let mut buf = Vec::new();
            let mut chunk = [0_u8; 4096];
            let header_end = loop {
                let n = match stream.read(&mut chunk) {
                    Ok(0) => break buf.len(),
                    Ok(n) => n,
                    Err(_) => break buf.len(),
                };
                buf.extend_from_slice(&chunk[..n]);
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos + 4;
                }
            };
            let headers = String::from_utf8_lossy(&buf[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    if name.eq_ignore_ascii_case("content-length") {
                        value.trim().parse::<usize>().ok()
                    } else {
                        None
                    }
                })
                .unwrap_or(0);
            let body_so_far = buf.len().saturating_sub(header_end);
            let mut remaining = content_length.saturating_sub(body_so_far);
            while remaining > 0 {
                let n = match stream.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => n,
                    Err(_) => break,
                };
                remaining = remaining.saturating_sub(n);
            }

            // For the truncation simulation we deliberately do NOT
            // declare a Content-Length, so reqwest happily streams
            // whatever bytes we write and then sees the connection
            // close mid-SSE. `Transfer-Encoding: identity` keeps it
            // simple and skips chunked framing.
            let header =
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(payload.as_bytes());
            let _ = stream.flush();
            drop(stream);
        }
    });

    (format!("http://{addr}"), attempts)
}

/// A deliberately-truncated SSE: tool_use opens, two input_json_deltas
/// arrive, then the connection drops with no content_block_stop and no
/// message_stop. Mirrors the user's roguelike-HTML failure shape.
const TRUNCATED_TOOL_STREAM: &str = "event: message_start\n\
    data: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_t1\",\"model\":\"m\"}}\n\n\
    event: content_block_start\n\
    data: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_cut\",\"name\":\"Write\",\"input\":{}}}\n\n\
    event: content_block_delta\n\
    data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"file_path\\\":\\\"a.html\\\",\\\"content\\\":\\\"<!DOCTYPE html>...\"}}\n\n";

#[test]
fn streaming_backend_auto_retries_on_mid_stream_truncation() {
    use deeptide_core::AnthropicAuthMode;

    // Sequence: first attempt truncates mid-tool, second attempt
    // returns a clean text response. The backend must retry without
    // surfacing the truncation error to the agent loop, AND must
    // emit a `MessageDelta` notice carrying the `deeptide:stream-retry`
    // marker so the UI can show "retrying…".
    let (base_url, attempts) = serve_sse_sequence(vec![TRUNCATED_TOOL_STREAM, TEXT_STREAM]);

    let mut config = AnthropicConfig::new(base_url, "test-key", "test-model");
    config.enable_streaming = true;
    config.enable_prompt_caching = false;
    config.auth_mode = AnthropicAuthMode::ApiKey;

    let observed: Arc<Mutex<Vec<StreamingEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&observed);
    let handler: StreamingHandler = Arc::new(move |event| {
        sink.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(event.clone());
    });

    let mut backend = AnthropicBackend::new(config)
        .expect("build backend")
        .with_streaming_handler(handler);

    let response = backend
        .respond(AgentRequest {
            messages: vec![ConversationMessage::user("hi")],
            model: "test-model".to_owned(),
            step: 0,
            max_turns: 1,
            system: None,
            allowed_tools: None,
            thinking: None,
        })
        .expect("backend must transparently recover from a truncated first attempt");

    // The successful retry produced the canonical Hello-world payload.
    assert_eq!(response.content, "Hello, world!");
    assert!(response.tool_calls.is_empty());

    // The mock saw exactly two connections — initial attempt + one
    // retry. If we ever loosen the backoff cap we'd see this climb;
    // pinning it asserts we don't accidentally spin forever.
    let total_attempts = *attempts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(
        total_attempts, 2,
        "expected exactly one retry (2 attempts total), got {total_attempts}"
    );

    // The UI handler must have observed a retry notice between the
    // two attempts so the user sees forward progress instead of a
    // frozen spinner.
    let events = observed
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let retry_notice = events.iter().find_map(|event| match event {
        StreamingEvent::MessageDelta {
            stop_reason: Some(reason),
            ..
        } if reason.starts_with("deeptide:stream-retry:") => Some(reason.clone()),
        _ => None,
    });
    assert!(
        retry_notice.is_some(),
        "retry must surface a deeptide:stream-retry MessageDelta for the UI to render"
    );
}

#[test]
fn streaming_backend_gives_up_after_max_retries_on_persistent_truncation() {
    use deeptide_core::AnthropicAuthMode;

    // All three connections truncate. The backend must bubble the
    // error rather than spin forever, and the failure must be
    // recognisable as the truncation class — not a generic JSON
    // parse error wallpapering the terminal.
    let (base_url, attempts) = serve_sse_sequence(vec![
        TRUNCATED_TOOL_STREAM,
        TRUNCATED_TOOL_STREAM,
        TRUNCATED_TOOL_STREAM,
        // 4th payload here would never be served because the cap is 3
        // attempts; included anyway as a guard against accidental
        // retry-budget creep.
        TRUNCATED_TOOL_STREAM,
    ]);

    let mut config = AnthropicConfig::new(base_url, "test-key", "test-model");
    config.enable_streaming = true;
    config.enable_prompt_caching = false;
    config.auth_mode = AnthropicAuthMode::ApiKey;

    let mut backend = AnthropicBackend::new(config).expect("build backend");
    let err = backend
        .respond(AgentRequest {
            messages: vec![ConversationMessage::user("hi")],
            model: "test-model".to_owned(),
            step: 0,
            max_turns: 1,
            system: None,
            allowed_tools: None,
            thinking: None,
        })
        .expect_err("persistent truncation must bubble");

    assert!(
        err.contains("stream truncated") || err.contains("cut before message_stop"),
        "error must keep the truncation framing on final attempt: {err}"
    );

    let total_attempts = *attempts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(
        total_attempts, 3,
        "retry budget must be capped at 3 attempts, got {total_attempts}"
    );
}

#[test]
fn streaming_backend_works_without_handler_callback() {
    // Sanity check: a backend with no handler still parses the SSE stream
    // and returns the assembled response. This is the common case for
    // single-shot `--print` invocations where the user only cares about
    // the final answer.
    let (base_url, _) = serve_sse_once(TEXT_STREAM);
    let mut config = AnthropicConfig::new(base_url, "test-key", "test-model");
    config.enable_streaming = true;
    config.enable_prompt_caching = false;

    let mut backend = AnthropicBackend::new(config).expect("build backend");
    let response = backend
        .respond(AgentRequest {
            messages: vec![ConversationMessage::user("hi")],
            model: "test-model".to_owned(),
            step: 0,
            max_turns: 1,
            system: None,
            allowed_tools: None,
            thinking: None,
        })
        .expect("streaming respond without handler");
    assert_eq!(response.content, "Hello, world!");
}
