use deeptide_core::{EditTool, Tool, ToolContext, ToolRegistry, WriteTool};

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
