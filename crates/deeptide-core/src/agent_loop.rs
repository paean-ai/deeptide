use crate::{
    CompressionReport, ContextWindowConfig, ContextWindowManager, CostTracker, PermissionDecision,
    PermissionManager, PermissionMode, PermissionRules, ToolBatchFailureClassifier, ToolBatchItem,
    ToolBatchLabeler, ToolContext, ToolRegistry, TurnUsage,
};
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Assistant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversationMessage {
    pub role: MessageRole,
    /// Free-form text payload. Empty when the message only carries
    /// structured `tool_calls` / `tool_results` blocks (Anthropic still
    /// requires a non-empty `content` array — the wire layer will skip the
    /// text block when this is empty).
    pub content: String,
    /// Tool invocations produced by the assistant in this turn. Preserved so
    /// the next API request can re-send the corresponding `tool_use` blocks
    /// alongside the assistant's text — Anthropic rejects conversations where
    /// a `tool_use` is not immediately followed by matching `tool_result`s.
    pub tool_calls: Vec<ToolCall>,
    /// Tool results synthesised by the agent loop for the next user turn,
    /// carrying the matching `tool_use_id` Anthropic expects.
    pub tool_results: Vec<ToolResultBlock>,
}

impl ConversationMessage {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: Vec::new(),
            tool_results: Vec::new(),
        }
    }

    /// Build an assistant turn that emitted both free-form text *and* one or
    /// more `tool_use` blocks. The text may be empty if the assistant only
    /// emitted tool calls.
    pub fn assistant_with_tool_calls(
        content: impl Into<String>,
        tool_calls: Vec<ToolCall>,
    ) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls,
            tool_results: Vec::new(),
        }
    }

    /// Build a user turn whose sole purpose is to deliver `tool_result` blocks
    /// back to the model. The text body is intentionally empty so the wire
    /// layer emits a pure tool_result content array.
    pub fn tool_results(blocks: Vec<ToolResultBlock>) -> Self {
        Self {
            role: MessageRole::User,
            content: String::new(),
            tool_calls: Vec::new(),
            tool_results: blocks,
        }
    }
}

/// Structured tool-result payload mirroring Anthropic's `tool_result` content
/// block. Stored on the user-role `ConversationMessage` synthesised after each
/// batch of tool executions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResultBlock {
    pub tool_use_id: String,
    pub content: String,
    pub is_error: bool,
}

