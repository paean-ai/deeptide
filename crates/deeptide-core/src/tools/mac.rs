//! macOS diagnostics tools: crash-log inspection (CrashLog), unified log queries (MacLog), and guided diagnose flows (MacDiagnose).
//!
//! Shared infrastructure lives in the parent module (reached via `use super::*`).

use super::*;

#[derive(Debug, Default, Clone, Copy)]
pub struct CrashLogTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct MacLogTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct MacDiagnoseTool;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CrashReport {
    path: PathBuf,
    modified: std::time::SystemTime,
    size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MacLogQuery {
    process: Option<String>,
    subsystem: Option<String>,
    category: Option<String>,
    contains: Option<String>,
    level: String,
    last_minutes: u64,
    limit: usize,
    timeout_ms: u64,
}

impl Tool for CrashLogTool {
    fn name(&self) -> &'static str {
        "CrashLog"
    }

    fn description(&self) -> &'static str {
        "Inspect local macOS DiagnosticReports for recent crash, hang, spin, panic, and ips reports."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let operation = input
            .get("operation")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("list");
        if !matches!(operation, "list" | "latest" | "read") {
            return ToolResult::error("operation must be list, latest, or read");
        }
        let app_name = input
            .get("app_name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let limit = input
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(20)
            .clamp(1, 100) as usize;
        let max_lines = input
            .get("max_lines")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(160)
            .clamp(1, 1000) as usize;

        match operation {
            "list" => ToolResult::text(render_crash_log_list(
                app_name,
                limit,
                &default_crash_log_dirs(),
            )),
            "latest" => ToolResult::text(render_crash_log_latest(
                app_name,
                max_lines,
                &default_crash_log_dirs(),
            )),
            "read" => {
                let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str)
                else {
                    return ToolResult::error("file_path is required for operation=read");
                };
                ToolResult::text(render_crash_log_read(
                    &context.resolve_path(file_path),
                    max_lines,
                ))
            }
            _ => unreachable!("validated operation"),
        }
    }
}

impl Tool for MacLogTool {
    fn name(&self) -> &'static str {
        "MacLog"
    }

    fn description(&self) -> &'static str {
        "Search recent macOS Unified Logging entries with bounded filters."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(query) = MacLogQuery::from_input(&input) else {
            return ToolResult::error(
                "level must be error_or_fault, fault, error, default, info, debug, or all",
            );
        };
        #[cfg(target_os = "macos")]
        {
            ToolResult::text(run_mac_log_query(&query))
        }
        #[cfg(not(target_os = "macos"))]
        {
            ToolResult::text(format!(
                "[MacLog] macOS Unified Logging is only available on macOS.\nrequested: /usr/bin/log {}",
                query.arguments().join(" ")
            ))
        }
    }
}

impl Tool for MacDiagnoseTool {
    fn name(&self) -> &'static str {
        "MacDiagnose"
    }

    fn description(&self) -> &'static str {
        "Build a focused macOS-native diagnostic path for local failures."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let scenario = input
            .get("scenario")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("general");
        if !is_valid_mac_diagnose_scenario(scenario) {
            return ToolResult::error(
                "scenario must be general, crash, permission, screen, audio, keychain, network, install, or performance",
            );
        }
        let app_name = input
            .get("app_name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("deeptide");
        ToolResult::text(render_mac_diagnose(scenario, app_name, &[]))
    }
}

impl MacLogQuery {
    fn from_input(input: &serde_json::Value) -> Option<Self> {
        let level = input
            .get("level")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("error_or_fault");
        if !matches!(
            level,
            "error_or_fault" | "fault" | "error" | "default" | "info" | "debug" | "all"
        ) {
            return None;
        }
        Some(Self {
            process: clean_json_string(input, "process"),
            subsystem: clean_json_string(input, "subsystem"),
            category: clean_json_string(input, "category"),
            contains: clean_json_string(input, "contains"),
            level: level.to_owned(),
            last_minutes: input
                .get("last_minutes")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(15)
                .clamp(1, 1440),
            limit: input
                .get("limit")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(80)
                .clamp(1, 300) as usize,
            timeout_ms: input
                .get("timeout_ms")
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(8000)
                .clamp(1000, 30000),
        })
    }

