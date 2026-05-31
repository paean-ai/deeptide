use deeptide_core::{
    AgentTool, AppendFileTool, AskUserQuestionTool, AudioTranscribeTool, BashTool, BriefTool,
    ClipboardTool, CrashLogTool, CronCreateTool, CronDeleteTool, CronListTool, CtxInspectTool,
    EditTool, EnterPlanModeTool, EnterWorktreeTool, ExitPlanModeTool, ExitWorktreeTool,
    FileMetadataTool, GetMcpPromptTool, ImagePreprocessTool, ListMcpPromptsTool,
    ListMcpResourcesTool, LspTool, MacDiagnoseTool, MacLogTool, McpTool, MemorySearchTool,
    MemoryWriteTool, MonitorTool, NotebookEditTool, PublishTool, PushNotificationTool,
    ReadFilesTool, ReadMcpResourceTool, RemoteTriggerTool, ReviewArtifactTool, ScreenCaptureTool,
    SkillTool, SleepTool, SnipTool, SpotlightSearchTool, TaskCreateTool, TaskGetTool, TaskListTool,
    TaskOutputTool, TaskStopTool, TaskUpdateTool, TodoWriteTool, Tool, ToolContext, ToolRegistry,
    ToolSearchTool, VerifyPlanExecutionTool, VideoTranscribeTool, VisionTool, WebFetchTool,
    WebSearchTool, WriteTool, memory::MemorySystem,
};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock, mpsc};
use std::thread;

#[test]
fn read_tool_reads_text_file_with_line_numbers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("notes.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\n").expect("write fixture");
    let registry = ToolRegistry::with_builtin_tools();

    let result = registry.call(
        "Read",
        serde_json::json!({"file_path": "notes.txt"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert_eq!(result.content, "1\talpha\n2\tbeta\n3\tgamma");
}

#[test]
fn read_tools_block_sensitive_files_until_opened() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join(".env"), "API_KEY=super-secret\n").expect("write .env");
    std::fs::write(temp.path().join("notes.txt"), "ok\n").expect("write notes");
    let registry = ToolRegistry::with_builtin_tools();
    let context = ToolContext::new(temp.path());

    // Read refuses the sensitive file and points the user at /open.
    let blocked = registry.call("Read", serde_json::json!({"file_path": ".env"}), &context);
    assert!(blocked.is_error, "reading .env should be blocked");
    assert!(blocked.content.contains("sensitive"));
    assert!(blocked.content.contains("/open"));
    assert!(
        !blocked.content.contains("super-secret"),
        "the secret value must not leak in the denial"
    );

    // ReadFiles denies the sensitive entry per-file but still reads the rest.
    let multi = registry.call(
        "ReadFiles",
        serde_json::json!({"paths": [".env", "notes.txt"]}),
        &context,
    );
    assert!(multi.content.contains("sensitive"));
    assert!(!multi.content.contains("super-secret"));
    assert!(
        multi.content.contains("ok"),
        "non-sensitive file still read"
    );

    // After /open marks it readable, Read succeeds. Mark the same resolved
    // path the Read tool computes so normalization can't cause a mismatch.
    deeptide_core::sensitive_file::mark_open(&context.resolve_path(".env"));
    let allowed = registry.call("Read", serde_json::json!({"file_path": ".env"}), &context);
    assert!(!allowed.is_error, "opened .env should read");
    assert!(allowed.content.contains("API_KEY=super-secret"));
}

#[test]
fn read_tool_respects_offset_and_limit() {
    let temp = tempfile::tempdir().expect("tempdir");
    let path = temp.path().join("notes.txt");
    std::fs::write(&path, "alpha\nbeta\ngamma\ndelta\n").expect("write fixture");
    let registry = ToolRegistry::with_builtin_tools();

    let result = registry.call(
        "Read",
        serde_json::json!({"file_path": "notes.txt", "offset": 2, "limit": 2}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    // An explicit limit truncates silently — no continuation notice is added.
    assert_eq!(result.content, "2\tbeta\n3\tgamma");
}

#[test]
fn read_tool_applies_default_line_limit_and_reports_continuation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut body = String::new();
    for n in 1..=2_050 {
        body.push_str(&format!("line {n}\n"));
    }
    std::fs::write(temp.path().join("big.txt"), body).expect("write fixture");
    let registry = ToolRegistry::with_builtin_tools();

    let result = registry.call(
        "Read",
        serde_json::json!({"file_path": "big.txt"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("1\tline 1"));
    assert!(result.content.contains("2000\tline 2000"));
    // The default limit stops at 2000 lines and never reaches 2001.
    assert!(!result.content.contains("2001\tline 2001"));
    assert!(
        result
            .content
            .contains("Read stopped at the default 2000-line limit")
    );
    assert!(result.content.contains("offset: 2001"));
}

#[test]
fn read_tool_rejects_output_exceeding_token_cap() {
    let temp = tempfile::tempdir().expect("tempdir");
    // ~1500 lines of 100 chars ≈ 150k chars ≈ 37k tokens, over the 25k cap but
    // within the 2000-line window so the token guard is what trips.
    let mut body = String::new();
    let filler = "x".repeat(100);
    for _ in 0..1_500 {
        body.push_str(&filler);
        body.push('\n');
    }
    std::fs::write(temp.path().join("wide.txt"), body).expect("write fixture");
    let registry = ToolRegistry::with_builtin_tools();

    let result = registry.call(
        "Read",
        serde_json::json!({"file_path": "wide.txt"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error, "oversized output should be an error");
    assert!(result.content.contains("too large to return safely"));
    assert!(result.content.contains("Grep"));
}

#[test]
fn read_tool_reports_missing_files_with_hint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry = ToolRegistry::with_builtin_tools();

    let result = registry.call(
        "Read",
        serde_json::json!({"file_path": "missing.rs"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(result.content.contains("File does not exist"));
    assert!(result.content.contains("Glob"));
}

#[test]
fn read_tool_reports_special_binary_formats_without_dumping_bytes() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("diagram.png"),
        [0x89, b'P', b'N', b'G', 0, 1],
    )
    .expect("write fixture");
    let registry = ToolRegistry::with_builtin_tools();

    let result = registry.call(
        "Read",
        serde_json::json!({"file_path": "diagram.png"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(result.content.contains("reason: unsupported binary"));
    assert!(result.content.contains("file: diagram.png"));
    assert!(result.content.contains("type: image"));
    assert!(result.content.contains("next_action: use a dedicated tool"));
    assert!(!result.content.contains("PNG\u{0}"));
}

#[test]
fn file_metadata_tool_reports_existing_text_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "hello").expect("write fixture");

    let result = FileMetadataTool.call(
        serde_json::json!({"file_path": "notes.txt"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("[FileMetadata]"));
    assert!(result.content.contains("exists: true"));
    assert!(result.content.contains("kind: file"));
    assert!(result.content.contains("size: 5 bytes"));
    assert!(result.content.contains("extension: txt"));
    assert!(result.content.contains("type: text"));
    assert!(result.content.contains("mime: text/plain"));
}

#[test]
fn file_metadata_tool_reports_missing_files() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = FileMetadataTool.call(
        serde_json::json!({"file_path": "missing.bin"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("missing.bin"));
    assert!(result.content.contains("exists: false"));
}

#[test]
fn read_files_tool_reads_multiple_files_with_headers() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("a.txt"), "alpha\n").expect("write fixture");
    std::fs::create_dir_all(temp.path().join("src")).expect("mkdir");
    std::fs::write(temp.path().join("src/b.txt"), "bravo\ncharlie\n").expect("write fixture");

    let result = ReadFilesTool.call(
        serde_json::json!({"paths": ["a.txt", "src/b.txt"]}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("===== a.txt =====\n1\talpha"));
    assert!(
        result
            .content
            .contains("===== src/b.txt =====\n1\tbravo\n2\tcharlie")
    );
}

#[test]
fn read_files_tool_keeps_per_file_errors_in_result() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("src")).expect("mkdir");
    std::fs::write(temp.path().join("ok.txt"), "alpha\n").expect("write fixture");

    let result = ReadFilesTool.call(
        serde_json::json!({"paths": ["ok.txt", "missing.txt", "src"]}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("===== ok.txt ====="));
    assert!(
        result
            .content
            .contains("===== missing.txt =====\n[Error: File does not exist")
    );
    assert!(
        result
            .content
            .contains("===== src =====\n[Error: Path is a directory")
    );
}

#[test]
fn read_files_tool_rejects_empty_and_oversized_batches() {
    let temp = tempfile::tempdir().expect("tempdir");
    let empty = ReadFilesTool.call(
        serde_json::json!({"paths": []}),
        &ToolContext::new(temp.path()),
    );
    assert!(empty.is_error);
    assert_eq!(empty.content, "No paths provided");

    let paths = (0..51)
        .map(|index| format!("{index}.txt"))
        .collect::<Vec<_>>();
    let oversized = ReadFilesTool.call(
        serde_json::json!({"paths": paths}),
        &ToolContext::new(temp.path()),
    );
    assert!(oversized.is_error);
    assert!(oversized.content.contains("exceeds 50 entries"));
}

#[test]
fn registry_reports_unknown_tools() {
    let result = ToolRegistry::with_builtin_tools().call(
        "Nope",
        serde_json::json!({}),
        &ToolContext::new("."),
    );

    assert!(result.is_error);
    assert_eq!(result.content, "Unknown tool: Nope");
}

#[test]
fn tool_search_finds_tools_by_capability_synonyms() {
    let result = ToolSearchTool.call(
        serde_json::json!({"query": "file metadata quarantine"}),
        &ToolContext::new("."),
    );

    assert!(!result.is_error);
    assert!(
        result
            .content
            .lines()
            .next()
            .unwrap_or("")
            .contains("FileMetadata")
    );

    let grep = ToolSearchTool.call(
        serde_json::json!({"query": "regex search"}),
        &ToolContext::new("."),
    );
    assert!(!grep.is_error);
    assert!(grep.content.contains("Grep"));
}

#[test]
fn tool_search_supports_exact_names_and_select_syntax() {
    let exact = ToolSearchTool.call(serde_json::json!({"query": "Read"}), &ToolContext::new("."));
    assert!(!exact.is_error);
    assert_eq!(exact.content.lines().count(), 1);
    assert!(exact.content.contains("- Read [read-only, parallel]"));

    let selected = ToolSearchTool.call(
        serde_json::json!({"query": "select:Read,Edit,MissingTool"}),
        &ToolContext::new("."),
    );
    assert!(!selected.is_error);
    assert!(selected.content.contains("- Read [read-only, parallel]"));
    assert!(selected.content.contains("- Edit [writes]"));
    assert!(selected.content.contains("- MissingTool - not found"));
}

#[test]
fn agent_tool_validates_swift_agent_types_and_reports_runtime_gap() {
    let context = ToolContext::new(".");

    let missing_prompt = AgentTool.call(
        serde_json::json!({"description": "Find auth flow"}),
        &context,
    );
    assert!(missing_prompt.is_error);
    assert_eq!(missing_prompt.content, "Missing prompt parameter");

    let unknown = AgentTool.call(
        serde_json::json!({
            "description": "Find auth flow",
            "prompt": "Map the auth flow.",
            "subagent_type": "Scout"
        }),
        &context,
    );
    assert!(unknown.is_error);
    assert!(
        unknown
            .content
            .contains("Unknown agent type: Scout. Available: general-purpose, Explore, Plan")
    );

    let invalid_background = AgentTool.call(
        serde_json::json!({
            "description": "Find auth flow",
            "prompt": "Map the auth flow.",
            "run_in_background": true,
            "isolation": "worktree"
        }),
        &context,
    );
    assert!(invalid_background.is_error);
    assert!(
        invalid_background
            .content
            .contains("Cannot combine run_in_background with isolation worktree")
    );

    let explore = AgentTool.call(
        serde_json::json!({
            "description": "Find auth flow",
            "prompt": "Map the auth flow.",
            "subagent_type": "Explore",
            "model": "fast-model"
        }),
        &context,
    );
    assert!(explore.is_error);
    // AgentTool only ever reaches the registry fallback when something
    // dispatches `Agent` outside the AgentLoop (library misuse or a
    // test bypassing the loop). The CLI always wires a factory, so the
    // fallback message must point users at the actual recovery path
    // rather than implying the feature is unimplemented.
    assert!(
        explore
            .content
            .contains("Agent tool reached the registry fallback path"),
        "stub must describe the real failure mode, not 'not available'"
    );
    assert!(
        explore.content.contains("with_subagent_backend_factory"),
        "stub must name the recovery API so callers can fix it"
    );
    // Diagnostic metadata that helps the caller understand what they
    // asked for must still be present.
    assert!(explore.content.contains("Type: Explore"));
    assert!(explore.content.contains("Model: fast-model"));
    assert!(explore.content.contains("Max turns: 40"));
    assert!(explore.content.contains("Read-only: true"));
    assert!(explore.content.contains("ListMcpResources"));
}

#[cfg(unix)]
#[test]
fn mcp_tools_call_configured_stdio_servers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let server = temp.path().join("fake-mcp.sh");
    std::fs::write(
        &server,
        r#"#!/bin/sh
if [ "$1" != "serve" ] || [ "$DEEPTIDE_TEST_MODE" != "stdio" ]; then
  echo "bad launch context" >&2
  exit 7
fi
cat >/dev/null
body1='{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{},"serverInfo":{"name":"fake","version":"1"}}}'
body2='{"jsonrpc":"2.0","id":2,"result":{"resources":[{"uri":"file://guide.md","name":"Guide"}],"prompts":[{"name":"review","description":"Review code"}],"tools":[{"name":"lookup","description":"Look up project facts"}],"content":[{"type":"text","text":"hello"}]}}'
printf 'Content-Length: %s\r\n\r\n%s' "${#body1}" "$body1"
printf 'Content-Length: %s\r\n\r\n%s' "${#body2}" "$body2"
"#,
    )
    .expect("write fake server");
    let mut permissions = std::fs::metadata(&server)
        .expect("fake server metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&server, permissions).expect("chmod fake server");

    std::fs::write(
        temp.path().join(".mcp.json"),
        serde_json::json!({
            "mcpServers": {
                "docs": {
                    "command": server.display().to_string(),
                    "args": ["serve"],
                    "env": {"DEEPTIDE_TEST_MODE": "stdio"}
                },
                "disabled": {
                    "command": server.display().to_string(),
                    "disabled": true
                },
                "remote": {"url": "https://mcp.example.invalid"}
            }
        })
        .to_string(),
    )
    .expect("write mcp fixture");
    let context = ToolContext::new(temp.path());

    let resources = ListMcpResourcesTool.call(serde_json::json!({}), &context);
    assert!(!resources.is_error);
    assert!(resources.content.contains("[docs]"));
    assert!(resources.content.contains("command:"));
    assert!(resources.content.contains("file://guide.md - Guide"));
    assert!(!resources.content.contains("[disabled]"));
    assert!(resources.content.contains("[remote]"));
    assert!(
        resources
            .content
            .contains("url: https://mcp.example.invalid")
    );
    // The remote server now attempts a real HTTP request; the reserved
    // `.invalid` host never resolves, so it surfaces a transport error rather
    // than the old "not implemented" stub.
    assert!(
        resources
            .content
            .contains("MCP HTTP request to remote failed")
    );
    assert!(!resources.content.contains("not implemented"));

    let prompts = ListMcpPromptsTool.call(serde_json::json!({"server": "docs"}), &context);
    assert!(!prompts.is_error);
    assert!(prompts.content.contains("[docs]"));
    assert!(prompts.content.contains("review - Review code"));
    assert!(!prompts.content.contains("[remote]"));

    let read = ReadMcpResourceTool.call(
        serde_json::json!({"server": "docs", "uri": "file://guide.md"}),
        &context,
    );
    assert!(!read.is_error);
    assert!(read.content.contains("\"text\": \"hello\""));

    let prompt = GetMcpPromptTool.call(
        serde_json::json!({"server": "docs", "name": "review"}),
        &context,
    );
    assert!(!prompt.is_error);
    assert!(prompt.content.contains("\"prompts\""));

    let forward = McpTool.call(
        serde_json::json!({"server": "docs", "method": "resources/list"}),
        &context,
    );
    assert!(!forward.is_error);
    assert!(forward.content.contains("\"resources\""));

    let dynamic = ToolRegistry::with_builtin_tools().call(
        "mcp__docs__lookup",
        serde_json::json!({"query": "guide"}),
        &context,
    );
    assert!(!dynamic.is_error);
    assert!(dynamic.content.contains("\"content\""));

    let search = ToolSearchTool.call(serde_json::json!({"query": "mcp lookup"}), &context);
    assert!(!search.is_error);
    assert!(search.content.contains("[MCP tools]"));
    assert!(search.content.contains("mcp__docs__lookup"));
    assert!(search.content.contains("Look up project facts"));
}

#[test]
fn mcp_tools_call_configured_http_servers() {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    fn json_rpc_http(id: Option<i64>, result: serde_json::Value) -> String {
        let body = serde_json::json!({"jsonrpc": "2.0", "id": id, "result": result}).to_string();
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        )
    }

    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock MCP server");
    let port = listener.local_addr().expect("addr").port();

    // Minimal Streamable HTTP MCP server: answers `initialize` with a session
    // id, ack's the `notifications/initialized` notification, and returns
    // method results as JSON (or SSE for prompts/list, to cover both paths).
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
            let mut content_length = 0usize;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                if line == "\r\n" || line == "\n" {
                    break;
                }
                if let Some(value) = line.to_ascii_lowercase().strip_prefix("content-length:") {
                    content_length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0u8; content_length];
            if reader.read_exact(&mut body).is_err() {
                continue;
            }
            let request: serde_json::Value =
                serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
            let method = request
                .get("method")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let id = request.get("id").and_then(serde_json::Value::as_i64);

            let response = match method {
                "initialize" => {
                    let payload = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {"protocolVersion": "2024-11-05", "capabilities": {}, "serverInfo": {"name": "http-fake", "version": "1"}}
                    })
                    .to_string();
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nMcp-Session-Id: sess-123\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        payload.len(),
                        payload
                    )
                }
                "notifications/initialized" => String::from(
                    "HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                ),
                "prompts/list" => {
                    let payload = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {"prompts": [{"name": "review", "description": "Review code"}]}
                    })
                    .to_string();
                    let sse = format!("event: message\ndata: {payload}\n\n");
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        sse.len(),
                        sse
                    )
                }
                "resources/list" => json_rpc_http(
                    id,
                    serde_json::json!({"resources": [{"uri": "file://guide.md", "name": "Guide"}]}),
                ),
                "resources/read" => json_rpc_http(
                    id,
                    serde_json::json!({"contents": [{"type": "text", "text": "hello"}]}),
                ),
                "tools/call" => json_rpc_http(
                    id,
                    serde_json::json!({"content": [{"type": "text", "text": "looked up"}]}),
                ),
                _ => json_rpc_http(id, serde_json::json!({})),
            };
            stream.write_all(response.as_bytes()).ok();
            stream.flush().ok();
        }
    });

    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join(".mcp.json"),
        serde_json::json!({
            "mcpServers": {
                "remote": {
                    "url": format!("http://127.0.0.1:{port}/mcp"),
                    "headers": {"Authorization": "Bearer test-token"}
                }
            }
        })
        .to_string(),
    )
    .expect("write mcp fixture");
    let context = ToolContext::new(temp.path());

    let resources = ListMcpResourcesTool.call(serde_json::json!({}), &context);
    assert!(!resources.is_error, "resources: {}", resources.content);
    assert!(resources.content.contains("[remote]"));
    assert!(resources.content.contains("file://guide.md - Guide"));

    // prompts/list is served as SSE, exercising the event-stream parser.
    let prompts = ListMcpPromptsTool.call(serde_json::json!({"server": "remote"}), &context);
    assert!(!prompts.is_error, "prompts: {}", prompts.content);
    assert!(prompts.content.contains("review - Review code"));

    let read = ReadMcpResourceTool.call(
        serde_json::json!({"server": "remote", "uri": "file://guide.md"}),
        &context,
    );
    assert!(!read.is_error, "read: {}", read.content);
    assert!(read.content.contains("\"text\": \"hello\""));

    let dynamic = ToolRegistry::with_builtin_tools().call(
        "mcp__remote__lookup",
        serde_json::json!({"query": "guide"}),
        &context,
    );
    assert!(!dynamic.is_error, "dynamic: {}", dynamic.content);
    assert!(dynamic.content.contains("looked up"));
}

