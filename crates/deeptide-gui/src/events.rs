//! The message types crossing the UI ⇄ worker-thread boundary.
//!
//! The agent core is synchronous and blocking, so each conversation runs on a
//! dedicated worker thread. The UI thread (egui) and the worker never share
//! mutable state — they only exchange these messages over channels:
//!  - [`WorkerMsg`] : UI → worker (a prompt to run; later, permission replies).
//!  - [`UiEvent`]   : worker → UI (streamed text, tool activity, terminal).

/// A message sent from the egui UI thread to a conversation's worker thread.
#[derive(Debug, Clone)]
pub enum WorkerMsg {
    /// Run a user prompt as a new agent turn.
    Prompt(String),
}

/// An event sent from a worker thread to the UI thread. Mirrors the subset of
/// `deeptide_core::AgentLoopEvent` / streaming the UI renders; the worker maps
/// core events to these so the UI never depends on core's internal event shape.
#[derive(Debug, Clone)]
pub enum UiEvent {
    /// Live token of assistant answer text (from the backend StreamingHandler).
    AssistantDelta(String),
    /// Live token of model reasoning / chain-of-thought (shown dimmed).
    ThinkingDelta(String),
    /// A complete assistant message (emitted by the loop after a turn step).
    /// Used as the source of truth when the backend doesn't stream deltas.
    Assistant(String),
    /// One finished tool call.
    ToolResult {
        tool: String,
        content: String,
        is_error: bool,
    },
    /// Summary line wrapping a batch of tool calls.
    ToolBatch { label: String, failed: usize },
    /// A gated tool needs the user's approval. The worker thread is BLOCKED in
    /// its `ask_callback` until the UI replies via `Conversation::respond_permission`.
    PermissionRequest {
        /// Correlates the reply to the blocked request (the tool-call id).
        req_id: String,
        /// Tool name (e.g. `Write`, `Bash`).
        tool: String,
        /// Short human preview of what the tool will do.
        preview: String,
    },
    /// The turn ended.
    Terminal(TerminalKind),
    /// The conversation was persisted (the worker saved it to the shared
    /// `SessionStore`); the UI should refresh its session sidebar.
    SessionsChanged,
    /// A resumed session's prior transcript, for the UI to render before the
    /// user continues it.
    Hydrate(Vec<HydratedBubble>),
    /// Updated cumulative token/cost totals (emitted after each turn).
    Usage(Usage),
    /// A worker-side failure (e.g. a panic caught around `run()`).
    Error(String),
}

/// Cumulative token/cost totals for the conversation, shown in the status bar.
#[derive(Debug, Clone, Copy, Default)]
pub struct Usage {
    pub input: usize,
    pub output: usize,
    pub cache_read: usize,
    pub cost_usd: f64,
}

/// One message restored from a saved session, for hydrating the transcript on
/// resume. Only user/assistant text is rebuilt (a readable history); tool
/// details aren't replayed.
#[derive(Debug, Clone)]
pub struct HydratedBubble {
    pub is_user: bool,
    pub text: String,
}

/// How a turn ended — mirrors `deeptide_core::AgentTerminalEvent`.
#[derive(Debug, Clone)]
pub enum TerminalKind {
    Complete,
    MaxTurns(usize),
    ModelError(String),
    Blocked,
    Interrupted,
}
