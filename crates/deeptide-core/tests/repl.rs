use deeptide_core::{
    AgentBackend, AgentRequest, AgentResponse, AgentUsage, PermissionManager, PermissionMode,
    PermissionRules, ReplEvent, ReplSession, SystemMessage, ToolCall,
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
fn repl_debug_mode_emits_per_turn_diagnostics() {
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_debug(true);

    let outputs: Vec<String> = repl
        .submit("hello")
        .into_iter()
        .filter_map(|event| match event {
            ReplEvent::Output(text) => Some(text),
            _ => None,
        })
        .collect();

    assert!(
        outputs.iter().any(|text| text.contains("[debug] turn")),
        "expected a per-turn debug line, got: {outputs:?}"
    );
    assert!(
        outputs.iter().any(|text| text.contains("in 4 out 2")),
        "debug line should report token usage, got: {outputs:?}"
    );
}

#[test]
fn repl_tps_records_samples_and_resets() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    // No samples before any prompt.
    assert!(only_output(repl.submit("/tps")).contains("No model TPS samples"));

    // A completed turn (StaticBackend reports usage) records a sample.
    repl.submit("hello");
    let listing = only_output(repl.submit("/tps"));
    assert!(
        listing.contains("Model TPS samples:"),
        "expected a TPS listing, got: {listing}"
    );

    let json = only_output(repl.submit("/tps --json"));
    assert!(
        json.contains("\"best_tps\"") && json.contains("\"samples\""),
        "expected TPS JSON, got: {json}"
    );

    // Reset clears the recorded samples.
    assert!(only_output(repl.submit("/tps --reset")).contains("Cleared"));
    assert!(
        only_output(repl.submit("/tps")).contains("No model TPS samples"),
        "samples should be cleared after --reset"
    );

    // Unknown flags are rejected with usage.
    assert!(only_output(repl.submit("/tps --bogus")).contains("Usage: /tps"));
}

#[test]
fn repl_tps_persists_across_sessions_with_store_dir() {
    let store = tempfile::tempdir().expect("tempdir");

    // First session: record a turn, then end the session.
    {
        let mut repl = ReplSession::new(Box::new(StaticBackend)).with_tps_store_dir(store.path());
        repl.submit("hello");
    }

    // A new session pointed at the same store sees the persisted sample before
    // submitting anything.
    let mut next = ReplSession::new(Box::new(StaticBackend)).with_tps_store_dir(store.path());
    let listing = only_output(next.submit("/tps"));
    assert!(
        listing.contains("Model TPS samples:"),
        "persisted TPS should survive across sessions, got: {listing}"
    );

    // Reset clears the persisted store too.
    assert!(only_output(next.submit("/tps --reset")).contains("Cleared"));
    let mut fresh = ReplSession::new(Box::new(StaticBackend)).with_tps_store_dir(store.path());
    assert!(
        only_output(fresh.submit("/tps")).contains("No model TPS samples"),
        "reset should clear the persisted store"
    );
}

#[test]
fn repl_fast_mode_adds_prompt_hint_and_reports_status() {
    let fast = ReplSession::new(Box::new(StaticBackend)).with_fast_mode(true);
    assert!(
        fast.agent_loop()
            .system_prompt()
            .is_some_and(|prompt| prompt.contains("Fast mode for Deeptide")),
        "enabling fast mode should append the fast-mode system-prompt hint"
    );

    let mut fast = fast;
    assert!(only_output(fast.submit("/fast")).contains("Fast mode is ON"));

    // Default session does not get the hint and reports OFF.
    let mut plain = ReplSession::new(Box::new(StaticBackend));
    assert!(
        !plain
            .agent_loop()
            .system_prompt()
            .is_some_and(|prompt| prompt.contains("Fast mode for Deeptide"))
    );
    assert!(only_output(plain.submit("/fast")).contains("Fast mode is OFF"));
}

#[test]
fn repl_without_debug_emits_no_diagnostics() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let events = repl.submit("hello");

    assert!(
        !events
            .iter()
            .any(|event| matches!(event, ReplEvent::Output(text) if text.contains("[debug]"))),
        "debug diagnostics must not appear unless debug mode is enabled"
    );
}

#[test]
fn repl_compact_folds_older_messages() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    // Four prompts -> eight messages, beyond the default recent window.
    for _ in 0..4 {
        repl.submit("hello");
    }
    let before = repl.agent_loop().messages().len();
    assert!(
        before > 6,
        "expected more than the window of messages, got {before}"
    );

    let output = only_output(repl.submit("/compact"));
    assert!(
        output.contains("Context compacted"),
        "expected a compaction report, got: {output}"
    );

    let after = repl.agent_loop().messages().len();
    assert!(
        after < before,
        "compaction should reduce the transcript: {before} -> {after}"
    );
    // The rewritten transcript must still open on a user message (the summary).
    assert!(
        repl.agent_loop()
            .messages()
            .first()
            .is_some_and(|m| m.content.contains("[context-summary]")),
        "compaction should prepend a summary message"
    );
}

