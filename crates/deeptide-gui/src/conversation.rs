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

use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, bounded, unbounded};
use deeptide_core::{
    AgentEventCallback, AgentLoop, AgentLoopEvent, AgentTerminalEvent, AskOutcome,
    ConversationMessage, DiffPreviewOptions, MessageRole, PermissionAskCallback, SessionStore,
    StreamingEvent, StreamingHandler, ToolCall, new_session_id, render_tool_call_diff,
};
use eframe::egui;

use crate::events::{HydratedBubble, TerminalKind, UiEvent, Usage, WorkerMsg};

/// Pending tool-approval rendezvous: maps a request id to the channel the
/// blocked `ask_callback` is waiting on. The UI inserts a reply via
/// [`Conversation::respond_permission`]; the worker's callback receives it.
type PendingPermissions = Arc<Mutex<HashMap<String, Sender<AskOutcome>>>>;

/// How to start a conversation: optionally resume a session, and optionally
/// override the provider / model the picker selected (empty = use config/env).
#[derive(Debug, Clone, Default)]
pub struct StartConfig {
    pub resume: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
}

/// Handle the UI holds to one running conversation.
pub struct Conversation {
    to_worker: Sender<WorkerMsg>,
    /// Drained by the UI each frame.
    pub from_worker: Receiver<UiEvent>,
    /// Cooperative cancel — flipped from the UI's Stop button; the loop and
    /// every running tool observe it.
    interrupt: Arc<AtomicBool>,
    /// Outstanding approval requests the UI can answer.
    pending: PendingPermissions,
}

impl Conversation {
    /// Spawn a fresh conversation. `ctx` is cloned so the worker can wake the UI
    /// on each event (egui's `Context` is `Send + Sync + Clone`).
    pub fn spawn(ctx: egui::Context) -> Self {
        Self::spawn_inner(ctx, StartConfig::default())
    }

    /// Spawn a conversation per `config` — fresh or resuming a session, with an
    /// optional provider/model override. Resuming loads the prior transcript and
    /// emits a `Hydrate` event so the UI shows the history before continuing.
    pub fn start(ctx: egui::Context, config: StartConfig) -> Self {
        Self::spawn_inner(ctx, config)
    }

    fn spawn_inner(ctx: egui::Context, config: StartConfig) -> Self {
        let (to_worker, worker_rx) = unbounded::<WorkerMsg>();
        let (worker_tx, from_worker) = unbounded::<UiEvent>();
        let interrupt = Arc::new(AtomicBool::new(false));
        let pending: PendingPermissions = Arc::new(Mutex::new(HashMap::new()));
        let interrupt_for_worker = Arc::clone(&interrupt);
        let pending_for_worker = Arc::clone(&pending);
        thread::Builder::new()
            .name("deeptide-conversation".to_owned())
            .spawn(move || {
                worker_loop(
                    worker_rx,
                    worker_tx,
                    ctx,
                    interrupt_for_worker,
                    pending_for_worker,
                    config,
                )
            })
            .expect("spawn conversation worker thread");
        Self {
            to_worker,
            from_worker,
            interrupt,
            pending,
        }
    }

    /// Enqueue a user prompt; output arrives asynchronously via `from_worker`.
    pub fn send_prompt(&self, text: String) {
        let _ = self.to_worker.send(WorkerMsg::Prompt(text));
    }

    /// Answer a pending [`UiEvent::PermissionRequest`]; unblocks the worker's
    /// `ask_callback`. A no-op if the request already timed out / was cancelled.
    pub fn respond_permission(&self, req_id: &str, outcome: AskOutcome) {
        if let Ok(mut map) = self.pending.lock()
            && let Some(sender) = map.remove(req_id)
        {
            let _ = sender.send(outcome);
        }
    }

    /// Request cooperative cancellation of the in-flight turn.
    pub fn interrupt(&self) {
        self.interrupt.store(true, Ordering::Relaxed);
    }
}

