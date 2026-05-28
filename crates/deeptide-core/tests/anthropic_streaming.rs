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
        request.contains("Accept: text/event-stream") || request.contains("accept: text/event-stream"),
        "missing Accept header for SSE: {request:.500}"
    );
    let body_idx = request.find("\r\n\r\n").expect("body separator");
    let body = &request[body_idx + 4..];
    assert!(
        body.contains("\"stream\":true"),
        "request body should set stream:true: {body:.500}"
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
        })
        .expect("streaming respond without handler");
    assert_eq!(response.content, "Hello, world!");
}
