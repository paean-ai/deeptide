use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, PermissionManager, PermissionMode,
    PermissionRules, ReplEvent, ReplSession, ToolCall,
};
use std::sync::{Arc, Mutex};

#[test]
fn repl_routes_plain_input_to_agent_loop() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let events = repl.submit("hello");

    assert_eq!(
        events,
        vec![ReplEvent::Output(String::from("assistant reply"))]
    );
    assert_eq!(repl.agent_loop().messages().len(), 2);
}

#[test]
fn repl_shows_tool_batch_summary_before_tool_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\nbeta\n").expect("write fixture");
    let mut repl = ReplSession::new(Box::new(ReadToolBackend::default())).with_cwd(temp.path());

    let events = repl.submit("read notes");

    assert!(matches!(
        events.first(),
        Some(ReplEvent::Output(output)) if output == "assistant with tool"
    ));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            ReplEvent::Output(output) if output == "Tools completed: Read 1 file in ."
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            ReplEvent::Output(output)
                if output.contains("Tool Read (toolu_read) completed: 1 lines")
                    && output.contains("1\talpha")
        )
    }));
}

#[test]
fn repl_summarizes_long_tool_output() {
    let temp = tempfile::tempdir().expect("tempdir");
    let content = (1..=20)
        .map(|line| format!("line-{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(temp.path().join("notes.txt"), content).expect("write fixture");
    let mut repl = ReplSession::new(Box::new(LongReadToolBackend::default())).with_cwd(temp.path());

    let events = repl.submit("read notes");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            ReplEvent::Output(output)
                if output == "Tool Read (toolu_read) completed: 20 lines (201 B)"
        )
    }));
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            ReplEvent::Output(output) if output.contains("13\tline-13")
        )
    }));
}

#[test]
fn repl_compacts_recoverable_tool_failures() {
    let mut repl = ReplSession::new(Box::new(MissingFileBackend::default()));

    let events = repl.submit("read missing");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            ReplEvent::Output(output)
                if output == "Tool Read (toolu_missing) failed: file not found — use Glob or find to locate it"
        )
    }));
}

#[test]
fn repl_executes_help_command() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let events = repl.submit("/help");

    let output = only_output(events);
    assert!(output.contains("Deeptide commands"));
    assert!(output.contains("/exit"));
    assert!(output.contains("/cost"));
    assert!(output.contains("/read"));
}

#[test]
fn repl_exit_command_requests_exit() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(repl.submit("/exit"), vec![ReplEvent::Exit]);
}

#[test]
fn repl_cost_command_uses_agent_loop_usage() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let _ = repl.submit("hello");

    let output = only_output(repl.submit("/cost"));

    assert!(output.contains("Cost breakdown"));
    assert!(output.contains("Total:"));
    assert!(output.contains("(4 in, 2 out)"));
}

#[test]
fn repl_status_command_reports_session_shape() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut repl = ReplSession::new(Box::new(StaticBackend))
        .with_cwd(temp.path())
        .with_model("deepseek-v4-flash")
        .with_permission_mode(PermissionMode::Plan)
        .with_max_turns(7);
    let _ = repl.submit("hello");

    let output = only_output(repl.submit("/status"));

    assert!(output.contains("Deeptide session status"));
    assert!(output.contains("Model:    deepseek-v4-flash"));
    assert!(output.contains("Branch:   (no git)"));
    assert!(output.contains("Session:  (not persisted)"));
    assert!(output.contains("Turns:    1 / 7"));
    assert!(output.contains("Messages: 2"));
    assert!(output.contains("Context:  ~"));
    assert!(output.contains("Mode:     plan"));
    assert!(output.contains("In/Out:   4 / 2"));
    assert!(output.contains("Cache:    warming"));
    assert!(output.contains("Cost:     $"));
}

