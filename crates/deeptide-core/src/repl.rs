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
    prompt::build_system_prompt,
    session::{SessionStore, new_session_id},
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
    fast_mode: bool,
    tps_samples: Vec<crate::tps::TpsSample>,
    tps_store_dir: Option<std::path::PathBuf>,
    active_goal: Option<String>,
    session_id: String,
    session_started_at: String,
    /// In-session "dream" auto-consolidation schedule. When enabled, a dream
    /// consolidation pass fires automatically every N user turns. Persisted
    /// only for the lifetime of this REPL — restarting `deeptide` resets to
    /// disabled. zero-cli's equivalent (the Dream task) also defaults off.
    dream_schedule: DreamSchedule,
    /// Number of user turns submitted in this REPL session. Counted only for
    /// non-empty, non-slash submissions — slash commands like `/help` should
    /// not advance the dream cadence counter.
    user_turn_count: usize,
    /// Whether conversation turns are autosaved to disk. Disabled by
    /// `--no-session-persistence` for privacy / scratch sessions.
    session_persistence: bool,
}

/// Configuration + bookkeeping for the persistent dream loop. Default state
/// is disabled; `/dream start [--every N]` enables, `/dream stop` disables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DreamSchedule {
    enabled: bool,
    /// Fire after this many user turns since `last_user_turn_count`. Clamped
    /// to `MIN_DREAM_CADENCE..=MAX_DREAM_CADENCE` on input.
    every_user_turns: usize,
    /// Snapshot of `user_turn_count` the last time a dream pass fired (or
    /// when `start` was called). The next fire happens when the difference
    /// reaches `every_user_turns`.
    last_user_turn_count: usize,
    /// Total number of automatic dream passes that have fired in this
    /// session. Used by `/dream status` for the operator.
    total_auto_runs: usize,
}

const MIN_DREAM_CADENCE: usize = 1;
const MAX_DREAM_CADENCE: usize = 500;
const DEFAULT_DREAM_CADENCE: usize = 25;

impl Default for DreamSchedule {
    fn default() -> Self {
        Self {
            enabled: false,
            every_user_turns: DEFAULT_DREAM_CADENCE,
            last_user_turn_count: 0,
            total_auto_runs: 0,
        }
    }
}

impl ReplSession {
    pub fn new(backend: Box<dyn AgentBackend>) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let system_prompt = build_system_prompt(&cwd);
        Self {
            agent_loop: AgentLoop::new(backend).with_system_prompt(system_prompt),
            cost_display_enabled: Arc::new(AtomicBool::new(false)),
            tool_registry: ToolRegistry::with_builtin_tools(),
            tool_context: ToolContext::new(cwd),
            clipboard_writer: Arc::new(write_to_system_clipboard),
            additional_dirs: Vec::new(),
            provider_profile: ProviderProfile::Legacy,
            debug_enabled: false,
            fast_mode: false,
            tps_samples: Vec::new(),
            tps_store_dir: None,
            active_goal: None,
            session_id: new_session_id(),
            session_started_at: time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z")),
            dream_schedule: DreamSchedule::default(),
            user_turn_count: 0,
            session_persistence: true,
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.agent_loop = self.agent_loop.with_model(model);
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<std::path::PathBuf>) -> Self {
        self.tool_context = ToolContext::new(cwd);
        let system_prompt = build_system_prompt(&self.tool_context.cwd);
        self.agent_loop = self
            .agent_loop
            .with_cwd(self.tool_context.cwd.clone())
            .with_system_prompt(system_prompt);
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

    /// Restrict which tools the agent may call (from `--allowed-tools` /
    /// `--disallowed-tools`). A `None` allowlist with an empty denylist leaves
    /// every tool available.
    pub fn with_tool_restrictions(
        mut self,
        allowed: Option<Vec<String>>,
        disallowed: Vec<String>,
    ) -> Self {
        self.agent_loop = self.agent_loop.with_tool_restrictions(allowed, disallowed);
        self
    }

    /// Enable or disable autosaving conversation turns to disk. Disabled by
    /// `--no-session-persistence`.
    pub fn with_session_persistence(mut self, enabled: bool) -> Self {
        self.session_persistence = enabled;
        self
    }

    /// Pre-register additional context directories (from `--add-dir`), resolving
    /// each against the session cwd. Non-directories and duplicates are skipped.
    /// Mirrors registering them via the `/add-dir` command at startup.
    pub fn with_additional_dirs(mut self, dirs: &[std::path::PathBuf]) -> Self {
        for dir in dirs {
            let resolved = resolve_session_dir(&dir.to_string_lossy(), &self.tool_context.cwd);
            if resolved.is_dir() && !self.additional_dirs.contains(&resolved) {
                self.additional_dirs.push(resolved);
            }
        }
        self
    }

    /// Append `text` after the current system prompt (from
    /// `--append-system-prompt`). Empty/whitespace text is ignored.
    pub fn with_appended_system_prompt(mut self, text: &str) -> Self {
        let text = text.trim();
        if !text.is_empty() {
            let base = self
                .agent_loop
                .system_prompt()
                .unwrap_or_default()
                .to_owned();
            let combined = if base.is_empty() {
                text.to_owned()
            } else {
                format!("{base}\n\n{text}")
            };
            self.agent_loop = self.agent_loop.with_system_prompt(combined);
        }
        self
    }

    /// Load a saved session's history into context and continue it (subsequent
    /// autosaves write back to the same session id). Returns the number of
    /// restored messages. Used by `--resume` / `--continue` at startup.
    pub fn resume_session(&mut self, session_id: &str) -> Result<usize, String> {
        let messages = SessionStore::load(&self.tool_context.cwd, session_id)?;
        let count = messages.len();
        self.agent_loop.restore_messages(messages);
        self.session_id = session_id.to_owned();
        Ok(count)
    }

    pub fn with_pricing_overrides(
        mut self,
        overrides: std::collections::HashMap<String, crate::ModelPricing>,
    ) -> Self {
        self.agent_loop = self.agent_loop.with_pricing_overrides(overrides);
        self
    }

    /// Set the initial debug-output state (the `--debug` flag / `debug`
    /// config). When enabled, each prompt is followed by per-turn token and
    /// cost diagnostics; `/debug` toggles it at runtime.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug_enabled = debug;
        self
    }

