use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, ConversationMessage, EscalationPolicy, Route,
    RoutingBackend,
};

/// A backend that records how many times it was called and replays a fixed
/// result. The call counter is shared so a test can assert a backend was — or
/// was not — invoked after the router decided where to send a turn.
struct ScriptedBackend {
    label: &'static str,
    ok: bool,
    calls: Arc<AtomicUsize>,
}

impl ScriptedBackend {
    fn new(label: &'static str, ok: bool) -> (Self, Arc<AtomicUsize>) {
        let calls = Arc::new(AtomicUsize::new(0));
        (
            Self {
                label,
                ok,
                calls: Arc::clone(&calls),
            },
            calls,
        )
    }
}

impl AgentBackend for ScriptedBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.ok {
            Ok(AgentResponse::text(format!("{}-ok", self.label)))
        } else {
            Err(format!("{}-error", self.label))
        }
    }
}

fn request() -> AgentRequest {
    AgentRequest {
        messages: vec![ConversationMessage::user("hi")],
        model: "primary-model".to_owned(),
        step: 0,
        max_turns: 1,
        system: None,
        allowed_tools: None,
        thinking: None,
    }
}

#[test]
fn without_escalation_behaves_like_the_primary() {
    let (primary, primary_calls) = ScriptedBackend::new("primary", true);
    let mut router = RoutingBackend::new(Box::new(primary));

    let response = router.respond(request()).expect("primary should succeed");

    assert_eq!(response.content, "primary-ok");
    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(router.last_route(), Some(Route::Primary));
    assert!(!router.has_escalation());
}

#[test]
fn primary_error_surfaces_when_no_escalation() {
    let (primary, _) = ScriptedBackend::new("primary", false);
    let mut router = RoutingBackend::new(Box::new(primary));

    let error = router.respond(request()).expect_err("primary fails");

    assert_eq!(error, "primary-error");
    assert_eq!(router.last_route(), Some(Route::Primary));
}

#[test]
fn manual_policy_does_not_auto_escalate_on_error() {
    let (primary, primary_calls) = ScriptedBackend::new("primary", false);
    let (escalation, escalation_calls) = ScriptedBackend::new("cloud", true);
    let mut router = RoutingBackend::new(Box::new(primary)).with_escalation(Box::new(escalation));

    let error = router.respond(request()).expect_err("primary fails");

    // Manual is the default: a primary error is NOT silently escalated.
    assert_eq!(error, "primary-error");
    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(escalation_calls.load(Ordering::SeqCst), 0);
    assert_eq!(router.last_route(), Some(Route::Primary));
}

#[test]
fn arming_routes_one_turn_to_escalation_then_reverts() {
    let (primary, primary_calls) = ScriptedBackend::new("primary", true);
    let (escalation, escalation_calls) = ScriptedBackend::new("cloud", true);
    let mut router = RoutingBackend::new(Box::new(primary)).with_escalation(Box::new(escalation));

    assert!(
        router.arm(),
        "arming succeeds when an escalation target exists"
    );
    assert!(router.is_armed());

    // Armed turn goes to the cloud backend.
    let escalated = router.respond(request()).expect("escalation succeeds");
    assert_eq!(escalated.content, "cloud-ok");
    assert_eq!(router.last_route(), Some(Route::Escalated));
    assert!(!router.is_armed(), "arm is one-shot");

    // The next turn reverts to the primary.
    let primary_turn = router.respond(request()).expect("primary succeeds");
    assert_eq!(primary_turn.content, "primary-ok");
    assert_eq!(router.last_route(), Some(Route::Primary));

    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(escalation_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn arm_is_a_noop_without_an_escalation_target() {
    let (primary, _) = ScriptedBackend::new("primary", true);
    let mut router = RoutingBackend::new(Box::new(primary));

    assert!(
        !router.arm(),
        "arming reports failure when no cloud is configured"
    );
    assert!(!router.is_armed());

    // Falls through to the primary rather than erroring.
    let response = router.respond(request()).expect("primary succeeds");
    assert_eq!(response.content, "primary-ok");
    assert_eq!(router.last_route(), Some(Route::Primary));
}

#[test]
fn on_error_policy_escalates_when_primary_fails() {
    let (primary, primary_calls) = ScriptedBackend::new("primary", false);
    let (escalation, escalation_calls) = ScriptedBackend::new("cloud", true);
    let mut router = RoutingBackend::new(Box::new(primary))
        .with_escalation(Box::new(escalation))
        .with_policy(EscalationPolicy::OnError);

    let response = router
        .respond(request())
        .expect("escalation rescues the turn");

    assert_eq!(response.content, "cloud-ok");
    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(escalation_calls.load(Ordering::SeqCst), 1);
    assert_eq!(router.last_route(), Some(Route::Escalated));
}

#[test]
fn on_error_policy_keeps_primary_when_it_succeeds() {
    let (primary, primary_calls) = ScriptedBackend::new("primary", true);
    let (escalation, escalation_calls) = ScriptedBackend::new("cloud", true);
    let mut router = RoutingBackend::new(Box::new(primary))
        .with_escalation(Box::new(escalation))
        .with_policy(EscalationPolicy::OnError);

    let response = router.respond(request()).expect("primary succeeds");

    assert_eq!(response.content, "primary-ok");
    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        escalation_calls.load(Ordering::SeqCst),
        0,
        "cloud untouched"
    );
    assert_eq!(router.last_route(), Some(Route::Primary));
}

#[test]
fn on_error_double_failure_reports_both_errors() {
    let (primary, _) = ScriptedBackend::new("primary", false);
    let (escalation, _) = ScriptedBackend::new("cloud", false);
    let mut router = RoutingBackend::new(Box::new(primary))
        .with_escalation(Box::new(escalation))
        .with_policy(EscalationPolicy::OnError);

    let error = router.respond(request()).expect_err("both fail");

    assert!(
        error.contains("primary-error"),
        "mentions primary failure: {error}"
    );
    assert!(
        error.contains("cloud-error"),
        "mentions escalation failure: {error}"
    );
    assert_eq!(router.last_route(), Some(Route::Escalated));
}

#[test]
fn disarm_cancels_a_pending_escalation() {
    let (primary, primary_calls) = ScriptedBackend::new("primary", true);
    let (escalation, escalation_calls) = ScriptedBackend::new("cloud", true);
    let mut router = RoutingBackend::new(Box::new(primary)).with_escalation(Box::new(escalation));

    router.arm();
    router.disarm();
    assert!(!router.is_armed());

    let response = router.respond(request()).expect("primary succeeds");
    assert_eq!(response.content, "primary-ok");
    assert_eq!(primary_calls.load(Ordering::SeqCst), 1);
    assert_eq!(escalation_calls.load(Ordering::SeqCst), 0);
}
