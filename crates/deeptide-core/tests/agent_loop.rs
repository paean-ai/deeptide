use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentRequest, AgentResponse, AgentTerminalEvent,
    AgentUsage, ContextWindowConfig, HookEngine, HookEntry, MessageRole, PermissionMode,
    SettingsHooks, ToolCall,
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
fn agent_loop_brief_tool_compacts_active_message_history() {
    let mut loop_ = AgentLoop::new(Box::new(BriefCallingBackend::default())).with_max_turns(5);

    let _ = loop_.run("first");
    let _ = loop_.run("second");
    let _ = loop_.run("third");
    let before = loop_.messages().len();
    let events = loop_.run("fourth");

    // The Brief tool result is surfaced to the model.
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult { tool_call, is_error: false, .. }
                if tool_call.name == "Brief"
        )
    }));
    // Older turns were actually folded into a rolling summary (not just claimed).
    assert!(
        loop_.messages()[0].content.starts_with("[context-summary]"),
        "Brief should prepend a context summary, got: {:?}",
        loop_.messages()[0].content
    );
    assert!(
        loop_.messages().len() < before + 3,
        "Brief should reduce the transcript rather than only grow it"
    );
}

#[cfg(unix)]
#[test]
fn agent_loop_pre_tool_use_hook_blocks_tool() {
    let hooks = SettingsHooks {
        pre_tool_use: Some(vec![HookEntry {
            matcher: String::from("*"),
            command: String::from("exit 1"),
            timeout_ms: Some(5_000),
            disabled: None,
            name: Some(String::from("deny-all")),
        }]),
        ..Default::default()
    };
    let engine = HookEngine::new(hooks, std::env::temp_dir());
    let mut loop_ = AgentLoop::new(Box::new(ToolCallingBackend::default()))
        .with_hooks(engine)
        .with_max_turns(2);

    let events = loop_.run("read the file");

    // The Read tool call is vetoed by the PreToolUse hook.
    assert!(events.iter().any(|event| {
        matches!(
            event,
            AgentLoopEvent::ToolResult { tool_call, content, is_error: true }
                if tool_call.name == "Read" && content.contains("Blocked by PreToolUse hook 'deny-all'")
        )
    }));
}

#[cfg(unix)]
#[test]
fn agent_loop_user_prompt_submit_hook_fires_on_run() {
    let dir = tempfile::tempdir().expect("tempdir");
    let marker = dir.path().join("prompt-submitted.txt");
    let hooks = SettingsHooks {
        user_prompt_submit: Some(vec![HookEntry {
            matcher: String::from("*"),
            command: format!("printf '%s' \"$TIDE_EVENT\" > {}", marker.display()),
            timeout_ms: Some(5_000),
            disabled: None,
            name: Some(String::from("record-prompt")),
        }]),
        ..Default::default()
    };
    let engine = HookEngine::new(hooks, dir.path());
    let mut loop_ = AgentLoop::new(Box::new(StaticBackend::new("ok")))
        .with_hooks(engine)
        .with_max_turns(2);

    let _ = loop_.run("hello there");

    let recorded = std::fs::read_to_string(&marker).expect("UserPromptSubmit hook should run");
    assert_eq!(recorded, "UserPromptSubmit");
}

fn tiny_context_window() -> ContextWindowConfig {
    // Soft threshold of 1 token forces compaction almost immediately, while the
    // generous 200-token hard limit keeps these runs off the blocking path.
    ContextWindowConfig {
        max_tokens: 200,
        soft_tokens: 1,
        window_size: 1,
        summary_prefix: String::from("[ctx]"),
    }
}

#[test]
fn agent_loop_auto_compacts_when_transcript_exceeds_threshold() {
    let mut loop_ = AgentLoop::new(Box::new(StaticBackend::new("ok")))
        .with_context_window_config(tiny_context_window())
        .with_max_turns(2);

    // First run seeds history; the second run's pre-request check exceeds the
    // tiny threshold and auto-compacts before assembling the request.
    let _ = loop_.run("first prompt");
    let events = loop_.run("second prompt");

    assert!(
        events.iter().any(|event| matches!(
            event,
            AgentLoopEvent::Compaction(report) if report.did_compress
        )),
        "second run should auto-compact the transcript"
    );
}

#[test]
fn agent_loop_context_window_limit_is_model_aware() {
    // Unknown model falls back to the default budget.
    let loop_ = AgentLoop::new(Box::new(StaticBackend::new("ok"))).with_model("mystery-model-9000");
    assert_eq!(loop_.context_window_limit(), 128_000);

    // Known cloud models resolve to their documented context windows, matching
    // Swift's ModelContextWindow.forModel table.
    let loop_ = AgentLoop::new(Box::new(StaticBackend::new("ok"))).with_model("claude-3-5-sonnet");
    assert_eq!(loop_.context_window_limit(), 200_000);

    let loop_ = AgentLoop::new(Box::new(StaticBackend::new("ok"))).with_model("gemini-1.5-pro");
    assert_eq!(loop_.context_window_limit(), 1_000_000);

    // A live /model switch re-resizes the window.
    let mut loop_ = AgentLoop::new(Box::new(StaticBackend::new("ok"))).with_model("claude-3-opus");
    assert_eq!(loop_.context_window_limit(), 200_000);
    loop_.set_model("deepseek-v3");
    assert_eq!(loop_.context_window_limit(), 128_000);
}

