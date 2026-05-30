//! Integration tests for the `/queue` slash command + the shared
//! `MessageQueue` handle exposed by `ReplSession`. These exercise the
//! end-to-end paths the CLI relies on:
//!
//!   * `/queue list` / `/queue list` empty case
//!   * `/queue add` + dedup of whitespace-only input
//!   * `/queue pop`, `/queue clear`
//!   * `/queue mode {single|batch}` parses, persists, and affects drain
//!   * `ReplSession::drain_next_queued_prompt` honours the mode
//!   * `ReplSession::message_queue_handle()` lets a worker thread enqueue
//!     concurrently with the main thread reading state
//!   * the status line surfaces a `queue N` segment only when non-empty
//!
//! The backend is the cheapest possible "echo a fixed reply" stub so
//! tests stay fast and deterministic.

use std::sync::{Arc, Mutex};
use std::thread;

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, QueueMode, ReplEvent, ReplSession,
};

struct StaticBackend;

impl AgentBackend for StaticBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: String::from("ack"),
            usage: Some(AgentUsage::new(1, 1, 0, 0, 0)),
            tool_calls: Vec::new(),
        })
    }
}

fn output_lines(events: Vec<ReplEvent>) -> String {
    let mut out = String::new();
    for event in events {
        if let ReplEvent::Output(line) = event {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(&line);
        }
    }
    out
}

#[test]
fn queue_list_when_empty_shows_zero_pending_and_mode() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let text = output_lines(repl.submit("/queue"));
    assert!(
        text.contains("0 pending"),
        "empty queue list should show depth 0: {text}"
    );
    assert!(
        text.contains("mode: single"),
        "empty queue list should advertise default mode: {text}"
    );
    assert!(
        text.contains("empty"),
        "empty queue list should hint that the queue is empty: {text}"
    );
}

#[test]
fn queue_add_appends_and_list_renders_preview() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let add = output_lines(repl.submit("/queue add explain the code"));
    assert!(add.contains("Queued (#1)"), "add response: {add}");

    let list = output_lines(repl.submit("/queue list"));
    assert!(list.contains("1 pending"), "depth in list: {list}");
    assert!(list.contains("explain the code"), "preview in list: {list}");
}

#[test]
fn queue_add_rejects_whitespace_only_message() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let response = output_lines(repl.submit("/queue add    "));
    assert!(
        response.to_lowercase().contains("usage"),
        "no-arg add should print usage hint: {response}"
    );

    let list = output_lines(repl.submit("/queue list"));
    assert!(
        list.contains("0 pending"),
        "whitespace add must not enqueue: {list}"
    );
}

#[test]
fn queue_pop_removes_head_and_returns_preview() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    repl.submit("/queue add first");
    repl.submit("/queue add second");
    let pop = output_lines(repl.submit("/queue pop"));
    assert!(pop.contains("first"), "pop should return head: {pop}");

    let list = output_lines(repl.submit("/queue list"));
    assert!(list.contains("1 pending"));
    assert!(list.contains("second"));
}

#[test]
fn queue_pop_when_empty_is_a_no_op_with_friendly_message() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let pop = output_lines(repl.submit("/queue pop"));
    assert!(
        pop.to_lowercase().contains("empty"),
        "empty pop response: {pop}"
    );
}

#[test]
fn queue_clear_drops_all_pending() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    repl.submit("/queue add a");
    repl.submit("/queue add b");
    repl.submit("/queue add c");

    let clear = output_lines(repl.submit("/queue clear"));
    assert!(
        clear.contains("Cleared 3"),
        "clear should report count: {clear}"
    );

    let list = output_lines(repl.submit("/queue list"));
    assert!(list.contains("0 pending"));
}

#[test]
fn queue_mode_switch_persists_and_round_trips() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let initial = output_lines(repl.submit("/queue mode"));
    assert!(initial.contains("single"));

    let switched = output_lines(repl.submit("/queue mode batch"));
    assert!(switched.contains("batch"), "switch reply: {switched}");

    let after = output_lines(repl.submit("/queue list"));
    assert!(after.contains("mode: batch"), "list after switch: {after}");
}

