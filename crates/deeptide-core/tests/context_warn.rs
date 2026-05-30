//! Integration test for the proactive context-window advisory.
//!
//! Strategy: use a static backend that returns a very long string,
//! enough to push `estimate_repl_context_tokens` past the model's
//! configured window. The default fallback window is 128k tokens, so
//! we generate ~520 KB of plain text (≈130k tokens by the cheap
//! 4-char/token estimator). That's enough to clear all three warning
//! thresholds (80% → 90% → 95%) without needing to mock the model
//! window function.
//!
//! Note: tests deliberately do NOT assert which bucket fires first —
//! they assert that *exactly one* warning fires per submit, that the
//! warning text mentions `/compact`, and that `/clear` rearms the
//! latch so a fresh transcript can warn again.

#![allow(clippy::unwrap_used)]

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplEvent, ReplSession,
};

/// Generate a synthetic reply large enough to comfortably exceed the
/// default 128 K-token context window. We use ASCII so the
/// 4-bytes-per-token heuristic in `estimate_tokens` is accurate.
fn huge_reply() -> String {
    // 520 000 chars ≈ 130 000 tokens > 128 000 window.
    let chunk = "the quick brown fox jumps over the lazy dog. ";
    let mut out = String::with_capacity(520_000);
    while out.len() < 520_000 {
        out.push_str(chunk);
    }
    out
}

struct HugeBackend;

impl AgentBackend for HugeBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: huge_reply(),
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

fn warning_count(text: &str) -> usize {
    text.matches("⚠ context").count()
}

#[test]
fn warning_fires_once_after_first_huge_turn() {
    let mut repl = ReplSession::new(Box::new(HugeBackend));
    let text = collect_text(repl.submit("kick"));
    assert_eq!(
        warning_count(&text),
        1,
        "expected exactly one ⚠ warning after a context-bursting turn, got: {text}",
    );
    assert!(text.contains("/compact") || text.contains("compact"));
}

#[test]
fn warning_does_not_refire_on_subsequent_turns_in_same_bucket() {
    let mut repl = ReplSession::new(Box::new(HugeBackend));
    let _ = repl.submit("first");
    let second = collect_text(repl.submit("second"));
    assert_eq!(
        warning_count(&second),
        0,
        "second turn in the same bucket must be silent: {second}",
    );
}

#[test]
fn clear_resets_the_warning_latch() {
    let mut repl = ReplSession::new(Box::new(HugeBackend));
    let _ = repl.submit("first");
    let _ = repl.submit("/clear");
    let after = collect_text(repl.submit("post-clear"));
    assert_eq!(
        warning_count(&after),
        1,
        "after /clear the latch should rearm and warn again: {after}",
    );
}

#[test]
fn new_resets_the_warning_latch() {
    let mut repl = ReplSession::new(Box::new(HugeBackend));
    let _ = repl.submit("first");
    let _ = repl.submit("/new");
    let after = collect_text(repl.submit("post-new"));
    assert_eq!(
        warning_count(&after),
        1,
        "after /new the latch should rearm and warn again: {after}",
    );
}
