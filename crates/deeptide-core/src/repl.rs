use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentTerminalEvent, ClearCommand,
    CommandCompletionSource, CommandContext, CommandResult, CompactCommand, CostCommand,
    CostTracker, HelpCommand, MemoryCommand, NewCommand, PermissionManager, PermissionMode,
    PermissionRules, RememberCommand, Rule, SlashCommand, Tool, ToolContext, ToolRegistry,
    ToolResultSummaryFormatter, WriteTool, memory::MemorySystem, tools::model_context_window,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplEvent {
    Output(String),
    Exit,
}

pub struct ReplSession {
    agent_loop: AgentLoop,
    cost_display_enabled: Arc<AtomicBool>,
    tool_registry: ToolRegistry,
    tool_context: ToolContext,
}

impl ReplSession {
    pub fn new(backend: Box<dyn AgentBackend>) -> Self {
        Self {
            agent_loop: AgentLoop::new(backend),
            cost_display_enabled: Arc::new(AtomicBool::new(false)),
            tool_registry: ToolRegistry::with_builtin_tools(),
            tool_context: ToolContext::new(
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            ),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.agent_loop = self.agent_loop.with_model(model);
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<std::path::PathBuf>) -> Self {
        self.tool_context = ToolContext::new(cwd);
        self.agent_loop = self.agent_loop.with_cwd(self.tool_context.cwd.clone());
        self
    }

    pub fn with_permission_mode(mut self, mode: PermissionMode) -> Self {
        self.agent_loop = self.agent_loop.with_permission_mode(mode);
        self
    }

    pub fn with_permission_manager(mut self, permission_manager: PermissionManager) -> Self {
        self.agent_loop = self.agent_loop.with_permission_manager(permission_manager);
        self
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.agent_loop = self.agent_loop.with_max_turns(max_turns);
        self
    }

    pub fn with_subagent_backend_factory<F>(mut self, factory: F) -> Self
    where
        F: Fn(&str) -> Box<dyn AgentBackend> + Send + Sync + 'static,
    {
        self.agent_loop = self.agent_loop.with_subagent_backend_factory(factory);
        self
    }

    pub fn submit(&mut self, line: &str) -> Vec<ReplEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        if let Some(command_line) = trimmed.strip_prefix('/') {
            return self.execute_command(command_line);
        }

        self.agent_loop
            .run(trimmed)
            .into_iter()
            .filter_map(agent_event_to_repl_event)
            .collect()
    }

    pub fn banner(&self) -> String {
        String::from("Deeptide Rust REPL")
    }

    pub fn prompt(&self) -> String {
        String::from("deeptide> ")
    }

    pub fn agent_loop(&self) -> &AgentLoop {
        &self.agent_loop
    }

    fn execute_command(&mut self, command_line: &str) -> Vec<ReplEvent> {
        let mut parts = command_line.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or_default().to_ascii_lowercase();
        let args = parts.next().unwrap_or_default();

        if matches!(name.as_str(), "exit" | "quit" | "q") {
            return vec![ReplEvent::Exit];
        }

        let context = self.command_context();
        let result = match name.as_str() {
            "help" | "h" | "?" => HelpCommand.execute(args, &context),
            "clear" | "cls" => {
                self.agent_loop.reset();
                ClearCommand.execute(args, &context)
            }
            "new" => {
                self.agent_loop.reset();
                NewCommand.execute(args, &context)
            }
            "compact" | "compress" => CompactCommand.execute(args, &context),
            "cost" => CostCommand.execute(args, &context),
            "status" => self.execute_status_command(args),
            "context" | "ctx" => self.execute_context_command(args),
            "read" => self.execute_read_command(args),
            "write" => self.execute_write_command(args),
            "memory" | "mem" => MemoryCommand.execute(args, &context),
            "remember" => RememberCommand.execute(args, &context),
            "permission" | "perm" | "permissions" => self.execute_permission_command(args),
            _ => CommandResult::Text(format!(
                "Unknown command: /{name}\nType /help for the full list."
            )),
        };

        command_result_to_repl_events(result)
    }

    fn execute_read_command(&self, args: &str) -> CommandResult {
        let parsed = match parse_read_args(args) {
            Ok(parsed) => parsed,
            Err(message) => return CommandResult::Text(message),
        };

        let mut input = serde_json::Map::new();
        input.insert(
            String::from("file_path"),
            serde_json::Value::String(parsed.file_path),
        );
        if let Some(offset) = parsed.offset {
            input.insert(String::from("offset"), serde_json::json!(offset));
        }
        if let Some(limit) = parsed.limit {
            input.insert(String::from("limit"), serde_json::json!(limit));
        }

        let result =
            self.tool_registry
                .call("Read", serde_json::Value::Object(input), &self.tool_context);
        CommandResult::Text(result.content)
    }

