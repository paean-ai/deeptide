use std::sync::Arc;

pub trait SlashCommand {
    fn name(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str];
    fn description(&self) -> &'static str;
    fn usage(&self) -> &'static str;
    fn execute(&self, args: &str, context: &CommandContext) -> CommandResult;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandResult {
    Text(String),
    Compact,
    Skip,
    Exit,
}

#[derive(Clone)]
pub struct CommandContext {
    reset_conversation: Arc<dyn Fn() + Send + Sync>,
    clear_conversation: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    compact_conversation: Arc<dyn Fn() + Send + Sync>,
    is_local_mode: Arc<dyn Fn() -> bool + Send + Sync>,
    local_warmup_estimate: Arc<dyn Fn() -> Option<String> + Send + Sync>,
    context_tokens: Arc<dyn Fn() -> usize + Send + Sync>,
    prime_local_cache_branch: Arc<dyn Fn() -> Option<String> + Send + Sync>,
}

impl Default for CommandContext {
    fn default() -> Self {
        Self {
            reset_conversation: Arc::new(|| {}),
            clear_conversation: Arc::new(|| None),
            compact_conversation: Arc::new(|| {}),
            is_local_mode: Arc::new(|| false),
            local_warmup_estimate: Arc::new(|| None),
            context_tokens: Arc::new(|| 0),
            prime_local_cache_branch: Arc::new(|| None),
        }
    }
}

impl CommandContext {
    pub fn builder() -> CommandContextBuilder {
        CommandContextBuilder::default()
    }

    pub fn reset_conversation(&self) {
        (self.reset_conversation)();
    }

    pub fn clear_conversation(&self) -> Option<String> {
        (self.clear_conversation)()
    }

    pub fn compact_conversation(&self) {
        (self.compact_conversation)();
    }

    pub fn is_local_mode(&self) -> bool {
        (self.is_local_mode)()
    }

    pub fn local_warmup_estimate(&self) -> Option<String> {
        (self.local_warmup_estimate)()
    }

    pub fn context_tokens(&self) -> usize {
        (self.context_tokens)()
    }

    pub fn prime_local_cache_branch(&self) -> Option<String> {
        (self.prime_local_cache_branch)()
    }
}

#[derive(Default)]
pub struct CommandContextBuilder {
    reset_conversation: Option<Arc<dyn Fn() + Send + Sync>>,
    clear_conversation: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
    compact_conversation: Option<Arc<dyn Fn() + Send + Sync>>,
    is_local_mode: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    local_warmup_estimate: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
    context_tokens: Option<Arc<dyn Fn() -> usize + Send + Sync>>,
    prime_local_cache_branch: Option<Arc<dyn Fn() -> Option<String> + Send + Sync>>,
}

impl CommandContextBuilder {
    pub fn reset_conversation<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.reset_conversation = Some(Arc::new(callback));
        self
    }

    pub fn clear_conversation<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> Option<String> + Send + Sync + 'static,
    {
        self.clear_conversation = Some(Arc::new(callback));
        self
    }

    pub fn compact_conversation<F>(mut self, callback: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.compact_conversation = Some(Arc::new(callback));
        self
    }