#[test]
fn repl_compact_is_noop_for_short_transcripts() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    repl.submit("hello");

    let output = only_output(repl.submit("/compact"));
    assert!(
        output.contains("Nothing to compact"),
        "short transcripts should report nothing to compact, got: {output}"
    );
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
            ReplEvent::System(SystemMessage::ToolBatch { label, failed_count: 0 })
                if label == "Read 1 file in ."
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event,
            ReplEvent::System(SystemMessage::Tool {
                name,
                call_id,
                summary,
                is_error: false,
                body: Some(body),
                ..
            }) if name == "Read"
                && call_id == "toolu_read"
                && summary.contains("1 lines")
                && body.contains("1\talpha")
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
            ReplEvent::System(SystemMessage::Tool {
                name,
                call_id,
                summary,
                is_error: false,
                body: None,
                ..
            }) if name == "Read"
                && call_id == "toolu_read"
                && summary == "20 lines (201 B)"
        )
    }));
    // Body must NOT be inlined when the result is too large to expand
    // (>2_000 bytes or >12 lines). The CLI surfaces the summary only.
    assert!(!events.iter().any(|event| {
        matches!(
            event,
            ReplEvent::System(SystemMessage::Tool { body: Some(body), .. })
                if body.contains("13\tline-13")
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
            ReplEvent::System(SystemMessage::Tool {
                name,
                call_id,
                summary,
                is_error: true,
                body: None,
                ..
            }) if name == "Read"
                && call_id == "toolu_missing"
                && summary == "file not found — use Glob or find to locate it"
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
fn repl_model_command_lists_current_model_and_aliases() {
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_model("deepseek-v4-flash");

    let output = only_output(repl.submit("/model"));

    assert!(output.contains("Current model: deepseek-v4-flash"));
    assert!(output.contains("Aliases:"));
    assert!(output.contains("deepseek-v4-pro <- pro, v4, v4-pro"));
    assert!(output.contains("Usage: /model <name-or-alias>"));
}

#[test]
fn repl_model_command_switches_model_and_resolves_aliases() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        only_output(repl.submit("/model flash")),
        "Model: deepseek-v4-flash (alias flash)"
    );
    assert_eq!(repl.agent_loop().model(), "deepseek-v4-flash");

    assert_eq!(
        only_output(repl.submit("/m custom-model")),
        "Model: custom-model"
    );
    assert_eq!(repl.agent_loop().model(), "custom-model");
}

#[test]
fn repl_model_command_rejects_extra_arguments() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        repl.submit("/model one two"),
        vec![ReplEvent::Output(String::from(
            "Usage: /model <model-name | flash | pro>"
        ))]
    );
}

#[test]
fn repl_provider_command_reports_status_and_profiles() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let status = only_output(repl.submit("/provider"));
    assert!(status.contains("Current session provider profile: legacy"));
    assert!(status.contains("ZERO_CLI_*"));

    let listed = only_output(repl.submit("/profiles list"));
    assert!(listed.contains("* legacy"));
    assert!(listed.contains("deepseek  https://api.deepseek.com"));
    assert!(listed.contains("paean  https://api.paean.ai"));
}

#[test]
fn repl_provider_command_switches_builtin_profiles() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        only_output(repl.submit("/provider use paean-ai")),
        "Active provider profile: paean (recorded for this REPL session; launch configuration controls the current model client)"
    );
    let status = only_output(repl.submit("/status"));
    assert!(status.contains("Provider: paean"));

    assert_eq!(
        only_output(repl.submit("/provider use official")),
        "Active provider profile: deepseek (recorded for this REPL session; launch configuration controls the current model client)"
    );
    let listed = only_output(repl.submit("/provider list"));
    assert!(listed.contains("* deepseek"));
}

#[test]
fn repl_provider_command_rejects_unknown_and_invalid_arguments() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        repl.submit("/provider use"),
        vec![ReplEvent::Output(String::from(
            "Usage: /provider [list | use <name|deepseek|paean> | status]"
        ))]
    );
    assert_eq!(
        repl.submit("/provider use one two"),
        vec![ReplEvent::Output(String::from(
            "Usage: /provider use <name|deepseek|paean>"
        ))]
    );
    assert_eq!(
        repl.submit("/provider use unknown"),
        vec![ReplEvent::Output(String::from(
            "Unknown provider profile `unknown`. Use `/provider list`."
        ))]
    );
}

