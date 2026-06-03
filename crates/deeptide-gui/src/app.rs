//! The egui application: a streaming chat transcript over a [`Conversation`].
//!
//! Immediate-mode: state lives in `App`, and each frame we (1) drain any events
//! the worker produced and fold them into the transcript, then (2) render. The
//! worker calls `ctx.request_repaint()` on every event, so streaming feels live
//! without polling.

use std::path::PathBuf;

use deeptide_core::{AskOutcome, SessionEntry, SessionStore};
use eframe::egui;

use crate::conversation::Conversation;
use crate::events::{TerminalKind, UiEvent};

/// An approval the user still needs to answer.
struct PendingPermission {
    req_id: String,
    tool: String,
    preview: String,
}

/// One rendered item in the transcript.
enum Bubble {
    User(String),
    /// Model reasoning / chain-of-thought (rendered dim, above the answer).
    Thinking(String),
    Assistant(String),
    Tool {
        tool: String,
        content: String,
        is_error: bool,
    },
    Status(String),
}

pub struct App {
    conversation: Conversation,
    transcript: Vec<Bubble>,
    input: String,
    /// True while a turn is in flight (disables send, enables stop).
    running: bool,
    /// Index of the assistant bubble currently being streamed into, if any.
    streaming: Option<usize>,
    /// Index of the reasoning bubble currently being streamed into, if any.
    thinking: Option<usize>,
    /// Tool approvals awaiting the user's decision.
    pending: Vec<PendingPermission>,
    /// Working directory whose sessions the sidebar lists (shared with the CLI).
    cwd: PathBuf,
    /// Past sessions for `cwd`, newest first — refreshed when a turn saves.
    sessions: Vec<SessionEntry>,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let sessions = SessionStore::list(&cwd);
        Self {
            conversation: Conversation::spawn(cc.egui_ctx.clone()),
            transcript: Vec::new(),
            input: String::new(),
            running: false,
            streaming: None,
            thinking: None,
            pending: Vec::new(),
            cwd,
            sessions,
        }
    }

    /// Fold every pending worker event into the transcript.
    fn drain_events(&mut self) {
        while let Ok(event) = self.conversation.from_worker.try_recv() {
            match event {
                UiEvent::AssistantDelta(delta) => {
                    // The answer has started; stop appending to the reasoning bubble.
                    self.thinking = None;
                    let idx = match self.streaming {
                        Some(i) => i,
                        None => {
                            self.transcript.push(Bubble::Assistant(String::new()));
                            let i = self.transcript.len() - 1;
                            self.streaming = Some(i);
                            i
                        }
                    };
                    if let Some(Bubble::Assistant(text)) = self.transcript.get_mut(idx) {
                        text.push_str(&delta);
                    }
                }
                UiEvent::ThinkingDelta(delta) => {
                    let idx = match self.thinking {
                        Some(i) => i,
                        None => {
                            self.transcript.push(Bubble::Thinking(String::new()));
                            let i = self.transcript.len() - 1;
                            self.thinking = Some(i);
                            i
                        }
                    };
                    if let Some(Bubble::Thinking(text)) = self.transcript.get_mut(idx) {
                        text.push_str(&delta);
                    }
                }
                UiEvent::Assistant(full) => {
                    // Authoritative assistant text. If we were streaming deltas,
                    // finalize that bubble; otherwise add a fresh one.
                    match self.streaming.take() {
                        Some(i) => {
                            if let Some(Bubble::Assistant(text)) = self.transcript.get_mut(i) {
                                *text = full;
                            }
                        }
                        None => self.transcript.push(Bubble::Assistant(full)),
                    }
                }
                UiEvent::ToolResult {
                    tool,
                    content,
                    is_error,
                } => self.transcript.push(Bubble::Tool {
                    tool,
                    content,
                    is_error,
                }),
                UiEvent::ToolBatch { label, failed } => {
                    let suffix = if failed > 0 {
                        format!(" ({failed} failed)")
                    } else {
                        String::new()
                    };
                    self.transcript
                        .push(Bubble::Status(format!("{label}{suffix}")));
                }
                UiEvent::PermissionRequest {
                    req_id,
                    tool,
                    preview,
                } => self.pending.push(PendingPermission {
                    req_id,
                    tool,
                    preview,
                }),
                UiEvent::Terminal(kind) => {
                    self.running = false;
                    self.streaming = None;
                    self.thinking = None;
                    // Any approvals still open at end-of-turn are moot (the
                    // worker auto-denies them on cancel/timeout).
                    self.pending.clear();
                    if let Some(note) = terminal_note(&kind) {
                        self.transcript.push(Bubble::Status(note));
                    }
                }
                UiEvent::SessionsChanged => {
                    self.sessions = SessionStore::list(&self.cwd);
                }
                UiEvent::Hydrate(bubbles) => {
                    // A resumed session's history replaces the current transcript.
                    self.transcript = bubbles
                        .into_iter()
                        .map(|bubble| {
                            if bubble.is_user {
                                Bubble::User(bubble.text)
                            } else {
                                Bubble::Assistant(bubble.text)
                            }
                        })
                        .collect();
                }
                UiEvent::Error(message) => {
                    self.running = false;
                    self.streaming = None;
                    self.thinking = None;
                    self.transcript.push(Bubble::Status(format!("⚠ {message}")));
                }
            }
        }
    }

    fn submit(&mut self) {
        let text = self.input.trim().to_owned();
        if text.is_empty() || self.running {
            return;
        }
        self.transcript.push(Bubble::User(text.clone()));
        self.conversation.send_prompt(text);
        self.input.clear();
        self.running = true;
    }

    /// Replace the live conversation with one resumed from `session_id`. The new
    /// worker loads the prior transcript and emits a `Hydrate` event to repopulate
    /// the (now-cleared) UI state.
    fn resume(&mut self, ctx: egui::Context, session_id: String) {
        self.conversation = Conversation::spawn_resume(ctx, session_id);
        self.transcript.clear();
        self.streaming = None;
        self.thinking = None;
        self.pending.clear();
        self.running = false;
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_events();

        egui::Panel::top("header").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Deeptide");
                ui.label(
                    egui::RichText::new("native preview")
                        .small()
                        .color(egui::Color32::GRAY),
                );
            });
        });

        let mut resume_request: Option<String> = None;
        egui::Panel::left("sessions")
            .resizable(true)
            .default_size(220.0)
            .show_inside(ui, |ui| {
                ui.add_space(4.0);
                ui.label(egui::RichText::new("Sessions").strong());
                ui.label(
                    egui::RichText::new("shared with the CLI · click to resume")
                        .small()
                        .color(egui::Color32::GRAY),
                );
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if self.sessions.is_empty() {
                            ui.label(
                                egui::RichText::new("no saved sessions yet")
                                    .italics()
                                    .color(egui::Color32::GRAY),
                            );
                        }
                        for session in &self.sessions {
                            let preview = if session.preview.is_empty() {
                                "(empty)"
                            } else {
                                session.preview.as_str()
                            };
                            ui.add_space(4.0);
                            let clicked = ui
                                .add(
                                    egui::Label::new(egui::RichText::new(preview).strong())
                                        .sense(egui::Sense::click())
                                        .truncate(),
                                )
                                .on_hover_text("Resume this session")
                                .clicked();
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} · {} msgs",
                                    session.model, session.message_count
                                ))
                                .small()
                                .color(egui::Color32::GRAY),
                            );
                            if clicked {
                                resume_request = Some(session.session_id.clone());
                            }
                            ui.separator();
                        }
                    });
            });
        if let Some(session_id) = resume_request {
            self.resume(ui.ctx().clone(), session_id);
        }

        egui::Panel::bottom("composer").show_inside(ui, |ui| {
            ui.add_space(4.0);
            let send_now = ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)
                && ui.memory(|m| m.has_focus(egui::Id::new("prompt")));
            ui.horizontal(|ui| {
                let field = egui::TextEdit::multiline(&mut self.input)
                    .id(egui::Id::new("prompt"))
                    .desired_rows(2)
                    .desired_width(ui.available_width() - 120.0)
                    .hint_text("Ask Deeptide…  (Enter to send, Shift+Enter for newline)");
                ui.add(field);
                ui.vertical(|ui| {
                    let can_send = !self.running && !self.input.trim().is_empty();
                    if ui
                        .add_enabled(can_send, egui::Button::new("Send"))
                        .clicked()
                    {
                        self.submit();
                    }
                    if ui
                        .add_enabled(self.running, egui::Button::new("Stop"))
                        .clicked()
                    {
                        self.conversation.interrupt();
                    }
                });
            });
            ui.add_space(4.0);
            if send_now {
                self.submit();
            }
        });

        // Pending tool approvals sit above the composer until answered.
        if !self.pending.is_empty() {
            let mut decision: Option<(String, AskOutcome)> = None;
            egui::Panel::bottom("approvals").show_inside(ui, |ui| {
                for pending in &self.pending {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(format!("⚠ Allow {} ?", pending.tool))
                            .strong()
                            .color(egui::Color32::YELLOW),
                    );
                    ui.label(
                        egui::RichText::new(&pending.preview)
                            .monospace()
                            .color(egui::Color32::GRAY),
                    );
                    ui.horizontal(|ui| {
                        if ui.button("Allow").clicked() {
                            decision = Some((pending.req_id.clone(), AskOutcome::Allow));
                        }
                        if ui.button(format!("Allow all {}", pending.tool)).clicked() {
                            decision = Some((
                                pending.req_id.clone(),
                                AskOutcome::AllowAllSession {
                                    tool_name: pending.tool.clone(),
                                },
                            ));
                        }
                        if ui.button("Deny").clicked() {
                            decision = Some((
                                pending.req_id.clone(),
                                AskOutcome::Deny {
                                    reason: String::from("denied by user"),
                                },
                            ));
                        }
                    });
                    ui.add_space(6.0);
                }
            });
            if let Some((req_id, outcome)) = decision {
                self.conversation.respond_permission(&req_id, outcome);
                self.pending.retain(|pending| pending.req_id != req_id);
            }
        }

        egui::CentralPanel::default().show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for bubble in &self.transcript {
                        render_bubble(ui, bubble);
                        ui.add_space(8.0);
                    }
                    if self.running {
                        ui.label(egui::RichText::new("…").color(egui::Color32::GRAY));
                    }
                });
        });
    }
}

