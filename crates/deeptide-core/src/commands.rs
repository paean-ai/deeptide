use std::sync::Arc;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

use crate::completion::CommandCompletionSource;
use crate::cost::{CostSummary, CostTracker, TurnRecord};
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
    all_commands: Arc<dyn Fn() -> Vec<CommandCompletionSource> + Send + Sync>,
    cost_summary: Arc<dyn Fn() -> CostSummary + Send + Sync>,
    cost_display_enabled: Arc<dyn Fn() -> bool + Send + Sync>,
    set_cost_display_enabled: Arc<dyn Fn(bool) + Send + Sync>,
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
            all_commands: Arc::new(builtin_command_sources),
            cost_summary: Arc::new(CostSummary::default),
            cost_display_enabled: Arc::new(|| false),
            set_cost_display_enabled: Arc::new(|_| {}),
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

    pub fn all_commands(&self) -> Vec<CommandCompletionSource> {
        (self.all_commands)()
    }

    pub fn cost_summary(&self) -> CostSummary {
        (self.cost_summary)()
    }

    pub fn cost_display_enabled(&self) -> bool {
        (self.cost_display_enabled)()
    }

    pub fn set_cost_display_enabled(&self, enabled: bool) {
        (self.set_cost_display_enabled)(enabled);
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
    all_commands: Option<Arc<dyn Fn() -> Vec<CommandCompletionSource> + Send + Sync>>,
    cost_summary: Option<Arc<dyn Fn() -> CostSummary + Send + Sync>>,
    cost_display_enabled: Option<Arc<dyn Fn() -> bool + Send + Sync>>,
    set_cost_display_enabled: Option<Arc<dyn Fn(bool) + Send + Sync>>,
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

    pub fn all_commands<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> Vec<CommandCompletionSource> + Send + Sync + 'static,
    {
        self.all_commands = Some(Arc::new(callback));
        self
    }

    pub fn cost_summary<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> CostSummary + Send + Sync + 'static,
    {
        self.cost_summary = Some(Arc::new(callback));
        self
    }

    pub fn cost_display_enabled<F>(mut self, callback: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.cost_display_enabled = Some(Arc::new(callback));
        self
    }

    pub fn set_cost_display_enabled<F>(mut self, callback: F) -> Self
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        self.set_cost_display_enabled = Some(Arc::new(callback));
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
            all_commands: self
                .all_commands
                .unwrap_or_else(|| defaults.all_commands.clone()),
            cost_summary: self
                .cost_summary
                .unwrap_or_else(|| defaults.cost_summary.clone()),
            cost_display_enabled: self
                .cost_display_enabled
                .unwrap_or_else(|| defaults.cost_display_enabled.clone()),
            set_cost_display_enabled: self
                .set_cost_display_enabled
                .unwrap_or_else(|| defaults.set_cost_display_enabled.clone()),
            cwd: self.cwd.unwrap_or_else(|| defaults.cwd.clone()),
            now_rfc3339: self
                .now_rfc3339
                .unwrap_or_else(|| defaults.now_rfc3339.clone()),
        }
    }
}

/// Exit the REPL.
///
/// Implemented as a first-class `SlashCommand` so it appears in `/help`,
/// in tab-completion candidates, and survives a future refactor where
/// the dispatch switches from hand-rolled string matches to a typed
/// registry. Aliases match the long-standing dispatch:
///
///   /exit, /quit, /q   →   CommandResult::Exit
///
/// The REPL's `execute_command` still short-circuits on the same set of
/// names *before* this struct is dispatched — that ordering is intentional
/// because exiting needs to drive `finalize_session()` (history save +
/// goodbye message), which only the REPL itself can do. The struct is
/// here so the command is *discoverable* and *unit-testable* in pure
/// `deeptide-core` without spinning up a full REPL.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExitCommand;

