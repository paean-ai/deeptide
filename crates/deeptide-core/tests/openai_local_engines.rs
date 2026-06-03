//! End-to-end coverage for the `OpenAiBackend` against the real-world quirks of
//! local inference engines (ollama, llama.cpp `server`, mlx-lm, LM Studio).
//!
//! These engines all expose an OpenAI-compatible `/v1/chat/completions`
//! endpoint, but each deviates from the canonical OpenAI SSE shape in small
//! ways that have historically broken naive clients. A real
//! `reqwest::blocking::Client` is pointed at a local TCP mock that replays each
//! engine's actual streaming dialect, so we prove the parser is robust to:
//!
//!   * **llama.cpp**: a leading `delta:{role:"assistant"}` chunk with no
//!     content, and the final `usage` arriving in its own chunk.
//!   * **mlx-lm / older servers**: NO `[DONE]` sentinel — the stream just ends
//!     when the connection closes (EOF).
//!   * **ollama**: model ids containing a colon (`qwen2.5-coder:7b`), empty
//!     keep-alive content deltas, and usage that may be absent entirely.
//!   * **tool calls** streamed in fragments over any of the above.
//!
//! If any of these regress, a user pointing `--provider ollama` / `mlx` /
//! `vllm` at a local server would see a broken or empty response, so we pin the
//! behaviour here rather than relying on a developer having a live engine.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use deeptide_core::{
    AgentBackend, AgentRequest, ConversationMessage, OpenAiBackend, OpenAiConfig, StreamingEvent,
    StreamingHandler,
};

/// Spin up a one-shot HTTP server that records the request and responds with
/// the supplied SSE payload. `close_without_done` omits the trailing framing so
/// we can model engines that end on connection close. Returns the base URL +
/// the captured request bytes.
fn serve_sse_once(payload: &'static str) -> (String, Arc<Mutex<Vec<u8>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind sse mock");
    let addr = listener.local_addr().expect("addr");
    let captured: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = Arc::clone(&captured);

    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
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

/// Drive a streaming request and return (assembled response, observed events).
fn run_stream(
    base_url: String,
    model: &str,
) -> (
    deeptide_core::AgentResponse,
    Arc<Mutex<Vec<StreamingEvent>>>,
) {
    let mut config = OpenAiConfig::new(base_url, "", model); // keyless (local)
    config.enable_streaming = true;

    let observed: Arc<Mutex<Vec<StreamingEvent>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&observed);
    let handler: StreamingHandler = Arc::new(move |event| {
        sink.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(event.clone());
    });

    let mut backend = OpenAiBackend::new(config)
        .expect("build backend")
        .with_streaming_handler(handler);

    let response = backend
        .respond(AgentRequest {
            messages: vec![ConversationMessage::user("hi")],
            model: model.to_owned(),
            step: 0,
            max_turns: 1,
            system: None,
            allowed_tools: None,
            thinking: None,
        })
        .expect("streaming respond");

    (response, observed)
}

fn text_deltas(events: &Arc<Mutex<Vec<StreamingEvent>>>) -> Vec<String> {
    events
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .iter()
        .filter_map(|e| match e {
            StreamingEvent::TextDelta { delta, .. } => Some(delta.clone()),
            _ => None,
        })
        .collect()
}

// ─── llama.cpp `server` ──────────────────────────────────────────────────────
//
// Quirks: a leading `delta:{role:"assistant"}` chunk (no content), content
// chunks, then a SEPARATE final chunk carrying only `usage`. We must (a) not
// choke on the extra `role` field, (b) not emit an empty text delta for the
// role-only chunk, (c) read usage from the trailing chunk.
const LLAMACPP_STREAM: &str = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n\
    data: {\"choices\":[{\"delta\":{\"content\":\"The \"}}]}\n\n\
    data: {\"choices\":[{\"delta\":{\"content\":\"answer.\"}}]}\n\n\
    data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
    data: {\"choices\":[],\"usage\":{\"prompt_tokens\":12,\"completion_tokens\":3}}\n\n\
    data: [DONE]\n\n";

#[test]
fn llamacpp_role_only_first_chunk_and_trailing_usage() {
    let (base, _) = serve_sse_once(LLAMACPP_STREAM);
    let (response, events) = run_stream(base, "local-model");

    assert_eq!(response.content, "The answer.");
    // The role-only chunk must NOT produce a text delta.
    assert_eq!(text_deltas(&events), vec!["The ", "answer."]);
    let usage = response.usage.expect("usage from trailing chunk");
    assert_eq!(usage.input_tokens, 12);
    assert_eq!(usage.output_tokens, 3);
}

// ─── mlx-lm / older servers: NO `[DONE]` sentinel ────────────────────────────
//
// The stream just ends when the socket closes. The parser must still assemble
// the response from whatever it received rather than erroring on the missing
// terminator.
const MLX_NO_DONE_STREAM: &str = "data: {\"choices\":[{\"delta\":{\"content\":\"Hi\"}}]}\n\n\
    data: {\"choices\":[{\"delta\":{\"content\":\" there\"}}]}\n\n\
    data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n";

#[test]
fn mlx_stream_without_done_sentinel_still_assembles() {
    let (base, _) = serve_sse_once(MLX_NO_DONE_STREAM);
    let (response, events) = run_stream(base, "mlx-community/Qwen2.5-Coder-7B-4bit");

    assert_eq!(response.content, "Hi there");
    assert_eq!(text_deltas(&events), vec!["Hi", " there"]);
}

// ─── ollama: colon model id, empty keep-alive deltas, no usage ───────────────
//
// ollama streams empty `delta:{}` keep-alive chunks while the model warms, uses
// model ids with a colon, and historically omitted `usage` on streamed
// responses. The response must still assemble; usage is simply absent.
const OLLAMA_STREAM: &str = "data: {\"choices\":[{\"delta\":{}}]}\n\n\
    data: {\"choices\":[{\"delta\":{\"content\":\"pong\"}}]}\n\n\
    data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n\
    data: [DONE]\n\n";

#[test]
fn ollama_empty_keepalive_deltas_and_absent_usage() {
    let (base, captured) = serve_sse_once(OLLAMA_STREAM);
    let (response, events) = run_stream(base, "qwen2.5-coder:7b");

    assert_eq!(response.content, "pong");
    // Empty keep-alive deltas must not appear as text deltas.
    assert_eq!(text_deltas(&events), vec!["pong"]);
    // No usage block in the stream → None (not a zero-filled phantom).
    assert!(response.usage.is_none());

    // The colon-bearing model id must reach the wire verbatim.
    let raw = captured
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let request = String::from_utf8_lossy(&raw);
    assert!(
        request.contains("qwen2.5-coder:7b"),
        "colon model id must survive to the request body: {request:.400}"
    );
    // Keyless local engine: NO Authorization header.
    assert!(
        !request.to_ascii_lowercase().contains("authorization:"),
        "keyless local engine must not send an Authorization header"
    );
}

// ─── tool call streamed from a local engine ──────────────────────────────────
//
// A local model that supports tool calling streams the call in fragments just
// like the cloud API. Proves the local path reaches a working tool round-trip.
const LOCAL_TOOL_STREAM: &str = "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"}}]}\n\n\
    data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"c1\",\"function\":{\"name\":\"Bash\",\"arguments\":\"{\\\"command\\\":\"}}]}}]}\n\n\
    data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"\\\"ls\\\"}\"}}]}}]}\n\n\
    data: {\"choices\":[{\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n\
    data: [DONE]\n\n";

