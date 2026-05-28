use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentTerminalEvent, ClearCommand,
    CommandCompletionSource, CommandContext, CommandResult, CompactCommand, CostCommand,
    CostTracker, HelpCommand, MemoryCommand, NewCommand, PermissionManager, PermissionMode,
    PermissionRules, RememberCommand, Rule, SlashCommand, Tool, ToolContext, ToolRegistry,
    ToolResultSummaryFormatter, WriteTool,
    agent_loop::{ConversationMessage, MessageRole},
    memory::MemorySystem,
    tools::{ClipboardTool, model_context_window},
    tui::{StatusLine, StatusSegment},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplEvent {
    Output(String),
    Exit,
}

type ClipboardWriter = Arc<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProviderProfile {
    Legacy,
    DeepSeek,
    Paean,
}

impl ProviderProfile {
    fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "legacy" | "env" => Some(Self::Legacy),
            "deepseek" | "official" | "direct" | "default" => Some(Self::DeepSeek),
            "paean" | "paean-ai" | "multimodal" => Some(Self::Paean),
            _ => None,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::DeepSeek => "deepseek",
            Self::Paean => "paean",
        }
    }
}

pub struct ReplSession {
    agent_loop: AgentLoop,
    cost_display_enabled: Arc<AtomicBool>,
    tool_registry: ToolRegistry,
    tool_context: ToolContext,
    clipboard_writer: ClipboardWriter,
    additional_dirs: Vec<std::path::PathBuf>,
    provider_profile: ProviderProfile,
    debug_enabled: bool,
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
            clipboard_writer: Arc::new(write_to_system_clipboard),
            additional_dirs: Vec::new(),
            provider_profile: ProviderProfile::Legacy,
            debug_enabled: false,
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

    pub fn with_clipboard_writer<F>(mut self, writer: F) -> Self
    where
        F: Fn(&str) -> Result<(), String> + Send + Sync + 'static,
    {
        self.clipboard_writer = Arc::new(writer);
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

    pub fn status_line(&self) -> StatusLine {
        let summary = self.agent_loop.cost_tracker().summary();
        let context_tokens = estimate_repl_context_tokens(self.agent_loop.messages());
        let window = model_context_window(self.agent_loop.model()) as usize;
        let context_pct = context_tokens
            .saturating_mul(100)
            .checked_div(window)
            .unwrap_or(0);
        let branch = git_branch(&self.tool_context.cwd).unwrap_or_else(|| String::from("no-git"));

        StatusLine::new([
            StatusSegment::new("model", self.agent_loop.model()),
            StatusSegment::new("mode", self.agent_loop.permission_mode().label()),
            StatusSegment::new("ctx", format!("{context_pct}%")),
            StatusSegment::new(
                "turns",
                format!("{}/{}", summary.turns.len(), self.agent_loop.max_turns()),
            ),
            StatusSegment::new("git", branch),
            StatusSegment::new("cost", CostTracker::format_usd(summary.total_cost_usd)),
        ])
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
            "model" | "m" => self.execute_model_command(args),
            "provider" | "profiles" => self.execute_provider_command(args),
            "status" => self.execute_status_command(args),
            "context" | "ctx" => self.execute_context_command(args),
            "retry" | "r" | "again" => return self.execute_retry_command(args),
            "copy" | "yank" => self.execute_copy_command(args),
            "export" => self.execute_export_command(args),
            "diff" => self.execute_diff_command(args),
            "branch" => self.execute_branch_command(args),
            "add-dir" | "add_dir" | "adddir" => self.execute_add_dir_command(args),
            "fast" | "faster" => self.execute_fast_command(args),
            "tps" | "speed" => self.execute_tps_command(args),
            "debug" | "dbg" => self.execute_debug_command(args),
            "keybindings" | "keys" => self.execute_keybindings_command(args),
            "sessions" | "session" => self.execute_sessions_command(args),
            "resume" | "load" | "restore" => self.execute_resume_command(args),
            "open" => self.execute_open_command(args),
            "paste" | "p" => self.execute_paste_command(args),
            "doctor" => self.execute_doctor_command(args),
            "config" => self.execute_config_command(args),
            "hooks" => self.execute_hooks_command(args),
            "init" => self.execute_init_command(args),
            "update" | "upgrade" => self.execute_update_command(args),
            "vim" | "edit" | "e" | "compose" => self.execute_vim_command(args),
            "read" => self.execute_read_command(args),
            "write" => self.execute_write_command(args),
            "memory" | "mem" => MemoryCommand.execute(args, &context),
            "remember" => RememberCommand.execute(args, &context),
            "permission" | "perm" | "permissions" => self.execute_permission_command(args),
            "commit" => return self.execute_commit_command(args),
            "review" => return self.execute_review_command(args),
            "simplify" => return self.execute_simplify_command(args),
            "skills" | "skill" => self.execute_skills_command(args),
            "reminder" | "anchor" | "reorient" => return self.execute_reminder_command(args),
            "dream" => return self.execute_dream_command(args),
            "cron" => self.execute_cron_command(args),
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

        CommandResult::Text(render_status(
            &self.agent_loop,
            &self.tool_context.cwd,
            &self.additional_dirs,
            self.provider_profile,
        ))
    }

    fn execute_provider_command(&mut self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() || trimmed == "status" {
            return CommandResult::Text(render_provider_status(self.provider_profile));
        }

        if trimmed == "list" {
            return CommandResult::Text(render_provider_list(self.provider_profile));
        }

        if let Some(raw) = trimmed.strip_prefix("use ") {
            let raw = raw.trim();
            if raw.is_empty() || raw.split_whitespace().count() > 1 {
                return CommandResult::Text(String::from(
                    "Usage: /provider use <name|deepseek|paean>",
                ));
            }
            let Some(profile) = ProviderProfile::parse(raw) else {
                return CommandResult::Text(format!(
                    "Unknown provider profile `{raw}`. Use `/provider list`."
                ));
            };
            self.provider_profile = profile;
            return CommandResult::Text(format!(
                "Active provider profile: {} (recorded for this REPL session; launch configuration controls the current model client)",
                profile.name()
            ));
        }

        CommandResult::Text(String::from(
            "Usage: /provider [list | use <name|deepseek|paean> | status]",
        ))
    }

    fn execute_fast_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /fast"));
        }

