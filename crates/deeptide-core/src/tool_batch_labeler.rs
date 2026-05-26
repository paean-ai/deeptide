use std::collections::BTreeMap;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolBatchItem {
    pub name: String,
    pub input: Value,
}

impl ToolBatchItem {
    pub fn new(name: impl Into<String>, input: Value) -> Self {
        Self {
            name: name.into(),
            input,
        }
    }
}

pub struct ToolBatchLabeler;

impl ToolBatchLabeler {
    pub fn label(items: &[ToolBatchItem], failed_count: usize) -> String {
        Self::label_with_failure_summaries(items, &vec![None; failed_count])
    }

    pub fn label_with_failure_summaries(
        items: &[ToolBatchItem],
        failure_summaries: &[Option<String>],
    ) -> String {
        let Some(first) = items.first() else {
            return "Ran tools".to_owned();
        };
        let same_tool = items
            .iter()
            .all(|item| item.name.eq_ignore_ascii_case(&first.name));
        let base = if same_tool {
            same_tool_label(&first.name, items)
        } else {
            mixed_tool_label(items)
        };
        if failure_summaries.is_empty() {
            base
        } else {
            format!("{base}, {}", failure_label(failure_summaries))
        }
    }
}

pub struct ToolBatchFailureClassifier;

impl ToolBatchFailureClassifier {
    pub fn classify(raw: &str, is_error: bool) -> Option<String> {
        if !is_error {
            return None;
        }
        let raw = raw.trim().to_ascii_lowercase();
        if raw.contains("unsupported binary or package-like file")
            || raw.contains("reason: unsupported binary or package-like file")
        {
            return Some("unsupported file types".to_owned());
        }
        if raw.starts_with("path is a directory") {
            return Some("directory paths".to_owned());
        }
        if raw.starts_with("file does not exist") {
            return Some("missing files".to_owned());
        }
        if raw.contains("websearch requires an api key") {
            return Some("unavailable web search".to_owned());
        }
        if raw.contains("validation error:") || raw.contains("invalid input") {
            return Some("invalid tool inputs".to_owned());
        }
        if raw.contains("old_string not found") {
            return Some("stale edit snippets".to_owned());
        }
        if raw.contains("old_string matches") && raw.contains("locations") {
            return Some("ambiguous edit snippets".to_owned());
        }
        if raw.starts_with("denied") || raw.contains("permission denied") {
            return Some("permission denials".to_owned());
        }
        if raw.starts_with("cancelled") || raw.starts_with("interrupted") {
            return Some("cancelled tools".to_owned());
        }
        if raw.starts_with("timed out") || raw.contains("timed out after") {
            return Some("timeouts".to_owned());
        }
        None
    }
}

fn same_tool_label(name: &str, items: &[ToolBatchItem]) -> String {
    let count = items.len();
    match name.to_ascii_lowercase().as_str() {
        "read" => {
            let paths = string_values(items, "file_path");
            if let Some(dir) = common_directory(&paths) {
                return format!("Read {count} {} in {dir}", plural("file", count));
            }
        }
        "grep" => {
            if let Some(pattern) = common_value(items, "pattern") {
                return format!(
                    "Searched {count} {} for {}",
                    plural("path", count),
                    quote(&pattern)
                );
            }
            if let Some(path) = common_value(items, "path") {
                return format!("Searched {count} {} in {path}", plural("path", count));
            }
        }
        "glob" => {
            if let Some(pattern) = common_value(items, "pattern") {
                return format!("Listed {count} {}: {pattern}", plural("glob", count));
            }
        }
        "bash" | "monitor" => {
            if count == 1
                && let Some(command) = common_value(items, "command")
            {
                return format!("Ran {}", shorten(&command, 42));
            }
        }
        "webfetch" => {
            let urls = string_values(items, "url");
            if let Some(host) = common_host(&urls) {
                return format!("Fetched {count} {} from {host}", plural("page", count));
            }
        }
        "websearch" | "spotlightsearch" | "memorysearch" => {
            if let Some(query) = common_value(items, "query") {
                return format!(
                    "Searched {count} {} for {}",
                    plural("source", count),
                    quote(&query)
                );
            }
        }
        _ => {}
    }

    format!(
        "{} {count} {}",
        past_tense_tool_verb(name),
        tool_batch_noun(name, count)
    )
}

fn mixed_tool_label(items: &[ToolBatchItem]) -> String {
    let mut grouped = BTreeMap::<String, usize>::new();
    for item in items {
        *grouped.entry(canonical_name(&item.name)).or_default() += 1;
    }
    let summary = grouped
        .iter()
        .take(3)
        .map(|(name, count)| format!("{name}×{count}"))
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if grouped.len() > 3 { "..." } else { "" };
    format!("Ran {} tools: {summary}{suffix}", items.len())
}