#[test]
fn local_engine_streams_a_tool_call_in_fragments() {
    let (base, _) = serve_sse_once(LOCAL_TOOL_STREAM);
    let (response, events) = run_stream(base, "local-model");

    assert!(response.content.is_empty());
    assert_eq!(response.tool_calls.len(), 1);
    assert_eq!(response.tool_calls[0].name, "Bash");
    assert_eq!(
        response.tool_calls[0].input,
        serde_json::json!({"command": "ls"})
    );

    let evs = events
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(evs.iter().any(|e| matches!(
        e,
        StreamingEvent::ToolUseStart { name, .. } if name == "Bash"
    )));
}

// ─── URL normalization for the three common local base-url shapes ────────────

#[test]
fn buffered_request_reaches_local_engine_with_v1_path() {
    // A buffered (non-streaming) request against a local engine: prove the
    // `/v1/chat/completions` path is hit and a plain JSON response parses.
    const JSON_RESP: &str = "data-ignored"; // not used; we serve JSON below
    let _ = JSON_RESP;

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("addr");
    let captured: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let cap = Arc::clone(&captured);
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept");
        let mut buf = Vec::new();
        let mut chunk = [0_u8; 4096];
        let header_end = loop {
            let n = stream.read(&mut chunk).expect("read");
            if n == 0 {
                break buf.len();
            }
            buf.extend_from_slice(&chunk[..n]);
            if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                break p + 4;
            }
        };
        let headers = String::from_utf8_lossy(&buf[..header_end]).to_string();
        // Drain the declared body so the client's write completes cleanly.
        let cl = headers
            .lines()
            .find_map(|l| {
                let (n, v) = l.split_once(':')?;
                n.eq_ignore_ascii_case("content-length")
                    .then(|| v.trim().parse::<usize>().ok())
                    .flatten()
            })
            .unwrap_or(0);
        let mut rem = cl.saturating_sub(buf.len().saturating_sub(header_end));
        while rem > 0 {
            let n = stream.read(&mut chunk).expect("read body");
            if n == 0 {
                break;
            }
            rem = rem.saturating_sub(n);
        }
        *cap.lock().unwrap_or_else(|p| p.into_inner()) = headers;

        let body = "{\"choices\":[{\"message\":{\"content\":\"ok\"}}],\"usage\":{\"prompt_tokens\":1,\"completion_tokens\":1}}";
        let resp = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = stream.write_all(resp.as_bytes());
        let _ = stream.flush();
    });

    // base_url already includes /v1 (the preset shape for local engines).
    let mut config = OpenAiConfig::new(format!("http://{addr}/v1"), "", "local-model");
    config.enable_streaming = false;
    let mut backend = OpenAiBackend::new(config).expect("backend");
    let response = backend
        .respond(AgentRequest {
            messages: vec![ConversationMessage::user("ping")],
            model: "local-model".to_owned(),
            step: 0,
            max_turns: 1,
            system: None,
            allowed_tools: None,
            thinking: None,
        })
        .expect("buffered respond");
    assert_eq!(response.content, "ok");

    let req = captured.lock().unwrap_or_else(|p| p.into_inner());
    assert!(
        req.starts_with("POST /v1/chat/completions"),
        "request line must hit /v1/chat/completions: {}",
        req.lines().next().unwrap_or("")
    );
}
