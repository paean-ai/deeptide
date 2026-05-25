use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use deeptide_core::{
    CommandContext, CommandResult, CostCommand, CostSummary, CostTracker, SlashCommand, TurnUsage,
};

#[test]
fn cost_command_reports_empty_session() {
    let output = text(CostCommand.execute("", &CommandContext::default()));

    assert_eq!(output, "No turns recorded yet.");
}

#[test]
fn cost_command_show_hide_and_toggle_update_display_preference() {
    let enabled = Arc::new(AtomicBool::new(false));
    let context = CommandContext::builder()
        .cost_display_enabled({
            let enabled = Arc::clone(&enabled);
            move || enabled.load(Ordering::SeqCst)
        })
        .set_cost_display_enabled({
            let enabled = Arc::clone(&enabled);
            move |next| enabled.store(next, Ordering::SeqCst)
        })
        .build();

    let show = text(CostCommand.execute("show", &context));
    assert!(enabled.load(Ordering::SeqCst));
    assert!(show.contains("Estimated cost display enabled for this session."));

    let hide = text(CostCommand.execute("hide", &context));
    assert!(!enabled.load(Ordering::SeqCst));
    assert!(hide.contains("Estimated cost display hidden."));

    let toggle = text(CostCommand.execute("toggle", &context));
    assert!(enabled.load(Ordering::SeqCst));
    assert_eq!(toggle, "Estimated cost display enabled.");
}

#[test]
fn cost_command_rejects_unknown_arguments_with_usage() {
    let output = text(CostCommand.execute("wat", &CommandContext::default()));

    assert_eq!(output, "Usage: /cost [show | hide | toggle]");
}

#[test]
fn cost_command_renders_breakdown_and_cache_health() {
    let summary = sample_summary_single_model();
    let context = CommandContext::builder()
        .cost_summary(move || summary.clone())
        .build();

    let output = text(CostCommand.execute("", &context));

    assert!(output.contains("Cost breakdown"));
    assert!(output.contains("turn   in       out     cache+"));
    assert!(output.contains("1      100      20      100"));
    assert!(output.contains("Total:"));
    assert!(output.contains("(300 in, 60 out)"));
    assert!(output.contains("Estimate only: provider pricing and cache billing can differ."));
    assert!(output.contains("Cache health:"));
    assert!(output.contains("created"));
    assert!(output.contains("read"));
}

#[test]
fn cost_command_renders_missing_cache_diagnostic() {
    let tracker = CostTracker::new();
    tracker.record(TurnUsage::new(1, "unknown", 100, 20, 0, 0, 10));
    tracker.record(TurnUsage::new(2, "unknown", 100, 20, 0, 0, 10));
    let summary = tracker.summary();
    let context = CommandContext::builder()
        .cost_summary(move || summary.clone())
        .build();

    let output = text(CostCommand.execute("", &context));

    assert!(output.contains("Cache health: unreported"));
    assert!(output.contains("provider did not report cache telemetry"));
}

#[test]
fn cost_command_adds_model_rollup_for_mixed_models() {
    let summary = sample_summary_mixed_models();
    let context = CommandContext::builder()
        .cost_summary(move || summary.clone())
        .build();

    let output = text(CostCommand.execute("", &context));

    assert!(output.contains("By model:"));
    assert!(output.contains("deepseek-v4-pro"));
    assert!(output.contains("deepseek-v4-flash"));
    assert!(output.contains("cache"));
}

fn sample_summary_single_model() -> CostSummary {
    let tracker = CostTracker::new();
    tracker.record(TurnUsage::new(1, "deepseek-v4-pro", 100, 20, 100, 800, 10));
    tracker.record(TurnUsage::new(2, "deepseek-v4-pro", 200, 40, 50, 100, 20));
    tracker.summary()
}

fn sample_summary_mixed_models() -> CostSummary {
    let tracker = CostTracker::new();
    tracker.record(TurnUsage::new(1, "deepseek-v4-pro", 100, 20, 100, 800, 10));
    tracker.record(TurnUsage::new(2, "deepseek-v4-flash", 200, 40, 50, 100, 20));
    tracker.summary()
}

fn text(result: CommandResult) -> String {
    match result {
        CommandResult::Text(value) => value,
        other => panic!("expected text command result, got {other:?}"),
    }
}