impl ToolResultBlock {
    pub fn new(tool_use_id: impl Into<String>, content: impl Into<String>, is_error: bool) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            content: content.into(),
            is_error,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRequest {
    pub messages: Vec<ConversationMessage>,
    pub model: String,
    pub step: usize,
    pub max_turns: usize,
    /// Optional system prompt sent on every request.  Empty string is treated
    /// as absent so callers can pass an empty string without sending a blank
    /// system field to the API.
    pub system: Option<String>,
    /// When `Some`, only these tools should be advertised to the model (the
    /// backend filters its tool schemas to this set). `None` advertises every
    /// tool. Set for restricted sub-agents so they are only offered the tools
    /// they are allowed to call.
    pub allowed_tools: Option<Vec<String>>,
    /// Per-request override of the extended-thinking directive. `None`
    /// (the default) falls back to whatever the backend was configured
    /// with at construction time; `Some(_)` takes precedence so the REPL
    /// can toggle thinking on/off mid-session via `/think` without
    /// rebuilding the backend.
    pub thinking: Option<crate::api::ThinkingConfig>,
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
    /// The transcript was auto-compacted mid-run because it exceeded the
    /// context-window threshold.
    Compaction(CompressionReport),
    Terminal(AgentTerminalEvent),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentTerminalEvent {
    Complete,
    /// The agent hit the per-prompt turn cap before the model produced a
    /// terminal assistant message. `cap` carries the configured ceiling
    /// so the UI layer can surface a helpful "raise via --max-turns N"
    /// hint without re-reading the loop's state.
    MaxTurnsReached {
        cap: usize,
    },
    ModelError(String),
    /// The transcript exceeded the model's hard context limit even after
    /// compaction, so the run was stopped rather than issuing a doomed request.
    Blocked,
    /// The user cancelled the turn mid-flight (Ctrl-C). The loop stops at the
    /// next safe boundary — between steps, after a cancelled model stream, or
    /// after the current tool batch (remaining tools in the batch get
    /// synthetic "cancelled" results so every `tool_use` keeps a matching
    /// `tool_result` and the session stays resumable).
    Interrupted,
}

pub trait AgentBackend: Send {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String>;
}

pub type SubAgentBackendFactory = Arc<dyn Fn(&str) -> Box<dyn AgentBackend> + Send + Sync>;

/// Fallback context-window budget when the model is unknown. Matches
/// [`ContextWindowConfig::default`] and Swift's `forModel` default.
const DEFAULT_CONTEXT_WINDOW: usize = 128_000;

/// Compact once the transcript exceeds this many messages, even when it is
/// still under the token threshold. Matches Swift's
/// `CompactionManager.maxMessagesBeforeCompact`.
const MAX_MESSAGES_BEFORE_COMPACT: usize = 200;

/// Outcome of an interactive permission prompt. The REPL surfaces a
/// `[y]es / [n]o / [t]his-tool / [a]llow-and-bypass` prompt and returns
/// one of these so the agent loop can both decide on the current tool
/// call AND adjust session-wide permissions (mode or rules) in one
/// round-trip.
#[derive(Debug, Clone)]
pub enum AskOutcome {
    /// Approve this single tool call. Session permission mode is unchanged.
    Allow,
    /// Approve this call AND install a session-scoped allow rule for every
    /// subsequent invocation of the **same tool name** (e.g. all `Write`
    /// calls for the rest of this REPL session). The rule is not persisted
    /// to disk — closing the session removes it. Use this when the user
    /// wants to stop being prompted for a specific tool without flipping
    /// the entire session into Bypass.
    AllowAllSession {
        /// The exact tool name (e.g. `"Write"`, `"Bash"`) to whitelist.
        tool_name: String,
    },
    /// Approve this single tool call AND switch the session to the given
    /// mode so subsequent calls of comparable risk don't re-prompt.
    AllowAndSetMode(PermissionMode),
    /// Reject this call. The tool result is replaced with an error string
    /// surfaced back to the model so it can recover or apologise.
    Deny { reason: String },
}

/// Callback invoked when a tool call needs interactive approval (the
/// permission system returned [`PermissionDecision::Ask`]). The callback
/// runs synchronously in the agent loop thread; the REPL's wiring stops the
/// spinner, prints the prompt, reads stdin, and returns the decision.
pub type PermissionAskCallback = Arc<dyn Fn(&ToolCall) -> AskOutcome + Send + Sync>;

/// Phases of a single tool invocation, threaded to the UI via
/// [`ToolProgressCallback`] so the CLI's spinner / pinned input
/// row can show *what* the agent is running right now (not just a
/// generic "Crunching…").
///
/// Two events bracket every `execute_tool_call`:
///
/// * `Started`  — fired BEFORE the tool runs. `index` is the
///   0-based position in the current turn's tool batch (1-indexed
///   for display); `total` is how many tools the model asked for
///   in this turn.
/// * `Finished` — fired AFTER the tool returns. `is_error` carries
///   whether the result was an error so the UI can flash a red
///   marker, but the body is not carried here (the UI gets the
///   full result via `AgentLoopEvent::ToolResult`).
///
/// Kept as a struct enum (rather than two free types) so future
/// phases — e.g. `Cancelled`, `Permission(Pending)` — can be added
/// without bumping the callback signature.
#[derive(Debug, Clone)]
pub enum ToolProgressEvent {
    Started {
        index: usize,
        total: usize,
        name: String,
        /// Short, pre-rendered preview of the tool's input
        /// (e.g. `"Bash(ls -la)"`, `"Read(/etc/hosts)"`). Built
        /// inside `agent_loop` from the same labelling helpers
        /// the batch summary uses, so the spinner string and the
        /// post-turn summary stay visually consistent. May be the
        /// empty string for tools with no compact preview.
        preview: String,
    },
    Finished {
        index: usize,
        total: usize,
        name: String,
        is_error: bool,
    },
}

/// Callback invoked synchronously around every tool execution.
/// `Arc<dyn Fn…>` so the CLI can clone it once and have the
/// closure observe `AtomicBool` / `Mutex<String>` state shared
/// with the spinner thread. Synchronous on purpose — the closure
/// must return quickly because the agent loop is blocked on it.
pub type ToolProgressCallback = Arc<dyn Fn(&ToolProgressEvent) + Send + Sync>;

/// Callback fired the moment each structured [`AgentLoopEvent`] is produced
/// during [`AgentLoop::run`] — in the SAME order they appear in the returned
/// `Vec`. Without it (the default) `run()` behaves exactly as before, returning
/// the full event list only when the turn ends; with it, a host (the GUI, or a
/// live headless stream) sees tool batches/results/compaction/terminal events
/// AS THEY HAPPEN rather than buffered-then-replayed. Synchronous and must
/// return quickly — the agent loop is blocked on it, like the other callbacks.
pub type AgentEventCallback = Arc<dyn Fn(&AgentLoopEvent) + Send + Sync>;

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
    system_prompt: Option<String>,
    context_window: ContextWindowManager,
    hooks: crate::hooks::HookEngine,
    /// When `Some`, only these tools may be called (allowlist). `None` allows
    /// every tool. Mirrors a sub-agent definition's `allowedTools`.
    allowed_tools: Option<Vec<String>>,
    /// Tools that may never be called, even if present in `allowed_tools`.
    /// Mirrors a sub-agent definition's `disallowedTools`.
    disallowed_tools: Vec<String>,
    /// Interactive approval callback. When unset, an `Ask` decision still
    /// returns an error (preserving the non-interactive contract used by
    /// `--print` mode and library embeds); when set, the REPL gets a chance
    /// to elicit a decision from the user.
    ask_callback: Option<PermissionAskCallback>,
    /// Optional callback fired around every tool execution
    /// ([`ToolProgressEvent::Started`] / [`ToolProgressEvent::Finished`]).
    /// `None` means "silent execution", matching the prior behaviour;
    /// the CLI plugs in a callback that updates the spinner / pinned
    /// input-row hint with the current tool name.
    tool_progress_callback: Option<ToolProgressCallback>,
    /// Optional live structured-event sink (see [`AgentEventCallback`]). When
    /// set, each `AgentLoopEvent` is delivered as it is produced, not just in
    /// the `Vec` returned by `run()`.
    event_callback: Option<AgentEventCallback>,
    /// Runtime override of the backend's extended-thinking directive.
    /// Threaded into every outbound `AgentRequest` so the REPL can flip
    /// thinking on/off (and tune its budget) without rebuilding the
    /// backend or restarting the session. `None` defers to whatever the
    /// backend was constructed with.
    thinking_override: Option<crate::api::ThinkingConfig>,
    /// Per-tool observability counters. Updated on every dispatch in
    /// [`AgentLoop::execute_tool_call`] so `/usage` has live data.
    tool_usage: crate::tool_usage::ToolUsageTracker,
    /// Cooperative cancellation flag, shared with the CLI's Ctrl-C handler and
    /// (via `tool_context`) every running tool. `None` = no cancellation wired
    /// up (the default for library embeds and `--print` mode). Checked between
    /// steps, after a cancelled model stream, and around each tool call.
    interrupt: Option<Arc<AtomicBool>>,
}

impl AgentLoop {
    pub fn new(backend: Box<dyn AgentBackend>) -> Self {
        let rules = PermissionRules::load(None).unwrap_or_else(|_| PermissionRules::in_memory());
        Self {
            backend,
            messages: Vec::new(),
            cost_tracker: CostTracker::new(),
            model: String::from("unconfigured"),
            // Keep in sync with the clap default in
            // crates/deeptide-cli/src/main.rs::Cli::max_turns and the
            // "(unset — default N)" display string in
            // crates/deeptide-core/src/config.rs.
            max_turns: 200,
            current_run_step: 0,
            tool_registry: ToolRegistry::with_builtin_tools(),
            tool_context: ToolContext::new(
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            ),
            permission_manager: PermissionManager::new(PermissionMode::Default, rules),
            subagent_backend_factory: None,
            system_prompt: None,
            context_window: ContextWindowManager::new(ContextWindowConfig::default()),
            hooks: crate::hooks::HookEngine::empty(),
            allowed_tools: None,
            disallowed_tools: Vec::new(),
            ask_callback: None,
            tool_progress_callback: None,
            event_callback: None,
            thinking_override: None,
            tool_usage: crate::tool_usage::ToolUsageTracker::new(),
            interrupt: None,
        }
    }

