pub struct ToolResultSummaryFormatter;

impl ToolResultSummaryFormatter {
    pub fn summary(tool_name: &str, raw: &str, is_error: bool) -> String {
        let trimmed = raw.trim();
        if is_error {
            if tool_name.eq_ignore_ascii_case("edit")
                && let Some(compact) = compact_edit_failure_summary(trimmed)
            {
                return compact;
            }
            if let Some(compact) = compact_recoverable_failure_summary(trimmed) {
                return compact;
            }

            let first_line = trimmed.lines().next().unwrap_or(trimmed);
            let lower = first_line.to_ascii_lowercase();
            let needs_prefix = !["error", "blocked", "denied", "cancelled", "interrupted"]
                .iter()
                .any(|prefix| lower.starts_with(prefix))
                && !lower.starts_with("timed out");
            let body = truncate_chars(first_line, 100);
            return if needs_prefix {
                format!("Error: {body}")
            } else {
                body
            };
        }

        if trimmed.is_empty() {
            return "ok".to_owned();
        }

        let line_count = trimmed.split('\n').count();
        let bytes = trimmed.len();
        let bytes_str = format_bytes_short(bytes);

        match tool_name.to_ascii_lowercase().as_str() {
            "bash" | "monitor" | "shell" => {
                if line_count == 1 && trimmed.chars().count() <= 100 {
                    trimmed.to_owned()
                } else {
                    format!("{line_count} lines ({bytes_str})")
                }
            }
            "read" => format!("{line_count} lines ({bytes_str})"),
            "edit" | "notebookedit" => {
                truncate_chars(trimmed.lines().next().unwrap_or("Edited"), 100)
            }
            "write" => format!("Wrote {bytes_str}"),
            "grep" => {
                if line_count == 0 {
                    "no matches".to_owned()
                } else {
                    format!("{line_count} matches")
                }
            }
            "glob" => format!("{line_count} paths"),
            "todowrite" | "taskcreate" | "taskupdate" | "taskstop" => {
                truncate_chars(trimmed.lines().next().unwrap_or("ok"), 100)
            }
            "webfetch" | "websearch" => format!("{line_count} lines ({bytes_str})"),
            _ => {
                if line_count == 1 && trimmed.chars().count() <= 100 {
                    trimmed.to_owned()
                } else {
                    format!("{line_count} lines ({bytes_str})")
                }
            }
        }
    }

    pub fn should_mute_appearance(tool_name: &str, raw: &str, is_error: bool) -> bool {
        if !is_error {
            return false;
        }
        if tool_name.eq_ignore_ascii_case("edit") && is_recoverable_edit_failure_text(raw) {
            return true;
        }
        is_recoverable_file_type_failure(raw)
    }

    pub fn is_recoverable_edit_failure_text(raw: &str) -> bool {
        is_recoverable_edit_failure_text(raw)
    }

    pub fn compact_edit_failure_summary(trimmed: &str) -> Option<String> {
        compact_edit_failure_summary(trimmed)
    }
}

fn compact_recoverable_failure_summary(trimmed: &str) -> Option<String> {
    let lower = trimmed.to_ascii_lowercase();
    if is_recoverable_file_type_failure(trimmed) {
        return Some("unsupported file type — use FileMetadata or a dedicated reader".to_owned());
    }
    if lower.starts_with("path is a directory") {
        return Some("directory path — use Glob to list or Grep to search".to_owned());
    }
    if lower.starts_with("file does not exist") {
        return Some("file not found — use Glob or find to locate it".to_owned());
    }
    if lower.contains("websearch requires an api key") {
        return Some("WebSearch unavailable — configure API key or use WebFetch".to_owned());
    }
    if lower.contains("validation error: command must not contain newlines")
        || lower.contains("command must be a single line")
    {
        return Some("invalid Bash command — rewrite as one line with && or ;".to_owned());
    }
    structured_value("reason", trimmed)
        .map(|reason| format!("{} — see next_action", truncate_chars(&reason, 72)))
}

fn is_recoverable_file_type_failure(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("unsupported binary or package-like file")
        || lower.contains("reason: unsupported binary or package-like file")
}

fn is_recoverable_edit_failure_text(raw: &str) -> bool {
    let lower = raw.to_ascii_lowercase();
    lower.contains("old_string not found")
        || lower.contains("old_string matches") && lower.contains("locations")
}

fn compact_edit_failure_summary(trimmed: &str) -> Option<String> {
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("old_string not found") {
        return Some("old_string not found — re-read file".to_owned());
    }
    if lower.contains("old_string matches") && lower.contains("locations") {
        return Some("multiple matches — narrow snippet or replace_all".to_owned());
    }
    None
}

fn structured_value(key: &str, text: &str) -> Option<String> {
    let prefix = format!("{}:", key.to_ascii_lowercase());
    text.lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.to_ascii_lowercase().starts_with(&prefix) {
            Some(trimmed[prefix.len()..].trim().to_owned())
        } else {
            None
        }
    })
}

fn truncate_chars(value: &str, limit: usize) -> String {
    value.chars().take(limit).collect()
}

fn format_bytes_short(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
