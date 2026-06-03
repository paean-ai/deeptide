//! End-to-end parity test for the headless `--print --output-format
//! stream-json` envelope. The Swift Deeptide app and zero-cli consume this
//! exact sequence: a leading `_spec` handshake line, then live
//! `assistant_delta` events, then a terminal `result`/`error` envelope. We
//! assert the stable framing here so a refactor can't silently break
//! downstream consumers.
//!
//! Runs against the built-in local-echo backend (no API key, no network) by
//! clearing every credential env var and passing `--isolated` so a developer's
//! `settings.json` can't inject a real provider.

use std::process::Command;

/// Env vars that could configure a real backend or redirect config. Cleared so
/// the spawned binary deterministically falls back to the local-echo backend.
const SCRUBBED_ENV: &[&str] = &[
    "DEEPTIDE_API_KEY",
    "DEEPTIDE_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ZERO_API_KEY",
    "ZERO_CLI_API_KEY",
    "ZERO_CLI_AUTH_TOKEN",
    "DEEPTIDE_MODEL",
    "ANTHROPIC_MODEL",
    "ZERO_CLI_MODEL",
    "DEEPTIDE_BASE_URL",
    "ZERO_CLI_BASE_URL",
    "DEEPTIDE_PROFILE",
    "TIDE_PROFILE",
    "DEEPTIDE_ISOLATED",
];

fn run_stream_json(prompt: &str) -> String {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_deeptide"));
    cmd.args([
        "--isolated",
        "--print",
        "--output-format",
        "stream-json",
        "-p",
        prompt,
    ]);
    for key in SCRUBBED_ENV {
        cmd.env_remove(key);
    }
    let output = cmd.output().expect("spawn deeptide binary");
    assert!(
        output.status.success(),
        "headless run exited non-zero: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout is utf-8")
}

fn json_lines(stdout: &str) -> Vec<serde_json::Value> {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).expect("each stream-json line is valid JSON"))
        .collect()
}

#[test]
fn stream_json_envelope_leads_with_spec_and_ends_with_result() {
    let stdout = run_stream_json("hello there");
    let events = json_lines(&stdout);
    assert!(
        events.len() >= 2,
        "expected at least a _spec and a result line, got: {stdout}"
    );

    // First line: the protocol handshake consumers read before anything else.
    let spec = &events[0];
    assert_eq!(spec["type"], "_spec");
    assert_eq!(spec["version"], "deepclide.embedded.v1");
    assert_eq!(spec["input_format"], "stream-json");
    assert_eq!(spec["output_format"], "stream-json");
    assert!(
        spec["supported_output_types"]
            .as_array()
            .expect("supported_output_types is an array")
            .iter()
            .any(|t| t == "result"),
        "spec must advertise the result output type"
    );

    // Last line: the terminal result envelope with the full usage breakdown.
    let result = events.last().expect("at least one event");
    assert_eq!(result["type"], "result");
    assert_eq!(result["status"], "completed");
    for field in [
        "response",
        "model",
        "cost_usd",
        "input_tokens",
        "output_tokens",
        "cache_create_tokens",
        "cache_read_tokens",
    ] {
        assert!(
            result.get(field).is_some(),
            "result envelope missing `{field}`: {result}"
        );
    }
    // Every interior line must be a recognised, parseable event type — no
    // stray non-JSON or unframed text leaking into the stream.
    for event in &events {
        assert!(
            event
                .get("type")
                .and_then(serde_json::Value::as_str)
                .is_some(),
            "every stream-json line carries a string `type`: {event}"
        );
    }
}

/// Run a headless command expected to FAIL (here: resuming a session that
/// doesn't exist), returning (exit_code, stdout). Does NOT assert success — the
/// whole point is the failure path.
fn run_failing(args: &[&str]) -> (Option<i32>, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_deeptide"));
    cmd.arg("--isolated").args(args);
    for key in SCRUBBED_ENV {
        cmd.env_remove(key);
    }
    let output = cmd.output().expect("spawn deeptide binary");
    let stdout = String::from_utf8(output.stdout).expect("stdout is utf-8");
    (output.status.code(), stdout)
}

#[test]
fn json_output_exits_nonzero_on_failure_but_still_prints_the_error_envelope() {
    // Resuming a missing session is a deterministic, network-free failure.
    let (code, stdout) = run_failing(&[
        "--print",
        "--output-format",
        "json",
        "--resume",
        "definitely-not-a-real-session-id",
        "-p",
        "hi",
    ]);
    // CRITICAL contract: a failed run must NOT exit 0, so shell scripts can
    // detect it via `$?` instead of having to parse the JSON.
    assert_eq!(
        code,
        Some(2),
        "json output on a failed run must exit non-zero, got {code:?}; stdout: {stdout}"
    );
    // ...AND the machine-readable error envelope must still be on stdout.
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("error envelope is still valid JSON");
    assert_eq!(value["type"], "error");
    assert_eq!(value["status"], "fatal");
    assert!(
        value
            .get("error")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|e| !e.is_empty()),
        "error envelope must carry a non-empty `error` field: {value}"
    );
}

#[test]
fn stream_json_output_exits_nonzero_on_failure_and_ends_with_error_envelope() {
    let (code, stdout) = run_failing(&[
        "--print",
        "--output-format",
        "stream-json",
        "--resume",
        "definitely-not-a-real-session-id",
        "-p",
        "hi",
    ]);
    assert_eq!(
        code,
        Some(2),
        "stream-json output on a failed run must exit non-zero, got {code:?}; stdout: {stdout}"
    );
    let events = json_lines(&stdout);
    // The spec handshake still leads; the terminal envelope is the error.
    assert_eq!(events[0]["type"], "_spec");
    let last = events.last().expect("at least the spec + result lines");
    assert_eq!(last["type"], "error");
    assert_eq!(last["status"], "fatal");
}

#[test]
fn json_output_format_is_a_single_result_object() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_deeptide"));
    cmd.args([
        "--isolated",
        "--print",
        "--output-format",
        "json",
        "-p",
        "ping",
    ]);
    for key in SCRUBBED_ENV {
        cmd.env_remove(key);
    }
    let output = cmd.output().expect("spawn deeptide binary");
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf-8");
    // The whole of stdout parses as one JSON object (pretty-printed).
    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("json output is a single JSON object");
    assert_eq!(value["type"], "result");
    assert_eq!(value["status"], "completed");
}