fn render_bubble(ui: &mut egui::Ui, bubble: &Bubble) {
    match bubble {
        Bubble::User(text) => {
            ui.label(
                egui::RichText::new("You")
                    .strong()
                    .color(egui::Color32::LIGHT_BLUE),
            );
            ui.label(text);
        }
        Bubble::Thinking(text) => {
            egui::CollapsingHeader::new(
                egui::RichText::new("💭 thinking")
                    .italics()
                    .color(egui::Color32::GRAY),
            )
            .id_salt(("thinking", ui.next_auto_id()))
            .default_open(true)
            .show(ui, |ui| {
                ui.label(
                    egui::RichText::new(text)
                        .italics()
                        .color(egui::Color32::GRAY),
                );
            });
        }
        Bubble::Assistant(text) => {
            ui.label(
                egui::RichText::new("Deeptide")
                    .strong()
                    .color(egui::Color32::LIGHT_GREEN),
            );
            // Markdown rendering arrives in a later phase; plain text for now.
            ui.label(text);
        }
        Bubble::Tool {
            tool,
            content,
            is_error,
        } => {
            let header = if *is_error {
                egui::RichText::new(format!("✗ {tool}")).color(egui::Color32::LIGHT_RED)
            } else {
                egui::RichText::new(format!("✓ {tool}")).color(egui::Color32::GRAY)
            };
            egui::CollapsingHeader::new(header)
                .id_salt(("tool", ui.next_auto_id()))
                .show(ui, |ui| {
                    ui.monospace(content);
                });
        }
        Bubble::Status(text) => {
            ui.label(
                egui::RichText::new(text)
                    .italics()
                    .color(egui::Color32::GRAY),
            );
        }
    }
}

fn terminal_note(kind: &TerminalKind) -> Option<String> {
    match kind {
        // A clean completion needs no banner — the assistant text is the result.
        TerminalKind::Complete => None,
        TerminalKind::MaxTurns(cap) => Some(format!("hit the {cap}-turn cap")),
        TerminalKind::ModelError(error) => Some(format!("model error: {error}")),
        TerminalKind::Blocked => Some("context window full".to_owned()),
        TerminalKind::Interrupted => Some("interrupted".to_owned()),
    }
}