    pub fn is_local_mode<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.is_local_mode = Some(Arc::new(callback));
        self
    }

    pub fn local_warmup_estimate<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> Option<String> + Send + Sync + 'static,
    {
        self.local_warmup_estimate = Some(Arc::new(callback));
        self
    }

    pub fn context_tokens<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> usize + Send + Sync + 'static,
    {
        self.context_tokens = Some(Arc::new(callback));
        self
    }

    pub fn prime_local_cache_branch<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> Option<String> + Send + Sync + 'static,
    {
        self.prime_local_cache_branch = Some(Arc::new(callback));
        self
    }

    pub fn build(self) -> CommandContext {
        let defaults = CommandContext::default();
        CommandContext {
            reset_conversation: self
                .reset_conversation
                .unwrap_or_else(|| defaults.reset_conversation.clone()),
            clear_conversation: self
                .clear_conversation
                .unwrap_or_else(|| defaults.clear_conversation.clone()),
            compact_conversation: self
                .compact_conversation
                .unwrap_or_else(|| defaults.compact_conversation.clone()),
            is_local_mode: self
                .is_local_mode
                .unwrap_or_else(|| defaults.is_local_mode.clone()),
            local_warmup_estimate: self
                .local_warmup_estimate
                .unwrap_or_else(|| defaults.local_warmup_estimate.clone()),
            context_tokens: self
                .context_tokens
                .unwrap_or_else(|| defaults.context_tokens.clone()),
            prime_local_cache_branch: self
                .prime_local_cache_branch
                .unwrap_or_else(|| defaults.prime_local_cache_branch.clone()),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ClearCommand;

impl SlashCommand for ClearCommand {
    fn name(&self) -> &'static str {
        "clear"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["cls"]
    }

    fn description(&self) -> &'static str {
        "Clear conversation and reset context"
    }

    fn usage(&self) -> &'static str {
        "/clear [--yes]"
    }

    fn execute(&self, args: &str, context: &CommandContext) -> CommandResult {
        if let Some(message) = LocalBranchConfirmation::message_if_needed(
            "/clear",
            args,
            "start a new local cache branch and refill the prompt prefix on the next turn",
            context,
        ) {
            return CommandResult::Text(message);
        }

        if let Some(detail) = context.clear_conversation() {
            let suffix = if detail.is_empty() {
                String::new()
            } else {
                format!(" {detail}")
            };
            let primer = context.prime_local_cache_branch().unwrap_or_default();
            return CommandResult::Text(format!("Conversation cleared.{suffix}{primer}"));
        }

        context.reset_conversation();
        CommandResult::Text(String::from("Conversation cleared."))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct NewCommand;

impl SlashCommand for NewCommand {
    fn name(&self) -> &'static str {
        "new"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "Start a new conversation branch"
    }

    fn usage(&self) -> &'static str {
        "/new [--yes]"
    }

    fn execute(&self, args: &str, context: &CommandContext) -> CommandResult {
        if let Some(message) = LocalBranchConfirmation::message_if_needed(
            "/new",
            args,
            "start a new local cache branch and refill the prompt prefix on the next turn",
            context,
        ) {
            return CommandResult::Text(message);
        }

        if let Some(detail) = context.clear_conversation() {
            let suffix = if detail.is_empty() {
                String::new()
            } else {
                format!(" {detail}")
            };
            let primer = context.prime_local_cache_branch().unwrap_or_default();
            return CommandResult::Text(format!("New conversation started.{suffix}{primer}"));
        }

        context.reset_conversation();
        CommandResult::Text(String::from("New conversation started."))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CompactCommand;

impl SlashCommand for CompactCommand {
    fn name(&self) -> &'static str {
        "compact"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["compress"]
    }

    fn description(&self) -> &'static str {
        "Manually compact the context window"
    }

    fn usage(&self) -> &'static str {
        "/compact [--yes]"
    }

    fn execute(&self, args: &str, context: &CommandContext) -> CommandResult {
        if let Some(message) = LocalBranchConfirmation::message_if_needed(
            "/compact",
            args,
            "rewrite the local prompt branch and refill the compacted prefix",
            context,
        ) {
            return CommandResult::Text(message);
        }

        context.compact_conversation();
        CommandResult::Text(String::from("Context compacted."))
    }
}

struct LocalBranchConfirmation;

impl LocalBranchConfirmation {
    fn message_if_needed(
        command: &str,
        args: &str,
        action: &str,
        context: &CommandContext,
    ) -> Option<String> {
        if !context.is_local_mode() || has_yes_flag(args) {
            return None;
        }

        let mut lines = vec![
            String::from("Local mode cache branch change requires confirmation."),
            format!("{command} will {action}."),
        ];

        if let Some(estimate) = context.local_warmup_estimate() {
            if !estimate.is_empty() {
                lines.push(format!("Recent refill/warmup estimate: {estimate}."));
            }
        } else if context.context_tokens() > 0 {
            lines.push(String::from(
                "The next turn may need to refill the current prompt prefix.",
            ));
        }

        lines.push(format!("Run `{command} --yes` to continue."));
        Some(lines.join("\n"))
    }
}

fn has_yes_flag(args: &str) -> bool {
    args.split_whitespace()
        .any(|flag| matches!(flag, "--yes" | "-y"))
}