    fn execute_write_command(&self, args: &str) -> CommandResult {
        let parsed = match parse_write_args(args) {
            Ok(parsed) => parsed,
            Err(message) => return CommandResult::Text(message),
        };

        let result = WriteTool.call(
            serde_json::json!({
                "file_path": parsed.file_path,
                "content": parsed.content,
            }),
            &self.tool_context,
        );
        CommandResult::Text(result.content)
    }

    fn execute_status_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /status"));
        }

        CommandResult::Text(render_status(&self.agent_loop, &self.tool_context.cwd))
    }

    fn execute_context_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /context"));
        }

        CommandResult::Text(render_context(
            &self.agent_loop,
            &self.tool_context.cwd,
            self.tool_registry.names(),
        ))
    }

    fn execute_permission_command(&mut self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return CommandResult::Text(render_permission_rules(
                self.agent_loop.permission_rules(),
            ));
        }

        if let Some(raw) = trimmed.strip_prefix("--allow ") {
            let raw = raw.trim();
            if raw.is_empty() {
                return CommandResult::Text(String::from(
                    "Usage: /permission [--allow Tool(pattern) | --deny Tool(pattern) | --remove pattern]",
                ));
            }
            let (tool, pattern) = parse_permission_rule_pattern(raw);
            return match self.agent_loop.add_permission_rule(true, pattern, tool) {
                Ok(()) => CommandResult::Text(format!("+allow {raw}")),
                Err(error) => CommandResult::Text(format!("Failed: {error}")),
            };
        }

        if let Some(raw) = trimmed.strip_prefix("--deny ") {
            let raw = raw.trim();
            if raw.is_empty() {
                return CommandResult::Text(String::from(
                    "Usage: /permission [--allow Tool(pattern) | --deny Tool(pattern) | --remove pattern]",
                ));
            }
            let (tool, pattern) = parse_permission_rule_pattern(raw);
            return match self.agent_loop.add_permission_rule(false, pattern, tool) {
                Ok(()) => CommandResult::Text(format!("+deny {raw}")),
                Err(error) => CommandResult::Text(format!("Failed: {error}")),
            };
        }

        if let Some(raw) = trimmed.strip_prefix("--remove ") {
            let pattern = raw.trim();
            if pattern.is_empty() {
                return CommandResult::Text(String::from(
                    "Usage: /permission [--allow Tool(pattern) | --deny Tool(pattern) | --remove pattern]",
                ));
            }
            return match self.agent_loop.remove_permission_rule(pattern) {
                Ok(()) => CommandResult::Text(format!("Removed {pattern}")),
                Err(error) => CommandResult::Text(format!("Failed: {error}")),
            };
        }

        CommandResult::Text(String::from(
            "Usage: /permission [--allow Tool(pattern) | --deny Tool(pattern) | --remove pattern]",
        ))
    }

    fn command_context(&self) -> CommandContext {
        let cost_display_enabled = Arc::clone(&self.cost_display_enabled);
        let set_cost_display_enabled = Arc::clone(&self.cost_display_enabled);
        let summary = self.agent_loop.cost_tracker().summary();

        CommandContext::builder()
            .clear_conversation(|| Some(String::new()))
            .compact_conversation(|| {})
            .all_commands(repl_command_sources)
            .cost_summary(move || summary.clone())
            .cost_display_enabled(move || cost_display_enabled.load(Ordering::SeqCst))
            .set_cost_display_enabled(move |enabled| {
                set_cost_display_enabled.store(enabled, Ordering::SeqCst);
            })
            .build()
    }
}

fn agent_event_to_repl_event(event: AgentLoopEvent) -> Option<ReplEvent> {
    match event {
        AgentLoopEvent::Assistant(message) => Some(ReplEvent::Output(message.content)),
        AgentLoopEvent::ToolBatchSummary {
            label,
            failed_count,
            ..
        } => {
            let status = if failed_count == 0 {
                "completed"
            } else {
                "completed with failures"
            };
            Some(ReplEvent::Output(format!("Tools {status}: {label}")))
        }
        AgentLoopEvent::Terminal(AgentTerminalEvent::MaxTurnsReached) => {
            Some(ReplEvent::Output(String::from("Maximum turns reached.")))
        }
        AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(error)) => {
            Some(ReplEvent::Output(format!("Model error: {error}")))
        }
        AgentLoopEvent::ToolResult {
            tool_call,
            content,
            is_error,
        } => Some(ReplEvent::Output(render_tool_result_for_repl(
            &tool_call.name,
            &tool_call.id,
            &content,
            is_error,
        ))),
        AgentLoopEvent::User(_) | AgentLoopEvent::Terminal(AgentTerminalEvent::Complete) => None,
    }
}

