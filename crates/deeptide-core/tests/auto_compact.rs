//! Integration tests for the smart auto-compact feature.
//!
//! Strategy mirrors `context_warn.rs`: feed the REPL a synthetic
//! backend that returns enough text to push `estimate_repl_context_tokens`
//! over the model's 128 K window. Then assert the auto-compact
//! pipeline triggers (or doesn't) according to the configured state.
//!
//! All tests construct their own `ReplSession` so they're isolated
//! from each other and from the warning-test state, even though the
//! suite runs in parallel.

#![allow(clippy::unwrap_used)]

use deeptide_core::{
    AUTO_COMPACT_DEFAULT_THRESHOLD, AUTO_COMPACT_MAX_THRESHOLD, AUTO_COMPACT_MIN_THRESHOLD,
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplEvent, ReplSession,
};

/// Generate a synthetic reply large enough to comfortably exceed the
/// default 128 K-token context window. The 520 000 ASCII chars come
/// out to ≈130 000 tokens under the 4-char/token estimator, which
/// clears every auto-compact threshold including the max (99%).
fn huge_reply() -> String {
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

/// Tiny backend used by tests that don't want to blow the context
/// window — for `/auto-compact` grammar exercises in particular.
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

fn fired_count(repl: &ReplSession) -> usize {
    repl.auto_compact_config().fired_count
}

// ── default state ──────────────────────────────────────────────────

#[test]
fn auto_compact_is_off_by_default_and_does_not_fire_under_pressure() {
    let mut repl = ReplSession::new(Box::new(HugeBackend));
    let text = collect_text(repl.submit("trigger"));
    // The warning still fires because that's a separate code path,
    // but the auto-compact notice must NOT be in there.
    assert!(
        !text.contains("auto-compact triggered"),
        "auto-compact must stay off until the user opts in: {text}"
    );
    assert_eq!(fired_count(&repl), 0);
}

// ── opt-in via /auto-compact on ────────────────────────────────────

/// Run enough huge turns to actually exercise the REPL-level
/// auto-compact code path. The transcript only becomes
/// "compactable" once `messages.len() > window_size` (default 6
/// — see `crates/deeptide-core/src/context_window.rs`). Empirically
/// that happens at the 6th huge submission (1 system + 6 user + 6
/// assistant = 13 messages, well above the window). Below that
/// `force_compress` is a no-op even when the token total is way
/// over budget — there's literally nothing older than the preserved
/// window to fold into a summary. Returns the collected output of
/// the FINAL turn so callers can assert on the auto-compact notice.
const TURNS_UNTIL_COMPACTABLE: usize = 6;

fn run_until_compactable(repl: &mut ReplSession) -> String {
    let mut last = String::new();
    for i in 0..TURNS_UNTIL_COMPACTABLE {
        last = collect_text(repl.submit(&format!("turn {i}")));
    }
    last
}

#[test]
fn auto_compact_fires_after_enable_and_enough_history_accumulates() {
    let mut repl = ReplSession::new(Box::new(HugeBackend));
    let enable = collect_text(repl.submit("/auto-compact on"));
    assert!(
        enable.to_ascii_lowercase().contains("enabled"),
        "enable response should confirm the state change: {enable}"
    );

    let last = run_until_compactable(&mut repl);
    assert!(
        last.contains("auto-compact triggered") || fired_count(&repl) >= 1,
        "after enable + enough huge turns the REPL-level auto-compact must fire \
         at least once. Last turn output: {last}"
    );
    assert!(
        fired_count(&repl) >= 1,
        "fired_count must increment on a real auto-compact (got {})",
        fired_count(&repl)
    );
}

// ── shorthand: /auto-compact <N> enables + sets threshold ──────────

#[test]
fn numeric_shorthand_enables_and_overrides_threshold() {
    let mut repl = ReplSession::new(Box::new(EchoBackend));
    let resp = collect_text(repl.submit("/auto-compact 88"));
    let cfg = repl.auto_compact_config();
    assert!(cfg.enabled, "shorthand must enable the feature: {resp}");
    assert_eq!(cfg.threshold_percent, 88);
    assert!(
        resp.contains("88"),
        "echo must mention the new threshold: {resp}"
    );
}

// ── clamping: out-of-range thresholds are bounded ──────────────────

#[test]
fn threshold_below_minimum_is_clamped_to_min() {
    let mut repl = ReplSession::new(Box::new(EchoBackend));
    let _ = repl.submit("/auto-compact threshold 5");
    assert_eq!(
        repl.auto_compact_config().threshold_percent,
        AUTO_COMPACT_MIN_THRESHOLD,
        "threshold 5 must clamp to MIN={AUTO_COMPACT_MIN_THRESHOLD}"
    );
}

#[test]
fn threshold_above_maximum_is_clamped_to_max() {
    let mut repl = ReplSession::new(Box::new(EchoBackend));
    let _ = repl.submit("/auto-compact threshold 200");
    assert_eq!(
        repl.auto_compact_config().threshold_percent,
        AUTO_COMPACT_MAX_THRESHOLD,
        "threshold 200 must clamp to MAX={AUTO_COMPACT_MAX_THRESHOLD}"
    );
}

// ── /auto-compact off after enable ─────────────────────────────────

#[test]
fn off_disables_the_feature_and_subsequent_pressure_does_not_fire() {
    let mut repl = ReplSession::new(Box::new(HugeBackend));
    let _ = repl.submit("/auto-compact on");
    // Build up the message backlog so we get into the regime where
    // auto-compact CAN fire — without this the test would
    // tautologically pass against any implementation because no
    // compaction is possible in the first place.
    let _ = run_until_compactable(&mut repl);
    let baseline = fired_count(&repl);
    assert!(
        baseline >= 1,
        "precondition: enable + huge turns must fire at least once (got {baseline})"
    );

    let _ = repl.submit("/auto-compact off");
    assert!(!repl.auto_compact_config().enabled);
    // Keep submitting huge turns while disabled — the warning will
    // keep latching but the fired counter must stay flat.
    let _ = run_until_compactable(&mut repl);
    assert_eq!(
        fired_count(&repl),
        baseline,
        "fired_count must not advance while disabled"
    );
}

// ── status output mentions thresholds + current state ──────────────

#[test]
fn status_shows_enabled_threshold_and_fired_count() {
    let mut repl = ReplSession::new(Box::new(EchoBackend));
    let _ = repl.submit("/auto-compact on");
    let status = collect_text(repl.submit("/auto-compact"));
    assert!(status.contains("ON"), "status must announce on: {status}");
    assert!(
        status.contains(&format!("{AUTO_COMPACT_DEFAULT_THRESHOLD}")),
        "status must announce the threshold: {status}"
    );
    assert!(
        status.to_ascii_lowercase().contains("fired"),
        "status must include fired counter: {status}"
    );
}

// ── reset zeroes the counter without touching enable/threshold ─────

#[test]
fn reset_zeroes_the_fired_counter_only() {
    let mut repl = ReplSession::new(Box::new(HugeBackend));
    let _ = repl.submit("/auto-compact on");
    let _ = run_until_compactable(&mut repl);
    let before = fired_count(&repl);
    assert!(
        before >= 1,
        "precondition: at least one fire before reset (got {before})"
    );

    let resp = collect_text(repl.submit("/auto-compact reset"));
    assert_eq!(
        fired_count(&repl),
        0,
        "reset must zero the lifetime counter: {resp}"
    );
    assert!(repl.auto_compact_config().enabled, "reset must NOT disable");
    assert_eq!(
        repl.auto_compact_config().threshold_percent,
        AUTO_COMPACT_DEFAULT_THRESHOLD,
        "reset must NOT change the threshold"
    );
}

// ── unknown subcommand path surfaces help ──────────────────────────

#[test]
fn unknown_subcommand_returns_help_text_and_lists_grammar() {
    let mut repl = ReplSession::new(Box::new(EchoBackend));
    let resp = collect_text(repl.submit("/auto-compact frobnicate"));
    assert!(
        resp.to_ascii_lowercase().contains("unknown"),
        "unknown sub should announce itself: {resp}"
    );
    for form in ["on", "off", "threshold", "reset"] {
        assert!(
            resp.contains(form),
            "unknown-sub help must mention grammar form `{form}`: {resp}"
        );
    }
}

// ── status bar segment exposes the auto<N> indicator ───────────────

#[test]
fn status_bar_ctx_segment_includes_auto_suffix_when_enabled() {
    let mut repl = ReplSession::new(Box::new(EchoBackend));
    // Disabled by default — no `auto` suffix.
    let before = repl.status_line().render(200);
    assert!(
        !before.contains("auto"),
        "disabled state must not show an auto indicator: {before}"
    );

    let _ = repl.submit("/auto-compact 80");
    let after = repl.status_line().render(200);
    assert!(
        after.contains("auto80"),
        "enabled state must surface `autoN` on the ctx segment: {after}"
    );
}
