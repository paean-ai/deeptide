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
// `AssistantDelta` / `ThinkingDelta` are constructed once real (streaming)
// model backends are wired via the shared host builder; the MVP's echo backend
// only emits whole `Assistant` messages.
#[allow(dead_code)]
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
    /// The turn ended.
    Terminal(TerminalKind),
    /// A worker-side failure (e.g. a panic caught around `run()`).
    Error(String),
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
