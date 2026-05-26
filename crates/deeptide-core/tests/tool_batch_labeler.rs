use deeptide_core::{ToolBatchFailureClassifier, ToolBatchItem, ToolBatchLabeler};

fn item(name: &str, input: serde_json::Value) -> ToolBatchItem {
    ToolBatchItem::new(name, input)
}

#[test]
fn read_batch_includes_common_directory() {
    let label = ToolBatchLabeler::label(
        &[
            item(
                "Read",
                serde_json::json!({"file_path": "Sources/REPL/REPL.swift"}),
            ),
            item(
                "Read",
                serde_json::json!({"file_path": "Sources/REPL/CompletionOverlay.swift"}),
            ),
        ],
        0,
    );

    assert_eq!(label, "Read 2 files in Sources/REPL");
}

#[test]
fn grep_batch_includes_search_pattern_and_failures() {
    let label = ToolBatchLabeler::label(
        &[
            item(
                "Grep",
                serde_json::json!({"pattern": "completion", "path": "Sources"}),
            ),
            item(
                "Grep",
                serde_json::json!({"pattern": "completion", "path": "Tests"}),
            ),
        ],
        1,
    );

    assert_eq!(label, "Searched 2 paths for \"completion\", 1 failed");
}

#[test]
fn mixed_batch_shows_type_counts() {
    let label = ToolBatchLabeler::label(
        &[
            item("Read", serde_json::json!({})),
            item("Read", serde_json::json!({})),
            item("Grep", serde_json::json!({})),
            item("Glob", serde_json::json!({})),
        ],
        0,
    );

    assert_eq!(label, "Ran 4 tools: Glob×1, Grep×1, Read×2");
}

#[test]
fn failure_summaries_use_specific_reason() {
    let label = ToolBatchLabeler::label_with_failure_summaries(
        &[
            item("Read", serde_json::json!({"file_path": "a.zip"})),
            item("Read", serde_json::json!({"file_path": "b.zip"})),
            item("Read", serde_json::json!({"file_path": "c.ts"})),
        ],
        &[
            Some("unsupported file types".to_owned()),
            Some("unsupported file types".to_owned()),
        ],
    );

    assert_eq!(label, "Read 3 files in ., 2 unsupported file types");
}

#[test]
fn failure_summaries_group_mixed_reasons() {
    let label = ToolBatchLabeler::label_with_failure_summaries(
        &[
            item("Read", serde_json::json!({})),
            item("FileMetadata", serde_json::json!({})),
            item("Read", serde_json::json!({})),
        ],
        &[
            Some("unsupported file types".to_owned()),
            Some("missing files".to_owned()),
            None,
        ],
    );

    assert_eq!(
        label,
        "Ran 3 tools: FileMetadata×1, Read×2, 1 missing files, 1 unsupported file types, 1 failed"
    );
}

#[test]
fn failure_classifier_recognizes_common_errors() {
    assert_eq!(
        ToolBatchFailureClassifier::classify(
            "reason: unsupported binary or package-like file\nfile: app.zip",
            true
        )
        .as_deref(),
        Some("unsupported file types")
    );
    assert_eq!(
        ToolBatchFailureClassifier::classify("File does not exist: /tmp/nope", true).as_deref(),
        Some("missing files")
    );
}

#[test]
fn failure_classifier_recognizes_tool_input_and_search_config_errors() {
    assert_eq!(
        ToolBatchFailureClassifier::classify(
            "Validation error: command must not contain newlines",
            true
        )
        .as_deref(),
        Some("invalid tool inputs")
    );
    assert_eq!(
        ToolBatchFailureClassifier::classify("Error: WebSearch requires an API key.", true)
            .as_deref(),
        Some("unavailable web search")
    );
    assert_eq!(
        ToolBatchFailureClassifier::classify("Error: WebSearch requires an API key.", false),
        None
    );
}