fn render_tool_result_for_repl(
    tool_name: &str,
    tool_id: &str,
    content: &str,
    is_error: bool,
) -> String {
    let summary = ToolResultSummaryFormatter::summary(tool_name, content, is_error);
    let verb = if is_error { "failed" } else { "completed" };
    let header = format!("Tool {tool_name} ({tool_id}) {verb}:");
    let trimmed = content.trim();

    if is_error
        || trimmed.is_empty()
        || ToolResultSummaryFormatter::should_mute_appearance(tool_name, content, is_error)
        || !should_expand_tool_result(trimmed)
    {
        return format!("{header} {summary}");
    }

    format!("{header} {summary}\n{trimmed}")
}

fn should_expand_tool_result(trimmed: &str) -> bool {
    trimmed.len() <= 2_000 && trimmed.lines().count() <= 12
}

fn command_result_to_repl_events(result: CommandResult) -> Vec<ReplEvent> {
    match result {
        CommandResult::Text(text) => vec![ReplEvent::Output(text)],
        CommandResult::Exit => vec![ReplEvent::Exit],
        CommandResult::Compact => vec![ReplEvent::Output(String::from("Context compacted."))],
        CommandResult::Skip => Vec::new(),
    }
}

fn repl_command_sources() -> Vec<CommandCompletionSource> {
    vec![
        CommandCompletionSource::from_command(&HelpCommand),
        CommandCompletionSource::new("exit", ["quit", "q"], "Exit the REPL", "/exit"),
        CommandCompletionSource::from_command(&ClearCommand),
        CommandCompletionSource::from_command(&NewCommand),
        CommandCompletionSource::from_command(&CompactCommand),
        CommandCompletionSource::from_command(&CostCommand),
        CommandCompletionSource::new(
            "status",
            Vec::<&str>::new(),
            "Show session status: model, cwd, turns, tokens, cost",
            "/status",
        ),
        CommandCompletionSource::new(
            "context",
            ["ctx"],
            "Inspect what is loaded into the session",
            "/context",
        ),
        CommandCompletionSource::new(
            "read",
            Vec::<&str>::new(),
            "Read a text file with optional line range",
            "/read <path> [--offset N] [--limit N]",
        ),
        CommandCompletionSource::new(
            "write",
            Vec::<&str>::new(),
            "Write complete text to a file",
            "/write <path> <content>",
        ),
        CommandCompletionSource::from_command(&MemoryCommand),
        CommandCompletionSource::from_command(&RememberCommand),
        CommandCompletionSource::new(
            "permission",
            ["perm", "permissions"],
            "List or modify permission rules",
            "/permission [--allow Pattern | --deny Pattern | --remove pattern]",
        ),
    ]
}

fn render_status(agent_loop: &AgentLoop, cwd: &std::path::Path) -> String {
    let summary = agent_loop.cost_tracker().summary();
    let cache = summary.cache_health();
    let context_tokens = estimate_repl_context_tokens(agent_loop.messages());
    let branch = git_branch(cwd).unwrap_or_else(|| String::from("(no git)"));
    let cache_summary = render_cache_health(&cache);

    let mut lines = vec![
        String::from("Deeptide session status"),
        format!("  Model:    {}", agent_loop.model()),
        format!("  CWD:      {}", cwd.display()),
        format!("  Branch:   {branch}"),
        String::from("  Session:  (not persisted)"),
        format!(
            "  Turns:    {} / {}",
            summary.turns.len(),
            agent_loop.max_turns()
        ),
        format!("  Messages: {}", agent_loop.messages().len()),
        format!(
            "  Context:  ~{} tokens",
            CostTracker::format_tokens(context_tokens)
        ),
        format!("  Mode:     {}", agent_loop.permission_mode().label()),
        format!(
            "  In/Out:   {} / {}",
            CostTracker::format_tokens(summary.total_input),
            CostTracker::format_tokens(summary.total_output)
        ),
        format!("  Cache:    {cache_summary}"),
    ];

    if let Some(diagnostic) = cache.diagnostic() {
        lines.push(format!("  Cache note: {diagnostic}"));
    }

    lines.push(format!(
        "  Cost:     {}",
        CostTracker::format_usd(summary.total_cost_usd)
    ));

    lines.join("\n")
}