#[test]
fn queue_mode_rejects_unknown_value() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let resp = output_lines(repl.submit("/queue mode banana"));
    assert!(
        resp.to_lowercase().contains("unknown"),
        "bad mode response: {resp}"
    );
}

#[test]
fn drain_next_queued_prompt_single_mode_pops_head() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    repl.submit("/queue add first");
    repl.submit("/queue add second");
    repl.submit("/queue add third");

    assert_eq!(repl.drain_next_queued_prompt().as_deref(), Some("first"));
    assert_eq!(repl.drain_next_queued_prompt().as_deref(), Some("second"));
    assert_eq!(repl.drain_next_queued_prompt().as_deref(), Some("third"));
    assert_eq!(repl.drain_next_queued_prompt(), None);
}

#[test]
fn drain_next_queued_prompt_batch_mode_joins_with_blank_line() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    repl.submit("/queue mode batch");
    repl.submit("/queue add first");
    repl.submit("/queue add second");

    let drained = repl.drain_next_queued_prompt().expect("batch drain");
    assert_eq!(drained, "first\n\nsecond");
    assert_eq!(
        repl.drain_next_queued_prompt(),
        None,
        "batch must clear the queue"
    );
}

#[test]
fn message_queue_handle_is_shared_across_threads() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let handle = repl.message_queue_handle();
    assert!(Arc::strong_count(&handle) >= 2, "session must keep a ref");

    // Simulate the CLI's mid-turn poll: a background thread pushes via
    // the handle while the main thread is "busy".
    let h_clone: Arc<Mutex<deeptide_core::MessageQueue>> = Arc::clone(&handle);
    let worker = thread::spawn(move || {
        for n in 0..5 {
            if let Ok(mut q) = h_clone.lock() {
                let _ = q.push(format!("mid-turn line {n}"));
            }
        }
    });
    worker.join().expect("background pusher must not panic");

    let list = output_lines(repl.submit("/queue list"));
    assert!(
        list.contains("5 pending"),
        "main thread should observe background pushes: {list}"
    );
}

#[test]
fn status_line_includes_queue_segment_only_when_non_empty() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    // Note: a `queue` substring can legitimately appear elsewhere in the
    // bar (e.g. when git reports a branch named `feat/message-queue`),
    // so we look for the full segment label-value pair instead of the
    // bare word.
    let line_empty = repl.status_line().render(200);
    assert!(
        !line_empty.contains("queue 0")
            && !line_empty.contains("queue 1")
            && !line_empty.contains("queue 2"),
        "empty queue must NOT add a `queue N` segment: {line_empty}"
    );

    repl.submit("/queue add first");
    repl.submit("/queue add second");
    let line_full = repl.status_line().render(200);
    assert!(
        line_full.contains("queue 2"),
        "non-empty queue must surface in status: {line_full}"
    );
}

#[test]
fn message_queue_snapshot_matches_internal_state() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    repl.submit("/queue add alpha");
    repl.submit("/queue add beta");

    let (items, mode, len) = repl.message_queue_snapshot();
    assert_eq!(items, vec!["alpha".to_owned(), "beta".to_owned()]);
    assert_eq!(mode, QueueMode::Single);
    assert_eq!(len, 2);
}

#[test]
fn drain_after_mode_change_uses_new_strategy_on_existing_items() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    repl.submit("/queue add one");
    repl.submit("/queue add two");
    // Switching to batch must take effect immediately on the next drain
    // call, even for items queued under `single`.
    repl.submit("/queue mode batch");
    let drained = repl.drain_next_queued_prompt().expect("batch drain");
    assert_eq!(drained, "one\n\ntwo");
}

#[test]
fn unknown_queue_subcommand_surfaces_help_hint() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let resp = output_lines(repl.submit("/queue wat"));
    assert!(
        resp.contains("Unknown subcommand"),
        "unknown subcommand response: {resp}"
    );
    assert!(resp.contains("/queue list"));
    assert!(resp.contains("/queue add"));
}