        CommandResult::Text(String::from(
            "Fast mode: use the --fast CLI flag at launch. Runtime toggle coming in a future update.",
        ))
    }

    fn execute_tps_command(&self, args: &str) -> CommandResult {
        let flags = args.split_whitespace().collect::<Vec<_>>();
        if flags.contains(&"--reset") {
            return CommandResult::Text(String::from(
                "No model TPS samples are recorded by the Rust REPL yet.",
            ));
        }

        if flags.iter().any(|flag| *flag != "--json") {
            return CommandResult::Text(String::from("Usage: /tps [--json | --reset]"));
        }

        if flags.contains(&"--json") {
            CommandResult::Text(String::from("[]"))
        } else {
            CommandResult::Text(String::from(
                "No model TPS samples recorded yet. Run a streamed model session to collect speed telemetry.",
            ))
        }
    }

    fn execute_debug_command(&mut self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /debug"));
        }

        self.debug_enabled = !self.debug_enabled;
        let status = if self.debug_enabled { "on" } else { "off" };
        CommandResult::Text(format!("Debug mode: {status}"))
    }

    fn execute_keybindings_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /keybindings"));
        }

        CommandResult::Text(
            [
                "Key bindings:",
                "  Enter           Submit prompt",
                "  Backslash + Enter Continue on next line",
                "  Tab             Autocomplete /command or @path",
                "  Ctrl+C          Interrupt running tool / exit when idle",
                "  Ctrl+D          Exit on empty line",
                "  Ctrl+L          Clear screen",
                "  Ctrl+A / Ctrl+E Move to start / end of line",
                "  Ctrl+K          Kill to end of line",
                "  Ctrl+U          Kill to start of line",
                "  Ctrl+W          Delete previous word",
                "  Up / Down       Browse history",
            ]
            .join("\n"),
        )
    }

    fn execute_sessions_command(&self, args: &str) -> CommandResult {
        if args.split_whitespace().count() > 1 {
            return CommandResult::Text(String::from("Usage: /sessions [filter]"));
        }

        CommandResult::Text(String::from(
            "No persisted sessions are available in the Rust REPL yet. Use /export [path] to save the current transcript.",
        ))
    }

    fn execute_resume_command(&self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.split_whitespace().count() > 1 {
            return CommandResult::Text(String::from("Usage: /resume [session-id]"));
        }

        if trimmed.is_empty() {
            CommandResult::Text(String::from("No sessions to resume in this project."))
        } else {
            CommandResult::Text(format!(
                "Session not found: {trimmed}. Persisted session restore is not available in the Rust REPL yet."
            ))
        }
    }

    fn execute_open_command(&self, args: &str) -> CommandResult {
        let raw = unquote_path(args.trim());
        if raw.is_empty() || raw.split_whitespace().count() > 1 && !args.trim().starts_with('"') {
            return CommandResult::Text(String::from("Usage: /open <path>"));
        }

        let path = self.tool_context.resolve_path(&raw);
        if !path.exists() {
            return CommandResult::Text(format!("File does not exist: {}", path.display()));
        }

        CommandResult::Text(format!(
            "{} is not classified as sensitive in the Rust build; normal tools can already read it.",
            path.display()
        ))
    }

    fn execute_paste_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /paste"));
        }

        let result =
            ClipboardTool.call(serde_json::json!({"operation": "read"}), &self.tool_context);
        if result.is_error {
            return CommandResult::Text(format!("/paste: {}", result.content));
        }

        let content = result.content.trim();
        if content.is_empty() || content == "[Clipboard is empty]" {
            return CommandResult::Text(String::from(
                "/paste: clipboard has no text content. Image prefill is not available in the Rust REPL yet.",
            ));
        }

        CommandResult::Text(format!(
            "Clipboard text:\n{content}\n\nPaste this into the prompt or use the Clipboard tool from the agent loop."
        ))
    }

    fn execute_doctor_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /doctor"));
        }

        let env = std::env::vars().collect::<std::collections::BTreeMap<_, _>>();
        let has_api_key = [
            "DEEPSEEK_API_KEY",
            "ZERO_CLI_API_KEY",
            "ZERO_API_KEY",
            "ANTHROPIC_API_KEY",
        ]
        .iter()
        .any(|key| env.get(*key).is_some_and(|value| !value.is_empty()));
        let base_url = env
            .get("DEEPSEEK_BASE_URL")
            .or_else(|| env.get("ZERO_CLI_BASE_URL"))
            .or_else(|| env.get("ANTHROPIC_BASE_URL"))
            .map(String::as_str)
            .unwrap_or("https://api.deepseek.com/anthropic");

        let mut lines = vec![String::from("Deeptide doctor")];
        lines.push(render_doctor_check(
            "API key",
            if has_api_key { "set" } else { "missing" },
            has_api_key,
        ));
        lines.push(render_doctor_check("Base URL", base_url, true));
        for command in ["git", "rg", "bash"] {
            let path = find_executable(command);
            lines.push(render_doctor_check(
                command,
                path.as_deref().unwrap_or("not found"),
                path.is_some(),
            ));
        }
        lines.push(String::new());
        lines.push(format!(
            "Tools: {} registered",
            self.tool_registry.names().len()
        ));
        lines.push(format!(
            "Commands: {} registered",
            repl_command_sources().len()
        ));
        lines.push(format!("CWD: {}", self.tool_context.cwd.display()));

        CommandResult::Text(lines.join("\n"))
    }

    fn execute_config_command(&self, args: &str) -> CommandResult {
        match args.trim() {
            "" | "show" => CommandResult::Text(render_config_overview(&self.tool_context.cwd)),
            _ => CommandResult::Text(String::from(
                "Usage: /config [show]\nSetting values from the Rust REPL is not available yet; edit the displayed settings files directly.",
            )),
        }
    }

    fn execute_hooks_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /hooks"));
        }

        CommandResult::Text(String::from(
            "No hooks configured. Add a `hooks` block to settings.json when Rust config persistence lands.",
        ))
    }

    fn execute_init_command(&self, args: &str) -> CommandResult {
        let extra = args.trim();
        let mut lines = vec![
            String::from("Project bootstrap is model-driven in Deeptide."),
            String::from(
                "Rust REPL can already inspect the workspace with /context, /memory, /read, /grep, and /glob-backed tools.",
            ),
            String::from(
                "Ask the agent to create or refresh TIDE.md after it scans the repository.",
            ),
        ];
        if !extra.is_empty() {
            lines.push(format!("Extra context: {extra}"));
        }
        CommandResult::Text(lines.join("\n"))
    }

    fn execute_update_command(&self, args: &str) -> CommandResult {
        let parts = args.split_whitespace().collect::<Vec<_>>();
        let allowed = ["--check", "--force"];
        if parts.iter().any(|part| !allowed.contains(part)) {
            return CommandResult::Text(String::from("Usage: /update [--check | --force]"));
        }

        CommandResult::Text(String::from(
            "Update checks are not available in the Rust REPL yet. Run the packaged installer or Swift `tide update` command from your shell.",
        ))
    }

    fn execute_vim_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /vim"));
        }

        CommandResult::Text(String::from(
            "Editor composition is not available in the Rust REPL yet. Use your editor to draft text, then paste it at the prompt.",
        ))
    }

    fn execute_model_command(&mut self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return CommandResult::Text(render_model_status(self.agent_loop.model()));
        }
        if trimmed.split_whitespace().count() > 1 {
            return CommandResult::Text(String::from("Usage: /model <model-name | flash | pro>"));
        }

        let resolved = resolve_model_alias(trimmed);
        self.agent_loop.set_model(resolved.clone());
        if resolved == trimmed {
            CommandResult::Text(format!("Model: {resolved}"))
        } else {
            CommandResult::Text(format!("Model: {resolved} (alias {trimmed})"))
        }
    }

    fn execute_retry_command(&mut self, args: &str) -> Vec<ReplEvent> {
        if !args.trim().is_empty() {
            return vec![ReplEvent::Output(String::from("Usage: /retry"))];
        }

        let Some(prompt) = last_user_prompt(self.agent_loop.messages()) else {
            return vec![ReplEvent::Output(String::from(
                "No previous prompt to retry.",
            ))];
        };

        let preview = truncate_chars(&prompt, 60);
        let mut events = vec![ReplEvent::Output(format!("Retrying: {preview}"))];
        events.extend(
            self.agent_loop
                .run(prompt)
                .into_iter()
                .filter_map(agent_event_to_repl_event),
        );
        events
    }

    fn execute_context_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /context"));
        }

        CommandResult::Text(render_context(
            &self.agent_loop,
            &self.tool_context.cwd,
            &self.additional_dirs,
            self.tool_registry.names(),
        ))
    }

    fn execute_copy_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /copy"));
        }

        let Some(reply) = last_assistant_reply(self.agent_loop.messages()) else {
            return CommandResult::Text(String::from("No assistant reply yet to copy."));
        };

        if reply.is_empty() {
            return CommandResult::Text(String::from(
                "Last assistant turn had no text content (tool calls only).",
            ));
        }

        match (self.clipboard_writer)(&reply) {
            Ok(()) => CommandResult::Text(render_copy_summary(&reply)),
            Err(error) => CommandResult::Text(format!("/copy: {error}")),
        }
    }

    fn execute_export_command(&self, args: &str) -> CommandResult {
        let path = match export_path(args, &self.tool_context.cwd) {
            Ok(path) => path,
            Err(message) => return CommandResult::Text(message),
        };

        let content = render_session_jsonl(self.agent_loop.messages());
        if let Some(parent) = path.parent()
            && let Err(error) = std::fs::create_dir_all(parent)
        {
            return CommandResult::Text(format!(
                "/export: failed to create {}: {error}",
                parent.display()
            ));
        }

        match std::fs::write(&path, content) {
            Ok(()) => CommandResult::Text(format!(
                "Exported {} messages -> {}",
                self.agent_loop.messages().len(),
                path.display()
            )),
            Err(error) => CommandResult::Text(format!(
                "/export: failed to write {}: {error}",
                path.display()
            )),
        }
    }

    fn execute_diff_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /diff"));
        }

        CommandResult::Text(render_workspace_diff(&self.tool_context.cwd))
    }

    fn execute_branch_command(&self, args: &str) -> CommandResult {
        match branch_args(args) {
            Ok(BranchAction::List) => CommandResult::Text(render_git_command(
                &self.tool_context.cwd,
                ["branch"].as_slice(),
                "(no branches)",
            )),
            Ok(BranchAction::Create(name)) => CommandResult::Text(render_git_command(
                &self.tool_context.cwd,
                ["checkout", "-b", name.as_str()].as_slice(),
                "",
            )),
            Ok(BranchAction::Checkout(name)) => CommandResult::Text(render_git_command(
                &self.tool_context.cwd,
                ["checkout", name.as_str()].as_slice(),
                "",
            )),
            Err(message) => CommandResult::Text(message),
        }
    }

    fn execute_add_dir_command(&mut self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() {
            return if self.additional_dirs.is_empty() {
                CommandResult::Text(String::from("No additional dirs."))
            } else {
                CommandResult::Text(
                    self.additional_dirs
                        .iter()
                        .map(|path| format!("  {}", path.display()))
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            };
        }

        if trimmed.split_whitespace().count() > 1 {
            return CommandResult::Text(String::from("Usage: /add-dir <path>"));
        }

        let path = resolve_session_dir(trimmed, &self.tool_context.cwd);
        if !path.is_dir() {
            return CommandResult::Text(format!("Not a directory: {}", path.display()));
        }

        if !self
            .additional_dirs
            .iter()
            .any(|existing| existing == &path)
        {
            self.additional_dirs.push(path.clone());
        }
        CommandResult::Text(format!("Added {}", path.display()))
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

    fn execute_commit_command(&mut self, args: &str) -> Vec<ReplEvent> {
        let result = self.tool_registry.call(
            "Skill",
            serde_json::json!({"skill": "commit", "args": args}),
            &self.tool_context,
        );
        if result.is_error {
            return vec![ReplEvent::Output(format!("/commit: {}", result.content))];
        }
        let mut events = vec![ReplEvent::Output(String::from(
            "Dispatching commit skill to the model.",
        ))];
        events.extend(
            self.agent_loop
                .run(&result.content)
                .into_iter()
                .filter_map(agent_event_to_repl_event),
        );
        events
    }

    fn execute_review_command(&mut self, args: &str) -> Vec<ReplEvent> {
        if args.trim().is_empty() {
            return vec![ReplEvent::Output(String::from(
                "Usage: /review <pr-number-or-url>",
            ))];
        }
        let result = self.tool_registry.call(
            "Skill",
            serde_json::json!({"skill": "review-pr", "args": args}),
            &self.tool_context,
        );
        if result.is_error {
            return vec![ReplEvent::Output(format!("/review: {}", result.content))];
        }
        let mut events = vec![ReplEvent::Output(String::from(
            "Dispatching review-pr skill to the model.",
        ))];
        events.extend(
            self.agent_loop
                .run(&result.content)
                .into_iter()
                .filter_map(agent_event_to_repl_event),
        );
        events
    }

    fn execute_simplify_command(&mut self, args: &str) -> Vec<ReplEvent> {
        let result = self.tool_registry.call(
            "Skill",
            serde_json::json!({"skill": "simplify", "args": args}),
            &self.tool_context,
        );
        if result.is_error {
            return vec![ReplEvent::Output(format!("/simplify: {}", result.content))];
        }
        let mut events = vec![ReplEvent::Output(String::from(
            "Dispatching simplify skill to the model.",
        ))];
        events.extend(
            self.agent_loop
                .run(&result.content)
                .into_iter()
                .filter_map(agent_event_to_repl_event),
        );
        events
    }

    fn execute_skills_command(&self, args: &str) -> CommandResult {
        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /skills"));
        }
        let direct: &[(&str, &str, &str)] = &[
            (
                "commit",
                "/commit",
                "Stage changes, draft message, and commit",
            ),
            ("review-pr", "/review", "Review a GitHub pull request"),
            (
                "simplify",
                "/simplify",
                "Review changed code for quality and efficiency",
            ),
        ];
        let model_only: &[(&str, &str)] = &[
            ("init", "Bootstrap project memory and write TIDE.md"),
            ("batch", "Plan and execute large parallelizable changes"),
            ("publish", "Publish a static frontend on clide.app"),
            ("update-config", "Configure Deeptide CLI settings"),
        ];
        let total = direct.len() + model_only.len();
        let mut lines = vec![
            String::new(),
            format!("Built-in skills ({total}):"),
            String::new(),
        ];
        for (name, cmd, desc) in direct {
            lines.push(format!("  {name:<20} {desc}"));
            lines.push(format!("  {:<20} trigger: {cmd}", ""));
        }
        for (name, desc) in model_only {
            lines.push(format!("  {name:<20} {desc}"));
            lines.push(format!("  {:<20} ask in prose, or use the Skill tool", ""));
        }
        lines.push(String::new());
        lines.push(String::from(
            "Skills expand to a structured prompt the model executes. $ARGUMENTS is replaced with the command tail.",
        ));
        CommandResult::Text(lines.join("\n"))
    }

    fn execute_reminder_command(&mut self, args: &str) -> Vec<ReplEvent> {
        let sub = args.trim().to_ascii_lowercase();
        let text = self.build_reminder_text();
        match sub.as_str() {
            "" | "send" | "now" => {
                let mut events = vec![ReplEvent::Output(String::from(
                    "Queued a short state reminder for the next model turn.",
                ))];
                events.extend(
                    self.agent_loop
                        .run(&text)
                        .into_iter()
                        .filter_map(agent_event_to_repl_event),
                );
                events
            }
            "show" | "print" => vec![ReplEvent::Output(text)],
            _ => vec![ReplEvent::Output(String::from(
                "Usage: /reminder [show|send]",
            ))],
        }
    }

    fn build_reminder_text(&self) -> String {
        let cwd = self.tool_context.cwd.display().to_string();
        let model = self.agent_loop.model().to_owned();
        let tool_names = self.tool_registry.names();
        let preferred = ["Read", "Write", "Edit", "Bash", "Glob", "Grep", "TodoWrite"];
        let available: std::collections::HashSet<&str> = tool_names.iter().copied().collect();
        let listed: Vec<&str> = preferred
            .iter()
            .copied()
            .filter(|t| available.contains(t))
            .collect();
        let tools = if listed.is_empty() {
            preferred.join(", ")
        } else {
            listed.join(", ")
        };
        format!(
            "<system-reminder>\n\
            You are Deeptide, a coding agent.\n\
            cwd: {cwd}\n\
            model: {model}\n\
            You can inspect and modify this workspace through tool calls. Do not claim you cannot access local files.\n\
            Core tools available: {tools}.\n\
            For file questions, call Read/Glob/Grep/Bash as needed. For requested file edits, call Write/Edit instead of printing large code blocks.\n\
            Continue the user's active coding task from the transcript; do not invent unrelated goals or identities.\n\
            </system-reminder>\n\
            Reply briefly that you are re-oriented, then proceed with the user's next instruction."
        )
    }

    fn execute_dream_command(&mut self, args: &str) -> Vec<ReplEvent> {
        let sub = args.trim().to_ascii_lowercase();
        match sub.as_str() {
            "run" | "now" | "" => {
                let cwd = self.tool_context.cwd.display().to_string();
                let prompt = format!(
                    "[dream manual run — execute once, do NOT create cron jobs or loops]\n\n\
                    You are running Deeptide's local dream consolidation pass for workspace:\n\
                    {cwd}\n\n\
                    Goal:\n\
                    - Review recent useful session history.\n\
                    - Extract durable project facts, decisions, preferences, recurring constraints, and unresolved follow-ups.\n\
                    - Save or update concise long-term memory in `.deeptide/MEMORY.md` or project memory files.\n\
                    - Merge duplicate or stale entries instead of appending noise.\n\n\
                    Rules:\n\
                    - Execute exactly once.\n\
                    - Do not edit system prompts, settings, cron jobs, or provider configuration.\n\
                    - Do not add memories that are generic, obvious, temporary, secret, or unsupported by session history.\n\
                    - Keep memory files compact and human-readable."
                );
                let mut events = vec![ReplEvent::Output(String::from(
                    "Queued one dream consolidation run.",
                ))];
                events.extend(
                    self.agent_loop
                        .run(&prompt)
                        .into_iter()
                        .filter_map(agent_event_to_repl_event),
                );
                events
            }
            "status" | "list" => vec![ReplEvent::Output(String::from(
                "No dream loop is active. Use `/dream run` to consolidate session history once.",
            ))],
            "start" | "on" | "enable" => vec![ReplEvent::Output(String::from(
                "Persistent dream loop is not available in the Rust REPL yet. Use `/dream run` to consolidate once.",
            ))],
            "stop" | "off" | "disable" | "cancel" => {
                vec![ReplEvent::Output(String::from("No dream loop is active."))]
            }
            _ => vec![ReplEvent::Output(String::from(
                "Usage: /dream [run | status]",
            ))],
        }
    }

    fn execute_cron_command(&self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        let parts = trimmed.split_whitespace().collect::<Vec<_>>();
        let sub = parts.first().copied().unwrap_or("").to_ascii_lowercase();
        match sub.as_str() {
            "" | "list" | "ls" => {
                let result =
                    self.tool_registry
                        .call("CronList", serde_json::json!({}), &self.tool_context);
                CommandResult::Text(result.content)
            }
            "delete" | "rm" | "remove" => {
                let id = parts.get(1).copied().unwrap_or("");
                if id.is_empty() {
                    return CommandResult::Text(String::from("Usage: /cron delete <id>"));
                }
                let result = self.tool_registry.call(
                    "CronDelete",
                    serde_json::json!({"id": id}),
                    &self.tool_context,
                );
                CommandResult::Text(result.content)
            }
            _ => CommandResult::Text(String::from("Usage: /cron [list | delete <id>]")),
        }
    }

    fn command_context(&self) -> CommandContext {
        let cost_display_enabled = Arc::clone(&self.cost_display_enabled);
        let set_cost_display_enabled = Arc::clone(&self.cost_display_enabled);
        let summary = self.agent_loop.cost_tracker().summary();
        let cwd = self.tool_context.cwd.clone();

        CommandContext::builder()
            .clear_conversation(|| Some(String::new()))
            .compact_conversation(|| {})
            .all_commands(repl_command_sources)
            .cost_summary(move || summary.clone())
            .cost_display_enabled(move || cost_display_enabled.load(Ordering::SeqCst))
            .set_cost_display_enabled(move |enabled| {
                set_cost_display_enabled.store(enabled, Ordering::SeqCst);
            })
            .cwd(move || cwd.clone())
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
            "provider",
            ["profiles"],
            "Show or switch named provider profiles",
            "/provider [list | use <name|deepseek|paean> | status]",
        ),
        CommandCompletionSource::new(
            "model",
            ["m"],
            "Switch the AI model at runtime",
            "/model <model-name | flash | pro>",
        ),
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
            "retry",
            ["r", "again"],
            "Re-submit the last user prompt",
            "/retry",
        ),
        CommandCompletionSource::new(
            "copy",
            ["yank"],
            "Copy the last assistant reply to the clipboard",
            "/copy",
        ),
        CommandCompletionSource::new(
            "export",
            Vec::<&str>::new(),
            "Export session transcript to JSONL",
            "/export [path]",
        ),
        CommandCompletionSource::new(
            "diff",
            Vec::<&str>::new(),
            "Show pending workspace git diff",
            "/diff",
        ),
        CommandCompletionSource::new(
            "branch",
            Vec::<&str>::new(),
            "List git branches; optionally checkout/create",
            "/branch [name | -b name]",
        ),
        CommandCompletionSource::new(
            "add-dir",
            ["add_dir", "adddir"],
            "Add an additional directory to the session context",
            "/add-dir <path>",
        ),
        CommandCompletionSource::new(
            "fast",
            ["faster"],
            "Toggle fast mode (same model, faster output)",
            "/fast",
        ),
        CommandCompletionSource::new(
            "tps",
            ["speed"],
            "Show recorded per-model TPS",
            "/tps [--json | --reset]",
        ),
        CommandCompletionSource::new("debug", ["dbg"], "Toggle debug output", "/debug"),
        CommandCompletionSource::new(
            "keybindings",
            ["keys"],
            "Show current key bindings",
            "/keybindings",
        ),
        CommandCompletionSource::new(
            "sessions",
            ["session"],
            "List saved sessions",
            "/sessions [filter]",
        ),
        CommandCompletionSource::new(
            "resume",
            ["load", "restore"],
            "Resume a previous session",
            "/resume [session-id]",
        ),
        CommandCompletionSource::new(
            "open",
            Vec::<&str>::new(),
            "Allow a sensitive file to be read this session",
            "/open <path>",
        ),
        CommandCompletionSource::new(
            "paste",
            ["p"],
            "Attach or read clipboard content for the next prompt",
            "/paste",
        ),
        CommandCompletionSource::new(
            "doctor",
            Vec::<&str>::new(),
            "Diagnose installation and environment",
            "/doctor",
        ),
        CommandCompletionSource::new(
            "config",
            Vec::<&str>::new(),
            "Show merged settings",
            "/config [show]",
        ),
        CommandCompletionSource::new(
            "hooks",
            Vec::<&str>::new(),
            "List configured hooks",
            "/hooks",
        ),
        CommandCompletionSource::new(
            "init",
            Vec::<&str>::new(),
            "Bootstrap project memory and guide files",
            "/init [extra context]",
        ),
        CommandCompletionSource::new(
            "update",
            ["upgrade"],
            "Update deeptide to the latest published version",
            "/update [--check | --force]",
        ),
        CommandCompletionSource::new(
            "vim",
            ["edit", "e", "compose"],
            "Open $EDITOR for the next prompt",
            "/vim",
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
        CommandCompletionSource::new(
            "commit",
            Vec::<&str>::new(),
            "Run the commit skill (stage changes, draft message, commit)",
            "/commit [extra context]",
        ),
        CommandCompletionSource::new(
            "review",
            Vec::<&str>::new(),
            "Run the review-pr skill on a GitHub PR",
            "/review <pr-number-or-url>",
        ),
        CommandCompletionSource::new(
            "simplify",
            Vec::<&str>::new(),
            "Review changed code for reuse, quality, and efficiency",
            "/simplify [extra context]",
        ),
        CommandCompletionSource::new(
            "skills",
            ["skill"],
            "List available built-in skills",
            "/skills",
        ),
        CommandCompletionSource::new(
            "reminder",
            ["anchor", "reorient"],
            "Re-anchor the agent's cwd/model/tool state",
            "/reminder [show|send]",
        ),
        CommandCompletionSource::new(
            "dream",
            Vec::<&str>::new(),
            "Consolidate session history into local long-term memory",
            "/dream [run | status]",
        ),
        CommandCompletionSource::new(
            "cron",
            Vec::<&str>::new(),
            "Manage scheduled cron jobs",
            "/cron [list | delete <id>]",
        ),
    ]
}