fn render_context(agent_loop: &AgentLoop, cwd: &std::path::Path, tool_names: Vec<&str>) -> String {
    let context_tokens = estimate_repl_context_tokens(agent_loop.messages());
    let context_window = model_context_window(agent_loop.model()) as usize;
    let percent = (context_tokens * 100)
        .checked_div(context_window)
        .unwrap_or(0);
    let bar_width = 20usize;
    let filled = (bar_width * context_tokens)
        .checked_div(context_window)
        .unwrap_or(0)
        .min(bar_width);
    let bar = format!(
        "{}{}",
        "#".repeat(filled),
        "-".repeat(bar_width.saturating_sub(filled))
    );

    let project_memory = MemorySystem::project_memory_index(cwd);
    let global_memory = MemorySystem::global_memory_index();
    let agents = discover_agent_names(cwd);
    let tool_preview = tool_names
        .iter()
        .take(10)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    let tool_suffix = if tool_names.len() > 10 { ", ..." } else { "" };

    let mut lines = vec![
        String::from("Session context"),
        format!("  CWD:      {}", cwd.display()),
        String::from("  + dirs:   (none)"),
        String::from("  Memory:"),
        format!(
            "    {} {}",
            exists_mark(&project_memory),
            project_memory.display()
        ),
        format!(
            "    {} {}",
            exists_mark(&global_memory),
            global_memory.display()
        ),
        format!(
            "  Agents:   {}",
            if agents.is_empty() {
                String::from("(none)")
            } else {
                agents.join(", ")
            }
        ),
        String::from("  Settings:"),
        format!("    runtime  {}", agent_loop.model()),
        format!("    mode     {}", agent_loop.permission_mode().label()),
        format!(
            "  Tools:    {} - {tool_preview}{tool_suffix}",
            tool_names.len()
        ),
        format!(
            "  Window:   {bar} {percent}%  ({} / {})",
            CostTracker::format_tokens(context_tokens),
            CostTracker::format_tokens(context_window)
        ),
    ];

    let suggestions = context_suggestions(context_tokens, context_window);
    if !suggestions.is_empty() {
        lines.push(String::new());
        lines.push(String::from("Suggestions"));
        lines.extend(
            suggestions
                .into_iter()
                .map(|suggestion| format!("  {suggestion}")),
        );
    }

    lines.join("\n")
}

fn exists_mark(path: &std::path::Path) -> &'static str {
    if path.exists() { "*" } else { "o" }
}

fn context_suggestions(context_tokens: usize, context_window: usize) -> Vec<String> {
    if context_window == 0 {
        return Vec::new();
    }
    let percent = (context_tokens * 100) / context_window;
    if percent >= 80 {
        vec![format!(
            "Context is {percent}% full - run /compact before starting a large edit."
        )]
    } else if percent >= 60 {
        vec![format!(
            "Context is {percent}% full - consider /status or /compact after the next large tool result."
        )]
    } else {
        Vec::new()
    }
}

fn discover_agent_names(cwd: &std::path::Path) -> Vec<String> {
    let mut dirs = vec![
        MemorySystem::tide_config_dir()
            .join("projects")
            .join(MemorySystem::project_slug(cwd))
            .join("agents"),
        MemorySystem::tide_config_dir().join("agents"),
        cwd.join(".deeptide").join("agents"),
    ];
    if let Some(home) = home_dir() {
        dirs.push(home.join(".deeptide").join("agents"));
    }

    let mut names = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
                continue;
            }
            if let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) {
                names.push(stem.to_owned());
            }
        }
    }
    names.sort();
    names.dedup();
    names
}

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
}

fn render_cache_health(cache: &crate::CacheHealth) -> String {
    let created = CostTracker::format_tokens(cache.total_create_tokens);
    let read = CostTracker::format_tokens(cache.total_read_tokens);
    match cache.hit_rate_percent {
        Some(hit_rate) => {
            let recent = cache
                .recent_hit_rate_percent
                .map(|rate| format!(" · recent {rate}% hit"))
                .unwrap_or_default();
            format!(
                "{created} created, {read} read · {hit_rate}% hit{recent} · {}",
                cache.label()
            )
        }
        None => format!("{} · no cache telemetry yet", cache.label()),
    }
}

fn estimate_repl_context_tokens(messages: &[crate::ConversationMessage]) -> usize {
    messages
        .iter()
        .map(|message| message.content.chars().count().div_ceil(4) + 4)
        .sum()
}

