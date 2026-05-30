//! Integration tests for `/checkpoint` + `/rewind`.
//!
//! Each test drives `ReplSession::submit("/...")` through a static
//! backend so we can grow the transcript deterministically. The tests
//! assert on two surfaces: the rendered text returned to the user (the
//! UX contract), and `ReplSession::checkpoint_count()` plus
//! `agent_loop().messages().len()` (the state contract).

#![allow(clippy::unwrap_used)]

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplEvent, ReplSession,
};

/// Static backend: every turn returns the same canned reply with zero
/// tool calls. Sufficient to grow a transcript of size N + 1 (user +
/// assistant) for each `repl.submit("…")` call.
struct StaticBackend {
    reply: String,
}

impl AgentBackend for StaticBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: self.reply.clone(),
            usage: Some(AgentUsage::new(1, 1, 0, 0, 0)),
            tool_calls: Vec::new(),
        })
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

fn make_repl(reply: &str) -> ReplSession {
    ReplSession::new(Box::new(StaticBackend {
        reply: reply.to_owned(),
    }))
}

/// Drive one user turn through the REPL to grow the transcript by 2.
fn one_turn(repl: &mut ReplSession, prompt: &str) {
    let _ = repl.submit(prompt);
}

// ── /checkpoint save ─────────────────────────────────────────────────

#[test]
fn checkpoint_on_empty_transcript_refuses_to_capture() {
    let mut repl = make_repl("hi");
    let text = collect_text(repl.submit("/checkpoint"));
    assert_eq!(repl.checkpoint_count(), 0);
    assert!(
        text.to_ascii_lowercase().contains("nothing to checkpoint"),
        "expected refusal text, got: {text}"
    );
}

#[test]
fn checkpoint_save_after_one_turn_captures_two_messages() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "hello");

    let text = collect_text(repl.submit("/checkpoint"));
    assert_eq!(repl.checkpoint_count(), 1);
    assert!(text.contains("captured 2 message(s)"), "got: {text}");
    assert!(text.starts_with("✔"));
}

#[test]
fn checkpoint_save_with_label_records_label_in_listing() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "hello");

    let _ = repl.submit("/checkpoint pre-refactor");
    let listing = collect_text(repl.submit("/checkpoints"));
    assert!(
        listing.contains("pre-refactor"),
        "label should appear in listing: {listing}"
    );
    assert!(listing.contains("msgs=2"));
}

#[test]
fn checkpoint_save_rejects_newline_in_label() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "hello");
    let text = collect_text(repl.submit("/checkpoint line1\nline2"));
    assert_eq!(repl.checkpoint_count(), 0, "save must not have stored");
    assert!(text.contains("cannot contain newlines"), "got: {text}");
}

#[test]
fn checkpoint_save_truncates_overlong_label_silently() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "hello");
    let long: String = "a".repeat(200);
    let _ = repl.submit(&format!("/checkpoint {long}"));
    assert_eq!(repl.checkpoint_count(), 1);
    let listing = collect_text(repl.submit("/checkpoints"));
    // 64-char cap.
    assert!(listing.contains(&"a".repeat(64)));
    assert!(!listing.contains(&"a".repeat(65)));
}

// ── /checkpoint list ─────────────────────────────────────────────────

#[test]
fn checkpoints_listing_on_empty_store_is_helpful() {
    let mut repl = make_repl("ok");
    let text = collect_text(repl.submit("/checkpoints"));
    assert!(
        text.to_ascii_lowercase().contains("no checkpoints"),
        "got: {text}"
    );
    assert!(text.contains("/checkpoint"));
}

#[test]
fn checkpoints_listing_shows_total_and_cap() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "hello");
    let _ = repl.submit("/checkpoint a");
    let _ = repl.submit("/checkpoint b");
    let text = collect_text(repl.submit("/checkpoints"));
    assert!(text.starts_with("Checkpoints (2/20)"), "got: {text}");
}

// ── /rewind + /checkpoint restore ────────────────────────────────────

#[test]
fn rewind_without_arg_restores_newest_checkpoint() {
    let mut repl = make_repl("ok");

    // Turn 1: 2 msgs, snapshot.
    one_turn(&mut repl, "first");
    let _ = repl.submit("/checkpoint cp1");
    let after_first = repl.agent_loop().messages().len();
    assert_eq!(after_first, 2);

    // Turn 2: grows to 4 msgs.
    one_turn(&mut repl, "second");
    assert_eq!(repl.agent_loop().messages().len(), 4);

    let text = collect_text(repl.submit("/rewind"));
    assert!(text.contains("rewound"), "got: {text}");
    assert_eq!(repl.agent_loop().messages().len(), 2);
}

