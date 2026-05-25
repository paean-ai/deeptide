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
