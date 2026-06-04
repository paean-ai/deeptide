use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentTerminalEvent, ClearCommand,
    CommandCompletionSource, CommandContext, CommandResult, CompactCommand, CostCommand,
    CostTracker, ExitCommand, HelpCommand, MemoryCommand, NewCommand, PermissionManager,
    PermissionMode, PermissionRules, RememberCommand, Rule, SlashCommand, Tool, ToolContext,
    ToolRegistry, ToolResultSummaryFormatter, WriteTool,
    agent_loop::{ConversationMessage, MessageRole},
    memory::MemorySystem,
    prompt::build_system_prompt,
    session::{SessionStore, new_session_id},
    tools::{ClipboardTool, model_context_window},
    tui::{StatusLine, StatusSegment},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplEvent {
    /// Free-form assistant text or slash-command output. The CLI renders
    /// this through the markdown pipeline (live or assembled).
    Output(String),
    /// A structured, system-emitted message — tool execution events,
    /// auto-compaction summaries, terminal conditions. Carries enough
    /// metadata for the CLI to apply tool-specific styling (color, icons,
    /// dim call IDs) without parsing strings.
    System(SystemMessage),
    /// Request the CLI to exit the REPL loop (e.g. `/exit`, Ctrl+D).
    Exit,
}

/// Structured payload for [`ReplEvent::System`]. Each variant carries the
/// raw data needed by the CLI to render the event; the deeptide-core layer
/// stays color-agnostic because color is a CLI-time concern (driven by
/// `--no-color` / `NO_COLOR` / TTY detection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemMessage {
    /// "Tools completed: Read 1 file in ." — the batch-level rollup the
    /// agent loop emits after a tool batch finishes.
    ToolBatch { label: String, failed_count: usize },
    /// Per-tool result line — the CLI styles success vs failure differently
    /// and dims the call_id by default.
    Tool {
        name: String,
        call_id: String,
        summary: String,
        is_error: bool,
        /// Optional expanded body shown verbatim after the summary line
        /// (small, successful tool results only — see
        /// `should_expand_tool_result`).
        body: Option<String>,
        /// Optional one-line "subject" extracted from the tool's INPUT —
        /// `file_path` for Read/Write/Edit, the shell command for Bash,
        /// the URL for WebFetch, the search pattern for Grep, etc. The
        /// CLI renders this between the tool name and the result summary
        /// so a user can see at a glance *what* was acted on without
        /// having to expand the body.
        ///
        /// Populated by `build_tool_system_message` from the tool's input
        /// JSON; `None` for tools we don't have a subject extractor for.
        subject: Option<String>,
    },
    /// Auto-compaction completed; `compressed_messages` older messages
    /// folded into a summary, `tokens_after` tokens remain.
    Compaction {
        compressed_messages: usize,
        tokens_after: usize,
    },
    /// Terminal notice (max-turns, model error, blocked context window).
    /// Carries a pre-formatted message; the CLI applies an "alert" style.
    Notice(String),
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
    /// Run one memory-consolidation pass when the session ends (on `/exit` or
    /// host teardown via [`finalize_session`]). On by default; the scheduled
    /// `/dream` loop is the in-session counterpart. This is what lets durable
    /// facts get captured without the user invoking `/remember` by hand.
    ///
    /// [`finalize_session`]: ReplSession::finalize_session
    session_end_capture: bool,
    /// Guards against firing the end-of-session pass twice (e.g. `/exit`
    /// followed by a host teardown call).
    session_consolidated: bool,
    /// Whether conversation turns are autosaved to disk. Disabled by
    /// `--no-session-persistence` for privacy / scratch sessions.
    session_persistence: bool,
    /// Per-session message queue: lines the user typed while the agent was
    /// busy (or explicitly enqueued via `/queue add`) accumulate here, and
    /// the CLI drains them after each turn according to the queue's mode.
    ///
    /// Wrapped in `Arc<Mutex<…>>` because the CLI's streaming handler
    /// pushes from a non-blocking stdin poll while the main thread holds
    /// `ReplSession` — both sides need shared mutable access without
    /// requiring the session itself to be `Sync`. Cloning the `Arc` is
    /// cheap and the lock is held for microseconds at a time (push one
    /// string, or drain N strings).
    message_queue: Arc<Mutex<crate::message_queue::MessageQueue>>,
    /// In-session conversation checkpoints. Populated by
    /// `/checkpoint`, drained by `/rewind` / `/checkpoint restore`.
    /// FIFO bounded at [`crate::checkpoints::MAX_CHECKPOINTS`].
    /// Never persisted — the on-disk session store is the durable
    /// counterpart for cross-process survival.
    checkpoints: crate::checkpoints::CheckpointStore,
    /// One-shot latch for the proactive context-window warning. The
    /// REPL emits a single advisory message at the first turn where
    /// usage crosses [`CONTEXT_WARN_THRESHOLD_PERCENT`]; further turns
    /// don't re-spam the user. Resets to `0` (no warning yet) on
    /// `/new`, `/clear`, and `/compact` so a fresh transcript can
    /// re-warn from scratch.
    last_context_warn_bucket: u8,
    /// Smart auto-compact configuration. When enabled, the REPL
    /// triggers `agent_loop.compact()` automatically the first time
    /// context usage crosses [`AutoCompactConfig::threshold_percent`]
    /// after each compaction reset (a "compact latch", same one-shot
    /// pattern as the warning bucket). Off by default — users opt in
    /// via `/auto-compact on` because silently rewriting the
    /// transcript can surprise people the first time they hit it.
    auto_compact: AutoCompactConfig,
    /// Build/version string for `/version`, injected by the CLI (which owns the
    /// git-provenance build vars). `None` falls back to the crate version.
    version_info: Option<String>,
    /// Whether to offer follow-up prompt suggestions after a task finishes.
    suggestions_enabled: bool,
    /// The suggestions shown after the last turn, so a bare numeric input
    /// (`1`/`2`/`3`) on the next line expands to the chosen follow-up prompt.
    last_suggestions: Vec<crate::suggestions::Suggestion>,
    /// An interactive menu awaiting a numeric pick (e.g. the session list shown
    /// by a bare `/import`). Each entry's `action` is a line re-submitted when
    /// chosen. Takes precedence over `last_suggestions` for a bare number.
    pending_menu: Vec<ReplMenuChoice>,
}

/// One row of an interactive `/command` selection menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplMenuChoice {
    /// Display text shown next to the number / in the picker.
    pub label: String,
    /// Line re-submitted verbatim when this row is chosen (a slash command or
    /// a prompt).
    pub action: String,
}

/// A selectable menu a `/command` wants to present. A rich CLI renders this as
/// an interactive picker (type-to-filter, arrows, Enter); a plain/non-TTY host
/// falls back to the numbered text form and a numeric reply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplMenu {
    pub title: String,
    pub footer: String,
    pub choices: Vec<ReplMenuChoice>,
}

/// Runtime configuration for the smart auto-compact feature.
///
/// The contract:
///   * `enabled = false` ⇒ never fire.
///   * `enabled = true` ⇒ fire ONCE per turn-batch the first time
///     `context % window >= threshold_percent`. The same turn's
///     warning still goes out so the user sees both the warning AND
///     the auto-compaction acknowledgement.
///   * After firing, the warning latch resets (see
///     [`ReplSession::reset_context_warn_latch`]); the next time
///     context refills past the warning thresholds it warns + fires
///     again.
///
/// `threshold_percent` is bounded to `1..=99` on set so callers can't
/// configure "auto-compact at 0%" (would fire every turn) or "at
/// 100%" (would never fire because the window is the post-compaction
/// budget, not the raw prompt size).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoCompactConfig {
    pub enabled: bool,
    pub threshold_percent: u8,
    /// Lifetime counter — how many auto-compactions have fired in
    /// this session. Surfaced by `/auto-compact status` so the user
    /// can see whether the feature has been working silently.
    pub fired_count: usize,
}

/// Default threshold matches the topmost warning bucket so
/// auto-compact fires at the same time as the loudest advisory
/// (`95%`). Users who want it to fire earlier (less risk of
/// truncation but more aggressive context rewrites) can dial it down
/// via `/auto-compact threshold N`.
pub const AUTO_COMPACT_DEFAULT_THRESHOLD: u8 = 95;
/// Minimum allowed threshold. Below this, the rolling summary
/// quality drops sharply because there's barely any conversation to
/// summarise.
pub const AUTO_COMPACT_MIN_THRESHOLD: u8 = 50;
/// Maximum allowed threshold. `99` is effectively "as late as
/// possible while still leaving room for one final compaction
/// turn"; `100` is meaningless because the warning is capped at
/// 100%.
pub const AUTO_COMPACT_MAX_THRESHOLD: u8 = 99;

impl Default for AutoCompactConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_percent: AUTO_COMPACT_DEFAULT_THRESHOLD,
            fired_count: 0,
        }
    }
}

