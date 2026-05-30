//! Integration tests for the dynamic segments injected into the
//! pinned status bar — currently `queue N` and the new `todo IP/N`.
//!
//! These exercise the public `ReplSession::status_line()` surface so
//! the CLI (and any future embedder rendering the same line) gets
//! tested against the exact contract it consumes.
//!
//! All assertions live in a SINGLE `#[test]` because the TODO
//! storage is a process-global `Mutex<Vec<TodoItem>>` (see
//! `tools::tasks::todo_storage`). Running parallel tests against it
//! would race on the visible state and produce flaky assertions; the
//! single-test layout keeps every transition deterministic.

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplSession, TodoSummary, TodoWriteTool,
    Tool, ToolContext, todo_summary,
};
use serde_json::json;

struct EchoBackend;

impl AgentBackend for EchoBackend {
    fn respond(&mut self, _req: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: String::from("ok"),
            usage: Some(AgentUsage::new(1, 1, 0, 0, 0)),
            tool_calls: Vec::new(),
        })
    }
}

/// Convenience: drive the public TodoWrite tool the same way the
/// model does in production, so we test the actual path that
/// populates the global storage.
fn write_todos(json_payload: serde_json::Value) {
    let tool = TodoWriteTool;
    let cwd = std::env::temp_dir();
    let context = ToolContext::new(&cwd);
    let result = tool.call(json_payload, &context);
    assert!(
        !result.is_error,
        "TodoWrite tool refused the fixture; cannot drive the test: {result:?}"
    );
}

fn render_status(repl: &ReplSession, width: usize) -> String {
    repl.status_line().render(width)
}

#[test]
fn status_line_pins_todo_segment_through_each_storage_transition() {
    // ── 1. Empty storage ⇒ no `todo` segment ────────────────────────
    write_todos(json!({ "todos": [] }));
    assert_eq!(todo_summary(), TodoSummary::default());

    let repl = ReplSession::new(Box::new(EchoBackend));
    let empty = render_status(&repl, 200);
    assert!(
        !empty.contains("todo "),
        "empty TODO storage must NOT produce a `todo` segment: {empty}"
    );

    // ── 2. Mixed-status storage ⇒ segment shows active/total ────────
    //
    // Active = pending + in_progress (the work that's still ahead).
    // Total = pending + in_progress + completed (the whole plan).
    write_todos(json!({
        "todos": [
            { "content": "step A", "status": "in_progress" },
            { "content": "step B", "status": "pending" },
            { "content": "step C", "status": "pending" },
            { "content": "step D", "status": "completed" },
        ]
    }));
    let summary = todo_summary();
    assert_eq!(
        summary,
        TodoSummary {
            pending: 2,
            in_progress: 1,
            completed: 1,
        },
        "TodoWrite must populate the global storage faithfully"
    );

    let wide = render_status(&repl, 200);
    assert!(
        wide.contains("todo 3/4"),
        "expected `todo 3/4` (active=3, total=4): {wide}"
    );
    // Sanity: the must-show segments are still there too.
    for needle in ["model", "mode", "cwd", "git", "ctx"] {
        assert!(
            wide.contains(needle),
            "wide render missing must-show segment `{needle}`: {wide}"
        );
    }

    // ── 3. All-completed storage: the TodoWrite tool clears it ──────
    //
    // Per `TodoWriteTool::call`, writing a list where every item is
    // `completed` is treated as "plan finished" — the tool still
    // replaces storage with the all-completed list, so the summary
    // reflects that, and the status segment surfaces `todo 0/N`
    // (no active work, but the plan is still there). This is the
    // signal the user needs to know the agent finished the
    // checklist.
    write_todos(json!({
        "todos": [
            { "content": "step A", "status": "completed" },
            { "content": "step B", "status": "completed" },
        ]
    }));
    let done = todo_summary();
    assert_eq!(done.in_progress + done.pending, 0);
    assert_eq!(done.completed, 2);
    let done_render = render_status(&repl, 200);
    assert!(
        done_render.contains("todo 0/2"),
        "all-completed plan must show `todo 0/2`: {done_render}"
    );

    // ── 4. Cleanup ──────────────────────────────────────────────────
    // Leave the global storage empty so any downstream test that
    // depends on a clean baseline isn't poisoned by this one.
    write_todos(json!({ "todos": [] }));
    assert_eq!(todo_summary(), TodoSummary::default());
    let final_render = render_status(&repl, 200);
    assert!(
        !final_render.contains("todo "),
        "post-cleanup render must not contain `todo`: {final_render}"
    );
}