impl SlashCommand for ExitCommand {
    fn name(&self) -> &'static str {
        "exit"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["quit", "q"]
    }

    fn description(&self) -> &'static str {
        "Exit the REPL"
    }

    fn usage(&self) -> &'static str {
        "/exit"
    }

    fn execute(&self, _args: &str, _context: &CommandContext) -> CommandResult {
        CommandResult::Exit
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HelpCommand;

impl SlashCommand for HelpCommand {
    fn name(&self) -> &'static str {
        "help"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["h", "?"]
    }

    fn description(&self) -> &'static str {
        "Show available commands and keybindings"
    }

    fn usage(&self) -> &'static str {
        "/help [command]"
    }

    fn execute(&self, args: &str, context: &CommandContext) -> CommandResult {
        let registered = context.all_commands();
        let needle = args.trim().trim_start_matches('/').to_ascii_lowercase();

        if !needle.is_empty() {
            if let Some(command) = find_command(&needle, &registered) {
                return CommandResult::Text(render_help_detail(command));
            }
            return CommandResult::Text(render_unknown_command(&needle, &registered));
        }

        CommandResult::Text(render_help_index(&registered))
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

#[derive(Debug, Default, Clone, Copy)]
pub struct CostCommand;

impl SlashCommand for CostCommand {
    fn name(&self) -> &'static str {
        "cost"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn description(&self) -> &'static str {
        "Detailed token/cost breakdown; use /cost show to display estimates in the status bar"
    }

    fn usage(&self) -> &'static str {
        "/cost [show | hide | toggle]"
    }

    fn execute(&self, args: &str, context: &CommandContext) -> CommandResult {
        match args.trim().to_ascii_lowercase().as_str() {
            "show" | "on" | "enable" => {
                context.set_cost_display_enabled(true);
                return CommandResult::Text(String::from(
                    "Estimated cost display enabled for this session. Values depend on provider pricing and cache billing semantics.",
                ));
            }
            "hide" | "off" | "disable" => {
                context.set_cost_display_enabled(false);
                return CommandResult::Text(String::from(
                    "Estimated cost display hidden. Token in/out and cache stats remain available.",
                ));
            }
            "toggle" => {
                let next = !context.cost_display_enabled();
                context.set_cost_display_enabled(next);
                let state = if next { "enabled" } else { "hidden" };
                return CommandResult::Text(format!("Estimated cost display {state}."));
            }
            "" => {}
            _ => return CommandResult::Text(format!("Usage: {}", self.usage())),
        }

        let summary = context.cost_summary();
        if summary.turns.is_empty() {
            return CommandResult::Text(String::from("No turns recorded yet."));
        }

        CommandResult::Text(render_cost_breakdown(&summary))
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

fn builtin_command_sources() -> Vec<CommandCompletionSource> {
    vec![
        CommandCompletionSource::from_command(&HelpCommand),
        CommandCompletionSource::from_command(&ExitCommand),
        CommandCompletionSource::from_command(&ClearCommand),
        CommandCompletionSource::from_command(&NewCommand),
        CommandCompletionSource::from_command(&CompactCommand),
        CommandCompletionSource::from_command(&CostCommand),
        CommandCompletionSource::from_command(&MemoryCommand),
        CommandCompletionSource::from_command(&RememberCommand),
    ]
}

fn render_cost_breakdown(summary: &CostSummary) -> String {
    let mut lines = vec![
        String::from("Cost breakdown"),
        String::from("  turn   in       out     cache+    cache-r  cost     ms"),
    ];

    for turn in &summary.turns {
        lines.push(format!(
            "  {:<5}  {:<7}  {:<6}  {:<8}  {:<7}  {:<7}  {}",
            turn.turn,
            turn.input_tokens,
            turn.output_tokens,
            turn.cache_create,
            turn.cache_read,
            CostTracker::format_usd(turn.cost_usd),
            turn.duration_ms
        ));
    }

    lines.push(String::new());
    lines.push(format!(
        "  Total: {} ({} in, {} out)",
        CostTracker::format_usd(summary.total_cost_usd),
        CostTracker::format_tokens(summary.total_input),
        CostTracker::format_tokens(summary.total_output)
    ));
    lines.push(String::from(
        "  Estimate only: provider pricing and cache billing can differ. Use /cost show to display this estimate in the status bar.",
    ));

    let cache = summary.cache_health();
    lines.push(String::new());
    if let Some(hit_rate) = cache.hit_rate_percent {
        let recent = cache
            .recent_hit_rate_percent
            .map(|recent| format!(", recent {recent}%"))
            .unwrap_or_default();
        lines.push(format!(
            "  Cache health: {hit_rate}% hit{recent} · {} ({created} created, {read} read)",
            cache.label(),
            created = CostTracker::format_tokens(cache.total_create_tokens),
            read = CostTracker::format_tokens(cache.total_read_tokens)
        ));
    } else {
        lines.push(format!(
            "  Cache health: {} · no cache telemetry yet",
            cache.label()
        ));
    }

    if let Some(diagnostic) = cache.diagnostic() {
        lines.push(format!("  Cache note: {diagnostic}"));
    }

    let by_model = group_turns_by_model(&summary.turns);
    if by_model.len() > 1 {
        lines.push(String::new());
        lines.push(String::from("By model:"));
        for model in by_model {
            lines.push(render_model_rollup(&model));
        }
    }

    lines.join("\n")
}

#[derive(Debug)]
struct ModelRollup<'a> {
    model: &'a str,
    turns: Vec<&'a TurnRecord>,
}

