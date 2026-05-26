use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentRequest, AgentResponse, AgentTerminalEvent,
    AgentUsage, MessageRole, PermissionMode, ToolCall,
};

#[test]
fn agent_loop_appends_user_and_assistant_messages() {
    let mut loop_ = AgentLoop::new(Box::new(StaticBackend::new("hello back")));

    let events = loop_.run("hello");

    assert!(matches!(events[0], AgentLoopEvent::User(_)));
    assert!(matches!(events[1], AgentLoopEvent::Assistant(_)));
    assert_eq!(
        events.last(),
        Some(&AgentLoopEvent::Terminal(AgentTerminalEvent::Complete))
    );
    assert_eq!(loop_.messages().len(), 2);
    assert_eq!(loop_.messages()[0].role, MessageRole::User);
    assert_eq!(loop_.messages()[1].content, "hello back");
}

#[test]
fn agent_loop_records_usage_in_cost_tracker() {
    let mut loop_ = AgentLoop::new(Box::new(
        StaticBackend::new("hello back").with_usage(AgentUsage::new(10, 5, 2, 8, 123)),
    ))
    .with_model("deepseek-v4-pro");

    let _ = loop_.run("hello");
    let summary = loop_.cost_tracker().summary();

    assert_eq!(summary.turns.len(), 1);
    assert_eq!(summary.total_input, 10);
    assert_eq!(summary.total_output, 5);
    assert_eq!(summary.total_cache_create, 2);
    assert_eq!(summary.total_cache_read, 8);
    assert_eq!(summary.turns[0].duration_ms, 123);
    assert_eq!(summary.turns[0].model, "deepseek-v4-pro");
}

#[test]
fn agent_loop_reports_backend_errors_without_adding_assistant_message() {
    let mut loop_ = AgentLoop::new(Box::new(FailingBackend));

    let events = loop_.run("hello");

    assert_eq!(
        events.last(),
        Some(&AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(
            "backend failed".to_owned()
        )))
    );
    assert_eq!(loop_.messages().len(), 1);
}

#[test]
fn agent_loop_executes_tool_calls_and_continues() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\nbeta\n").expect("write fixture");
    let mut loop_ = AgentLoop::new(Box::new(ToolCallingBackend::default()))
        .with_cwd(temp.path())
        .with_max_turns(3);

    let events = loop_.run("read notes");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolBatchSummary {
                label,
                failed_count: 0,
                ..
            } if label == "Read 1 file in ."
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: false,
            } if tool_call.name == "Read" && content.contains("1\talpha")
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::Assistant(message) if message.content == "done after tool"
        )
    }));
    assert_eq!(
        events.last(),
        Some(&AgentLoopEvent::Terminal(AgentTerminalEvent::Complete))
    );
}

#[test]
fn agent_loop_blocks_write_tool_calls_without_edit_permission() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut loop_ = AgentLoop::new(Box::new(WriteCallingBackend::default()))
        .with_cwd(temp.path())
        .with_max_turns(3);

    let events = loop_.run("write notes");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: true,
            } if tool_call.name == "Write" && content.contains("Permission required")
        )
    }));
    assert!(!temp.path().join("notes.txt").exists());
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolBatchSummary {
                label,
                failed_count: 1,
                ..
            } if label == "Wrote 1 write, 1 failed"
        )
    }));
}

#[test]
fn agent_loop_allows_write_tool_calls_in_accept_edits_mode() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut loop_ = AgentLoop::new(Box::new(WriteCallingBackend::default()))
        .with_cwd(temp.path())
        .with_permission_mode(PermissionMode::AcceptEdits)
        .with_max_turns(3);

    let events = loop_.run("write notes");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: false,
            } if tool_call.name == "Write" && content.contains("Created file: notes.txt")
        )
    }));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("notes.txt")).expect("written file"),
        "hello from agent"
    );
}

#[test]
fn agent_loop_allows_edit_tool_calls_in_accept_edits_mode() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\n").expect("write fixture");
    let mut loop_ = AgentLoop::new(Box::new(EditCallingBackend::default()))
        .with_cwd(temp.path())
        .with_permission_mode(PermissionMode::AcceptEdits)
        .with_max_turns(3);

    let events = loop_.run("edit notes");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: false,
            } if tool_call.name == "Edit" && content.contains("File edited successfully")
        )
    }));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("notes.txt")).expect("edited file"),
        "bravo\n"
    );
}

#[test]
fn agent_loop_blocks_bash_tool_calls_without_permission() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut loop_ = AgentLoop::new(Box::new(BashCallingBackend::default()))
        .with_cwd(temp.path())
        .with_max_turns(3);

    let events = loop_.run("run shell");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: true,
            } if tool_call.name == "Bash" && content.contains("Permission required")
        )
    }));
}

#[test]
fn agent_loop_allows_bash_tool_calls_in_bypass_mode() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut loop_ = AgentLoop::new(Box::new(BashCallingBackend::default()))
        .with_cwd(temp.path())
        .with_permission_mode(PermissionMode::Bypass)
        .with_max_turns(3);

    let events = loop_.run("run shell");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: false,
            } if tool_call.name == "Bash" && content.contains("shell-ok")
        )
    }));
}

struct StaticBackend {
    content: String,
    usage: Option<AgentUsage>,
}

impl StaticBackend {
    fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            usage: None,
        }
    }

    fn with_usage(mut self, usage: AgentUsage) -> Self {
        self.usage = Some(usage);
        self
    }
}

impl AgentBackend for StaticBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse {
            content: self.content.clone(),
            usage: self.usage.clone(),
            tool_calls: Vec::new(),
        })
    }
}

struct FailingBackend;

impl AgentBackend for FailingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Err(String::from("backend failed"))
    }
}

#[derive(Default)]
struct ToolCallingBackend {
    calls: usize,
}

impl AgentBackend for ToolCallingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("I will read the file."),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_1",
                    "Read",
                    serde_json::json!({"file_path": "notes.txt", "limit": 1}),
                )],
            })
        } else {
            Ok(AgentResponse::text("done after tool"))
        }
    }
}

#[derive(Default)]
struct WriteCallingBackend {
    calls: usize,
}

impl AgentBackend for WriteCallingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("I will write the file."),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_write",
                    "Write",
                    serde_json::json!({
                        "file_path": "notes.txt",
                        "content": "hello from agent",
                    }),
                )],
            })
        } else {
            Ok(AgentResponse::text("done after write"))
        }
    }
}

#[derive(Default)]
struct EditCallingBackend {
    calls: usize,
}

impl AgentBackend for EditCallingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("I will edit the file."),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_edit",
                    "Edit",
                    serde_json::json!({
                        "file_path": "notes.txt",
                        "old_string": "alpha",
                        "new_string": "bravo",
                    }),
                )],
            })
        } else {
            Ok(AgentResponse::text("done after edit"))
        }
    }
}

#[derive(Default)]
struct BashCallingBackend {
    calls: usize,
}

impl AgentBackend for BashCallingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("I will run a shell command."),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_bash",
                    "Bash",
                    serde_json::json!({
                        "command": echo_ok_command(),
                    }),
                )],
            })
        } else {
            Ok(AgentResponse::text("done after bash"))
        }
    }
}

#[cfg(windows)]
fn echo_ok_command() -> &'static str {
    "echo shell-ok"
}

#[cfg(not(windows))]
fn echo_ok_command() -> &'static str {
    "printf shell-ok"
}
