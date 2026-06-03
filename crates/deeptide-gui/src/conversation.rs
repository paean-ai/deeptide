//! A conversation = one dedicated worker thread owning an `AgentLoop`.
//!
//! `AgentLoop::run()` is synchronous and blocks for the whole turn, so it must
//! never run on egui's UI thread. Each conversation owns a worker thread; the
//! UI talks to it only via channels and a shared interrupt flag. The worker
//! installs the core's live event callback plus a `StreamingHandler` so events
//! (token deltas, tool activity, terminal) flow to the UI AS THEY HAPPEN, then
//! calls `ctx.request_repaint()` to wake the UI immediately.
//!
//! The backend is built from the SHARED config (`settings.json`) through the
//! same `deeptide-host` builder the CLI uses, so the GUI talks to the identical
//! model/provider with no duplicated wiring. When no credential is configured,
//! the host builder yields the local-echo backend and the UI shows a banner.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

use crossbeam_channel::{Receiver, Sender, unbounded};
use deeptide_core::{
    AgentEventCallback, AgentLoop, AgentLoopEvent, AgentTerminalEvent, StreamingEvent,
    StreamingHandler,
};
use eframe::egui;

use crate::events::{TerminalKind, UiEvent, WorkerMsg};

/// Handle the UI holds to one running conversation.
pub struct Conversation {
    to_worker: Sender<WorkerMsg>,
    /// Drained by the UI each frame.
    pub from_worker: Receiver<UiEvent>,
    /// Cooperative cancel — flipped from the UI's Stop button; the loop and
    /// every running tool observe it.
    interrupt: Arc<AtomicBool>,
}

impl Conversation {
    /// Spawn the worker thread. `ctx` is cloned so the worker can wake the UI on
    /// each event (egui's `Context` is `Send + Sync + Clone`).
    pub fn spawn(ctx: egui::Context) -> Self {
        let (to_worker, worker_rx) = unbounded::<WorkerMsg>();
        let (worker_tx, from_worker) = unbounded::<UiEvent>();
        let interrupt = Arc::new(AtomicBool::new(false));
        let interrupt_for_worker = Arc::clone(&interrupt);
        thread::Builder::new()
            .name("deeptide-conversation".to_owned())
            .spawn(move || worker_loop(worker_rx, worker_tx, ctx, interrupt_for_worker))
            .expect("spawn conversation worker thread");
        Self {
            to_worker,
            from_worker,
            interrupt,
        }
    }

    /// Enqueue a user prompt; output arrives asynchronously via `from_worker`.
    pub fn send_prompt(&self, text: String) {
        let _ = self.to_worker.send(WorkerMsg::Prompt(text));
    }

    /// Request cooperative cancellation of the in-flight turn.
    pub fn interrupt(&self) {
        self.interrupt.store(true, Ordering::Relaxed);
    }
}