#[test]
fn rewind_by_index_picks_the_right_snapshot() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "t1");
    let _ = repl.submit("/checkpoint a");
    one_turn(&mut repl, "t2");
    let _ = repl.submit("/checkpoint b");
    one_turn(&mut repl, "t3");

    // Pre-rewind transcript depth: t1+t2+t3 → 6 messages.
    assert_eq!(repl.agent_loop().messages().len(), 6);

    // Restore index 1 (cp `a`, captured at 2 msgs).
    let text = collect_text(repl.submit("/rewind 1"));
    assert!(text.contains("rewound"), "got: {text}");
    assert_eq!(repl.agent_loop().messages().len(), 2);
}

#[test]
fn rewind_by_label_works() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "t1");
    let _ = repl.submit("/checkpoint anchor");
    one_turn(&mut repl, "t2");
    one_turn(&mut repl, "t3");
    assert_eq!(repl.agent_loop().messages().len(), 6);

    let _ = repl.submit("/rewind anchor");
    assert_eq!(repl.agent_loop().messages().len(), 2);
}

#[test]
fn rewind_missing_selector_reports_clearly() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "x");
    let _ = repl.submit("/checkpoint a");
    let text = collect_text(repl.submit("/rewind bogus"));
    assert!(text.contains("No checkpoint matches"), "got: {text}");
}

#[test]
fn rewind_on_empty_store_reports_helpful_hint() {
    let mut repl = make_repl("ok");
    let text = collect_text(repl.submit("/rewind"));
    assert!(
        text.to_ascii_lowercase().contains("no checkpoints"),
        "got: {text}"
    );
}

#[test]
fn rewind_same_position_reports_already_matches() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "t1");
    let _ = repl.submit("/checkpoint a");

    // No new turns → restoring should be a no-op.
    let text = collect_text(repl.submit("/rewind"));
    assert!(
        text.contains("already matches"),
        "no-op restore should annotate the message: {text}"
    );
}

// ── /checkpoint drop + clear ─────────────────────────────────────────

#[test]
fn checkpoint_drop_by_id_removes_one() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "x");
    let _ = repl.submit("/checkpoint a");
    let _ = repl.submit("/checkpoint b");
    assert_eq!(repl.checkpoint_count(), 2);

    // Drop by label.
    let text = collect_text(repl.submit("/checkpoint drop a"));
    assert!(text.starts_with("Dropped checkpoint"), "got: {text}");
    assert_eq!(repl.checkpoint_count(), 1);
}

#[test]
fn checkpoint_drop_requires_selector() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "x");
    let _ = repl.submit("/checkpoint a");
    let text = collect_text(repl.submit("/checkpoint drop"));
    assert!(text.starts_with("Usage:"), "got: {text}");
}

#[test]
fn checkpoint_clear_removes_everything() {
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "x");
    let _ = repl.submit("/checkpoint a");
    let _ = repl.submit("/checkpoint b");
    assert_eq!(repl.checkpoint_count(), 2);

    let text = collect_text(repl.submit("/checkpoint clear"));
    assert_eq!(repl.checkpoint_count(), 0);
    assert!(text.contains("Cleared 2"), "got: {text}");

    // Second clear is harmless.
    let text2 = collect_text(repl.submit("/checkpoint clear"));
    assert!(text2.contains("No checkpoints"), "got: {text2}");
}

// ── help + unknown subcommand ────────────────────────────────────────

#[test]
fn checkpoint_help_text_documents_all_subcommands() {
    let mut repl = make_repl("ok");
    let text = collect_text(repl.submit("/checkpoint --help"));
    for needle in [
        "/checkpoint",
        "list",
        "restore",
        "drop",
        "clear",
        "Selectors",
        "FIFO",
    ] {
        assert!(text.contains(needle), "help missing {needle:?}: {text}");
    }
}

#[test]
fn checkpoint_treats_unknown_first_word_as_label() {
    // `/checkpoint <freeform>` should save with `<freeform>` as the
    // label rather than complaining about an unknown subcommand. This
    // is the path most users will hit, so the dispatch table reserves
    // only the well-known verbs.
    let mut repl = make_repl("ok");
    one_turn(&mut repl, "hi");
    let _ = repl.submit("/checkpoint frobnicate");
    assert_eq!(repl.checkpoint_count(), 1);
    let listing = collect_text(repl.submit("/checkpoints"));
    assert!(listing.contains("frobnicate"), "got: {listing}");
}
