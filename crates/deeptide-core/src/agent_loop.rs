use crate::{CostTracker, TurnUsage};

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
}

impl AgentResponse {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            usage: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentLoopEvent {
    User(ConversationMessage),
    Assistant(ConversationMessage),
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

pub struct AgentLoop {
    backend: Box<dyn AgentBackend>,
    messages: Vec<ConversationMessage>,
    cost_tracker: CostTracker,
    model: String,
    max_turns: usize,
    current_run_step: usize,
}

impl AgentLoop {
    pub fn new(backend: Box<dyn AgentBackend>) -> Self {
        Self {
            backend,
            messages: Vec::new(),
            cost_tracker: CostTracker::new(),
            model: String::from("unconfigured"),
            max_turns: 25,
            current_run_step: 0,
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

    pub fn run(&mut self, user_input: impl Into<String>) -> Vec<AgentLoopEvent> {
        let user_message = ConversationMessage::user(user_input);
        self.current_run_step = 0;
        self.messages.push(user_message.clone());

        let mut events = vec![AgentLoopEvent::User(user_message)];
        if self.messages.len() / 2 >= self.max_turns {
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
                events.push(AgentLoopEvent::Terminal(AgentTerminalEvent::Complete));
            }
            Err(error) => {
                events.push(AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(
                    error,
                )));
            }
        }

        events
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
        })
    }
}
