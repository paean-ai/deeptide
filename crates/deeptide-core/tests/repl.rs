use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, PermissionManager, PermissionMode,
    PermissionRules, ReplEvent, ReplSession, ToolCall,
};

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