fn unquote_path(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        trimmed[1..trimmed.len() - 1].to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn render_doctor_check(label: &str, value: &str, ok: bool) -> String {
    let mark = if ok { "ok" } else { "missing" };
    format!("  {mark:<7} {label:<9} {value}")
}

fn find_executable(command: &str) -> Option<String> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate.display().to_string());
        }

        #[cfg(windows)]
        {
            for extension in ["exe", "cmd", "bat"] {
                let candidate = dir.join(format!("{command}.{extension}"));
                if candidate.is_file() {
                    return Some(candidate.display().to_string());
                }
            }
        }
    }
    None
}

fn render_config_overview(cwd: &std::path::Path) -> String {
    let mut lines = vec![String::from("Settings files:")];
    for (label, path) in candidate_config_files(cwd) {
        let status = if path.exists() { "present" } else { "missing" };
        lines.push(format!("  {label:<8} {status:<7} {}", path.display()));
    }
    lines.push(String::new());
    lines.push(String::from(
        "Rust config editing is intentionally read-only for now; align cloud API behavior with zero-cli launch environment variables.",
    ));
    lines.join("\n")
}

fn candidate_config_files(cwd: &std::path::Path) -> Vec<(&'static str, std::path::PathBuf)> {
    let mut files = vec![
        ("project", cwd.join(".deeptide").join("settings.json")),
        ("local", cwd.join(".deeptide").join("settings.local.json")),
    ];
    if let Some(home) = repl_home_dir() {
        files.push((
            "global",
            home.join(".config").join("tide").join("settings.json"),
        ));
    }
    files
}