    /// Attach a cooperative cancellation flag. The same `Arc` is shared with
    /// the tool context (so running shell/monitor commands observe it) and is
    /// checked by [`AgentLoop::run`] between steps and around each tool call.
    /// The CLI sets this from its Ctrl-C handler.
    pub fn with_interrupt_flag(mut self, flag: Arc<AtomicBool>) -> Self {
        self.tool_context.interrupt = Some(Arc::clone(&flag));
        self.interrupt = Some(flag);
        self
    }

    /// `true` when a cancellation has been requested for the current turn.
    fn is_interrupted(&self) -> bool {
        self.interrupt
            .as_ref()
            .is_some_and(|flag| flag.load(Ordering::Relaxed))
    }

    /// Read-only access to the per-tool observability counters. Used by
    /// `/usage` to render the dashboard.
    pub fn tool_usage(&self) -> &crate::tool_usage::ToolUsageTracker {
        &self.tool_usage
    }

    /// Clear every per-tool counter. Used by `/usage reset`.
    pub fn reset_tool_usage(&mut self) {
        self.tool_usage.reset();
    }

    /// Install a callback to invoke when a tool call needs interactive
    /// approval. Without this callback (the default), `Ask` decisions are
    /// returned to the model as errors — appropriate for `--print` mode and
    /// library embeds where there's no human at the keyboard.
    pub fn with_ask_callback(mut self, callback: PermissionAskCallback) -> Self {
        self.ask_callback = Some(callback);
        self
    }

    /// Install a callback fired around every tool invocation.
    /// The callback runs synchronously on the agent-loop thread
    /// (before and after each `execute_tool_call`); it should
    /// only do cheap work — typically updating an `Arc<Mutex<…>>`
    /// the spinner / input-row painter reads from — and must
    /// never block.
    ///
    /// Without this callback installed the agent loop runs
    /// silently between turns: the CLI's spinner shows
    /// "Crunching…" with no information about which tool is
    /// currently executing, which is especially confusing for
    /// slow tools like long Bash invocations.
    pub fn with_tool_progress_callback(mut self, callback: ToolProgressCallback) -> Self {
        self.tool_progress_callback = Some(callback);
        self
    }

    /// Install a live structured-event sink (see [`AgentEventCallback`]). Each
    /// `AgentLoopEvent` is delivered to the callback the instant it is produced
    /// during `run()`, in the same order as the returned `Vec`. Used by the GUI
    /// (live tool cards) and live headless streaming.
    pub fn with_event_callback(mut self, callback: AgentEventCallback) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// Deliver `event` to the live sink (if any) and append it to `events`.
    /// Routing every emission through this helper guarantees the live stream
    /// and the returned `Vec` carry the identical sequence.
    fn emit(&self, events: &mut Vec<AgentLoopEvent>, event: AgentLoopEvent) {
        if let Some(cb) = &self.event_callback {
            cb(&event);
        }
        events.push(event);
    }

    /// Update the active permission mode mid-session. Used by the REPL's
    /// Shift+Tab cycle and by [`AskOutcome::AllowAndSetMode`].
    pub fn set_permission_mode(&mut self, mode: PermissionMode) {
        self.permission_manager.set_mode(mode);
    }

    /// Install the lifecycle hook engine (built from `settings.json` hooks).
    /// `PreToolUse` hooks can block a tool call; `PostToolUse` hooks observe it.
    pub fn with_hooks(mut self, hooks: crate::hooks::HookEngine) -> Self {
        self.hooks = hooks;
        self
    }

    /// Restrict which tools this loop may call, mirroring a sub-agent
    /// definition's `allowedTools`/`disallowedTools`. `allowed = None` permits
    /// every tool except those in `disallowed`; `allowed = Some(list)` permits
    /// only `list` minus `disallowed`.
    pub fn with_tool_restrictions(
        mut self,
        allowed: Option<Vec<String>>,
        disallowed: Vec<String>,
    ) -> Self {
        self.allowed_tools = allowed;
        self.disallowed_tools = disallowed;
        self
    }

    /// Mutable counterpart to [`with_tool_restrictions`], for scoping a one-off
    /// pass (e.g. a memory-only consolidation over untrusted imported text)
    /// without rebuilding the loop. Returns the previous restrictions so the
    /// caller can restore them afterwards.
    pub fn set_tool_restrictions(
        &mut self,
        allowed: Option<Vec<String>>,
        disallowed: Vec<String>,
    ) -> (Option<Vec<String>>, Vec<String>) {
        let prev = (
            self.allowed_tools.take(),
            std::mem::take(&mut self.disallowed_tools),
        );
        self.allowed_tools = allowed;
        self.disallowed_tools = disallowed;
        prev
    }

    /// Temporarily narrow the active tool policy for a scoped pass without
    /// relaxing any existing caller restrictions. The requested allowlist is
    /// intersected with the current allowlist (when present), and the current
    /// denylist is preserved.
    pub fn set_tool_restrictions_intersecting(
        &mut self,
        requested_allowed: &[&str],
    ) -> (Option<Vec<String>>, Vec<String>) {
        let requested: Vec<String> = requested_allowed
            .iter()
            .map(|tool| (*tool).to_string())
            .collect();
        let narrowed = if let Some(current_allowed) = &self.allowed_tools {
            Some(
                requested
                    .into_iter()
                    .filter(|tool| current_allowed.iter().any(|current| current == tool))
                    .collect(),
            )
        } else {
            Some(requested)
        };
        self.set_tool_restrictions(narrowed, self.disallowed_tools.clone())
    }

    /// The set of tools to advertise to the model. `None` when unrestricted
    /// (advertise everything); otherwise only the permitted tools, so a
    /// restricted sub-agent is never offered a tool it cannot call.
    fn advertised_tools(&self) -> Option<Vec<String>> {
        if self.allowed_tools.is_none() && self.disallowed_tools.is_empty() {
            return None;
        }
        Some(
            self.tool_registry
                .names()
                .into_iter()
                .filter(|name| self.is_tool_permitted(name))
                .map(ToOwned::to_owned)
                .collect(),
        )
    }

    /// Whether `tool` may be called under the configured restrictions. Uses the
    /// same logic as the Swift `AgentDefinition.filterTools`.
    fn is_tool_permitted(&self, tool: &str) -> bool {
        if let Some(allowed) = &self.allowed_tools
            && !allowed.iter().any(|name| name == tool)
        {
            return false;
        }
        !self.disallowed_tools.iter().any(|name| name == tool)
    }

