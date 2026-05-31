//! Integration coverage for follow-up suggestions wired through `ReplSession`:
//! a turn that edits a file surfaces a "Next steps" block, and a bare numeric
//! reply expands to the chosen follow-up prompt.

use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, PermissionMode, ReplEvent, ReplSession, ToolCall,
};

/// First turn writes a file (so the engine sees `edited_files`); every later
/// turn just replies with text.
#[derive(Default)]
struct WriteThenText {
    calls: usize,
}

impl AgentBackend for WriteThenText {
    fn respond(&mut self, _req: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("Creating the file."),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "tw",
                    "Write",
                    serde_json::json!({"file_path": "notes.txt", "content": "hi"}),
                )],
            })
        } else {
            Ok(AgentResponse::text("done"))
        }
    }
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
fn finished_edit_task_suggests_followups_and_numeric_pick_runs_one() {
    let project = tempfile::tempdir().expect("project");
    let mut repl = ReplSession::new(Box::new(WriteThenText::default()))
        .with_cwd(project.path())
        .with_permission_mode(PermissionMode::Bypass);

    // A turn that edits a file should surface follow-up suggestions.
    let out = outputs(repl.submit("create notes.txt"));
    assert!(out.contains("Next steps:"), "expected a suggestion block:\n{out}");
    assert!(
        out.contains("Review the diff") || out.contains("test"),
        "expected verify/review follow-ups for an edit:\n{out}"
    );

    // Pick #1 by number → it expands to that suggestion's prompt and runs as a
    // normal turn, so a matching user message lands in the transcript.
    let _ = repl.submit("1");
    let has_expanded = repl
        .agent_loop()
        .messages()
        .iter()
        .any(|m| m.content.contains("Run the project's tests") || m.content.contains("diff"));
    assert!(
        has_expanded,
        "numeric pick should expand to the suggestion prompt and submit it"
    );
}

#[test]
fn suggest_off_hides_followups_and_disables_numeric_pick() {
    let project = tempfile::tempdir().expect("project");
    let mut repl = ReplSession::new(Box::new(WriteThenText::default()))
        .with_cwd(project.path())
        .with_permission_mode(PermissionMode::Bypass)
        .with_suggestions(false);

    let out = outputs(repl.submit("create notes.txt"));
    assert!(
        !out.contains("Next steps:"),
        "suggestions disabled must not emit a block:\n{out}"
    );

    // With no stored suggestions, a bare "1" is just an ordinary prompt.
    let before = repl.agent_loop().messages().len();
    let _ = repl.submit("1");
    let sent_literal_one = repl.agent_loop().messages()[before..]
        .iter()
        .any(|m| m.content.trim() == "1");
    assert!(sent_literal_one, "a bare number should be a normal prompt when suggestions are off");
}