fn git_branch(cwd: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["-C"])
        .arg(cwd)
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let branch = String::from_utf8(output.stdout).ok()?.trim().to_owned();
    (!branch.is_empty()).then_some(branch)
}

fn render_permission_rules(rules: &PermissionRules) -> String {
    let mut lines = vec![String::from("Permission rules:"), String::from("  allow:")];
    append_permission_rules(&mut lines, rules.allow_list());
    if !rules.session_allow_list().is_empty() {
        lines.push(String::from("  session allow:"));
        append_permission_rules(&mut lines, rules.session_allow_list());
    }
    lines.push(String::from("  deny:"));
    append_permission_rules(&mut lines, rules.deny_list());
    if !rules.session_deny_list().is_empty() {
        lines.push(String::from("  session deny:"));
        append_permission_rules(&mut lines, rules.session_deny_list());
    }
    lines.join("\n")
}

fn append_permission_rules(lines: &mut Vec<String>, rules: &[Rule]) {
    if rules.is_empty() {
        lines.push(String::from("    (none)"));
        return;
    }
    for rule in rules {
        lines.push(format!("    {}", format_permission_rule(rule)));
    }
}

fn format_permission_rule(rule: &Rule) -> String {
    rule.tool
        .as_ref()
        .map(|tool| format!("{tool}({})", rule.pattern))
        .unwrap_or_else(|| rule.pattern.clone())
}

fn parse_permission_rule_pattern(raw: &str) -> (Option<String>, String) {
    if let Some(open_index) = raw.find('(')
        && raw.ends_with(')')
    {
        let tool = raw[..open_index].trim();
        let pattern = raw[open_index + 1..raw.len() - 1].trim();
        if !tool.is_empty() && !pattern.is_empty() {
            return (Some(tool.to_owned()), pattern.to_owned());
        }
    }

    if let Some((tool, pattern)) = raw.split_once(':') {
        let tool = tool.trim();
        let pattern = pattern.trim();
        if !tool.is_empty()
            && !pattern.is_empty()
            && tool
                .chars()
                .next()
                .is_some_and(|character| character.is_ascii_uppercase())
        {
            return (Some(tool.to_owned()), pattern.to_owned());
        }
    }

    (None, raw.to_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadArgs {
    file_path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriteArgs {
    file_path: String,
    content: String,
}

fn parse_read_args(args: &str) -> Result<ReadArgs, String> {
    let parts = split_shell_like(args);
    if parts.is_empty() {
        return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
    }

    let mut file_path = None;
    let mut offset = None;
    let mut limit = None;
    let mut index = 0;
    while index < parts.len() {
        match parts[index].as_str() {
            "--offset" => {
                index += 1;
                let Some(raw) = parts.get(index) else {
                    return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
                };
                offset = Some(
                    raw.parse::<usize>()
                        .map_err(|_| String::from("--offset must be a positive number"))?,
                );
            }
            "--limit" => {
                index += 1;
                let Some(raw) = parts.get(index) else {
                    return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
                };
                limit = Some(
                    raw.parse::<usize>()
                        .map_err(|_| String::from("--limit must be a positive number"))?,
                );
            }
            value if value.starts_with("--") => {
                return Err(format!("Unknown /read option: {value}"));
            }
            value => {
                if file_path.is_some() {
                    return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
                }
                file_path = Some(value.to_owned());
            }
        }
        index += 1;
    }

    let Some(file_path) = file_path else {
        return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
    };

    Ok(ReadArgs {
        file_path,
        offset,
        limit,
    })
}

fn parse_write_args(args: &str) -> Result<WriteArgs, String> {
    let trimmed = args.trim_start();
    if trimmed.is_empty() {
        return Err(String::from("Usage: /write <path> <content>"));
    }

    let mut split_at = None;
    for (index, character) in trimmed.char_indices() {
        if character.is_whitespace() {
            split_at = Some(index);
            break;
        }
    }

    let Some(split_at) = split_at else {
        return Err(String::from("Usage: /write <path> <content>"));
    };
    let file_path = trimmed[..split_at].to_owned();
    let content = trimmed[split_at..].trim_start().to_owned();
    if content.is_empty() {
        return Err(String::from("Usage: /write <path> <content>"));
    }

    Ok(WriteArgs { file_path, content })
}

fn split_shell_like(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;

    for character in input.chars() {
        if Some(character) == quote {
            quote = None;
        } else if quote.is_none() && matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if quote.is_none() && character.is_whitespace() {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}