    fn predicate(&self) -> String {
        let mut clauses = Vec::new();
        if let Some(process) = &self.process {
            clauses.push(format!("process == \"{}\"", escape_log_predicate(process)));
        }
        if let Some(subsystem) = &self.subsystem {
            clauses.push(format!(
                "subsystem == \"{}\"",
                escape_log_predicate(subsystem)
            ));
        }
        if let Some(category) = &self.category {
            clauses.push(format!(
                "category == \"{}\"",
                escape_log_predicate(category)
            ));
        }
        if let Some(contains) = &self.contains {
            clauses.push(format!(
                "eventMessage CONTAINS[c] \"{}\"",
                escape_log_predicate(contains)
            ));
        }
        match self.level.as_str() {
            "fault" => clauses.push(String::from("messageType == fault")),
            "error" => clauses.push(String::from("messageType == error")),
            "error_or_fault" => clauses.push(String::from(
                "(messageType == error OR messageType == fault)",
            )),
            _ => {}
        }
        if clauses.is_empty() {
            String::from("TRUEPREDICATE")
        } else {
            clauses.join(" AND ")
        }
    }

    fn arguments(&self) -> Vec<String> {
        let mut args = vec![
            String::from("show"),
            String::from("--last"),
            format!("{}m", self.last_minutes),
            String::from("--style"),
            String::from("compact"),
            String::from("--predicate"),
            self.predicate(),
        ];
        if matches!(self.level.as_str(), "info" | "all") {
            args.push(String::from("--info"));
        }
        if matches!(self.level.as_str(), "debug" | "all") {
            args.push(String::from("--debug"));
        }
        args
    }
}

fn clean_json_string(input: &serde_json::Value, key: &str) -> Option<String> {
    input
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn default_crash_log_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = home_dir() {
        dirs.push(home.join("Library/Logs/DiagnosticReports"));
    }
    dirs.push(PathBuf::from("/Library/Logs/DiagnosticReports"));
    dirs
}

fn render_crash_log_list(app_name: Option<&str>, limit: usize, dirs: &[PathBuf]) -> String {
    let reports = crash_reports(dirs, app_name);
    let mut lines = vec![String::from("[CrashLog] DiagnosticReports")];
    if let Some(app_name) = app_name {
        lines.push(format!("filter: {app_name}"));
    }
    lines.push(format!("matches: {}", reports.len()));
    if reports.is_empty() {
        lines.push(String::from("No matching reports in DiagnosticReports."));
        return lines.join("\n");
    }
    for report in reports.iter().take(limit) {
        lines.push(format!(
            "{}  {}  {}",
            format_system_time(report.modified),
            format_bytes(report.size),
            report
                .path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("report")
        ));
        lines.push(format!("  path: {}", report.path.display()));
    }
    if reports.len() > limit {
        lines.push(format!("... {} more", reports.len() - limit));
    }
    lines.join("\n")
}

fn render_crash_log_latest(app_name: Option<&str>, max_lines: usize, dirs: &[PathBuf]) -> String {
    crash_reports(dirs, app_name).first().map_or_else(
        || {
            app_name.map_or_else(
                || String::from("[CrashLog] No reports found in DiagnosticReports."),
                |name| format!("[CrashLog] No matching reports for {name}."),
            )
        },
        |report| render_crash_log_read(&report.path, max_lines),
    )
}

fn render_crash_log_read(path: &Path, max_lines: usize) -> String {
    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !is_supported_crash_report_extension(&extension) {
        return format!("[CrashLog] Unsupported report extension: {extension}");
    }
    if !path.exists() {
        return format!("[CrashLog] Report not found: {}", path.display());
    }
    let content = read_text_prefix(path, 2_000_000);
    if content.is_empty() {
        return format!(
            "[CrashLog] Report is empty or unreadable: {}",
            path.display()
        );
    }
    let raw_lines = content.lines().map(ToOwned::to_owned).collect::<Vec<_>>();
    let selected = raw_lines.iter().take(max_lines);
    let mut lines = vec![format!("[CrashLog] {}", path.display())];
    let summary = summarize_crash_report(&content);
    if !summary.is_empty() {
        lines.push(String::from("summary:"));
        lines.extend(summary.into_iter().map(|line| format!("  {line}")));
    }
    lines.push(String::from("content:"));
    lines.extend(selected.cloned());
    if raw_lines.len() > max_lines {
        lines.push(format!(
            "... [{} lines truncated]",
            raw_lines.len() - max_lines
        ));
    }
    lines.join("\n")
}

fn crash_reports(dirs: &[PathBuf], app_name: Option<&str>) -> Vec<CrashReport> {
    let query = app_name.map(|value| value.to_ascii_lowercase());
    let mut reports = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let extension = path
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !is_supported_crash_report_extension(&extension)
                || !crash_report_matches(&path, query.as_deref())
            {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            reports.push(CrashReport {
                path,
                modified: metadata
                    .modified()
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                size: metadata.len(),
            });
        }
    }
    reports.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| left.path.cmp(&right.path))
    });
    reports
}

