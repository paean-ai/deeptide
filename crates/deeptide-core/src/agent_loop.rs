use crate::{
    CostTracker, PermissionDecision, PermissionManager, PermissionMode, PermissionRules,
    ToolBatchFailureClassifier, ToolBatchItem, ToolBatchLabeler, ToolContext, ToolRegistry,
    TurnUsage,
};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: MessageRole,
    pub content: String,
}

impl ConversationMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequest {
    pub messages: Vec<ConversationMessage>,
    pub model: String,
    pub step: usize,
    pub max_turns: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub cache_create: usize,
    pub cache_read: usize,
    pub duration_ms: usize,
}

impl AgentUsage {
    pub const fn new(
        input_tokens: usize,
        output_tokens: usize,
        cache_create: usize,
        cache_read: usize,
        duration_ms: usize,
    ) -> Self {
        Self {
            input_tokens,
            output_tokens,
            cache_create,
            cache_read,
            duration_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResponse {
    pub content: String,
    pub usage: Option<AgentUsage>,
    pub tool_calls: Vec<ToolCall>,
}

impl AgentResponse {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            usage: None,
            tool_calls: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            input,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentLoopEvent {
    User(ConversationMessage),
    Assistant(ConversationMessage),
    ToolBatchSummary {
        label: String,
        tool_calls: Vec<ToolCall>,
        failed_count: usize,
    },
    ToolResult {
        tool_call: ToolCall,
        content: String,
        is_error: bool,
    },
    Terminal(AgentTerminalEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTerminalEvent {
    Complete,
    MaxTurnsReached,
    ModelError(String),
}

pub trait AgentBackend: Send {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String>;
}

pub type SubAgentBackendFactory = Arc<dyn Fn(&str) -> Box<dyn AgentBackend> + Send + Sync>;

pub struct AgentLoop {
    backend: Box<dyn AgentBackend>,
    messages: Vec<ConversationMessage>,
    cost_tracker: CostTracker,
    model: String,
    max_turns: usize,
    current_run_step: usize,
    tool_registry: ToolRegistry,
    tool_context: ToolContext,
    permission_manager: PermissionManager,
    subagent_backend_factory: Option<SubAgentBackendFactory>,
}

impl AgentLoop {
    pub fn new(backend: Box<dyn AgentBackend>) -> Self {
        let rules = PermissionRules::load(None).unwrap_or_else(|_| PermissionRules::in_memory());
        Self {
            backend,
            messages: Vec::new(),
            cost_tracker: CostTracker::new(),
            model: String::from("unconfigured"),
            max_turns: 25,
            current_run_step: 0,
            tool_registry: ToolRegistry::with_builtin_tools(),
            tool_context: ToolContext::new(
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            ),
            permission_manager: PermissionManager::new(PermissionMode::Default, rules),
            subagent_backend_factory: None,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns.max(1);
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<std::path::PathBuf>) -> Self {
        self.tool_context = ToolContext::new(cwd);
        self
    }

    pub fn with_permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_manager.set_mode(mode);
        self
    }

    pub fn with_permission_manager(mut self, permission_manager: PermissionManager) -> Self {
        self.permission_manager = permission_manager;
        self
    }

    pub fn with_subagent_backend_factory<F>(mut self, factory: F) -> Self
    where
        F: Fn(&str) -> Box<dyn AgentBackend> + Send + Sync + 'static,
    {
        self.subagent_backend_factory = Some(Arc::new(factory));
        self
    }

    pub fn run(&mut self, user_input: impl Into<String>) -> Vec<AgentLoopEvent> {
        let user_message = ConversationMessage::user(user_input);
        self.current_run_step = 0;
        self.messages.push(user_message.clone());

        let mut events = vec![AgentLoopEvent::User(user_message)];

        loop {
            if self.current_run_step >= self.max_turns {
                events.push(AgentLoopEvent::Terminal(
                    AgentTerminalEvent::MaxTurnsReached,
                ));
                return events;
            }

            self.current_run_step += 1;
            let request = AgentRequest {
                messages: self.messages.clone(),
                model: self.model.clone(),
                step: self.current_run_step,
                max_turns: self.max_turns,
            };

            match self.backend.respond(request) {
                Ok(response) => {
                    if let Some(usage) = response.usage {
                        self.cost_tracker.record(TurnUsage::new(
                            self.current_run_step,
                            self.model.clone(),
                            usage.input_tokens,
                            usage.output_tokens,
                            usage.cache_create,
                            usage.cache_read,
                            usage.duration_ms,
                        ));
                    }

                    let assistant_message = ConversationMessage::assistant(response.content);
                    self.messages.push(assistant_message.clone());
                    events.push(AgentLoopEvent::Assistant(assistant_message));

                    if response.tool_calls.is_empty() {
                        events.push(AgentLoopEvent::Terminal(AgentTerminalEvent::Complete));
                        return events;
                    }

                    let mut tool_results = Vec::with_capacity(response.tool_calls.len());
                    let mut failure_summaries = Vec::new();
                    for tool_call in response.tool_calls {
                        let result = self.execute_tool_call(&tool_call);
                        let content = result.content;
                        let is_error = result.is_error;
                        if is_error {
                            failure_summaries
                                .push(ToolBatchFailureClassifier::classify(&content, is_error));
                        }
                        tool_results.push((tool_call, content, is_error));
                    }

                    let tool_calls = tool_results
                        .iter()
                        .map(|(tool_call, _, _)| tool_call.clone())
                        .collect::<Vec<_>>();
                    let label_items = tool_calls
                        .iter()
                        .map(|tool_call| {
                            ToolBatchItem::new(tool_call.name.clone(), tool_call.input.clone())
                        })
                        .collect::<Vec<_>>();
                    let failed_count = tool_results
                        .iter()
                        .filter(|(_, _, is_error)| *is_error)
                        .count();
                    events.push(AgentLoopEvent::ToolBatchSummary {
                        label: ToolBatchLabeler::label_with_failure_summaries(
                            &label_items,
                            &failure_summaries,
                        ),
                        tool_calls,
                        failed_count,
                    });

                    for (tool_call, content, is_error) in tool_results {
                        events.push(AgentLoopEvent::ToolResult {
                            tool_call: tool_call.clone(),
                            content: content.clone(),
                            is_error,
                        });
                        self.messages.push(ConversationMessage::user(format!(
                            "[tool_result id={} name={} is_error={}]\n{}",
                            tool_call.id, tool_call.name, is_error, content
                        )));
                    }
                }
                Err(error) => {
                    events.push(AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(
                        error,
                    )));
                    return events;
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.messages.clear();
        self.cost_tracker.reset();
        self.current_run_step = 0;
    }

    pub fn messages(&self) -> &[ConversationMessage] {
        &self.messages
    }

    pub fn cost_tracker(&self) -> &CostTracker {
        &self.cost_tracker
    }

    pub fn current_run_step(&self) -> usize {
        self.current_run_step
    }

    pub fn max_turns(&self) -> usize {
        self.max_turns
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    fn execute_tool_call(&mut self, tool_call: &ToolCall) -> crate::ToolResult {
        let permission = tool_call
            .input
            .as_object()
            .map(|input| self.permission_manager.check_json(&tool_call.name, input))
            .unwrap_or_else(|| {
                self.permission_manager
                    .check(&tool_call.name, &crate::permissions::ToolInput::default())
            });

        match permission {
            PermissionDecision::Allow => {
                if tool_call.name == "Agent"
                    && let Some(result) = self.execute_subagent_tool_call(tool_call)
                {
                    return result;
                }
                self.tool_registry.call(
                    &tool_call.name,
                    tool_call.input.clone(),
                    &self.tool_context,
                )
            }
            PermissionDecision::Deny { reason } => crate::ToolResult::error(format!(
                "Permission denied for {}: {reason}",
                tool_call.name
            )),
            PermissionDecision::Ask => crate::ToolResult::error(format!(
                "Permission required for {}. Re-run with --permission-mode accept-edits or add an allow rule to approve this tool call.",
                tool_call.name
            )),
        }
    }

    fn execute_subagent_tool_call(&self, tool_call: &ToolCall) -> Option<crate::ToolResult> {
        let factory = self.subagent_backend_factory.as_ref()?;
        let invocation = match crate::tools::parse_agent_invocation(&tool_call.input) {
            Ok(invocation) => invocation,
            Err(error) => return Some(crate::ToolResult::error(error)),
        };

        let subagent_model = invocation
            .model
            .as_deref()
            .unwrap_or(self.model.as_str())
            .to_owned();
        let permission_mode = if invocation.definition.is_read_only {
            PermissionMode::Plan
        } else {
            self.permission_manager.mode()
        };
        let mut subagent = AgentLoop::new(factory(&subagent_model))
            .with_model(subagent_model.clone())
            .with_max_turns(invocation.definition.max_turns)
            .with_cwd(self.tool_context.cwd.clone())
            .with_permission_mode(permission_mode);
        let events = subagent.run(format!(
            "Sub-agent task: {}\n\n{}",
            invocation.description, invocation.prompt
        ));

        Some(render_subagent_result(
            invocation.definition.kind,
            &subagent_model,
            &events,
        ))
    }
}

fn render_subagent_result(kind: &str, model: &str, events: &[AgentLoopEvent]) -> crate::ToolResult {
    if let Some(AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(error))) = events.last() {
        return crate::ToolResult::error(format!("Sub-agent {kind} failed: {error}"));
    }

    let mut lines = vec![format!("Sub-agent {kind} completed with model {model}.")];
    for event in events {
        match event {
            AgentLoopEvent::Assistant(message) if !message.content.trim().is_empty() => {
                lines.push(String::new());
                lines.push(message.content.trim().to_owned());
            }
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error,
            } => {
                let status = if *is_error { "error" } else { "ok" };
                lines.push(String::new());
                lines.push(format!(
                    "[tool_result name={} status={}]\n{}",
                    tool_call.name,
                    status,
                    content.trim()
                ));
            }
            AgentLoopEvent::Terminal(AgentTerminalEvent::MaxTurnsReached) => {
                lines.push(String::new());
                lines.push(String::from(
                    "Sub-agent stopped after reaching the turn limit.",
                ));
            }
            _ => {}
        }
    }

    crate::ToolResult::text(lines.join("\n"))
}

#[derive(Debug, Default)]
pub struct LocalEchoBackend;

impl AgentBackend for LocalEchoBackend {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String> {
        let prompt = request
            .messages
            .iter()
            .rev()
            .find(|message| message.role == MessageRole::User)
            .map(|message| message.content.as_str())
            .unwrap_or_default();
        let input_tokens = prompt.split_whitespace().count().max(1);
        let content = format!(
            "Agent loop is running locally, but no model backend is configured yet.\n\nPrompt received:\n{prompt}"
        );

        Ok(AgentResponse {
            usage: Some(AgentUsage::new(
                input_tokens,
                content.split_whitespace().count(),
                0,
                0,
                0,
            )),
            content,
            tool_calls: Vec::new(),
        })
    }
}