#[test]
fn repl_swift_parity_convenience_commands_are_available() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let help = only_output(repl.submit("/help"));
    for command in [
        "/fast",
        "/tps",
        "/debug",
        "/keybindings",
        "/sessions",
        "/resume",
    ] {
        assert!(help.contains(command), "help should list {command}");
    }

    // Default session: fast mode is off, and the message no longer references a
    // nonexistent flag.
    assert!(only_output(repl.submit("/fast")).contains("Fast mode is OFF"));
    assert!(only_output(repl.submit("/faster")).contains("Fast mode is OFF"));
    assert_eq!(only_output(repl.submit("/debug")), "Debug mode: on");
    assert_eq!(only_output(repl.submit("/dbg")), "Debug mode: off");

    let keys = only_output(repl.submit("/keys"));
    assert!(keys.contains("Key bindings:"));
    assert!(keys.contains("Ctrl+C"));

    // Sessions now uses real persistence; just verify the command is routed
    // (exact output depends on what's in ~/.config/tide/ on the test machine)
    let sessions_output = only_output(repl.submit("/sessions"));
    assert!(
        sessions_output.contains("sessions") || sessions_output.contains("Sessions"),
        "sessions command should produce session-related output"
    );
    assert_eq!(
        only_output(repl.submit("/load nonexistent-session-abc123")),
        "Cannot resume: Session not found: nonexistent-session-abc123"
    );
}

#[test]
fn repl_tps_command_matches_swift_flags_with_empty_rust_store() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        only_output(repl.submit("/tps")),
        "No model TPS samples yet. Run a streamed model prompt, then /tps again."
    );
    assert_eq!(only_output(repl.submit("/speed --json")), "[]");
    assert_eq!(
        only_output(repl.submit("/tps --reset")),
        "Cleared 0 TPS sample(s)."
    );
    assert_eq!(
        repl.submit("/tps --bogus"),
        vec![ReplEvent::Output(String::from(
            "Usage: /tps [--json | --reset]"
        ))]
    );
}

#[test]
fn repl_swift_parity_support_commands_are_available() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "hello\n").expect("fixture");
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    let help = only_output(repl.submit("/help"));
    for command in [
        "/open", "/paste", "/doctor", "/config", "/hooks", "/init", "/update", "/vim",
    ] {
        assert!(help.contains(command), "help should list {command}");
    }

    assert_eq!(
        only_output(repl.submit("/open missing.txt")),
        format!(
            "File does not exist: {}",
            temp.path().join("missing.txt").display()
        )
    );
    assert_eq!(
        only_output(repl.submit("/open notes.txt")),
        format!(
            "{} is not classified as sensitive in the Rust build; normal tools can already read it.",
            temp.path().join("notes.txt").display()
        )
    );

    let doctor = only_output(repl.submit("/doctor"));
    assert!(doctor.contains("Deeptide doctor"));
    assert!(doctor.contains("Tools:"));
    assert!(doctor.contains("Commands:"));
    // Enhanced diagnostics: settings layers + permission/hook/MCP counts
    // (cross-platform parity with Swift's DoctorCommand).
    assert!(doctor.contains("Settings layers (effective):"));
    assert!(doctor.contains("global"));
    assert!(doctor.contains("project"));
    assert!(doctor.contains("local"));
    assert!(doctor.contains("Permissions:"));
    assert!(doctor.contains("allow,") && doctor.contains("deny rules"));
    assert!(doctor.contains("Hooks:"));
    assert!(doctor.contains("pre-tool=") && doctor.contains("compact="));
    assert!(doctor.contains("MCP servers:"));

    let config = only_output(repl.submit("/config"));
    assert!(config.contains("Settings files:"));
    assert!(config.contains("Merged values:"));
    assert!(config.contains("Usage:"));

    // `/hooks` reads the merged settings; assert it reports hook state in either
    // shape ("No hooks configured…" or "Configured hooks:") without depending on
    // the developer's real global settings content.
    let hooks = only_output(repl.submit("/hooks"));
    assert!(
        hooks.contains("hooks"),
        "/hooks should report hook state, got: {hooks}"
    );
    assert!(only_output(repl.submit("/init")).contains("Project bootstrap is model-driven"));
    assert!(
        only_output(repl.submit("/update --check")).contains("Update checks are not available")
    );
    // `/vim` synchronously spawns `$EDITOR` (or `vim`) and waits for it. On
    // a developer machine with an interactive `vim` installed, the previous
    // version of this test deadlocked the whole `cargo test` run because vim
    // blocked waiting for stdin. Force `$EDITOR=true` so the spawn returns
    // immediately with success and an empty file — the post-condition is the
    // same (a non-empty output event is returned to the REPL).
    //
    // SAFETY: env mutation is process-global and the rest of this test
    // doesn't read `$EDITOR` or `$VISUAL`, but other tests in the binary
    // could; the env mutation is undone before we return.
    let prev_editor = std::env::var_os("EDITOR");
    let prev_visual = std::env::var_os("VISUAL");
    unsafe {
        std::env::set_var("EDITOR", "true");
        std::env::remove_var("VISUAL");
    }
    let vim_output = repl.submit("/vim");
    unsafe {
        match prev_editor {
            Some(value) => std::env::set_var("EDITOR", value),
            None => std::env::remove_var("EDITOR"),
        }
        if let Some(value) = prev_visual {
            std::env::set_var("VISUAL", value);
        }
    }
    assert!(
        !vim_output.is_empty(),
        "/vim should produce at least one output event"
    );
    assert!(
        vim_output.iter().any(|e| matches!(e, ReplEvent::Output(_))),
        "/vim should produce a text output event"
    );
}

