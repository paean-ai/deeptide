use deeptide_core::{
    BashTool, EditTool, FileMetadataTool, ReadFilesTool, TaskCreateTool, TaskGetTool, TaskListTool,
    TaskOutputTool, TaskStopTool, TaskUpdateTool, TodoWriteTool, Tool, ToolContext, ToolRegistry,
    WebFetchTool, WebSearchTool, WriteTool,
};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Mutex, MutexGuard, OnceLock};
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
    assert_eq!(result.content, "2\tbeta\n3\tgamma");
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

fn todo_test_guard() -> MutexGuard<'static, ()> {
    static TODO_TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TODO_TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("todo test lock")
}
