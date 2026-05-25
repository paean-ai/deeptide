use deeptide_core::ToolResultSummaryFormatter;

#[test]
fn compacts_structured_unsupported_file_type_failure() {
    let raw = "\
reason: unsupported binary or package-like file
file: archive.zip
type: public.zip-archive · mime: application/zip · text: no
likely_cause: macOS classified this path as non-text.
next_action: use FileMetadata.
";

    let summary = ToolResultSummaryFormatter::summary("Read", raw, true);

    assert_eq!(
        summary,
        "unsupported file type — use FileMetadata or a dedicated reader"
    );
    assert!(ToolResultSummaryFormatter::should_mute_appearance(
        "Read", raw, true
    ));
}

#[test]
fn compacts_directory_and_missing_file_failures() {
    assert_eq!(
        ToolResultSummaryFormatter::summary(
            "Read",
            "Path is a directory: /tmp/project\nUse Glob to list files or Grep to search within it.",
            true,
        ),
        "directory path — use Glob to list or Grep to search"
    );
    assert_eq!(
        ToolResultSummaryFormatter::summary(
            "Read",
            "File does not exist: /tmp/nope.ts\nHint: use Glob.",
            true,
        ),
        "file not found — use Glob or find to locate it"
    );
}

#[test]
fn keeps_edit_recoverability_summary() {
    let summary = ToolResultSummaryFormatter::summary("Edit", "old_string not found in file", true);

    assert_eq!(summary, "old_string not found — re-read file");
    assert!(ToolResultSummaryFormatter::should_mute_appearance(
        "Edit",
        "old_string not found",
        true
    ));
}

#[test]
fn compacts_common_tool_input_failures() {
    assert_eq!(
        ToolResultSummaryFormatter::summary(
            "Bash",
            "Validation error: command must not contain newlines",
            true,
        ),
        "invalid Bash command — rewrite as one line with && or ;"
    );
    assert_eq!(
        ToolResultSummaryFormatter::summary(
            "WebSearch",
            "Error: WebSearch requires an API key. Set one of: TAVILY_API_KEY",
            true,
        ),
        "WebSearch unavailable — configure API key or use WebFetch"
    );
}

#[test]
fn success_summaries_still_use_counts() {
    let summary = ToolResultSummaryFormatter::summary("Read", "1\talpha\n2\tbeta\n", false);

    assert_eq!(summary, "2 lines (14 B)");
}

#[test]
fn formats_general_errors_and_successes_like_swift() {
    assert_eq!(
        ToolResultSummaryFormatter::summary("Bash", "done", false),
        "done"
    );
    assert_eq!(
        ToolResultSummaryFormatter::summary("Write", "hello", false),
        "Wrote 5 B"
    );
    assert_eq!(
        ToolResultSummaryFormatter::summary("Other", "Something failed badly", true),
        "Error: Something failed badly"
    );
    assert_eq!(
        ToolResultSummaryFormatter::summary("Other", "Error: already prefixed", true),
        "Error: already prefixed"
    );
}