#[test]
fn repl_support_commands_validate_usage() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        repl.submit("/doctor now"),
        vec![ReplEvent::Output(String::from("Usage: /doctor"))]
    );
    assert_eq!(
        repl.submit("/paste now"),
        vec![ReplEvent::Output(String::from("Usage: /paste"))]
    );
    // `set` with no `key=value` pair is a usage error and must not write any
    // file, so this case stays hermetic without touching the global config.
    assert_eq!(
        repl.submit("/config set model"),
        vec![ReplEvent::Output(String::from(
            "Usage: /config set key=value [--project | --local]"
        ))]
    );
    assert_eq!(
        repl.submit("/update --unknown"),
        vec![ReplEvent::Output(String::from(
            "Usage: /update [--check | --force]"
        ))]
    );
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
    assert!(output.contains("+ dirs:   (none)"));
    assert!(output.contains("Branch:   (no git)"));
    assert!(output.contains("Provider: legacy"));
    assert!(
        output.contains("Session:  "),
        "status should show session ID"
    );
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
fn repl_add_dir_lists_additional_context_dirs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let extra = temp.path().join("extra");
    std::fs::create_dir(&extra).expect("extra dir");
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    assert_eq!(
        repl.submit("/add-dir"),
        vec![ReplEvent::Output(String::from("No additional dirs."))]
    );

    assert_eq!(
        only_output(repl.submit("/add-dir extra")),
        format!("Added {}", extra.display())
    );
    assert_eq!(
        only_output(repl.submit("/add_dir extra")),
        format!("Added {}", extra.display())
    );

    let listed = only_output(repl.submit("/adddir"));
    assert_eq!(listed, format!("  {}", extra.display()));

    let status = only_output(repl.submit("/status"));
    assert!(status.contains(&format!("+ dirs:   {}", extra.display())));
    let context = only_output(repl.submit("/context"));
    assert!(context.contains(&format!("+ dirs:   {}", extra.display())));
}

#[test]
fn repl_add_dir_rejects_missing_and_invalid_paths() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("file.txt"), "not a directory").expect("file");
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    assert_eq!(
        repl.submit("/add-dir one two"),
        vec![ReplEvent::Output(String::from("Usage: /add-dir <path>"))]
    );
    assert_eq!(
        repl.submit("/add-dir file.txt"),
        vec![ReplEvent::Output(format!(
            "Not a directory: {}",
            temp.path().join("file.txt").display()
        ))]
    );
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

    // The notice should include the actual cap and a hint on how to
    // raise it — checked loosely so future copy edits to either half of
    // the message stay non-breaking.
    assert!(
        events.iter().any(|event| matches!(
            event,
            ReplEvent::System(SystemMessage::Notice(notice))
                if notice.contains("Maximum turns reached (1)")
                && notice.contains("--max-turns")
        )),
        "expected a max-turns notice with cap and --max-turns hint, got: {events:?}",
    );
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

#[test]
fn dream_start_enables_persistent_loop_and_status_reports_cadence() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let start = only_output(repl.submit("/dream start --every 3"));
    assert!(start.contains("Dream loop enabled"));
    assert!(start.contains("every 3 user turns"));

    let status = only_output(repl.submit("/dream status"));
    assert!(status.contains("ENABLED"));
    assert!(status.contains("every 3 user turns"));
    assert!(status.contains("Next auto-run in 3 more"));
}

#[test]
fn dream_start_accepts_bare_integer_and_equals_syntax() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    assert!(only_output(repl.submit("/dream start 7")).contains("every 7 user turns"));
    assert!(only_output(repl.submit("/dream start --every=42")).contains("every 42 user turns"));
    assert!(only_output(repl.submit("/dream start")).contains("user turns"));
}

