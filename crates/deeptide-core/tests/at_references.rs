//! Integration tests for the `@file` reference path: end-to-end through
//! `ReplSession::submit` so we verify the attachments notice, the
//! agent-visible prompt text, and the inlined `<file …>` blocks.

// Fixture-heavy tests: the workspace lint `unwrap_used = "deny"` is too
// strict for `tempdir() / fs::write` setup where any failure is a test-
// infra bug. The integration tests for the queue / repl features in
// this crate use the same allow-locally style.
#![allow(clippy::unwrap_used)]

use std::fs;

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, ReplEvent, ReplSession,
};
use std::sync::{Arc, Mutex};
use tempfile::tempdir;

/// Backend that records every prompt it receives, so tests can assert
/// what the agent actually saw after `@`-expansion.
#[derive(Default)]
struct CapturingBackend {
    prompts: Arc<Mutex<Vec<String>>>,
}

impl AgentBackend for CapturingBackend {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String> {
        let prompt = request
            .messages
            .iter()
            .rev()
            .find_map(|m| {
                if matches!(m.role, deeptide_core::MessageRole::User) {
                    Some(m.content.clone())
                } else {
                    None
                }
            })
            .unwrap_or_default();
        if let Ok(mut store) = self.prompts.lock() {
            store.push(prompt);
        }
        Ok(AgentResponse {
            content: String::from("ok"),
            usage: Some(AgentUsage::new(1, 1, 0, 0, 0)),
            tool_calls: Vec::new(),
        })
    }
}

fn make_repl(cwd: &std::path::Path) -> (ReplSession, Arc<Mutex<Vec<String>>>) {
    let backend = CapturingBackend::default();
    let prompts = Arc::clone(&backend.prompts);
    let repl = ReplSession::new(Box::new(backend)).with_cwd(cwd);
    (repl, prompts)
}

fn collect_outputs(events: Vec<ReplEvent>) -> Vec<String> {
    events
        .into_iter()
        .filter_map(|e| match e {
            ReplEvent::Output(s) => Some(s),
            _ => None,
        })
        .collect()
}

#[test]
fn submit_expands_at_reference_into_agent_prompt() {
    let dir = tempdir().expect("tempdir");
    fs::write(dir.path().join("notes.md"), "# the secret is 42").expect("write");

    let (mut repl, prompts) = make_repl(dir.path());
    let _events = repl.submit("explain @notes.md please");

    let prompts = prompts.lock().expect("prompts lock");
    let last = prompts.last().expect("at least one prompt captured");
    assert!(
        last.contains("explain @notes.md please"),
        "original text preserved: {last}"
    );
    assert!(
        last.contains("<file path=\"notes.md\">"),
        "inline block present: {last}"
    );
    assert!(
        last.contains("the secret is 42"),
        "file body inlined: {last}"
    );
}

#[test]
fn submit_emits_attachment_notice_for_inlined_file() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("file.rs"), "fn main() {}").unwrap();
    let (mut repl, _) = make_repl(dir.path());
    let outputs = collect_outputs(repl.submit("see @file.rs"));
    // The first output line is the notice; the second is the agent reply.
    assert!(
        outputs.iter().any(|s| s.starts_with("📎 attached 1 file")),
        "expected notice, got: {outputs:?}"
    );
}

#[test]
fn submit_emits_skip_notice_for_missing_file() {
    let dir = tempdir().unwrap();
    let (mut repl, _) = make_repl(dir.path());
    let outputs = collect_outputs(repl.submit("look at @nope.md"));
    let notice = outputs
        .iter()
        .find(|s| s.contains("skipped"))
        .expect("expected a skip notice");
    assert!(
        notice.contains("@nope.md"),
        "notice references path: {notice}"
    );
    assert!(notice.contains("not found"), "notice has reason: {notice}");
}

#[test]
fn submit_does_not_expand_email_addresses() {
    let dir = tempdir().unwrap();
    let (mut repl, prompts) = make_repl(dir.path());
    let outputs = collect_outputs(repl.submit("ping ryan@example.com please"));
    // No attachments notice should fire.
    assert!(
        !outputs
            .iter()
            .any(|s| s.contains("attached") || s.contains("skipped")),
        "no @-expansion notice for email: {outputs:?}"
    );
    let prompts = prompts.lock().unwrap();
    assert!(!prompts[0].contains("<deeptide-attachments>"));
}

#[test]
fn submit_skips_binary_files_with_notice() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("bin.dat"), b"\x00\x01\x02hello").unwrap();
    let (mut repl, _) = make_repl(dir.path());
    let outputs = collect_outputs(repl.submit("look at @bin.dat"));
    let notice = outputs
        .iter()
        .find(|s| s.contains("skipped"))
        .expect("notice");
    assert!(notice.contains("binary"), "skip notice reason: {notice}");
}

