use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentRequest, AgentResponse, AgentTerminalEvent,
    AgentUsage, ToolCall,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A model that never finishes: every step asks for another Read and reports
/// the same usage. Without a dollar cap only `max_turns` stops it.
struct EndlessToolBackend {
    calls: Arc<AtomicUsize>,
    usage: AgentUsage,
}

impl AgentBackend for EndlessToolBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(AgentResponse {
            content: String::from("reading again"),
            usage: Some(self.usage.clone()),
            tool_calls: vec![ToolCall::new(
                format!("toolu_{n}"),
                "Read",
                serde_json::json!({"file_path": "notes.txt", "limit": 1}),
            )],
        })
    }
}

fn endless_loop(
    model: &str,
    usage: AgentUsage,
) -> (AgentLoop, Arc<AtomicUsize>, tempfile::TempDir) {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\n").expect("write fixture");
    let calls = Arc::new(AtomicUsize::new(0));
    let loop_ = AgentLoop::new(Box::new(EndlessToolBackend {
        calls: Arc::clone(&calls),
        usage,
    }))
    .with_model(model)
    .with_cwd(temp.path())
    .with_max_turns(200);
    (loop_, calls, temp)
}

fn budget_stop(events: &[AgentLoopEvent]) -> Option<&str> {
    match events.last() {
        Some(AgentLoopEvent::Terminal(AgentTerminalEvent::BudgetExceeded(reason))) => {
            Some(reason.as_str())
        }
        _ => None,
    }
}

#[test]
fn without_a_dollar_cap_only_max_turns_stops_a_runaway_loop() {
    // 1M input + 100k output tokens per step on deepseek-v4-pro is about
    // $0.38 a step, so 200 steps is about $76 before anything stops it.
    let (mut loop_, calls, _temp) = endless_loop(
        "deepseek-v4-pro",
        AgentUsage::new(1_000_000, 100_000, 0, 0, 1),
    );

    let events = loop_.run("go");

    assert_eq!(
        events.last(),
        Some(&AgentLoopEvent::Terminal(
            AgentTerminalEvent::MaxTurnsReached { cap: 200 }
        ))
    );
    assert_eq!(calls.load(Ordering::SeqCst), 200);
    assert!(loop_.cost_tracker().summary().total_cost_usd > 70.0);
}

#[test]
fn dollar_cap_stops_the_loop_once_spend_reaches_it() {
    let (loop_, calls, _temp) = endless_loop(
        "deepseek-v4-pro",
        AgentUsage::new(1_000_000, 100_000, 0, 0, 1),
    );
    let mut loop_ = loop_.with_max_budget_usd(1.0);

    let events = loop_.run("go");

    let reason = budget_stop(&events).expect("stopped by the dollar cap");
    assert!(reason.contains("spend cap reached"), "{reason}");
    // $0.38 per step: steps 1-2 stay under $1, step 3 crosses it, then stop.
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    let tracked = loop_.cost_tracker().summary().total_cost_usd;
    let booked = loop_.budget_spent_usd().expect("cap is set");
    assert!(
        (tracked - booked).abs() < 1e-6,
        "tracked {tracked} booked {booked}"
    );
    // The last tool batch still got its results, so the transcript is valid.
    assert!(matches!(
        events[events.len() - 2],
        AgentLoopEvent::ToolBatchSummary { .. }
    ));
}

#[test]
fn dollar_cap_holds_across_prompts_and_resets_with_the_session() {
    let (loop_, calls, _temp) = endless_loop(
        "deepseek-v4-pro",
        AgentUsage::new(1_000_000, 100_000, 0, 0, 1),
    );
    let mut loop_ = loop_.with_max_budget_usd(1.0);

    let _ = loop_.run("first");
    let second = loop_.run("second");
    // The cap is per session, not per prompt: the second prompt is refused
    // before it issues a request.
    assert!(budget_stop(&second).is_some());
    assert_eq!(calls.load(Ordering::SeqCst), 3);

    loop_.reset();
    let _ = loop_.run("after reset");
    assert_eq!(calls.load(Ordering::SeqCst), 6);
}

#[test]
fn models_missing_from_the_builtin_table_are_priced_from_the_catalog() {
    // deeptide's built-in table only knows DeepSeek, so the cost tracker
    // records $0 for gpt-4o and a cap built on it would never trip.
    let (loop_, calls, _temp) =
        endless_loop("gpt-4o", AgentUsage::new(1_000_000, 100_000, 0, 0, 1));
    let mut loop_ = loop_.with_max_budget_usd(5.0);

    let events = loop_.run("go");

    assert_eq!(loop_.cost_tracker().summary().total_cost_usd, 0.0);
    assert!(budget_stop(&events).is_some(), "{:?}", events.last());
    assert!(calls.load(Ordering::SeqCst) < 200);
    assert!(loop_.budget_spent_usd().expect("cap is set") >= 5.0);
}

#[test]
fn unpriceable_model_under_a_cap_fails_closed() {
    let (loop_, calls, _temp) =
        endless_loop("in-house-7b-unlisted", AgentUsage::new(1_000, 100, 0, 0, 1));
    let mut loop_ = loop_.with_max_budget_usd(1.0);

    let events = loop_.run("go");

    let reason = budget_stop(&events).expect("stopped");
    assert!(reason.contains("no known price"), "{reason}");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn non_positive_cap_means_no_cap() {
    let (loop_, _calls, _temp) = endless_loop("deepseek-v4-pro", AgentUsage::new(10, 10, 0, 0, 1));
    let loop_ = loop_.with_max_budget_usd(0.0);
    assert_eq!(loop_.budget_spent_usd(), None);
}