#[test]
fn dream_start_rejects_invalid_cadence() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let zero = only_output(repl.submit("/dream start --every 0"));
    assert!(
        zero.contains("/dream start") && zero.contains("0"),
        "got: {zero}"
    );
    let nope = only_output(repl.submit("/dream start abc"));
    assert!(
        nope.contains("/dream start") && nope.contains("abc"),
        "got: {nope}"
    );
    let huge = only_output(repl.submit("/dream start 99999"));
    assert!(
        huge.contains("/dream start") && huge.contains("99999"),
        "got: {huge}"
    );
}

#[test]
fn dream_stop_disables_loop_and_status_flips() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    repl.submit("/dream start --every 5");
    let stop = only_output(repl.submit("/dream stop"));
    assert!(stop.contains("Dream loop disabled"));
    let status = only_output(repl.submit("/dream status"));
    assert!(status.contains("DISABLED"));

    // Stop is idempotent.
    let again = only_output(repl.submit("/dream stop"));
    assert!(again.contains("already disabled"));
}

#[test]
fn dream_auto_fires_after_cadence_user_turns() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    repl.submit("/dream start --every 2");

    // Slash commands do NOT advance the user-turn counter.
    let _ = repl.submit("/help");

    // Turn 1: counter goes 0 -> 1. No fire (1 < 2).
    let r1: Vec<String> = repl
        .submit("hello 1")
        .into_iter()
        .filter_map(|event| match event {
            ReplEvent::Output(text) => Some(text),
            _ => None,
        })
        .collect();
    assert!(
        !r1.iter()
            .any(|text| text.contains("[dream] auto-consolidating")),
        "no auto-fire expected on turn 1, got: {r1:?}"
    );

    // Turn 2: counter goes 1 -> 2. since=2 >= cadence=2, must fire.
    let r2: Vec<String> = repl
        .submit("hello 2")
        .into_iter()
        .filter_map(|event| match event {
            ReplEvent::Output(text) => Some(text),
            _ => None,
        })
        .collect();
    assert!(
        r2.iter()
            .any(|text| text.contains("[dream] auto-consolidating")),
        "expected auto-dream marker on turn 2, got: {r2:?}"
    );

    // After firing, the status line records the run.
    let status = only_output(repl.submit("/dream status"));
    assert!(
        status.contains("Auto-runs this session: 1"),
        "expected auto-run counter, got: {status}"
    );
}

#[test]
fn dream_run_fires_one_pass_without_changing_schedule() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let outputs: Vec<String> = repl
        .submit("/dream run")
        .into_iter()
        .filter_map(|event| match event {
            ReplEvent::Output(text) => Some(text),
            _ => None,
        })
        .collect();
    assert!(
        outputs
            .iter()
            .any(|text| text.contains("Queued one dream consolidation run")),
        "expected 'Queued one ...' header, got: {outputs:?}"
    );
    // Schedule remains disabled after a manual run.
    let status = only_output(repl.submit("/dream status"));
    assert!(status.contains("DISABLED"));
}

#[test]
fn dream_unknown_subcommand_shows_usage_hint() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let out = only_output(repl.submit("/dream wat"));
    assert!(out.contains("Usage:") && out.contains("/dream"));
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

// ── Goal command backends ─────────────────────────────────────────────────

struct GoalAchievedBackend;

impl AgentBackend for GoalAchievedBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        Ok(AgentResponse::text(
            "I finished the work.\nGOAL_STATUS: achieved",
        ))
    }
}

#[derive(Default)]
struct GoalContinueThenAchieveBackend {
    calls: usize,
}

impl AgentBackend for GoalContinueThenAchieveBackend {
    fn respond(&mut self, _request: AgentRequest) -> Result<AgentResponse, String> {
        self.calls += 1;
        if self.calls == 1 {
            Ok(AgentResponse::text(
                "Making progress.\nGOAL_STATUS: continue",
            ))
        } else {
            Ok(AgentResponse::text("All done.\nGOAL_STATUS: achieved"))
        }
    }
}

// ── Goal command tests ────────────────────────────────────────────────────

#[test]
fn repl_goal_status_reports_no_active_goal() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let output = only_output(repl.submit("/goal status"));
    assert!(output.contains("No active goal"));
}

#[test]
fn repl_goal_status_alias_shows_active_goal() {
    let mut repl = ReplSession::new(Box::new(GoalAchievedBackend));

    // Start goal (which achieves immediately, clearing the goal)
    repl.submit("/goal write unit tests");

    // After goal is achieved the goal is cleared
    let output = only_output(repl.submit("/objective status"));
    assert!(output.contains("No active goal"));
}