    /// Compact the in-memory transcript on demand (the `/compact` command),
    /// folding older turns into a rolling summary while preserving recent
    /// context and any tool_use/tool_result pairing. Returns a report of what
    /// was compressed.
    pub fn compact(&mut self) -> CompressionReport {
        self.context_window.force_compress(&mut self.messages)
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self.refresh_context_window();
        self
    }

    pub fn set_model(&mut self, model: impl Into<String>) {
        self.model = model.into();
        self.refresh_context_window();
    }

    /// Current extended-thinking override (the value the REPL has
    /// dialled in via `/think`, if any). `None` means "use whatever the
    /// backend was constructed with".
    pub fn thinking_override(&self) -> Option<&crate::api::ThinkingConfig> {
        self.thinking_override.as_ref()
    }

    /// Mid-session toggle of the extended-thinking directive. The
    /// override is threaded into every subsequent `AgentRequest` so the
    /// backend sees the new value on the *next* call without rebuild.
    /// Pass `None` to clear the override and defer back to the
    /// construction-time default.
    pub fn set_thinking_override(&mut self, thinking: Option<crate::api::ThinkingConfig>) {
        self.thinking_override = thinking;
    }

    /// Resize the context window to the current model's token budget, mirroring
    /// Swift's `resolvedContextWindow = ModelContextWindow.forModel(model)`.
    /// Auto-compaction then triggers at the model-appropriate threshold rather
    /// than a fixed default.
    fn refresh_context_window(&mut self) {
        let limit = usize::try_from(crate::tools::model_context_window(&self.model))
            .unwrap_or(DEFAULT_CONTEXT_WINDOW);
        self.context_window =
            ContextWindowManager::new(ContextWindowConfig::with_max_tokens(limit));
    }

    /// The model's context-window token budget currently driving auto-compaction.
    pub fn context_window_limit(&self) -> usize {
        self.context_window.config().max_tokens
    }

    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns.max(1);
        self
    }

    /// Mutable counterpart to [`with_max_turns`], for temporarily bounding a
    /// one-off pass (e.g. end-of-session consolidation) without rebuilding the
    /// loop. Returns the previous cap so the caller can restore it.
    pub fn set_max_turns(&mut self, max_turns: usize) -> usize {
        let prev = self.max_turns;
        self.max_turns = max_turns.max(1);
        prev
    }

    /// Replace the context-window manager configuration used for auto-compaction
    /// (the soft token threshold and preserved-message window).
    pub fn with_context_window_config(mut self, config: ContextWindowConfig) -> Self {
        self.context_window = ContextWindowManager::new(config);
        self
    }

    /// Replace the cost tracker with one that applies per-model pricing
    /// overrides (e.g. from `settings.json`). Keys are model identifiers; rates
    /// are per-token USD.
    pub fn with_pricing_overrides(
        mut self,
        overrides: std::collections::HashMap<String, crate::ModelPricing>,
    ) -> Self {
        self.cost_tracker = CostTracker::with_pricing_overrides(overrides);
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

    /// Set or replace the system prompt sent on every API request.
    ///
    /// An empty string clears the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        let s = prompt.into();
        self.system_prompt = if s.is_empty() { None } else { Some(s) };
        self
    }