fn is_supported_crash_report_extension(extension: &str) -> bool {
    matches!(
        extension,
        "crash" | "ips" | "diag" | "spin" | "hang" | "panic"
    )
}

fn crash_report_matches(path: &Path, query: Option<&str>) -> bool {
    let Some(query) = query.filter(|value| !value.is_empty()) else {
        return true;
    };
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().contains(query))
    {
        return true;
    }
    let prefix = read_text_prefix(path, 64_000).to_ascii_lowercase();
    prefix.lines().take(80).any(|line| {
        let trimmed = line.trim();
        (trimmed.starts_with("process:")
            || trimmed.starts_with("path:")
            || trimmed.starts_with("identifier:"))
            && trimmed.contains(query)
    })
}

fn read_text_prefix(path: &Path, max_bytes: usize) -> String {
    fs::read(path)
        .map(|bytes| {
            let len = bytes.len().min(max_bytes);
            String::from_utf8_lossy(&bytes[..len]).into_owned()
        })
        .unwrap_or_default()
}

fn summarize_crash_report(content: &str) -> Vec<String> {
    let prefixes = [
        "Incident Identifier:",
        "Process:",
        "Path:",
        "Identifier:",
        "Version:",
        "Code Type:",
        "Parent Process:",
        "Date/Time:",
        "OS Version:",
        "Exception Type:",
        "Exception Codes:",
        "Termination Reason:",
        "Crashed Thread:",
    ];
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            prefixes
                .iter()
                .any(|prefix| trimmed.starts_with(prefix))
                .then(|| trimmed.to_owned())
        })
        .take(24)
        .collect()
}

fn escape_log_predicate(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(target_os = "macos")]
fn run_mac_log_query(query: &MacLogQuery) -> String {
    let args = query.arguments();
    let output = Command::new("/usr/bin/log")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    match output {
        Ok(output) => {
            let mut raw = String::from_utf8_lossy(&output.stdout).into_owned();
            raw.push_str(&String::from_utf8_lossy(&output.stderr));
            render_mac_log_output(&raw, query, output.status.code().unwrap_or(1), false)
        }
        Err(error) => render_mac_log_output(
            &format!("failed to start /usr/bin/log: {error}"),
            query,
            1,
            false,
        ),
    }
}

#[cfg(any(test, target_os = "macos"))]
fn render_mac_log_output(
    raw_output: &str,
    query: &MacLogQuery,
    status: i32,
    timed_out: bool,
) -> String {
    let mut lines = vec![format!(
        "[MacLog] /usr/bin/log {}",
        query.arguments().join(" ")
    )];
    if timed_out {
        lines.push(format!("status: timed out after {}ms", query.timeout_ms));
    } else {
        lines.push(format!("status: {status}"));
    }
    let raw_lines = raw_output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if status != 0 && !timed_out {
        lines.push(String::from("error:"));
        lines.extend(raw_lines.into_iter().take(query.limit));
        return lines.join("\n");
    }
    if raw_lines.is_empty() {
        lines.push(String::from(
            "No matching log entries. Widen last_minutes or level only if the failure is expected to be recent.",
        ));
        return lines.join("\n");
    }
    lines.push(format!(
        "matches_returned: {}",
        raw_lines.len().min(query.limit)
    ));
    lines.extend(raw_lines.iter().take(query.limit).cloned());
    if raw_lines.len() > query.limit {
        lines.push(format!(
            "... [{} lines truncated]",
            raw_lines.len() - query.limit
        ));
    }
    lines.join("\n")
}

fn is_valid_mac_diagnose_scenario(scenario: &str) -> bool {
    matches!(
        scenario,
        "general"
            | "crash"
            | "permission"
            | "screen"
            | "audio"
            | "keychain"
            | "network"
            | "install"
            | "performance"
    )
}

fn render_mac_diagnose(scenario: &str, app_name: &str, signal_rows: &[(&str, &str)]) -> String {
    let scenario = if is_valid_mac_diagnose_scenario(scenario) {
        scenario
    } else {
        "general"
    };
    let mut lines = vec![format!("[MacDiagnose] scenario={scenario} app={app_name}")];
    if signal_rows.is_empty() {
        lines.push(String::from(
            "signals: no warnings from supplied live diagnostics",
        ));
    } else {
        lines.push(String::from("signals:"));
        lines.extend(
            signal_rows
                .iter()
                .take(10)
                .map(|(label, value)| format!("- {label}: {value}")),
        );
        if signal_rows.len() > 10 {
            lines.push(format!("- ... {} more warnings", signal_rows.len() - 10));
        }
    }
    lines.push(String::from("recommended_path:"));
    lines.extend(
        mac_diagnose_steps(scenario, app_name)
            .iter()
            .enumerate()
            .map(|(index, step)| format!("{}. {step}", index + 1)),
    );
    lines.push(String::from("agent_protocol:"));
    lines.push(String::from(
        "- Treat this as routing guidance, not evidence.",
    ));
    lines.push(String::from(
        "- Use the named tools to collect current evidence before making claims.",
    ));
    lines.push(String::from(
        "- Prefer file-first verification: source files, git state, command output, and tests decide fixes.",
    ));
    lines.push(String::from(
        "- Keep log and crash reads bounded; widen only after the narrow query is empty.",
    ));
    lines.join("\n")
}