#[cfg(unix)]
#[test]
fn mcp_tools_support_swift_style_newline_json_framing() {
    let temp = tempfile::tempdir().expect("tempdir");
    let server = temp.path().join("fake-mcp-newline.sh");
    std::fs::write(
        &server,
        r#"#!/bin/sh
cat >/dev/null
printf '%s\n' '{"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"2024-11-05","capabilities":{},"serverInfo":{"name":"fake-newline","version":"1"}}}'
printf '%s\n' '{"jsonrpc":"2.0","id":2,"result":{"prompts":[{"name":"triage","description":"Triage issue"}]}}'
"#,
    )
    .expect("write fake newline server");
    let mut permissions = std::fs::metadata(&server)
        .expect("fake newline server metadata")
        .permissions();
    permissions.set_mode(0o755);
    std::fs::set_permissions(&server, permissions).expect("chmod fake newline server");

    std::fs::write(
        temp.path().join(".mcp.json"),
        serde_json::json!({
            "mcpServers": {
                "swiftish": {
                    "command": server.display().to_string(),
                    "framing": "newline"
                }
            }
        })
        .to_string(),
    )
    .expect("write mcp fixture");

    let prompts = ListMcpPromptsTool.call(
        serde_json::json!({"server": "swiftish"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!prompts.is_error);
    assert!(prompts.content.contains("[swiftish]"));
    assert!(prompts.content.contains("triage - Triage issue"));
}

#[test]
fn mcp_tools_report_missing_configuration_and_required_inputs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let context = ToolContext::new(temp.path());

    let list = ListMcpResourcesTool.call(serde_json::json!({}), &context);
    assert!(list.is_error);
    assert!(list.content.contains("No MCP servers configured"));

    let missing = McpTool.call(serde_json::json!({"server": "docs"}), &context);
    assert!(missing.is_error);
    assert_eq!(missing.content, "Missing server or method");

    let unknown = ReadMcpResourceTool.call(
        serde_json::json!({"server": "docs", "uri": "file://guide.md"}),
        &context,
    );
    assert!(unknown.is_error);
    assert_eq!(unknown.content, "MCP server not configured: docs");
}

#[test]
fn ask_user_question_tool_formats_questions_and_options() {
    let result = AskUserQuestionTool.call(
        serde_json::json!({
            "questions": [
                {
                    "question": "Which implementation path should I use?",
                    "header": "Path",
                    "multiSelect": false,
                    "options": [
                        {"label": "Small", "description": "Ship a narrow compatible increment."},
                        {"label": "Broad", "description": "Spend longer on a larger pass."}
                    ]
                },
                {
                    "question": "Which checks should run?",
                    "header": "Checks",
                    "multiSelect": true,
                    "options": [
                        {"label": "Tests", "description": "Run the Rust test suite."},
                        {"label": "Clippy", "description": "Run Rust lints."}
                    ]
                }
            ]
        }),
        &ToolContext::new("."),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("Questions for the user:"));
    assert!(
        result
            .content
            .contains("Q1: Which implementation path should I use?")
    );
    assert!(result.content.contains("Header: Path"));
    assert!(
        result
            .content
            .contains("[Small] Ship a narrow compatible increment.")
    );
    assert!(
        result
            .content
            .contains("Q2: Which checks should run? (multi-select)")
    );
    assert!(result.content.contains("Please answer each question"));
}

#[test]
fn ask_user_question_tool_validates_shape_and_limits() {
    let missing = AskUserQuestionTool.call(serde_json::json!({}), &ToolContext::new("."));
    assert!(missing.is_error);
    assert_eq!(missing.content, "At least one question is required");

    let one_option = AskUserQuestionTool.call(
        serde_json::json!({
            "questions": [{
                "question": "Pick one",
                "header": "Choice",
                "multiSelect": false,
                "options": [{"label": "Only", "description": "No alternative."}]
            }]
        }),
        &ToolContext::new("."),
    );
    assert!(one_option.is_error);
    assert!(
        one_option
            .content
            .contains("questions[0].options must contain 2 to 4 options")
    );

    let long_header = AskUserQuestionTool.call(
        serde_json::json!({
            "questions": [{
                "question": "Pick one",
                "header": "VeryLongHeader",
                "multiSelect": false,
                "options": [
                    {"label": "A", "description": "First."},
                    {"label": "B", "description": "Second."}
                ]
            }]
        }),
        &ToolContext::new("."),
    );
    assert!(long_header.is_error);
    assert!(
        long_header
            .content
            .contains("questions[0].header must be 12 characters or fewer")
    );
}

#[test]
fn bash_tool_executes_commands_in_workspace() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\n").expect("write fixture");

    let result = BashTool.call(
        serde_json::json!({"command": list_workspace_command()}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("notes.txt"));
}

#[test]
fn bash_tool_blocks_reading_sensitive_files_until_opened() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join(".env"), "API_KEY=super-secret\n").expect("write .env");
    let context = ToolContext::new(temp.path());

    // `cat .env` is refused before the command runs — the secret never leaks.
    let blocked = BashTool.call(serde_json::json!({"command": "cat .env"}), &context);
    assert!(blocked.is_error, "cat .env should be blocked");
    assert!(blocked.content.contains("Sensitive file access blocked"));
    assert!(!blocked.content.contains("super-secret"));

    // After /open marks it readable, the same command is permitted to run.
    deeptide_core::sensitive_file::mark_open(&context.resolve_path(".env"));
    let allowed = BashTool.call(serde_json::json!({"command": "cat .env"}), &context);
    assert!(!allowed.is_error, "opened .env should be readable via Bash");
    assert!(allowed.content.contains("super-secret"));
}