/// The worker thread body. Builds the loop once (so message history persists
/// across turns), then blocks on the inbound channel until a prompt arrives or
/// the UI drops the sender (channel closed → loop ends → thread exits).
#[allow(clippy::too_many_arguments)] // worker wiring; each arg is a distinct concern
fn worker_loop(
    rx: Receiver<WorkerMsg>,
    tx: Sender<UiEvent>,
    ctx: egui::Context,
    interrupt: Arc<AtomicBool>,
    pending: PendingPermissions,
    config: StartConfig,
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
    let mut params = deeptide_host::config::resolve_backend_params(&cwd);
    // The picker's selections override config/env for this conversation.
    if let Some(provider) = config.provider.as_deref().filter(|p| !p.is_empty()) {
        params.provider = provider.to_owned();
    }
    if let Some(model) = config.model.as_deref().filter(|m| !m.is_empty()) {
        params.model_override = Some(model.to_owned());
    }
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

    // Interactive approval: emit a PermissionRequest, then BLOCK on a per-request
    // channel until the UI replies (or we're interrupted / it times out). This
    // is the synchronous rendezvous the agent loop expects — it parks the worker
    // thread here exactly like the CLI parks on stdin.
    let tx_perm = tx.clone();
    let ctx_perm = ctx.clone();
    let interrupt_perm = Arc::clone(&interrupt);
    let pending_perm = Arc::clone(&pending);
    let cwd_perm = cwd.clone();
    let ask_cb: PermissionAskCallback = Arc::new(move |tool_call: &ToolCall| -> AskOutcome {
        let req_id = tool_call.id.clone();
        let (decision_tx, decision_rx) = bounded::<AskOutcome>(1);
        if let Ok(mut map) = pending_perm.lock() {
            map.insert(req_id.clone(), decision_tx);
        }
        // For Write/Edit, show the proposed change as a diff instead of raw JSON.
        let diff = render_tool_call_diff(
            &tool_call.name,
            &tool_call.input,
            &cwd_perm,
            DiffPreviewOptions {
                max_lines: 200,
                context_lines: 3,
            },
        )
        .map(|preview| preview.body);
        let _ = tx_perm.send(UiEvent::PermissionRequest {
            req_id: req_id.clone(),
            tool: tool_call.name.clone(),
            preview: permission_preview(tool_call),
            diff,
        });
        ctx_perm.request_repaint();
        // Park until the UI answers; poll so a Stop/cancel can't hang us.
        loop {
            match decision_rx.recv_timeout(Duration::from_millis(150)) {
                Ok(outcome) => return outcome,
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    if interrupt_perm.load(Ordering::Relaxed) {
                        cleanup_pending(&pending_perm, &req_id);
                        return AskOutcome::Deny {
                            reason: String::from("cancelled"),
                        };
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                    return AskOutcome::Deny {
                        reason: String::from("approval channel closed"),
                    };
                }
            }
        }
    });

    let model_name = configured.model.clone();
    let mut agent = AgentLoop::new(configured.backend)
        .with_model(configured.model)
        .with_event_callback(event_cb)
        .with_ask_callback(ask_cb)
        .with_interrupt_flag(Arc::clone(&interrupt));

    // One session id + start time for this conversation. After each turn we
    // persist to the SHARED store (the same JSONL the CLI reads), so a GUI
    // session is `deeptide --resume`-able and vice-versa. Resuming reuses the
    // existing id so we continue the same file.
    let started_at = now_rfc3339();
    let session_id = match config.resume {
        Some(id) => {
            // Load the prior transcript, restore it into the loop, and hand the
            // UI a hydrated history to render before the user continues.
            if let Ok(messages) = SessionStore::load(&cwd, &id) {
                let bubbles = hydrate(&messages);
                agent.restore_messages(messages);
                let _ = tx.send(UiEvent::Hydrate(bubbles));
                ctx.request_repaint();
            }
            id
        }
        None => new_session_id(),
    };

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
                }
                SessionStore::save(
                    &cwd,
                    &session_id,
                    &model_name,
                    &started_at,
                    agent.messages(),
                );
                let summary = agent.cost_tracker().summary();
                let _ = tx.send(UiEvent::Usage(Usage {
                    input: summary.total_input,
                    output: summary.total_output,
                    cache_read: summary.total_cache_read,
                    cost_usd: summary.total_cost_usd,
                }));
                let _ = tx.send(UiEvent::SessionsChanged);
                ctx.request_repaint();
            }
        }
    }
}

/// The current UTC time as an RFC-3339 string — the `started_at` the
/// `SessionStore` meta line records (matches the REPL's format).
fn now_rfc3339() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_default()
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

/// A short, human-readable preview of what a gated tool will do, for the
/// approval prompt — the tool's most relevant argument, truncated.
fn permission_preview(tool_call: &ToolCall) -> String {
    let raw = tool_call.input.to_string();
    const MAX: usize = 200;
    if raw.chars().count() > MAX {
        let head: String = raw.chars().take(MAX).collect();
        format!("{head}…")
    } else {
        raw
    }
}

/// Drop a stale pending entry (e.g. when a request is cancelled before reply).
fn cleanup_pending(pending: &PendingPermissions, req_id: &str) {
    if let Ok(mut map) = pending.lock() {
        map.remove(req_id);
    }
}

/// Rebuild a readable transcript from a restored session: the user/assistant
/// text turns (tool calls/results are not replayed as cards in history).
fn hydrate(messages: &[ConversationMessage]) -> Vec<HydratedBubble> {
    messages
        .iter()
        .filter_map(|message| {
            if message.content.trim().is_empty() {
                return None;
            }
            Some(HydratedBubble {
                is_user: message.role == MessageRole::User,
                text: message.content.clone(),
            })
        })
        .collect()
}