#[test]
fn repl_context_command_reports_loaded_context_shape() {
    let temp = tempfile::tempdir().expect("tempdir");
    let agent_dir = temp.path().join(".deeptide").join("agents");
    std::fs::create_dir_all(&agent_dir).expect("agent dir");
    std::fs::write(agent_dir.join("reviewer.md"), "# Reviewer\n").expect("agent definition");

    let mut repl = ReplSession::new(Box::new(StaticBackend))
        .with_cwd(temp.path())
        .with_model("deepseek-v4-flash-q4k")
        .with_permission_mode(PermissionMode::AcceptEdits);
    let _ = repl.submit("hello");

    let output = only_output(repl.submit("/context"));

    assert!(output.contains("Session context"));
    assert!(output.contains(&format!("CWD:      {}", temp.path().display())));
    assert!(output.contains("+ dirs:   (none)"));
    assert!(output.contains("Memory:"));
    assert!(output.contains("Agents:   reviewer"));
    assert!(output.contains("Settings:"));
    assert!(output.contains("runtime  deepseek-v4-flash-q4k"));
    assert!(output.contains("mode     accept-edits"));
    assert!(output.contains("Tools:"));
    assert!(output.contains("Agent"));
    assert!(output.contains("Window:"));
    assert!(output.contains("/ 1,000,000)"));
}

#[test]
fn repl_retry_resubmits_last_user_prompt() {
    let mut repl = ReplSession::new(Box::new(EchoUserBackend));

    assert_eq!(
        repl.submit("please check status"),
        vec![ReplEvent::Output(String::from("echo: please check status"))]
    );

    let retry = repl.submit("/retry");

    assert_eq!(
        retry,
        vec![
            ReplEvent::Output(String::from("Retrying: please check status")),
            ReplEvent::Output(String::from("echo: please check status")),
        ]
    );
    assert_eq!(repl.agent_loop().messages().len(), 4);
}

#[test]
fn repl_retry_reports_missing_prompt() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        repl.submit("/retry"),
        vec![ReplEvent::Output(String::from(
            "No previous prompt to retry."
        ))]
    );
}

#[test]
fn repl_copy_writes_last_assistant_reply() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let copied_for_writer = Arc::clone(&copied);
    let mut repl =
        ReplSession::new(Box::new(StaticBackend)).with_clipboard_writer(move |content| {
            copied_for_writer
                .lock()
                .expect("clipboard capture lock")
                .push(content.to_owned());
            Ok(())
        });

    repl.submit("hello");
    let output = only_output(repl.submit("/copy"));

    assert_eq!(output, "Copied last reply to clipboard (15 chars, 1 line).");
    assert_eq!(
        copied.lock().expect("clipboard capture lock").as_slice(),
        ["assistant reply"]
    );
}

#[test]
fn repl_copy_supports_yank_alias() {
    let copied = Arc::new(Mutex::new(Vec::new()));
    let copied_for_writer = Arc::clone(&copied);
    let mut repl =
        ReplSession::new(Box::new(StaticBackend)).with_clipboard_writer(move |content| {
            copied_for_writer
                .lock()
                .expect("clipboard capture lock")
                .push(content.to_owned());
            Ok(())
        });

    repl.submit("hello");
    let output = only_output(repl.submit("/yank"));

    assert_eq!(output, "Copied last reply to clipboard (15 chars, 1 line).");
    assert_eq!(
        copied.lock().expect("clipboard capture lock").as_slice(),
        ["assistant reply"]
    );
}

#[test]
fn repl_copy_reports_missing_reply() {
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_clipboard_writer(|_| {
        panic!("clipboard writer should not be called without an assistant reply")
    });

    assert_eq!(
        repl.submit("/copy"),
        vec![ReplEvent::Output(String::from(
            "No assistant reply yet to copy."
        ))]
    );
}

#[test]
fn repl_copy_reports_clipboard_errors() {
    let mut repl = ReplSession::new(Box::new(StaticBackend))
        .with_clipboard_writer(|_| Err(String::from("clipboard unavailable")));

    repl.submit("hello");

    assert_eq!(
        repl.submit("/copy"),
        vec![ReplEvent::Output(String::from(
            "/copy: clipboard unavailable"
        ))]
    );
}

#[test]
fn repl_export_writes_session_jsonl() {
    let temp = tempfile::tempdir().expect("tempdir");
    let export_path = temp.path().join("session.jsonl");
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    repl.submit("hello");
    let output = only_output(repl.submit(&format!("/export {}", export_path.display())));

    assert_eq!(
        output,
        format!("Exported 2 messages -> {}", export_path.display())
    );
    let exported = std::fs::read_to_string(export_path).expect("exported transcript");
    let lines = exported.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    let user: serde_json::Value = serde_json::from_str(lines[0]).expect("user json line");
    let assistant: serde_json::Value = serde_json::from_str(lines[1]).expect("assistant json line");
    assert_eq!(user["type"], "user");
    assert_eq!(user["message"]["content"], "hello");
    assert_eq!(assistant["type"], "assistant");
    assert_eq!(assistant["message"]["content"], "assistant reply");
}