fn repl_home_dir() -> Option<std::path::PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(std::path::PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(std::path::PathBuf::from)
    }
}

fn render_status(
    agent_loop: &AgentLoop,
    cwd: &std::path::Path,
    additional_dirs: &[std::path::PathBuf],
    provider_profile: ProviderProfile,
) -> String {
    let summary = agent_loop.cost_tracker().summary();
    let cache = summary.cache_health();
    let context_tokens = estimate_repl_context_tokens(agent_loop.messages());
    let branch = git_branch(cwd).unwrap_or_else(|| String::from("(no git)"));
    let cache_summary = render_cache_health(&cache);

    let mut lines = vec![
        String::from("Deeptide session status"),
        format!("  Model:    {}", agent_loop.model()),
        format!("  CWD:      {}", cwd.display()),
        format!("  + dirs:   {}", render_additional_dirs(additional_dirs)),
        format!("  Branch:   {branch}"),
        format!("  Provider: {}", provider_profile.name()),
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

fn render_context(
    agent_loop: &AgentLoop,
    cwd: &std::path::Path,
    additional_dirs: &[std::path::PathBuf],
    tool_names: Vec<&str>,
) -> String {
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
        format!("  + dirs:   {}", render_additional_dirs(additional_dirs)),
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

fn resolve_session_dir(raw: &str, cwd: &std::path::Path) -> std::path::PathBuf {
    let expanded = if raw == "~" {
        home_dir().unwrap_or_else(|| std::path::PathBuf::from(raw))
    } else if let Some(rest) = raw.strip_prefix("~/") {
        home_dir()
            .map(|home| home.join(rest))
            .unwrap_or_else(|| std::path::PathBuf::from(raw))
    } else {
        std::path::PathBuf::from(raw)
    };

    if expanded.is_absolute() {
        expanded
    } else {
        cwd.join(expanded)
    }
}

fn render_additional_dirs(additional_dirs: &[std::path::PathBuf]) -> String {
    if additional_dirs.is_empty() {
        String::from("(none)")
    } else {
        additional_dirs
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }
}

fn render_model_status(current_model: &str) -> String {
    let mut lines = vec![
        format!("Current model: {current_model}"),
        String::new(),
        String::from("Aliases:"),
    ];
    lines.extend(model_alias_summary().into_iter().map(String::from));
    lines.push(String::new());
    lines.push(String::from(
        "Usage: /model <name-or-alias>  (e.g. /model flash, /model pro)",
    ));
    lines.join("\n")
}

fn render_provider_status(active: ProviderProfile) -> String {
    let mut lines = vec![
        format!("Current session provider profile: {}", active.name()),
        String::from("Provider selection is recorded in this REPL session."),
    ];
    if active == ProviderProfile::Legacy {
        lines.push(String::from(
            "Effective cloud settings still follow launch-time DEEPTIDE_*, ZERO_CLI_*, and ANTHROPIC_* environment resolution.",
        ));
    } else {
        lines.push(String::from(
            "The existing model client was created at launch; restart or relaunch with matching provider settings to change HTTP connection details.",
        ));
    }
    lines.push(String::from(
        "Use /provider list to inspect built-in profiles.",
    ));
    lines.into_iter().collect::<Vec<_>>().join("\n")
}

fn render_provider_list(active: ProviderProfile) -> String {
    let mut lines = vec![String::from("Provider profiles:")];
    for (profile, base_url, note) in [
        (
            ProviderProfile::DeepSeek,
            "https://api.deepseek.com",
            "built-in DeepSeek official endpoint",
        ),
        (
            ProviderProfile::Paean,
            "https://api.paean.ai",
            "built-in Paean AI gateway",
        ),
        (
            ProviderProfile::Legacy,
            "(launch environment)",
            "DEEPTIDE_*, ZERO_CLI_*, and ANTHROPIC_* resolution",
        ),
    ] {
        let mark = if profile == active { "*" } else { " " };
        lines.push(format!(" {mark} {}  {base_url} - {note}", profile.name()));
    }
    lines.join("\n")
}

fn resolve_model_alias(name: &str) -> String {
    match name.trim().to_ascii_lowercase().as_str() {
        "pro" | "v4-pro" | "v4" => String::from("deepseek-v4-pro"),
        "flash" | "fast" | "v4-flash" => String::from("deepseek-v4-flash"),
        "flash-q4" | "flash-q4k" | "v4-flash-q4" | "v4-flash-q4k" => {
            String::from("deepseek-v4-flash-q4k")
        }
        _ => name.to_owned(),
    }
}

fn model_alias_summary() -> Vec<&'static str> {
    vec![
        "  deepseek-v4-flash <- fast, flash, v4-flash",
        "  deepseek-v4-flash-q4k <- flash-q4, flash-q4k, v4-flash-q4, v4-flash-q4k",
        "  deepseek-v4-pro <- pro, v4, v4-pro",
    ]
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

fn last_user_prompt(messages: &[ConversationMessage]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|message| message.role == MessageRole::User)
        .map(|message| message.content.trim().to_owned())
        .filter(|content| !content.is_empty())
}

fn last_assistant_reply(messages: &[ConversationMessage]) -> Option<String> {
    messages
        .iter()
        .rev()
        .find(|message| message.role == MessageRole::Assistant)
        .map(|message| message.content.trim().to_owned())
}

fn render_copy_summary(content: &str) -> String {
    let chars = content.chars().count();
    let lines = content.split('\n').count();
    let char_label = if chars == 1 { "char" } else { "chars" };
    let line_label = if lines == 1 { "line" } else { "lines" };
    format!("Copied last reply to clipboard ({chars} {char_label}, {lines} {line_label}).")
}

fn write_to_system_clipboard(content: &str) -> Result<(), String> {
    let context =
        ToolContext::new(std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")));
    let result = ClipboardTool.call(
        serde_json::json!({
            "operation": "write",
            "content": content,
        }),
        &context,
    );
    if result.is_error {
        Err(result.content)
    } else {
        Ok(())
    }
}

fn export_path(args: &str, cwd: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let trimmed = args.trim();
    if trimmed.split_whitespace().count() > 1 {
        return Err(String::from("Usage: /export [path]"));
    }

    let raw = if trimmed.is_empty() {
        std::env::temp_dir().join("deeptide-session.jsonl")
    } else if trimmed == "~" {
        home_dir().ok_or_else(|| String::from("/export: could not resolve home directory"))?
    } else if let Some(rest) = trimmed.strip_prefix("~/") {
        home_dir()
            .ok_or_else(|| String::from("/export: could not resolve home directory"))?
            .join(rest)
    } else {
        std::path::PathBuf::from(trimmed)
    };

    if raw.is_absolute() {
        Ok(raw)
    } else {
        Ok(cwd.join(raw))
    }
}

fn render_session_jsonl(messages: &[ConversationMessage]) -> String {
    let mut lines = messages
        .iter()
        .map(|message| {
            let role = match message.role {
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
            };
            serde_json::json!({
                "type": role,
                "message": {
                    "role": role,
                    "content": message.content,
                },
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    lines.push('\n');
    lines
}

fn render_workspace_diff(cwd: &std::path::Path) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["diff", "--"])
        .output();

    let Ok(output) = output else {
        return String::from("/diff: failed to run git diff");
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if stderr.is_empty() {
            return format!("/diff: git diff exited with status {}", output.status);
        }
        return format!("/diff: {stderr}");
    }

    let diff = String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned();
    if diff.is_empty() {
        String::from("No pending git diff in workspace.")
    } else {
        format!("Pending workspace diff:\n{diff}")
    }
}

enum BranchAction {
    List,
    Create(String),
    Checkout(String),
}

fn branch_args(args: &str) -> Result<BranchAction, String> {
    let trimmed = args.trim();
    if trimmed.is_empty() {
        return Ok(BranchAction::List);
    }

    if trimmed == "-b" {
        return Err(String::from("Usage: /branch [name | -b name]"));
    }

    if let Some(raw) = trimmed.strip_prefix("-b ") {
        let name = raw.trim();
        if name.is_empty() || name.split_whitespace().count() > 1 {
            return Err(String::from("Usage: /branch [name | -b name]"));
        }
        return Ok(BranchAction::Create(name.to_owned()));
    }

    if trimmed.split_whitespace().count() > 1 {
        return Err(String::from("Usage: /branch [name | -b name]"));
    }

    Ok(BranchAction::Checkout(trimmed.to_owned()))
}

fn render_git_command(cwd: &std::path::Path, args: &[&str], empty_message: &str) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output();

    let Ok(output) = output else {
        return String::from("git: failed to run git");
    };

    let stdout = String::from_utf8_lossy(&output.stdout)
        .trim_end()
        .to_owned();
    let stderr = String::from_utf8_lossy(&output.stderr)
        .trim_end()
        .to_owned();
    let combined = match (stdout.is_empty(), stderr.is_empty()) {
        (false, false) => format!("{stdout}\n{stderr}"),
        (false, true) => stdout,
        (true, false) => stderr,
        (true, true) => String::new(),
    };

    if combined.is_empty() {
        empty_message.to_owned()
    } else {
        combined
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
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