#[test]
fn bash_tool_reports_stderr_and_exit_status_as_error() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = BashTool.call(
        serde_json::json!({"command": stderr_failure_command()}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(result.content.contains("[stderr]"));
    assert!(result.content.contains("boom"));
}

#[test]
fn bash_tool_rejects_multiline_commands() {
    let result = BashTool.call(
        serde_json::json!({"command": "echo one\necho two"}),
        &ToolContext::new("."),
    );

    assert!(result.is_error);
    assert!(result.content.contains("command must be a single line"));
}

#[test]
fn monitor_tool_returns_stdout_and_exit_status() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = MonitorTool.call(
        serde_json::json!({"command": monitor_stdout_command(), "max_seconds": 5}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("(captured 2 lines, exit=0)"));
    assert!(result.content.contains("ready"));
    assert!(result.content.contains("done"));
}

#[test]
fn monitor_tool_returns_early_when_until_matches() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = MonitorTool.call(
        serde_json::json!({
            "command": monitor_until_command(),
            "max_seconds": 10,
            "until": "READY"
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("(matched `READY`"));
    assert!(result.content.contains("READY"));
}

#[test]
fn monitor_tool_reports_stderr_and_failed_exit() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = MonitorTool.call(
        serde_json::json!({"command": stderr_failure_command(), "max_seconds": 5}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(result.content.contains("exit=7"));
    assert!(result.content.contains("--- stderr ---"));
    assert!(result.content.contains("boom"));
}

#[test]
fn memory_write_tool_writes_project_memory_shard_and_index() {
    let _guard = memory_env_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    install_memory_env(temp.path());
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");

    let result = MemoryWriteTool.call(
        serde_json::json!({
            "title": "Coding Style",
            "body": "Keep tests narrow and deterministic.",
            "reason": "User wants durable test guidance",
            "type": "project"
        }),
        &ToolContext::new(&workspace),
    );

    assert!(!result.is_error);
    assert!(
        result
            .content
            .contains("Saved project memory: Coding Style")
    );
    let memory_dir = MemorySystem::project_memory_dir(&workspace);
    let shard = memory_dir.join("coding-style.md");
    let body = std::fs::read_to_string(&shard).expect("memory shard");
    assert!(body.contains("name: Coding Style"));
    assert!(body.contains("description: User wants durable test guidance"));
    assert!(body.contains("type: project"));
    assert!(body.contains("Keep tests narrow and deterministic."));

    let index = std::fs::read_to_string(memory_dir.join("MEMORY.md")).expect("memory index");
    assert!(index.contains("- [Coding Style](coding-style.md) - User wants durable test guidance"));
}

#[test]
fn memory_write_tool_defaults_global_type_and_avoids_duplicate_names() {
    let _guard = memory_env_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    install_memory_env(temp.path());
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let input = serde_json::json!({
        "title": "Editor Preference",
        "body": "Prefer concise code review findings first.",
        "reason": "Durable user preference",
        "scope": "global"
    });

    let first = MemoryWriteTool.call(input.clone(), &ToolContext::new(&workspace));
    let second = MemoryWriteTool.call(input, &ToolContext::new(&workspace));

    assert!(!first.is_error);
    assert!(!second.is_error);
    let memory_dir = MemorySystem::global_memory_dir();
    assert!(memory_dir.join("editor-preference.md").exists());
    assert!(memory_dir.join("editor-preference-2.md").exists());
    let content =
        std::fs::read_to_string(memory_dir.join("editor-preference.md")).expect("memory shard");
    assert!(content.contains("type: user"));
}

#[test]
fn memory_write_tool_rejects_invalid_inputs() {
    let missing_reason = MemoryWriteTool.call(
        serde_json::json!({
            "title": "Valid Title",
            "body": "Valid durable memory body."
        }),
        &ToolContext::new("."),
    );
    assert!(missing_reason.is_error);
    assert_eq!(missing_reason.content, "reason is required");

    let bad_scope = MemoryWriteTool.call(
        serde_json::json!({
            "title": "Valid Title",
            "body": "Valid durable memory body.",
            "reason": "Useful later",
            "scope": "team"
        }),
        &ToolContext::new("."),
    );
    assert!(bad_scope.is_error);
    assert_eq!(bad_scope.content, "scope must be project or global");

    let bad_type = MemoryWriteTool.call(
        serde_json::json!({
            "title": "Valid Title",
            "body": "Valid durable memory body.",
            "reason": "Useful later",
            "type": "transient"
        }),
        &ToolContext::new("."),
    );
    assert!(bad_type.is_error);
    assert_eq!(
        bad_type.content,
        "type must be user, feedback, project, or reference"
    );
}

#[test]
fn memory_search_tool_finds_project_and_global_memory() {
    let _guard = memory_env_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    install_memory_env(temp.path());
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");

    let project_input = serde_json::json!({
        "title": "Provider Policy",
        "body": "Use configured provider profiles instead of hard-coded endpoints.",
        "reason": "Repository convention",
        "scope": "project"
    });
    let global_input = serde_json::json!({
        "title": "Review Preference",
        "body": "Prefer concise code review findings first.",
        "reason": "Durable user preference",
        "scope": "global"
    });
    assert!(
        !MemoryWriteTool
            .call(project_input, &ToolContext::new(&workspace))
            .is_error
    );
    assert!(
        !MemoryWriteTool
            .call(global_input, &ToolContext::new(&workspace))
            .is_error
    );

    let all = MemorySearchTool.call(
        serde_json::json!({"query": "preference", "scope": "all"}),
        &ToolContext::new(&workspace),
    );
    assert!(!all.is_error);
    assert!(all.content.contains("Review Preference"));
    assert!(all.content.contains("scope: global"));

    let project = MemorySearchTool.call(
        serde_json::json!({"query": "provider", "scope": "project", "max_results": 5}),
        &ToolContext::new(&workspace),
    );
    assert!(!project.is_error);
    assert!(project.content.contains("Provider Policy"));
    assert!(project.content.contains("scope: project"));
    assert!(project.content.contains("configured provider profiles"));
}

#[test]
fn memory_search_shows_match_line_for_multi_word_query() {
    let _guard = memory_env_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    install_memory_env(temp.path());
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");

    assert!(
        !MemoryWriteTool
            .call(
                serde_json::json!({
                    "title": "Cache Layout",
                    "body": "The system prompt prefix must stay byte-stable so DeepSeek prefix caching keeps hitting across turns.",
                    "reason": "Cost lever",
                    "scope": "project"
                }),
                &ToolContext::new(&workspace),
            )
            .is_error
    );

    // The whole query is NOT a contiguous substring of the body, but several
    // terms are — BM25 ranks it, and the snippet must still surface a match line
    // (the old whole-query-substring behaviour showed none).
    let result = MemorySearchTool.call(
        serde_json::json!({"query": "byte-stable prefix caching turns", "scope": "project"}),
        &ToolContext::new(&workspace),
    );
    assert!(!result.is_error, "{}", result.content);
    assert!(result.content.contains("Cache Layout"));
    assert!(
        result.content.contains("match:"),
        "multi-word query should still surface a match snippet:\n{}",
        result.content
    );
    assert!(
        result.content.contains("byte-stable"),
        "match snippet should be the body line sharing query terms:\n{}",
        result.content
    );
}

#[test]
fn memory_search_dedups_canonical_and_legacy_copies() {
    let _guard = memory_env_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    install_memory_env(temp.path());
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");

    // Canonical copy via the tool.
    assert!(
        !MemoryWriteTool
            .call(
                serde_json::json!({
                    "title": "Provider Policy",
                    "body": "Use configured provider profiles instead of hard-coded endpoints.",
                    "reason": "Repository convention",
                    "scope": "project"
                }),
                &ToolContext::new(&workspace),
            )
            .is_error
    );

    // A legacy copy of the SAME shard filename (a half-finished migration):
    // cwd/.deeptide/memory/<same-file>. Both dirs are searched, so without
    // (scope, file name) de-dup the one memory would surface twice.
    let canonical_dir = deeptide_core::memory::MemorySystem::project_memory_dir(&workspace);
    let shard = std::fs::read_dir(&canonical_dir)
        .expect("read canonical dir")
        .flatten()
        .map(|entry| entry.file_name())
        .find(|name| {
            let name = name.to_string_lossy();
            name.ends_with(".md") && name != "MEMORY.md"
        })
        .expect("a shard file in the canonical dir");
    let legacy_dir = workspace.join(".deeptide").join("memory");
    std::fs::create_dir_all(&legacy_dir).expect("legacy dir");
    std::fs::copy(canonical_dir.join(&shard), legacy_dir.join(&shard)).expect("copy to legacy");

    let result = MemorySearchTool.call(
        serde_json::json!({"query": "provider", "scope": "project", "max_results": 10}),
        &ToolContext::new(&workspace),
    );
    assert!(!result.is_error, "{}", result.content);
    assert_eq!(
        result.content.matches("scope: project").count(),
        1,
        "the same memory present in both canonical and legacy dirs must surface \
         once, not twice:\n{}",
        result.content
    );
}

#[test]
fn brief_tool_requests_context_compaction() {
    let result = BriefTool.call(serde_json::json!({}), &ToolContext::new("."));

    assert!(!result.is_error);
    assert!(result.content.contains("Context compaction triggered"));
    assert!(result.content.contains("Summarize older messages"));
    assert!(result.content.contains("Continue your task"));
}

#[test]
fn ctx_inspect_tool_reports_context_budget_and_warnings() {
    let result = CtxInspectTool.call(
        serde_json::json!({
            "model": "deepseek-v4-flash",
            "estimated_tokens": 470000,
            "message_count": 42
        }),
        &ToolContext::new("."),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("Context Window Report:"));
    assert!(result.content.contains("Model: deepseek-v4-flash"));
    assert!(result.content.contains("Context window: 512.0K tokens"));
    assert!(result.content.contains("Active messages: 42"));
    assert!(result.content.contains("CRITICAL: Context at 91%"));
}

#[test]
fn ctx_inspect_tool_matches_swift_model_context_windows() {
    let cases = [
        ("deepseek-v4-flash-q4k", "1.0M tokens"),
        ("Deviad/DeepSeek-V4-Flash-MLX-Q4Q8", "1.0M tokens"),
        ("deepseek-v4-pro", "1.0M tokens"),
        ("mlx-community/Qwen3-Coder-Next-8bit", "262.1K tokens"),
        ("mlx-community/Qwen3.6-35B-A3B-4bit", "262.1K tokens"),
        ("glm-4.7-flash", "131.1K tokens"),
        ("deepseek-chat", "128.0K tokens"),
        ("claude-3.5-sonnet", "200.0K tokens"),
        ("gemini-1.5-pro", "1.0M tokens"),
    ];

    for (model, expected_window) in cases {
        let result = CtxInspectTool.call(
            serde_json::json!({
                "model": model,
                "estimated_tokens": 1,
                "message_count": 1
            }),
            &ToolContext::new("."),
        );

        assert!(!result.is_error, "{model} should inspect successfully");
        assert!(
            result
                .content
                .contains(&format!("Context window: {expected_window}")),
            "{model} should report {expected_window}, got:\n{}",
            result.content
        );
    }
}

#[test]
fn snip_tool_formats_history_trim_request() {
    let result = SnipTool.call(
        serde_json::json!({"keepLast": 200, "explanation": "Older build logs are no longer needed."}),
        &ToolContext::new("."),
    );

    assert!(!result.is_error);
    assert!(
        result
            .content
            .contains("History trim requested: keeping last 100 messages.")
    );
    assert!(
        result
            .content
            .contains("Reason: Older build logs are no longer needed.")
    );
}

#[test]
fn plan_mode_tools_return_approval_flow_text() {
    let enter = EnterPlanModeTool.call(serde_json::json!({}), &ToolContext::new("."));
    assert!(!enter.is_error);
    assert!(enter.content.contains("Plan mode activated"));
    assert!(enter.content.contains("Do NOT modify project files"));

    let exit = ExitPlanModeTool.call(
        serde_json::json!({
            "allowedPrompts": [
                {"tool": "Bash", "prompt": "Run cargo test"}
            ]
        }),
        &ToolContext::new("."),
    );
    assert!(!exit.is_error);
    assert!(exit.content.contains("Plan is ready for review"));
    assert!(exit.content.contains("- Bash: Run cargo test"));
}

#[test]
fn exit_plan_mode_echoes_zero_cli_plan_metadata() {
    let exit = ExitPlanModeTool.call(
        serde_json::json!({
            "plan": "1. Inspect code\n2. Run tests",
            "planFilePath": "/tmp/plan.md",
            "planWasEdited": true,
            "allowedPrompts": [
                {"tool": "Bash", "prompt": "Run cargo test"}
            ]
        }),
        &ToolContext::new("."),
    );

    assert!(!exit.is_error);
    assert!(exit.content.contains("Plan file: /tmp/plan.md"));
    assert!(exit.content.contains("Plan was edited before approval."));
    assert!(
        exit.content
            .contains("Plan:\n1. Inspect code\n2. Run tests")
    );
    assert!(exit.content.contains("- Bash: Run cargo test"));
    assert!(
        !exit
            .content
            .contains("The plan has been written to the plan file")
    );
}

#[test]
fn clipboard_tool_validates_operation_and_write_content() {
    let missing_operation = ClipboardTool.call(serde_json::json!({}), &ToolContext::new("."));
    assert!(missing_operation.is_error);
    assert!(
        missing_operation
            .content
            .contains("operation must be one of")
    );

    let missing_content = ClipboardTool.call(
        serde_json::json!({"operation": "write"}),
        &ToolContext::new("."),
    );
    assert!(missing_content.is_error);
    assert_eq!(missing_content.content, "write operation requires content");
}

#[test]
fn media_transcribe_tools_validate_inputs_and_report_backend_gap() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("clip.mp3"), b"not real audio").expect("audio fixture");
    std::fs::write(temp.path().join("movie.mp4"), b"not real video").expect("video fixture");
    std::fs::write(temp.path().join("notes.txt"), b"text").expect("text fixture");
    let context = ToolContext::new(temp.path());

    let missing_audio = AudioTranscribeTool.call(serde_json::json!({}), &context);
    assert!(missing_audio.is_error);
    assert_eq!(missing_audio.content, "file_path is required");

    let unsupported =
        AudioTranscribeTool.call(serde_json::json!({"file_path": "notes.txt"}), &context);
    assert!(unsupported.is_error);
    assert!(unsupported.content.contains("Unsupported audio format"));

    let audio = AudioTranscribeTool.call(
        serde_json::json!({"file_path": "clip.mp3", "language_hint": "en-US"}),
        &context,
    );
    assert!(audio.is_error);
    assert!(audio.content.contains("[AudioTranscribe] clip.mp3"));
    assert!(audio.content.contains("Transcription backend unavailable"));
    assert!(audio.content.contains("Language hint: en-US"));

    let video = VideoTranscribeTool.call(
        serde_json::json!({"file_path": "movie.mp4", "allow_server": true}),
        &context,
    );
    assert!(video.is_error);
    assert!(video.content.contains("[VideoTranscribe] movie.mp4"));
    assert!(
        video
            .content
            .contains("Recognition mode: local or server fallback allowed by input")
    );
    assert!(video.content.contains("Visual frames: not analyzed"));
}

#[test]
fn spotlight_search_tool_validates_inputs_and_platform_fallback() {
    let context = ToolContext::new(".");
    let missing_query = SpotlightSearchTool.call(serde_json::json!({}), &context);
    assert!(missing_query.is_error);
    assert_eq!(missing_query.content, "query is required");

    let empty_query = SpotlightSearchTool.call(serde_json::json!({"query": "   "}), &context);
    assert!(empty_query.is_error);
    assert_eq!(empty_query.content, "query is required");

    let invalid_limit = SpotlightSearchTool.call(
        serde_json::json!({"query": "Package", "max_results": 0}),
        &context,
    );
    assert!(invalid_limit.is_error);
    assert_eq!(invalid_limit.content, "max_results must be >= 1");

    #[cfg(not(target_os = "macos"))]
    {
        let result = SpotlightSearchTool.call(serde_json::json!({"query": "Package"}), &context);
        assert!(result.is_error);
        assert!(result.content.contains("only available on macOS"));
    }
}

#[test]
fn screen_capture_tool_validates_swift_shape_and_platform_fallback() {
    let context = ToolContext::new(".");
    let missing_operation = ScreenCaptureTool.call(serde_json::json!({}), &context);
    assert!(missing_operation.is_error);
    assert_eq!(
        missing_operation.content,
        "operation must be \"list\" or \"capture\""
    );

    let bad_operation =
        ScreenCaptureTool.call(serde_json::json!({"operation": "record"}), &context);
    assert!(bad_operation.is_error);
    assert_eq!(
        bad_operation.content,
        "operation must be \"list\" or \"capture\""
    );

    let missing_target =
        ScreenCaptureTool.call(serde_json::json!({"operation": "capture"}), &context);
    assert!(missing_target.is_error);
    assert_eq!(
        missing_target.content,
        "capture requires app_name or window_id"
    );

    #[cfg(not(target_os = "macos"))]
    {
        let result = ScreenCaptureTool.call(serde_json::json!({"operation": "list"}), &context);
        assert!(result.is_error);
        assert!(result.content.contains("only available on macOS"));
    }
}

#[test]
fn lsp_tool_validates_operation_file_and_line_before_server_lookup() {
    let context = ToolContext::new(".");
    let missing_operation = LspTool.call(serde_json::json!({}), &context);
    assert!(missing_operation.is_error);
    assert!(
        missing_operation
            .content
            .contains("operation must be one of")
    );

    let missing_file = LspTool.call(
        serde_json::json!({"operation": "hover", "line": 1}),
        &context,
    );
    assert!(missing_file.is_error);
    assert_eq!(missing_file.content, "file_path is required");

    let missing_line = LspTool.call(
        serde_json::json!({"operation": "documentSymbol", "file_path": "src/lib.rs"}),
        &context,
    );
    assert!(missing_line.is_error);
    assert_eq!(missing_line.content, "line is required");
}

#[test]
fn image_preprocess_tool_inspects_and_preprocesses_local_images() {
    let temp = tempfile::tempdir().expect("tempdir");
    let image_path = temp.path().join("sample.png");
    let mut image = image::RgbaImage::from_pixel(32, 24, image::Rgba([255, 255, 255, 255]));
    for y in 8..16 {
        for x in 10..22 {
            image.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    image.save(&image_path).expect("save fixture");

    let context = ToolContext::new(temp.path());
    let inspect = ImagePreprocessTool.call(
        serde_json::json!({"file_path": "sample.png", "operation": "inspect"}),
        &context,
    );
    assert!(!inspect.is_error);
    assert!(
        inspect
            .content
            .contains("[ImagePreprocess.inspect] sample.png")
    );
    assert!(inspect.content.contains("size: 32x24"));
    assert!(inspect.content.contains("likely_blank: false"));
    assert!(inspect.content.contains("content_box: x="));

    let preprocess = ImagePreprocessTool.call(
        serde_json::json!({
            "file_path": "sample.png",
            "operation": "preprocess",
            "auto_trim": true,
            "max_dimension": 16,
            "format": "png"
        }),
        &context,
    );
    assert!(!preprocess.is_error);
    assert!(
        preprocess
            .content
            .contains("[ImagePreprocess.preprocess] sample.png")
    );
    assert!(preprocess.content.contains("steps: auto_trim"));
    assert!(preprocess.content.contains("format: image/png"));
    assert!(preprocess.content.contains("image_base64:"));
}

#[test]
fn vision_tool_classifies_local_images_and_validates_input() {
    let temp = tempfile::tempdir().expect("tempdir");
    let image_path = temp.path().join("sample.png");
    let mut image = image::RgbaImage::from_pixel(32, 24, image::Rgba([255, 255, 255, 255]));
    for y in 8..16 {
        for x in 10..22 {
            image.put_pixel(x, y, image::Rgba([0, 0, 0, 255]));
        }
    }
    image.save(&image_path).expect("save fixture");

    let context = ToolContext::new(temp.path());
    let classify = VisionTool.call(
        serde_json::json!({"file_path": "sample.png", "operation": "classify"}),
        &context,
    );
    assert!(!classify.is_error);
    assert!(classify.content.contains("[Vision.classify] sample.png"));
    assert!(classify.content.contains("size: 32x24"));
    assert!(classify.content.contains("labels: image"));
    assert!(classify.content.contains("likely_blank: false"));
    assert!(classify.content.contains("content_box: x="));

    let invalid_operation = VisionTool.call(
        serde_json::json!({"file_path": "sample.png", "operation": "inspect"}),
        &context,
    );
    assert!(invalid_operation.is_error);
    assert_eq!(
        invalid_operation.content,
        "operation must be one of: ocr, layout, classify"
    );

    let invalid_languages = VisionTool.call(
        serde_json::json!({
            "file_path": "sample.png",
            "operation": "classify",
            "language_hints": "eng"
        }),
        &context,
    );
    assert!(invalid_languages.is_error);
    assert_eq!(
        invalid_languages.content,
        "language_hints must be an array of strings"
    );

    let missing = VisionTool.call(
        serde_json::json!({"file_path": "missing.png", "operation": "classify"}),
        &context,
    );
    assert!(missing.is_error);
    assert!(missing.content.contains("File not found:"));
}

#[test]
fn mac_diagnostic_tools_validate_inputs_and_render_guidance() {
    let context = ToolContext::new(".");

    let crash_missing_path = CrashLogTool.call(serde_json::json!({"operation": "read"}), &context);
    assert!(crash_missing_path.is_error);
    assert_eq!(
        crash_missing_path.content,
        "file_path is required for operation=read"
    );

    let invalid_log_level = MacLogTool.call(serde_json::json!({"level": "verbose"}), &context);
    assert!(invalid_log_level.is_error);
    assert!(invalid_log_level.content.contains("level must be"));

    let diagnose = MacDiagnoseTool.call(
        serde_json::json!({"scenario": "crash", "app_name": "Tide"}),
        &context,
    );
    assert!(!diagnose.is_error);
    assert!(
        diagnose
            .content
            .contains("[MacDiagnose] scenario=crash app=Tide")
    );
    assert!(diagnose.content.contains("1. CrashLog list app_name: Tide"));
    assert!(diagnose.content.contains("2. MacLog process: Tide"));
}

#[test]
fn cron_tools_create_list_and_delete_jobs() {
    let context = ToolContext::new(".");
    let create = CronCreateTool.call(
        serde_json::json!({"cron": "*/5 * * * *", "prompt": "collect news"}),
        &context,
    );
    assert!(!create.is_error);
    assert!(create.content.contains("Recurring task"));
    assert!(create.content.contains("every 5 minutes"));
    assert!(create.content.contains("Permission mode switched to YOLO"));
    let id = create
        .content
        .split_whitespace()
        .nth(2)
        .expect("job id")
        .to_owned();

    let list = CronListTool.call(serde_json::json!({}), &context);
    assert!(!list.is_error);
    assert!(
        list.content
            .contains(&format!("[{id}] Recurring: every 5 minutes"))
    );
    assert!(list.content.contains("Prompt: collect news"));

    let delete = CronDeleteTool.call(serde_json::json!({"id": id}), &context);
    assert!(!delete.is_error);
    assert!(delete.content.contains("deleted"));
}

#[test]
fn cron_create_rejects_malformed_expressions_and_empty_prompts() {
    let context = ToolContext::new(".");
    let bad_cron = CronCreateTool.call(
        serde_json::json!({"cron": "/5 * * *", "prompt": "collect news"}),
        &context,
    );
    assert!(bad_cron.is_error);
    assert!(bad_cron.content.contains("5-field"));
    assert!(bad_cron.content.contains("*/5 * * * *"));

    let missing_prompt = CronCreateTool.call(
        serde_json::json!({"cron": "*/5 * * * *", "prompt": ""}),
        &context,
    );
    assert!(missing_prompt.is_error);
    assert_eq!(missing_prompt.content, "prompt is required");
}

#[test]
fn review_artifact_marks_resolved_workspace_path_with_reason() {
    let temp = tempfile::tempdir().expect("tempdir");
    let context = ToolContext::new(temp.path());
    let result = ReviewArtifactTool.call(
        serde_json::json!({"file_path": "src/lib.rs", "reason": "check edge case"}),
        &context,
    );

    assert!(!result.is_error);
    assert!(result.content.contains("Marked"));
    assert!(result.content.contains("src/lib.rs"));
    assert!(result.content.contains("check edge case"));

    let missing = ReviewArtifactTool.call(serde_json::json!({}), &context);
    assert!(missing.is_error);
    assert_eq!(missing.content, "file_path is required");
}

#[test]
fn skill_tool_expands_builtin_skill_prompts_and_reports_unknown_names() {
    let context = ToolContext::new(".");
    let commit = SkillTool.call(
        serde_json::json!({"skill": "commit", "args": "-m fix"}),
        &context,
    );
    assert!(!commit.is_error);
    assert!(commit.content.contains("Skill invoked: commit"));
    assert!(commit.content.contains("[SKILL PROMPT START]"));
    assert!(commit.content.contains("git diff --cached"));
    assert!(commit.content.contains("-m fix"));

    let unknown = SkillTool.call(serde_json::json!({"skill": "missing"}), &context);
    assert!(unknown.is_error);
    assert!(unknown.content.contains("Unknown skill: missing"));
    assert!(unknown.content.contains("commit"));
    assert!(unknown.content.contains("publish"));

    // The init/batch/publish/update-config prompts carry Swift's detailed
    // multi-phase guidance, not the earlier condensed summaries.
    let init = SkillTool.call(serde_json::json!({"skill": "init"}), &context);
    assert!(!init.is_error);
    assert!(init.content.contains("Phase 1 - explore"));
    assert!(init.content.contains("keep total length 60-120 lines"));

    let batch = SkillTool.call(serde_json::json!({"skill": "batch"}), &context);
    assert!(!batch.is_error);
    assert!(batch.content.contains("subagent_type: Explore"));
    assert!(batch.content.contains(".deeptide-worktrees/"));

    let update_config = SkillTool.call(serde_json::json!({"skill": "update-config"}), &context);
    assert!(!update_config.is_error);
    assert!(update_config.content.contains("default: 4096"));
    assert!(
        update_config
            .content
            .contains("permission_mode: default, accept-edits, plan, bypass")
    );
}

#[test]
fn discover_skills_lists_every_builtin_skill_as_json() {
    use deeptide_core::tools::DiscoverSkillsTool;
    let context = ToolContext::new(".");
    let result = DiscoverSkillsTool.call(serde_json::json!({}), &context);
    assert!(!result.is_error);

    let parsed: serde_json::Value =
        serde_json::from_str(&result.content).expect("DiscoverSkills output must be valid JSON");
    let count = parsed
        .get("count")
        .and_then(serde_json::Value::as_u64)
        .expect("count field");
    assert!(
        count >= 7,
        "expected at least the 7 known skills; got {count}"
    );

    let skills = parsed
        .get("skills")
        .and_then(serde_json::Value::as_array)
        .expect("skills array");
    let names: Vec<&str> = skills
        .iter()
        .filter_map(|s| s.get("name").and_then(serde_json::Value::as_str))
        .collect();
    // Every shipped built-in must show up.
    for required in [
        "commit",
        "simplify",
        "review-pr",
        "init",
        "batch",
        "publish",
        "update-config",
    ] {
        assert!(
            names.contains(&required),
            "DiscoverSkills missing skill '{required}'; got: {names:?}"
        );
    }

    // The when_to_use field is optional in the schema and must only appear
    // on skills that actually carry guidance — `publish` is the canonical
    // example.
    let publish = skills
        .iter()
        .find(|s| s.get("name").and_then(serde_json::Value::as_str) == Some("publish"))
        .expect("publish skill in catalog");
    assert!(
        publish
            .get("when_to_use")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|hint| hint.contains("publish")),
        "publish skill must carry when_to_use guidance"
    );

    let commit = skills
        .iter()
        .find(|s| s.get("name").and_then(serde_json::Value::as_str) == Some("commit"))
        .expect("commit skill");
    assert!(
        commit.get("when_to_use").is_none(),
        "commit skill has no when_to_use; the field must be omitted"
    );

    let hint = parsed
        .get("invocation_hint")
        .and_then(serde_json::Value::as_str)
        .expect("invocation_hint present");
    assert!(hint.contains("Skill(") && hint.contains("\"skill\""));
}

#[test]
fn discover_skills_ignores_input_arguments() {
    use deeptide_core::tools::DiscoverSkillsTool;
    let context = ToolContext::new(".");
    let with_args = DiscoverSkillsTool.call(
        serde_json::json!({"unused": 42, "skill": "noise"}),
        &context,
    );
    assert!(!with_args.is_error);
    // The schema enforces additionalProperties:false on the wire, but the
    // local implementation must not break if extras sneak through.
    let parsed: serde_json::Value = serde_json::from_str(&with_args.content).expect("json");
    assert!(parsed.get("skills").is_some());
}

#[test]
fn publish_tool_renders_status_and_validates_option_conflicts() {
    // PublishTool reads PAEAN_API_* / CLIDE_API_* env vars during dispatch.
    // Other publish_tool_* tests mutate those vars under `publish_env_guard`,
    // and concurrent read+write on std::env is undefined behaviour, so this
    // read-only call must take the same lock to avoid corrupting the race.
    let _guard = publish_env_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    let context = ToolContext::new(temp.path());

    let status = PublishTool.call(serde_json::json!({"status": true}), &context);
    assert!(!status.is_error);
    assert!(
        status
            .content
            .contains("No saved clide.app publish state at .clide/publish.json")
    );

    let conflict = PublishTool.call(
        serde_json::json!({"status": true, "dry_run": true}),
        &context,
    );
    assert!(conflict.is_error);
    assert_eq!(
        conflict.content,
        "status cannot be combined with publish/delete options."
    );

    let handle_conflict = PublishTool.call(
        serde_json::json!({"random": true, "handle": "demo"}),
        &context,
    );
    assert!(handle_conflict.is_error);
    assert_eq!(
        handle_conflict.content,
        "Use either random or handle, not both."
    );
}

#[test]
fn publish_tool_dry_run_detects_static_output_and_writes_safety_ignore() {
    // PublishTool reads PAEAN_API_* / CLIDE_API_* env vars during dispatch.
    // Take the publish guard so we never observe a half-written env state
    // from sibling tests under `install_publish_env`/`clear_publish_env`.
    let _guard = publish_env_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    let dist = temp.path().join("dist");
    std::fs::create_dir_all(&dist).expect("mkdir");
    std::fs::write(dist.join("index.html"), "<h1>Hello</h1>").expect("index");
    std::fs::write(dist.join("app.js"), "console.log('ok');").expect("asset");
    std::fs::write(dist.join("app.js.map"), "{}").expect("sourcemap");
    std::fs::create_dir_all(dist.join("node_modules/pkg")).expect("node_modules");
    std::fs::write(dist.join("node_modules/pkg/private.js"), "ignored").expect("ignored");

    let result = PublishTool.call(
        serde_json::json!({"dry_run": true, "handle": "demo"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error, "{}", result.content);
    assert!(result.content.contains("Publish dry run: ready"));
    assert!(result.content.contains("Directory:  dist"));
    assert!(result.content.contains("Handle:     demo"));
    assert!(result.content.contains("Index:      yes"));
    assert!(!result.content.contains("app.js.map"));
    assert!(!result.content.contains("Source maps are included"));
    assert!(!result.content.contains("private.js"));

    let ignore = std::fs::read_to_string(temp.path().join(".clideignore")).expect("ignore");
    assert!(ignore.contains("# Added by Clide publish safety defaults"));
    assert!(ignore.contains(".env"));
    assert!(ignore.contains("node_modules/"));
}

#[test]
fn publish_tool_reports_missing_output_and_missing_auth() {
    let _guard = publish_env_guard();
    clear_publish_env();
    let temp = tempfile::tempdir().expect("tempdir");
    let missing = PublishTool.call(
        serde_json::json!({"dry_run": true}),
        &ToolContext::new(temp.path()),
    );
    assert!(missing.is_error);
    assert!(
        missing
            .content
            .contains("No publishable static directory found")
    );

    let public = temp.path().join("public");
    std::fs::create_dir_all(&public).expect("mkdir public");
    std::fs::write(public.join("index.html"), "<html></html>").expect("index");
    let upload = PublishTool.call(serde_json::json!({}), &ToolContext::new(temp.path()));
    assert!(upload.is_error);
    assert!(upload.content.contains("Paean login is missing or expired"));
}

#[test]
fn publish_tool_uploads_archive_and_saves_state() {
    let _guard = publish_env_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    let public = temp.path().join("public");
    std::fs::create_dir_all(&public).expect("mkdir public");
    std::fs::write(public.join("index.html"), "<html></html>").expect("index");
    std::fs::write(public.join("app.js"), "console.log('ok');").expect("asset");

    let base_url = serve_publish_sequence(vec![
        (200, r#"{"success":true}"#),
        (
            200,
            r#"{"success":true,"data":{"handle":"demo","assignedHandle":null,"url":"https://demo.clide.app","shortUrl":"https://demo.clide.app","fileCount":2,"totalBytes":31,"overwritten":false,"archiveUrl":"https://example.com/site.zip"}}"#,
        ),
    ]);
    install_publish_env(&base_url);

    let result = PublishTool.call(
        serde_json::json!({"handle": "demo"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error, "{}", result.content);
    assert!(result.content.contains("Published: https://demo.clide.app"));
    assert!(result.content.contains("Handle:     demo"));
    let state = std::fs::read_to_string(temp.path().join(".clide/publish.json")).expect("state");
    assert!(state.contains("\"handle\": \"demo\""));
    assert!(state.contains("\"url\": \"https://demo.clide.app\""));
}

#[test]
fn publish_tool_deletes_saved_handle_and_marks_state() {
    let _guard = publish_env_guard();
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join(".clide")).expect("mkdir .clide");
    std::fs::write(
        temp.path().join(".clide/publish.json"),
        r#"{"handle":"demo","url":"https://demo.clide.app"}"#,
    )
    .expect("state");
    let base_url = serve_publish_sequence(vec![
        (200, r#"{"success":true}"#),
        (
            200,
            r#"{"success":true,"handle":"demo","deletedObjects":3}"#,
        ),
    ]);
    install_publish_env(&base_url);

    let result = PublishTool.call(
        serde_json::json!({"delete": true}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error, "{}", result.content);
    assert!(
        result
            .content
            .contains("Deleted remote publish: demo.clide.app")
    );
    assert!(result.content.contains("Deleted objects: 3"));
    let state = std::fs::read_to_string(temp.path().join(".clide/publish.json")).expect("state");
    assert!(state.contains("\"lastDeletedHandle\": \"demo\""));
    assert!(!state.contains("\"handle\": \"demo\""));
}

#[test]
fn remote_trigger_tool_posts_configured_payload() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (url, request_rx) = serve_remote_trigger(202, r#"{"accepted":true}"#);
    std::fs::create_dir_all(temp.path().join(".deeptide")).expect("mkdir config");
    std::fs::write(
        temp.path().join(".deeptide/settings.json"),
        format!(
            r#"{{
                "remote_trigger": {{
                    "url": "{url}",
                    "token": "fixture-token",
                    "headers": {{"X-Deeptide-Test": "yes"}}
                }}
            }}"#
        ),
    )
    .expect("settings");

    let result = RemoteTriggerTool.call(
        serde_json::json!({"payload": "deploy docs"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error, "{}", result.content);
    assert!(result.content.contains("HTTP 202"));
    assert!(result.content.contains(r#"{"accepted":true}"#));
    let request = request_rx.recv().expect("captured request");
    let lower_request = request.to_ascii_lowercase();
    assert!(request.contains("POST /hook HTTP/1.1"));
    assert!(lower_request.contains("authorization: bearer fixture-token"));
    assert!(lower_request.contains("x-deeptide-test: yes"));
    assert!(request.contains(r#"{"payload":"deploy docs"}"#));
}

#[test]
fn remote_trigger_and_push_notification_validate_inputs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let context = ToolContext::new(temp.path());

    let missing_payload = RemoteTriggerTool.call(serde_json::json!({}), &context);
    assert!(missing_payload.is_error);
    assert_eq!(missing_payload.content, "payload is required");

    let missing_config = RemoteTriggerTool.call(serde_json::json!({"payload": "deploy"}), &context);
    assert!(missing_config.is_error);
    assert_eq!(
        missing_config.content,
        "Remote trigger not configured. Add settings.remote_trigger.url."
    );

    let missing_message = PushNotificationTool.call(serde_json::json!({}), &context);
    assert!(missing_message.is_error);
    assert_eq!(missing_message.content, "message is required");

    let too_long =
        PushNotificationTool.call(serde_json::json!({"message": "x".repeat(501)}), &context);
    assert!(too_long.is_error);
    assert_eq!(too_long.content, "message must be <= 500 chars (got 501)");
}

#[test]
fn edge_tools_edit_notebooks_sleep_and_verify_changes() {
    let temp = tempfile::tempdir().expect("tempdir");
    init_git_fixture(temp.path());
    let notebook_path = temp.path().join("analysis.ipynb");
    std::fs::write(
        &notebook_path,
        r#"{
  "cells": [
    {
      "id": "cell-1",
      "cell_type": "code",
      "source": ["print('old')"],
      "metadata": {},
      "outputs": [],
      "execution_count": null
    }
  ],
  "metadata": {},
  "nbformat": 4,
  "nbformat_minor": 5
}"#,
    )
    .expect("write notebook");

    let context = ToolContext::new(temp.path());
    let replace = NotebookEditTool.call(
        serde_json::json!({
            "notebook_path": "analysis.ipynb",
            "cell_id": "cell-1",
            "new_source": "print('new')",
            "cell_type": "code"
        }),
        &context,
    );
    assert!(!replace.is_error, "{}", replace.content);
    assert_eq!(replace.content, "Cell cell-1 replaced.");
    let updated = std::fs::read_to_string(&notebook_path).expect("updated notebook");
    assert!(updated.contains("print('new')"));

    let insert = NotebookEditTool.call(
        serde_json::json!({
            "notebook_path": "analysis.ipynb",
            "cell_id": "cell-1",
            "edit_mode": "insert",
            "cell_type": "markdown",
            "new_source": "# Notes"
        }),
        &context,
    );
    assert!(!insert.is_error, "{}", insert.content);
    assert_eq!(insert.content, "Cell inserted at position 1.");

    let sleep = SleepTool.call(serde_json::json!({"duration_ms": 0}), &context);
    assert!(!sleep.is_error);
    assert_eq!(sleep.content, "Slept 0 ms");

    let verify = VerifyPlanExecutionTool.call(
        serde_json::json!({"expected_files": ["analysis.ipynb"]}),
        &context,
    );
    assert!(!verify.is_error, "{}", verify.content);
    assert!(verify.content.contains("Plan Verification Report:"));
    assert!(verify.content.contains("[NEW] analysis.ipynb"));
    assert!(verify.content.contains("OK analysis.ipynb"));
    assert!(verify.content.contains("All expected changes verified."));
}

#[test]
fn worktree_tools_create_keep_and_remove_git_worktrees() {
    let temp = tempfile::tempdir().expect("tempdir");
    init_git_fixture(temp.path());
    let context = ToolContext::new(temp.path());

    let create = EnterWorktreeTool.call(serde_json::json!({"name": "parity-test"}), &context);
    assert!(!create.is_error, "{}", create.content);
    assert!(create.content.contains("Worktree created:"));
    assert!(create.content.contains("Branch: parity-test"));
    let worktree = temp
        .path()
        .parent()
        .expect("temp parent")
        .join(".deeptide-worktrees")
        .join("parity-test");
    assert!(worktree.exists());

    let keep = ExitWorktreeTool.call(
        serde_json::json!({"action": "keep", "path": worktree}),
        &context,
    );
    assert!(!keep.is_error, "{}", keep.content);
    assert!(keep.content.contains("Worktree kept at:"));

    let remove = ExitWorktreeTool.call(
        serde_json::json!({"action": "remove", "path": worktree}),
        &context,
    );
    assert!(!remove.is_error, "{}", remove.content);
    assert!(remove.content.contains("Worktree removed:"));
}

#[test]
fn todo_write_tool_updates_in_memory_list() {
    let _guard = todo_test_guard();
    let result = TodoWriteTool.call(
        serde_json::json!({
            "todos": [
                {"content": "Inspect Swift behavior", "status": "completed"},
                {"content": "Port Rust behavior", "status": "in_progress", "activeForm": "Porting"}
            ]
        }),
        &ToolContext::new("."),
    );

    assert!(!result.is_error);
    assert_eq!(
        result.content,
        "Todo list updated (2 items). Ensure that you continue to use the todo list to track your progress. Please proceed with the current tasks if applicable."
    );
}

#[test]
fn todo_write_tool_clears_when_all_tasks_complete() {
    let _guard = todo_test_guard();
    let result = TodoWriteTool.call(
        serde_json::json!({
            "todos": [
                {"content": "Finish", "status": "completed"}
            ]
        }),
        &ToolContext::new("."),
    );

    assert!(!result.is_error);
    assert_eq!(
        result.content,
        "Todo list cleared (all tasks completed). Proceed with your summary."
    );
}

#[test]
fn todo_write_tool_rejects_missing_todos_array() {
    let _guard = todo_test_guard();
    let result = TodoWriteTool.call(serde_json::json!({}), &ToolContext::new("."));

    assert!(result.is_error);
    assert_eq!(result.content, "Missing or invalid todos array");
}

#[test]
fn task_create_tool_adds_task_with_description() {
    let _guard = todo_test_guard();
    let _ = TodoWriteTool.call(serde_json::json!({"todos": []}), &ToolContext::new("."));

    let created = TaskCreateTool.call(
        serde_json::json!({
            "subject": "Implement task parity",
            "description": "Port TaskCreate, TaskStop, and TaskOutput"
        }),
        &ToolContext::new("."),
    );

    assert!(!created.is_error);
    assert_eq!(created.content, "Task created: Implement task parity");

    let detail = TaskGetTool.call(serde_json::json!({"taskId": "1"}), &ToolContext::new("."));
    assert_eq!(
        detail.content,
        "Task: Implement task parity\nID: 1\nStatus: pending\nDescription: Port TaskCreate, TaskStop, and TaskOutput"
    );
}

#[test]
fn task_create_tool_requires_subject_and_description() {
    let _guard = todo_test_guard();
    let result = TaskCreateTool.call(
        serde_json::json!({"subject": "Missing details"}),
        &ToolContext::new("."),
    );

    assert!(result.is_error);
    assert_eq!(result.content, "Missing subject or description");
}

#[test]
fn task_list_tool_lists_current_todos() {
    let _guard = todo_test_guard();
    let _ = TodoWriteTool.call(
        serde_json::json!({
            "todos": [
                {"content": "Plan work", "status": "completed"},
                {"content": "Implement work", "status": "in_progress"},
                {"content": "Verify work", "status": "pending"}
            ]
        }),
        &ToolContext::new("."),
    );

    let result = TaskListTool.call(serde_json::json!({}), &ToolContext::new("."));

    assert!(!result.is_error);
    assert_eq!(
        result.content,
        "#1 ⌬ Plan work\n#2 ◉ Implement work\n#3 ○ Verify work"
    );
}

#[test]
fn task_list_tool_reports_empty_list() {
    let _guard = todo_test_guard();
    let _ = TodoWriteTool.call(serde_json::json!({"todos": []}), &ToolContext::new("."));

    let result = TaskListTool.call(serde_json::json!({}), &ToolContext::new("."));

    assert!(!result.is_error);
    assert_eq!(result.content, "No tasks.");
}

#[test]
fn task_get_tool_returns_task_details() {
    let _guard = todo_test_guard();
    let _ = TodoWriteTool.call(
        serde_json::json!({
            "todos": [
                {"content": "Plan work", "status": "completed"},
                {"content": "Implement work", "status": "in_progress", "activeForm": "Porting the task"}
            ]
        }),
        &ToolContext::new("."),
    );

    let result = TaskGetTool.call(serde_json::json!({"taskId": "2"}), &ToolContext::new("."));

    assert!(!result.is_error);
    assert_eq!(
        result.content,
        "Task: Implement work\nID: 2\nStatus: in_progress\nDescription: Porting the task"
    );
}

#[test]
fn task_get_tool_reports_missing_and_unknown_ids() {
    let _guard = todo_test_guard();
    let missing = TaskGetTool.call(serde_json::json!({}), &ToolContext::new("."));
    assert!(missing.is_error);
    assert_eq!(missing.content, "Missing taskId parameter");

    let unknown = TaskGetTool.call(serde_json::json!({"taskId": "99"}), &ToolContext::new("."));
    assert!(unknown.is_error);
    assert_eq!(unknown.content, "Task not found: 99");
}

#[test]
fn task_update_tool_updates_status_subject_and_description() {
    let _guard = todo_test_guard();
    let _ = TodoWriteTool.call(
        serde_json::json!({
            "todos": [
                {"content": "Implement work", "status": "pending"}
            ]
        }),
        &ToolContext::new("."),
    );

    let result = TaskUpdateTool.call(
        serde_json::json!({
            "taskId": "1",
            "status": "in_progress",
            "subject": "Implement Rust task update",
            "description": "Updating task metadata"
        }),
        &ToolContext::new("."),
    );

    assert!(!result.is_error);
    assert_eq!(
        result.content,
        "Task #1 updated: status -> in_progress, subject updated, description updated"
    );

    let detail = TaskGetTool.call(serde_json::json!({"taskId": "1"}), &ToolContext::new("."));
    assert_eq!(
        detail.content,
        "Task: Implement Rust task update\nID: 1\nStatus: in_progress\nDescription: Updating task metadata"
    );
}

#[test]
fn task_update_tool_deletes_tasks_and_reports_no_changes() {
    let _guard = todo_test_guard();
    let _ = TodoWriteTool.call(
        serde_json::json!({
            "todos": [
                {"content": "Remove me", "status": "pending"}
            ]
        }),
        &ToolContext::new("."),
    );

    let deleted = TaskUpdateTool.call(
        serde_json::json!({"taskId": "1", "status": "deleted"}),
        &ToolContext::new("."),
    );
    assert!(!deleted.is_error);
    assert_eq!(deleted.content, "Task #1 deleted");

    let unchanged = TaskUpdateTool.call(serde_json::json!({"taskId": "1"}), &ToolContext::new("."));
    assert!(!unchanged.is_error);
    assert_eq!(
        unchanged.content,
        "Task #1: no changes (task may not exist)"
    );
}

#[test]
fn task_stop_tool_marks_tasks_completed() {
    let _guard = todo_test_guard();
    let _ = TodoWriteTool.call(
        serde_json::json!({
            "todos": [
                {"content": "Run verification", "status": "in_progress", "activeForm": "Running cargo test"}
            ]
        }),
        &ToolContext::new("."),
    );

    let stopped = TaskStopTool.call(
        serde_json::json!({"taskId": "1", "explanation": "Verification finished"}),
        &ToolContext::new("."),
    );

    assert!(!stopped.is_error);
    assert_eq!(stopped.content, "Task stopped: Verification finished");

    let detail = TaskGetTool.call(serde_json::json!({"taskId": "1"}), &ToolContext::new("."));
    assert!(detail.content.contains("Status: completed"));
}

#[test]
fn task_output_tool_returns_task_metadata_as_json() {
    let _guard = todo_test_guard();
    let _ = TodoWriteTool.call(
        serde_json::json!({
            "todos": [
                {"content": "Summarize work", "status": "completed", "activeForm": "Writing summary"}
            ]
        }),
        &ToolContext::new("."),
    );

    let output = TaskOutputTool.call(
        serde_json::json!({"task_id": "1", "block": false}),
        &ToolContext::new("."),
    );

    assert!(!output.is_error);
    let value: serde_json::Value = serde_json::from_str(&output.content).expect("json");
    assert_eq!(value["retrieval_status"], "success");
    assert_eq!(value["task"]["task_id"], "1");
    assert_eq!(value["task"]["status"], "completed");
    assert_eq!(value["task"]["description"], "Writing summary");
    assert_eq!(value["task"]["output"], "Summarize work");
}

#[test]
fn task_output_tool_reports_missing_tasks_as_not_ready() {
    let _guard = todo_test_guard();
    let _ = TodoWriteTool.call(serde_json::json!({"todos": []}), &ToolContext::new("."));

    let output = TaskOutputTool.call(serde_json::json!({"task_id": "99"}), &ToolContext::new("."));

    assert!(output.is_error);
    let value: serde_json::Value = serde_json::from_str(&output.content).expect("json");
    assert_eq!(value["retrieval_status"], "not_ready");
    assert!(value["note"].as_str().unwrap_or_default().contains("99"));
}

#[test]
fn glob_tool_finds_matching_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("src/nested")).expect("mkdir");
    std::fs::write(temp.path().join("src/lib.rs"), "").expect("write");
    std::fs::write(temp.path().join("src/nested/mod.rs"), "").expect("write");
    std::fs::write(temp.path().join("README.md"), "").expect("write");

    let result = ToolRegistry::with_builtin_tools().call(
        "Glob",
        serde_json::json!({"pattern": "**/*.rs"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("src/lib.rs"));
    assert!(result.content.contains("src/nested/mod.rs"));
    assert!(!result.content.contains("README.md"));
}

#[test]
fn glob_tool_excludes_sensitive_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "").expect("write notes");
    std::fs::write(temp.path().join(".env"), "API_KEY=secret\n").expect("write .env");

    let result = ToolRegistry::with_builtin_tools().call(
        "Glob",
        serde_json::json!({"pattern": "**/*"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("notes.txt"));
    assert!(
        !result.content.contains(".env"),
        "Glob must not reveal sensitive file paths: {}",
        result.content
    );
}

#[test]
fn write_tool_blocks_sensitive_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let result = ToolRegistry::with_builtin_tools().call(
        "Write",
        serde_json::json!({"file_path": ".env", "content": "API_KEY=leaked\n"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error, "writing a sensitive file should be denied");
    assert!(result.content.contains("sensitive"));
    assert!(result.content.contains("/open"));
    assert!(
        !temp.path().join(".env").exists(),
        "the sensitive file must not be created"
    );
}

#[test]
fn edit_tool_blocks_sensitive_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join(".env"), "API_KEY=old\n").expect("write .env");

    let result = ToolRegistry::with_builtin_tools().call(
        "Edit",
        serde_json::json!({"file_path": ".env", "old_string": "old", "new_string": "new"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error, "editing a sensitive file should be denied");
    assert!(result.content.contains("sensitive"));
    assert_eq!(
        std::fs::read_to_string(temp.path().join(".env")).expect("read"),
        "API_KEY=old\n",
        "the sensitive file must be left unmodified"
    );
}

#[test]
fn grep_tool_multiline_matches_across_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("code.rs"),
        "fn start() {}\nfn wrapped(\n    a: i32,\n) {}\nfn end() {}\n",
    )
    .expect("write");

    // This pattern spans the `fn wrapped(` line and its args, so a normal
    // per-line search would miss it.
    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({
            "pattern": r"fn wrapped\([^)]*\)",
            "path": "code.rs",
            "output_mode": "content",
            "multiline": true
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error, "{}", result.content);
    assert!(result.content.contains("code.rs:2:fn wrapped("));
    assert!(result.content.contains("code.rs:3:    a: i32,"));
    assert!(result.content.contains("code.rs:4:) {}"));
    assert!(!result.content.contains("fn start"));
    assert!(!result.content.contains("fn end"));
}

#[test]
fn grep_tool_without_multiline_does_not_span_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("code.rs"),
        "fn wrapped(\n    a: i32,\n) {}\n",
    )
    .expect("write");

    // The same cross-line pattern finds nothing in per-line (default) mode.
    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({
            "pattern": r"fn wrapped\([^)]*\)",
            "path": "code.rs",
            "output_mode": "content"
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("No matches"), "{}", result.content);
}

#[test]
fn grep_tool_multiline_respects_case_insensitive() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("code.rs"), "Alpha\nBETA\n").expect("write");

    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({
            "pattern": "beta",
            "path": "code.rs",
            "output_mode": "content",
            "multiline": true,
            "-i": true
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("code.rs:2:BETA"));
}

#[test]
fn grep_tool_finds_files_with_matches() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(temp.path().join("src")).expect("mkdir");
    std::fs::write(temp.path().join("src/lib.rs"), "pub fn hello() {}\n").expect("write");
    std::fs::write(temp.path().join("src/main.rs"), "fn main() {}\n").expect("write");

    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({"pattern": "pub fn", "glob": "**/*.rs"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("src/lib.rs"));
    assert!(!result.content.contains("src/main.rs"));
}

#[test]
fn grep_tool_excludes_sensitive_files_until_opened() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join(".env"), "API_KEY=super-secret\n").expect("write .env");
    std::fs::write(temp.path().join("app.txt"), "API_KEY=placeholder\n").expect("write app");
    let context = ToolContext::new(temp.path());
    let registry = ToolRegistry::with_builtin_tools();

    // Directory grep must not surface the secret line from .env.
    let dir = registry.call(
        "Grep",
        serde_json::json!({"pattern": "API_KEY", "output_mode": "content"}),
        &context,
    );
    assert!(
        !dir.content.contains("super-secret"),
        "secret leaked via grep"
    );
    assert!(
        dir.content.contains("placeholder"),
        "non-sensitive match still found"
    );

    // Grepping the sensitive file directly yields no matches until opened.
    let direct = registry.call(
        "Grep",
        serde_json::json!({"pattern": "API_KEY", "path": ".env", "output_mode": "content"}),
        &context,
    );
    assert!(!direct.content.contains("super-secret"));

    // After /open, the secret is searchable.
    deeptide_core::sensitive_file::mark_open(&context.resolve_path(".env"));
    let opened = registry.call(
        "Grep",
        serde_json::json!({"pattern": "API_KEY", "path": ".env", "output_mode": "content"}),
        &context,
    );
    assert!(
        opened.content.contains("super-secret"),
        "opened .env should be searchable"
    );
}

#[test]
fn grep_tool_content_mode_includes_line_numbers() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\nbeta\nalphabet\n").expect("write");

    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({"pattern": "alpha", "path": "notes.txt", "output_mode": "content"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("notes.txt:1:alpha"));
    assert!(result.content.contains("notes.txt:3:alphabet"));
}

#[test]
fn grep_tool_content_mode_shows_context_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("notes.txt"),
        "one\ntwo\nMATCH\nfour\nfive\n",
    )
    .expect("write");

    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({
            "pattern": "MATCH",
            "path": "notes.txt",
            "output_mode": "content",
            "-C": 1
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    // The match uses a ':' separator; context lines use '-'.
    assert!(
        result.content.contains("notes.txt:3:MATCH"),
        "match line: {}",
        result.content
    );
    assert!(
        result.content.contains("notes.txt-2-two"),
        "before-context: {}",
        result.content
    );
    assert!(
        result.content.contains("notes.txt-4-four"),
        "after-context: {}",
        result.content
    );
    // Lines outside the 1-line window are excluded.
    assert!(!result.content.contains("one"));
    assert!(!result.content.contains("five"));
}

#[test]
fn grep_tool_before_and_after_context_are_independent() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("notes.txt"),
        "one\ntwo\nMATCH\nfour\nfive\n",
    )
    .expect("write");

    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({
            "pattern": "MATCH",
            "path": "notes.txt",
            "output_mode": "content",
            "-B": 2,
            "-A": 0
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("notes.txt-1-one"));
    assert!(result.content.contains("notes.txt-2-two"));
    assert!(result.content.contains("notes.txt:3:MATCH"));
    assert!(!result.content.contains("four"));
}

#[test]
fn grep_tool_line_numbers_can_be_disabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\nbeta\n").expect("write");

    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({
            "pattern": "alpha",
            "path": "notes.txt",
            "output_mode": "content",
            "-n": false
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("notes.txt:alpha"));
    assert!(!result.content.contains("notes.txt:1:alpha"));
}

#[test]
fn grep_tool_type_filter_restricts_to_matching_extensions() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("keep.rs"), "needle here\n").expect("write rs");
    std::fs::write(temp.path().join("skip.txt"), "needle here\n").expect("write txt");

    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({"pattern": "needle", "type": "rust"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("keep.rs"));
    assert!(!result.content.contains("skip.txt"));
}

#[test]
fn grep_tool_unknown_type_reports_error() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("a.rs"), "x\n").expect("write");

    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({"pattern": "x", "type": "nonsense"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(result.content.contains("Unknown file type"));
}

#[test]
fn grep_tool_applies_offset_after_head_limit_for_pagination() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        temp.path().join("notes.txt"),
        "alpha one\nalpha two\nalpha three\n",
    )
    .expect("write");

    let result = ToolRegistry::with_builtin_tools().call(
        "Grep",
        serde_json::json!({
            "pattern": "alpha",
            "path": "notes.txt",
            "output_mode": "content",
            "offset": 1,
            "head_limit": 1
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(!result.content.contains("alpha one"));
    assert!(result.content.contains("notes.txt:2:alpha two"));
    assert!(!result.content.contains("alpha three"));
    assert!(result.content.contains("use offset to paginate"));
}

#[test]
fn web_fetch_tool_fetches_html_and_preserves_response_context() {
    let url = serve_once(
        200,
        "text/html; charset=utf-8",
        r#"<!doctype html>
        <html>
          <head><title>Fixture</title><style>.hidden { color: red; }</style></head>
          <body>
            <h1>Hello &amp; welcome</h1>
            <p>Read the <a href="/docs">docs</a>.</p>
            <script>window.secret = true;</script>
          </body>
        </html>"#,
    );

    let result = WebFetchTool.call(
        serde_json::json!({"url": url, "prompt": "Extract the page"}),
        &ToolContext::new("."),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("HTTP 200 |"));
    assert!(result.content.contains("Content-Type: text/html"));
    assert!(result.content.contains("Hello & welcome"));
    assert!(result.content.contains("docs ["));
    assert!(result.content.contains("/docs]"));
    assert!(!result.content.contains("window.secret"));
}

#[test]
fn web_fetch_tool_reports_invalid_urls() {
    let result = WebFetchTool.call(
        serde_json::json!({"url": "file:///etc/passwd", "prompt": "read"}),
        &ToolContext::new("."),
    );

    assert!(result.is_error);
    assert_eq!(result.content, "Invalid URL: file:///etc/passwd");
}

#[test]
fn web_search_tool_reports_missing_api_keys_with_fetch_alternatives() {
    let result = WebSearchTool.call_with_environment(
        serde_json::json!({"query": "rust cli", "allowed_domains": ["example.com"]}),
        &BTreeMap::new(),
    );

    assert!(result.is_error);
    assert!(result.content.contains("WebSearch requires an API key"));
    assert!(result.content.contains("BRAVE_SEARCH_API_KEY"));
    assert!(result.content.contains("SERPER_API_KEY"));
    assert!(
        result
            .content
            .contains("https://html.duckduckgo.com/html/?q=rust+cli")
    );
    assert!(
        result
            .content
            .contains("https://www.google.com/search?q=rust+cli")
    );
}

#[test]
fn web_search_tool_validates_query_and_domain_filter_modes() {
    let short =
        WebSearchTool.call_with_environment(serde_json::json!({"query": "r"}), &BTreeMap::new());
    assert!(short.is_error);
    assert_eq!(short.content, "query must be at least 2 characters");

    let conflicting = WebSearchTool.call_with_environment(
        serde_json::json!({
            "query": "rust",
            "allowed_domains": ["example.com"],
            "blocked_domains": ["example.org"]
        }),
        &BTreeMap::new(),
    );
    assert!(conflicting.is_error);
    assert_eq!(
        conflicting.content,
        "Cannot specify both allowed_domains and blocked_domains"
    );
}

#[test]
fn write_tool_creates_parent_directories_and_normalizes_line_endings() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = WriteTool.call(
        serde_json::json!({"file_path": "src/notes.txt", "content": "alpha\r\nbeta\rgamma\n"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("Created file: notes.txt"));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("src/notes.txt")).expect("read written file"),
        "alpha\nbeta\ngamma\n"
    );
}

#[test]
fn write_tool_reports_updated_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "old").expect("write fixture");

    let result = WriteTool.call(
        serde_json::json!({"file_path": "notes.txt", "content": "new"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("Updated file: notes.txt"));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("notes.txt")).expect("read written file"),
        "new"
    );
}

#[test]
fn write_tool_reports_missing_content() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = WriteTool.call(
        serde_json::json!({"file_path": "notes.txt"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(result.content.contains("string `content` field"));
}

#[test]
fn append_file_tool_creates_file_when_missing() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = AppendFileTool.call(
        serde_json::json!({"file_path": "out/log.txt", "content": "first chunk\n"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error, "got error: {}", result.content);
    assert!(
        result.content.contains("Created file: log.txt"),
        "unexpected: {}",
        result.content
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("out/log.txt")).expect("read"),
        "first chunk\n"
    );
}

#[test]
fn append_file_tool_appends_to_existing_file() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("doc.md"), "# Title\n\n").expect("seed");

    let result = AppendFileTool.call(
        serde_json::json!({"file_path": "doc.md", "content": "Body line 1.\n"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(
        result.content.contains("Appended to file: doc.md"),
        "unexpected: {}",
        result.content
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("doc.md")).expect("read"),
        "# Title\n\nBody line 1.\n"
    );
}

#[test]
fn append_file_tool_inserts_separator_newline_when_tail_missing() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("data.txt"), "no-newline-here").expect("seed");

    let result = AppendFileTool.call(
        serde_json::json!({"file_path": "data.txt", "content": "next\n"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert_eq!(
        std::fs::read_to_string(temp.path().join("data.txt")).expect("read"),
        "no-newline-here\nnext\n"
    );
}

#[test]
fn append_file_tool_respects_ensure_trailing_newline_false() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("data.txt"), "no-newline-here").expect("seed");

    let result = AppendFileTool.call(
        serde_json::json!({
            "file_path": "data.txt",
            "content": "next",
            "ensure_trailing_newline": false,
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert_eq!(
        std::fs::read_to_string(temp.path().join("data.txt")).expect("read"),
        "no-newline-herenext",
        "byte-exact concatenation must be preserved when separator is disabled"
    );
}

#[test]
fn append_file_tool_skips_separator_when_file_already_ends_in_newline() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("data.txt"), "first\n").expect("seed");

    let result = AppendFileTool.call(
        serde_json::json!({"file_path": "data.txt", "content": "second\n"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert_eq!(
        std::fs::read_to_string(temp.path().join("data.txt")).expect("read"),
        "first\nsecond\n",
        "no double-newline when file already terminates correctly"
    );
}

#[test]
fn append_file_tool_normalizes_line_endings() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = AppendFileTool.call(
        serde_json::json!({"file_path": "x.txt", "content": "alpha\r\nbeta\rgamma\n"}),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert_eq!(
        std::fs::read_to_string(temp.path().join("x.txt")).expect("read"),
        "alpha\nbeta\ngamma\n",
        "AppendFile must normalize line endings identically to Write"
    );
}

#[test]
fn append_file_tool_chains_to_build_a_large_file() {
    // The chunked-build pattern the user explicitly asked for: Write a
    // skeleton, then AppendFile each subsequent section. The resulting
    // file must be byte-identical to a single-shot write of the same
    // content, including final terminator.
    let temp = tempfile::tempdir().expect("tempdir");
    let ctx = ToolContext::new(temp.path());

    let head = WriteTool.call(
        serde_json::json!({
            "file_path": "page.html",
            "content": "<!DOCTYPE html>\n<html>\n<head><title>x</title></head>\n",
        }),
        &ctx,
    );
    assert!(!head.is_error);

    for chunk in ["<body>\n", "<h1>hi</h1>\n", "</body>\n", "</html>\n"] {
        let r = AppendFileTool.call(
            serde_json::json!({"file_path": "page.html", "content": chunk}),
            &ctx,
        );
        assert!(!r.is_error, "chunk append failed: {}", r.content);
    }

    assert_eq!(
        std::fs::read_to_string(temp.path().join("page.html")).expect("read"),
        "<!DOCTYPE html>\n<html>\n<head><title>x</title></head>\n<body>\n<h1>hi</h1>\n</body>\n</html>\n"
    );
}

#[test]
fn append_file_tool_blocks_sensitive_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join(".env"), "API_KEY=old\n").expect("seed");

    let result = ToolRegistry::with_builtin_tools().call(
        "AppendFile",
        serde_json::json!({"file_path": ".env", "content": "API_KEY=leaked\n"}),
        &ToolContext::new(temp.path()),
    );

    assert!(
        result.is_error,
        "appending to a sensitive file should be denied: {}",
        result.content
    );
    assert!(result.content.contains("sensitive"));
    assert_eq!(
        std::fs::read_to_string(temp.path().join(".env")).expect("read"),
        "API_KEY=old\n",
        "the sensitive file must not have been mutated"
    );
}

#[test]
fn append_file_tool_rejects_missing_path() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = AppendFileTool.call(
        serde_json::json!({"content": "anything"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(
        result.content.contains("file_path") && result.content.contains("content"),
        "error must teach the required schema, got: {}",
        result.content
    );
}

#[test]
fn append_file_tool_rejects_missing_content() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = AppendFileTool.call(
        serde_json::json!({"file_path": "x.txt"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(result.content.contains("string `content` field"));
}

#[test]
fn append_file_tool_rejects_blank_path() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = AppendFileTool.call(
        serde_json::json!({"file_path": "   ", "content": "x"}),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
}

#[test]
fn append_file_tool_is_registered_in_default_registry() {
    let temp = tempfile::tempdir().expect("tempdir");
    let registry = ToolRegistry::with_builtin_tools();

    let result = registry.call(
        "AppendFile",
        serde_json::json!({"file_path": "via-registry.txt", "content": "hello\n"}),
        &ToolContext::new(temp.path()),
    );

    assert!(
        !result.is_error,
        "AppendFile should be discoverable via the default registry, got: {}",
        result.content
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("via-registry.txt")).expect("read"),
        "hello\n"
    );
}

#[test]
fn edit_tool_replaces_one_exact_match() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\nbeta\ngamma\n").expect("write fixture");

    let result = EditTool.call(
        serde_json::json!({
            "file_path": "notes.txt",
            "old_string": "beta",
            "new_string": "bravo"
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(
        result
            .content
            .contains("File edited successfully: notes.txt")
    );
    assert_eq!(
        std::fs::read_to_string(temp.path().join("notes.txt")).expect("read edited file"),
        "alpha\nbravo\ngamma\n"
    );
}

#[test]
fn edit_tool_requires_replace_all_for_multiple_matches() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha alpha\n").expect("write fixture");

    let result = EditTool.call(
        serde_json::json!({
            "file_path": "notes.txt",
            "old_string": "alpha",
            "new_string": "bravo"
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(result.content.contains("old_string matches 2 locations"));
}

#[test]
fn edit_tool_replace_all_updates_every_match() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha alpha\n").expect("write fixture");

    let result = EditTool.call(
        serde_json::json!({
            "file_path": "notes.txt",
            "old_string": "alpha",
            "new_string": "bravo",
            "replace_all": true
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert!(result.content.contains("all 2 occurrences"));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("notes.txt")).expect("read edited file"),
        "bravo bravo\n"
    );
}

#[test]
fn edit_tool_can_create_file_when_old_string_is_empty() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = EditTool.call(
        serde_json::json!({
            "file_path": "src/new.txt",
            "old_string": "",
            "new_string": "created\n"
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(!result.is_error);
    assert_eq!(result.content, "Created new file: new.txt");
    assert_eq!(
        std::fs::read_to_string(temp.path().join("src/new.txt")).expect("read created file"),
        "created\n"
    );
}

#[test]
fn edit_tool_reports_missing_old_string_with_reread_guidance() {
    let temp = tempfile::tempdir().expect("tempdir");
    std::fs::write(temp.path().join("notes.txt"), "alpha\n").expect("write fixture");

    let result = EditTool.call(
        serde_json::json!({
            "file_path": "notes.txt",
            "old_string": "beta",
            "new_string": "bravo"
        }),
        &ToolContext::new(temp.path()),
    );

    assert!(result.is_error);
    assert!(result.content.contains("old_string not found in file"));
    assert!(result.content.contains("Please re-read the file"));
}

#[cfg(windows)]
fn list_workspace_command() -> &'static str {
    "dir /b"
}

#[cfg(not(windows))]
fn list_workspace_command() -> &'static str {
    "ls"
}

#[cfg(windows)]
fn stderr_failure_command() -> &'static str {
    "echo boom 1>&2 && exit /b 7"
}

#[cfg(not(windows))]
fn stderr_failure_command() -> &'static str {
    "echo boom >&2; exit 7"
}

#[cfg(windows)]
fn monitor_stdout_command() -> &'static str {
    "echo ready && echo done"
}

#[cfg(not(windows))]
fn monitor_stdout_command() -> &'static str {
    "printf 'ready\\ndone\\n'"
}

#[cfg(windows)]
fn monitor_until_command() -> &'static str {
    "echo boot && echo READY && ping -n 6 127.0.0.1 > nul"
}

#[cfg(not(windows))]
fn monitor_until_command() -> &'static str {
    "printf 'boot\\nREADY\\n'; sleep 5"
}

fn serve_once(status: u16, content_type: &'static str, body: &'static str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local fixture server");
    let addr = listener.local_addr().expect("fixture server address");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept fixture request");
        let mut buffer = [0_u8; 1024];
        let _ = stream.read(&mut buffer);
        let reason = if status == 200 { "OK" } else { "ERROR" };
        write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("write fixture response");
    });
    format!("http://{addr}/page")
}

fn serve_remote_trigger(status: u16, body: &'static str) -> (String, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local remote trigger server");
    let addr = listener
        .local_addr()
        .expect("remote trigger server address");
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept remote trigger request");
        let request = read_http_request_text(&mut stream);
        tx.send(request).expect("send captured request");
        let reason = if (200..300).contains(&status) {
            "OK"
        } else {
            "ERROR"
        };
        write!(
            stream,
            "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .expect("write remote trigger response");
    });
    (format!("http://{addr}/hook"), rx)
}

fn serve_publish_sequence(responses: Vec<(u16, &'static str)>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local publish server");
    let addr = listener.local_addr().expect("publish server address");
    thread::spawn(move || {
        for (status, body) in responses {
            let (mut stream, _) = listener.accept().expect("accept publish request");
            read_http_request(&mut stream);
            let reason = if status == 200 { "OK" } else { "ERROR" };
            write!(
                stream,
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("write publish response");
        }
    });
    format!("http://{addr}")
}

fn read_http_request_text(stream: &mut impl Read) -> String {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let mut header_end = None;
    while header_end.is_none() {
        let read = stream.read(&mut chunk).expect("read request");
        if read == 0 {
            return String::from_utf8_lossy(&buffer).into_owned();
        }
        buffer.extend_from_slice(&chunk[..read]);
        header_end = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    }
    let header_end = header_end.expect("header end") + 4;
    let headers = String::from_utf8_lossy(&buffer[..header_end]);
    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let mut remaining = content_length.saturating_sub(buffer.len().saturating_sub(header_end));
    while remaining > 0 {
        let read = stream.read(&mut chunk).expect("read request body");
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        remaining = remaining.saturating_sub(read);
    }
    String::from_utf8_lossy(&buffer).into_owned()
}

fn read_http_request(stream: &mut impl Read) {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 4096];
    let mut header_end = None;
    while header_end.is_none() {
        let read = stream.read(&mut chunk).expect("read request");
        if read == 0 {
            return;
        }
        buffer.extend_from_slice(&chunk[..read]);
        header_end = buffer.windows(4).position(|window| window == b"\r\n\r\n");
    }
    let header_end = header_end.expect("header end") + 4;
    let headers = String::from_utf8_lossy(&buffer[..header_end]);

    // reqwest multipart::Form::Part::reader produces a chunked-encoded body
    // because the file Reader has no known length. Without this branch the
    // server would read 0 body bytes, send `Connection: close`, and tear down
    // the socket while the client was mid-upload — exactly the source of the
    // flaky `publish_tool_uploads_archive_and_saves_state` failure.
    let chunked = headers.lines().any(|line| {
        line.split_once(':').is_some_and(|(name, value)| {
            name.eq_ignore_ascii_case("transfer-encoding")
                && value
                    .split(',')
                    .any(|token| token.trim().eq_ignore_ascii_case("chunked"))
        })
    });
    if chunked {
        let mut body = buffer.split_off(header_end);
        loop {
            if body.windows(5).any(|window| window == b"0\r\n\r\n") {
                return;
            }
            let read = stream.read(&mut chunk).expect("read chunked body");
            if read == 0 {
                return;
            }
            body.extend_from_slice(&chunk[..read]);
        }
    }

    let content_length = headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())
                .flatten()
        })
        .unwrap_or(0);
    let mut remaining = content_length.saturating_sub(buffer.len().saturating_sub(header_end));
    while remaining > 0 {
        let read = stream.read(&mut chunk).expect("read request body");
        if read == 0 {
            return;
        }
        remaining = remaining.saturating_sub(read);
    }
}

fn todo_test_guard() -> MutexGuard<'static, ()> {
    static TODO_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TODO_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("todo test lock")
}