#[test]
fn repl_export_rejects_extra_arguments() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        repl.submit("/export one two"),
        vec![ReplEvent::Output(String::from("Usage: /export [path]"))]
    );
}

#[test]
fn repl_diff_reports_empty_workspace_diff() {
    let temp = tempfile::tempdir().expect("tempdir");
    git(temp.path(), ["init"]);
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    assert_eq!(
        repl.submit("/diff"),
        vec![ReplEvent::Output(String::from(
            "No pending git diff in workspace."
        ))]
    );
}

#[test]
fn repl_diff_reports_pending_workspace_diff() {
    let temp = tempfile::tempdir().expect("tempdir");
    git(temp.path(), ["init"]);
    std::fs::write(temp.path().join("notes.txt"), "before\n").expect("write initial file");
    git(temp.path(), ["add", "notes.txt"]);
    std::fs::write(temp.path().join("notes.txt"), "after\n").expect("modify tracked file");
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    let output = only_output(repl.submit("/diff"));

    assert!(output.starts_with("Pending workspace diff:\n"));
    assert!(output.contains("diff --git a/notes.txt b/notes.txt"));
    assert!(output.contains("-before"));
    assert!(output.contains("+after"));
}

#[test]
fn repl_branch_lists_creates_and_switches_branches() {
    let temp = tempfile::tempdir().expect("tempdir");
    git(temp.path(), ["init"]);
    git(
        temp.path(),
        ["config", "user.email", "deeptide@example.invalid"],
    );
    git(temp.path(), ["config", "user.name", "Deeptide Tests"]);
    std::fs::write(temp.path().join("notes.txt"), "hello\n").expect("write initial file");
    git(temp.path(), ["add", "notes.txt"]);
    git(temp.path(), ["commit", "-m", "initial"]);
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    let listed = only_output(repl.submit("/branch"));
    assert!(listed.contains("*"));
    assert!(listed.contains("master") || listed.contains("main"));

    let created = only_output(repl.submit("/branch -b feature/test"));
    assert!(created.contains("feature/test"));
    assert_eq!(
        git_stdout(temp.path(), ["branch", "--show-current"]),
        "feature/test"
    );

    let default_branch = if listed.contains("master") {
        "master"
    } else {
        "main"
    };
    let switched = only_output(repl.submit(&format!("/branch {default_branch}")));
    assert!(switched.contains(default_branch));
    assert_eq!(
        git_stdout(temp.path(), ["branch", "--show-current"]),
        default_branch
    );
}

#[test]
fn repl_branch_rejects_invalid_arguments() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        repl.submit("/branch one two"),
        vec![ReplEvent::Output(String::from(
            "Usage: /branch [name | -b name]"
        ))]
    );
    assert_eq!(
        repl.submit("/branch -b"),
        vec![ReplEvent::Output(String::from(
            "Usage: /branch [name | -b name]"
        ))]
    );
}

#[test]
fn repl_honors_max_turns_setting() {
    let mut repl = ReplSession::new(Box::new(AlwaysToolBackend)).with_max_turns(1);

    let events = repl.submit("keep going");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            ReplEvent::Output(output) if output == "Maximum turns reached."
        )
    }));
    assert_eq!(repl.agent_loop().max_turns(), 1);
}

#[test]
fn repl_clear_resets_agent_loop_state() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let _ = repl.submit("hello");

    let output = only_output(repl.submit("/clear"));

    assert!(output.contains("Conversation cleared."));
    assert!(repl.agent_loop().messages().is_empty());
}

#[test]
fn repl_read_command_reads_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\nbeta\ngamma\n").expect("write fixture");

    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());
    let output = only_output(repl.submit("/read notes.txt --offset 2 --limit 1"));

    assert_eq!(output, "2\tbeta");
}

#[test]
fn repl_write_command_writes_files() {
    let temp = tempfile::tempdir().expect("tempdir");

    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());
    let output = only_output(repl.submit("/write notes.txt hello from repl"));

    assert!(output.contains("Created file: notes.txt"));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("notes.txt")).expect("read written file"),
        "hello from repl"
    );
}

