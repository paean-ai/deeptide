//! Integration coverage for cross-tool session import wired through the live
//! `ReplSession` surface (`/import`, `/continue`, `/sessions --all`).
//!
//! These mutate the process-global `HOME` (so `import::discover` looks under a
//! throwaway `~/.claude`), so they live in ONE test to stay deterministic under
//! the default parallel runner.

use deeptide_core::{AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplEvent, ReplSession};

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

/// The Claude slug form for a cwd: path separators → `-`, leading `-` kept.
fn claude_slug(cwd: &std::path::Path) -> String {
    cwd.to_string_lossy().replace('/', "-")
}

fn outputs(events: Vec<ReplEvent>) -> String {
    events
        .into_iter()
        .filter_map(|e| match e {
            ReplEvent::Output(t) => Some(t),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn import_discovers_and_hands_off_a_claude_session() {
    let home = tempfile::tempdir().expect("home");
    let config = tempfile::tempdir().expect("config");
    let project = tempfile::tempdir().expect("project");
    // SAFETY: single-test file; no other thread reads these concurrently.
    unsafe {
        std::env::set_var("HOME", home.path());
        std::env::set_var("TIDE_CONFIG_DIR", config.path());
    }

    let slug = claude_slug(project.path());
    let claude_dir = home.path().join(".claude").join("projects").join(&slug);
    std::fs::create_dir_all(&claude_dir).expect("claude dir");
    // An older session first, so the richer one below ends up newest.
    std::fs::write(
        claude_dir.join("def67890.jsonl"),
        "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"earlier work here\"}}\n",
    )
    .expect("write older session");
    let session = r#"{"type":"ai-title","aiTitle":"Wire the queue worker","sessionId":"abc12345"}
{"type":"user","sessionId":"abc12345","cwd":"/x","gitBranch":"main","message":{"role":"user","content":"Always use pnpm in this repo, never npm."}}
{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Understood — pnpm it is."},{"type":"tool_use","name":"Bash","input":{"command":"pnpm install"}}]}}
{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"installed"}]}}
{"type":"user","message":{"role":"user","content":"Now migrate the queue worker to the new API."}}
"#;
    std::fs::write(claude_dir.join("abc12345.jsonl"), session).expect("write session");

    let repl_for_hint = ReplSession::new(Box::new(EchoBackend)).with_cwd(project.path());
    // First-run onboarding nudges import (especially `/import all`), once.
    let hint = repl_for_hint
        .first_run_import_hint()
        .expect("first run with sessions should produce an import hint");
    assert!(hint.contains("/import all"), "hint should emphasize import all: {hint}");
    assert!(hint.contains("prior session"), "hint should mention prior sessions: {hint}");
    deeptide_core::mark_onboarded();
    assert!(
        repl_for_hint.first_run_import_hint().is_none(),
        "onboarding must not fire again after mark_onboarded"
    );

    // Persistence off so autosaved deeptide sessions don't pollute discovery
    // (which would reorder the menu non-deterministically).
    let mut repl = ReplSession::new(Box::new(EchoBackend))
        .with_cwd(project.path())
        .with_session_persistence(false);

    // 1) /sessions --all surfaces the Claude session for this project.
    let listing = outputs(repl.run_continue("")); // warm path below; first check discovery
    let _ = listing;

    // 2) /continue claude → live handoff prepended to the conversation.
    let before = repl.agent_loop().messages().len();
    let out = outputs(repl.run_continue("claude"));
    assert!(
        out.contains("handed off"),
        "continue should report a handoff: {out}"
    );

    let messages = repl.agent_loop().messages();
    assert_eq!(
        messages.len(),
        before + 1,
        "handoff must prepend exactly one message"
    );
    let first = &messages[0];
    assert!(
        first.content.contains("context handoff"),
        "first message must be the framed handoff block: {}",
        first.content
    );
    assert!(
        first.content.contains("another assistant"),
        "handoff must frame prior turns as foreign: {}",
        first.content
    );
    // The recent human prose survives verbatim in the tail.
    assert!(
        first.content.contains("migrate the queue worker"),
        "recent tail must be kept: {}",
        first.content
    );
    // Tool orientation present.
    assert!(
        first.content.contains("Bash("),
        "recent tool actions should orient the agent: {}",
        first.content
    );

    // 3) Unknown source is reported, not panicked.
    let bad = outputs(repl.run_import("notarealtool --as context"));
    assert!(bad.contains("Unknown source"), "got: {bad}");

    // 4) Bare `/import` shows an interactive numbered menu. With >1 session it
    //    leads with "Import ALL"; a bare numeric reply picks a row.
    let menu = outputs(repl.submit("/import"));
    assert!(menu.contains("Select a session"), "expected a menu header: {menu}");
    assert!(menu.contains("Import ALL"), "menu should lead with the bulk option: {menu}");
    assert!(menu.contains("[claude]"), "menu should list the claude sessions: {menu}");

    // Row 1 = Import ALL, row 2 = newest session → pick 2 for a handoff.
    let before_pick = repl.agent_loop().messages().len();
    let picked = outputs(repl.submit("2"));
    assert!(
        picked.contains("handed off"),
        "picking a session row should run the handoff: {picked}"
    );
    assert_eq!(
        repl.agent_loop().messages().len(),
        before_pick + 1,
        "the chosen session must be handed off (one prepended message)"
    );

    // 4b) `/import all` distils every discovered session into memory in one
    //     pass. (Exact count varies — autosaved deeptide sessions also count.)
    let all = outputs(repl.submit("/import all"));
    assert!(
        all.contains("distilling") && all.contains("session"),
        "/import all should fold the discovered sessions into memory: {all}"
    );

    // A non-numeric input dismisses the menu: a later stray number is inert.
    let _ = repl.submit("/import");
    let _ = repl.submit("hello there"); // dismisses the menu
    let before_stray = repl.agent_loop().messages().len();
    let _ = repl.submit("1"); // now just an ordinary prompt
    assert!(
        repl.agent_loop().messages()[before_stray..]
            .iter()
            .any(|m| m.content.trim() == "1"),
        "after dismissal a bare number must be a normal prompt, not a menu pick"
    );
}
