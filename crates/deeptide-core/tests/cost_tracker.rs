use std::collections::HashMap;

use deeptide_core::{CostTracker, ModelPricing, TurnUsage};

#[test]
fn cache_health_reports_strong_hit_rate() {
    let tracker = CostTracker::new();
    tracker.record(TurnUsage::new(1, "deepseek-v4-pro", 100, 20, 100, 800, 10));

    let health = tracker.summary().cache_health();

    assert_eq!(health.hit_rate_percent, Some(80));
    assert_eq!(health.recent_hit_rate_percent, Some(80));
    assert_eq!(health.label(), "strong");
    assert_eq!(health.diagnostic(), None);
}

#[test]
fn cache_health_diagnoses_recreated_cache() {
    let tracker = CostTracker::new();
    tracker.record(TurnUsage::new(1, "deepseek-v4-pro", 400, 20, 600, 100, 10));

    let health = tracker.summary().cache_health();

    assert_eq!(health.hit_rate_percent, Some(9));
    assert_eq!(health.label(), "cold");
    assert_eq!(
        health.diagnostic(),
        Some(
            "cache is being recreated more than read; avoid changing model, tools, or stable prompt prefix mid-session"
        )
    );
}

#[test]
fn cache_health_handles_missing_provider_telemetry() {
    let tracker = CostTracker::new();
    tracker.record(TurnUsage::new(1, "unknown", 100, 20, 0, 0, 10));
    tracker.record(TurnUsage::new(2, "unknown", 100, 20, 0, 0, 10));

    let health = tracker.summary().cache_health();

    assert_eq!(health.hit_rate_percent, None);
    assert_eq!(health.label(), "unreported");
    assert_eq!(
        health.diagnostic(),
        Some("provider did not report cache telemetry; verify endpoint support")
    );
}

#[test]
fn cache_health_marks_single_unreported_turn_as_warming() {
    let tracker = CostTracker::new();
    tracker.record(TurnUsage::new(1, "unknown", 100, 20, 0, 0, 10));

    let health = tracker.summary().cache_health();

    assert_eq!(health.hit_rate_percent, None);
    assert_eq!(health.label(), "warming");
    assert_eq!(
        health.diagnostic(),
        Some("first turn usually creates cache before later turns can read it")
    );
}

#[test]
fn summary_tracks_turns_totals_and_pricing() {
    let tracker = CostTracker::new();
    tracker.record(TurnUsage::new(1, "deepseek-v4-pro", 100, 20, 100, 800, 10));
    tracker.record(TurnUsage::new(2, "deepseek-v4-flash", 200, 40, 50, 100, 20));

    let summary = tracker.summary();

    assert_eq!(summary.turns.len(), 2);
    assert_eq!(summary.total_input, 300);
    assert_eq!(summary.total_output, 60);
    assert_eq!(summary.total_cache_create, 150);
    assert_eq!(summary.total_cache_read, 900);
    assert!(summary.total_cost_usd > 0.0);
}

#[test]
fn pricing_uses_overrides_exact_match_and_builtin_prefix_match() {
    let mut overrides = HashMap::new();
    overrides.insert(
        "custom-model".to_owned(),
        ModelPricing::new(1.0, 2.0, 3.0, 4.0),
    );
    let tracker = CostTracker::with_pricing_overrides(overrides);

    assert_eq!(
        tracker.pricing_for("custom-model"),
        ModelPricing::new(1.0, 2.0, 3.0, 4.0)
    );
    assert_eq!(
        tracker.pricing_for("deepseek-v4-pro-2026-04-01"),
        tracker.pricing_for("deepseek-v4-pro")
    );
    assert_eq!(
        tracker.pricing_for("unknown"),
        ModelPricing::new(0.0, 0.0, 0.0, 0.0)
    );
}

#[test]
fn reset_clears_accumulated_state() {
    let tracker = CostTracker::new();
    tracker.record(TurnUsage::new(1, "deepseek-v4-pro", 100, 20, 10, 10, 10));

    tracker.reset();
    let summary = tracker.summary();

    assert!(summary.turns.is_empty());
    assert_eq!(summary.total_input, 0);
    assert_eq!(summary.total_cost_usd, 0.0);
}

#[test]
fn formatting_matches_swift_precision_and_grouping() {
    assert_eq!(CostTracker::format_usd(0.00123), "$0.0012");
    assert_eq!(CostTracker::format_usd(0.01234), "$0.012");
    assert_eq!(CostTracker::format_usd(1.234), "$1.23");

    assert_eq!(CostTracker::format_tokens(0), "0");
    assert_eq!(CostTracker::format_tokens(999), "999");
    assert_eq!(CostTracker::format_tokens(1_234), "1,234");
    assert_eq!(CostTracker::format_tokens(1_234_567), "1,234,567");
}
