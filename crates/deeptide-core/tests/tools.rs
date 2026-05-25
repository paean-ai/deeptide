use deeptide_core::{ToolContext, ToolRegistry};

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
fn registry_reports_unknown_tools() {
    let result = ToolRegistry::with_builtin_tools().call(
        "Nope",
        serde_json::json!({}),
        &ToolContext::new("."),
    );

    assert!(result.is_error);
    assert_eq!(result.content, "Unknown tool: Nope");
}
