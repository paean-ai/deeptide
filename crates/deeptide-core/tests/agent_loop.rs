use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentRequest, AgentResponse, AgentTerminalEvent,
    AgentUsage, MessageRole, PermissionMode, ToolCall,
};
use std::sync::{Arc, Mutex};

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

#[test]
fn agent_loop_executes_agent_tool_with_subagent_backend_factory() {
    let requested_models = Arc::new(Mutex::new(Vec::<String>::new()));
    let requested_models_for_factory = Arc::clone(&requested_models);
    let mut loop_ = AgentLoop::new(Box::new(AgentCallingBackend::default()))
        .with_permission_mode(PermissionMode::Bypass)
        .with_subagent_backend_factory(move |model| {
            requested_models_for_factory
                .lock()
                .expect("model log")
                .push(model.to_owned());
            Box::new(StaticBackend::new("auth flow is handled in api/auth.rs"))
        })
        .with_model("parent-model")
        .with_max_turns(3);

    let events = loop_.run("delegate exploration");

    assert_eq!(
        requested_models.lock().expect("model log").as_slice(),
        ["fast-model"]
    );
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: false,
            } if tool_call.name == "Agent"
                && content.contains("Sub-agent Explore completed with model fast-model.")
                && content.contains("auth flow is handled in api/auth.rs")
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::Assistant(message) if message.content == "done after agent"
        )
    }));
}

#[test]
fn agent_loop_binds_context_state_to_ctx_inspect_tool() {
    let mut loop_ = AgentLoop::new(Box::new(CtxInspectCallingBackend::default()))
        .with_model("deepseek-v4-flash")
        .with_max_turns(3);

    let events = loop_.run("inspect this context please");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: false,
            } if tool_call.name == "CtxInspect"
                && content.contains("Model: deepseek-v4-flash")
                && content.contains("Active messages: 2")
                && content.contains("Estimated usage:")
        )
    }));
}

#[test]
fn agent_loop_snip_tool_trims_active_message_history() {
    let mut loop_ = AgentLoop::new(Box::new(SnipCallingBackend::default())).with_max_turns(3);

    let _ = loop_.run("first");
    let _ = loop_.run("second");
    let events = loop_.run("third");

    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: false,
            } if tool_call.name == "Snip"
                && content.contains("History trim requested: keeping last 2 messages.")
        )
    }));
    assert!(loop_.messages().len() <= 5);
    assert!(
        loop_.messages()[0]
            .content
            .starts_with("[context trimmed by Snip:")
    );
    assert!(loop_.messages().iter().any(|message| {
        message
            .tool_results
            .iter()
            .any(|block| block.tool_use_id == "toolu_snip")
    }));
}

#[test]
fn agent_loop_enter_plan_mode_changes_permission_mode_and_blocks_writes() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut loop_ = AgentLoop::new(Box::new(EnterPlanThenWriteBackend::default()))
        .with_cwd(temp.path())
        .with_max_turns(3);

    let events = loop_.run("plan before writing");

    assert_eq!(loop_.permission_mode(), PermissionMode::Plan);
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: false,
            } if tool_call.name == "EnterPlanMode" && content.contains("Plan mode activated")
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: true,
            } if tool_call.name == "Write" && content.contains("Plan mode")
        )
    }));
    assert!(!temp.path().join("notes.txt").exists());
}

#[test]
fn agent_loop_exit_plan_mode_returns_to_default_permission_mode() {
    let mut loop_ = AgentLoop::new(Box::new(ExitPlanBackend::default()))
        .with_permission_mode(PermissionMode::Plan)
        .with_max_turns(3);

    let events = loop_.run("implementation plan is ready");

    assert_eq!(loop_.permission_mode(), PermissionMode::Default);
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult {
                tool_call,
                content,
                is_error: false,
            } if tool_call.name == "ExitPlanMode"
                && content.contains("Plan is ready for review")
                && content.contains("- Bash: Run cargo test")
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

#[derive(Default)]
struct AgentCallingBackend {
    calls: usize,
}

impl AgentBackend for AgentCallingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("I will delegate exploration."),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_agent",
                    "Agent",
                    serde_json::json!({
                        "description": "Find auth flow",
                        "prompt": "Map the auth flow.",
                        "subagent_type": "Explore",
                        "model": "fast-model",
                    }),
                )],
            })
        } else {
            Ok(AgentResponse::text("done after agent"))
        }
    }
}

#[derive(Default)]
struct CtxInspectCallingBackend {
    calls: usize,
}

impl AgentBackend for CtxInspectCallingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("I will inspect context."),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_ctx",
                    "CtxInspect",
                    serde_json::json!({}),
                )],
            })
        } else {
            Ok(AgentResponse::text("done after context inspection"))
        }
    }
}

#[derive(Default)]
struct SnipCallingBackend {
    calls: usize,
}

impl AgentBackend for SnipCallingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 3 {
            Ok(AgentResponse {
                content: String::from("I will trim history."),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_snip",
                    "Snip",
                    serde_json::json!({"keepLast": 2, "explanation": "Older turns are no longer needed."}),
                )],
            })
        } else {
            Ok(AgentResponse::text(format!("turn {}", self.calls)))
        }
    }
}

#[derive(Default)]
struct EnterPlanThenWriteBackend {
    calls: usize,
}

impl AgentBackend for EnterPlanThenWriteBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("I will enter plan mode and try a write."),
                usage: None,
                tool_calls: vec![
                    ToolCall::new("toolu_plan", "EnterPlanMode", serde_json::json!({})),
                    ToolCall::new(
                        "toolu_write_in_plan",
                        "Write",
                        serde_json::json!({
                            "file_path": "notes.txt",
                            "content": "should not be written",
                        }),
                    ),
                ],
            })
        } else {
            Ok(AgentResponse::text("done after plan"))
        }
    }
}

#[derive(Default)]
struct ExitPlanBackend {
    calls: usize,
}

impl AgentBackend for ExitPlanBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse {
                content: String::from("Plan is ready."),
                usage: None,
                tool_calls: vec![ToolCall::new(
                    "toolu_exit_plan",
                    "ExitPlanMode",
                    serde_json::json!({
                        "allowedPrompts": [
                            {"tool": "Bash", "prompt": "Run cargo test"}
                        ]
                    }),
                )],
            })
        } else {
            Ok(AgentResponse::text("done after exit plan"))
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
