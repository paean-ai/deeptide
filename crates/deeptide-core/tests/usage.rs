//! Integration tests for `/usage`.
//!
//! Two layers:
//!
//! 1. The renderer (`render_tool_usage`) is private to `repl.rs`, but
//!    we exercise it indirectly by driving `/usage` through
//!    `ReplSession::submit` and asserting on the rendered text.
//! 2. The state plumbing — `AgentLoop` must populate `tool_usage()` on
//!    every dispatch — is harder to drive from a static backend because
//!    the in-tree dispatch needs a real `ToolCall` flow. Instead, we
//!    seed the tracker directly via `agent_loop_mut` style accessors
//!    where available, falling back to asserting on the dashboard's
//!    structural format (counts header, columns) when starting empty.

#![allow(clippy::unwrap_used)]

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplEvent, ReplSession, ToolCall,
};

struct StaticBackend {
    /// On first call: emit a tool call. On second: emit a plain reply
    /// so the loop terminates.
    served_tool: bool,
    tool_name: String,
}

impl AgentBackend for StaticBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        if !self.served_tool {
            self.served_tool = true;
            Ok(AgentResponse {
                content: String::new(),
                usage: Some(AgentUsage::new(1, 1, 0, 0, 0)),
                tool_calls: vec![ToolCall {
                    id: String::from("call_1"),
                    name: self.tool_name.clone(),
                    input: serde_json::json!({}),
                }],
            })
        } else {
            Ok(AgentResponse {
                content: String::from("done"),
                usage: Some(AgentUsage::new(1, 1, 0, 0, 0)),
                tool_calls: Vec::new(),
            })
        }
    }
}

fn collect_text(events: Vec<ReplEvent>) -> String {
    events
        .into_iter()
        .filter_map(|event| match event {
            ReplEvent::Output(s) => Some(s),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn make_repl(tool_name: &str) -> ReplSession {
    ReplSession::new(Box::new(StaticBackend {
        served_tool: false,
        tool_name: tool_name.to_owned(),
    }))
}

// ── Empty state ──────────────────────────────────────────────────────

#[test]
fn usage_on_empty_session_reports_zero_invocations() {
    let mut repl = make_repl("noop");
    let text = collect_text(repl.submit("/usage"));
    assert!(text.contains("0 invocation"), "got: {text}");
}

#[test]
fn usage_json_on_empty_session_is_valid_json_with_zero_total() {
    let mut repl = make_repl("noop");
    let text = collect_text(repl.submit("/usage --json"));
    let parsed: serde_json::Value =
        serde_json::from_str(&text).expect("--json must produce valid JSON");
    assert_eq!(parsed["total_invocations"], 0);
    assert_eq!(parsed["unique_tools"], 0);
    assert!(parsed["tools"].as_array().unwrap().is_empty());
}

// ── State after a tool call ──────────────────────────────────────────

#[test]
fn usage_after_tool_call_records_one_invocation() {
    // The agent emits a tool call for an unknown name; the registry
    // returns an error result. We don't care about success — we care
    // that the tracker recorded *something*.
    let mut repl = make_repl("BogusToolThatDoesNotExist");
    let _ = repl.submit("trigger turn");

    let text = collect_text(repl.submit("/usage"));
    // The dashboard truncates very long names to fit the column, so we
    // only assert on a stable prefix that the truncation leaves
    // visible. The /usage --json test below asserts the full name.
    assert!(text.contains("BogusToolThatDoesNot"), "got: {text}");
    assert!(
        text.contains("1 call(s)") || text.contains("1 call("),
        "expected one invocation in header: {text}",
    );
}

#[test]
fn usage_dashboard_includes_column_headers() {
    let mut repl = make_repl("BogusToolThatDoesNotExist");
    let _ = repl.submit("kick");
    let text = collect_text(repl.submit("/usage"));
    for needle in ["calls", "ok", "err", "total", "avg", "peak", "bytes"] {
        assert!(
            text.contains(needle),
            "expected column {needle:?} in dashboard: {text}",
        );
    }
}

#[test]
fn usage_json_after_tool_call_reports_entry() {
    let mut repl = make_repl("BogusToolThatDoesNotExist");
    let _ = repl.submit("kick");

    let text = collect_text(repl.submit("/usage --json"));
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["total_invocations"], 1);
    assert_eq!(parsed["unique_tools"], 1);
    let tools = parsed["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0]["tool"], "BogusToolThatDoesNotExist");
    assert_eq!(tools[0]["invocations"], 1);
    // The bogus tool returns an error, so error count is 1.
    assert_eq!(tools[0]["errors"], 1);
    assert_eq!(tools[0]["success"], 0);
}

// ── reset ─────────────────────────────────────────────────────────────

#[test]
fn usage_reset_drops_recorded_samples() {
    let mut repl = make_repl("BogusToolThatDoesNotExist");
    let _ = repl.submit("kick");
    let before = collect_text(repl.submit("/usage --json"));
    let before: serde_json::Value = serde_json::from_str(&before).unwrap();
    assert_eq!(before["total_invocations"], 1);

    let reset_text = collect_text(repl.submit("/usage reset"));
    assert!(reset_text.contains("Cleared 1"), "got: {reset_text}");

    let after = collect_text(repl.submit("/usage --json"));
    let after: serde_json::Value = serde_json::from_str(&after).unwrap();
    assert_eq!(after["total_invocations"], 0);
}

#[test]
fn usage_reset_is_idempotent_on_empty_session() {
    let mut repl = make_repl("noop");
    let text = collect_text(repl.submit("/usage reset"));
    assert!(text.contains("already empty"), "got: {text}");
}

// ── Help + unknown subcommand ───────────────────────────────────────

#[test]
fn usage_help_lists_subcommands() {
    let mut repl = make_repl("noop");
    let text = collect_text(repl.submit("/usage --help"));
    for needle in ["Usage:", "--json", "reset"] {
        assert!(text.contains(needle), "help missing {needle:?}: {text}");
    }
}

#[test]
fn usage_unknown_subcommand_lists_alternatives() {
    let mut repl = make_repl("noop");
    let text = collect_text(repl.submit("/usage frobnicate"));
    assert!(text.contains("Unknown subcommand"), "got: {text}");
    assert!(text.contains("--json"), "got: {text}");
}