fn failure_label(summaries: &[Option<String>]) -> String {
    let fallback = format!("{} failed", summaries.len());
    let values = summaries
        .iter()
        .filter_map(|value| value.as_ref())
        .filter(|value| !value.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if values.is_empty() {
        return fallback;
    }

    let mut grouped = BTreeMap::<String, usize>::new();
    for value in &values {
        *grouped.entry(value.clone()).or_default() += 1;
    }
    let mut sorted = grouped.into_iter().collect::<Vec<_>>();
    sorted.sort_by(|(left_reason, left_count), (right_reason, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_reason.cmp(right_reason))
    });

    if sorted.len() == 1 && sorted[0].1 == summaries.len() {
        return format!("{} {}", sorted[0].1, sorted[0].0);
    }

    let unknown = summaries.len().saturating_sub(values.len());
    let mut parts = sorted
        .iter()
        .take(2)
        .map(|(reason, count)| format!("{count} {reason}"))
        .collect::<Vec<_>>();
    if unknown > 0 {
        parts.push(format!("{unknown} failed"));
    }
    let accounted = sorted
        .iter()
        .take(2)
        .map(|(_, count)| *count)
        .sum::<usize>()
        + unknown;
    if summaries.len() > accounted {
        parts.push(format!("{} other failed", summaries.len() - accounted));
    }
    parts.join(", ")
}

fn common_value(items: &[ToolBatchItem], key: &str) -> Option<String> {
    let values = string_values(items, key);
    let first = values.first()?;
    if values.len() == items.len() && values.iter().all(|value| value == first) {
        Some(first.clone())
    } else {
        None
    }
}

fn string_values(items: &[ToolBatchItem], key: &str) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| item.input.get(key).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn common_directory(paths: &[String]) -> Option<String> {
    if paths.is_empty() {
        return None;
    }
    let dirs = paths
        .iter()
        .map(|path| {
            std::path::Path::new(path)
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned())
                .filter(|dir| !dir.is_empty())
                .unwrap_or_else(|| ".".to_owned())
        })
        .collect::<Vec<_>>();
    let first = dirs.first()?;
    let absolute = first.starts_with('/');
    let mut common = first
        .split('/')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();

    for dir in dirs.iter().skip(1) {
        let next = dir
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();
        let len = common
            .iter()
            .zip(next.iter())
            .take_while(|(left, right)| left == right)
            .count();
        common.truncate(len);
        if common.is_empty() {
            break;
        }
    }

    if common.is_empty() {
        return Some(".".to_owned());
    }
    let prefix = if absolute { "/" } else { "" };
    Some(shorten(&format!("{prefix}{}", common.join("/")), 36))
}

fn common_host(urls: &[String]) -> Option<String> {
    let hosts = urls
        .iter()
        .filter_map(|url| {
            reqwest::Url::parse(url)
                .ok()?
                .host_str()
                .map(ToOwned::to_owned)
        })
        .collect::<Vec<_>>();
    let first = hosts.first()?;
    if hosts.len() == urls.len() && hosts.iter().all(|host| host == first) {
        Some(first.clone())
    } else {
        None
    }
}

fn quote(value: &str) -> String {
    format!("\"{}\"", shorten(value, 32))
}

fn shorten(value: &str, max: usize) -> String {
    if value.chars().count() > max && max > 1 {
        format!("{}...", value.chars().take(max - 1).collect::<String>())
    } else {
        value.to_owned()
    }
}

fn plural(noun: &str, count: usize) -> String {
    if count == 1 {
        noun.to_owned()
    } else {
        format!("{noun}s")
    }
}

fn canonical_name(name: &str) -> String {
    match name.to_ascii_lowercase().as_str() {
        "websearch" => "WebSearch".to_owned(),
        "webfetch" => "WebFetch".to_owned(),
        "spotlightsearch" => "Spotlight".to_owned(),
        "memorysearch" => "MemorySearch".to_owned(),
        _ => {
            let mut chars = name.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        }
    }
}

fn past_tense_tool_verb(tool_name: &str) -> &'static str {
    match tool_name.to_ascii_lowercase().as_str() {
        "read" | "readfiles" => "Read",
        "grep" | "websearch" | "spotlightsearch" | "memorysearch" => "Searched",
        "glob" => "Listed",
        "bash" | "monitor" => "Ran",
        "edit" | "notebookedit" => "Edited",
        "write" => "Wrote",
        "webfetch" => "Fetched",
        "agent" => "Ran",
        "vision" | "screencapture" => "Analyzed",
        _ => "Ran",
    }
}

fn tool_batch_noun(tool_name: &str, count: usize) -> String {
    let suffix = if count == 1 { "" } else { "s" };
    match tool_name.to_ascii_lowercase().as_str() {
        "read" | "readfiles" => format!("file{suffix}"),
        "grep" => format!("search{suffix}"),
        "glob" => format!("glob{suffix}"),
        "websearch" | "spotlightsearch" | "memorysearch" => format!("search{suffix}"),
        "webfetch" => format!("page{suffix}"),
        "edit" | "notebookedit" => format!("edit{suffix}"),
        "write" => format!("write{suffix}"),
        "bash" | "monitor" => format!("command{suffix}"),
        "agent" => format!("agent{suffix}"),
        "vision" | "screencapture" => format!("capture{suffix}"),
        _ => format!("tool{suffix}"),
    }
}
