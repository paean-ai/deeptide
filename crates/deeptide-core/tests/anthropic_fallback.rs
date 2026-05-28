//! End-to-end coverage for `AnthropicBackend`'s fallback-model retry: a real
//! `reqwest::blocking::Client` is pointed at a local TCP mock server that
//! replies with a transient overload before succeeding. Validates that:
//!
//! 1. A `529 overloaded` response triggers a single retry with the configured
//!    fallback model, and the assembled response is returned.
//! 2. The retry request carries the fallback model on the wire.
//! 3. A non-retryable error (HTTP 400) is surfaced without any retry.
//! 4. With no fallback configured, an overload error is surfaced as-is.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use deeptide_core::{
    AgentBackend, AgentRequest, AnthropicBackend, AnthropicConfig, ConversationMessage,
};

const SUCCESS_BODY: &str = r#"{"id":"msg_1","type":"message","role":"assistant","content":[{"type":"text","text":"recovered"}],"usage":{"input_tokens":5,"output_tokens":2}}"#;
const OVERLOAD_BODY: &str =
    r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
const BAD_REQUEST_BODY: &str =
    r#"{"type":"error","error":{"type":"invalid_request_error","message":"bad"}}"#;

/// Serve a fixed sequence of `(status, json_body)` responses, one per inbound
/// connection, capturing each full request (headers + body) in arrival order.
fn serve_sequence(responses: Vec<(u16, &'static str)>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock");
    let addr = listener.local_addr().expect("addr");
    let captured: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_clone = Arc::clone(&captured);

    thread::spawn(move || {
        for (status, body) in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let request = read_http_request(&mut stream);
            captured_clone
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(request);

            let reason = match status {
                529 => "Overloaded",
                400 => "Bad Request",
                _ => "OK",
            };
            let header = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            let _ = stream.write_all(body.as_bytes());
            let _ = stream.flush();
        }
    });

    (format!("http://{addr}"), captured)
}

/// Drain a full HTTP request (headers and Content-Length body) so the client's
/// large body write completes before we respond with `Connection: close`.
fn read_http_request(stream: &mut std::net::TcpStream) -> String {
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
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let mut remaining = content_length.saturating_sub(buf.len().saturating_sub(header_end));
    while remaining > 0 {
        let n = stream.read(&mut chunk).expect("read body");
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&chunk[..n]);
        remaining = remaining.saturating_sub(n);
    }
    String::from_utf8_lossy(&buf).into_owned()
}

fn request() -> AgentRequest {
    AgentRequest {
        messages: vec![ConversationMessage::user("hi")],
        model: "primary-model".to_owned(),
        step: 0,
        max_turns: 1,
        system: None,
    }
}

#[test]
fn overload_retries_once_with_fallback_model() {
    let (base_url, captured) = serve_sequence(vec![(529, OVERLOAD_BODY), (200, SUCCESS_BODY)]);

    let mut config = AnthropicConfig::new(base_url, "test-key", "primary-model");
    config.enable_prompt_caching = false;
    config.fallback_model = Some("fallback-model".to_owned());

    let mut backend = AnthropicBackend::new(config).expect("build backend");
    let response = backend.respond(request()).expect("fallback should succeed");
    assert_eq!(response.content, "recovered");

    let requests = captured
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(
        requests.len(),
        2,
        "expected a primary attempt and one retry"
    );
    assert!(
        requests[0].contains("\"model\":\"primary-model\""),
        "first attempt should use the primary model"
    );
    assert!(
        requests[1].contains("\"model\":\"fallback-model\""),
        "retry should use the fallback model"
    );
}

#[test]
fn non_retryable_error_is_not_retried() {
    let (base_url, captured) = serve_sequence(vec![(400, BAD_REQUEST_BODY)]);

    let mut config = AnthropicConfig::new(base_url, "test-key", "primary-model");
    config.enable_prompt_caching = false;
    config.fallback_model = Some("fallback-model".to_owned());

    let mut backend = AnthropicBackend::new(config).expect("build backend");
    let error = backend.respond(request()).expect_err("400 should fail");
    assert!(
        error.contains("400"),
        "error should report HTTP 400: {error}"
    );

    let requests = captured
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(requests.len(), 1, "a 400 must not trigger a fallback retry");
}

#[test]
fn overload_without_fallback_surfaces_error() {
    let (base_url, captured) = serve_sequence(vec![(529, OVERLOAD_BODY)]);

    let mut config = AnthropicConfig::new(base_url, "test-key", "primary-model");
    config.enable_prompt_caching = false;
    // No fallback configured.

    let mut backend = AnthropicBackend::new(config).expect("build backend");
    let error = backend.respond(request()).expect_err("529 should fail");
    assert!(
        error.contains("529"),
        "error should report HTTP 529: {error}"
    );

    let requests = captured
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert_eq!(requests.len(), 1, "without a fallback there is no retry");
}