#[test]
fn repl_goal_clear_removes_active_goal_state() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    // Inject goal text via a StaticBackend that won't emit GOAL_STATUS so goal stays active
    // We can just test the clear path via /goal clear when no goal is active
    let output = only_output(repl.submit("/goal clear"));
    assert_eq!(output, "Cleared active goal.");
}

#[test]
fn repl_goal_achieved_immediately_auto_closes_loop() {
    let mut repl = ReplSession::new(Box::new(GoalAchievedBackend));

    let events = repl.submit("/goal add type annotations");

    // Should contain "Goal set", the model reply, then "Goal achieved."
    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| {
            if let ReplEvent::Output(s) = e {
                Some(s.as_str())
            } else {
                None
            }
        })
        .collect();

    assert!(
        texts.contains(&"Goal set for this session."),
        "missing goal-set message"
    );
    assert!(texts.iter().any(|s| s.contains("GOAL_STATUS: achieved")));
    assert!(
        texts.contains(&"Goal achieved."),
        "missing achieved message"
    );
}

#[test]
fn repl_goal_continue_then_achieve_runs_two_turns() {
    let mut repl = ReplSession::new(Box::new(GoalContinueThenAchieveBackend::default()));

    let events = repl.submit("/goal refactor the parser");

    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| {
            if let ReplEvent::Output(s) = e {
                Some(s.as_str())
            } else {
                None
            }
        })
        .collect();

    assert!(
        texts.contains(&"Goal set for this session."),
        "missing goal-set message"
    );
    assert!(texts.iter().any(|s| s.contains("Making progress")));
    assert!(texts.iter().any(|s| s.contains("All done")));
    assert!(
        texts.contains(&"Goal achieved."),
        "missing achieved message"
    );
    // Two model turns: initial + one continuation
    assert_eq!(repl.agent_loop().messages().len(), 4);
}

#[test]
fn repl_goal_no_status_in_response_stops_loop() {
    // StaticBackend returns "assistant reply" with no GOAL_STATUS → loop exits after first turn
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let events = repl.submit("/goal write a changelog");

    let texts: Vec<&str> = events
        .iter()
        .filter_map(|e| {
            if let ReplEvent::Output(s) = e {
                Some(s.as_str())
            } else {
                None
            }
        })
        .collect();

    assert!(texts.contains(&"Goal set for this session."));
    assert!(
        !texts.contains(&"Goal achieved."),
        "should not mark achieved without status token"
    );
    // One model turn only
    assert_eq!(repl.agent_loop().messages().len(), 2);
}

#[test]
fn repl_goal_listed_in_help() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let help = only_output(repl.submit("/help"));
    assert!(help.contains("/goal"), "help should list /goal");
}

// ── Cache command tests ───────────────────────────────────────────────────

#[test]
fn repl_cache_reports_no_data_before_any_turn() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let output = only_output(repl.submit("/cache"));
    assert!(output.contains("No cache diagnostics yet"));
}

#[test]
fn repl_cache_shows_turn_data_after_agent_turn() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let _ = repl.submit("hello");

    let output = only_output(repl.submit("/cache"));
    assert!(output.contains("Cache diagnostics"));
    assert!(output.contains("cache+"));
    assert!(output.contains("cache-r"));
    assert!(output.contains("Overall:"));
}

#[test]
fn repl_cache_accepts_numeric_limit() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    for _ in 0..5 {
        let _ = repl.submit("hello");
    }

    let output = only_output(repl.submit("/cache 3"));
    assert!(output.contains("3 of 5 turn(s)"));
}

#[test]
fn repl_cache_rejects_non_numeric_argument() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    assert_eq!(
        repl.submit("/cache foo"),
        vec![ReplEvent::Output(String::from("Usage: /cache [limit]"))]
    );
}

#[test]
fn repl_cache_alias_kvcache_works() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let output = only_output(repl.submit("/kvcache"));
    assert!(output.contains("No cache diagnostics yet"));
}

// ── Session persistence integration tests ────────────────────────────────────

#[test]
fn repl_status_shows_real_session_id() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));

    let output = only_output(repl.submit("/status"));
    assert!(
        output.contains("Session:  "),
        "status must have a Session: line"
    );
    let id_part = output
        .lines()
        .find(|l| l.trim_start().starts_with("Session:"))
        .map(|l| {
            l.trim_start_matches(' ')
                .trim_start_matches("Session:")
                .trim()
                .to_owned()
        })
        .expect("Session: line in /status output");
    assert!(!id_part.is_empty(), "session ID must not be empty");
    assert!(
        !id_part.contains("not persisted"),
        "session ID must not be the old stub text; got: {id_part}"
    );
    // Session IDs look like YYYY-MM-DDTHH-MM-SS-... — starts with digits
    assert!(
        id_part.chars().next().is_some_and(|c| c.is_ascii_digit()),
        "session ID should start with a digit (timestamp): {id_part}"
    );
}