// `std::env::set_var` / `remove_var` are process-global and Rust 2024 marks
// them `unsafe` precisely because they race with every other env reader in the
// binary, regardless of which "group" of vars is being mutated. Originally
// `publish_env_guard` and `memory_env_guard` were two independent mutexes, so
// a memory test mutating HOME could race with a publish test mutating
// PAEAN_API_TOKEN and corrupt either side. Both helpers now share a single
// global lock so any env-mutating or env-reading test is fully serialised.
fn env_test_lock() -> &'static Mutex<()> {
    static ENV_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    ENV_TEST_LOCK.get_or_init(|| Mutex::new(()))
}

fn publish_env_guard() -> MutexGuard<'static, ()> {
    env_test_lock()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

fn install_publish_env(base_url: &str) {
    unsafe {
        std::env::set_var("PAEAN_API_TOKEN", "fixture-credential");
        std::env::set_var("PAEAN_API_BASE_URL", base_url);
        std::env::remove_var("PAEAN_TOKEN");
        std::env::remove_var("CLIDE_API_TOKEN");
        std::env::remove_var("CLIDE_API_BASE_URL");
    }
}

fn clear_publish_env() {
    unsafe {
        std::env::remove_var("PAEAN_API_TOKEN");
        std::env::remove_var("PAEAN_TOKEN");
        std::env::remove_var("CLIDE_API_TOKEN");
        std::env::remove_var("PAEAN_API_BASE_URL");
        std::env::remove_var("CLIDE_API_BASE_URL");
    }
}

fn memory_env_guard() -> MutexGuard<'static, ()> {
    env_test_lock()
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

fn install_memory_env(root: &std::path::Path) {
    unsafe {
        std::env::set_var("HOME", root.join("home"));
        std::env::set_var("TIDE_CONFIG_DIR", root.join("tide-config"));
    }
}

fn init_git_fixture(path: &std::path::Path) {
    run_git_fixture(path, &["init"]);
    run_git_fixture(path, &["config", "user.email", "deeptide@example.invalid"]);
    run_git_fixture(path, &["config", "user.name", "Deeptide Tests"]);
    std::fs::write(path.join("README.md"), "fixture\n").expect("write readme");
    run_git_fixture(path, &["add", "README.md"]);
    run_git_fixture(path, &["commit", "-m", "initial"]);
}

fn run_git_fixture(path: &std::path::Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(path)
        .output()
        .unwrap_or_else(|error| panic!("git fixture command should run: {error}"));
    assert!(
        output.status.success(),
        "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