/// The worker thread body. Builds the loop once (so message history persists
/// across turns), then blocks on the inbound channel until a prompt arrives or
/// the UI drops the sender (channel closed → loop ends → thread exits).
fn worker_loop(
    rx: Receiver<WorkerMsg>,
    tx: Sender<UiEvent>,
    ctx: egui::Context,
    interrupt: Arc<AtomicBool>,
) {
    // Live structured-event sink: map each core event to a UI event, send it,
    // and wake the UI. Synchronous and cheap (just a channel send + repaint).
    let tx_events = tx.clone();
    let ctx_events = ctx.clone();
    let event_cb: AgentEventCallback = Arc::new(move |event: &AgentLoopEvent| {
        if let Some(ui_event) = map_event(event) {
            let _ = tx_events.send(ui_event);
            ctx_events.request_repaint();
        }
    });

    // Live token stream: the backend fires this per delta during a turn. Answer
    // text → AssistantDelta, reasoning → ThinkingDelta; both wake the UI.
    let tx_stream = tx.clone();
    let ctx_stream = ctx.clone();
    let streaming_handler: StreamingHandler = Arc::new(move |event: &StreamingEvent| {
        let ui_event = match event {
            StreamingEvent::TextDelta { delta, .. } if !delta.is_empty() => {
                Some(UiEvent::AssistantDelta(delta.clone()))
            }
            StreamingEvent::ThinkingDelta { delta } if !delta.is_empty() => {
                Some(UiEvent::ThinkingDelta(delta.clone()))
            }
            _ => None,
        };
        if let Some(ui_event) = ui_event {
            let _ = tx_stream.send(ui_event);
            ctx_stream.request_repaint();
        }
    });

    // Build the real backend from the shared config (same settings.json the CLI
    // reads), through the shared host builder — identical to the CLI's path.
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let params = deeptide_host::config::resolve_backend_params(&cwd);
    let configured = match deeptide_host::backend::build_backend(
        &params,
        Some(streaming_handler),
        Some(Arc::clone(&interrupt)),
    ) {
        Ok(configured) => configured,
        Err(error) => {
            let _ = tx.send(UiEvent::Error(format!(
                "failed to configure backend: {error}"
            )));
            ctx.request_repaint();
            return;
        }
    };
    if !configured.is_configured {
        let _ = tx.send(UiEvent::Error(
            "no API key configured — set DEEPTIDE_API_KEY (or settings.json) to talk to a model"
                .to_owned(),
        ));
        ctx.request_repaint();
    }

    let mut agent = AgentLoop::new(configured.backend)
        .with_model(configured.model)
        .with_event_callback(event_cb)
        .with_interrupt_flag(Arc::clone(&interrupt));

    while let Ok(msg) = rx.recv() {
        match msg {
            WorkerMsg::Prompt(text) => {
                // Clear any stale cancellation from a previous turn.
                interrupt.store(false, Ordering::Relaxed);
                // A tool panic must not take down the whole app: catch it and
                // surface a synthetic error instead.
                let outcome = catch_unwind(AssertUnwindSafe(|| {
                    let _ = agent.run(text);
                }));
                if outcome.is_err() {
                    let _ = tx.send(UiEvent::Error(
                        "the agent turn panicked and was recovered".to_owned(),
                    ));
                    ctx.request_repaint();
                }
            }
        }
    }
}

/// Translate a core `AgentLoopEvent` to a `UiEvent`. `User`/`Compaction` are
/// not rendered as bubbles here (the UI already echoes the user's prompt, and
/// compaction is a background detail), so they map to `None`.
fn map_event(event: &AgentLoopEvent) -> Option<UiEvent> {
    match event {
        AgentLoopEvent::Assistant(message) => {
            // The echo backend doesn't stream, so the full assistant text
            // arrives here. (With a streaming backend, deltas arrive first via
            // the StreamingHandler and this is the authoritative final text.)
            if message.content.is_empty() {
                None
            } else {
                Some(UiEvent::Assistant(message.content.clone()))
            }
        }
        AgentLoopEvent::ToolResult {
            tool_call,
            content,
            is_error,
        } => Some(UiEvent::ToolResult {
            tool: tool_call.name.clone(),
            content: content.clone(),
            is_error: *is_error,
        }),
        AgentLoopEvent::ToolBatchSummary {
            label,
            failed_count,
            ..
        } => Some(UiEvent::ToolBatch {
            label: label.clone(),
            failed: *failed_count,
        }),
        AgentLoopEvent::Terminal(terminal) => Some(UiEvent::Terminal(match terminal {
            AgentTerminalEvent::Complete => TerminalKind::Complete,
            AgentTerminalEvent::MaxTurnsReached { cap } => TerminalKind::MaxTurns(*cap),
            AgentTerminalEvent::ModelError(error) => TerminalKind::ModelError(error.clone()),
            AgentTerminalEvent::Blocked => TerminalKind::Blocked,
            AgentTerminalEvent::Interrupted => TerminalKind::Interrupted,
        })),
        AgentLoopEvent::User(_) | AgentLoopEvent::Compaction(_) => None,
    }
}
