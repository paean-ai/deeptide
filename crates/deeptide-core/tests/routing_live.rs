//! Live routing tests against the real DeepSeek cloud API.
//!
//! These are `#[ignore]`d so the default `cargo test` / CI run never touches
//! the network or needs a key. Run them explicitly with a key in the
//! environment:
//!
//! ```sh
//! DEEPSEEK_API_KEY=sk-... cargo test -p deeptide-core --test routing_live -- --ignored
//! ```
//!
//! They exercise the full path that matters for the local→cloud story:
//! `RoutingBackend` → `AnthropicBackend` → DeepSeek's Anthropic-compatible
//! endpoint → real response parsing.

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AnthropicConfig, ConversationMessage,
    EscalationPolicy, Route, RoutingBackend,
};

/// DeepSeek's Anthropic-compatible base; `/v1/messages` is appended by the backend.
const DEEPSEEK_ANTHROPIC_BASE: &str = "https://api.deepseek.com/anthropic";
const DEEPSEEK_MODEL: &str = "deepseek-chat";

/// A primary backend that always fails, standing in for a local runtime that
/// errored — so the router must fall through to the real cloud backend.
struct AlwaysFails;

impl AgentBackend for AlwaysFails {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Err(String::from("local runtime unavailable"))
    }
}

/// A primary backend that answers locally, so an armed turn has something
/// distinct to bypass.
struct LocalEcho;

impl AgentBackend for LocalEcho {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse::text("LOCAL_ANSWER"))
    }
}

fn cloud_backend() -> Box<dyn AgentBackend> {
    let key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY must be set");
    let mut config = AnthropicConfig::new(DEEPSEEK_ANTHROPIC_BASE, key, DEEPSEEK_MODEL);
    // No tools/system in these probes, so caching markers wouldn't attach
    // anyway; disable explicitly to keep the wire body minimal and portable.
    config.enable_prompt_caching = false;
    config.max_tokens = 64;
    Box::new(AnthropicBackendForTest::new(config))
}

/// Thin wrapper so we don't depend on the exact `AnthropicBackend::new` return
/// shape changing — it returns `Result`, which we unwrap here in test code.
struct AnthropicBackendForTest(deeptide_core::AnthropicBackend);

impl AnthropicBackendForTest {
    fn new(config: AnthropicConfig) -> Self {
        Self(deeptide_core::AnthropicBackend::new(config).expect("build AnthropicBackend"))
    }
}

impl AgentBackend for AnthropicBackendForTest {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String> {
        self.0.respond(request)
    }
}

fn ask(prompt: &str) -> AgentRequest {
    AgentRequest {
        messages: vec![ConversationMessage::user(prompt)],
        model: DEEPSEEK_MODEL.to_owned(),
        step: 0,
        max_turns: 1,
        system: None,
        allowed_tools: None,
    }
}

#[test]
#[ignore = "live network + DEEPSEEK_API_KEY"]
fn on_error_escalates_to_real_deepseek() {
    let mut router = RoutingBackend::new(Box::new(AlwaysFails))
        .with_escalation(cloud_backend())
        .with_policy(EscalationPolicy::OnError);

    let response = router
        .respond(ask("Reply with exactly: ROUTING_OK"))
        .expect("cloud should rescue the failed local turn");

    assert_eq!(router.last_route(), Some(Route::Escalated));
    assert!(
        response.content.contains("ROUTING_OK"),
        "unexpected cloud reply: {:?}",
        response.content
    );
}

#[test]
#[ignore = "live network + DEEPSEEK_API_KEY"]
fn arm_routes_directly_to_real_deepseek() {
    let mut router = RoutingBackend::new(Box::new(LocalEcho)).with_escalation(cloud_backend());

    assert!(router.arm());
    let response = router
        .respond(ask("Reply with exactly: ROUTING_OK"))
        .expect("armed turn should reach cloud");

    assert_eq!(router.last_route(), Some(Route::Escalated));
    assert!(
        response.content.contains("ROUTING_OK"),
        "armed turn did not reach cloud, got: {:?}",
        response.content
    );

    // The next, un-armed turn falls back to the local primary.
    let local = router
        .respond(ask("anything"))
        .expect("local primary answers");
    assert_eq!(router.last_route(), Some(Route::Primary));
    assert_eq!(local.content, "LOCAL_ANSWER");
}