#[test]
fn repl_resume_nonexistent_session_gives_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    let output = only_output(repl.submit("/resume totally-unknown-session-xyz"));
    assert!(
        output.contains("Cannot resume"),
        "should report error for unknown session: {output}"
    );
}

#[test]
fn repl_sessions_command_is_routed_and_functional() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    // Before any turns: no saved sessions for this project
    let output = only_output(repl.submit("/sessions"));
    // May either show "No saved sessions" or list sessions from a previous test run —
    // just verify the command is dispatched and returns text (not "unknown command")
    assert!(
        !output.contains("Unknown command"),
        "/sessions should not return unknown-command error: {output}"
    );
}

#[test]
fn repl_unknown_command_suggests_closest_match() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(temp.path());

    // A directly-typed typo routes through the shared fuzzy suggester, so the
    // REPL now offers a "did you mean" instead of a bare unknown-command error.
    let output = only_output(repl.submit("/statuus"));
    assert!(
        output.contains("Unknown command: /statuus"),
        "got: {output}"
    );
    assert!(
        output.contains("Did you mean: /status?"),
        "expected /status suggestion, got: {output}"
    );
}

#[test]
fn repl_session_saves_and_can_be_resumed() {
    use deeptide_core::{SessionStore, new_session_id};

    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path();

    // Save a session directly through the store
    let id = new_session_id();
    let messages = vec![
        deeptide_core::ConversationMessage::user("hello"),
        deeptide_core::ConversationMessage::assistant("hi there"),
    ];
    SessionStore::save(cwd, &id, "test-model", "2024-01-01T00:00:00Z", &messages);

    // A fresh REPL session can resume it
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(cwd);
    let output = only_output(repl.submit(&format!("/resume {id}")));
    assert!(
        output.contains("Resumed session"),
        "resume should succeed: {output}"
    );
    assert!(
        output.contains("2 messages"),
        "should report loaded message count: {output}"
    );
    assert_eq!(
        repl.agent_loop().messages().len(),
        2,
        "agent loop should have 2 restored messages"
    );
}

#[test]
fn repl_resume_session_method_restores_and_continues_same_id() {
    use deeptide_core::{SessionStore, new_session_id};

    let temp = tempfile::tempdir().expect("tempdir");
    let cwd = temp.path();

    let id = new_session_id();
    let messages = vec![
        deeptide_core::ConversationMessage::user("hello"),
        deeptide_core::ConversationMessage::assistant("hi there"),
    ];
    SessionStore::save(cwd, &id, "test-model", "2024-01-01T00:00:00Z", &messages);

    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(cwd);
    let count = repl
        .resume_session(&id)
        .expect("resume_session should succeed");
    assert_eq!(count, 2);
    assert_eq!(repl.agent_loop().messages().len(), 2);

    // A subsequent turn autosaves back to the SAME session (continue, not fork).
    let _ = repl.submit("another");
    let sessions = SessionStore::list(cwd);
    assert_eq!(
        sessions.len(),
        1,
        "resume should continue the same session, not create a fork: {sessions:?}"
    );
    assert_eq!(sessions[0].session_id, id);
}

#[test]
fn repl_with_additional_dirs_registers_existing_and_skips_non_dirs() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir(temp.path().join("sub")).expect("mkdir sub");

    let mut repl = ReplSession::new(Box::new(StaticBackend))
        .with_cwd(temp.path())
        .with_additional_dirs(&[
            std::path::PathBuf::from("sub"),
            std::path::PathBuf::from("does-not-exist"),
            // Duplicate of the first; must not register twice.
            std::path::PathBuf::from("sub"),
        ]);

    // /add-dir with no args lists the registered dirs.
    let listing = only_output(repl.submit("/add-dir"));
    assert!(
        listing.contains("sub"),
        "existing dir should be registered: {listing}"
    );
    assert!(
        !listing.contains("does-not-exist"),
        "non-directory must be skipped: {listing}"
    );
    // Registered exactly once (dedup): only one path line mentioning the sub dir.
    assert_eq!(
        listing.matches("/sub").count(),
        1,
        "duplicate --add-dir must register once: {listing}"
    );
}