    /// Enable fast mode (the `--fast` flag / `fast_mode` config). Same model,
    /// biased toward faster/terser output via a system-prompt hint, mirroring
    /// the Swift implementation's fast-mode prompt section.
    pub fn with_fast_mode(mut self, fast: bool) -> Self {
        self.fast_mode = fast;
        if fast {
            let base = self.agent_loop.system_prompt().unwrap_or("").to_owned();
            let prompt = format!("{base}\n\n{FAST_MODE_PROMPT}");
            self.agent_loop = self.agent_loop.with_system_prompt(prompt);
        }
        self
    }

    /// Persist TPS samples to `dir` so `/tps` reports throughput across
    /// sessions. When unset (the default), `/tps` reflects only the current
    /// session's in-memory samples.
    pub fn with_tps_store_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.tps_store_dir = Some(dir.into());
        self
    }

    /// Install the lifecycle hook engine (built from `settings.json` hooks) so
    /// PreToolUse/PostToolUse hooks fire around tool calls.
    pub fn with_hooks(mut self, hooks: crate::hooks::HookEngine) -> Self {
        self.agent_loop = self.agent_loop.with_hooks(hooks);
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

        self.user_turn_count += 1;
        let turns_before = self.agent_loop.cost_tracker().summary().turns.len();
        let mut events: Vec<ReplEvent> = self
            .agent_loop
            .run(trimmed)
            .into_iter()
            .filter_map(agent_event_to_repl_event)
            .collect();

        // Record per-turn throughput and (optionally) emit debug diagnostics
        // for the turns this run produced, reading the cost summary once.
        let summary = self.agent_loop.cost_tracker().summary();
        let new_turns = summary.turns.get(turns_before..).unwrap_or(&[]);
        for turn in new_turns {
            if turn.output_tokens > 0 && turn.duration_ms > 0 {
                let sample = crate::tps::TpsSample {
                    model: turn.model.clone(),
                    output_tokens: turn.output_tokens,
                    duration_ms: turn.duration_ms,
                };
                if let Some(dir) = &self.tps_store_dir {
                    crate::tps::TpsStore::record(dir, &sample);
                }
                self.tps_samples.push(sample);
            }
        }
        if self.debug_enabled
            && let Some(debug) = format_debug_turns(new_turns)
        {
            events.push(ReplEvent::Output(debug));
        }

        if let Some(dream_events) = self.maybe_run_scheduled_dream() {
            events.extend(dream_events);
        }

        self.autosave_session();
        events
    }

    /// If the persistent dream loop is enabled and the user-turn cadence has
    /// been met since the last fire, run one consolidation pass synchronously
    /// and return its events. Returns `None` when no fire is due.
    ///
    /// Synchronous (rather than threaded) because the agent loop is `!Sync`
    /// and currently the REPL drives a single backend. The user-visible
    /// effect is one extra round-trip after every Nth user message — which
    /// matches what an operator typing `/dream run` themselves would do.
    fn maybe_run_scheduled_dream(&mut self) -> Option<Vec<ReplEvent>> {
        if !self.dream_schedule.enabled {
            return None;
        }
        let since = self
            .user_turn_count
            .saturating_sub(self.dream_schedule.last_user_turn_count);
        if since < self.dream_schedule.every_user_turns {
            return None;
        }
        self.dream_schedule.last_user_turn_count = self.user_turn_count;
        self.dream_schedule.total_auto_runs += 1;

        let mut events = vec![ReplEvent::Output(format!(
            "[dream] auto-consolidating after {} user turns (run #{})",
            self.dream_schedule.every_user_turns, self.dream_schedule.total_auto_runs,
        ))];
        events.extend(self.run_dream_consolidation_once());
        Some(events)
    }

    /// Build the dream-consolidation system prompt and run one agent pass.
    /// Shared by both manual `/dream run` and the scheduled auto-fire path.
    fn run_dream_consolidation_once(&mut self) -> Vec<ReplEvent> {
        let cwd = self.tool_context.cwd.display().to_string();
        let prompt = format!(
            "[dream consolidation — execute once, do NOT create cron jobs or loops]\n\n\
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
        self.agent_loop
            .run(&prompt)
            .into_iter()
            .filter_map(agent_event_to_repl_event)
            .collect()
    }

    fn autosave_session(&self) {
        if !self.session_persistence {
            return;
        }
        SessionStore::save(
            &self.tool_context.cwd,
            &self.session_id,
            self.agent_loop.model(),
            &self.session_started_at,
            self.agent_loop.messages(),
        );
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

    /// Return all registered slash-command sources.
    ///
    /// Exposes the list so embedders (e.g. the CLI readline helper) can build
    /// tab-completion candidates without depending on the private
    /// `repl_command_sources` function.
    pub fn command_sources(&self) -> Vec<CommandCompletionSource> {
        repl_command_sources()
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
            "compact" | "compress" => self.execute_compact_command(args),
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
            "vim" | "edit" | "e" | "compose" => return self.execute_vim_command(args),
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
            "goal" | "objective" => return self.execute_goal_command(args),
            "cache" | "kvcache" | "manifest" => self.execute_cache_command(args),
            _ => CommandResult::Text(crate::commands::render_unknown_command(
                &name,
                &context.all_commands(),
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
            &self.session_id,
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

        if self.fast_mode {
            CommandResult::Text(String::from(
                "Fast mode is ON: same model, biased toward faster/terser output. \
                 Set at launch with --fast or `fast_mode: true` in settings.json.",
            ))
        } else {
            CommandResult::Text(String::from(
                "Fast mode is OFF. Enable it with --fast at launch or `fast_mode: true` \
                 in settings.json (same model, faster/terser output).",
            ))
        }
    }

    fn execute_tps_command(&mut self, args: &str) -> CommandResult {
        let flags = args.split_whitespace().collect::<Vec<_>>();
        if flags
            .iter()
            .any(|flag| !matches!(*flag, "--json" | "--reset"))
        {
            return CommandResult::Text(String::from("Usage: /tps [--json | --reset]"));
        }

        if flags.contains(&"--reset") {
            let cleared = self.tps_samples.len();
            self.tps_samples.clear();
            if let Some(dir) = &self.tps_store_dir {
                crate::tps::TpsStore::reset(dir);
            }
            return CommandResult::Text(format!("Cleared {cleared} TPS sample(s)."));
        }

        // Prefer the persisted (cross-session) store when configured, otherwise
        // fall back to this session's in-memory samples.
        let records = match &self.tps_store_dir {
            Some(dir) => crate::tps::TpsStore::load(dir),
            None => crate::tps::aggregate(&self.tps_samples),
        };
        if flags.contains(&"--json") {
            CommandResult::Text(crate::tps::to_json(&records))
        } else {
            CommandResult::Text(crate::tps::render(&records))
        }
    }

    fn execute_compact_command(&mut self, _args: &str) -> CommandResult {
        let report = self.agent_loop.compact();
        let text = if report.did_compress {
            format!(
                "Context compacted: folded {} message(s) into a summary; ~{} tokens remain.",
                report.compressed_messages, report.tokens_after
            )
        } else {
            format!(
                "Nothing to compact yet (~{} tokens; the transcript fits the recent window).",
                report.tokens_after
            )
        };
        CommandResult::Text(text)
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

        let entries = SessionStore::list(&self.tool_context.cwd);
        if entries.is_empty() {
            return CommandResult::Text(String::from(
                "No saved sessions for this project. Sessions are saved automatically on each turn.",
            ));
        }

        let filter = args.trim().to_ascii_lowercase();
        let shown: Vec<_> = entries
            .iter()
            .filter(|e| {
                filter.is_empty()
                    || e.session_id.contains(&filter)
                    || e.preview.to_ascii_lowercase().contains(&filter)
            })
            .take(20)
            .collect();

        if shown.is_empty() {
            return CommandResult::Text(format!("No sessions match filter: {filter}"));
        }

        let mut lines = vec![format!("Sessions ({} saved):", shown.len())];
        for entry in &shown {
            let project = std::path::Path::new(&entry.cwd)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&entry.cwd);
            let preview = if entry.preview.is_empty() {
                "(empty)"
            } else {
                &entry.preview
            };
            lines.push(format!(
                "  {}  {}  \"{}\"  ({} messages)",
                entry.session_id, project, preview, entry.message_count
            ));
        }
        lines.push(String::new());
        lines.push(String::from("Use /resume <session-id> to restore."));
        CommandResult::Text(lines.join("\n"))
    }

    fn execute_resume_command(&mut self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.split_whitespace().count() > 1 {
            return CommandResult::Text(String::from("Usage: /resume [session-id]"));
        }

        if trimmed.is_empty() {
            return self.execute_sessions_command("");
        }

        match SessionStore::load(&self.tool_context.cwd, trimmed) {
            Ok(messages) => {
                let count = messages.len();
                self.agent_loop.restore_messages(messages);
                CommandResult::Text(format!(
                    "Resumed session {trimmed}: loaded {count} messages into context."
                ))
            }
            Err(e) => CommandResult::Text(format!("Cannot resume: {e}")),
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

        if crate::sensitive_file::is_sensitive(&path) {
            crate::sensitive_file::mark_open(&path);
            CommandResult::Text(format!(
                "Opened {} for this session; file-read tools may now read it.",
                path.display()
            ))
        } else {
            CommandResult::Text(format!(
                "{} is not classified as sensitive in the Rust build; normal tools can already read it.",
                path.display()
            ))
        }
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
        use crate::config::ConfigStore;

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
        // Settings layers (effective): which config files contribute, lowest to
        // highest precedence. Mirrors Swift's `settingsLayers()` listing.
        let cwd = &self.tool_context.cwd;
        let settings = ConfigStore::load(cwd);
        lines.push(String::new());
        lines.push(String::from("Settings layers (effective):"));
        for (layer, path) in [
            ("global", ConfigStore::global_path()),
            ("project", ConfigStore::project_path(cwd)),
            ("local", ConfigStore::local_path(cwd)),
        ] {
            let status = if path.is_file() {
                path.display().to_string()
            } else {
                format!("{} (absent)", path.display())
            };
            lines.push(format!("  {layer:<8} {status}"));
        }

        // Permissions, hooks, and MCP servers from the merged settings.
        lines.push(String::new());
        lines.push(String::from("Permissions:"));
        let (allow, deny) = settings
            .permissions
            .as_ref()
            .map(|p| {
                (
                    p.allow.as_ref().map_or(0, Vec::len),
                    p.deny.as_ref().map_or(0, Vec::len),
                )
            })
            .unwrap_or((0, 0));
        lines.push(format!("  {allow} allow, {deny} deny rules"));

        lines.push(String::from("Hooks:"));
        let h = settings.hooks.unwrap_or_default();
        lines.push(format!(
            "  pre-tool={} post-tool={} prompt={} start={} end={} compact={}",
            h.pre_tool_use.as_ref().map_or(0, Vec::len),
            h.post_tool_use.as_ref().map_or(0, Vec::len),
            h.user_prompt_submit.as_ref().map_or(0, Vec::len),
            h.session_start.as_ref().map_or(0, Vec::len),
            h.session_end.as_ref().map_or(0, Vec::len),
            h.pre_compact.as_ref().map_or(0, Vec::len),
        ));

        lines.push(format!(
            "MCP servers: {}",
            settings.mcp_servers.as_ref().map_or(0, |m| m.len())
        ));

        lines.push(String::new());
        lines.push(format!(
            "Tools: {} registered",
            self.tool_registry.names().len()
        ));
        lines.push(format!(
            "Commands: {} registered",
            repl_command_sources().len()
        ));
        lines.push(format!("CWD: {}", cwd.display()));

        CommandResult::Text(lines.join("\n"))
    }

    fn execute_config_command(&self, args: &str) -> CommandResult {
        use crate::config::{ConfigScope, ConfigStore};

        let trimmed = args.trim();
        if trimmed.is_empty() || trimmed == "show" {
            return CommandResult::Text(ConfigStore::show(&self.tool_context.cwd));
        }

        // /config set key=value [--project | --local]
        if let Some(rest) = trimmed.strip_prefix("set ") {
            let rest = rest.trim();
            let (rest, scope) = if let Some(r) = rest.strip_suffix("--project") {
                (r.trim(), ConfigScope::Project)
            } else if let Some(r) = rest.strip_suffix("--local") {
                (r.trim(), ConfigScope::Local)
            } else {
                (rest, ConfigScope::Global)
            };

            let Some((key, value)) = rest.split_once('=') else {
                return CommandResult::Text(String::from(
                    "Usage: /config set key=value [--project | --local]",
                ));
            };
            let (key, value) = (key.trim(), value.trim());
            let path = ConfigStore::scope_path(scope, &self.tool_context.cwd);
            return match ConfigStore::set_value(key, value, &path) {
                Ok(()) => CommandResult::Text(format!("Set {key}={value} in {}", path.display())),
                Err(e) => CommandResult::Text(format!("Error: {e}")),
            };
        }

        // /config unset key [--project | --local]
        if let Some(rest) = trimmed.strip_prefix("unset ") {
            let rest = rest.trim();
            let (key, scope) = if let Some(k) = rest.strip_suffix("--project") {
                (k.trim(), ConfigScope::Project)
            } else if let Some(k) = rest.strip_suffix("--local") {
                (k.trim(), ConfigScope::Local)
            } else {
                (rest, ConfigScope::Global)
            };
            let path = ConfigStore::scope_path(scope, &self.tool_context.cwd);
            return match ConfigStore::unset_value(key, &path) {
                Ok(()) => CommandResult::Text(format!("Removed {key} from {}", path.display())),
                Err(e) => CommandResult::Text(format!("Error: {e}")),
            };
        }

        CommandResult::Text(String::from(
            "Usage: /config [show | set key=value [--project|--local] | unset key [--project|--local]]",
        ))
    }

    fn execute_hooks_command(&self, args: &str) -> CommandResult {
        use crate::config::ConfigStore;

        if !args.trim().is_empty() {
            return CommandResult::Text(String::from("Usage: /hooks"));
        }

        let hooks = ConfigStore::load(&self.tool_context.cwd).hooks;
        let Some(hooks) = hooks else {
            return CommandResult::Text(String::from(
                "No hooks configured in settings.json. Add a `hooks` block to enable pre/post-tool hooks.",
            ));
        };

        let mut lines = vec![String::from("Configured hooks:")];
        let mut add = |event: &str, entries: &[crate::config::HookEntry]| {
            for h in entries {
                if h.is_disabled() {
                    continue;
                }
                let name = h.name.as_deref().unwrap_or("(unnamed)");
                lines.push(format!(
                    "  {event:<18} {name:<20} matcher={} timeout={}ms",
                    h.matcher,
                    h.effective_timeout_ms()
                ));
                lines.push(format!("    command: {}", h.command));
            }
        };

        if let Some(ref v) = hooks.pre_tool_use {
            add("PreToolUse", v);
        }
        if let Some(ref v) = hooks.post_tool_use {
            add("PostToolUse", v);
        }
        if let Some(ref v) = hooks.user_prompt_submit {
            add("UserPromptSubmit", v);
        }
        if let Some(ref v) = hooks.session_start {
            add("SessionStart", v);
        }
        if let Some(ref v) = hooks.session_end {
            add("SessionEnd", v);
        }
        if let Some(ref v) = hooks.pre_compact {
            add("PreCompact", v);
        }

        if lines.len() == 1 {
            lines.push(String::from("  (all hooks are disabled)"));
        }
        CommandResult::Text(lines.join("\n"))
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

    fn execute_vim_command(&mut self, args: &str) -> Vec<ReplEvent> {
        if !args.trim().is_empty() {
            return vec![ReplEvent::Output(String::from("Usage: /vim"))];
        }

        let editor = std::env::var("VISUAL")
            .or_else(|_| std::env::var("EDITOR"))
            .unwrap_or_else(|_| default_editor());

        let tmp_path = vim_temp_path();
        if let Err(error) = std::fs::write(&tmp_path, b"") {
            return vec![ReplEvent::Output(format!(
                "/vim: cannot create temp file {}: {error}",
                tmp_path.display()
            ))];
        }

        let status = std::process::Command::new(&editor).arg(&tmp_path).status();
        let content = std::fs::read_to_string(&tmp_path).unwrap_or_default();
        let _ = std::fs::remove_file(&tmp_path);

        match status {
            Err(error) => {
                return vec![ReplEvent::Output(format!(
                    "/vim: cannot launch '{editor}': {error}\n\
                    Set $EDITOR or $VISUAL to your preferred editor."
                ))];
            }
            Ok(status) if !status.success() => {
                let code = status
                    .code()
                    .map_or_else(|| String::from("signal"), |c| c.to_string());
                return vec![ReplEvent::Output(format!(
                    "/vim: editor exited with status {code}; discarding content."
                ))];
            }
            Ok(_) => {}
        }

        let composed = content.trim().to_owned();
        if composed.is_empty() {
            return vec![ReplEvent::Output(String::from(
                "/vim: empty content, nothing submitted.",
            ))];
        }

        let char_count = composed.chars().count();
        let line_count = composed.lines().count();
        let mut events = vec![ReplEvent::Output(format!(
            "Submitting composed text ({char_count} chars, {line_count} lines)."
        ))];
        events.extend(
            self.agent_loop
                .run(&composed)
                .into_iter()
                .filter_map(agent_event_to_repl_event),
        );
        self.autosave_session();
        events
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
        let trimmed = args.trim();
        // Parse "<verb> [--every N | N]" by splitting once on whitespace —
        // dream verbs themselves are single tokens.
        let (verb_raw, rest) = match trimmed.split_once(char::is_whitespace) {
            Some((v, r)) => (v, r.trim()),
            None => (trimmed, ""),
        };
        let verb = verb_raw.to_ascii_lowercase();
        match verb.as_str() {
            "run" | "now" | "" => {
                let mut events = vec![ReplEvent::Output(String::from(
                    "Queued one dream consolidation run.",
                ))];
                events.extend(self.run_dream_consolidation_once());
                events
            }
            "status" | "list" => {
                if self.dream_schedule.enabled {
                    let remaining = self.dream_schedule.every_user_turns.saturating_sub(
                        self.user_turn_count
                            .saturating_sub(self.dream_schedule.last_user_turn_count),
                    );
                    vec![ReplEvent::Output(format!(
                        "Dream loop ENABLED. Cadence: every {} user turns. \
                         Auto-runs this session: {}. Next auto-run in {} more user turns.",
                        self.dream_schedule.every_user_turns,
                        self.dream_schedule.total_auto_runs,
                        remaining,
                    ))]
                } else {
                    vec![ReplEvent::Output(String::from(
                        "Dream loop DISABLED. Use `/dream start [--every N]` to schedule \
                         automatic consolidation every N user turns.",
                    ))]
                }
            }
            "start" | "on" | "enable" => {
                let cadence = match parse_dream_cadence(rest) {
                    Ok(value) => value,
                    Err(error) => {
                        return vec![ReplEvent::Output(format!("/dream start: {error}"))];
                    }
                };
                self.dream_schedule.enabled = true;
                self.dream_schedule.every_user_turns = cadence;
                self.dream_schedule.last_user_turn_count = self.user_turn_count;
                vec![ReplEvent::Output(format!(
                    "Dream loop enabled. Will auto-consolidate every {cadence} user turns. \
                     Use `/dream stop` to disable, `/dream status` to inspect.",
                ))]
            }
            "stop" | "off" | "disable" | "cancel" => {
                if self.dream_schedule.enabled {
                    self.dream_schedule.enabled = false;
                    vec![ReplEvent::Output(format!(
                        "Dream loop disabled. {} automatic runs fired during this session.",
                        self.dream_schedule.total_auto_runs,
                    ))]
                } else {
                    vec![ReplEvent::Output(String::from(
                        "Dream loop already disabled.",
                    ))]
                }
            }
            _ => vec![ReplEvent::Output(String::from(
                "Usage: /dream [run | start [--every N] | stop | status]",
            ))],
        }
    }

    fn execute_goal_command(&mut self, args: &str) -> Vec<ReplEvent> {
        let trimmed = args.trim();
        let lower = trimmed.to_ascii_lowercase();
        match lower.as_str() {
            "" | "status" | "show" => match &self.active_goal {
                None => vec![ReplEvent::Output(String::from(
                    "No active goal. Use /goal <target> to start one for this session.",
                ))],
                Some(goal) => vec![ReplEvent::Output(format!("Active goal:\n  {goal}"))],
            },
            "clear" | "stop" | "cancel" | "remove" => {
                self.active_goal = None;
                vec![ReplEvent::Output(String::from("Cleared active goal."))]
            }
            _ => {
                self.active_goal = Some(trimmed.to_owned());
                let prompt = build_goal_initial_prompt(trimmed);
                let mut events = vec![ReplEvent::Output(String::from(
                    "Goal set for this session.",
                ))];
                events.extend(self.run_goal_loop(&prompt));
                events
            }
        }
    }

    fn run_goal_loop(&mut self, initial_prompt: &str) -> Vec<ReplEvent> {
        let mut all_events = Vec::new();
        let mut prompt = initial_prompt.to_owned();

        for turn in 0..MAX_GOAL_CONTINUATION_TURNS {
            let turn_events: Vec<ReplEvent> = self
                .agent_loop
                .run(&prompt)
                .into_iter()
                .filter_map(agent_event_to_repl_event)
                .collect();
            all_events.extend(turn_events);

            let status = last_assistant_reply(self.agent_loop.messages())
                .and_then(|reply| parse_goal_status(&reply));
            match status {
                Some(GoalStatus::Achieved) => {
                    self.active_goal = None;
                    all_events.push(ReplEvent::Output(String::from("Goal achieved.")));
                    return all_events;
                }
                Some(GoalStatus::Continue) => {
                    if let Some(goal) = self.active_goal.clone() {
                        prompt = build_goal_continuation_prompt(&goal);
                    } else {
                        return all_events;
                    }
                }
                None => return all_events,
            }

            if turn + 1 == MAX_GOAL_CONTINUATION_TURNS {
                all_events.push(ReplEvent::Output(format!(
                    "Goal reached the continuation limit ({MAX_GOAL_CONTINUATION_TURNS} turns). \
                    Use /goal status to check the active goal or continue manually."
                )));
            }
        }
        all_events
    }

    fn execute_cache_command(&self, args: &str) -> CommandResult {
        let raw = args.trim();
        let limit = if raw.is_empty() {
            8usize
        } else {
            match raw.parse::<usize>() {
                Ok(n) => n.clamp(1, 50),
                Err(_) => return CommandResult::Text(String::from("Usage: /cache [limit]")),
            }
        };

        let summary = self.agent_loop.cost_tracker().summary();
        if summary.turns.is_empty() {
            return CommandResult::Text(String::from(
                "No cache diagnostics yet. Run at least one agent turn to see cache telemetry.",
            ));
        }

        let all_turns = &summary.turns;
        let shown: Vec<_> = all_turns.iter().rev().take(limit).collect();
        let shown: Vec<_> = shown.into_iter().rev().collect();

        let mut lines = vec![
            format!(
                "Cache diagnostics ({} of {} turn(s)):",
                shown.len(),
                all_turns.len()
            ),
            String::from("  turn   in       out     cache+    cache-r  cost"),
        ];
        for turn in &shown {
            let cache_note = if turn.cache_read + turn.cache_create > 0 {
                let total = turn.cache_read + turn.cache_create;
                let pct = (turn.cache_read * 100).checked_div(total).unwrap_or(0);
                format!(" ({pct}% hit)")
            } else {
                String::new()
            };
            lines.push(format!(
                "  {:<5}  {:<7}  {:<6}  {:<8}  {:<7}  {}{}",
                turn.turn,
                CostTracker::format_tokens(turn.input_tokens),
                CostTracker::format_tokens(turn.output_tokens),
                CostTracker::format_tokens(turn.cache_create),
                CostTracker::format_tokens(turn.cache_read),
                CostTracker::format_usd(turn.cost_usd),
                cache_note,
            ));
        }

        lines.push(String::new());
        let health = summary.cache_health();
        if let Some(hit_rate) = health.hit_rate_percent {
            let recent = health
                .recent_hit_rate_percent
                .map(|r| format!(" · recent {r}% hit"))
                .unwrap_or_default();
            lines.push(format!(
                "Overall: {} created, {} read · {hit_rate}% hit{recent} · {}",
                CostTracker::format_tokens(health.total_create_tokens),
                CostTracker::format_tokens(health.total_read_tokens),
                health.label()
            ));
        } else {
            lines.push(format!(
                "Overall: {} · no cache telemetry yet",
                health.label()
            ));
        }

        CommandResult::Text(lines.join("\n"))
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

const MAX_GOAL_CONTINUATION_TURNS: usize = 20;

/// System-prompt hint appended when fast mode is enabled. Mirrors the Swift
/// implementation's fast-mode section.
const FAST_MODE_PROMPT: &str = "- Fast mode for Deeptide uses the same model with faster output. It does NOT switch to a different model. It can be toggled with /fast.";

fn default_editor() -> String {
    // On Windows, default to notepad when no $EDITOR is set.
    #[cfg(windows)]
    {
        String::from("notepad")
    }
    #[cfg(not(windows))]
    {
        String::from("vim")
    }
}

fn vim_temp_path() -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("deeptide-compose-{n}.md"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GoalStatus {
    Achieved,
    Continue,
}

fn parse_goal_status(text: &str) -> Option<GoalStatus> {
    let lower = text.to_ascii_lowercase();
    if lower.contains("goal_status: achieved") {
        Some(GoalStatus::Achieved)
    } else if lower.contains("goal_status: continue") {
        Some(GoalStatus::Continue)
    } else {
        None
    }
}

fn build_goal_initial_prompt(goal: &str) -> String {
    format!(
        "[deeptide-goal]\n\
        Goal:\n\
        {goal}\n\n\
        Work autonomously toward this goal. If the goal is already fully achieved, say so briefly \
        and end your final answer with exactly:\n\
        GOAL_STATUS: achieved\n\n\
        If more work is still needed after this turn, do the next useful work now and end your \
        final answer with exactly:\n\
        GOAL_STATUS: continue\n\n\
        Do not create cron jobs, recurring jobs, background loops, or a new persistent goal. \
        This goal exists only in the current Deeptide TUI session.\n\
        [/deeptide-goal]"
    )
}

fn build_goal_continuation_prompt(goal: &str) -> String {
    format!(
        "[deeptide-goal-continue]\n\
        Continue working toward the active goal:\n\
        {goal}\n\n\
        Review the current session history and workspace state. If the goal is fully achieved now, \
        stop and end your final answer with exactly:\n\
        GOAL_STATUS: achieved\n\n\
        If it is not fully achieved, perform the next useful work now and end your final answer \
        with exactly:\n\
        GOAL_STATUS: continue\n\n\
        Do not create cron jobs, recurring jobs, background loops, or a new persistent goal.\n\
        [/deeptide-goal-continue]"
    )
}

/// Format per-turn token and cost diagnostics for the supplied turns. Returns
/// `None` when there are no turns (e.g. a model error or a backend that
/// reports no usage), so callers can skip emitting an empty debug line.
/// Parse a `/dream start` cadence argument. Accepts both bare integers
/// (`/dream start 50`) and `--every`/`-n` flags (`/dream start --every 50`).
/// Empty input means "use the default cadence". Invalid input produces a
/// short, actionable error message.
fn parse_dream_cadence(rest: &str) -> Result<usize, String> {
    let rest = rest.trim();
    if rest.is_empty() {
        return Ok(DEFAULT_DREAM_CADENCE);
    }
    let value_str = if let Some(after) = rest.strip_prefix("--every") {
        after.trim_start_matches('=').trim()
    } else if let Some(after) = rest.strip_prefix("--every=") {
        after.trim()
    } else if let Some(after) = rest.strip_prefix("-n") {
        after.trim_start_matches('=').trim()
    } else {
        rest
    };
    let value: usize = value_str
        .parse()
        .map_err(|_| format!("expected integer cadence, got '{value_str}'"))?;
    if value < MIN_DREAM_CADENCE {
        return Err(format!(
            "cadence must be >= {MIN_DREAM_CADENCE}; got {value}"
        ));
    }
    if value > MAX_DREAM_CADENCE {
        return Err(format!(
            "cadence must be <= {MAX_DREAM_CADENCE}; got {value}"
        ));
    }
    Ok(value)
}

fn format_debug_turns(turns: &[crate::TurnRecord]) -> Option<String> {
    if turns.is_empty() {
        return None;
    }
    let lines: Vec<String> = turns
        .iter()
        .map(|turn| {
            format!(
                "[debug] turn {} · {} · in {} out {} · cache +{}/{} · {}ms · {}",
                turn.turn,
                turn.model,
                CostTracker::format_tokens(turn.input_tokens),
                CostTracker::format_tokens(turn.output_tokens),
                CostTracker::format_tokens(turn.cache_create),
                CostTracker::format_tokens(turn.cache_read),
                turn.duration_ms,
                CostTracker::format_usd(turn.cost_usd),
            )
        })
        .collect();
    Some(lines.join("\n"))
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
        AgentLoopEvent::Terminal(AgentTerminalEvent::Blocked) => {
            Some(ReplEvent::Output(String::from(
                "Context window full: the transcript exceeds the model's limit even after compaction. Start a new session (/new) or trim context.",
            )))
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
        AgentLoopEvent::Compaction(report) => Some(ReplEvent::Output(format!(
            "Context auto-compacted: folded {} earlier message(s); ~{} tokens now.",
            report.compressed_messages, report.tokens_after
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
        CommandCompletionSource::new(
            "goal",
            ["objective"],
            "Run an autonomous agent goal loop until achieved",
            "/goal [status | clear | <goal text>]",
        ),
        CommandCompletionSource::new(
            "cache",
            ["kvcache", "manifest"],
            "Show recent prompt-cache diagnostics for this session",
            "/cache [limit]",
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

fn render_status(
    agent_loop: &AgentLoop,
    cwd: &std::path::Path,
    additional_dirs: &[std::path::PathBuf],
    provider_profile: ProviderProfile,
    session_id: &str,
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
        format!("  Session:  {session_id}"),
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