fn mac_diagnose_steps(scenario: &str, app_name: &str) -> Vec<String> {
    match scenario {
        "crash" => vec![
            format!(
                "CrashLog list app_name: {app_name}, then CrashLog latest app_name: {app_name} if a recent report exists."
            ),
            format!(
                "MacLog process: {app_name} level: error_or_fault last_minutes: 30 for failures without a crash report."
            ),
            String::from(
                "Read the implicated source files and reproduce with the smallest command or test.",
            ),
        ],
        "permission" => vec![
            format!(
                "Run MacLog subsystem: com.apple.TCC contains: {app_name} level: default last_minutes: 60."
            ),
            String::from(
                "Check tide doctor macOS Native rows for Screen Recording, Accessibility, Microphone, Speech Recognition, and AppleScript.",
            ),
            String::from(
                "Use ScreenCapture list/capture, Clipboard inspect, or AudioTranscribe only for the exact denied capability.",
            ),
        ],
        "screen" => vec![
            String::from("Check Screen Recording in tide doctor or MacNative diagnostics."),
            String::from(
                "Use ScreenCapture list to identify the target window, then ScreenCapture capture with auto_trim/enhance_text when UI text matters.",
            ),
            String::from(
                "Use Vision layout/OCR or ImagePreprocess inspect before asking the model to reason about dense screenshots.",
            ),
        ],
        "audio" => vec![
            String::from("Check Microphone and Speech Recognition rows in tide doctor."),
            String::from(
                "Use MacLog contains: Speech or contains: microphone level: default last_minutes: 60 if permissions look wrong.",
            ),
            String::from(
                "Use AudioTranscribe on a concrete file path; do not infer audio contents from metadata.",
            ),
        ],
        "keychain" => vec![
            String::from(
                "Run tide doctor Auth rows first: provider, base URL, env key, DeepSeek key, Paean token.",
            ),
            String::from(
                "Use MacLog contains: keychain level: error_or_fault last_minutes: 30 for native Keychain failures.",
            ),
            String::from(
                "Use tide login or tide auth login only after confirming which provider path is active.",
            ),
        ],
        "network" => vec![
            String::from("Run tide doctor Auth Reachability and Base URL rows first."),
            String::from(
                "Use Bash for a bounded curl -I to the active endpoint when reachability is unclear.",
            ),
            String::from("Check APIError remediation and settings layers before changing code."),
        ],
        "install" => vec![
            String::from(
                "Run tide doctor System and Storage rows: PATH shadows, code signing, quarantine, settings layers.",
            ),
            String::from(
                "Use FileMetadata on the active binary and any shadowed binary to inspect xattrs/quarantine/provenance.",
            ),
            String::from(
                "Use Bash command -v tide and tide --version to verify the shell resolves the intended binary.",
            ),
        ],
        "performance" => vec![
            String::from("Check tide doctor Power and Low Power Mode rows."),
            String::from("Use /cost or /status for KV cache health and token/cost shape."),
            format!(
                "Use MacLog process: {app_name} level: error_or_fault last_minutes: 30 for system throttling or repeated failures."
            ),
        ],
        _ => vec![
            String::from(
                "Run tide doctor or MacDiagnose with a narrower scenario if the symptom class is known.",
            ),
            String::from(
                "Check Recent Diagnostics for crash/log warnings, then use CrashLog or MacLog only when those warnings exist.",
            ),
            String::from(
                "Use FileMetadata, Clipboard inspect, ScreenCapture, Vision, or AudioTranscribe only when the user references a concrete file/UI/clipboard/audio artifact.",
            ),
        ],
    }
}

