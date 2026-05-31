//! Integration coverage for the per-segment severity policy in
//! [`ReplSession::status_line_with_auth`].
//!
//! The status-bar coloring policy lives in
//! `repl.rs::status_line_with_auth` (mode → severity, ctx percent →
//! severity, queue depth → Info). The CLI consumes the result via
//! [`StatusLine::render_styled`] so the user sees `mode bypass`,
//! `ctx 96%`, etc. light up against the otherwise dim bar.
//!
//! These tests pin the contract by inspecting the SGR opener in the
//! rendered output. That's the actual signal the CLI feeds to the
//! anchored status bar — no internal accessors, no test-only helpers,
//! just the rendered string the user sees.

use deeptide_core::permissions::PermissionMode;
use deeptide_core::{AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplSession};

struct EchoBackend;
impl AgentBackend for EchoBackend {
    fn respond(&mut self, _req: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: String::from("ok"),
            usage: Some(AgentUsage::new(1, 1, 0, 0, 0)),
            tool_calls: Vec::new(),
        })
    }
}

fn render_for_mode(mode: PermissionMode) -> String {
    let mut repl = ReplSession::new(Box::new(EchoBackend));
    repl.set_permission_mode(mode);
    // 120 columns leaves plenty of room so `mode` survives any
    // truncation policy — we're testing severity, not fit.
    repl.status_line().render_styled(120, true)
}

#[test]
fn mode_default_renders_dim_without_alert_sgr() {
    let styled = render_for_mode(PermissionMode::Default);
    assert!(
        styled.contains("\x1b[2mmode default\x1b[0m"),
        "Default mode should render as a single dim block: {styled}"
    );
    // No severity color around the value.
    assert!(!styled.contains("\x1b[1;31m"));
    assert!(!styled.contains("\x1b[33m"));
    assert!(!styled.contains("\x1b[36mdefault"));
}

#[test]
fn mode_plan_carries_info_sgr_on_value() {
    let styled = render_for_mode(PermissionMode::Plan);
    assert!(
        styled.contains("\x1b[36mplan\x1b[0m"),
        "Plan mode should colorize the value in cyan: {styled}"
    );
    // The label stays dim, separately from the value's SGR.
    assert!(
        styled.contains("\x1b[2mmode\x1b[0m"),
        "Label should still render dim: {styled}"
    );
}

#[test]
fn mode_accept_edits_carries_warning_sgr_on_value() {
    let styled = render_for_mode(PermissionMode::AcceptEdits);
    assert!(
        styled.contains("\x1b[33maccept-edits\x1b[0m"),
        "AcceptEdits should colorize the value in yellow: {styled}"
    );
}

#[test]
fn mode_bypass_carries_alert_sgr_on_value() {
    // `PermissionMode::Bypass.label()` is "yolo" — that's the
    // string the status bar puts on screen, so the SGR must wrap
    // it under the same name.
    let styled = render_for_mode(PermissionMode::Bypass);
    assert!(
        styled.contains("\x1b[1;31myolo\x1b[0m"),
        "Bypass (YOLO) should colorize the value in bold red: {styled}"
    );
}

#[test]
fn render_styled_color_off_is_sgr_free_for_pipe_safe_output() {
    let mut repl = ReplSession::new(Box::new(EchoBackend));
    repl.set_permission_mode(PermissionMode::Bypass);
    let plain = repl.status_line().render_styled(120, false);
    assert!(
        !plain.contains('\x1b'),
        "no-color path must be SGR-free, got: {plain:?}"
    );
    assert!(plain.contains("yolo"), "value still present: {plain}");
}

#[test]
fn render_styled_for_bypass_visible_width_equals_request() {
    let mut repl = ReplSession::new(Box::new(EchoBackend));
    repl.set_permission_mode(PermissionMode::Bypass);
    let styled = repl.status_line().render_styled(80, true);
    // Strip SGR sequences so we can assert on the visible columns.
    let mut visible = String::with_capacity(styled.len());
    let mut chars = styled.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for ch in chars.by_ref() {
                if ('@'..='~').contains(&ch) {
                    break;
                }
            }
        } else {
            visible.push(c);
        }
    }
    // ASCII-only output here, so chars==columns is accurate.
    assert_eq!(
        visible.chars().count(),
        80,
        "styled render must pad to requested visible width: {visible:?}"
    );
}