fn group_turns_by_model(turns: &[TurnRecord]) -> Vec<ModelRollup<'_>> {
    let mut grouped = std::collections::BTreeMap::<&str, Vec<&TurnRecord>>::new();
    for turn in turns {
        grouped.entry(&turn.model).or_default().push(turn);
    }

    let mut rollups = grouped
        .into_iter()
        .map(|(model, turns)| ModelRollup { model, turns })
        .collect::<Vec<_>>();
    rollups.sort_by(|left, right| {
        let left_cost: f64 = left.turns.iter().map(|turn| turn.cost_usd).sum();
        let right_cost: f64 = right.turns.iter().map(|turn| turn.cost_usd).sum();
        right_cost
            .total_cmp(&left_cost)
            .then_with(|| left.model.cmp(right.model))
    });
    rollups
}

fn render_model_rollup(rollup: &ModelRollup<'_>) -> String {
    let count = rollup.turns.len();
    let total_cost: f64 = rollup.turns.iter().map(|turn| turn.cost_usd).sum();
    let total_input: usize = rollup.turns.iter().map(|turn| turn.input_tokens).sum();
    let total_output: usize = rollup.turns.iter().map(|turn| turn.output_tokens).sum();
    let total_cache_create: usize = rollup.turns.iter().map(|turn| turn.cache_create).sum();
    let total_cache_read: usize = rollup.turns.iter().map(|turn| turn.cache_read).sum();
    let billed_input = (total_input + total_cache_create + total_cache_read).max(1);
    let hit_percent = (total_cache_read * 100) / billed_input;
    let turn_label = format!("{count} turn{}", if count == 1 { "" } else { "s" });

    format!(
        "  {model:<18} {turn_label:<9} {cost:<8} {input}/{output} tok · cache {hit_percent}% hit",
        model = rollup.model,
        cost = CostTracker::format_usd(total_cost),
        input = CostTracker::format_tokens(total_input),
        output = CostTracker::format_tokens(total_output)
    )
}