    /// Return the current system prompt, if any.
    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn run(&mut self, user_input: impl Into<String>) -> Vec<AgentLoopEvent> {
        let user_input = user_input.into();

        // UserPromptSubmit hooks fire before the prompt is processed. They are
        // observational (the result is discarded), matching the Swift
        // implementation; the prompt text is passed as the hook input.
        if self
            .hooks
            .has_hooks(crate::hooks::HookEvent::UserPromptSubmit)
        {
            let _ = self.hooks.run(
                crate::hooks::HookEvent::UserPromptSubmit,
                None,
                Some(&user_input),
            );
        }

        let user_message = ConversationMessage::user(user_input);
        self.current_run_step = 0;
        self.messages.push(user_message.clone());

        let mut events = Vec::new();
        self.emit(&mut events, AgentLoopEvent::User(user_message));

        loop {
            // Auto-compact the transcript before assembling the next request,
            // mirroring the Swift turn loop's checkAndCompact step: compress
            // when over the token threshold, or when the message count alone
            // grows too large. A PreCompact hook fires after a compaction
            // actually happens (observational).
            let mut report = self.context_window.compress(&mut self.messages);
            if !report.did_compress && self.messages.len() > MAX_MESSAGES_BEFORE_COMPACT {
                report = self.context_window.force_compress(&mut self.messages);
            }
            if report.did_compress {
                self.emit(&mut events, AgentLoopEvent::Compaction(report));
                if self.hooks.has_hooks(crate::hooks::HookEvent::PreCompact) {
                    let _ = self
                        .hooks
                        .run(crate::hooks::HookEvent::PreCompact, None, None);
                }
            }

            // Circuit breaker: if the transcript is still over the hard limit
            // after compaction, stop instead of issuing a request the model
            // will reject. Mirrors Swift's isBlocked terminal.
            if self.context_window.is_blocked(&self.messages) {
                self.emit(
                    &mut events,
                    AgentLoopEvent::Terminal(AgentTerminalEvent::Blocked),
                );
                return events;
            }

            if self.current_run_step >= self.max_turns {
                self.emit(
                    &mut events,
                    AgentLoopEvent::Terminal(AgentTerminalEvent::MaxTurnsReached {
                        cap: self.max_turns,
                    }),
                );
                return events;
            }

            // Cancellation requested between steps (e.g. Ctrl-C while we were
            // about to issue the next request). Stop before spending a turn.
            if self.is_interrupted() {
                self.emit(
                    &mut events,
                    AgentLoopEvent::Terminal(AgentTerminalEvent::Interrupted),
                );
                return events;
            }

            self.current_run_step += 1;
            let request = AgentRequest {
                messages: self.messages.clone(),
                model: self.model.clone(),
                step: self.current_run_step,
                max_turns: self.max_turns,
                system: self.system_prompt.clone(),
                allowed_tools: self.advertised_tools(),
                thinking: self.thinking_override.clone(),
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

                    let tool_calls_for_message = response.tool_calls.clone();
                    let assistant_message = ConversationMessage::assistant_with_tool_calls(
                        response.content,
                        tool_calls_for_message,
                    );
                    self.messages.push(assistant_message.clone());
                    self.emit(&mut events, AgentLoopEvent::Assistant(assistant_message));

                    if response.tool_calls.is_empty() {
                        self.emit(
                            &mut events,
                            AgentLoopEvent::Terminal(AgentTerminalEvent::Complete),
                        );
                        return events;
                    }

                    let total_tools = response.tool_calls.len();
                    let mut tool_results = Vec::with_capacity(total_tools);
                    let mut failure_summaries = Vec::new();
                    // Set once a cancellation is observed mid-batch. Remaining
                    // tools are NOT executed — but they still get a synthetic
                    // "cancelled" tool_result below so every `tool_use` block
                    // has a matching `tool_result` (Anthropic rejects a
                    // transcript where one doesn't), keeping the session
                    // resumable after the interrupt.
                    let mut interrupted = false;
                    for (idx, tool_call) in response.tool_calls.into_iter().enumerate() {
                        if interrupted || self.is_interrupted() {
                            interrupted = true;
                            let content = String::from("Cancelled by user (Ctrl-C).");
                            failure_summaries
                                .push(ToolBatchFailureClassifier::classify(&content, true));
                            tool_results.push((tool_call, content, true));
                            continue;
                        }
                        // Fire the "tool starting" hook BEFORE we
                        // execute. The CLI uses this to swap the
                        // spinner / pinned-row hint from a generic
                        // "Crunching…" to a tool-specific label
                        // (`Bash(npm test)`, `Read(/etc/hosts)`, …)
                        // — without it the user has no idea what's
                        // taking the wall-clock time during slow
                        // tool calls.
                        if let Some(cb) = self.tool_progress_callback.as_ref() {
                            cb(&ToolProgressEvent::Started {
                                index: idx,
                                total: total_tools,
                                name: tool_call.name.clone(),
                                preview: tool_progress_preview(&tool_call),
                            });
                        }
                        let result = self.execute_tool_call(&tool_call);
                        let content = result.content;
                        let is_error = result.is_error;
                        // Fire the "tool finished" hook BEFORE we
                        // touch any local state — the CLI flips
                        // the spinner back into neutral mode here.
                        if let Some(cb) = self.tool_progress_callback.as_ref() {
                            cb(&ToolProgressEvent::Finished {
                                index: idx,
                                total: total_tools,
                                name: tool_call.name.clone(),
                                is_error,
                            });
                        }
                        if is_error {
                            failure_summaries
                                .push(ToolBatchFailureClassifier::classify(&content, is_error));
                        }
                        tool_results.push((tool_call, content, is_error));
                        // The tool may have been cancelled mid-run (its poll
                        // loop observed the flag and killed the child). Stop
                        // executing the rest of the batch.
                        if self.is_interrupted() {
                            interrupted = true;
                        }
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
                    // ⚠ Ordering matters: ToolResult events first,
                    // ToolBatchSummary last. We render each result
                    // inline (✓ Glob, ✓ Bash, …) and the summary
                    // is the "Tools completed: …" line that wraps
                    // the whole batch — emitting the summary FIRST
                    // (as we used to) made the UI read backwards
                    // ("· Tools completed: Ran 2 tools" before the
                    // user has seen any tool actually finish), so
                    // we now emit results first and the summary
                    // last.
                    let mut result_blocks = Vec::with_capacity(tool_results.len());
                    for (tool_call, content, is_error) in tool_results {
                        self.emit(
                            &mut events,
                            AgentLoopEvent::ToolResult {
                                tool_call: tool_call.clone(),
                                content: content.clone(),
                                is_error,
                            },
                        );
                        result_blocks.push(ToolResultBlock::new(
                            tool_call.id.clone(),
                            content,
                            is_error,
                        ));
                    }
                    self.emit(
                        &mut events,
                        AgentLoopEvent::ToolBatchSummary {
                            label: ToolBatchLabeler::label_with_failure_summaries(
                                &label_items,
                                &failure_summaries,
                            ),
                            tool_calls,
                            failed_count,
                        },
                    );
                    if !result_blocks.is_empty() {
                        self.messages
                            .push(ConversationMessage::tool_results(result_blocks));
                    }

                    // The transcript is now valid (every tool_use has a
                    // matching tool_result, including the synthetic cancelled
                    // ones). Stop here rather than issuing another request.
                    if interrupted {
                        self.emit(
                            &mut events,
                            AgentLoopEvent::Terminal(AgentTerminalEvent::Interrupted),
                        );
                        return events;
                    }
                }
                Err(error) => {
                    // A cancelled stream surfaces here as a transport error.
                    // Report it as an interruption (not a model error) so the
                    // UI shows "Interrupted by user" rather than a scary
                    // connection-failure line.
                    if self.is_interrupted() {
                        self.emit(
                            &mut events,
                            AgentLoopEvent::Terminal(AgentTerminalEvent::Interrupted),
                        );
                        return events;
                    }
                    self.emit(
                        &mut events,
                        AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(error)),
                    );
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

    /// Replace the conversation history with the supplied messages.
    ///
    /// Used by `/resume` to restore a previously saved session transcript.
    /// Resets the cost tracker and turn counter since the historical costs are
    /// no longer tracked in memory.
    pub fn restore_messages(&mut self, messages: Vec<ConversationMessage>) {
        self.messages = messages;
        self.cost_tracker.reset();
        self.current_run_step = 0;
    }

    /// Replace the entire conversation transcript with `messages`. Used
    /// by `/restore` (resume from saved session) and `/checkpoint
    /// restore` (rewind to an in-session snapshot) so the loop's
    /// internal state stays consistent with what the rest of the REPL
    /// sees via [`messages`].
    ///
    /// Does NOT reset the cost tracker or the current step counter —
    /// those are cumulative session telemetry, not transcript data.
    /// Future turns continue numbering from where they were.
    ///
    /// [`messages`]: AgentLoop::messages
    pub fn replace_messages(&mut self, messages: Vec<ConversationMessage>) {
        self.messages = messages;
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

    pub fn permission_mode(&self) -> PermissionMode {
        self.permission_manager.mode()
    }

    pub fn permission_rules(&self) -> &PermissionRules {
        self.permission_manager.rules()
    }

    pub fn add_permission_rule(
        &mut self,
        allowed: bool,
        pattern: impl Into<String>,
        tool: Option<String>,
    ) -> io::Result<()> {
        self.permission_manager.add_rule(allowed, pattern, tool)
    }

    pub fn remove_permission_rule(&mut self, pattern: &str) -> io::Result<()> {
        self.permission_manager.remove_rule(pattern)
    }

    fn execute_tool_call(&mut self, tool_call: &ToolCall) -> crate::ToolResult {
        // Wall-clock the entire dispatch (permission check + hooks +
        // actual tool body) so `/usage` reflects what the user would
        // perceive as "this tool was slow", not just the inner call.
        let start = std::time::Instant::now();
        let result = self.execute_tool_call_inner(tool_call);
        self.tool_usage.record(
            &tool_call.name,
            start.elapsed(),
            result.is_error,
            result.content.len(),
        );
        result
    }

    fn execute_tool_call_inner(&mut self, tool_call: &ToolCall) -> crate::ToolResult {
        // Enforce sub-agent tool restrictions before any dispatch (including the
        // special-cased EnterPlanMode/ExitPlanMode/Agent/Brief/Snip/CtxInspect
        // paths), so a restricted agent cannot reach a forbidden tool.
        if !self.is_tool_permitted(&tool_call.name) {
            return crate::ToolResult::error(format!(
                "Tool {} is not available to this agent.",
                tool_call.name
            ));
        }

        if tool_call.name == "EnterPlanMode" {
            return self.execute_plan_mode_transition(tool_call, PermissionMode::Plan);
        }
        if tool_call.name == "ExitPlanMode"
            && self.permission_manager.mode() == PermissionMode::Plan
        {
            return self.execute_plan_mode_transition(tool_call, PermissionMode::Default);
        }

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
                // PreToolUse hooks may veto the call (non-zero exit blocks it).
                if self.hooks.has_hooks(crate::hooks::HookEvent::PreToolUse) {
                    let input_json = serde_json::to_string(&tool_call.input).ok();
                    if let Some(reason) = self
                        .hooks
                        .pre_tool_block_reason(&tool_call.name, input_json.as_deref())
                    {
                        return crate::ToolResult::error(reason);
                    }
                }

                let result = if tool_call.name == "Agent"
                    && let Some(result) = self.execute_subagent_tool_call(tool_call)
                {
                    result
                } else if tool_call.name == "CtxInspect" {
                    self.execute_ctx_inspect_tool_call(tool_call)
                } else if tool_call.name == "Snip" {
                    self.execute_snip_tool_call(tool_call)
                } else if tool_call.name == "Brief" {
                    self.execute_brief_tool_call(tool_call)
                } else {
                    self.tool_registry.call(
                        &tool_call.name,
                        tool_call.input.clone(),
                        &self.tool_context,
                    )
                };

                // PostToolUse hooks observe the completed call (non-blocking).
                if self.hooks.has_hooks(crate::hooks::HookEvent::PostToolUse) {
                    let input_json = serde_json::to_string(&tool_call.input).ok();
                    let _ = self.hooks.run(
                        crate::hooks::HookEvent::PostToolUse,
                        Some(&tool_call.name),
                        input_json.as_deref(),
                    );
                }

                result
            }
            PermissionDecision::Deny { reason } => crate::ToolResult::error(format!(
                "Permission denied for {}: {reason}",
                tool_call.name
            )),
            PermissionDecision::Ask => self.handle_ask_decision(tool_call),
        }
    }

    /// Resolve an `Ask` permission decision either by prompting the user
    /// (when an interactive callback is installed) or by returning the
    /// long-standing non-interactive error so headless callers behave
    /// unchanged.
    fn handle_ask_decision(&mut self, tool_call: &ToolCall) -> crate::ToolResult {
        let Some(callback) = self.ask_callback.clone() else {
            return crate::ToolResult::error(format!(
                "Permission required for {}. Re-run with --permission-mode accept-edits or add an allow rule to approve this tool call.",
                tool_call.name
            ));
        };

        let outcome = callback(tool_call);
        match outcome {
            AskOutcome::Allow => self.dispatch_approved_tool_call(tool_call),
            AskOutcome::AllowAllSession { tool_name } => {
                // Wildcard pattern bound to the tool name: matches every
                // future call to this tool regardless of arguments. The
                // rule lives in the session_allow_list (in-memory only) so
                // it disappears at REPL exit — users can't accidentally
                // grant durable trust by accepting one prompt.
                self.permission_manager
                    .add_session_rule(true, &tool_name, "*");
                self.dispatch_approved_tool_call(tool_call)
            }
            AskOutcome::AllowAndSetMode(mode) => {
                self.permission_manager.set_mode(mode);
                self.dispatch_approved_tool_call(tool_call)
            }
            AskOutcome::Deny { reason } => crate::ToolResult::error(format!(
                "Permission denied for {}: {reason}",
                tool_call.name
            )),
        }
    }

    /// Execute a tool call that the user just approved, honouring the same
    /// `PreToolUse` / `PostToolUse` hook lifecycle as the auto-allow path so
    /// users can't sidestep their hooks by clicking "yes" at the prompt.
    fn dispatch_approved_tool_call(&mut self, tool_call: &ToolCall) -> crate::ToolResult {
        if self.hooks.has_hooks(crate::hooks::HookEvent::PreToolUse) {
            let input_json = serde_json::to_string(&tool_call.input).ok();
            if let Some(reason) = self
                .hooks
                .pre_tool_block_reason(&tool_call.name, input_json.as_deref())
            {
                return crate::ToolResult::error(reason);
            }
        }

        let result = if tool_call.name == "Agent"
            && let Some(result) = self.execute_subagent_tool_call(tool_call)
        {
            result
        } else if tool_call.name == "CtxInspect" {
            self.execute_ctx_inspect_tool_call(tool_call)
        } else if tool_call.name == "Snip" {
            self.execute_snip_tool_call(tool_call)
        } else if tool_call.name == "Brief" {
            self.execute_brief_tool_call(tool_call)
        } else {
            self.tool_registry
                .call(&tool_call.name, tool_call.input.clone(), &self.tool_context)
        };

        if self.hooks.has_hooks(crate::hooks::HookEvent::PostToolUse) {
            let input_json = serde_json::to_string(&tool_call.input).ok();
            let _ = self.hooks.run(
                crate::hooks::HookEvent::PostToolUse,
                Some(&tool_call.name),
                input_json.as_deref(),
            );
        }

        result
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
        let allowed_tools = invocation
            .definition
            .allowed_tools
            .map(|tools| tools.iter().map(|tool| (*tool).to_owned()).collect());
        let disallowed_tools = invocation
            .definition
            .disallowed_tools
            .iter()
            .map(|tool| (*tool).to_owned())
            .collect();
        let system_prompt = invocation
            .definition
            .full_system_prompt(&self.tool_context.cwd, &subagent_model);
        let mut subagent = AgentLoop::new(factory(&subagent_model))
            .with_model(subagent_model.clone())
            .with_max_turns(invocation.definition.max_turns)
            .with_cwd(self.tool_context.cwd.clone())
            .with_permission_mode(permission_mode)
            .with_tool_restrictions(allowed_tools, disallowed_tools)
            .with_system_prompt(system_prompt);
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

/// Render a compact one-line preview of a tool call, used inside
/// [`ToolProgressEvent::Started`] so the UI can show "Bash(npm
/// test)" / "Read(/etc/hosts)" / "Glob(\*\*/\*.rs)" rather than a
/// bare "Crunching…". Mirrors the style of the post-turn batch
/// label but for a single call.
///
/// Lookup priority for the descriptor portion:
///
/// 1. Tool-specific key in `input` (`command` for Bash, `path`
///    or `file_path` for Read/Edit/Write, `pattern` for Glob,
///    `query` for Grep / WebSearch, `notebook_path` for
///    Notebook tools), shortened to a path tail when it looks
///    like a filesystem path.
/// 2. First string-valued input field if no canonical key
///    matches (covers custom MCP tools).
/// 3. Just the tool name with no parens — never empty.
///
/// All input strings are truncated at `MAX_PREVIEW` chars with a
/// trailing `…` so a multi-megabyte file path or a giant shell
/// command can't blow out the spinner row width.
fn tool_progress_preview(tool_call: &ToolCall) -> String {
    const MAX_PREVIEW: usize = 48;

    let descriptor = canonical_preview_field(&tool_call.name, &tool_call.input)
        .or_else(|| first_string_field(&tool_call.input));

    let Some(raw) = descriptor else {
        return tool_call.name.clone();
    };

    let display = shorten_for_preview(&raw, &tool_call.name);
    let truncated = if display.chars().count() > MAX_PREVIEW {
        let mut s: String = display
            .chars()
            .take(MAX_PREVIEW.saturating_sub(1))
            .collect();
        s.push('…');
        s
    } else {
        display
    };
    format!("{}({truncated})", tool_call.name)
}

/// Lookup table for "what input key best describes this tool?".
/// Returns the raw string value if present; the caller handles
/// truncation and path tail extraction.
fn canonical_preview_field(name: &str, input: &serde_json::Value) -> Option<String> {
    let key = match name.to_ascii_lowercase().as_str() {
        "bash" | "shell" | "run" => "command",
        "read" | "view" => "file_path",
        "write" | "edit" | "multiedit" | "str_replace" | "str_replace_editor" => "file_path",
        "glob" => "pattern",
        "grep" | "search" => "pattern",
        "websearch" | "web_search" => "query",
        "webfetch" | "web_fetch" => "url",
        "notebook_read" | "notebook_edit" => "notebook_path",
        "task" => "description",
        _ => return None,
    };
    // Try the canonical key plus a small set of common aliases
    // (path / file / cmd) so we still find something useful when
    // a tool schema drifts.
    for candidate in [key, "path", "file", "cmd", "input"] {
        if let Some(value) = input.get(candidate)
            && let Some(s) = value.as_str()
            && !s.trim().is_empty()
        {
            return Some(s.to_owned());
        }
    }
    None
}

/// Fallback descriptor: pick the first string-valued field from
/// the input object. Skips objects/arrays/nulls — only flat
/// strings get surfaced, otherwise the preview becomes a dump.
fn first_string_field(input: &serde_json::Value) -> Option<String> {
    input
        .as_object()
        .and_then(|map| {
            map.values()
                .find_map(|value| value.as_str().filter(|s| !s.trim().is_empty()))
        })
        .map(str::to_owned)
}

/// Trim filesystem-pathy descriptors down to the last 1–2 segments
/// so an absolute `/Users/ryan/very/deep/repo/file.rs` previews as
/// `repo/file.rs`. Non-path strings (shell commands, Glob
/// patterns, queries) are returned unchanged.
fn shorten_for_preview(raw: &str, tool_name: &str) -> String {
    let is_path_tool = matches!(
        tool_name.to_ascii_lowercase().as_str(),
        "read"
            | "view"
            | "write"
            | "edit"
            | "multiedit"
            | "str_replace"
            | "str_replace_editor"
            | "notebook_read"
            | "notebook_edit"
    );
    if !is_path_tool {
        return raw.trim().to_owned();
    }
    let trimmed = raw.trim();
    let segments: Vec<&str> = trimmed.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() <= 2 {
        return trimmed.to_owned();
    }
    let tail: Vec<&str> = segments.iter().rev().take(2).rev().copied().collect();
    format!("…/{}", tail.join("/"))
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
            AgentLoopEvent::Terminal(AgentTerminalEvent::MaxTurnsReached { cap }) => {
                lines.push(String::new());
                lines.push(format!(
                    "Sub-agent stopped after reaching the turn limit ({cap} turns)."
                ));
            }
            _ => {}
        }
    }

    crate::ToolResult::text(lines.join("\n"))
}

impl AgentLoop {
    fn execute_ctx_inspect_tool_call(&self, tool_call: &ToolCall) -> crate::ToolResult {
        let mut input = tool_call
            .input
            .as_object()
            .cloned()
            .unwrap_or_else(serde_json::Map::new);
        input
            .entry(String::from("model"))
            .or_insert_with(|| serde_json::Value::String(self.model.clone()));
        input
            .entry(String::from("estimated_tokens"))
            .or_insert_with(|| serde_json::Value::from(estimate_context_tokens(&self.messages)));
        input
            .entry(String::from("message_count"))
            .or_insert_with(|| serde_json::Value::from(self.messages.len()));

        self.tool_registry.call(
            &tool_call.name,
            serde_json::Value::Object(input),
            &self.tool_context,
        )
    }

    fn execute_snip_tool_call(&mut self, tool_call: &ToolCall) -> crate::ToolResult {
        let result =
            self.tool_registry
                .call(&tool_call.name, tool_call.input.clone(), &self.tool_context);
        if !result.is_error {
            let keep_last = snip_keep_last(&tool_call.input);
            self.trim_messages_for_snip(keep_last);
        }
        result
    }

    /// Handle a model-issued `Brief` call: surface the tool's message, then
    /// actually fold older turns into a rolling summary (the tool alone cannot
    /// reach the transcript, so without this the "compaction triggered" reply
    /// would be a no-op). Mirrors the `Snip` side-effect pattern.
    fn execute_brief_tool_call(&mut self, tool_call: &ToolCall) -> crate::ToolResult {
        let result =
            self.tool_registry
                .call(&tool_call.name, tool_call.input.clone(), &self.tool_context);
        if !result.is_error {
            self.compact();
        }
        result
    }

    fn trim_messages_for_snip(&mut self, keep_last: usize) {
        if self.messages.len() <= keep_last {
            return;
        }

        let removed_count = self.messages.len() - keep_last;
        let mut kept = self.messages.split_off(removed_count);
        let marker = ConversationMessage::user(format!(
            "[context trimmed by Snip: {removed_count} older messages removed; {keep_last} recent messages kept]"
        ));
        self.messages.clear();
        self.messages.push(marker);
        self.messages.append(&mut kept);
    }

    fn execute_plan_mode_transition(
        &mut self,
        tool_call: &ToolCall,
        mode: PermissionMode,
    ) -> crate::ToolResult {
        let result =
            self.tool_registry
                .call(&tool_call.name, tool_call.input.clone(), &self.tool_context);
        if !result.is_error {
            self.permission_manager.set_mode(mode);
        }
        result
    }
}

fn estimate_context_tokens(messages: &[ConversationMessage]) -> u64 {
    messages
        .iter()
        .map(|message| (message.content.chars().count() as u64).div_ceil(4) + 4)
        .sum()
}

fn snip_keep_last(input: &serde_json::Value) -> usize {
    input
        .get("keepLast")
        .or_else(|| input.get("keep_last"))
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(10)
        .clamp(1, 100)
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tool_progress_preview_tests {
    use super::{ToolCall, tool_progress_preview};
    use serde_json::json;

    fn call(name: &str, input: serde_json::Value) -> ToolCall {
        ToolCall {
            id: format!("call_{name}"),
            name: name.to_owned(),
            input,
        }
    }

    #[test]
    fn bash_uses_command_field() {
        let tc = call("Bash", json!({"command": "npm test"}));
        assert_eq!(tool_progress_preview(&tc), "Bash(npm test)");
    }

    #[test]
    fn read_shortens_long_paths_to_last_two_segments() {
        let tc = call(
            "Read",
            json!({"file_path": "/Users/ryan/a8e/paean/repo/file.rs"}),
        );
        // Long absolute path → tail with leading "…/" marker so the
        // user knows it's been abbreviated.
        assert_eq!(tool_progress_preview(&tc), "Read(…/repo/file.rs)");
    }

    #[test]
    fn read_short_path_kept_as_is() {
        let tc = call("Read", json!({"file_path": "src/lib.rs"}));
        assert_eq!(tool_progress_preview(&tc), "Read(src/lib.rs)");
    }

    #[test]
    fn glob_uses_pattern_field_without_path_shortening() {
        // Glob patterns aren't paths — they should pass through
        // verbatim even when they contain `/`.
        let tc = call("Glob", json!({"pattern": "**/*.rs"}));
        assert_eq!(tool_progress_preview(&tc), "Glob(**/*.rs)");
    }

    #[test]
    fn grep_uses_pattern_field() {
        let tc = call("Grep", json!({"pattern": "fn main"}));
        assert_eq!(tool_progress_preview(&tc), "Grep(fn main)");
    }

    #[test]
    fn websearch_uses_query_field() {
        let tc = call("WebSearch", json!({"query": "rust 2026 edition"}));
        assert_eq!(tool_progress_preview(&tc), "WebSearch(rust 2026 edition)");
    }

    #[test]
    fn unknown_tool_falls_back_to_first_string_field() {
        let tc = call(
            "MyCustomTool",
            json!({"unused_int": 42, "label": "hello world"}),
        );
        let out = tool_progress_preview(&tc);
        assert!(out.starts_with("MyCustomTool("), "got: {out}");
        assert!(out.contains("hello world"), "got: {out}");
    }

    #[test]
    fn empty_input_falls_back_to_bare_tool_name() {
        let tc = call("Ping", json!({}));
        assert_eq!(tool_progress_preview(&tc), "Ping");
    }

    #[test]
    fn truncates_oversized_descriptors_with_ellipsis() {
        let huge = "a".repeat(200);
        let tc = call("Bash", json!({"command": huge}));
        let out = tool_progress_preview(&tc);
        // Total visible length stays bounded; truncation marker
        // appears inside the parens so the user knows there was
        // more to the command.
        assert!(out.ends_with("…)"), "got: {out}");
        assert!(
            out.chars().count() < 80,
            "expected truncation to bound width: {out}"
        );
    }

    #[test]
    fn write_aliases_to_path_field() {
        let tc = call("Write", json!({"file_path": "/tmp/x.txt"}));
        assert_eq!(tool_progress_preview(&tc), "Write(/tmp/x.txt)");
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod event_callback_tests {
    use super::{AgentEventCallback, AgentLoop, AgentLoopEvent, LocalEchoBackend};
    use std::sync::{Arc, Mutex};

    #[test]
    fn live_event_callback_receives_the_same_sequence_as_the_returned_vec() {
        // The live sink (used by the GUI / live headless streaming) must observe
        // EXACTLY the events run() returns, in order — otherwise the returned Vec
        // and the live stream could diverge. LocalEchoBackend emits no tool calls,
        // so the turn is User → Assistant → Terminal(Complete).
        let seen: Arc<Mutex<Vec<AgentLoopEvent>>> = Arc::new(Mutex::new(Vec::new()));
        let sink = Arc::clone(&seen);
        let cb: AgentEventCallback = Arc::new(move |event: &AgentLoopEvent| {
            sink.lock().unwrap().push(event.clone());
        });

        let mut loop_ = AgentLoop::new(Box::new(LocalEchoBackend)).with_event_callback(cb);
        let returned = loop_.run("hello there");

        let live = seen.lock().unwrap().clone();
        assert_eq!(
            live, returned,
            "live callback sequence must equal the returned Vec"
        );
        // Sanity: the expected shape for a no-tool turn.
        assert!(matches!(returned.first(), Some(AgentLoopEvent::User(_))));
        assert!(matches!(returned.last(), Some(AgentLoopEvent::Terminal(_))));
        assert_eq!(returned.len(), 3, "User, Assistant, Terminal: {returned:?}");
    }

    #[test]
    fn no_callback_still_returns_events() {
        // Backwards-compatible: without a sink, run() behaves exactly as before.
        let mut loop_ = AgentLoop::new(Box::new(LocalEchoBackend));
        let returned = loop_.run("hi");
        assert_eq!(returned.len(), 3);
    }
}