impl AutoCompactConfig {
    /// Clamp + assign `value` to `threshold_percent`. Returns the
    /// final stored value so callers can echo it back to the user
    /// ("threshold set to 95%") even when the input was out of
    /// range.
    pub fn set_threshold(&mut self, value: u8) -> u8 {
        self.threshold_percent =
            value.clamp(AUTO_COMPACT_MIN_THRESHOLD, AUTO_COMPACT_MAX_THRESHOLD);
        self.threshold_percent
    }
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

/// Turn cap for the end-of-session / `/dream` consolidation pass. It only needs
/// a MemorySearch and a handful of MemoryWrite calls; bounding it here (instead
/// of inheriting the session's `max_turns`, up to 200) keeps `/exit` and Ctrl-D
/// from blocking on many model round-trips before the REPL returns.
const CONSOLIDATION_MAX_TURNS: usize = 8;

/// Turn budget for a `/deep-seek` research pass. Higher than the consolidation
/// cap because real research fans out across several search → fetch → cross-check
/// rounds before it can synthesize, but still bounded so a runaway can't loop
/// indefinitely (and so the user keeps an interrupt point).
const DEEP_SEEK_MAX_TURNS: usize = 24;

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
            session_end_capture: true,
            session_consolidated: false,
            message_queue: Arc::new(Mutex::new(crate::message_queue::MessageQueue::new())),
            checkpoints: crate::checkpoints::CheckpointStore::new(),
            last_context_warn_bucket: 0,
            auto_compact: AutoCompactConfig::default(),
            version_info: None,
            suggestions_enabled: true,
            last_suggestions: Vec::new(),
            pending_menu: Vec::new(),
        }
    }

    /// Enable or disable follow-up suggestions (`--no-suggestions` turns them
    /// off). When off, no "Next steps" block is shown and numeric pick is inert.
    pub fn with_suggestions(mut self, enabled: bool) -> Self {
        self.suggestions_enabled = enabled;
        self
    }

    /// Inject the build/version string shown by `/version`. The CLI passes its
    /// full provenance line (version + git hash + date); core alone can't see
    /// the CLI crate's build vars.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version_info = Some(version.into());
        self
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

    /// Enable or disable the end-of-session memory-consolidation pass.
    pub fn with_session_end_capture(mut self, enabled: bool) -> Self {
        self.session_end_capture = enabled;
        self
    }

    /// Run the end-of-session memory-consolidation pass, if warranted, and
    /// return its events, so durable facts are captured without the user
    /// invoking `/remember`. The in-crate `/exit` command routes through this,
    /// and the CLI host also calls it on Ctrl-D / EOF. A host that can be torn
    /// down by signal should install a handler that calls this too — it is
    /// idempotent, so an extra call after `/exit` is a no-op.
    ///
    /// The pass itself reuses [`run_dream_consolidation_once`] (the agent-driven
    /// consolidation prompt). The standalone [`crate::memory_capture`] prompt
    /// builder / parser is a separate, not-yet-wired building block — it is not
    /// on this path.
    ///
    /// Fires at most once per session, and only when capture is enabled, the
    /// session persists turns, and at least one user turn happened. A scheduled
    /// `/dream` pass that already fired on this exact turn count is treated as
    /// the consolidation, so we don't double-run.
    ///
    /// [`run_dream_consolidation_once`]: ReplSession::run_dream_consolidation_once
    pub fn finalize_session(&mut self) -> Vec<ReplEvent> {
        if self.session_consolidated
            || !self.session_end_capture
            || !self.session_persistence
            || self.user_turn_count == 0
        {
            return Vec::new();
        }
        self.session_consolidated = true;

        // If a scheduled dream already consolidated at the current turn count,
        // there's nothing new to capture — skip the redundant pass.
        if self.dream_schedule.last_user_turn_count == self.user_turn_count {
            return Vec::new();
        }

        let mut events = vec![ReplEvent::Output(String::from(
            "[session end] consolidating memory… (skip next time with --no-session-capture)",
        ))];
        events.extend(self.run_dream_consolidation_once());
        events
    }

    /// Side-effect-free predicate that reports whether a subsequent
    /// [`finalize_session`] call would actually run the (model-backed,
    /// multi-second) consolidation pass. The CLI host uses this to
    /// decide whether to animate a "consolidating memory" spinner before
    /// it drives the synchronous exit — otherwise the terminal looks
    /// frozen while the pass runs.
    ///
    /// Must stay in lockstep with the gate conditions in
    /// [`finalize_session`]; a `true` here followed by a no-op finalize
    /// would leave a spinner with nothing behind it.
    ///
    /// [`finalize_session`]: ReplSession::finalize_session
    pub fn will_consolidate_on_exit(&self) -> bool {
        !self.session_consolidated
            && self.session_end_capture
            && self.session_persistence
            && self.user_turn_count > 0
            && self.dream_schedule.last_user_turn_count != self.user_turn_count
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

    /// `/suggest [on|off|show]` — toggle or re-display follow-up suggestions.
    fn execute_suggest_command(&mut self, args: &str) -> CommandResult {
        match args.trim().to_ascii_lowercase().as_str() {
            "on" | "enable" => {
                self.suggestions_enabled = true;
                CommandResult::Text(String::from("Follow-up suggestions enabled."))
            }
            "off" | "disable" => {
                self.suggestions_enabled = false;
                self.last_suggestions.clear();
                CommandResult::Text(String::from("Follow-up suggestions disabled."))
            }
            "" | "show" | "status" => {
                if !self.suggestions_enabled {
                    return CommandResult::Text(String::from(
                        "Follow-up suggestions are OFF. Enable with /suggest on.",
                    ));
                }
                match crate::suggestions::render_block(&self.last_suggestions) {
                    Some(block) => CommandResult::Text(block),
                    None => CommandResult::Text(String::from(
                        "No suggestions yet — they appear after a task finishes.",
                    )),
                }
            }
            other => CommandResult::Text(format!("Usage: /suggest [on|off|show] (got `{other}`)")),
        }
    }

    /// `/version` — show the running build. Uses the CLI-injected provenance
    /// string when present, else the core crate version.
    fn execute_version_command(&self) -> CommandResult {
        let body = self
            .version_info
            .clone()
            .unwrap_or_else(|| format!("deeptide-rs {}", env!("CARGO_PKG_VERSION")));
        CommandResult::Text(body)
    }

    /// `/todo` (aliases `/todos`, `/tasklist`) — expand the TODO backlog the
    /// status bar only summarizes as `todo N/M`. Read-only; the agent owns the
    /// list via the `TodoWrite` tool.
    fn execute_todo_command(&self) -> CommandResult {
        let lines = crate::tools::todo_lines();
        if lines.is_empty() {
            return CommandResult::Text(String::from(
                "No active todos. The agent populates this list with the TodoWrite tool as it works through a multi-step task.",
            ));
        }
        let summary = crate::tools::todo_summary();
        let mut body = format!(
            "Todo backlog ({} in progress · {} pending · {} done):\n",
            summary.in_progress, summary.pending, summary.completed
        );
        for line in lines {
            body.push_str("  ");
            body.push_str(&line);
            body.push('\n');
        }
        CommandResult::Text(body.trim_end().to_owned())
    }

    /// CLI-facing entry for `/import` — same arg grammar as the slash command
    /// (`<tool> [<id>|--latest] [--as memory|context]`). Returns the events the
    /// REPL would emit so the caller can render them identically.
    pub fn run_import(&mut self, args: &str) -> Vec<ReplEvent> {
        self.execute_import_command(args)
    }

    /// CLI-facing entry for `/continue` — newest foreign session → live handoff.
    pub fn run_continue(&mut self, args: &str) -> Vec<ReplEvent> {
        self.execute_continue_command(args)
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

    /// Forward an interactive permission callback into the underlying agent
    /// loop so that tools needing approval (`PermissionDecision::Ask`)
    /// surface a prompt instead of failing.
    pub fn with_ask_callback(mut self, callback: crate::PermissionAskCallback) -> Self {
        self.agent_loop = self.agent_loop.with_ask_callback(callback);
        self
    }

    /// Install a callback fired around every tool invocation. See
    /// [`crate::AgentLoop::with_tool_progress_callback`] for the
    /// semantic contract; the REPL forwards verbatim so callers
    /// don't need to reach through `agent_loop()`.
    pub fn with_tool_progress_callback(mut self, callback: crate::ToolProgressCallback) -> Self {
        self.agent_loop = self.agent_loop.with_tool_progress_callback(callback);
        self
    }

    /// Install the cooperative cancellation flag. Forwarded to the agent loop
    /// (which shares it with the tool context). The CLI flips this from its
    /// Ctrl-C handler to cancel an in-flight turn. See
    /// [`crate::AgentLoop::with_interrupt_flag`].
    pub fn with_interrupt_flag(
        mut self,
        flag: std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> Self {
        self.agent_loop = self.agent_loop.with_interrupt_flag(flag);
        self
    }

    /// Update the active permission mode mid-session. Used by the
    /// Shift+Tab keybinding to cycle Default → AcceptEdits → Plan → Bypass.
    pub fn set_permission_mode(&mut self, mode: crate::permissions::PermissionMode) {
        self.agent_loop.set_permission_mode(mode);
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

        let numeric = trimmed.parse::<usize>().ok();

        // Numeric pick of an interactive menu (e.g. a bare `/import`'s session
        // list). Highest precedence — a menu is an explicit, just-shown prompt.
        if !self.pending_menu.is_empty()
            && let Some(choice) = numeric
            && choice >= 1
            && choice <= self.pending_menu.len()
        {
            let action = self.pending_menu[choice - 1].action.clone();
            self.pending_menu.clear();
            return self.submit(&action);
        }
        // Any other input dismisses a stale menu so a later bare number can't
        // accidentally trigger it.
        self.pending_menu.clear();

        // Numeric pick of a follow-up suggestion from the previous turn: a bare
        // `1`/`2`/`3` in range expands to that suggestion's prompt and runs it
        // as a normal turn. Cleared first so it can't re-trigger or recurse.
        if !self.last_suggestions.is_empty()
            && let Some(choice) = numeric
            && choice >= 1
            && choice <= self.last_suggestions.len()
        {
            let prompt = self.last_suggestions[choice - 1].prompt.clone();
            self.last_suggestions.clear();
            return self.submit(&prompt);
        }

        if let Some(command_line) = trimmed.strip_prefix('/') {
            return self.execute_command(command_line);
        }

        // Expand `@path/to/file` references in the user message BEFORE
        // forwarding to the agent. Each successfully-resolved reference
        // gets inlined as a `<file path="…">…</file>` block in a single
        // attachments appendix. Skipped refs (not-found, binary, too
        // large, directory) surface as system-message notices so the
        // user has positive feedback their `@` didn't silently bind to
        // nothing — matches the Claude Code / Cursor UX.
        let expansion = crate::at_references::expand_at_references(
            trimmed,
            &self.tool_context.cwd,
            crate::at_references::AtExpansionOptions::default(),
        );
        let mut events: Vec<ReplEvent> = Vec::new();
        if !expansion.attachments.is_empty() {
            let notice = format_attachment_notice(&expansion);
            events.push(ReplEvent::Output(notice));
        }
        let prompt_to_agent = expansion.expanded;

        self.user_turn_count += 1;
        let turns_before = self.agent_loop.cost_tracker().summary().turns.len();
        let messages_before = self.agent_loop.messages().len();
        events.extend(
            self.agent_loop
                .run(&prompt_to_agent)
                .into_iter()
                .filter_map(agent_event_to_repl_event),
        );

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

        // Proactive context-window advisory. Emitted at most once per
        // bucket (80% → 90% → 95%) so the user gets warnings before
        // they hit the hard cliff but doesn't get spammed on every
        // turn. Reset by /new, /clear, /compact.
        if let Some(warn) = self.maybe_context_window_warning() {
            events.push(ReplEvent::Output(warn));
        }

        // Smart auto-compact: when enabled, fold older turns into a
        // rolling summary the moment usage crosses the configured
        // threshold. Runs AFTER the warning so the user sees both
        // signals in the same turn (warning + acknowledgement) the
        // first time it fires, and AFTER tps/debug emission so the
        // event stream stays in chronological order.
        if let Some(notice) = self.maybe_auto_compact() {
            events.push(ReplEvent::Output(notice));
        }

        if let Some(dream_events) = self.maybe_run_scheduled_dream() {
            events.extend(dream_events);
        }

        // Offer follow-up suggestions for the task that just finished. Derived
        // deterministically from this turn's signals (tools run, errors, active
        // TODO, closing text) — no extra model call. Stored so a bare `1`/`2`
        // next line expands to the chosen prompt.
        if self.suggestions_enabled {
            let signals = self.collect_turn_signals(messages_before);
            let suggestions = crate::suggestions::suggest(&signals);
            if let Some(block) = crate::suggestions::render_block(&suggestions) {
                events.push(ReplEvent::Output(block));
            }
            self.last_suggestions = suggestions;
        }

        self.autosave_session();
        events
    }

    /// Read the signals the suggestion engine needs from the messages this turn
    /// appended (assistant tool calls, tool errors, shell commands, closing
    /// text) plus the current TODO state.
    fn collect_turn_signals(&self, messages_before: usize) -> crate::suggestions::TurnSignals {
        let mut signals = crate::suggestions::TurnSignals {
            next_todo: crate::tools::next_actionable_todo(),
            ..Default::default()
        };
        let messages = self.agent_loop.messages();
        for message in messages.iter().skip(messages_before) {
            for call in &message.tool_calls {
                match call.name.as_str() {
                    "Edit" | "Write" | "NotebookEdit" | "MultiEdit" => signals.edited_files = true,
                    "Bash" => {
                        if let Some(cmd) = call.input.get("command").and_then(|v| v.as_str()) {
                            signals.bash_commands.push(cmd.to_owned());
                        }
                    }
                    _ => {}
                }
            }
            if message.tool_results.iter().any(|r| r.is_error) {
                signals.had_tool_error = true;
            }
            if message.role == crate::agent_loop::MessageRole::Assistant
                && !message.content.trim().is_empty()
            {
                signals.assistant_text = message.content.clone();
            }
        }
        signals
    }

    /// Inspect the current transcript size against the model's window
    /// and return a one-shot warning when it has just crossed a fresh
    /// threshold. Bumps `last_context_warn_bucket` as a side effect so
    /// subsequent calls in the same bucket are silent.
    fn maybe_context_window_warning(&mut self) -> Option<String> {
        let context_tokens = estimate_repl_context_tokens(self.agent_loop.messages());
        let window = model_context_window(self.agent_loop.model()) as usize;
        if window == 0 {
            return None;
        }
        let percent = context_tokens
            .saturating_mul(100)
            .checked_div(window)
            .unwrap_or(0)
            .min(100) as u8;
        let bucket = context_warn_bucket(percent);
        if bucket <= self.last_context_warn_bucket {
            return None;
        }
        self.last_context_warn_bucket = bucket;
        render_context_warn_message(bucket, percent, self.agent_loop.model())
    }

    /// Reset the one-shot warning latch so the next big transcript can
    /// re-warn from bucket 0. Callers: `/new`, `/clear`, `/compact`.
    fn reset_context_warn_latch(&mut self) {
        self.last_context_warn_bucket = 0;
    }

    /// Smart auto-compact entry point: if the feature is enabled and
    /// context usage just crossed [`AutoCompactConfig::threshold_percent`],
    /// fold older turns into a rolling summary right now and return a
    /// user-visible notice. Returns `None` when:
    ///
    /// * auto-compact is disabled (default state), or
    /// * usage is below the configured threshold, or
    /// * the underlying [`AgentLoop::compact`] reports `did_compress
    ///   = false` (nothing to compact — recent transcript already
    ///   fits the rolling window).
    ///
    /// Implementation note: like the warning, this method has a
    /// "fire once per crossing" semantics. The warning latch
    /// (`last_context_warn_bucket`) doubles as the fire latch — when
    /// auto-compact runs it ALSO resets the warning latch via
    /// `reset_context_warn_latch`, so a fresh refill of the context
    /// will re-warn AND re-fire. This pairing avoids the
    /// "warn-but-never-fire" and "fire-twice-without-warning" anti
    /// patterns.
    fn maybe_auto_compact(&mut self) -> Option<String> {
        if !self.auto_compact.enabled {
            return None;
        }
        let context_tokens = estimate_repl_context_tokens(self.agent_loop.messages());
        let window = model_context_window(self.agent_loop.model()) as usize;
        if window == 0 {
            return None;
        }
        let percent = context_tokens
            .saturating_mul(100)
            .checked_div(window)
            .unwrap_or(0)
            .min(100) as u8;
        if percent < self.auto_compact.threshold_percent {
            return None;
        }

        let report = self.agent_loop.compact();
        if !report.did_compress {
            // Nothing to compact — either the transcript is short
            // (over-budget but no foldable history yet) or the
            // summarizer rejected the pass. Don't claim victory
            // when nothing changed, and don't reset the warning
            // latch (the user still needs the advisory).
            return None;
        }

        self.auto_compact.fired_count += 1;
        // Reset the warning latch so the next pass through the
        // submit() pipeline starts re-warning from bucket 0 — the
        // post-compaction window is much smaller, and ignoring this
        // would leave the user thinking we're still at 95% forever.
        self.reset_context_warn_latch();

        Some(format!(
            "↻ auto-compact triggered at {percent}% (threshold {threshold}%): \
             folded {folded} message(s) into a summary; ~{remaining} tokens remain.",
            percent = percent,
            threshold = self.auto_compact.threshold_percent,
            folded = report.compressed_messages,
            remaining = report.tokens_after,
        ))
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
            - Persist each kept fact with the `MemoryWrite` tool (title, body, reason; scope `project` unless it is a cross-project user preference, then `global`). Do NOT write memory files by hand — `MemoryWrite` is the only path that lands in the location future sessions load from.\n\
            - Before writing, call `MemorySearch` for the same fact; if it is already stored, skip it — do NOT write a duplicate (`MemoryWrite` only appends new shards, it cannot edit one in place).\n\n\
            Rules:\n\
            - Execute exactly once.\n\
            - Do not edit system prompts, settings, cron jobs, or provider configuration.\n\
            - Do not edit `MEMORY.md` or files under the memory directory directly — go through `MemoryWrite`.\n\
            - Do not add memories that are generic, obvious, temporary, secret, or unsupported by session history.\n\
            - Keep each memory shard concise and human-readable."
        );
        // Bound the pass: consolidation is a MemorySearch + a few MemoryWrite
        // calls, not open-ended agentic work. Without this it inherits the
        // session `max_turns` (up to 200), so on `/exit` / Ctrl-D it could run
        // many model round-trips before the REPL returned — a slow, unskippable
        // exit. Cap it low, then restore the prior limit.
        let prev_turns = self.agent_loop.set_max_turns(CONSOLIDATION_MAX_TURNS);
        let events = self
            .agent_loop
            .run(&prompt)
            .into_iter()
            .filter_map(agent_event_to_repl_event)
            .collect();
        self.agent_loop.set_max_turns(prev_turns);
        events
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
        self.status_line_with_auth(None)
    }

    /// Build the status line with an optional auth segment inserted at its
    /// canonical priority slot (between `ctx` and `turns`).
    ///
    /// Segments are listed from highest to lowest survival priority. The
    /// renderer drops trailing segments first when the terminal is narrow, so
    /// the must-show items (model, mode, cwd, git, ctx, auth) stay visible
    /// while `turns` and `cost` fall off first.
    pub fn status_line_with_auth(&self, auth: Option<StatusSegment>) -> StatusLine {
        let summary = self.agent_loop.cost_tracker().summary();
        let context_tokens = estimate_repl_context_tokens(self.agent_loop.messages());
        let window = model_context_window(self.agent_loop.model()) as usize;
        let context_pct = context_tokens
            .saturating_mul(100)
            .checked_div(window)
            .unwrap_or(0);
        let branch = git_branch(&self.tool_context.cwd).unwrap_or_else(|| String::from("no-git"));
        let cwd = format_cwd_for_status(&self.tool_context.cwd);

        // The `ctx` segment carries either just the usage percent
        // (`87%`) or the percent plus an `auto<N>` suffix
        // (`87% auto95`) when smart auto-compact is enabled. Keeping
        // it on the same segment instead of adding a separate
        // `auto` segment saves a column slot on narrow terminals
        // and visually associates the indicator with the metric it
        // operates on.
        let ctx_value = if self.auto_compact.enabled {
            format!(
                "{context_pct}% auto{threshold}",
                threshold = self.auto_compact.threshold_percent
            )
        } else {
            format!("{context_pct}%")
        };

        // Severity assignment for the live segments. Kept terse and
        // local so the policy is visible at-a-glance instead of
        // scattered across helpers:
        //
        //   * `mode`  — Alert when YOLO is on (`bypass`), Warning for
        //               `accept-edits` (silent file modifications),
        //               Info for `plan` (read-only), Neutral
        //               otherwise. Mirrors how Codex / Claude Code
        //               flag dangerous modes.
        //   * `ctx`   — Alert at ≥ 95% (about to truncate), Warning
        //               at ≥ 80% (compaction zone), Neutral below.
        //               Without this the user has no chance to
        //               trigger `/compact` before the model starts
        //               dropping context.
        let mode_label = self.agent_loop.permission_mode().label();
        // Label strings come from `PermissionMode::label()` — keep
        // these arms in sync with that definition. "yolo" is the
        // user-visible label for `Bypass`; pinned in
        // `permissions::PermissionMode::label` tests so any rename
        // there fails the unit tests covering this mapping.
        let mode_severity = match mode_label {
            "yolo" => crate::Severity::Alert,
            "accept-edits" => crate::Severity::Warning,
            "plan" => crate::Severity::Info,
            _ => crate::Severity::Neutral,
        };
        let ctx_severity = if context_pct >= 95 {
            crate::Severity::Alert
        } else if context_pct >= 80 {
            crate::Severity::Warning
        } else {
            crate::Severity::Neutral
        };

        // Segment order IS priority order: `StatusLine::render` keeps the
        // leading segments and drops the trailing ones first when the terminal
        // is too narrow to fit them all. So the high-value, volatile session
        // metrics the user actively watches — `model`, `mode`, and especially
        // `ctx` (context/token usage) — lead, and the longer, more static
        // context (`cwd`, `git`) plus the low-priority `turns`/`cost` trail and
        // get pruned first. Putting `cwd`/`git` ahead of `ctx` (as we used to)
        // let a long working-directory path push the token indicator off a
        // narrow tab entirely — the exact thing the user shouldn't lose.
        let mut segments = vec![
            StatusSegment::new("model", self.agent_loop.model()),
            StatusSegment::new("mode", mode_label).with_severity(mode_severity),
            StatusSegment::new("ctx", ctx_value).with_severity(ctx_severity),
        ];
        // Live prompt-cache health, right after `ctx` so it rides along with
        // the other session-state indicators through narrow-terminal
        // truncation. Silent until the provider reports any cache telemetry —
        // the bar stays quiet when caching isn't in play (e.g. turn 1, or a
        // provider that doesn't report cache tokens). This is the real-time
        // counterpart to the fuller `/usage` cache breakdown: the user sees
        // "cache 87%" climb as the prefix warms, without running a command.
        if let Some(cache) = cache_status_segment(&summary.cache_health()) {
            segments.push(cache);
        }
        if let Some(auth) = auth {
            segments.push(auth);
        }
        // Surface message-queue depth ONLY when non-empty. We want
        // status-bar real estate to be silent when the feature is
        // unused; the moment something is queued (either via mid-turn
        // type-ahead or `/queue add`), a `queue N` segment appears so
        // the user knows automatic submits are pending. Inserted right
        // after auth so it survives narrow-terminal truncation along
        // with the other "active session state" indicators (model,
        // mode, ctx, auth) — `turns` and `cost` get pruned first.
        if let Ok(q) = self.message_queue.lock()
            && !q.is_empty()
        {
            // Cyan `queue N` so the user spots that automatic
            // submits are pending even at a glance — the bar is
            // otherwise dim everywhere.
            segments.push(
                StatusSegment::new("queue", format!("{}", q.len()))
                    .with_severity(crate::Severity::Info),
            );
        }
        // Surface the active TODO backlog the same way. Format is
        // `todo IP/N` where IP is the count of items currently
        // in-progress and N is the total (pending + in_progress +
        // completed). This gives the user a persistent "task
        // tracker" pinned at the bottom that updates on every
        // repaint without scrolling away — matching how
        // Codex/Claude Code surface their plan/task lists. Only
        // rendered when there's at least one item so greenfield
        // sessions stay clean.
        let todo = crate::tools::todo_summary();
        if todo.is_active() {
            let active = todo.in_progress + todo.pending;
            segments.push(StatusSegment::new(
                "todo",
                format!("{}/{}", active, todo.total()),
            ));
        }
        // Static context — useful but recoverable elsewhere (`pwd`, the shell
        // prompt, `git status`), so it trails the live metrics and is the first
        // thing dropped on a narrow terminal.
        segments.push(StatusSegment::new("cwd", cwd));
        segments.push(StatusSegment::new("git", branch));
        segments.push(StatusSegment::new(
            "turns",
            format!("{}/{}", summary.turns.len(), self.agent_loop.max_turns()),
        ));
        segments.push(StatusSegment::new(
            "cost",
            CostTracker::format_usd(summary.total_cost_usd),
        ));

        StatusLine::new(segments)
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

    /// Borrow the shared message-queue handle. Callers `Arc::clone` it to
    /// share between the main thread (which drains the queue between
    /// turns and runs `/queue` slash commands) and the CLI's mid-turn
    /// stdin poller (which pushes new lines into it from the streaming
    /// handler). The handle stays valid for the entire session — the
    /// REPL never replaces the inner queue, only mutates through it.
    pub fn message_queue_handle(&self) -> Arc<Mutex<crate::message_queue::MessageQueue>> {
        Arc::clone(&self.message_queue)
    }

    /// Read-only snapshot of the queued items + current mode + length,
    /// taken under the lock. Convenience for callers that want to render
    /// status without keeping the lock alive while writing to stdout.
    pub fn message_queue_snapshot(&self) -> (Vec<String>, crate::message_queue::QueueMode, usize) {
        match self.message_queue.lock() {
            Ok(q) => (q.snapshot(), q.mode(), q.len()),
            Err(_) => (Vec::new(), crate::message_queue::QueueMode::Single, 0),
        }
    }

    /// Pop the next prompt from the queue according to the configured
    /// mode. The CLI calls this after a turn ends; when it returns
    /// `Some`, the CLI feeds that string into the next agent submit.
    pub fn drain_next_queued_prompt(&self) -> Option<String> {
        self.message_queue.lock().ok()?.drain_next()
    }

    fn execute_command(&mut self, command_line: &str) -> Vec<ReplEvent> {
        let mut parts = command_line.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or_default().to_ascii_lowercase();
        let args = parts.next().unwrap_or_default();

        if matches!(name.as_str(), "exit" | "quit" | "q") {
            // Exit immediately — `/exit` should quit as fast as Ctrl-C.
            // We deliberately do NOT run the memory consolidation pass
            // here: it drives a full agent loop (up to `max_turns` model
            // round-trips, with no timeout) that can take minutes or
            // appear to hang. Durable facts are still captured by the
            // scheduled dream loop during the session, and `/dream`
            // forces a consolidation pass on demand for anyone who wants
            // one before leaving.
            return vec![ReplEvent::Exit];
        }

        let context = self.command_context();
        let result = match name.as_str() {
            "help" | "h" | "?" => HelpCommand.execute(args, &context),
            "clear" | "cls" => {
                self.agent_loop.reset();
                self.reset_context_warn_latch();
                ClearCommand.execute(args, &context)
            }
            "new" => {
                self.agent_loop.reset();
                self.reset_context_warn_latch();
                NewCommand.execute(args, &context)
            }
            "compact" | "compress" => {
                let result = self.execute_compact_command(args);
                self.reset_context_warn_latch();
                result
            }
            // `/auto-compact`: configure the smart auto-compact
            // feature. Off by default — see `AutoCompactConfig`. The
            // grammar accepts `on|off|status` plus
            // `threshold <N>` and the shorthand `<N>` (an integer
            // anywhere becomes "enable + set threshold").
            "auto-compact" | "autocompact" | "auto_compact" => {
                self.execute_auto_compact_command(args)
            }
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
            "import" => return self.execute_import_command(args),
            "continue" | "handoff" => return self.execute_continue_command(args),
            "version" | "ver" => self.execute_version_command(),
            "todo" | "todos" | "tasklist" => self.execute_todo_command(),
            "suggest" | "suggestions" => self.execute_suggest_command(args),
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
            "explain" => return self.execute_explain_command(args),
            "changelog" => return self.execute_changelog_command(args),
            "skills" | "skill" => self.execute_skills_command(args),
            "reminder" | "anchor" | "reorient" => return self.execute_reminder_command(args),
            "dream" => return self.execute_dream_command(args),
            "deep-seek" | "deepseek" | "research" => {
                return self.execute_deep_seek_command(args);
            }
            "cron" => self.execute_cron_command(args),
            "goal" | "objective" => return self.execute_goal_command(args),
            "cache" | "kvcache" | "manifest" => self.execute_cache_command(args),
            // `/queue`: manage the per-session message queue (lines typed
            // mid-turn or explicitly enqueued with `/queue add`). `/q` is
            // already taken by `/exit`, so we don't claim it as an alias.
            "queue" => self.execute_queue_command(args),
            // `/tools`: discovery surface for the agent's tool catalog.
            // Mirrors the role `/help` plays for slash commands.
            "tools" | "tool" => self.execute_tools_command(args),
            // `/think`: toggle extended-thinking (reasoning) on/off,
            // tune its budget, or read the current state.
            "think" | "thinking" | "reason" | "reasoning" => self.execute_think_command(args),
            // `/search`: full-text search across the current session's
            // user/assistant messages.
            "search" | "find" | "grep-chat" => self.execute_search_command(args),
            // `/checkpoint` + `/checkpoints` + `/rewind`: in-session
            // snapshot/restore. See `execute_checkpoint_command` for
            // the full grammar.
            "checkpoint" | "snap" | "snapshot" => self.execute_checkpoint_command(args),
            "checkpoints" => self.execute_checkpoint_command("list"),
            "rewind" | "undo-turn" => self.execute_rewind_command(args),
            // `/usage`: per-tool observability dashboard. Use a separate
            // verb from `/cost` (model spend) and `/cache` (prompt
            // cache) so each command has a single responsibility.
            "usage" | "tooltime" | "telemetry" => self.execute_usage_command(args),
            // `/test` + `/lint`: detect project toolchain (Cargo,
            // package.json, pyproject, go.mod, Gemfile) and either
            // print the suggested command (default) or run it
            // synchronously (`--run`). See `execute_toolchain_command`.
            "test" | "tests" => self.execute_toolchain_command(args, ToolchainAction::Test),
            "lint" | "check" => self.execute_toolchain_command(args, ToolchainAction::Lint),
            // A user-authored markdown command (`<name>.md` under a commands/
            // dir) expands to a prompt and runs as a normal turn. Checked only
            // after every built-in handler, so a built-in always wins over a
            // same-named file (no shadowing of core commands).
            _ if find_command_file(&self.tool_context.cwd, &name).is_some() => {
                return self.execute_custom_command(&name, args);
            }
            _ => CommandResult::Text(crate::commands::render_unknown_command(
                &name,
                &context.all_commands(),
            )),
        };

        command_result_to_repl_events(result)
    }

    /// Run a user-authored markdown slash command: read `<name>.md`, drop its
    /// YAML frontmatter, substitute `$ARGUMENTS` / `$1..$11` from the command
    /// tail (reusing the built-in skill expander), and submit the result as a
    /// normal agent turn. The body becomes a *prompt*, not a nested slash
    /// command, so there's no command-recursion to guard against.
    fn execute_custom_command(&mut self, name: &str, args: &str) -> Vec<ReplEvent> {
        let Some(path) = find_command_file(&self.tool_context.cwd, name) else {
            // Raced away between dispatch and here; fall back to the usage line.
            return vec![ReplEvent::Output(crate::commands::render_unknown_command(
                name,
                &self.command_context().all_commands(),
            ))];
        };
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return vec![ReplEvent::Output(format!(
                "Could not read custom command `/{name}` at {}.",
                path.display()
            ))];
        };
        let body = crate::memory::strip_frontmatter(&raw);
        if body.trim().is_empty() {
            return vec![ReplEvent::Output(format!(
                "Custom command `/{name}` ({}) has no body to run.",
                path.display()
            ))];
        }
        let args = args.trim();
        let expanded = crate::tools::expand_skill_prompt(&body, (!args.is_empty()).then_some(args));
        self.submit(&expanded)
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

    /// Implement `/auto-compact <subcommand>`. See [`auto_compact_help_text`]
    /// for the full grammar.  Parsing precedence:
    ///
    /// 1. Empty args ⇒ status.
    /// 2. A bare integer (`/auto-compact 90`) ⇒ enable + set threshold.
    /// 3. `on` / `enable`, `off` / `disable`, `status`.
    /// 4. `threshold <N>` ⇒ set threshold only (does NOT enable;
    ///    callers who want both can use the shorthand above).
    /// 5. `reset` ⇒ zero out the `fired_count` lifetime counter.
    /// 6. Anything else ⇒ help text.
    fn execute_auto_compact_command(&mut self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        if trimmed.is_empty() || matches!(trimmed, "status" | "show") {
            return CommandResult::Text(self.auto_compact_status_line());
        }

        // Shorthand: `/auto-compact 88` ⇒ enable + threshold 88.
        if let Ok(value) = trimmed.parse::<u8>() {
            return self.auto_compact_set(true, Some(value));
        }

        let mut parts = trimmed.split_whitespace();
        let verb = parts
            .next()
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        let rest = parts.collect::<Vec<_>>().join(" ");

        match verb.as_str() {
            "on" | "enable" | "enabled" => self.auto_compact_set(true, None),
            "off" | "disable" | "disabled" => self.auto_compact_set(false, None),
            "threshold" | "at" | "percent" | "pct" => {
                let Ok(value) = rest.trim().parse::<u8>() else {
                    return CommandResult::Text(format!(
                        "Usage: /auto-compact threshold <{min}..={max}>",
                        min = AUTO_COMPACT_MIN_THRESHOLD,
                        max = AUTO_COMPACT_MAX_THRESHOLD,
                    ));
                };
                self.auto_compact_set(self.auto_compact.enabled, Some(value))
            }
            "reset" => {
                let prev = self.auto_compact.fired_count;
                self.auto_compact.fired_count = 0;
                CommandResult::Text(format!(
                    "Auto-compact fired_count reset (was {prev}). Current settings: {}",
                    self.auto_compact_status_line()
                ))
            }
            "help" | "?" => CommandResult::Text(auto_compact_help_text()),
            _ => CommandResult::Text(format!(
                "Unknown /auto-compact subcommand: {verb}\n\n{}",
                auto_compact_help_text()
            )),
        }
    }

    /// Apply an enable/disable + optional threshold change atomically
    /// and return the resulting status line, so the user sees the
    /// final settled state (including any clamping). Centralising the
    /// mutation in one spot keeps the audit trail simple — every
    /// state change goes through here.
    fn auto_compact_set(&mut self, enabled: bool, threshold: Option<u8>) -> CommandResult {
        let was_enabled = self.auto_compact.enabled;
        let prev_threshold = self.auto_compact.threshold_percent;
        self.auto_compact.enabled = enabled;
        if let Some(value) = threshold {
            self.auto_compact.set_threshold(value);
        }
        let final_threshold = self.auto_compact.threshold_percent;

        let mut summary = vec![format!(
            "Auto-compact {} (threshold {final_threshold}%).",
            if enabled { "enabled" } else { "disabled" }
        )];
        if was_enabled != enabled {
            summary.push(format!(
                "  state: {} → {}",
                if was_enabled { "on" } else { "off" },
                if enabled { "on" } else { "off" }
            ));
        }
        if let Some(_value) = threshold
            && prev_threshold != final_threshold
        {
            summary.push(format!(
                "  threshold: {prev_threshold}% → {final_threshold}%"
            ));
        }
        summary.push(format!(
            "  fired so far: {} (use `/auto-compact reset` to zero this counter)",
            self.auto_compact.fired_count
        ));
        CommandResult::Text(summary.join("\n"))
    }

    /// One-line status renderer used by `/auto-compact` (empty args)
    /// and the post-mutation echo. Keeps the wording identical
    /// between the two paths so the user can recognise the format
    /// regardless of how they got there.
    fn auto_compact_status_line(&self) -> String {
        let cfg = self.auto_compact;
        format!(
            "Auto-compact: {} | threshold {}% | fired {} time(s) this session.\n\n{}",
            if cfg.enabled { "ON" } else { "off" },
            cfg.threshold_percent,
            cfg.fired_count,
            auto_compact_help_text()
        )
    }

    /// Read-only snapshot for embedders that want to surface the
    /// auto-compact state on their own UI (e.g. the status bar
    /// indicator). Returns a copy because [`AutoCompactConfig`] is
    /// `Copy` and small.
    pub fn auto_compact_config(&self) -> AutoCompactConfig {
        self.auto_compact
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
                "  Ctrl+C          Cancel the running turn · press twice at the prompt to exit",
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
        // `--all` widens the listing to importable sessions from other agents
        // (Claude Code, Codex) discovered for this project.
        let want_all = args.split_whitespace().any(|a| a == "--all" || a == "-a");
        let filter_args: Vec<&str> = args
            .split_whitespace()
            .filter(|a| *a != "--all" && *a != "-a")
            .collect();
        if filter_args.len() > 1 {
            return CommandResult::Text(String::from("Usage: /sessions [filter] [--all]"));
        }
        let args = filter_args.first().copied().unwrap_or("");

        if want_all {
            return self.execute_sessions_all_command(args);
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

    /// `/sessions --all`: native deeptide sessions plus importable sessions
    /// from other agents (Claude Code, Codex), newest first.
    fn execute_sessions_all_command(&self, filter: &str) -> CommandResult {
        let refs = crate::import::discover(&self.tool_context.cwd);
        let native = SessionStore::list(&self.tool_context.cwd);
        if refs.is_empty() && native.is_empty() {
            return CommandResult::Text(String::from(
                "No sessions found for this project (deeptide, Claude Code, or Codex).",
            ));
        }
        let filter = filter.trim().to_ascii_lowercase();
        let mut lines = vec![String::from("Importable sessions (newest first):")];
        let mut shown = 0;
        for r in refs.iter() {
            if shown >= 25 {
                break;
            }
            let id = session_short(&r.session_id);
            if !filter.is_empty() && !id.to_ascii_lowercase().contains(&filter) {
                continue;
            }
            lines.push(format!(
                "  [{}] {}  →  /import {} {}",
                r.source.label(),
                id,
                r.source.label(),
                id,
            ));
            shown += 1;
        }
        if shown == 0 {
            lines.push(String::from("  (none matched)"));
        }
        lines.push(String::new());
        lines.push(String::from(
            "Use `/import <tool> <id> --as memory` to distil it, or `--as context` \
             (or `/continue <tool>`) for a live handoff.",
        ));
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

    /// Whether `line` (a full input line) would open an interactive selection
    /// menu, and the menu's data if so. Pure / side-effect-free: a rich CLI
    /// calls this BEFORE `submit` to render a picker; returning `None` means
    /// "just submit it normally".
    pub fn menu_for(&self, line: &str) -> Option<ReplMenu> {
        let command = line.trim().strip_prefix('/')?;
        let (name, args) = command
            .split_once(char::is_whitespace)
            .unwrap_or((command, ""));
        match name.to_ascii_lowercase().as_str() {
            "import" => self.import_menu_for_args(args),
            "resume" | "load" | "restore" => self.resume_menu(args),
            "model" | "m" => self.model_menu(args),
            _ => None,
        }
    }

    /// `/resume` (no id) → a picker of saved sessions for this project; each row
    /// resumes that session. `None` when an id is given or there are none.
    fn resume_menu(&self, args: &str) -> Option<ReplMenu> {
        if !args.trim().is_empty() {
            return None;
        }
        let choices: Vec<ReplMenuChoice> = SessionStore::list(&self.tool_context.cwd)
            .into_iter()
            .take(30)
            .map(|entry| {
                let preview = if entry.preview.is_empty() {
                    "(empty)".to_owned()
                } else {
                    entry.preview.clone()
                };
                ReplMenuChoice {
                    label: format!(
                        "{}  \"{}\"  ({} msgs)",
                        session_short(&entry.session_id),
                        preview,
                        entry.message_count
                    ),
                    action: format!("/resume {}", entry.session_id),
                }
            })
            .collect();
        if choices.is_empty() {
            return None;
        }
        Some(ReplMenu {
            title: String::from("Resume a session"),
            footer: String::from(
                "/sessions for the full list  ·  /sessions --all to import from other agents",
            ),
            choices,
        })
    }

    /// `/model` (no name) → a picker of known models; each row switches to it.
    /// `None` when a name is given. The current model leads and is tagged.
    fn model_menu(&self, args: &str) -> Option<ReplMenu> {
        if !args.trim().is_empty() {
            return None;
        }
        let current = self.agent_loop.model().to_owned();
        let mut names: Vec<String> = vec![current.clone()];
        for model in crate::cost::known_models() {
            if !names.iter().any(|n| n == model.name) {
                names.push(model.name.to_owned());
            }
        }
        let choices: Vec<ReplMenuChoice> = names
            .into_iter()
            .map(|name| {
                let tag = if name == current { "  · current" } else { "" };
                ReplMenuChoice {
                    label: format!("{name}{tag}"),
                    action: format!("/model {name}"),
                }
            })
            .collect();
        Some(ReplMenu {
            title: String::from("Switch model"),
            footer: String::from("or type a name/alias directly: /model <name|flash|pro>"),
            choices,
        })
    }

    /// The `/import` menu for the given args, or `None` when the args name a
    /// concrete session (so it should import directly, not open a menu) or no
    /// sessions exist.
    fn import_menu_for_args(&self, args: &str) -> Option<ReplMenu> {
        let mut tokens = args.split_whitespace().peekable();
        let only = match tokens.next() {
            None => None, // bare `/import` → all sources
            Some(raw) => Some(crate::import::SourceTool::parse(raw)?),
        };
        // A concrete selector or an explicit --as means "not a menu".
        let mut has_selector = false;
        let mut mode_given = false;
        while let Some(tok) = tokens.next() {
            if tok == "--as" || tok == "-a" {
                mode_given = true;
                let _ = tokens.next();
            } else {
                has_selector = true;
            }
        }
        if has_selector || mode_given {
            return None;
        }
        self.import_menu(only)
    }

    /// Build the `/import` selection menu: one row per discovered session
    /// (optionally filtered to `only`), newest first. Choosing a row hands that
    /// session off into the live conversation. `None` when there are none.
    fn import_menu(&self, only: Option<crate::import::SourceTool>) -> Option<ReplMenu> {
        let sessions: Vec<_> = crate::import::discover(&self.tool_context.cwd)
            .into_iter()
            .filter(|r| only.is_none_or(|s| r.source == s))
            .take(20)
            .collect();
        if sessions.is_empty() {
            return None;
        }
        let mut choices: Vec<ReplMenuChoice> = Vec::with_capacity(sessions.len() + 1);
        // Lead with the bulk action — "stand on all prior work" — when there's
        // more than one session to fold in.
        if sessions.len() > 1 {
            choices.push(ReplMenuChoice {
                label: format!(
                    "✨ Import ALL {} sessions into long-term memory",
                    sessions.len()
                ),
                action: String::from("/import all"),
            });
        }
        choices.extend(sessions.iter().enumerate().map(|(i, r)| {
            let id = session_short(&r.session_id);
            let newest = if i == 0 { " · newest" } else { "" };
            ReplMenuChoice {
                label: format!(
                    "[{}] {id}{newest} — continue here (handoff)",
                    r.source.label()
                ),
                action: format!("/import {} {} --as context", r.source.label(), r.session_id),
            }
        }));
        Some(ReplMenu {
            title: String::from("Select a session to continue"),
            footer: String::from(
                "Distil one into memory: /import <tool> <id> --as memory  ·  /sessions --all  ·  import sends session content to your model",
            ),
            choices,
        })
    }

    /// Store a menu for numeric-pick and render its numbered text form (the
    /// fallback path when the host can't run an interactive picker).
    fn present_menu(&mut self, menu: ReplMenu) -> Vec<ReplEvent> {
        let mut lines = vec![format!("{} (type a number):", menu.title)];
        for (i, choice) in menu.choices.iter().enumerate() {
            lines.push(format!("  {}. {}", i + 1, choice.label));
        }
        if !menu.footer.is_empty() {
            lines.push(format!("  {}", menu.footer));
        }
        self.pending_menu = menu.choices;
        self.last_suggestions.clear();
        vec![ReplEvent::Output(lines.join("\n"))]
    }

    /// First-run onboarding: if deeptide hasn't been used here before AND there
    /// are importable sessions from other agents for this project, return a hint
    /// nudging the user to import them (especially `/import all`). The host
    /// prints it once after the welcome banner and then calls [`mark_onboarded`].
    pub fn first_run_import_hint(&self) -> Option<String> {
        if !is_first_run() {
            return None;
        }
        let count = crate::import::discover(&self.tool_context.cwd).len();
        if count == 0 {
            return None;
        }
        Some(format!(
            "Found {count} prior session(s) from Claude Code / Codex in this project.\n\
             Start with their context instead of from scratch:\n\
             \x20 /import all   distil every past session into long-term memory (recommended)\n\
             \x20 /import       pick one session to continue here\n\
             \x20 (import sends those sessions' content to your configured model.)"
        ))
    }

    /// Numbered-text fallback for `/import`'s menu (used by non-TTY hosts).
    fn present_import_menu(&mut self, only: Option<crate::import::SourceTool>) -> Vec<ReplEvent> {
        match self.import_menu(only) {
            Some(menu) => self.present_menu(menu),
            None => {
                let scope = only.map(|s| s.label()).unwrap_or("any agent");
                vec![ReplEvent::Output(format!(
                    "No importable sessions found for this project ({scope}). Sessions \
                     come from Claude Code, Codex, or deeptide for this directory."
                ))]
            }
        }
    }

    /// Resolve `<source> [<id>|--latest]` to a concrete foreign session for the
    /// current project. Empty/`--latest`/`latest` picks the newest.
    fn resolve_import_ref(
        &self,
        source: crate::import::SourceTool,
        selector: &str,
    ) -> Result<crate::import::SessionRef, String> {
        crate::import::resolve_ref(&self.tool_context.cwd, source, selector)
    }

    /// `/import <tool> [<id>|--latest] [--as memory|context]` — bring an
    /// external agent's session into deeptide. `--as memory` (default) distils
    /// durable facts via the consolidation pass; `--as context` splices a
    /// framed handoff block into the live conversation.
    fn execute_import_command(&mut self, args: &str) -> Vec<ReplEvent> {
        let mut tokens = args.split_whitespace().peekable();

        // Bare `/import` (no source) — show an interactive menu of every
        // importable session for this project instead of a usage line.
        let Some(source_raw) = tokens.next() else {
            return self.present_import_menu(None);
        };
        // `/import all` — distil EVERY discovered session into long-term memory
        // (the first-install "stand on prior work" bootstrap).
        if source_raw.eq_ignore_ascii_case("all") {
            return self.import_all_to_memory();
        }
        let Some(source) = crate::import::SourceTool::parse(source_raw) else {
            return vec![ReplEvent::Output(format!(
                "Unknown source `{source_raw}`. Use claude, codex, or deeptide."
            ))];
        };
        // Split remaining args into a selector and an `--as <mode>`.
        let mut selector = String::new();
        let mut mode = "memory";
        let mut mode_given = false;
        while let Some(tok) = tokens.next() {
            if tok == "--as" || tok == "-a" {
                if let Some(m) = tokens.next() {
                    mode_given = true;
                    // Validate rather than silently falling back to memory, so a
                    // typo'd mode is reported here just as the CLI's clap
                    // value_parser reports it on `--import-as` (no divergence).
                    mode = match m {
                        "memory" => "memory",
                        "context" | "handoff" => "context",
                        other => {
                            return vec![ReplEvent::Output(format!(
                                "Unknown import mode `{other}`. Use `--as memory` or `--as context`."
                            ))];
                        }
                    };
                }
            } else if selector.is_empty() {
                selector = tok.to_owned();
            }
        }

        // `/import <tool>` with no session AND no mode → menu of that tool's
        // sessions, so the user picks rather than silently getting the newest.
        if selector.is_empty() && !mode_given {
            return self.present_import_menu(Some(source));
        }

        let session_ref = match self.resolve_import_ref(source, &selector) {
            Ok(r) => r,
            Err(e) => return vec![ReplEvent::Output(e)],
        };
        let transcript = match crate::import::parse_file(&session_ref.path, source) {
            Ok(t) => t,
            Err(e) => return vec![ReplEvent::Output(format!("Cannot read session: {e}"))],
        };

        if mode == "context" {
            self.import_as_context(transcript)
        } else {
            self.import_as_memory(transcript)
        }
    }

    /// `/continue [tool]` — newest foreign session → live handoff. Convenience
    /// over `/import <tool> --latest --as context`; defaults to Claude.
    fn execute_continue_command(&mut self, args: &str) -> Vec<ReplEvent> {
        let source_raw = args.split_whitespace().next().unwrap_or("claude");
        let Some(source) = crate::import::SourceTool::parse(source_raw) else {
            return vec![ReplEvent::Output(format!(
                "Unknown source `{source_raw}`. Use claude, codex, or deeptide."
            ))];
        };
        let session_ref = match self.resolve_import_ref(source, "--latest") {
            Ok(r) => r,
            Err(e) => return vec![ReplEvent::Output(e)],
        };
        match crate::import::parse_file(&session_ref.path, source) {
            Ok(t) => self.import_as_context(t),
            Err(e) => vec![ReplEvent::Output(format!("Cannot read session: {e}"))],
        }
    }

    /// `/import all` — distil EVERY discovered session (Claude Code, Codex,
    /// deeptide) for this project into long-term memory in ONE consolidation
    /// pass. Flattened transcripts are concatenated newest-first up to a
    /// display-column budget (each session truncated to its slice of what
    /// remains, so even one huge transcript can't blow the window); the model
    /// dedups via MemorySearch as usual.
    fn import_all_to_memory(&mut self) -> Vec<ReplEvent> {
        const MAX_SESSIONS: usize = 15;
        // Total budget for the concatenated transcript, in display columns
        // (matches `import::truncate`, which is width-aware). Sized to stay well
        // under the model's context window across the bundled sessions.
        const WIDTH_BUDGET: usize = 60_000;
        // Below this many columns of room left, stop adding more sessions: a
        // header plus only a sliver of content isn't worth a whole session slot.
        const MIN_CONTENT_SLICE: usize = 200;

        let refs: Vec<_> = crate::import::discover(&self.tool_context.cwd)
            .into_iter()
            .take(MAX_SESSIONS)
            .collect();
        if refs.is_empty() {
            return vec![ReplEvent::Output(String::from(
                "No importable sessions found for this project (Claude Code, Codex, or deeptide).",
            ))];
        }

        // Parse + flatten each discovered session, then bound the concatenation
        // (see `build_bulk_import_corpus`, which is unit-tested for the ceiling).
        let sessions: Vec<(String, String)> = refs
            .iter()
            .filter_map(|r| {
                let transcript = crate::import::parse_file(&r.path, r.source).ok()?;
                let flat = crate::import::flatten_for_extraction(&transcript);
                if flat.trim().is_empty() {
                    return None;
                }
                let header = format!(
                    "\n\n===== {} session {} =====\n",
                    r.source.label(),
                    session_short(&r.session_id)
                );
                Some((header, flat))
            })
            .collect();
        let BulkImportCorpus {
            combined,
            included,
            skipped,
        } = build_bulk_import_corpus(&sessions, WIDTH_BUDGET, MIN_CONTENT_SLICE);
        if included == 0 {
            return vec![ReplEvent::Output(String::from(
                "Found sessions, but none had distillable conversational content.",
            ))];
        }

        let cwd = self.tool_context.cwd.display().to_string();
        let prompt = format!(
            "[bulk session import — execute once, do NOT create cron jobs or loops]\n\n\
            You are importing context from {included} previous coding sessions (Claude Code, \
            Codex, deeptide) into deeptide's long-term memory for workspace:\n{cwd}\n\n\
            Below are the flattened transcripts, separated by `===== … =====` headers. Extract \
            the DURABLE, reusable project facts, decisions, conventions, environment \
            constraints, and unresolved follow-ups that recur across them — favour facts that \
            show up in more than one session.\n\n\
            Rules:\n\
            - Persist each kept fact with the `MemoryWrite` tool (scope `project` unless it is a \
            cross-project user preference, then `global`).\n\
            - Before each write, call `MemorySearch`; if a fact is already stored, skip it — do \
            NOT write duplicates (there will be overlap across sessions).\n\
            - Do not import one-off task details, transient state, secrets, or anything not \
            durable.\n\
            - Execute exactly once; do not edit settings, cron, or provider config.\n\n\
            Transcripts:\n{combined}"
        );

        let tail = if skipped > 0 {
            format!(" ({skipped} older session(s) skipped for length)")
        } else {
            String::new()
        };
        let mut events = vec![ReplEvent::Output(format!(
            "[import] distilling {included} session(s) into long-term memory{tail} \
             (sends their content to your configured model)…"
        ))];
        // Bulk distillation may touch more facts than a single session; give it
        // a slightly higher ceiling than the end-of-session pass, still bounded.
        events.extend(self.run_memory_consolidation(&prompt, CONSOLIDATION_MAX_TURNS * 3));
        events
    }

    /// Run a one-off memory-consolidation pass over `prompt`, restricted to the
    /// memory tools only and bounded to `max_turns`, restoring both afterwards.
    ///
    /// The tool allowlist matters because `prompt` embeds imported, potentially
    /// untrusted transcript text (a past session may have pasted web/file
    /// content). Even if that text tries to prompt-inject, the pass can only
    /// `MemorySearch` / `MemoryWrite` — never run a shell, edit files, or touch
    /// settings/cron/provider config — instead of relying on prose guardrails.
    fn run_memory_consolidation(&mut self, prompt: &str, max_turns: usize) -> Vec<ReplEvent> {
        self.run_bounded_pass(prompt, max_turns, &["MemorySearch", "MemoryWrite"])
    }

    /// Run a one-off agent pass over `prompt`, bounded to `max_turns` and
    /// restricted to `allowed_tools` only, restoring both afterwards. This
    /// intersects with any user-supplied restrictions instead of replacing them.
    ///
    /// The scoped tool allowlist is a real security boundary, not just a hint:
    /// `is_tool_permitted` gates dispatch, so a pass seeded with untrusted text
    /// (an imported transcript, or web content pulled in by `/deep-seek`) can
    /// only reach the tools the caller named — never a shell, file edit, or
    /// settings/cron/provider mutation, even if that text tries to prompt-inject.
    fn run_bounded_pass(
        &mut self,
        prompt: &str,
        max_turns: usize,
        allowed_tools: &[&str],
    ) -> Vec<ReplEvent> {
        let prev_turns = self.agent_loop.set_max_turns(max_turns);
        let (prev_allowed, prev_disallowed) = self
            .agent_loop
            .set_tool_restrictions_intersecting(allowed_tools);
        let events = self
            .agent_loop
            .run(prompt)
            .into_iter()
            .filter_map(agent_event_to_repl_event)
            .collect::<Vec<_>>();
        self.agent_loop
            .set_tool_restrictions(prev_allowed, prev_disallowed);
        self.agent_loop.set_max_turns(prev_turns);
        events
    }

    /// `/deep-seek <question>` (aliases `/deepseek`, `/research`) — run a bounded
    /// web-research pass: decompose the question, search + fetch sources,
    /// cross-check, and synthesize a cited answer. Restricted to web-only
    /// research tools (no shell / local file reads / file writes / config), so
    /// it's safe to point at arbitrary questions whose answers pull in untrusted
    /// web content.
    fn execute_deep_seek_command(&mut self, args: &str) -> Vec<ReplEvent> {
        let question = args.trim();
        if question.is_empty() {
            return vec![ReplEvent::Output(String::from(
                "Usage: /deep-seek <question>\n\nRuns a bounded web-research pass. By default, \
                 the model proposes likely official/canonical URLs from its built-in knowledge \
                 and verifies them with WebFetch. Optional search backends (BRAVE_SEARCH_API_KEY \
                 or SERPER_API_KEY) can improve source discovery. Sources are labeled as \
                 [verified], [known], or [unverified]. Sends your question and fetched page \
                 content to your configured model.",
            ))];
        }
        // Evidence policy:
        //
        // `/deep-seek` is intentionally not a strict citation gate by default.
        // LLMs often know stable canonical URLs (official docs, RFCs, crate docs,
        // standards pages) and can use those as WebFetch targets without a search
        // backend, which keeps the open-source default path cheap and simple.
        // The important product contract is transparency: the final report must
        // distinguish pages actually fetched this run from model-prior knowledge
        // and from claims that remain unverified. A future `--strict` mode can
        // hard-enforce "only cite successfully fetched URLs" once we plumb
        // per-pass fetched-URL tracking into the event stream.
        let prompt = format!(
            "[deep research — execute once; do NOT create cron jobs, recurring jobs, or background loops]\n\n\
            You are running Deeptide's deep-research pass. Investigate this question thoroughly \
            and answer it with evidence:\n\n\
            QUESTION: {question}\n\n\
            Evidence labels:\n\
            - [verified]: you successfully retrieved and read the page with `WebFetch` in this pass.\n\
            - [known]: a source URL or fact you know from model knowledge but did not fetch in this pass.\n\
            - [unverified]: plausible but not confirmed by a fetched source or reliable model knowledge.\n\n\
            Method:\n\
            - Decompose the question into the specific sub-questions you must answer.\n\
            - First, use your built-in knowledge to identify likely official/canonical URLs \
            and call `WebFetch` on them. Prefer stable sources such as official docs, standards, \
            source repositories, release notes, filings, papers, and primary announcements.\n\
            - If a WebSearch backend is configured, you may use `WebSearch` to improve source \
            discovery; then call `WebFetch` on the promising results. Never rely on search \
            snippets alone.\n\
            - If you cannot identify exact URLs and WebSearch is unavailable, say that source \
            discovery is limited and make a best-effort answer from fetched URLs and your own \
            knowledge, clearly marked as unverified where appropriate.\n\
            - Corroborate each key claim across at least two independent sources; note where \
            sources disagree or where evidence is thin.\n\
            - Prefer primary and recent sources; be explicit about publication dates when \
            recency matters.\n\n\
            Output a single final report with:\n\
            - A direct answer up top (2–4 sentences).\n\
            - The key findings as bullets, each with an inline source URL and an evidence label \
            (`[verified]`, `[known]`, or `[unverified]`).\n\
            - A short 'Confidence & gaps' note: what is well-supported, what is uncertain, \
            and which important sources were not fetched.\n\
            - A 'Sources' list grouped by evidence label: `[verified]` URLs fetched this pass, \
            `[known]` canonical URLs not fetched, and `[unverified]` attempted or uncertain URLs.\n\n\
            Rules:\n\
            - Research only: do NOT read local files, edit files, run shell commands, or change settings/config.\n\
            - Do not pretend a source was fetched. If you did not successfully retrieve a URL \
            with `WebFetch`, label it `[known]` or `[unverified]`, not `[verified]`.\n\
            - Do not fabricate URLs. If a likely URL fails to fetch, mention it only as an \
            attempted or unverified source."
        );
        let mut events = vec![ReplEvent::Output(format!(
            "[deep-seek] researching: {question}\n(fetches likely sources and sends your question \
             + fetched page content to your configured model; optional WebSearch may be used when \
             configured)…"
        ))];
        events.extend(self.run_bounded_pass(
            &prompt,
            DEEP_SEEK_MAX_TURNS,
            &["WebSearch", "WebFetch"],
        ));
        events
    }

    /// Distil an imported transcript into durable memory shards by running the
    /// consolidation pass seeded with its flattened text. Reuses the same
    /// agent-driven `MemoryWrite` path as `/dream`, so dedup + scope rules
    /// apply unchanged.
    fn import_as_memory(
        &mut self,
        transcript: crate::import::ImportedTranscript,
    ) -> Vec<ReplEvent> {
        let flat = crate::import::flatten_for_extraction(&transcript);
        if flat.trim().is_empty() {
            return vec![ReplEvent::Output(String::from(
                "Imported session has no conversational content to distil.",
            ))];
        }
        let source = transcript.source.label();
        let cwd = self.tool_context.cwd.display().to_string();
        let prompt = format!(
            "[session import — execute once, do NOT create cron jobs or loops]\n\n\
            You are importing context from a previous {source} session into deeptide's \
            long-term memory for workspace:\n{cwd}\n\n\
            Below is the conversational transcript of that session. Extract durable, \
            reusable project facts, decisions, conventions, constraints, and unresolved \
            follow-ups worth remembering for future sessions.\n\n\
            Rules:\n\
            - Persist each kept fact with the `MemoryWrite` tool (scope `project` unless it \
            is a cross-project user preference, then `global`).\n\
            - Before writing, call `MemorySearch`; if a fact is already stored, skip it — \
            do NOT write duplicates.\n\
            - Do not import one-off task details, transient state, secrets, or anything not \
            durable.\n\
            - Execute exactly once; do not edit settings, cron, or provider config.\n\n\
            Transcript:\n{flat}"
        );
        let mut events = vec![ReplEvent::Output(format!(
            "[import] distilling {} conversational turns from {source} session {} into memory \
             (sends the transcript to your configured model)…",
            transcript.message_turns(),
            session_short(&transcript.session_id),
        ))];
        events.extend(self.run_memory_consolidation(&prompt, CONSOLIDATION_MAX_TURNS));
        events
    }

    /// Splice a framed handoff block (older turns noted, recent tail verbatim)
    /// to the FRONT of the live conversation so it's a stable, cache-friendly
    /// prefix. Snapshots the pre-splice transcript first so `/rewind` undoes it.
    fn import_as_context(
        &mut self,
        transcript: crate::import::ImportedTranscript,
    ) -> Vec<ReplEvent> {
        if transcript.message_turns() == 0 {
            return vec![ReplEvent::Output(String::from(
                "Imported session has no conversational content to hand off.",
            ))];
        }
        // Snapshot the current transcript so the splice is undoable via /rewind.
        // (No-op when the conversation is still empty.)
        self.snapshot_checkpoint("before import handoff");

        const RECENT_TAIL: usize = 8;
        let handoff = crate::import::handoff_message(&transcript, RECENT_TAIL);
        let mut combined = vec![handoff];
        combined.extend(self.agent_loop.messages().to_vec());
        // `replace_messages` (not `restore_messages`): this AUGMENTS history, so
        // the cumulative cost/turn telemetry in the status bar must be preserved,
        // not reset as it would be for resuming a saved session.
        self.agent_loop.replace_messages(combined);

        let source = transcript.source.label();
        vec![ReplEvent::Output(format!(
            "[import] handed off {} turns from {source} session {} into the live \
             conversation (recent {} kept verbatim). Continue where it left off; \
             use `/import {source} --as memory` for the full durable context.",
            transcript.message_turns(),
            session_short(&transcript.session_id),
            RECENT_TAIL.min(transcript.message_turns()),
        ))]
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
        if let Some(ref v) = hooks.stop {
            add("Stop", v);
        }
        if let Some(ref v) = hooks.subagent_stop {
            add("SubagentStop", v);
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
        self.dispatch_skill_command("commit", "/commit", "commit", args)
    }

    /// Expand a built-in skill via the Skill tool and run the resulting prompt as
    /// a turn. Shared by the thin skill-backed slash commands (`/commit`,
    /// `/explain`, `/changelog`, …): `skill` is the registry skill name,
    /// `command` the user-facing name for error messages, and `label` the verb
    /// shown in the "Dispatching …" status line.
    fn dispatch_skill_command(
        &mut self,
        skill: &str,
        command: &str,
        label: &str,
        args: &str,
    ) -> Vec<ReplEvent> {
        let result = self.tool_registry.call(
            "Skill",
            serde_json::json!({"skill": skill, "args": args}),
            &self.tool_context,
        );
        if result.is_error {
            return vec![ReplEvent::Output(format!("{command}: {}", result.content))];
        }
        let mut events = vec![ReplEvent::Output(format!(
            "Dispatching {label} skill to the model."
        ))];
        events.extend(
            self.agent_loop
                .run(&result.content)
                .into_iter()
                .filter_map(agent_event_to_repl_event),
        );
        events
    }

    /// `/explain <file|symbol|area>` — read-only codebase explanation.
    fn execute_explain_command(&mut self, args: &str) -> Vec<ReplEvent> {
        if args.trim().is_empty() {
            return vec![ReplEvent::Output(String::from(
                "Usage: /explain <file path, symbol, or area to explain>",
            ))];
        }
        self.dispatch_skill_command("explain", "/explain", "explain", args)
    }

    /// `/changelog [range]` — draft release notes from the git history.
    fn execute_changelog_command(&mut self, args: &str) -> Vec<ReplEvent> {
        self.dispatch_skill_command("changelog", "/changelog", "changelog", args)
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
        let preferred = [
            "Read",
            "Write",
            "Edit",
            "AppendFile",
            "Bash",
            "Glob",
            "Grep",
            "TodoWrite",
        ];
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

    /// `/queue [list|clear|add <msg>|pop|mode single|mode batch]`
    ///
    /// Manages the per-session message queue. Lines the user types during
    /// an active agent turn flow into this queue automatically (the CLI's
    /// streaming handler does the actual polling); the REPL drains it
    /// between turns according to `QueueMode`. Calling `/queue` with no
    /// arguments is the same as `/queue list` — it shows the current
    /// depth, mode, and a preview of each pending message so the user can
    /// confirm what's about to fire on the next turn.
    fn execute_queue_command(&mut self, args: &str) -> CommandResult {
        use crate::message_queue::QueueMode;

        let trimmed = args.trim();
        let (verb, rest) = match trimmed.split_once(char::is_whitespace) {
            Some((head, tail)) => (head, tail.trim_start()),
            None => (trimmed, ""),
        };
        let verb = verb.to_ascii_lowercase();

        // Lock-once snapshot for verbs that need to render OR write.
        // Each branch acquires only what it needs.
        match verb.as_str() {
            "" | "list" | "show" | "ls" => {
                let (items, mode, len) = self.message_queue_snapshot();
                CommandResult::Text(render_queue_list(&items, mode, len))
            }
            "clear" | "drop" | "reset" => {
                let cleared = match self.message_queue.lock() {
                    Ok(mut q) => q.clear(),
                    Err(_) => return CommandResult::Text(String::from("Queue lock poisoned.")),
                };
                CommandResult::Text(if cleared == 0 {
                    String::from("Queue already empty.")
                } else {
                    format!("Cleared {cleared} queued message(s).")
                })
            }
            "add" | "push" | "enqueue" => {
                if rest.is_empty() {
                    return CommandResult::Text(String::from(
                        "Usage: /queue add <message>\n\nAdds <message> to the back of the queue. The next turn (or all queued turns in batch mode) will fire it automatically.",
                    ));
                }
                let queued = match self.message_queue.lock() {
                    Ok(mut q) => {
                        if q.push(rest) {
                            (q.len(), Some(format_queue_preview(rest)))
                        } else {
                            (q.len(), None)
                        }
                    }
                    Err(_) => return CommandResult::Text(String::from("Queue lock poisoned.")),
                };
                match queued.1 {
                    Some(preview) => {
                        CommandResult::Text(format!("Queued (#{n}): {preview}", n = queued.0))
                    }
                    None => CommandResult::Text(String::from(
                        "Nothing queued: the message was empty or whitespace-only.",
                    )),
                }
            }
            "pop" | "remove" => {
                let popped = match self.message_queue.lock() {
                    Ok(mut q) => q.pop_front(),
                    Err(_) => return CommandResult::Text(String::from("Queue lock poisoned.")),
                };
                CommandResult::Text(match popped {
                    Some(msg) => format!("Popped: {}", format_queue_preview(&msg)),
                    None => String::from("Queue is empty; nothing to pop."),
                })
            }
            "mode" => {
                if rest.is_empty() {
                    let (_, mode, _) = self.message_queue_snapshot();
                    return CommandResult::Text(format!(
                        "Current queue mode: {mode}\n  single  pop one message per turn (default)\n  batch   join the whole queue with blank lines and send as one prompt\n\nUse `/queue mode single` or `/queue mode batch` to switch.",
                    ));
                }
                let parsed = match QueueMode::parse(rest) {
                    Some(m) => m,
                    None => {
                        return CommandResult::Text(format!(
                            "Unknown queue mode `{rest}`. Use `single` or `batch`.",
                        ));
                    }
                };
                match self.message_queue.lock() {
                    Ok(mut q) => q.set_mode(parsed),
                    Err(_) => return CommandResult::Text(String::from("Queue lock poisoned.")),
                };
                CommandResult::Text(format!("Queue mode set to: {parsed}"))
            }
            other => CommandResult::Text(format!(
                "Unknown subcommand `/queue {other}`. Try `/queue list`, `/queue add <msg>`, `/queue pop`, `/queue clear`, or `/queue mode single|batch`.",
            )),
        }
    }

    /// `/tools [filter] [--read-only|--writes|--all]`
    ///
    /// Lists the agent's registered tools so the user can quickly check
    /// "do I have AppendFile?" or "what tools touch the network?".
    /// Filter is a case-insensitive substring match against the tool
    /// name; the optional flags partition by read-only/write capability
    /// (default: all tools, no partitioning).
    ///
    /// Output layout mirrors `/help`: name in fixed-width column,
    /// description trimmed to one line on the right. Counts on the
    /// header line so it's obvious at a glance how many tools are
    /// available and how many were filtered out.
    fn execute_tools_command(&self, args: &str) -> CommandResult {
        let mut filter: Option<String> = None;
        let mut mode = ToolListMode::All;
        let mut wants_details = false;

        for raw in args.split_whitespace() {
            match raw {
                "--read-only" | "--readonly" | "--ro" => mode = ToolListMode::ReadOnly,
                "--writes" | "--write" | "--wr" => mode = ToolListMode::Writes,
                "--all" => mode = ToolListMode::All,
                "--details" | "--full" | "-v" => wants_details = true,
                "--help" | "-h" => {
                    return CommandResult::Text(String::from(
                        "Usage: /tools [filter] [--read-only|--writes|--all] [--details]\n\nList registered agent tools. Optional `filter` is a case-insensitive substring match on the tool name.\n\nFlags:\n  --read-only    Show only tools that don't modify state (Read, Grep, …)\n  --writes       Show only tools that can modify state (Write, Bash, …)\n  --all          Show every tool (default)\n  --details      Print each tool's full description (multi-line)",
                    ));
                }
                other if !other.starts_with("--") => filter = Some(other.to_ascii_lowercase()),
                other => {
                    return CommandResult::Text(format!(
                        "Unknown flag `{other}`. See `/tools --help`."
                    ));
                }
            }
        }

        let all = self.tool_registry.metadata();
        let total = all.len();
        let mut visible: Vec<crate::tools::ToolMetadata> = all
            .into_iter()
            .filter(|t| match mode {
                ToolListMode::All => true,
                ToolListMode::ReadOnly => t.read_only,
                ToolListMode::Writes => !t.read_only,
            })
            .filter(|t| match &filter {
                Some(f) => t.name.to_ascii_lowercase().contains(f),
                None => true,
            })
            .collect();
        visible.sort_by_key(|t| t.name);

        let mut lines = Vec::with_capacity(visible.len() + 3);
        let mode_label = match mode {
            ToolListMode::All => "all",
            ToolListMode::ReadOnly => "read-only",
            ToolListMode::Writes => "writes",
        };
        let filter_part = match &filter {
            Some(f) => format!(" matching `{f}`"),
            None => String::new(),
        };
        lines.push(format!(
            "Tools ({} of {total}, {mode_label}){filter_part}:",
            visible.len()
        ));
        if visible.is_empty() {
            lines.push(String::from(
                "  (no tools match — try `/tools` to see everything)",
            ));
        } else {
            for t in &visible {
                let marker = if t.read_only { "·" } else { "✎" };
                if wants_details {
                    lines.push(format!("  {marker} {} — {}", t.name, t.description));
                } else {
                    let one_line = first_line_summary(t.description, 80);
                    lines.push(format!("  {marker} {:<22} {one_line}", t.name));
                }
            }
            lines.push(String::new());
            lines.push(String::from(
                "Legend: `·` read-only · `✎` may modify state. Use `/tools <filter>` to narrow.",
            ));
        }

        CommandResult::Text(lines.join("\n"))
    }

    /// `/think [on|off|low|medium|high|status|budget <N>|auto]`
    ///
    /// Toggles the extended-thinking (reasoning) directive on the agent
    /// loop without rebuilding the backend. Persisted only for the
    /// lifetime of the session — restarting `deeptide-rs` reverts to
    /// the construction-time default (which may itself be set via
    /// `--thinking auto|low|medium|high` on the CLI).
    ///
    /// Semantic mapping (matches `ThinkingConfig::from_label`):
    ///   * `on` / `enable` / `medium` → 16K thinking budget
    ///   * `low`                        →  4K
    ///   * `high`                       → 32K
    ///   * `off` / `disable`            → explicit "disabled"
    ///   * `auto` / `default`           → clear the override
    ///   * `budget <N>`                 → enabled with custom budget
    ///
    /// `status` (the no-arg default) prints a human-readable summary
    /// without changing anything — useful for confirming "is thinking
    /// actually on for this session?"
    fn execute_think_command(&mut self, args: &str) -> CommandResult {
        use crate::api::ThinkingConfig;

        let trimmed = args.trim();
        let (verb, rest) = match trimmed.split_once(char::is_whitespace) {
            Some((head, tail)) => (head.to_ascii_lowercase(), tail.trim()),
            None => (trimmed.to_ascii_lowercase(), ""),
        };

        match verb.as_str() {
            "" | "status" | "show" => {
                CommandResult::Text(render_think_status(self.agent_loop.thinking_override()))
            }
            "auto" | "default" | "clear" => {
                self.agent_loop.set_thinking_override(None);
                CommandResult::Text(String::from(
                    "Thinking override cleared. Subsequent turns will use the backend's default (whatever was set via --thinking / config).",
                ))
            }
            "off" | "disable" | "disabled" | "none" => {
                self.agent_loop
                    .set_thinking_override(Some(ThinkingConfig::disabled()));
                CommandResult::Text(String::from(
                    "Thinking disabled. The model will respond without an extended-thinking pass.",
                ))
            }
            "on" | "enable" | "enabled" | "medium" => {
                self.agent_loop
                    .set_thinking_override(Some(ThinkingConfig::medium()));
                CommandResult::Text(String::from(
                    "Thinking enabled at medium budget (16 000 tokens).",
                ))
            }
            "low" => {
                self.agent_loop
                    .set_thinking_override(Some(ThinkingConfig::low()));
                CommandResult::Text(String::from(
                    "Thinking enabled at low budget (4 000 tokens).",
                ))
            }
            "high" => {
                self.agent_loop
                    .set_thinking_override(Some(ThinkingConfig::high()));
                CommandResult::Text(String::from(
                    "Thinking enabled at high budget (32 000 tokens).",
                ))
            }
            "budget" => {
                if rest.is_empty() {
                    return CommandResult::Text(String::from(
                        "Usage: /think budget <tokens>\n\nEnables thinking with a custom budget. Anthropic requires the budget to be at least 1 024 tokens; values are clamped to a sane ceiling of 64 000 to avoid runaway prompt costs.",
                    ));
                }
                let parsed: usize = match rest.parse() {
                    Ok(n) => n,
                    Err(_) => {
                        return CommandResult::Text(format!(
                            "`{rest}` is not a valid token count. Try `/think budget 8000`."
                        ));
                    }
                };
                let clamped = parsed.clamp(1_024, 64_000);
                // The struct only exposes preset constructors + the
                // `from_label` shortcut, but it's a plain struct, so we
                // construct it directly here to thread the user's
                // exact budget through.
                let cfg = ThinkingConfig {
                    kind: String::from("enabled"),
                    budget_tokens: Some(clamped),
                };
                self.agent_loop.set_thinking_override(Some(cfg));
                let note = if clamped != parsed {
                    format!(" (clamped from {parsed})")
                } else {
                    String::new()
                };
                CommandResult::Text(format!("Thinking enabled with budget {clamped}{note}."))
            }
            other => CommandResult::Text(format!(
                "Unknown subcommand `/think {other}`. Try `/think status`, `/think on|off|auto|low|medium|high`, or `/think budget <N>`.",
            )),
        }
    }

    /// `/search <query>` — case-insensitive substring search across
    /// the in-memory transcript. Returns matching lines with role +
    /// turn index + a brief context window, so the user can quickly
    /// rediscover something the model or they said earlier.
    ///
    /// We deliberately scope this to the *current* session's messages
    /// (`self.agent_loop.messages()`) rather than spelunking the
    /// session store on disk — searching the latter would need an
    /// extra index step and is better served by `/sessions` + manual
    /// follow-up. Doing both would also surface stale results from
    /// other sessions, which is rarely what the user wants when they
    /// type `/search "the bug we just fixed"`.
    fn execute_search_command(&self, args: &str) -> CommandResult {
        let query_raw = args.trim();
        let (query, want_regex) = if let Some(rest) = query_raw.strip_prefix("--regex ") {
            (rest.trim(), true)
        } else if let Some(rest) = query_raw.strip_prefix("-r ") {
            (rest.trim(), true)
        } else {
            (query_raw, false)
        };

        if query.is_empty() {
            return CommandResult::Text(String::from(
                "Usage: /search <query>            (case-insensitive substring)\n       /search --regex <pattern>  (Rust regex syntax)\n\nSearches the current session's user + assistant messages. Use `/sessions` to switch session first if you want to search history from another session.",
            ));
        }

        let regex = if want_regex {
            match regex::RegexBuilder::new(query)
                .case_insensitive(true)
                .build()
            {
                Ok(re) => Some(re),
                Err(error) => {
                    return CommandResult::Text(format!("Invalid regex: {error}"));
                }
            }
        } else {
            None
        };
        let query_lower = query.to_ascii_lowercase();

        let mut hits: Vec<(usize, &str, String)> = Vec::new();
        for (idx, message) in self.agent_loop.messages().iter().enumerate() {
            let role = match message.role {
                crate::MessageRole::User => "user",
                crate::MessageRole::Assistant => "assistant",
            };
            for line in message.content.lines() {
                let matched = match &regex {
                    Some(re) => re.is_match(line),
                    None => line.to_ascii_lowercase().contains(&query_lower),
                };
                if matched {
                    hits.push((idx, role, line.trim().to_owned()));
                }
            }
        }

        const MAX_HITS: usize = 32;
        let total = hits.len();
        if total == 0 {
            return CommandResult::Text(format!(
                "No matches for `{query}` in this session's {} message(s).",
                self.agent_loop.messages().len()
            ));
        }
        let shown = total.min(MAX_HITS);
        let mut lines = Vec::with_capacity(shown + 3);
        lines.push(format!("Search hits ({shown} of {total}) for `{query}`:"));
        for (idx, role, body) in hits.iter().take(shown) {
            // Trim very long lines so a hit on a multi-KB tool result
            // doesn't blow up the terminal.
            let preview: String = body.chars().take(140).collect();
            let ellipsis = if body.chars().count() > 140 {
                "…"
            } else {
                ""
            };
            lines.push(format!("  [#{idx:>3} {role:<9}] {preview}{ellipsis}"));
        }
        if total > MAX_HITS {
            lines.push(format!(
                "  … {} more hit(s) suppressed. Refine the query to narrow.",
                total - MAX_HITS
            ));
        }
        CommandResult::Text(lines.join("\n"))
    }

    // ── /checkpoint + /rewind ────────────────────────────────────────
    //
    // Snapshots the live transcript into an in-memory `CheckpointStore`
    // so the user can rewind multi-turn mistakes without restarting
    // the session. The store is intentionally NOT persisted — for
    // crash-safe resume the user reaches for `/sessions` + `/resume`.
    //
    // The two commands stay decoupled: `/checkpoint …` only ever
    // mutates the store; `/rewind …` only ever reads from it and
    // restores into `agent_loop`. This makes both unit-testable
    // without standing up an integration harness.

    fn execute_checkpoint_command(&mut self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        let (verb_raw, rest) = match trimmed.split_once(char::is_whitespace) {
            Some((head, tail)) => (head, tail.trim()),
            None => (trimmed, ""),
        };
        let verb = verb_raw.to_ascii_lowercase();

        // Known sub-verbs take a fixed dispatch table; anything else is
        // treated as the leading word of a free-form label so the common
        // case `/checkpoint pre-refactor` Just Works.
        match verb.as_str() {
            "" | "save" | "create" | "new" => self.checkpoint_save(rest),
            "list" | "ls" | "show" => self.checkpoint_list(),
            "restore" | "load" | "rewind" => self.checkpoint_restore(rest),
            "drop" | "delete" | "rm" => self.checkpoint_drop(rest),
            "clear" | "reset" => {
                let n = self.checkpoints.clear();
                CommandResult::Text(if n == 0 {
                    String::from("No checkpoints to clear.")
                } else {
                    format!("Cleared {n} checkpoint(s).")
                })
            }
            "--help" | "help" | "-h" => CommandResult::Text(checkpoint_help_text()),
            _ => self.checkpoint_save(trimmed),
        }
    }

    fn execute_rewind_command(&mut self, args: &str) -> CommandResult {
        // `/rewind` is a thin alias for `/checkpoint restore` with the
        // empty-selector default (newest snapshot).
        self.checkpoint_restore(args.trim())
    }

    /// Push an in-memory checkpoint of the current transcript (for programmatic
    /// callers that mutate history, e.g. an import handoff). No-op when the
    /// transcript is empty. Mirrors `checkpoint_save` without the user-facing
    /// validation / messaging.
    fn snapshot_checkpoint(&mut self, label: &str) {
        let messages = self.agent_loop.messages().to_vec();
        if messages.is_empty() {
            return;
        }
        let created_at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        self.checkpoints.push(crate::checkpoints::Checkpoint {
            id: crate::checkpoints::fresh_checkpoint_id(),
            label: label.to_owned(),
            created_at,
            message_count: messages.len(),
            model: self.agent_loop.model().to_owned(),
            messages,
        });
    }

    fn checkpoint_save(&mut self, label_args: &str) -> CommandResult {
        // Reject the no-op case early: a snapshot of an empty
        // transcript would just clutter the list.
        let messages = self.agent_loop.messages().to_vec();
        if messages.is_empty() {
            return CommandResult::Text(String::from(
                "Nothing to checkpoint yet — send at least one message first.",
            ));
        }

        // The label is intentionally raw (no trimming of inner
        // whitespace) but we strip the outer trim so `/checkpoint   foo`
        // produces label "foo" instead of "  foo".
        let label = label_args.trim().to_owned();
        // Validate label: forbid newlines (would break the listing) and
        // truncate at 64 chars (UI-friendly + selector lookup still
        // works on the full string).
        if label.contains('\n') {
            return CommandResult::Text(String::from("Checkpoint labels cannot contain newlines."));
        }
        let label = if label.chars().count() > 64 {
            let truncated: String = label.chars().take(64).collect();
            truncated
        } else {
            label
        };

        let created_at = time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_default();
        let id = crate::checkpoints::fresh_checkpoint_id();
        let message_count = messages.len();
        let model = self.agent_loop.model().to_owned();

        let checkpoint = crate::checkpoints::Checkpoint {
            id: id.clone(),
            label: label.clone(),
            created_at,
            message_count,
            model,
            messages,
        };

        let was_full = self.checkpoints.len() >= crate::checkpoints::MAX_CHECKPOINTS;
        self.checkpoints.push(checkpoint);

        let label_part = if label.is_empty() {
            String::new()
        } else {
            format!(" `{label}`")
        };
        let evict_note = if was_full {
            format!(
                " (oldest evicted — cap is {})",
                crate::checkpoints::MAX_CHECKPOINTS
            )
        } else {
            String::new()
        };
        CommandResult::Text(format!(
            "✔ checkpoint {id}{label_part}: captured {message_count} message(s){evict_note}. Restore with `/rewind {id}` or `/checkpoint restore {id}`.",
        ))
    }

    fn checkpoint_list(&self) -> CommandResult {
        if self.checkpoints.is_empty() {
            return CommandResult::Text(String::from(
                "No checkpoints in this session. Use `/checkpoint [label]` to take one.",
            ));
        }
        let total = self.checkpoints.len();
        let mut lines = Vec::with_capacity(total + 2);
        lines.push(format!(
            "Checkpoints ({total}/{cap}):",
            cap = crate::checkpoints::MAX_CHECKPOINTS,
        ));
        for (i, cp) in self.checkpoints.iter().enumerate() {
            lines.push(cp.summarise(i));
        }
        lines.push(String::from(
            "  Restore with `/rewind <id|index|label>` (no arg = newest).",
        ));
        CommandResult::Text(lines.join("\n"))
    }

    fn checkpoint_restore(&mut self, selector: &str) -> CommandResult {
        use crate::checkpoints::SelectorOutcome;

        let outcome = self.checkpoints.resolve(selector);
        let idx = match outcome {
            SelectorOutcome::Found(i) => i,
            SelectorOutcome::Empty => {
                return CommandResult::Text(String::from(
                    "No checkpoints in this session. Use `/checkpoint [label]` to take one.",
                ));
            }
            SelectorOutcome::NotFound => {
                return CommandResult::Text(format!(
                    "No checkpoint matches `{selector}`. Use `/checkpoints` to list available IDs/labels.",
                ));
            }
            SelectorOutcome::Ambiguous { matches } => {
                return CommandResult::Text(format!(
                    "Ambiguous selector `{selector}` — matches {}. Use a longer ID prefix or the 1-based index.",
                    matches.join(", "),
                ));
            }
        };

        let Some(checkpoint) = self.checkpoints.get(idx).cloned() else {
            // Defensive — `resolve` should never return an out-of-range index.
            return CommandResult::Text(String::from("Internal error: checkpoint index drifted."));
        };

        let before = self.agent_loop.messages().len();
        let after = checkpoint.message_count;
        self.agent_loop
            .replace_messages(checkpoint.messages.clone());

        let label_part = if checkpoint.label.is_empty() {
            String::new()
        } else {
            format!(" `{}`", checkpoint.label)
        };
        let direction = if before == after {
            "(transcript already matches checkpoint)"
        } else if before > after {
            "rewound"
        } else {
            "advanced"
        };
        CommandResult::Text(format!(
            "✔ {direction} to checkpoint {id}{label_part}: {before} → {after} message(s).",
            id = checkpoint.id,
        ))
    }

    fn checkpoint_drop(&mut self, selector: &str) -> CommandResult {
        use crate::checkpoints::SelectorOutcome;

        if selector.is_empty() {
            return CommandResult::Text(String::from("Usage: /checkpoint drop <id|index|label>"));
        }

        let outcome = self.checkpoints.resolve(selector);
        let idx = match outcome {
            SelectorOutcome::Found(i) => i,
            SelectorOutcome::Empty => {
                return CommandResult::Text(String::from("No checkpoints to drop."));
            }
            SelectorOutcome::NotFound => {
                return CommandResult::Text(format!("No checkpoint matches `{selector}`.",));
            }
            SelectorOutcome::Ambiguous { matches } => {
                return CommandResult::Text(format!(
                    "Ambiguous selector `{selector}` — matches {}.",
                    matches.join(", "),
                ));
            }
        };

        match self.checkpoints.remove(idx) {
            Some(cp) => CommandResult::Text(format!(
                "Dropped checkpoint {id}{label}.",
                id = cp.id,
                label = if cp.label.is_empty() {
                    String::new()
                } else {
                    format!(" `{}`", cp.label)
                },
            )),
            None => CommandResult::Text(String::from("Internal error: drop index drifted.")),
        }
    }

    /// Read-only view of in-session checkpoints. Used by tests and
    /// potential future UI surfaces (e.g. a status-bar segment showing
    /// "ckpt 3"). Not part of any slash command grammar.
    pub fn checkpoint_count(&self) -> usize {
        self.checkpoints.len()
    }

    // ── /usage ───────────────────────────────────────────────────────
    //
    // Per-tool observability. The data source is
    // `AgentLoop::tool_usage()`, populated automatically on every
    // dispatch in `execute_tool_call`.  The command lives in the REPL
    // (not on `AgentLoop` directly) because it needs to render — the
    // renderer stays decoupled so it's testable without firing a real
    // turn.

    fn execute_usage_command(&mut self, args: &str) -> CommandResult {
        let trimmed = args.trim();
        let (verb, _rest) = match trimmed.split_once(char::is_whitespace) {
            Some((head, tail)) => (head.to_ascii_lowercase(), tail.trim()),
            None => (trimmed.to_ascii_lowercase(), ""),
        };

        match verb.as_str() {
            "" | "show" | "list" => {
                CommandResult::Text(render_tool_usage(self.agent_loop.tool_usage(), false))
            }
            "--json" | "json" => {
                CommandResult::Text(render_tool_usage(self.agent_loop.tool_usage(), true))
            }
            "reset" | "clear" => {
                let total = self.agent_loop.tool_usage().total_invocations();
                self.agent_loop.reset_tool_usage();
                CommandResult::Text(if total == 0 {
                    String::from("Tool usage telemetry already empty.")
                } else {
                    format!("Cleared {total} tool invocation sample(s).")
                })
            }
            "--help" | "help" | "-h" => CommandResult::Text(String::from(
                "Usage: /usage [show | --json | reset]\n\n\
                 Per-tool activity dashboard. Tracks invocations, success/error\n\
                 counts, total + average + peak duration, and cumulative result\n\
                 bytes for every tool dispatched in this session.\n\
                 \n\
                 Subcommands:\n  \
                   /usage           Render the dashboard (default)\n  \
                   /usage --json    Emit a machine-readable JSON snapshot\n  \
                   /usage reset     Discard every sample (start over)",
            )),
            other => CommandResult::Text(format!(
                "Unknown subcommand `/usage {other}`. Try `/usage`, `/usage --json`, or `/usage reset`.",
            )),
        }
    }

    // ── /test + /lint ────────────────────────────────────────────────
    //
    // Auto-detect the project's primary toolchain (Cargo, package.json,
    // pyproject.toml, go.mod, Gemfile) and either suggest or run the
    // corresponding test/lint command. The default is to *suggest*
    // (print the command, don't execute) because we want users to opt
    // into long-running subprocess execution explicitly; pass `--run`
    // or `-r` to actually fire it.

    fn execute_toolchain_command(&self, args: &str, action: ToolchainAction) -> CommandResult {
        let mut run = false;
        let mut want_help = false;
        for token in args.split_whitespace() {
            match token {
                "--run" | "-r" | "run" => run = true,
                "--help" | "-h" | "help" => want_help = true,
                other => {
                    return CommandResult::Text(format!(
                        "Unknown flag `{other}`. Usage: /{verb} [--run]",
                        verb = action.verb(),
                    ));
                }
            }
        }

        if want_help {
            return CommandResult::Text(action.help_text());
        }

        let cwd = &self.tool_context.cwd;
        let kinds = crate::project_toolchain::detect_toolchains(cwd);
        if kinds.is_empty() {
            return CommandResult::Text(format!(
                "No known project toolchain found under {}. \
                 Looked for: Cargo.toml, package.json, pyproject.toml, setup.py, requirements.txt, go.mod, Gemfile.",
                cwd.display(),
            ));
        }

        // Pick the first detected toolchain by stated precedence. When
        // a polyglot repo has multiple, show the others as a hint so
        // the user can re-run with a more specific tool.
        let primary = &kinds[0];
        let other_kinds: Vec<&str> = kinds.iter().skip(1).map(|k| k.label()).collect();
        let command = match action {
            ToolchainAction::Test => primary.test_command(),
            ToolchainAction::Lint => primary.lint_command(),
        };
        let Some(cmd) = command else {
            return CommandResult::Text(format!(
                "Detected {} but no default {} command. Pass `--run` flags or run manually.",
                primary.label(),
                action.verb(),
            ));
        };

        if !run {
            let mut out = format!(
                "Detected toolchain: {}\nSuggested {} command:\n  {}\n",
                primary.label(),
                action.verb(),
                cmd.display(),
            );
            if !other_kinds.is_empty() {
                out.push_str(&format!("Also detected: {}\n", other_kinds.join(", ")));
            }
            out.push_str(&format!(
                "Run it inside the REPL with `/{verb} --run` (synchronous; output streamed when done).",
                verb = action.verb(),
            ));
            return CommandResult::Text(out);
        }

        CommandResult::Text(run_toolchain_command(cwd, &cmd, action))
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
        let commands_cwd = cwd.clone();

        CommandContext::builder()
            .clear_conversation(|| Some(String::new()))
            .compact_conversation(|| {})
            // Built-in command list plus any user-authored markdown commands
            // discovered for this cwd, so the latter tab-complete and show in
            // /help just like the built-ins.
            .all_commands(move || {
                let mut sources = repl_command_sources();
                for name in discover_command_names(&commands_cwd) {
                    // A custom command never shadows a built-in in completion.
                    if sources
                        .iter()
                        .any(|s| s.name == name || s.aliases.iter().any(|a| a == &name))
                    {
                        continue;
                    }
                    sources.push(CommandCompletionSource::new(
                        name,
                        Vec::<&str>::new(),
                        "Custom command (project .deeptide/commands)",
                        "",
                    ));
                }
                sources
            })
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
/// Path of the marker written after the first interactive launch.
fn onboarded_marker() -> std::path::PathBuf {
    crate::memory::MemorySystem::tide_config_dir().join("onboarded")
}

/// True until [`mark_onboarded`] has run (i.e. this looks like a first install).
pub fn is_first_run() -> bool {
    !onboarded_marker().exists()
}

/// Record that the first-run onboarding has been shown, so it never fires again.
/// Returns `true` if the marker was persisted; `false` if the write failed
/// (e.g. an unwritable config dir) — the caller can warn so the user isn't
/// silently re-nudged (and the discovery walk re-run) on every startup.
#[must_use]
pub fn mark_onboarded() -> bool {
    let path = onboarded_marker();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    std::fs::write(&path, "1\n").is_ok()
}

/// Result of bounding a set of `(header, flattened-body)` sessions into one
/// budget-capped corpus for `/import all`.
struct BulkImportCorpus {
    combined: String,
    included: usize,
    skipped: usize,
}

/// Concatenate `sessions` (newest-first `(header, body)` pairs) into a single
/// string capped at `width_budget` **display columns**. The newest session is
/// always included (truncated if it alone exceeds the budget); each subsequent
/// session is truncated to the columns remaining, and once fewer than
/// `min_content_slice` columns (beyond its header) are left, the rest are
/// skipped. Width-aware so the cap is a real ceiling on the model-bound text,
/// never split across a multibyte char. Pure — unit-tested for the ceiling.
fn build_bulk_import_corpus(
    sessions: &[(String, String)],
    width_budget: usize,
    min_content_slice: usize,
) -> BulkImportCorpus {
    let mut combined = String::new();
    let mut used = 0usize; // display columns accumulated so far
    let mut included = 0usize;
    let mut skipped = 0usize;
    for (header, body) in sessions {
        let header_width = crate::width::display_width(header);
        let remaining = width_budget.saturating_sub(used);
        if included > 0 && remaining <= header_width + min_content_slice {
            skipped += 1;
            continue;
        }
        let content_budget = remaining.saturating_sub(header_width);
        let body = crate::import::truncate(body, content_budget);
        used += header_width + crate::width::display_width(&body);
        combined.push_str(header);
        combined.push_str(&body);
        included += 1;
    }
    BulkImportCorpus {
        combined,
        included,
        skipped,
    }
}

/// Shorten a (possibly UUID-long) session id for display: first dash-group,
/// capped, so listings and import receipts stay tidy.
fn session_short(id: &str) -> String {
    let head = id.split('-').next().unwrap_or(id);
    if head.chars().count() >= 8 {
        head.chars().take(8).collect()
    } else {
        id.chars().take(16).collect()
    }
}

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

/// Render the system-message line that announces `@file` expansion to
/// the user. Shows the per-reference outcome so the user knows exactly
/// which `@` worked and which didn't.
///
/// Examples:
///   `📎 attached 1 file (3.2 KB): src/main.rs`
///   `📎 attached 2 files (5.0 KB): a.rs, b.rs · skipped: @nope.rs (not found)`
fn format_attachment_notice(result: &crate::at_references::ExpansionResult) -> String {
    use crate::at_references::ExpansionStatus;

    let text_files: Vec<&crate::at_references::AttachedFile> = result
        .attachments
        .iter()
        .filter(|a| matches!(a.status, ExpansionStatus::Inlined { .. }))
        .collect();
    let images: Vec<&crate::at_references::AttachedFile> =
        result.attachments.iter().filter(|a| a.is_image()).collect();
    let skipped: Vec<&crate::at_references::AttachedFile> = result
        .attachments
        .iter()
        .filter(|a| !a.is_inlined())
        .collect();

    let mut segments: Vec<String> = Vec::new();
    if !text_files.is_empty() {
        let total_bytes = result.total_inlined_bytes();
        let size = humanize_bytes(total_bytes);
        let noun = if text_files.len() == 1 {
            "file"
        } else {
            "files"
        };
        let paths = text_files
            .iter()
            .map(|a| a.display_path.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        segments.push(format!(
            "📎 attached {} {noun} ({size}): {paths}",
            text_files.len()
        ));
    }
    if !images.is_empty() {
        let noun = if images.len() == 1 { "image" } else { "images" };
        // Show each image's path + format so the user can immediately
        // tell what the model has been told about.
        let parts: Vec<String> = images
            .iter()
            .map(|a| {
                let fmt = match &a.status {
                    ExpansionStatus::Image { kind, .. } => kind.label(),
                    _ => "?",
                };
                let bytes = match &a.status {
                    ExpansionStatus::Image { bytes, .. } => humanize_bytes(*bytes),
                    _ => String::new(),
                };
                format!("{} [{fmt}, {bytes}]", a.display_path)
            })
            .collect();
        segments.push(format!(
            "🖼 attached {} {noun} (Vision-tool hint emitted): {}",
            images.len(),
            parts.join(", "),
        ));
    }
    if segments.is_empty() {
        segments.push(String::from("📎 no files attached"));
    }

    let mut head = segments.join(" · ");

    // Only count actual skips (not images, which are listed above).
    let real_skipped: Vec<&crate::at_references::AttachedFile> = skipped
        .into_iter()
        .filter(|a| !matches!(a.status, ExpansionStatus::Image { .. }))
        .collect();
    if !real_skipped.is_empty() {
        let parts: Vec<String> = real_skipped
            .iter()
            .map(|a| {
                let reason = match &a.status {
                    ExpansionStatus::NotFound => "not found".to_owned(),
                    ExpansionStatus::Directory => "directory".to_owned(),
                    ExpansionStatus::Binary => "binary".to_owned(),
                    ExpansionStatus::TooLarge { bytes, limit } => format!(
                        "too large {} > {}",
                        humanize_bytes(*bytes),
                        humanize_bytes(*limit)
                    ),
                    ExpansionStatus::BudgetExhausted => "budget exhausted".to_owned(),
                    ExpansionStatus::Unreadable { reason } => format!("unreadable: {reason}"),
                    ExpansionStatus::Image { .. } | ExpansionStatus::Inlined { .. } => {
                        unreachable!()
                    }
                };
                format!("{} ({reason})", a.reference)
            })
            .collect();
        head.push_str(" · skipped: ");
        head.push_str(&parts.join(", "));
    }

    head
}

/// Render a byte count using KB / MB / GB units. Two significant digits
/// (e.g. `3.2 KB`, `1.4 MB`) — enough resolution for prompt-budget
/// triage without making the line too long.
fn humanize_bytes(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let n = bytes as f64;
    if n >= GB {
        format!("{:.1} GB", n / GB)
    } else if n >= MB {
        format!("{:.1} MB", n / MB)
    } else if n >= KB {
        format!("{:.1} KB", n / KB)
    } else {
        format!("{} B", bytes)
    }
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
        } => Some(ReplEvent::System(SystemMessage::ToolBatch {
            label,
            failed_count,
        })),
        AgentLoopEvent::Terminal(AgentTerminalEvent::MaxTurnsReached { cap }) => {
            // Surface the actual cap and how to raise it. Users hitting
            // this in long refactor sessions want a one-line hint, not
            // a generic "Maximum turns reached." with no recourse.
            Some(ReplEvent::System(SystemMessage::Notice(format!(
                "Maximum turns reached ({cap}). Raise with --max-turns N or set `max_turns` in settings.json."
            ))))
        }
        AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(error)) => Some(ReplEvent::System(
            SystemMessage::Notice(format!("Model error: {error}")),
        )),
        AgentLoopEvent::Terminal(AgentTerminalEvent::Blocked) => {
            Some(ReplEvent::System(SystemMessage::Notice(String::from(
                "Context window full: the transcript exceeds the model's limit even after compaction. Start a new session (/new) or trim context.",
            ))))
        }
        AgentLoopEvent::ToolResult {
            tool_call,
            content,
            is_error,
        } => Some(ReplEvent::System(build_tool_system_message(
            &tool_call.name,
            &tool_call.id,
            &tool_call.input,
            &content,
            is_error,
        ))),
        AgentLoopEvent::Compaction(report) => Some(ReplEvent::System(SystemMessage::Compaction {
            compressed_messages: report.compressed_messages,
            tokens_after: report.tokens_after,
        })),
        AgentLoopEvent::Terminal(AgentTerminalEvent::Interrupted) => Some(ReplEvent::System(
            SystemMessage::Notice(String::from("⎿ Interrupted by user")),
        )),
        AgentLoopEvent::User(_) | AgentLoopEvent::Terminal(AgentTerminalEvent::Complete) => None,
    }
}

/// Build a structured [`SystemMessage::Tool`] from a single tool result.
/// Decides whether the body is small enough to inline below the summary
/// (`should_expand_tool_result`); the CLI then renders the body verbatim
/// without re-applying markdown styling so tool output (file contents,
/// command output, etc.) is preserved exactly.
fn build_tool_system_message(
    tool_name: &str,
    tool_id: &str,
    input: &serde_json::Value,
    content: &str,
    is_error: bool,
) -> SystemMessage {
    let summary = ToolResultSummaryFormatter::summary(tool_name, content, is_error);
    let subject = extract_tool_subject(tool_name, input);
    let trimmed = content.trim();

    let body = if is_error
        || trimmed.is_empty()
        || ToolResultSummaryFormatter::should_mute_appearance(tool_name, content, is_error)
        || !should_expand_tool_result(trimmed)
    {
        None
    } else {
        Some(trimmed.to_owned())
    };

    SystemMessage::Tool {
        name: tool_name.to_owned(),
        call_id: tool_id.to_owned(),
        summary,
        is_error,
        body,
        subject,
    }
}

/// Classify a slash-command line by whether its handler will invoke
/// the agent (model round-trip + tool execution) or can run entirely
/// locally. Used by the CLI to decide whether to spawn the
/// turn-scoped spinner + raw-mode queue editor before calling
/// [`ReplSession::submit`].
///
/// Why this lives in `deeptide-core`
/// =================================
///
/// The dispatch table inside `execute_command` is the source of
/// truth — every new agent-invoking slash command should be added to
/// the constants below in the same patch that wires it up, otherwise
/// the CLI will skip the spinner/editor and the user will see a
/// silent terminal while the model runs. Keeping the list adjacent
/// to the dispatch makes that coupling easy to audit.
///
/// Returns `true` only for commands whose handler calls
/// `agent_loop.run(...)` (or kicks off an external editor that
/// ultimately submits a prompt). Everything else — including
/// `/exit`, `/clear`, `/new`, `/help`, `/status`, `/cost`, `/copy`,
/// `/diff`, `/branch`, `/tps`, `/queue`, … — is local and should
/// execute instantly without the spinner flash that previously made
/// the user wonder if `/exit` was "talking to the model".
///
/// Empty / non-slash input returns `false` (the caller is expected
/// to short-circuit on non-slash input before consulting this).
pub fn slash_command_invokes_agent(line: &str) -> bool {
    /// Commands whose handler runs the model. Every entry maps to a
    /// branch of `execute_command` that calls `self.agent_loop.run`.
    const AGENT_INVOKING: &[&str] = &[
        // /retry resubmits the last user prompt
        "retry",
        "r",
        "again",
        // Skill-shortcut commands all run the model after staging a
        // skill payload — see `execute_commit_command` etc.
        "commit",
        "review",
        "simplify",
        "dream",
        "goal",
        "objective",
        "reminder",
        "anchor",
        "reorient",
        // /vim opens an external editor and (when the user saves
        // non-empty content) submits the buffer to the model.
        "vim",
        "edit",
        "e",
        "compose",
    ];

    let trimmed = line.trim();
    let Some(command_line) = trimmed.strip_prefix('/') else {
        return false;
    };
    let name = command_line
        .split(char::is_whitespace)
        .next()
        .unwrap_or_default()
        .to_ascii_lowercase();
    AGENT_INVOKING.contains(&name.as_str())
}

/// Pull a short, human-readable "subject" out of a tool call's input
/// JSON — the file path for Read / Write / Edit / AppendFile /
/// NotebookEdit, the command for Bash / Monitor, the URL for
/// WebFetch, the pattern for Grep / Glob, the search term for
/// WebSearch / MemorySearch, etc.
///
/// Why a separate helper, not part of `ToolResultSummaryFormatter`?
/// The summary formatter only sees the *result*; the subject lives
/// in the *input*. They're independent slices of the call (output
/// shape vs input shape) and conflating them would make either
/// piece harder to evolve.
///
/// Returns `None` when:
///   * the tool isn't one we recognise, OR
///   * the input shape doesn't carry an obvious subject field, OR
///   * the candidate string is empty / whitespace.
///
/// The returned string is already truncated to `MAX_SUBJECT_LEN`
/// chars (UTF-8 aware) so the CLI can splice it directly into a
/// single-line header without further checks.
pub(crate) fn extract_tool_subject(tool_name: &str, input: &serde_json::Value) -> Option<String> {
    const MAX_SUBJECT_LEN: usize = 80;

    let raw = match tool_name.to_ascii_lowercase().as_str() {
        // Filesystem tools — the file path is the subject.
        "read" | "write" | "edit" | "appendfile" | "notebookedit" => {
            input.get("file_path").and_then(|v| v.as_str())
        }
        // Multi-file read — show count instead of the full list.
        "readfiles" => {
            let count = input
                .get("file_paths")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .or_else(|| {
                    input
                        .get("paths")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                })?;
            return Some(format!("{count} file(s)"));
        }
        // Shell-flavoured tools — the command is the subject. Newlines
        // collapsed so it fits on one line; the diff-preview is where
        // the full multi-line text lives.
        "bash" | "monitor" | "shell" => input.get("command").and_then(|v| v.as_str()),
        // Search / discovery — the pattern.
        "grep" => input.get("pattern").and_then(|v| v.as_str()),
        "glob" => input.get("pattern").and_then(|v| v.as_str()),
        // Web.
        "webfetch" => input.get("url").and_then(|v| v.as_str()),
        "websearch" => input.get("query").and_then(|v| v.as_str()),
        // Memory & search.
        "memorysearch" => input.get("query").and_then(|v| v.as_str()),
        "memorywrite" => input.get("key").and_then(|v| v.as_str()),
        // Tasks / planning — the description / title.
        "todowrite" => input
            .get("todos")
            .and_then(|v| v.as_array())
            .and_then(|a| a.first())
            .and_then(|t| t.get("content").or_else(|| t.get("title")))
            .and_then(|v| v.as_str()),
        // Subagent / skill — name them so the user knows which one ran.
        "agent" => input
            .get("subagent_type")
            .or_else(|| input.get("agent_type"))
            .or_else(|| input.get("name"))
            .and_then(|v| v.as_str()),
        "skill" => input
            .get("name")
            .or_else(|| input.get("skill"))
            .and_then(|v| v.as_str()),
        // MCP — surface the tool ID.
        "mcp" => input
            .get("tool")
            .or_else(|| input.get("name"))
            .and_then(|v| v.as_str()),
        // Sleep / wait.
        "sleep" => {
            let secs = input.get("seconds").and_then(|v| v.as_f64())?;
            return Some(format!("{secs}s"));
        }
        _ => None,
    };

    let raw = raw?;
    let single_line = raw
        .lines()
        .next()
        .unwrap_or(raw)
        .trim_end()
        .trim_end_matches('\\');
    let collapsed = single_line.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return None;
    }
    if collapsed.chars().count() > MAX_SUBJECT_LEN {
        let mut truncated: String = collapsed.chars().take(MAX_SUBJECT_LEN).collect();
        truncated.push('…');
        Some(truncated)
    } else {
        Some(collapsed)
    }
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
        CommandCompletionSource::from_command(&ExitCommand),
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
            "List saved sessions (--all includes Claude Code / Codex)",
            "/sessions [filter] [--all]",
        ),
        CommandCompletionSource::new(
            "resume",
            ["load", "restore"],
            "Resume a previous session",
            "/resume [session-id]",
        ),
        CommandCompletionSource::new(
            "import",
            Vec::<&str>::new(),
            "Import a session from another agent (Claude Code / Codex)",
            "/import <claude|codex|deeptide> [<id>|--latest] [--as memory|context]",
        ),
        CommandCompletionSource::new(
            "continue",
            ["handoff"],
            "Hand off the newest foreign session into the live conversation",
            "/continue [claude|codex|deeptide]",
        ),
        CommandCompletionSource::new(
            "deep-seek",
            ["deepseek", "research"],
            "Run a bounded web-research pass (search → fetch → cross-check → cited answer)",
            "/deep-seek <question>",
        ),
        CommandCompletionSource::new(
            "version",
            ["ver"],
            "Show the running deeptide build (version, commit, date)",
            "/version",
        ),
        CommandCompletionSource::new(
            "todo",
            ["todos", "tasklist"],
            "List the agent's current todo backlog (status-bar count, expanded)",
            "/todo",
        ),
        CommandCompletionSource::new(
            "suggest",
            ["suggestions"],
            "Toggle or re-show follow-up suggestions",
            "/suggest [on|off|show]",
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
            "explain",
            Vec::<&str>::new(),
            "Explain a file, symbol, or area of the codebase (read-only)",
            "/explain <file|symbol|area>",
        ),
        CommandCompletionSource::new(
            "changelog",
            Vec::<&str>::new(),
            "Draft release notes from the git history",
            "/changelog [git-range]",
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
        CommandCompletionSource::new(
            "queue",
            Vec::<&str>::new(),
            "Manage the mid-turn message queue (auto-drained after each turn)",
            "/queue [list | add <msg> | pop | clear | mode single|batch]",
        ),
        CommandCompletionSource::new(
            "tools",
            ["tool"],
            "List registered agent tools (filter, partition by read-only/writes)",
            "/tools [filter] [--read-only|--writes|--all] [--details]",
        ),
        CommandCompletionSource::new(
            "think",
            ["thinking", "reason", "reasoning"],
            "Toggle / inspect the extended-thinking (reasoning) directive",
            "/think [on|off|low|medium|high|auto|status|budget <N>]",
        ),
        CommandCompletionSource::new(
            "search",
            ["find", "grep-chat"],
            "Search the current session's message history (substring or --regex)",
            "/search [--regex] <query>",
        ),
        CommandCompletionSource::new(
            "checkpoint",
            ["snap", "snapshot"],
            "Snapshot the transcript to an in-memory checkpoint (rewindable)",
            "/checkpoint [save|list|restore <sel>|drop <sel>|clear] [label]",
        ),
        CommandCompletionSource::new(
            "checkpoints",
            Vec::<&str>::new(),
            "List in-memory transcript checkpoints",
            "/checkpoints",
        ),
        CommandCompletionSource::new(
            "rewind",
            ["undo-turn"],
            "Restore an in-memory checkpoint (empty = newest)",
            "/rewind [id|index|label]",
        ),
        CommandCompletionSource::new(
            "usage",
            ["tooltime", "telemetry"],
            "Per-tool observability dashboard (invocations, durations, bytes)",
            "/usage [show | --json | reset]",
        ),
        CommandCompletionSource::new(
            "test",
            ["tests"],
            "Auto-detect & suggest the project's test command (`--run` to execute)",
            "/test [--run]",
        ),
        CommandCompletionSource::new(
            "lint",
            ["check"],
            "Auto-detect & suggest the project's lint command (`--run` to execute)",
            "/lint [--run]",
        ),
        CommandCompletionSource::new(
            "auto-compact",
            ["autocompact", "auto_compact"],
            "Auto-fold older turns into a summary when context crosses a threshold",
            "/auto-compact [on|off|<N>|threshold <N>|reset]",
        ),
    ]
}

/// Context-window thresholds (percent of model window) at which the
/// REPL emits a one-shot advisory. The bucket index returned by
/// [`context_warn_bucket`] is monotonic so we only fire each step once
/// per "fresh transcript window".
///
/// Bucket 0 → no warning yet. Bucket 1 → 80%+. Bucket 2 → 90%+.
/// Bucket 3 → 95%+. The renderer maps each bucket to a tailored
/// message — at 80% the user gets a gentle nudge, at 95% a hard
/// recommendation to `/compact` immediately.
const CONTEXT_WARN_THRESHOLDS: &[u8] = &[80, 90, 95];

/// Map a usage percentage to the highest threshold it cleared.
/// Returns 0 when no threshold is crossed.
fn context_warn_bucket(percent: u8) -> u8 {
    let mut bucket = 0u8;
    for (idx, threshold) in CONTEXT_WARN_THRESHOLDS.iter().enumerate() {
        if percent >= *threshold {
            bucket = (idx as u8) + 1;
        }
    }
    bucket
}

/// User-facing grammar for `/auto-compact`. Kept in one place so the
/// status line, the unknown-subcommand error, and the `/help`
/// surface all share the exact same text.
fn auto_compact_help_text() -> String {
    format!(
        "Smart auto-compact — automatically folds older turns into a summary when\n\
         context usage crosses a threshold.\n\n\
         Usage:\n\
         \u{2003}/auto-compact                 show current status\n\
         \u{2003}/auto-compact on              enable at the current threshold\n\
         \u{2003}/auto-compact off             disable\n\
         \u{2003}/auto-compact <N>             enable + set threshold (e.g. `/auto-compact 90`)\n\
         \u{2003}/auto-compact threshold <N>   set threshold without changing enable state\n\
         \u{2003}/auto-compact reset           reset the lifetime fired counter\n\n\
         Threshold range: {min}..={max}% (default {default}%). Off by default.",
        min = AUTO_COMPACT_MIN_THRESHOLD,
        max = AUTO_COMPACT_MAX_THRESHOLD,
        default = AUTO_COMPACT_DEFAULT_THRESHOLD,
    )
}

/// Render the advisory message for a given bucket. Returns `None`
/// for bucket 0 so callers can early-out.
fn render_context_warn_message(bucket: u8, percent: u8, model: &str) -> Option<String> {
    let advice = match bucket {
        1 => "consider /compact when convenient",
        2 => "compact soon — `/compact` summarises the transcript without losing key facts",
        3 => "running out of headroom — `/compact` or `/new` strongly recommended NOW",
        _ => return None,
    };
    Some(format!(
        "⚠ context {percent}% of {model}'s window; {advice}.",
    ))
}

/// Which verb the user invoked: `/test` vs `/lint`. Used by
/// `execute_toolchain_command` to switch defaults between the two
/// paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolchainAction {
    Test,
    Lint,
}

impl ToolchainAction {
    fn verb(self) -> &'static str {
        match self {
            ToolchainAction::Test => "test",
            ToolchainAction::Lint => "lint",
        }
    }

    fn help_text(self) -> String {
        format!(
            "Usage: /{verb} [--run]\n\n\
             Auto-detect the current project's toolchain and suggest (or run)\n\
             the appropriate {verb} command.\n\
             \n\
             Detection markers (in order):\n  \
               Cargo.toml          → cargo\n  \
               package.json        → npm / pnpm / yarn / bun (by lockfile)\n  \
               pyproject.toml      → ruff (lint) / pytest (test)\n  \
               setup.py            → pytest\n  \
               requirements.txt    → pytest\n  \
               go.mod              → go\n  \
               Gemfile             → bundle exec rubocop / rake test\n\
             \n\
             Flags:\n  \
               --run, -r           Execute the suggested command (synchronous)\n  \
               --help, -h          Show this help",
            verb = self.verb(),
        )
    }
}

/// Spawn the toolchain command synchronously and render the captured
/// stdout/stderr plus a one-line summary. Output is buffered; for
/// long-running test suites the user sees nothing until the process
/// exits. That's a limitation we accept for the simple first cut;
/// streaming would require a separate event channel into the CLI.
fn run_toolchain_command(
    cwd: &std::path::Path,
    cmd: &crate::project_toolchain::ToolchainCommand,
    action: ToolchainAction,
) -> String {
    let start = std::time::Instant::now();
    let mut command = std::process::Command::new(&cmd.program);
    command.args(&cmd.args).current_dir(cwd);

    let output = match command.output() {
        Ok(out) => out,
        Err(err) => {
            return format!(
                "Failed to spawn `{display}`: {err}\n\
                 Is `{program}` installed and on PATH?",
                display = cmd.display(),
                program = cmd.program,
            );
        }
    };

    let elapsed = start.elapsed();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let status_label = if output.status.success() {
        format!("PASSED in {}", format_duration_ms(elapsed))
    } else {
        match output.status.code() {
            Some(code) => format!("FAILED (exit {code}) in {}", format_duration_ms(elapsed)),
            None => format!("FAILED (signal) in {}", format_duration_ms(elapsed)),
        }
    };

    let mut body = format!("$ {} ({})\n", cmd.display(), action.verb());
    if !stdout.is_empty() {
        body.push_str(&stdout);
        if !stdout.ends_with('\n') {
            body.push('\n');
        }
    }
    if !stderr.is_empty() {
        body.push_str("--- stderr ---\n");
        body.push_str(&stderr);
        if !stderr.ends_with('\n') {
            body.push('\n');
        }
    }
    body.push_str(&status_label);
    body
}

/// Render the `/usage` dashboard. Lives outside the impl block so the
/// renderer can be unit-tested against a hand-rolled
/// `ToolUsageTracker` without standing up a full agent loop.
///
/// `as_json = true` emits a compact JSON object instead of the table.
/// We keep the structure small and stable: callers piping `/usage --json`
/// into `jq` shouldn't have to chase schema churn.
fn render_tool_usage(tracker: &crate::tool_usage::ToolUsageTracker, as_json: bool) -> String {
    if as_json {
        return render_tool_usage_json(tracker);
    }

    if tracker.is_empty() {
        return String::from(
            "Tool usage: 0 invocation(s). Run a turn that exercises a tool and re-check.",
        );
    }

    let rows = tracker.sorted_by_total_duration();
    let total = tracker.total_invocations();
    let unique = tracker.len();

    // Compute an aggregate error count for the header so the user
    // immediately sees "the agent failed 7/42 tool calls".
    let total_errors: u64 = rows.iter().map(|(_, e)| e.error_count()).sum();

    let mut lines = Vec::with_capacity(rows.len() + 4);
    lines.push(format!(
        "Tool usage: {total} call(s) across {unique} tool(s), {total_errors} error(s)",
    ));
    // Column widths picked so the longest built-in tool name (e.g.
    // `ListMcpResources` = 16 chars) fits without truncation.
    lines.push(format!(
        "  {:<22}  {:>6}  {:>6}  {:>6}  {:>9}  {:>9}  {:>9}  {:>9}",
        "tool", "calls", "ok", "err", "total", "avg", "peak", "bytes",
    ));
    for (name, entry) in &rows {
        lines.push(format!(
            "  {:<22}  {:>6}  {:>6}  {:>6}  {:>9}  {:>9}  {:>9}  {:>9}",
            truncate_for_column(name, 22),
            entry.invocations(),
            entry.success_count(),
            entry.error_count(),
            format_duration_ms(entry.total_duration()),
            format_duration_ms(entry.average_duration()),
            format_duration_ms(entry.peak_duration()),
            humanize_bytes(entry.total_result_bytes() as usize),
        ));
    }
    lines.push(String::new());
    lines.push(String::from(
        "  Use `/usage --json` for machine-readable output, `/usage reset` to clear.",
    ));
    lines.join("\n")
}

fn render_tool_usage_json(tracker: &crate::tool_usage::ToolUsageTracker) -> String {
    let mut entries: Vec<serde_json::Value> = Vec::new();
    for (name, entry) in tracker.sorted_by_total_duration() {
        entries.push(serde_json::json!({
            "tool": name,
            "invocations": entry.invocations(),
            "success": entry.success_count(),
            "errors": entry.error_count(),
            "total_ms": entry.total_duration().as_millis() as u64,
            "avg_ms": entry.average_duration().as_millis() as u64,
            "peak_ms": entry.peak_duration().as_millis() as u64,
            "result_bytes": entry.total_result_bytes(),
        }));
    }
    let payload = serde_json::json!({
        "total_invocations": tracker.total_invocations(),
        "unique_tools": tracker.len(),
        "tools": entries,
    });
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| String::from("{}"))
}

/// Pretty-print a `Duration` as either `<NNNms>` or `<N.NNs>` depending
/// on magnitude. Caps at 9 chars to fit the `/usage` column width.
fn format_duration_ms(d: std::time::Duration) -> String {
    let ms = d.as_millis();
    if ms < 1_000 {
        format!("{ms}ms")
    } else if ms < 60_000 {
        format!("{:.2}s", ms as f64 / 1_000.0)
    } else {
        let minutes = ms / 60_000;
        let seconds = (ms % 60_000) / 1_000;
        format!("{minutes}m{seconds}s")
    }
}

/// Chop a tool name down to fit a fixed column. We append a `…`
/// sentinel so the user sees the name was truncated; built-in tools
/// fit comfortably in 22 chars so this only fires for very long MCP
/// names.
fn truncate_for_column(name: &str, max: usize) -> String {
    // Width-aware: `max` is a column budget, so a CJK MCP tool/server name
    // (2 cells per glyph) is cut on the right display boundary and the
    // surrounding column alignment stays intact.
    crate::width::truncate_to_width(name, max)
}

/// Render the body of `/think status`. Lives outside the impl block so
/// unit tests can drive it without an `AgentLoop`.
fn render_think_status(thinking: Option<&crate::api::ThinkingConfig>) -> String {
    match thinking {
        None => String::from(
            "Thinking: auto (no override). Subsequent turns use the backend's default — set via `--thinking` flag at startup or via provider config.\n\nToggle with `/think on`, `/think off`, `/think low|medium|high`, or `/think budget <tokens>`.",
        ),
        Some(cfg) if cfg.is_enabled() => match cfg.budget_tokens {
            Some(b) => format!(
                "Thinking: enabled (budget {b} tokens).\n\nClear with `/think auto`; switch budget with `/think low|medium|high|budget <N>`."
            ),
            None => String::from(
                "Thinking: enabled (provider default budget).\n\nClear with `/think auto`.",
            ),
        },
        Some(_) => String::from(
            "Thinking: explicitly disabled.\n\nRe-enable with `/think on`, or clear the override with `/think auto`.",
        ),
    }
}

/// Sub-mode of `/tools` controlling which subset of the registry to render.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolListMode {
    All,
    ReadOnly,
    Writes,
}

/// Trim a tool description down to a single-line summary for the table
/// view. Splits on the first newline, then truncates to `max_chars`
/// graphemes-ish (chars in practice) with an ellipsis. Keeps `/tools`
/// readable on standard 80-column terminals.
fn first_line_summary(description: &str, max_chars: usize) -> String {
    let first = description.lines().next().unwrap_or("").trim();
    if first.chars().count() <= max_chars {
        first.to_owned()
    } else {
        let truncated: String = first.chars().take(max_chars.saturating_sub(1)).collect();
        format!("{truncated}…")
    }
}

/// Render a single queued message as a short, single-line preview. Used
/// Formatted help text for `/checkpoint --help`. Kept as a free fn (not
/// a const) so we can produce it lazily and edit the grammar without
/// chasing a static literal.
fn checkpoint_help_text() -> String {
    String::from(
        "Usage: /checkpoint [save] [label]\n\
         \n\
         Snapshot the current conversation into an in-memory checkpoint.\n\
         Useful for rolling back multi-turn mistakes without restarting.\n\
         \n\
         Subcommands:\n\
           /checkpoint                Save (no label) — same as `save`\n\
           /checkpoint [label]        Save with a human-readable label\n\
           /checkpoint list           List checkpoints (alias: /checkpoints)\n\
           /checkpoint restore <sel>  Restore to a checkpoint (alias: /rewind)\n\
           /checkpoint drop <sel>     Remove a specific checkpoint\n\
           /checkpoint clear          Remove every checkpoint\n\
         \n\
         Selectors (resolve in order: index → exact id → label → id prefix):\n\
           (empty)         newest checkpoint\n\
           1, 2, …         1-based index from `/checkpoints`\n\
           <8-char id>     exact id (e.g. a1b2c3d4)\n\
           <id prefix>     at least 2 chars; ambiguous prefixes are reported\n\
           <label>         exact label match (case-sensitive)\n\
         \n\
         The store is capped at 20 entries (FIFO) and lives in memory only.\n\
         For cross-process resume, use `/sessions` + `/resume`.",
    )
}

/// by `/queue list`, `/queue add`, and `/queue pop` so the user can see
/// what's about to fire without flooding the terminal with multi-line
/// blobs. We cap at 80 chars in the message body and collapse interior
/// newlines to a sentinel so the preview stays on one row.
fn format_queue_preview(message: &str) -> String {
    const MAX_CHARS: usize = 80;
    let collapsed: String = message
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    if collapsed.chars().count() <= MAX_CHARS {
        collapsed
    } else {
        let truncated: String = collapsed.chars().take(MAX_CHARS).collect();
        format!("{truncated}…")
    }
}

/// Render `/queue list` output. Always shows the mode + depth, even when
/// empty, so the user can verify their `/queue mode` toggle stuck.
fn render_queue_list(
    items: &[String],
    mode: crate::message_queue::QueueMode,
    len: usize,
) -> String {
    let mut lines = Vec::with_capacity(items.len() + 4);
    lines.push(format!("Message queue: {len} pending  (mode: {mode})"));
    if items.is_empty() {
        lines.push(String::from(
            "  (empty — type during a turn or use `/queue add <msg>` to populate)",
        ));
    } else {
        for (i, message) in items.iter().enumerate() {
            lines.push(format!("  {:>2}. {}", i + 1, format_queue_preview(message)));
        }
    }
    lines.push(String::new());
    lines.push(String::from(
        "Subcommands: list · add <msg> · pop · clear · mode single|batch",
    ));
    lines.join("\n")
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

/// Directories searched for user-authored slash commands (`<name>.md`), in
/// precedence order: project-scoped config, global config, in-repo `.deeptide`,
/// then home `.deeptide`. Mirrors the agent-discovery layout so a project can
/// ship commands alongside its agents.
fn command_dirs(cwd: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut dirs = vec![
        MemorySystem::tide_config_dir()
            .join("projects")
            .join(MemorySystem::project_slug(cwd))
            .join("commands"),
        MemorySystem::tide_config_dir().join("commands"),
        cwd.join(".deeptide").join("commands"),
    ];
    if let Some(home) = home_dir() {
        dirs.push(home.join(".deeptide").join("commands"));
    }
    dirs
}

/// Names of all discoverable custom slash commands for `cwd`, deduped + sorted.
/// Used to register them for tab-completion and `/help`.
fn discover_command_names(cwd: &std::path::Path) -> Vec<String> {
    let mut names = Vec::new();
    for dir in command_dirs(cwd) {
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

/// Resolve a custom-command name to its `.md` file, honouring `command_dirs`
/// precedence (first match wins). Command names are restricted to a safe
/// charset so a `/..%2f`-style name can't escape the commands directory.
fn find_command_file(cwd: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return None;
    }
    for dir in command_dirs(cwd) {
        let candidate = dir.join(format!("{name}.md"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
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

/// Build the compact live `cache` status-bar segment from cache health, or
/// `None` when there's nothing worth showing (no cache telemetry reported yet).
///
/// Value shape: `cache <rate>%` using the *recent*-turns hit rate (the last few
/// turns reflect the current prefix's warmth far better than a session-lifetime
/// average — a session that switched models mid-way shouldn't show a stale
/// blended number). Falls back to the lifetime rate if recent is unavailable.
///
/// Severity drives the color so a glance tells the story:
///   * `Info` (cyan)    — strong cache (≥ 80%): the prefix is warm, turns cheap.
///   * `Neutral` (dim)  — warming (60–79%): reads happening, climbing.
///   * `Warning` (amber)— cold (< 60%) with reads present: prefix not sticking.
///
/// Returns `None` for the "warming, turn 1, no reads yet" and "provider didn't
/// report" cases so the bar stays silent until caching is actually observable.
fn cache_status_segment(cache: &crate::CacheHealth) -> Option<StatusSegment> {
    let rate = cache.recent_hit_rate_percent.or(cache.hit_rate_percent)?;
    let severity = if rate >= 80 {
        crate::Severity::Info
    } else if rate >= 60 {
        crate::Severity::Neutral
    } else {
        crate::Severity::Warning
    };
    Some(StatusSegment::new("cache", format!("{rate}%")).with_severity(severity))
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

/// Compact the absolute working-directory path for the status bar.
///
/// We substitute `$HOME` with `~` and, when the remaining string is still
/// wide enough to crowd the bar on an 80-column terminal, collapse it to the
/// basename. This keeps the most identifying piece of information visible
/// without hijacking the bar with deep nested paths.
pub(crate) fn format_cwd_for_status(cwd: &std::path::Path) -> String {
    const MAX_DISPLAY_CHARS: usize = 28;

    let raw = cwd.display().to_string();
    let with_tilde = match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => raw
            .strip_prefix(&home)
            .map(|rest| {
                if rest.is_empty() {
                    String::from("~")
                } else if rest.starts_with('/') {
                    format!("~{rest}")
                } else {
                    raw.clone()
                }
            })
            .unwrap_or(raw),
        _ => raw,
    };

    if with_tilde.chars().count() <= MAX_DISPLAY_CHARS {
        return with_tilde;
    }

    cwd.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .unwrap_or(with_tilde)
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
    // Display-facing one-line previews (retry prompt, etc.): budget by terminal
    // display width so a CJK prompt is cut on the right column boundary rather
    // than at a char count that over-counts wide glyphs.
    crate::width::truncate_to_width(value, max_chars)
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

#[cfg(test)]
mod bulk_import_corpus_tests {
    use super::{BulkImportCorpus, build_bulk_import_corpus};

    fn width(s: &str) -> usize {
        crate::width::display_width(s)
    }

    #[test]
    fn single_oversized_session_is_truncated_to_the_ceiling() {
        // One session whose body dwarfs the budget must still be capped — the
        // motivating bug was that the first session was included whole.
        let budget = 1_000;
        let header = String::from("\n\n===== claude session abc =====\n");
        let body = "x".repeat(50_000);
        let BulkImportCorpus {
            combined,
            included,
            skipped,
        } = build_bulk_import_corpus(&[(header, body)], budget, 200);
        assert_eq!(included, 1);
        assert_eq!(skipped, 0);
        assert!(
            width(&combined) <= budget,
            "combined width {} must not exceed budget {budget}",
            width(&combined)
        );
    }

    #[test]
    fn later_sessions_are_truncated_then_skipped_within_budget() {
        // First session eats most of the budget; the rest get a shrinking slice
        // and are eventually skipped — but the total never exceeds the budget.
        let budget = 2_000;
        let sessions: Vec<(String, String)> = (0..5)
            .map(|i| {
                (
                    format!("\n\n===== claude session s{i} =====\n"),
                    "y".repeat(1_500),
                )
            })
            .collect();
        let BulkImportCorpus {
            combined,
            included,
            skipped,
        } = build_bulk_import_corpus(&sessions, budget, 200);
        assert_eq!(included + skipped, 5, "every session is accounted for");
        assert!(included >= 1, "the newest session is always kept");
        assert!(
            skipped >= 1,
            "some later sessions must be skipped for length"
        );
        assert!(
            width(&combined) <= budget,
            "combined width {} must not exceed budget {budget}",
            width(&combined)
        );
    }

    #[test]
    fn everything_fits_when_under_budget() {
        let sessions = vec![
            (String::from("==h1==\n"), String::from("short one")),
            (String::from("==h2==\n"), String::from("short two")),
        ];
        let BulkImportCorpus {
            included, skipped, ..
        } = build_bulk_import_corpus(&sessions, 60_000, 200);
        assert_eq!(included, 2);
        assert_eq!(skipped, 0);
    }
}

#[cfg(test)]
mod cwd_format_tests {
    use super::format_cwd_for_status;
    use std::path::PathBuf;

    fn with_home<T>(home: &str, body: impl FnOnce() -> T) -> T {
        use std::sync::{Mutex, OnceLock};
        // `HOME` is process-global and `cargo test` runs these cases in
        // parallel, so serialize them: without the lock one test's save/restore
        // interleaves with another's body and it reads the wrong `HOME`.
        static HOME_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        let _guard = HOME_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = std::env::var("HOME").ok();
        // SAFETY: process-wide env mutation; restored below. Acceptable in a
        // single-threaded unit test that does not run alongside other env
        // probes (no other test in this module touches HOME).
        unsafe {
            std::env::set_var("HOME", home);
        }
        let out = body();
        match previous {
            Some(value) => unsafe { std::env::set_var("HOME", value) },
            None => unsafe { std::env::remove_var("HOME") },
        }
        out
    }

    #[test]
    fn collapses_home_prefix_to_tilde() {
        with_home("/Users/ryan", || {
            let formatted = format_cwd_for_status(&PathBuf::from("/Users/ryan/projects/foo"));
            assert_eq!(formatted, "~/projects/foo");
        });
    }

    #[test]
    fn long_path_falls_back_to_basename() {
        with_home("/Users/ryan", || {
            let formatted = format_cwd_for_status(&PathBuf::from(
                "/Users/ryan/a/very/deeply/nested/path/that/exceeds/the/limit/deeptide-npm",
            ));
            assert_eq!(formatted, "deeptide-npm");
        });
    }

    #[test]
    fn unrelated_root_is_kept_verbatim_when_short() {
        with_home("/Users/ryan", || {
            let formatted = format_cwd_for_status(&PathBuf::from("/opt/work"));
            assert_eq!(formatted, "/opt/work");
        });
    }

    #[test]
    fn exact_home_collapses_to_tilde() {
        with_home("/Users/ryan", || {
            let formatted = format_cwd_for_status(&PathBuf::from("/Users/ryan"));
            assert_eq!(formatted, "~");
        });
    }
}

#[cfg(test)]
mod context_warn_tests {
    #![allow(clippy::unwrap_used)]

    use super::{CONTEXT_WARN_THRESHOLDS, context_warn_bucket, render_context_warn_message};

    #[test]
    fn bucket_is_zero_below_any_threshold() {
        for pct in 0..CONTEXT_WARN_THRESHOLDS[0] {
            assert_eq!(context_warn_bucket(pct), 0, "pct={pct}");
        }
    }

    #[test]
    fn bucket_one_at_80_percent_through_89_percent() {
        for pct in CONTEXT_WARN_THRESHOLDS[0]..CONTEXT_WARN_THRESHOLDS[1] {
            assert_eq!(context_warn_bucket(pct), 1, "pct={pct}");
        }
    }

    #[test]
    fn bucket_two_at_90_through_94_percent() {
        for pct in CONTEXT_WARN_THRESHOLDS[1]..CONTEXT_WARN_THRESHOLDS[2] {
            assert_eq!(context_warn_bucket(pct), 2, "pct={pct}");
        }
    }

    #[test]
    fn bucket_three_at_95_percent_and_above() {
        for pct in CONTEXT_WARN_THRESHOLDS[2]..=100 {
            assert_eq!(context_warn_bucket(pct), 3, "pct={pct}");
        }
    }

    #[test]
    fn render_bucket_zero_returns_none() {
        assert!(render_context_warn_message(0, 50, "any-model").is_none());
    }

    #[test]
    fn render_includes_model_and_percent() {
        let msg = render_context_warn_message(1, 82, "my-model").expect("bucket 1 has message");
        assert!(msg.contains("82%"));
        assert!(msg.contains("my-model"));
        assert!(msg.to_ascii_lowercase().contains("compact"));
    }

    #[test]
    fn render_message_escalates_with_bucket() {
        let b1 = render_context_warn_message(1, 80, "m").unwrap();
        let b2 = render_context_warn_message(2, 90, "m").unwrap();
        let b3 = render_context_warn_message(3, 95, "m").unwrap();
        assert!(
            b1.len() < b2.len() || b1 != b2,
            "b1 vs b2 should differ: {b1} | {b2}",
        );
        assert!(
            b3.to_ascii_uppercase().contains("NOW"),
            "highest bucket should emphasise urgency: {b3}",
        );
    }
}

#[cfg(test)]
mod auto_compact_config_tests {
    #![allow(clippy::unwrap_used)]
    use super::{
        AUTO_COMPACT_DEFAULT_THRESHOLD, AUTO_COMPACT_MAX_THRESHOLD, AUTO_COMPACT_MIN_THRESHOLD,
        AutoCompactConfig, auto_compact_help_text,
    };

    #[test]
    fn default_is_disabled_at_the_default_threshold() {
        let cfg = AutoCompactConfig::default();
        assert!(!cfg.enabled, "auto-compact must be opt-in (off by default)");
        assert_eq!(cfg.threshold_percent, AUTO_COMPACT_DEFAULT_THRESHOLD);
        assert_eq!(cfg.fired_count, 0);
    }

    #[test]
    fn set_threshold_clamps_below_minimum() {
        let mut cfg = AutoCompactConfig::default();
        let final_value = cfg.set_threshold(10);
        assert_eq!(final_value, AUTO_COMPACT_MIN_THRESHOLD);
        assert_eq!(cfg.threshold_percent, AUTO_COMPACT_MIN_THRESHOLD);
    }

    #[test]
    fn set_threshold_clamps_above_maximum() {
        let mut cfg = AutoCompactConfig::default();
        let final_value = cfg.set_threshold(250);
        assert_eq!(final_value, AUTO_COMPACT_MAX_THRESHOLD);
        assert_eq!(cfg.threshold_percent, AUTO_COMPACT_MAX_THRESHOLD);
    }

    #[test]
    fn set_threshold_accepts_in_range_values_unchanged() {
        let mut cfg = AutoCompactConfig::default();
        for value in [
            AUTO_COMPACT_MIN_THRESHOLD,
            AUTO_COMPACT_MIN_THRESHOLD + 1,
            85,
            AUTO_COMPACT_DEFAULT_THRESHOLD,
            AUTO_COMPACT_MAX_THRESHOLD - 1,
            AUTO_COMPACT_MAX_THRESHOLD,
        ] {
            let stored = cfg.set_threshold(value);
            assert_eq!(stored, value, "in-range value {value} must round-trip");
        }
    }

    #[test]
    fn help_text_documents_the_full_grammar_and_bounds() {
        let text = auto_compact_help_text();
        // Each grammar form must be mentioned by name; users land
        // here when they typo a subcommand so missing any is a
        // discoverability bug.
        for needle in [
            "/auto-compact",
            "on",
            "off",
            "threshold",
            "reset",
            "default",
        ] {
            assert!(
                text.contains(needle),
                "help text missing `{needle}`: {text}"
            );
        }
        // Threshold bounds must be surfaced literally so the user
        // doesn't have to guess what range is valid.
        assert!(
            text.contains(&format!("{AUTO_COMPACT_MIN_THRESHOLD}")),
            "help text must mention the min threshold ({AUTO_COMPACT_MIN_THRESHOLD}): {text}"
        );
        assert!(
            text.contains(&format!("{AUTO_COMPACT_MAX_THRESHOLD}")),
            "help text must mention the max threshold ({AUTO_COMPACT_MAX_THRESHOLD}): {text}"
        );
    }
}

#[cfg(test)]
mod extract_tool_subject_tests {
    #![allow(clippy::unwrap_used)]
    use super::extract_tool_subject;
    use serde_json::json;

    #[test]
    fn read_returns_file_path() {
        let subj = extract_tool_subject("Read", &json!({"file_path": "src/main.rs"}));
        assert_eq!(subj.as_deref(), Some("src/main.rs"));
    }

    #[test]
    fn write_returns_file_path_case_insensitive() {
        let subj = extract_tool_subject("write", &json!({"file_path": "out/log.txt"}));
        assert_eq!(subj.as_deref(), Some("out/log.txt"));
    }

    #[test]
    fn edit_and_appendfile_and_notebookedit_return_file_path() {
        for name in ["Edit", "AppendFile", "NotebookEdit"] {
            let subj = extract_tool_subject(name, &json!({"file_path": "notes.md"}));
            assert_eq!(subj.as_deref(), Some("notes.md"), "tool: {name}");
        }
    }

    #[test]
    fn readfiles_summarises_count() {
        let subj = extract_tool_subject(
            "ReadFiles",
            &json!({"file_paths": ["a.rs", "b.rs", "c.rs"]}),
        );
        assert_eq!(subj.as_deref(), Some("3 file(s)"));
    }

    #[test]
    fn bash_collapses_newlines_to_a_single_line() {
        let subj = extract_tool_subject(
            "Bash",
            &json!({"command": "cargo test\n  --workspace\n  -- --nocapture"}),
        );
        // Only the first command line is shown — multi-line bodies
        // belong in the diff preview, not the post-completion row.
        assert_eq!(subj.as_deref(), Some("cargo test"));
    }

    #[test]
    fn bash_truncates_long_commands_with_ellipsis() {
        let long_cmd = "a".repeat(200);
        let subj = extract_tool_subject("Bash", &json!({"command": long_cmd}));
        let s = subj.expect("subject");
        assert!(s.ends_with('…'), "expected ellipsis suffix: {s}");
        assert!(
            s.chars().count() <= 81,
            "expected ≤80 chars + ellipsis: len={}",
            s.chars().count()
        );
    }

    #[test]
    fn webfetch_returns_url() {
        let subj = extract_tool_subject("WebFetch", &json!({"url": "https://example.com/a"}));
        assert_eq!(subj.as_deref(), Some("https://example.com/a"));
    }

    #[test]
    fn websearch_returns_query() {
        let subj = extract_tool_subject("WebSearch", &json!({"query": "rustyline highlight_char"}));
        assert_eq!(subj.as_deref(), Some("rustyline highlight_char"));
    }

    #[test]
    fn grep_returns_pattern() {
        let subj = extract_tool_subject("Grep", &json!({"pattern": "TODO|FIXME", "path": "src/"}));
        assert_eq!(subj.as_deref(), Some("TODO|FIXME"));
    }

    #[test]
    fn todowrite_returns_first_todo_content() {
        let subj = extract_tool_subject(
            "TodoWrite",
            &json!({"todos": [
                {"content": "Diff preview", "status": "done"},
                {"content": "Slash palette", "status": "in_progress"},
            ]}),
        );
        assert_eq!(subj.as_deref(), Some("Diff preview"));
    }

    #[test]
    fn agent_returns_subagent_type() {
        let subj = extract_tool_subject(
            "Agent",
            &json!({"subagent_type": "generalPurpose", "prompt": "x"}),
        );
        assert_eq!(subj.as_deref(), Some("generalPurpose"));
    }

    #[test]
    fn sleep_formats_seconds() {
        let subj = extract_tool_subject("Sleep", &json!({"seconds": 5.0}));
        assert_eq!(subj.as_deref(), Some("5s"));
    }

    #[test]
    fn unknown_tool_returns_none() {
        let subj = extract_tool_subject("DefinitelyNotARealTool", &json!({"foo": "bar"}));
        assert!(subj.is_none());
    }

    #[test]
    fn missing_input_field_returns_none() {
        let subj = extract_tool_subject("Read", &json!({}));
        assert!(subj.is_none());
    }

    #[test]
    fn whitespace_only_subject_returns_none() {
        let subj = extract_tool_subject("Read", &json!({"file_path": "   "}));
        assert!(subj.is_none(), "expected None for whitespace-only path");
    }
}

#[cfg(test)]
mod slash_command_classifier_tests {
    use super::slash_command_invokes_agent;

    #[test]
    fn local_only_commands_do_not_invoke_agent() {
        // These all run synchronously inside execute_command without
        // ever touching agent_loop.run — they must NOT trigger the
        // spinner+queue-editor spawn in the CLI.
        for cmd in [
            "/exit",
            "/quit",
            "/q",
            "/clear",
            "/cls",
            "/new",
            "/help",
            "/h",
            "/?",
            "/status",
            "/cost",
            "/copy",
            "/yank",
            "/diff",
            "/branch",
            "/tps",
            "/queue",
            "/queue add hello",
            "/context",
            "/ctx",
            "/model gpt-5",
            "/m flash",
            "/provider",
            "/provider use deepseek",
            "/permission",
            "/perm",
            "/permissions",
            "/auto-compact",
            "/auto-compact on",
            "/auto-compact 90",
            "/compact",
            "/compress",
            "/export",
            "/export /tmp/log.jsonl",
            "/fast",
            "/faster",
            "/add-dir",
            "/add-dir src/",
            "/debug",
            "/dbg",
            "/keys",
            "/keybindings",
            "/sessions",
            "/session",
            "/resume",
            "/load",
            "/restore",
            "/open",
            "/paste",
            "/p",
            "/doctor",
            "/config",
            "/hooks",
            "/init",
            "/update",
            "/upgrade",
            "/read foo.rs",
            "/write foo.rs hi",
            "/memory",
            "/mem",
            "/remember",
            "/skills",
            "/skill",
            "/cron",
            "/cache",
            "/kvcache",
            "/manifest",
        ] {
            assert!(
                !slash_command_invokes_agent(cmd),
                "expected `{cmd}` to be local-only, but classifier said agent-invoking"
            );
        }
    }

    #[test]
    fn agent_invoking_commands_are_recognised() {
        // These branches all call agent_loop.run internally — the
        // CLI MUST spin up the spinner + queue editor when the user
        // submits one of these.
        for cmd in [
            "/retry",
            "/r",
            "/again",
            "/commit",
            "/commit -m feat",
            "/review 42",
            "/simplify src/",
            "/dream",
            "/dream now",
            "/goal",
            "/goal ship the cli",
            "/objective ship the cli",
            "/reminder",
            "/anchor",
            "/reorient",
            "/vim",
            "/edit",
            "/e",
            "/compose",
        ] {
            assert!(
                slash_command_invokes_agent(cmd),
                "expected `{cmd}` to invoke the agent, but classifier returned false"
            );
        }
    }

    #[test]
    fn case_insensitive_name_match() {
        // The dispatch in execute_command lowercases the command
        // name; the classifier must do the same so the CLI matches
        // the actual handler behaviour.
        assert!(slash_command_invokes_agent("/RETRY"));
        assert!(slash_command_invokes_agent("/Retry"));
        assert!(slash_command_invokes_agent("/CoMMit args here"));
        assert!(!slash_command_invokes_agent("/EXIT"));
        assert!(!slash_command_invokes_agent("/Clear"));
    }

    #[test]
    fn non_slash_input_returns_false() {
        // Non-slash chat input is always agent-invoking, but callers
        // are expected to handle that BEFORE consulting this
        // classifier. We return false to keep the contract simple:
        // "the slash command this string starts with is agent-invoking?"
        assert!(!slash_command_invokes_agent("hello world"));
        assert!(!slash_command_invokes_agent("look at @foo.rs"));
        assert!(!slash_command_invokes_agent(""));
        assert!(!slash_command_invokes_agent("   "));
    }

    #[test]
    fn unknown_slash_command_returns_false() {
        // An unknown slash command falls through to "render unknown
        // command" — strictly local. Don't trip the spinner.
        assert!(!slash_command_invokes_agent("/totally-fake-command"));
        assert!(!slash_command_invokes_agent("/foo bar baz"));
    }

    #[test]
    fn leading_whitespace_is_tolerated() {
        // The CLI trims input before submission, but the classifier
        // should be robust to a user typing a space before `/`.
        assert!(slash_command_invokes_agent("  /retry"));
        assert!(!slash_command_invokes_agent("  /exit"));
    }
}

#[cfg(test)]
mod cache_segment_tests {
    use super::cache_status_segment;
    use crate::{CacheHealth, Severity};

    fn health(
        recent: Option<usize>,
        total: Option<usize>,
        reads: usize,
        turns: usize,
    ) -> CacheHealth {
        CacheHealth {
            hit_rate_percent: total,
            recent_hit_rate_percent: recent,
            total_create_tokens: 0,
            total_read_tokens: reads,
            turn_count: turns,
        }
    }

    #[test]
    fn no_telemetry_yields_no_segment() {
        // Turn 1, nothing reported yet → the bar stays silent.
        assert!(cache_status_segment(&health(None, None, 0, 1)).is_none());
    }

    #[test]
    fn strong_cache_is_info_severity() {
        let seg =
            cache_status_segment(&health(Some(87), Some(80), 5000, 4)).expect("segment present");
        assert_eq!(seg.label, "cache");
        assert_eq!(seg.value, "87%");
        assert_eq!(seg.severity, Severity::Info);
    }

    #[test]
    fn warming_cache_is_neutral_severity() {
        let seg =
            cache_status_segment(&health(Some(70), Some(65), 2000, 3)).expect("segment present");
        assert_eq!(seg.value, "70%");
        assert_eq!(seg.severity, Severity::Neutral);
    }

    #[test]
    fn cold_cache_is_warning_severity() {
        let seg =
            cache_status_segment(&health(Some(20), Some(25), 500, 5)).expect("segment present");
        assert_eq!(seg.value, "20%");
        assert_eq!(seg.severity, Severity::Warning);
    }

    #[test]
    fn recent_rate_is_preferred_over_lifetime() {
        // recent diverges from lifetime; the segment must reflect recent
        // (the current prefix warmth), not the blended average.
        let seg =
            cache_status_segment(&health(Some(90), Some(40), 9000, 6)).expect("segment present");
        assert_eq!(seg.value, "90%");
        assert_eq!(seg.severity, Severity::Info);
    }

    #[test]
    fn falls_back_to_lifetime_when_recent_absent() {
        let seg = cache_status_segment(&health(None, Some(82), 4000, 5)).expect("segment present");
        assert_eq!(seg.value, "82%");
        assert_eq!(seg.severity, Severity::Info);
    }
}