#[test]
fn repl_permission_command_lists_adds_and_removes_rules() {
    let temp = tempfile::tempdir().expect("tempdir");
    let rules_path = temp.path().join("permissions.json");
    let permission_manager = PermissionManager::new(
        PermissionMode::Default,
        PermissionRules::load(Some(rules_path.clone())).expect("rules should load"),
    );
    let mut repl =
        ReplSession::new(Box::new(StaticBackend)).with_permission_manager(permission_manager);

    let empty = only_output(repl.submit("/permission"));
    assert!(empty.contains("Permission rules:"));
    assert!(empty.contains("  allow:\n    (none)"));
    assert!(empty.contains("  deny:\n    (none)"));

    assert_eq!(
        only_output(repl.submit("/permission --allow Bash(cargo test*)")),
        "+allow Bash(cargo test*)"
    );
    assert_eq!(
        only_output(repl.submit("/permission --deny Write:secrets*")),
        "+deny Write:secrets*"
    );
    assert_eq!(
        only_output(repl.submit("/permission --allow npm:*")),
        "+allow npm:*"
    );

    let listed = only_output(repl.submit("/permission"));
    assert!(listed.contains("Bash(cargo test*)"));
    assert!(listed.contains("Write(secrets*)"));
    assert!(listed.contains("npm:*"));

    assert_eq!(
        only_output(repl.submit("/permission --remove cargo test*")),
        "Removed cargo test*"
    );
    let after_remove = only_output(repl.submit("/permission"));
    assert!(!after_remove.contains("Bash(cargo test*)"));
    assert!(after_remove.contains("Write(secrets*)"));

    let stored = std::fs::read_to_string(rules_path).expect("rules should be saved");
    assert!(stored.contains("secrets*"));
    assert!(!stored.contains("cargo test*"));
}

fn only_output(events: Vec<ReplEvent>) -> String {
    match events.as_slice() {
        [ReplEvent::Output(output)] => output.clone(),
        other => panic!("expected one output event, got {other:?}"),
    }
}

fn git<const N: usize>(cwd: &std::path::Path, args: [&str; N]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .status()
        .expect("git command should start");
    assert!(status.success(), "git command should succeed");
}

fn git_stdout<const N: usize>(cwd: &std::path::Path, args: [&str; N]) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .expect("git command should start");
    assert!(output.status.success(), "git command should succeed");
    String::from_utf8(output.stdout)
        .expect("git stdout should be utf8")
        .trim()
        .to_owned()
}

struct StaticBackend;

impl AgentBackend for StaticBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: String::from("assistant reply"),
            usage: Some(AgentUsage::new(4, 2, 0, 0, 10)),
            tool_calls: Vec::new(),
        })
    }
}

struct EchoUserBackend;

impl AgentBackend for EchoUserBackend {
    fn respond(&mut self, request: AgentRequest) -> Result<AgentResponse, String> {
        let prompt = request
            .messages
            .iter()
            .rev()
            .find(|message| matches!(message.role, deeptide_core::MessageRole::User))
            .map(|message| message.content.as_str())
            .unwrap_or_default();
        Ok(AgentResponse::text(format!("echo: {prompt}")))
    }
}

struct AlwaysToolBackend;

impl AgentBackend for AlwaysToolBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: String::from("assistant with tool"),
            usage: None,
            tool_calls: vec![ToolCall::new(
                "toolu_read",
                "Read",
                serde_json::json!({"file_path": "missing.txt"}),
            )],
        })
    }
}

#[derive(Default)]
struct ReadToolBackend {
    calls: usize,
}

#[derive(Default)]
struct LongReadToolBackend {
    calls: usize,
}

impl AgentBackend for LongReadToolBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("assistant with tool"),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_read",
                    "Read",
                    serde_json::json!({"file_path": "notes.txt"}),
                )],
            })
        } else {
            Ok(AgentResponse::text("assistant after tool"))
        }
    }
}

#[derive(Default)]
struct MissingFileBackend {
    calls: usize,
}

impl AgentBackend for MissingFileBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("assistant with missing read"),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_missing",
                    "Read",
                    serde_json::json!({"file_path": "missing.txt"}),
                )],
            })
        } else {
            Ok(AgentResponse::text("assistant after missing read"))
        }
    }
}

impl AgentBackend for ReadToolBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("assistant with tool"),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_read",
                    "Read",
                    serde_json::json!({"file_path": "notes.txt", "limit": 1}),
                )],
            })
        } else {
            Ok(AgentResponse::text("assistant after tool"))
        }
    }
}
