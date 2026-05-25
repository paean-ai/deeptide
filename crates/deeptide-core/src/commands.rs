use std::sync::Arc;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::memory::{
    MemoryResolveResult, MemoryScope, delete_memory, list_memory_blocks, remember_project_note,
    render_memory, resolve_memory,
};

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
    cwd: Arc<dyn Fn() -> std::path::PathBuf + Send + Sync>,
    now_rfc3339: Arc<dyn Fn() -> String + Send + Sync>,
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
            cwd: Arc::new(|| {
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
            }),
            now_rfc3339: Arc::new(|| {
                OffsetDateTime::now_utc()
                    .format(&Rfc3339)
                    .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
            }),
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

    pub fn cwd(&self) -> std::path::PathBuf {
        (self.cwd)()
    }

    pub fn now_rfc3339(&self) -> String {
        (self.now_rfc3339)()
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
    cwd: Option<Arc<dyn Fn() -> std::path::PathBuf + Send + Sync>>,
    now_rfc3339: Option<Arc<dyn Fn() -> String + Send + Sync>>,
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

    pub fn cwd<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> std::path::PathBuf + Send + Sync + 'static,
    {
        self.cwd = Some(Arc::new(callback));
        self
    }

    pub fn now_rfc3339<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.now_rfc3339 = Some(Arc::new(callback));
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
            cwd: self.cwd.unwrap_or_else(|| defaults.cwd.clone()),
            now_rfc3339: self
                .now_rfc3339
                .unwrap_or_else(|| defaults.now_rfc3339.clone()),
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

#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryCommand;

impl SlashCommand for MemoryCommand {
    fn name(&self) -> &'static str {
        "memory"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["mem"]
    }

    fn description(&self) -> &'static str {
        "Show, inspect, and prune memory files"
    }

    fn usage(&self) -> &'static str {
        "/memory [show|delete <project|global> <file-or-title>]"
    }

    fn execute(&self, args: &str, context: &CommandContext) -> CommandResult {
        let cwd = context.cwd();
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return CommandResult::Text(render_memory_list(&cwd));
        }

        let parts = trimmed.splitn(3, char::is_whitespace).collect::<Vec<_>>();
        let Some(subcommand) = parts.first().map(|part| part.to_ascii_lowercase()) else {
            return CommandResult::Text(format!("Usage: {}", self.usage()));
        };

        match subcommand.as_str() {
            "show" | "cat" | "open" => {
                let (scope, target) = parse_memory_target(&parts);
                let Some(target) = target else {
                    return CommandResult::Text(String::from(
                        "Usage: /memory show [project|global] <file-or-title>",
                    ));
                };

                match resolve_memory(&target, scope, &cwd) {
                    MemoryResolveResult::Found(entry) => CommandResult::Text(render_memory(&entry)),
                    MemoryResolveResult::Missing(message)
                    | MemoryResolveResult::Ambiguous(message) => CommandResult::Text(message),
                }
            }
            "delete" | "rm" | "remove" | "forget" => {
                let (scope, target) = parse_memory_target(&parts);
                let Some(target) = target else {
                    return CommandResult::Text(String::from(
                        "Usage: /memory delete [project|global] <file-or-title>",
                    ));
                };

                match resolve_memory(&target, scope, &cwd) {
                    MemoryResolveResult::Found(entry) => {
                        let file_name = entry
                            .file_path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .unwrap_or_default()
                            .to_owned();
                        let scope = entry.scope;
                        match delete_memory(&entry) {
                            Ok(()) => {
                                CommandResult::Text(format!("Deleted {scope} memory: {file_name}"))
                            }
                            Err(error) => CommandResult::Text(format!("Failed: {error}")),
                        }
                    }
                    MemoryResolveResult::Missing(message)
                    | MemoryResolveResult::Ambiguous(message) => CommandResult::Text(message),
                }
            }
            _ => CommandResult::Text(format!("Usage: {}", self.usage())),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RememberCommand;

impl SlashCommand for RememberCommand {
    fn name(&self) -> &'static str {
        "remember"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "Save a note to project memory"
    }

    fn usage(&self) -> &'static str {
        "/remember <text>"
    }

    fn execute(&self, args: &str, context: &CommandContext) -> CommandResult {
        let body = args.trim();
        if body.is_empty() {
            return CommandResult::Text(format!("Usage: {}", self.usage()));
        }

        match remember_project_note(body, context.cwd(), &context.now_rfc3339()) {
            Ok(path) => CommandResult::Text(format!("Saved to {}", path.display())),
            Err(error) => CommandResult::Text(format!("Failed: {error}")),
        }
    }
}

fn render_memory_list(cwd: &std::path::Path) -> String {
    let mut lines = vec![String::from("Memory files:")];

    for (index, block) in list_memory_blocks(cwd).into_iter().enumerate() {
        if index > 0 {
            lines.push(String::new());
        }

        let status = if block.exists { "*" } else { "o" };
        let created = if block.exists { "" } else { ", not created" };
        lines.push(format!(
            "  {status} {} ({}{created})",
            block.index_path.display(),
            block.label
        ));

        if block.exists {
            if block.files.is_empty() {
                lines.push(String::from("    (no entries yet - index alone)"));
            } else {
                for file in block.files {
                    let description = file
                        .description
                        .map(|description| format!(" - {description}"))
                        .unwrap_or_default();
                    lines.push(format!("    - {}{description}", file.name));
                }
            }
        }
    }

    lines.push(String::new());
    lines.push(String::from("/remember <text> saves a new memory."));
    lines.push(String::from(
        "/memory show <file> inspects one memory; /memory delete <file> prunes it.",
    ));
    lines.join("\n")
}

fn parse_memory_target(parts: &[&str]) -> (Option<MemoryScope>, Option<String>) {
    if parts.len() < 2 {
        return (None, None);
    }

    if parts.len() >= 3
        && let Some(scope) = MemoryScope::parse(parts[1])
    {
        return (Some(scope), Some(parts[2].trim().to_owned()));
    }

    let target = parts[1..].join(" ").trim().to_owned();
    if target.is_empty() {
        (None, None)
    } else {
        (None, Some(target))
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
