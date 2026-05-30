//! Integration tests for the `/think` and `/search` slash commands.
//!
//! `/think` is exercised through `ReplSession::submit("/think ...")`
//! and we observe its effect on `agent_loop().thinking_override()`,
//! which is what threads the directive onto the wire on the next
//! request.
//!
//! `/search` operates on `self.agent_loop.messages()` so we need a
//! backend that lets us seed both a user turn and an assistant turn —
//! the `StaticBackend` from this file does both. We then submit
//! `/search …` and assert on its rendered hit list.

#![allow(clippy::unwrap_used)]

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplEvent, ReplSession,
    api::ThinkingConfig,
};

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

// ---- /think -----------------------------------------------------------

#[test]
fn think_status_without_override_reports_auto() {
    let mut repl = make_repl("ok");
    let text = collect_text(repl.submit("/think"));
    assert!(
        text.contains("auto"),
        "no-arg status should say auto: {text}"
    );
    assert!(text.contains("/think on"));
}

#[test]
fn think_on_enables_medium_budget_override() {
    let mut repl = make_repl("ok");
    let _ = repl.submit("/think on");
    let cfg = repl.agent_loop().thinking_override().cloned();
    assert_eq!(cfg, Some(ThinkingConfig::medium()));
}

#[test]
fn think_low_high_match_preset_constructors() {
    let mut repl = make_repl("ok");
    let _ = repl.submit("/think low");
    assert_eq!(
        repl.agent_loop().thinking_override().cloned(),
        Some(ThinkingConfig::low())
    );
    let _ = repl.submit("/think high");
    assert_eq!(
        repl.agent_loop().thinking_override().cloned(),
        Some(ThinkingConfig::high())
    );
}

#[test]
fn think_off_sets_explicit_disabled_directive() {
    let mut repl = make_repl("ok");
    let _ = repl.submit("/think off");
    let cfg = repl
        .agent_loop()
        .thinking_override()
        .cloned()
        .expect("override should be set to disabled, not cleared");
    assert!(!cfg.is_enabled(), "off must be `disabled`, got: {cfg:?}");
}

#[test]
fn think_auto_clears_the_override() {
    let mut repl = make_repl("ok");
    let _ = repl.submit("/think on");
    assert!(repl.agent_loop().thinking_override().is_some());
    let _ = repl.submit("/think auto");
    assert!(repl.agent_loop().thinking_override().is_none());
}

#[test]
fn think_budget_with_value_threads_through_to_override() {
    let mut repl = make_repl("ok");
    let _ = repl.submit("/think budget 8000");
    let cfg = repl
        .agent_loop()
        .thinking_override()
        .cloned()
        .expect("override set");
    assert!(cfg.is_enabled());
    assert_eq!(cfg.budget_tokens, Some(8000));
}

#[test]
fn think_budget_clamps_too_small_value() {
    let mut repl = make_repl("ok");
    let text = collect_text(repl.submit("/think budget 50"));
    assert!(text.contains("clamped"), "must report clamping: {text}");
    let cfg = repl
        .agent_loop()
        .thinking_override()
        .cloned()
        .expect("override set");
    assert_eq!(cfg.budget_tokens, Some(1024));
}

#[test]
fn think_budget_without_value_returns_usage() {
    let mut repl = make_repl("ok");
    let text = collect_text(repl.submit("/think budget"));
    assert!(text.starts_with("Usage:"));
}

#[test]
fn think_unknown_subcommand_lists_valid_ones() {
    let mut repl = make_repl("ok");
    let text = collect_text(repl.submit("/think wat"));
    assert!(text.contains("Unknown"));
    assert!(text.contains("/think status"));
}

// ---- /search ----------------------------------------------------------

#[test]
fn search_without_query_returns_usage() {
    let mut repl = make_repl("ok");
    let text = collect_text(repl.submit("/search"));
    assert!(text.starts_with("Usage:"));
    assert!(text.contains("--regex"));
}

#[test]
fn search_finds_substring_in_user_message() {
    let mut repl = make_repl("got it");
    let _ = repl.submit("please refactor the parser module");
    let text = collect_text(repl.submit("/search parser"));
    assert!(text.contains("Search hits"));
    assert!(text.contains("parser module"));
    assert!(text.contains("user"));
}

#[test]
fn search_finds_substring_in_assistant_message() {
    let mut repl = make_repl("Sure, the streaming logic lives in api.rs");
    let _ = repl.submit("where does streaming live?");
    let text = collect_text(repl.submit("/search streaming"));
    // The assistant text contains "streaming logic".
    assert!(text.contains("assistant"), "must find in assistant: {text}");
    assert!(text.contains("streaming logic"));
}

#[test]
fn search_is_case_insensitive() {
    let mut repl = make_repl("ok");
    let _ = repl.submit("UPPERCASE SHOUTING");
    let text = collect_text(repl.submit("/search shouting"));
    assert!(text.contains("Search hits"));
    assert!(text.contains("UPPERCASE"));
}

#[test]
fn search_regex_flag_compiles_pattern() {
    let mut repl = make_repl("ok");
    let _ = repl.submit("the version is 1.4.7");
    let text = collect_text(repl.submit("/search --regex \\d+\\.\\d+"));
    assert!(text.contains("Search hits"));
    assert!(text.contains("1.4.7"));
}

#[test]
fn search_regex_with_bad_pattern_reports_error() {
    let mut repl = make_repl("ok");
    let text = collect_text(repl.submit("/search --regex ((("));
    assert!(text.contains("Invalid regex"), "bad pattern: {text}");
}

#[test]
fn search_no_match_reports_zero_hits() {
    let mut repl = make_repl("yes");
    let _ = repl.submit("ordinary message");
    let text = collect_text(repl.submit("/search xyzzy"));
    assert!(text.contains("No matches"));
    assert!(text.contains("xyzzy"));
}

#[test]
fn search_truncates_long_lines_in_preview() {
    let mut repl = make_repl("ok");
    let long: String = "needle".to_owned() + &" filler".repeat(200);
    let _ = repl.submit(&long);
    let text = collect_text(repl.submit("/search needle"));
    // Truncation marker should appear (the line is > 140 chars).
    assert!(text.contains("…"), "long lines must truncate: {text}");
}

#[test]
fn search_caps_total_hits_with_overflow_notice() {
    let mut repl = make_repl("ok");
    // Seed a single multi-line message with 40 needle lines so we
    // blow past MAX_HITS = 32 without needing 32 backend calls.
    let many_needles = (0..40)
        .map(|i| format!("line-{i} needle"))
        .collect::<Vec<_>>()
        .join("\n");
    let _ = repl.submit(&many_needles);
    let text = collect_text(repl.submit("/search needle"));
    assert!(
        text.contains("more hit(s) suppressed"),
        "no overflow: {text}"
    );
}
