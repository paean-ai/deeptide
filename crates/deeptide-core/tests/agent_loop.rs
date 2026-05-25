use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentRequest, AgentResponse, AgentTerminalEvent,
    AgentUsage, MessageRole,
};

#[test]
fn agent_loop_appends_user_and_assistant_messages() {
    let mut loop_ = AgentLoop::new(Box::new(StaticBackend::new("hello back")));

    let events = loop_.run("hello");

    assert!(matches!(events[0], AgentLoopEvent::User(_)));
    assert!(matches!(events[1], AgentLoopEvent::Assistant(_)));
    assert_eq!(
        events.last(),
        Some(&AgentLoopEvent::Terminal(AgentTerminalEvent::Complete))
    );
    assert_eq!(loop_.messages().len(), 2);
    assert_eq!(loop_.messages()[0].role, MessageRole::User);
    assert_eq!(loop_.messages()[1].content, "hello back");
}

#[test]
fn agent_loop_records_usage_in_cost_tracker() {
    let mut loop_ = AgentLoop::new(Box::new(
        StaticBackend::new("hello back").with_usage(AgentUsage::new(10, 5, 2, 8, 123)),
    ))
    .with_model("deepseek-v4-pro");

    let _ = loop_.run("hello");
    let summary = loop_.cost_tracker().summary();

    assert_eq!(summary.turns.len(), 1);
    assert_eq!(summary.total_input, 10);
    assert_eq!(summary.total_output, 5);
    assert_eq!(summary.total_cache_create, 2);
    assert_eq!(summary.total_cache_read, 8);
    assert_eq!(summary.turns[0].duration_ms, 123);
    assert_eq!(summary.turns[0].model, "deepseek-v4-pro");
}

#[test]
fn agent_loop_reports_backend_errors_without_adding_assistant_message() {
    let mut loop_ = AgentLoop::new(Box::new(FailingBackend));

    let events = loop_.run("hello");

    assert_eq!(
        events.last(),
        Some(&AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(
            "backend failed".to_owned()
        )))
    );
    assert_eq!(loop_.messages().len(), 1);
}

struct StaticBackend {
    content: String,
    usage: Option<AgentUsage>,
}

impl StaticBackend {
    fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            usage: None,
        }
    }

    fn with_usage(mut self, usage: AgentUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

impl AgentBackend for StaticBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: self.content.clone(),
            usage: self.usage.clone(),
        })
    }
}

struct FailingBackend;

impl AgentBackend for FailingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Err(String::from("backend failed"))
    }
}