#[cfg(test)]
mod mac_diagnostic_tests {
    use super::{
        MacLogQuery, render_crash_log_latest, render_crash_log_list, render_crash_log_read,
        render_mac_diagnose, render_mac_log_output,
    };
    use std::fs;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn crash_log_list_filters_and_sorts_reports() {
        let temp = tempfile::tempdir().expect("tempdir");
        let older = temp.path().join("Other_2026-05-09.crash");
        let newer = temp.path().join("Tide_2026-05-09-101010_Mac.crash");
        let ignored = temp.path().join("Tide.txt");
        fs::write(&older, "Process: Other\n").expect("write older");
        fs::write(&newer, "Process: Tide\nException Type: EXC_BAD_ACCESS\n").expect("write newer");
        fs::write(&ignored, "not a report").expect("write ignored");

        let output = render_crash_log_list(Some("tide"), 10, &[temp.path().to_path_buf()]);

        assert!(output.contains("matches: 1"));
        assert!(output.contains("Tide_2026-05-09-101010_Mac.crash"));
        assert!(!output.contains("Other_2026-05-09.crash"));
        assert!(!output.contains("Tide.txt"));
    }

    #[test]
    fn crash_log_latest_reads_newest_matching_report() {
        let temp = tempfile::tempdir().expect("tempdir");
        let older = temp.path().join("Tide_older.crash");
        let newer = temp.path().join("Tide_newer.crash");
        fs::write(&older, "Process: Tide\nException Type: OLD\n").expect("write older");
        thread::sleep(Duration::from_millis(20));
        fs::write(&newer, "Process: Tide\nException Type: NEW\n").expect("write newer");

        let output = render_crash_log_latest(Some("tide"), 20, &[temp.path().to_path_buf()]);

        assert!(output.contains("Tide_newer.crash"));
        assert!(output.contains("Exception Type: NEW"));
        assert!(!output.contains("Exception Type: OLD"));
    }

    #[test]
    fn crash_log_read_summarizes_and_truncates_report() {
        let temp = tempfile::tempdir().expect("tempdir");
        let report = temp.path().join("Tide_2026-05-09.ips");
        fs::write(
            &report,
            "Incident Identifier: ABCD\nProcess: Tide [123]\nPath: /Applications/Tide.app/Contents/MacOS/Tide\nException Type: EXC_BAD_ACCESS (SIGSEGV)\nCrashed Thread: 4\nline one\nline two\nline three\n",
        )
        .expect("write report");

        let output = render_crash_log_read(&report, 3);

        assert!(output.contains("summary:"));
        assert!(output.contains("Process: Tide [123]"));
        assert!(output.contains("Exception Type: EXC_BAD_ACCESS"));
        assert!(output.contains("... [5 lines truncated]"));
    }

    #[test]
    fn mac_log_query_escapes_and_combines_filters() {
        let query = MacLogQuery {
            process: Some(String::from("deeptide")),
            subsystem: Some(String::from("ai.a8e.tide")),
            category: Some(String::from("agent")),
            contains: Some(String::from("permission \"denied\"")),
            level: String::from("error_or_fault"),
            last_minutes: 15,
            limit: 80,
            timeout_ms: 8000,
        };

        assert_eq!(
            query.predicate(),
            "process == \"deeptide\" AND subsystem == \"ai.a8e.tide\" AND category == \"agent\" AND eventMessage CONTAINS[c] \"permission \\\"denied\\\"\" AND (messageType == error OR messageType == fault)"
        );
    }

    #[test]
    fn mac_log_renderer_limits_output_and_reports_empty_results() {
        let query = MacLogQuery {
            process: Some(String::from("deeptide")),
            subsystem: None,
            category: None,
            contains: None,
            level: String::from("default"),
            last_minutes: 15,
            limit: 2,
            timeout_ms: 8000,
        };
        let output = render_mac_log_output("line 1\nline 2\nline 3\n", &query, 0, false);

        assert!(output.contains("matches_returned: 2"));
        assert!(output.contains("line 1"));
        assert!(output.contains("line 2"));
        assert!(!output.contains("line 3"));
        assert!(output.contains("... [1 lines truncated]"));

        let empty = render_mac_log_output("", &query, 0, false);
        assert!(empty.contains("No matching log entries"));
    }

    #[test]
    fn mac_diagnose_routes_crash_and_permission_scenarios() {
        let crash = render_mac_diagnose("crash", "Tide", &[]);
        assert!(crash.contains("[MacDiagnose] scenario=crash app=Tide"));
        assert!(crash.contains("1. CrashLog list app_name: Tide"));
        assert!(crash.contains("2. MacLog process: Tide"));
        assert!(crash.contains("file-first verification"));

        let permission = render_mac_diagnose("permission", "deeptide", &[]);
        assert!(permission.contains("MacLog subsystem: com.apple.TCC"));
        assert!(permission.contains("Screen Recording"));
        assert!(permission.contains("Microphone"));
    }
}