fn render_help_index(registered: &[CommandCompletionSource]) -> String {
    let mut by_name = std::collections::BTreeMap::new();
    for command in registered {
        by_name.insert(command.name.as_str(), command);
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut lines = vec![
        String::new(),
        format!("Deeptide commands ({}):", registered.len()),
    ];

    for (title, names) in help_categories() {
        let commands = names
            .iter()
            .filter_map(|name| by_name.get(name).copied())
            .collect::<Vec<_>>();
        if commands.is_empty() {
            continue;
        }

        lines.push(String::new());
        lines.push((*title).to_owned());
        for command in commands {
            lines.push(format!(
                "  {:<28} {}",
                format_command_names(command),
                command.description
            ));
            seen.insert(command.name.as_str());
        }
    }

    let leftover = registered
        .iter()
        .filter(|command| !seen.contains(command.name.as_str()))
        .collect::<Vec<_>>();
    if !leftover.is_empty() {
        lines.push(String::new());
        lines.push(String::from("Other"));
        for command in leftover {
            lines.push(format!(
                "  {:<28} {}",
                format_command_names(command),
                command.description
            ));
        }
    }

    lines.push(String::new());
    lines.push(String::from(
        "Keybindings: Enter=submit  Tab=autocomplete  Ctrl+C=interrupt  Ctrl+D=exit",
    ));
    lines.push(String::from(
        "Type /help <command> for details on a single command.",
    ));
    lines.join("\n")
}

fn render_help_detail(command: &CommandCompletionSource) -> String {
    let mut lines = vec![
        String::new(),
        format_command_names_with_separator(command, " \u{00b7} "),
        command.description.clone(),
        String::new(),
        format!("Usage:   {}", command.usage),
    ];

    if !command.aliases.is_empty() {
        lines.push(format!(
            "Aliases: {}",
            command
                .aliases
                .iter()
                .map(|alias| format!("/{alias}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }

    lines.join("\n")
}

pub(crate) fn render_unknown_command(
    needle: &str,
    registered: &[CommandCompletionSource],
) -> String {
    let suggestions = suggest_commands(needle, registered, 3);

    let mut lines = vec![format!("Unknown command: /{needle}")];
    if suggestions.is_empty() {
        lines.push(String::from("Type /help for the full list."));
    } else {
        lines.push(format!("Did you mean: {}?", suggestions.join(", ")));
    }
    lines.join("\n")
}

/// Suggest up to `limit` command names close to `needle`. Strong signals come
/// first: a command whose name/alias starts with the needle, then one that
/// merely contains it. Only when neither matches do we fall back to small
/// edit-distance (typo) matches like `/commti` -> `/commit`, so the common
/// abbreviation case keeps its existing, predictable ordering.
fn suggest_commands(
    needle: &str,
    registered: &[CommandCompletionSource],
    limit: usize,
) -> Vec<String> {
    let mut prefix_hits = Vec::new();
    let mut substring_hits = Vec::new();
    let mut fuzzy_hits: Vec<(usize, String)> = Vec::new();

    // Longer needles get more slack; a 2-3 char needle stays strict so a single
    // typo doesn't fuzzily match half the command table.
    let max_distance = if needle.len() <= 3 { 1 } else { 2 };

    for command in registered {
        let candidates = std::iter::once(command.name.as_str())
            .chain(command.aliases.iter().map(String::as_str))
            .map(str::to_ascii_lowercase)
            .collect::<Vec<_>>();
        let label = format!("/{}", command.name);

        if candidates
            .iter()
            .any(|candidate| candidate.starts_with(needle))
        {
            prefix_hits.push(label);
        } else if candidates
            .iter()
            .any(|candidate| candidate.contains(needle))
        {
            substring_hits.push(label);
        } else if let Some(distance) = candidates
            .iter()
            .map(|candidate| levenshtein(needle, candidate))
            .min()
            .filter(|distance| *distance <= max_distance)
        {
            fuzzy_hits.push((distance, label));
        }
    }

    let mut suggestions: Vec<String> = prefix_hits
        .into_iter()
        .chain(substring_hits)
        .take(limit)
        .collect();

    // Typo matches are a last resort: only surface them when nothing matched by
    // prefix or substring, ranked closest-first (ties keep registration order).
    if suggestions.is_empty() {
        fuzzy_hits.sort_by_key(|(distance, _)| *distance);
        suggestions = fuzzy_hits
            .into_iter()
            .map(|(_, label)| label)
            .take(limit)
            .collect();
    }

    suggestions
}

/// Classic two-row Levenshtein edit distance over chars. Command names are
/// short, so the O(n*m) cost is negligible.
fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }

    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut curr = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        curr[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[b.len()]
}

fn find_command<'a>(
    needle: &str,
    registered: &'a [CommandCompletionSource],
) -> Option<&'a CommandCompletionSource> {
    registered.iter().find(|command| {
        command.name.eq_ignore_ascii_case(needle)
            || command
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(needle))
    })
}

fn format_command_names(command: &CommandCompletionSource) -> String {
    format_command_names_with_separator(command, ", ")
}

fn format_command_names_with_separator(
    command: &CommandCompletionSource,
    separator: &str,
) -> String {
    std::iter::once(command.name.as_str())
        .chain(command.aliases.iter().map(String::as_str))
        .map(|name| format!("/{name}"))
        .collect::<Vec<_>>()
        .join(separator)
}

fn help_categories() -> &'static [(&'static str, &'static [&'static str])] {
    &[
        (
            "Core",
            &[
                "help", "exit", "new", "clear", "compact", "status", "cost", "usage", "reminder",
                "goal",
            ],
        ),
        ("Model", &["model", "provider", "fast", "tps", "think"]),
        ("Memory", &["memory", "remember", "dream"]),
        (
            "Sessions",
            &[
                "resume",
                "retry",
                "sessions",
                "queue",
                "search",
                "checkpoint",
                "checkpoints",
                "rewind",
                "export",
                "copy",
                "paste",
            ],
        ),
        ("Permissions", &["permission"]),
        ("Hooks", &["hooks"]),
        ("Cron", &["cron"]),
        (
            "Files",
            &[
                "diff", "add-dir", "open", "context", "init", "read", "write",
            ],
        ),
        ("Git", &["branch", "commit"]),
        ("Review", &["review", "test", "lint"]),
        ("Skills", &["skills", "simplify", "tools"]),
        ("Config", &["config", "doctor", "update", "cache"]),
        ("UX", &["keybindings", "debug", "vim"]),
    ]
}