#[test]
fn agent_loop_denylist_blocks_a_restricted_tool() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\n").expect("write fixture");
    let mut loop_ = AgentLoop::new(Box::new(ToolCallingBackend::default()))
        .with_cwd(temp.path())
        .with_tool_restrictions(None, vec![String::from("Read")])
        .with_max_turns(3);

    let events = loop_.run("read notes");

    assert!(
        events.iter().any(|event| matches!(
            event,
            AgentLoopEvent::ToolResult { tool_call, content, is_error: true }
                if tool_call.name == "Read" && content.contains("not available to this agent")
        )),
        "a disallowed tool must be rejected before execution"
    );
}

#[test]
fn agent_loop_allowlist_excludes_unlisted_tools() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\n").expect("write fixture");
    // Only Bash is allowed, so the Read call the backend makes is rejected.
    let mut loop_ = AgentLoop::new(Box::new(ToolCallingBackend::default()))
        .with_cwd(temp.path())
        .with_tool_restrictions(Some(vec![String::from("Bash")]), Vec::new())
        .with_max_turns(3);

    let events = loop_.run("read notes");

    assert!(
        events.iter().any(|event| matches!(
            event,
            AgentLoopEvent::ToolResult { tool_call, content, is_error: true }
                if tool_call.name == "Read" && content.contains("not available to this agent")
        )),
        "a tool outside the allowlist must be rejected"
    );
}

#[test]
fn agent_loop_blocks_when_transcript_exceeds_hard_limit() {
    // window_size is large enough that a single oversized message cannot be
    // compacted away, so the transcript stays over the 98% blocking threshold.
    let config = ContextWindowConfig {
        max_tokens: 10,
        soft_tokens: 1,
        window_size: 8,
        summary_prefix: String::from("[ctx]"),
    };
    let mut loop_ = AgentLoop::new(Box::new(StaticBackend::new("ok")))
        .with_context_window_config(config)
        .with_max_turns(3);

    let huge = "x".repeat(400); // ~100 tokens, far over the 10-token budget
    let events = loop_.run(huge);

    assert!(
        events
            .iter()
            .any(|event| matches!(event, AgentLoopEvent::Terminal(AgentTerminalEvent::Blocked))),
        "an over-limit transcript should terminate with Blocked"
    );
    // The run must stop before reaching a normal completion.
    assert!(!events.iter().any(|event| matches!(
        event,
        AgentLoopEvent::Terminal(AgentTerminalEvent::Complete)
    )));
}

#[test]
fn agent_loop_compacts_on_high_message_count() {
    // A huge token budget means the token threshold never fires; only the
    // message-count trigger can cause compaction.
    let config = ContextWindowConfig {
        max_tokens: 10_000_000,
        soft_tokens: 9_000_000,
        window_size: 4,
        summary_prefix: String::from("[ctx]"),
    };
    let mut loop_ = AgentLoop::new(Box::new(StaticBackend::new("ok")))
        .with_context_window_config(config)
        .with_max_turns(2);

    // Each run appends a user + assistant message; ~120 runs clears 200.
    let mut compacted = false;
    for _ in 0..130 {
        let events = loop_.run("ping");
        if events
            .iter()
            .any(|event| matches!(event, AgentLoopEvent::Compaction(report) if report.did_compress))
        {
            compacted = true;
            break;
        }
    }
    assert!(
        compacted,
        "exceeding the message-count limit should trigger compaction"
    );
}

#[cfg(unix)]
#[test]
fn agent_loop_pre_compact_hook_fires_on_auto_compaction() {
    let dir = tempfile::tempdir().expect("tempdir");
    let marker = dir.path().join("pre-compact.txt");
    let hooks = SettingsHooks {
        pre_compact: Some(vec![HookEntry {
            matcher: String::from("*"),
            command: format!("printf '%s' \"$TIDE_EVENT\" > {}", marker.display()),
            timeout_ms: Some(5_000),
            disabled: None,
            name: Some(String::from("on-compact")),
        }]),
        ..Default::default()
    };
    let engine = HookEngine::new(hooks, dir.path());
    let mut loop_ = AgentLoop::new(Box::new(StaticBackend::new("ok")))
        .with_hooks(engine)
        .with_context_window_config(tiny_context_window())
        .with_max_turns(2);

    let _ = loop_.run("first prompt");
    let _ = loop_.run("second prompt");

    let recorded = std::fs::read_to_string(&marker).expect("PreCompact hook should run");
    assert_eq!(recorded, "PreCompact");
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
struct BriefCallingBackend {
    calls: usize,
}

impl AgentBackend for BriefCallingBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        // After three plain turns (six messages, beyond the default window),
        // request a Brief compaction.
        if self.calls == 4 {
            Ok(AgentResponse {
                content: String::from("Context is getting long; compacting."),
                usage: None,
                tool_calls: vec![ToolCall::new("toolu_brief", "Brief", serde_json::json!({}))],
            })
        } else {
            Ok(AgentResponse::text(format!("turn {}", self.calls)))
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