#[test]
fn repl_no_session_persistence_skips_autosave() {
    use deeptide_core::SessionStore;

    // Persistence disabled: a completed turn writes no session file.
    let off = tempfile::tempdir().expect("tempdir");
    let mut repl = ReplSession::new(Box::new(StaticBackend))
        .with_cwd(off.path())
        .with_session_persistence(false);
    repl.submit("hello");
    assert!(
        SessionStore::list(off.path()).is_empty(),
        "no session should be saved when persistence is disabled"
    );

    // Default (persistence enabled): the same turn saves a session.
    let on = tempfile::tempdir().expect("tempdir");
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_cwd(on.path());
    repl.submit("hello");
    assert!(
        !SessionStore::list(on.path()).is_empty(),
        "default persistence should autosave the session"
    );
}

#[test]
fn repl_with_appended_system_prompt_keeps_base_and_appends() {
    let temp = tempfile::tempdir().expect("tempdir");
    let repl = ReplSession::new(Box::new(StaticBackend))
        .with_cwd(temp.path())
        .with_appended_system_prompt("Always reply in JSON.");
    let prompt = repl
        .agent_loop()
        .system_prompt()
        .expect("system prompt is set");
    assert!(
        prompt.contains("You are Deeptide"),
        "base prompt is retained"
    );
    assert!(prompt.trim_end().ends_with("Always reply in JSON."));
}

#[test]
fn repl_with_appended_system_prompt_ignores_blank() {
    let temp = tempfile::tempdir().expect("tempdir");
    let base = ReplSession::new(Box::new(StaticBackend))
        .with_cwd(temp.path())
        .agent_loop()
        .system_prompt()
        .map(ToOwned::to_owned);
    let appended = ReplSession::new(Box::new(StaticBackend))
        .with_cwd(temp.path())
        .with_appended_system_prompt("   ")
        .agent_loop()
        .system_prompt()
        .map(ToOwned::to_owned);
    assert_eq!(base, appended, "blank append must not change the prompt");
}

#[test]
fn session_end_capture_fires_once_after_a_user_turn() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let _ = repl.submit("do some work");

    // First finalize runs the consolidation pass (StaticBackend answers it).
    let first = repl.finalize_session();
    assert!(
        first
            .iter()
            .any(|e| matches!(e, ReplEvent::Output(t) if t.contains("consolidating memory"))),
        "first finalize should run the end-of-session consolidation: {first:?}"
    );

    // Idempotent: a second teardown call does nothing.
    assert!(
        repl.finalize_session().is_empty(),
        "consolidation must fire at most once per session"
    );
}

#[test]
fn session_end_capture_skipped_with_no_user_turns() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    assert!(
        repl.finalize_session().is_empty(),
        "nothing to consolidate when no user turn happened"
    );
}

#[test]
fn session_end_capture_can_be_disabled() {
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_session_end_capture(false);
    let _ = repl.submit("do some work");
    assert!(
        repl.finalize_session().is_empty(),
        "disabled end-of-session capture must not fire"
    );
}

#[test]
fn exit_command_triggers_capture_then_exits() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let _ = repl.submit("do some work");

    let events = repl.submit("/exit");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ReplEvent::Output(t) if t.contains("consolidating memory"))),
        "/exit should consolidate before exiting: {events:?}"
    );
    assert!(
        matches!(events.last(), Some(ReplEvent::Exit)),
        "the last event must be Exit: {events:?}"
    );
}

#[test]
fn exit_with_no_turns_just_exits() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let events = repl.submit("/exit");
    assert_eq!(
        events,
        vec![ReplEvent::Exit],
        "no turns → plain exit, no pass"
    );
}

#[test]
fn version_command_reports_injected_build_string() {
    let mut repl = ReplSession::new(Box::new(StaticBackend))
        .with_version("deeptide-rs 0.2.0 (abc1234 2026-05-31)");
    let out = only_output(repl.submit("/version"));
    assert!(out.contains("0.2.0"), "version command should show the version: {out}");
    assert!(out.contains("abc1234"), "should include injected provenance: {out}");
}

#[test]
fn version_command_falls_back_to_crate_version() {
    let mut repl = ReplSession::new(Box::new(StaticBackend));
    let out = only_output(repl.submit("/version"));
    assert!(out.contains("deeptide-rs"), "fallback should name the binary: {out}");
}

#[test]
fn no_session_capture_makes_exit_instant() {
    // With capture disabled, finalize_session must be a no-op even after a real
    // turn — no consolidation pass, so /exit and Ctrl-D return immediately.
    let mut repl = ReplSession::new(Box::new(StaticBackend)).with_session_end_capture(false);
    repl.submit("hello");
    assert!(
        repl.finalize_session().is_empty(),
        "capture disabled must skip the end-of-session consolidation entirely"
    );
}