#[test]
fn submit_inlines_multiple_references_in_one_message() {
    let dir = tempdir().unwrap();
    fs::write(dir.path().join("a.txt"), "AAA").unwrap();
    fs::write(dir.path().join("b.txt"), "BBB").unwrap();
    let (mut repl, prompts) = make_repl(dir.path());
    let outputs = collect_outputs(repl.submit("compare @a.txt and @b.txt"));
    assert!(outputs[0].contains("attached 2 files"));
    let captured = prompts.lock().unwrap();
    assert!(captured[0].contains("AAA"));
    assert!(captured[0].contains("BBB"));
}

#[test]
fn submit_does_not_modify_prompt_when_no_references() {
    let dir = tempdir().unwrap();
    let (mut repl, prompts) = make_repl(dir.path());
    let _ = repl.submit("just a plain question");
    let captured = prompts.lock().unwrap();
    assert_eq!(captured[0], "just a plain question");
}

#[test]
fn tools_command_lists_registered_tools() {
    let dir = tempdir().unwrap();
    let (mut repl, _) = make_repl(dir.path());
    let out = collect_outputs(repl.submit("/tools"));
    let body = out.join("\n");
    assert!(body.starts_with("Tools ("), "header: {body}");
    // Spot-check a handful of canonical tools.
    for must in &["Read", "Write", "Bash", "Grep", "AppendFile"] {
        assert!(body.contains(must), "missing tool {must}: {body}");
    }
}

/// Pull just the tool-name column out of a `/tools` listing so tests
/// can assert "name X is/isn't visible" without false matches from the
/// description column. Lines that start with `  · ` or `  ✎ ` are tool
/// rows; the second whitespace-separated token after the marker is
/// the name.
fn tool_names_in(body: &str) -> Vec<&str> {
    body.lines()
        .filter_map(|line| {
            let line = line.trim_start();
            let after = line
                .strip_prefix("· ")
                .or_else(|| line.strip_prefix("✎ "))?;
            after.split_whitespace().next()
        })
        .collect()
}

#[test]
fn tools_command_filters_by_substring() {
    let dir = tempdir().unwrap();
    let (mut repl, _) = make_repl(dir.path());
    let out = collect_outputs(repl.submit("/tools bash"));
    let body = out.join("\n");
    assert!(body.contains("matching `bash`"), "header label: {body}");

    let names = tool_names_in(&body);
    assert!(names.contains(&"Bash"), "Bash missing: {names:?}");
    // Filter is `bash`; tool NAMES other than the `*Bash*` family must
    // not appear in the listing. (Tool descriptions may legitimately
    // mention the word "read" elsewhere — those are not matches.)
    assert!(
        !names.contains(&"Read"),
        "Read leaked through filter: {names:?}"
    );
    assert!(
        !names.contains(&"Write"),
        "Write leaked through filter: {names:?}"
    );
}

#[test]
fn tools_command_partitions_read_only_vs_writes() {
    let dir = tempdir().unwrap();
    let (mut repl, _) = make_repl(dir.path());

    let ro_body = collect_outputs(repl.submit("/tools --read-only")).join("\n");
    assert!(ro_body.contains("read-only"));
    let ro_names = tool_names_in(&ro_body);
    assert!(
        ro_names.contains(&"Read"),
        "Read missing from read-only: {ro_names:?}"
    );
    assert!(
        !ro_names.contains(&"Write"),
        "Write must not appear in read-only set: {ro_names:?}"
    );

    let wr_body = collect_outputs(repl.submit("/tools --writes")).join("\n");
    let wr_names = tool_names_in(&wr_body);
    assert!(wr_names.contains(&"Write"), "Write missing: {wr_names:?}");
    assert!(
        !wr_names.contains(&"Read"),
        "Read leaked into writes set: {wr_names:?}"
    );
}

#[test]
fn tools_command_help_flag_renders_usage() {
    let dir = tempdir().unwrap();
    let (mut repl, _) = make_repl(dir.path());
    let body = collect_outputs(repl.submit("/tools --help")).join("\n");
    assert!(body.starts_with("Usage:"));
    assert!(body.contains("--read-only"));
    assert!(body.contains("--writes"));
}

#[test]
fn tools_command_rejects_unknown_flag() {
    let dir = tempdir().unwrap();
    let (mut repl, _) = make_repl(dir.path());
    let body = collect_outputs(repl.submit("/tools --banana")).join("\n");
    assert!(body.contains("Unknown flag"));
    assert!(body.contains("--banana"));
}
