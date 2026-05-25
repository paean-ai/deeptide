use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolResult {
    pub content: String,
    pub is_error: bool,
}

impl ToolResult {
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: false,
        }
    }

    pub fn error(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            is_error: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolContext {
    pub cwd: PathBuf,
}

impl ToolContext {
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self { cwd: cwd.into() }
    }

    pub fn resolve_path(&self, path: &str) -> PathBuf {
        let expanded = expand_home(path);
        let path = PathBuf::from(expanded);
        if path.is_absolute() {
            normalize_path(path)
        } else {
            normalize_path(self.cwd.join(path))
        }
    }
}

pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;
    fn is_read_only(&self) -> bool;
    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult;
}

#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<&'static str, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_builtin_tools() -> Self {
        let mut registry = Self::new();
        registry.register(Box::<ReadTool>::default());
        registry.register(Box::<ReadFilesTool>::default());
        registry.register(Box::<GlobTool>::default());
        registry.register(Box::<GrepTool>::default());
        registry.register(Box::<WriteTool>::default());
        registry.register(Box::<EditTool>::default());
        registry.register(Box::<BashTool>::default());
        registry.register(Box::<TodoWriteTool>::default());
        registry
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(Box::as_ref)
    }

    pub fn call(&self, name: &str, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(tool) = self.get(name) else {
            return ToolResult::error(format!("Unknown tool: {name}"));
        };
        tool.call(input, context)
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReadTool;

impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "Read"
    }

    fn description(&self) -> &'static str {
        "Read a text file with optional line offset and limit."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing file_path parameter");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }

        let path = context.resolve_path(file_path);
        let offset = input
            .get("offset")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok());
        let limit = input
            .get("limit")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok());

        read_text_file(&path, offset, limit)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReadFilesTool;

impl Tool for ReadFilesTool {
    fn name(&self) -> &'static str {
        "ReadFiles"
    }

    fn description(&self) -> &'static str {
        "Read multiple text files in one ordered result."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(paths) = input.get("paths").and_then(serde_json::Value::as_array) else {
            return ToolResult::error("paths must be an array");
        };
        if paths.is_empty() {
            return ToolResult::error("No paths provided");
        }
        if paths.len() > 50 {
            return ToolResult::error("paths array exceeds 50 entries; split into smaller batches");
        }

        let mut sections = Vec::new();
        let mut estimated_tokens = 0usize;
        for path_value in paths {
            let Some(file_path) = path_value.as_str() else {
                sections.push(String::from(
                    "===== <invalid> =====\n[Error: path must be a string]",
                ));
                continue;
            };
            let path = context.resolve_path(file_path);
            let section = match read_text_file_limited(&path, Some(2_000)) {
                Ok(content) => format!("===== {file_path} =====\n{content}"),
                Err(message) => format!("===== {file_path} =====\n[Error: {message}]"),
            };
            estimated_tokens += estimate_tokens(&section);
            if estimated_tokens > 60_000 {
                sections.push(format!(
                    "===== {file_path} =====\n[Skipped: total output cap reached (60000 tokens)]"
                ));
                break;
            }
            sections.push(section);
        }

        ToolResult::text(sections.join("\n\n"))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GlobTool;

impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "Glob"
    }

    fn description(&self) -> &'static str {
        "Find files by glob pattern."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(pattern) = input.get("pattern").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing pattern parameter");
        };
        if pattern.trim().is_empty() {
            return ToolResult::error("pattern is required");
        }

        let base = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(|path| context.resolve_path(path))
            .unwrap_or_else(|| context.cwd.clone());
        if !base.is_dir() {
            return ToolResult::error(format!("Path is not a directory: {}", base.display()));
        }

        let matcher = GlobMatcher::new(pattern);
        let mut matches = Vec::new();
        collect_files(&base, &base, &mut |relative, _full_path| {
            if matcher.matches(relative) {
                matches.push(relative.to_string_lossy().replace('\\', "/"));
            }
            matches.len() < 100
        });

        matches.sort();
        let truncated = matches.len() >= 100;
        let output = matches.join("\n");
        let suffix = if truncated {
            "\n\n[Results truncated at 100]"
        } else {
            ""
        };
        ToolResult::text(format!(
            "Found {} file{}\n\n{}{}",
            matches.len(),
            if matches.len() == 1 { "" } else { "s" },
            output,
            suffix
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct GrepTool;

impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "Grep"
    }

    fn description(&self) -> &'static str {
        "Search text files using a regular expression."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(pattern) = input.get("pattern").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing pattern parameter");
        };
        if pattern.trim().is_empty() {
            return ToolResult::error("pattern is required");
        }

        let pattern = if input.get("-i").and_then(serde_json::Value::as_bool) == Some(true) {
            format!("(?i){pattern}")
        } else {
            pattern.to_owned()
        };
        let regex = match regex::Regex::new(&pattern) {
            Ok(regex) => regex,
            Err(error) => return ToolResult::error(format!("Invalid regex pattern: {error}")),
        };
        let base = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(|path| context.resolve_path(path))
            .unwrap_or_else(|| context.cwd.clone());
        let output_mode = input
            .get("output_mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("files_with_matches");
        let head_limit = input
            .get("head_limit")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(250);
        let glob = input
            .get("glob")
            .and_then(serde_json::Value::as_str)
            .map(GlobMatcher::new);

        grep_path(&base, &base, &regex, output_mode, glob.as_ref(), head_limit)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct BashTool;

impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "Bash"
    }

    fn description(&self) -> &'static str {
        "Execute a single-line shell command in the current workspace."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(command) = input.get("command").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing command parameter");
        };
        if command.trim().is_empty() {
            return ToolResult::error("command is required and must be non-empty");
        }
        if command.contains('\n') || command.contains('\r') {
            return ToolResult::error("command must be a single line; rewrite it with && or ;");
        }

        let timeout_ms = input
            .get("timeout")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(120_000)
            .min(600_000);
        if input
            .get("timeout")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|timeout| timeout > 600_000)
        {
            return ToolResult::error("timeout must be <= 600000ms (10 minutes)");
        }

        if input
            .get("run_in_background")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            return start_background_command(command, context);
        }

        execute_shell_command(command, context, Duration::from_millis(timeout_ms))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TodoWriteTool;

impl Tool for TodoWriteTool {
    fn name(&self) -> &'static str {
        "TodoWrite"
    }

    fn description(&self) -> &'static str {
        "Replace the in-memory todo list in one call."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(raw_items) = input.get("todos").and_then(serde_json::Value::as_array) else {
            return ToolResult::error("Missing or invalid todos array");
        };

        let parsed = raw_items
            .iter()
            .filter_map(parse_todo_item)
            .collect::<Vec<_>>();
        let all_done = !parsed.is_empty()
            && parsed
                .iter()
                .all(|item| item.status == TodoStatus::Completed);
        replace_todos(parsed.clone());

        if all_done {
            return ToolResult::text(
                "Todo list cleared (all tasks completed). Proceed with your summary.",
            );
        }

        ToolResult::text(format!(
            "Todo list updated ({} items). Ensure that you continue to use the todo list to track your progress. Please proceed with the current tasks if applicable.",
            parsed.len()
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TodoItem {
    content: String,
    status: TodoStatus,
    active_form: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Deleted,
}

fn parse_todo_item(value: &serde_json::Value) -> Option<TodoItem> {
    let object = value.as_object()?;
    let content = object.get("content")?.as_str()?.to_owned();
    let status = object
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(TodoStatus::parse)
        .unwrap_or(TodoStatus::Pending);
    let active_form = object
        .get("activeForm")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);
    Some(TodoItem {
        content,
        status,
        active_form,
    })
}

impl TodoStatus {
    fn parse(value: &str) -> Self {
        match value {
            "in_progress" => Self::InProgress,
            "completed" => Self::Completed,
            "deleted" => Self::Deleted,
            _ => Self::Pending,
        }
    }
}

fn todo_storage() -> &'static Mutex<Vec<TodoItem>> {
    static STORAGE: OnceLock<Mutex<Vec<TodoItem>>> = OnceLock::new();
    STORAGE.get_or_init(|| Mutex::new(Vec::new()))
}

fn replace_todos(items: Vec<TodoItem>) {
    if let Ok(mut todos) = todo_storage().lock() {
        *todos = items;
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WriteTool;

impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "Write"
    }

    fn description(&self) -> &'static str {
        "Write complete UTF-8 file contents, creating parent directories as needed."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error(
                "Write requires JSON with both `file_path` and `content`. Example: {\"file_path\":\"roguelike.html\",\"content\":\"<complete file contents>\"}. Do not call Write with `{}`. Retry with the exact requested file path and complete content.",
            );
        };
        if file_path.trim().is_empty() {
            return ToolResult::error(
                "Write requires JSON with both `file_path` and `content`. Example: {\"file_path\":\"roguelike.html\",\"content\":\"<complete file contents>\"}. Do not call Write with `{}`. Retry with the exact requested file path and complete content.",
            );
        }

        let Some(content) = input.get("content").and_then(serde_json::Value::as_str) else {
            return ToolResult::error(
                "Write requires a string `content` field containing the complete file contents. Retry with JSON keys exactly `file_path` and `content`.",
            );
        };

        let path = context.resolve_path(file_path);
        let existed = path.exists();
        if let Some(parent) = path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            return ToolResult::error(format!(
                "Failed to create parent directory {}: {error}",
                parent.display()
            ));
        }

        let normalized = normalize_line_endings(content);
        if let Err(error) = fs::write(&path, normalized.as_bytes()) {
            return ToolResult::error(format!("Failed to write {}: {error}", path.display()));
        }

        let action = if existed { "Updated" } else { "Created" };
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(file_path);
        ToolResult::text(format!(
            "{action} file: {name} ({})\nPath: {}",
            format_byte_count(normalized.len()),
            path.display()
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EditTool;

impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "Edit"
    }

    fn description(&self) -> &'static str {
        "Perform exact string replacement in an existing UTF-8 file."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }
        let Some(old_string) = input.get("old_string").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("old_string is required");
        };
        let Some(new_string) = input.get("new_string").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("new_string is required");
        };
        if old_string == new_string {
            return ToolResult::error("old_string and new_string must be different");
        }

        let path = context.resolve_path(file_path);
        let replace_all = input
            .get("replace_all")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);

        if !path.exists() && old_string.is_empty() {
            if let Some(parent) = path.parent()
                && let Err(error) = fs::create_dir_all(parent)
            {
                return ToolResult::error(format!(
                    "Failed to create parent directory {}: {error}",
                    parent.display()
                ));
            }
            let normalized_new = normalize_line_endings(new_string);
            if let Err(error) = fs::write(&path, normalized_new.as_bytes()) {
                return ToolResult::error(format!("Failed to write {}: {error}", path.display()));
            }
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(file_path);
            return ToolResult::text(format!("Created new file: {name}"));
        }

        let original = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return ToolResult::error(format!("File does not exist: {file_path}"));
            }
            Err(error) => {
                return ToolResult::error(format!("Failed to read {}: {error}", path.display()));
            }
        };
        if old_string.is_empty() {
            return ToolResult::error(
                "old_string not found in file. The file may have been modified. Please re-read the file.",
            );
        }

        let (edited, matches) =
            match apply_exact_edit(&original, old_string, new_string, replace_all) {
                Ok(result) => result,
                Err(message) => return ToolResult::error(message),
            };
        if let Err(error) = fs::write(&path, edited.as_bytes()) {
            return ToolResult::error(format!("Failed to write {}: {error}", path.display()));
        }

        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(file_path);
        let occurrence = if replace_all && matches > 1 {
            format!("all {matches} occurrences")
        } else {
            String::from("1 occurrence")
        };
        ToolResult::text(format!("File edited successfully: {name} ({occurrence})"))
    }
}

fn read_text_file(path: &Path, offset: Option<usize>, limit: Option<usize>) -> ToolResult {
    let content = match read_text_file_limited(path, None) {
        Ok(content) => content,
        Err(message) => return ToolResult::error(message),
    };

    let start = offset.unwrap_or(1).saturating_sub(1);
    let needed = limit.unwrap_or(usize::MAX);
    let output = content
        .lines()
        .skip(start)
        .take(needed)
        .collect::<Vec<_>>()
        .join("\n");

    ToolResult::text(output)
}

fn read_text_file_limited(path: &Path, line_limit: Option<usize>) -> Result<String, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            return Err(format!(
                "File does not exist: {}\nHint: the file may be at a different location. Use Glob(pattern: \"**/{filename}\") to locate it.",
                path.display()
            ));
        }
        Err(error) => {
            return Err(format!("Failed to inspect {}: {error}", path.display()));
        }
    };

    if metadata.is_dir() {
        return Err(format!(
            "Path is a directory: {}\nUse Glob to list files or Grep to search within it.",
            path.display()
        ));
    }

    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return Err(format!("Failed to open {}: {error}", path.display()));
        }
    };

    let mut selected = Vec::new();
    let mut total_lines = 0usize;

    for line in io::BufReader::new(file).lines() {
        total_lines += 1;
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                return Err(format!("Failed to read {}: {error}", path.display()));
            }
        };

        if line_limit.is_none_or(|limit| selected.len() < limit) {
            selected.push((total_lines, line));
        }
        if line_limit.is_some_and(|limit| selected.len() >= limit) {
            break;
        }
    }

    let mut output = selected
        .into_iter()
        .map(|(line_number, line)| format!("{line_number}\t{line}"))
        .collect::<Vec<_>>()
        .join("\n");
    if let Some(limit) = line_limit
        && total_lines >= limit
    {
        output.push_str(&format!("\n[File truncated at {limit} lines]"));
    }

    Ok(output)
}

fn execute_shell_command(command: &str, context: &ToolContext, timeout: Duration) -> ToolResult {
    let mut child = match shell_command(command)
        .current_dir(&context.cwd)
        .env_remove("TIDE_API_KEY")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("ZERO_CLI_API_KEY")
        .env_remove("ZERO_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return ToolResult::error(format!("Failed to execute command: {error}"));
        }
    };

    let started = Instant::now();
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_status)) => break,
            Ok(None) if started.elapsed() >= timeout => {
                timed_out = true;
                let _ = child.kill();
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => return ToolResult::error(format!("Failed to wait for command: {error}")),
        }
    }

    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            return ToolResult::error(format!("Failed to collect command output: {error}"));
        }
    };

    render_command_output(
        command,
        output.status.code(),
        timed_out,
        &output.stdout,
        &output.stderr,
    )
}

fn start_background_command(command: &str, context: &ToolContext) -> ToolResult {
    match shell_command(command)
        .current_dir(&context.cwd)
        .env_remove("TIDE_API_KEY")
        .env_remove("DEEPSEEK_API_KEY")
        .env_remove("ZERO_CLI_API_KEY")
        .env_remove("ZERO_API_KEY")
        .env_remove("ANTHROPIC_API_KEY")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(_child) => ToolResult::text(format!(
            "Command started in background: `{}`",
            truncate_chars(command, 100)
        )),
        Err(error) => ToolResult::error(format!("Failed to execute command: {error}")),
    }
}

fn shell_command(command: &str) -> Command {
    #[cfg(windows)]
    {
        let mut process = Command::new("cmd");
        process.args(["/C", command]);
        process
    }
    #[cfg(not(windows))]
    {
        let mut process = Command::new("sh");
        process.args(["-c", command]);
        process
    }
}

fn render_command_output(
    _command: &str,
    exit_code: Option<i32>,
    timed_out: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> ToolResult {
    let stdout = String::from_utf8_lossy(stdout);
    let stderr = String::from_utf8_lossy(stderr);
    let mut output = String::new();
    if timed_out {
        output.push_str("[Timed out after configured timeout - process killed]\n");
    }
    if !stdout.is_empty() {
        output.push_str(&stdout);
    }
    if !stderr.is_empty() {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str("[stderr]\n");
        output.push_str(&stderr);
    }

    let failed = timed_out || exit_code.unwrap_or(1) != 0;
    if output.is_empty() && failed {
        output = format!(
            "Command exited with code {} (no output)",
            exit_code.unwrap_or(-1)
        );
    }

    let output = truncate_command_output(&output, 30_000, 500);
    if failed {
        ToolResult::error(output)
    } else {
        ToolResult::text(output)
    }
}

fn truncate_command_output(output: &str, max_chars: usize, max_lines: usize) -> String {
    let mut truncated = output
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n");
    if output.ends_with('\n') && output.lines().count() <= max_lines {
        truncated.push('\n');
    }
    if truncated.chars().count() > max_chars {
        truncated = truncate_chars(&truncated, max_chars);
        truncated.push_str("\n[Output truncated]");
    } else if output.lines().count() > max_lines {
        truncated.push_str("\n[Output truncated]");
    }
    truncated
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_owned();
    }
    value.chars().take(max_chars).collect()
}

fn estimate_tokens(value: &str) -> usize {
    value.len().div_ceil(4)
}

fn normalize_line_endings(content: &str) -> String {
    content.replace("\r\n", "\n").replace('\r', "\n")
}

fn format_byte_count(bytes: usize) -> String {
    if bytes == 1 {
        String::from("1 byte")
    } else {
        format!("{bytes} bytes")
    }
}

fn apply_exact_edit(
    content: &str,
    old_string: &str,
    new_string: &str,
    replace_all: bool,
) -> Result<(String, usize), String> {
    let matches = content.matches(old_string).count();
    if matches == 0 {
        let normalized_content = normalize_quotes(content);
        let normalized_old = normalize_quotes(old_string);
        let normalized_matches = normalized_content.matches(&normalized_old).count();
        if normalized_matches == 0 {
            return Err(String::from(
                "old_string not found in file. The file may have been modified. Please re-read the file.",
            ));
        }
        if normalized_matches > 1 && !replace_all {
            return Err(format!(
                "old_string matches {normalized_matches} locations. Use replace_all=true or provide a more specific string."
            ));
        }

        let normalized_new = normalize_quotes(new_string);
        let edited = if replace_all {
            normalized_content.replace(&normalized_old, &normalized_new)
        } else {
            normalized_content.replacen(&normalized_old, &normalized_new, 1)
        };
        return Ok((edited, normalized_matches));
    }

    if matches > 1 && !replace_all {
        return Err(format!(
            "old_string matches {matches} locations. Use replace_all=true or provide a more specific string."
        ));
    }

    let edited = if replace_all {
        content.replace(old_string, new_string)
    } else {
        content.replacen(old_string, new_string, 1)
    };
    Ok((edited, matches))
}

fn normalize_quotes(content: &str) -> String {
    content
        .replace(['\u{201c}', '\u{201d}'], "\"")
        .replace(['\u{2018}', '\u{2019}'], "'")
}

fn grep_path(
    base: &Path,
    path: &Path,
    regex: &regex::Regex,
    output_mode: &str,
    glob: Option<&GlobMatcher>,
    head_limit: usize,
) -> ToolResult {
    if path.is_file() {
        return grep_file(
            path.parent().unwrap_or(base),
            path,
            regex,
            output_mode,
            head_limit,
        );
    }
    if !path.is_dir() {
        return ToolResult::error(format!("Path does not exist: {}", path.display()));
    }

    let mut matching_files = BTreeMap::<String, usize>::new();
    let mut content_matches = Vec::new();
    collect_files(path, path, &mut |relative, full_path| {
        if let Some(glob) = glob
            && !glob.matches(relative)
        {
            return true;
        }
        match grep_file_matches(path, full_path, regex) {
            Ok(matches) if !matches.is_empty() => {
                let relative = relative.to_string_lossy().replace('\\', "/");
                matching_files.insert(relative.clone(), matches.len());
                for (line_number, line) in matches {
                    content_matches.push(format!("{relative}:{line_number}:{line}"));
                }
            }
            Ok(_) => {}
            Err(_) => {}
        }
        head_limit == 0 || matching_files.len().max(content_matches.len()) < head_limit
    });

    render_grep_output(
        output_mode,
        matching_files,
        content_matches,
        head_limit,
        base,
    )
}

fn grep_file(
    base: &Path,
    path: &Path,
    regex: &regex::Regex,
    output_mode: &str,
    head_limit: usize,
) -> ToolResult {
    match grep_file_matches(base, path, regex) {
        Ok(matches) => {
            let relative = path
                .strip_prefix(base)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            let mut matching_files = BTreeMap::new();
            let mut content_matches = Vec::new();
            if !matches.is_empty() {
                matching_files.insert(relative.clone(), matches.len());
                for (line_number, line) in matches {
                    content_matches.push(format!("{relative}:{line_number}:{line}"));
                }
            }
            render_grep_output(
                output_mode,
                matching_files,
                content_matches,
                head_limit,
                base,
            )
        }
        Err(error) => ToolResult::error(format!("Failed to search {}: {error}", path.display())),
    }
}

fn grep_file_matches(
    base: &Path,
    path: &Path,
    regex: &regex::Regex,
) -> io::Result<Vec<(usize, String)>> {
    let file = fs::File::open(path)?;
    let mut matches = Vec::new();
    for (index, line) in io::BufReader::new(file).lines().enumerate() {
        let line = line?;
        if regex.is_match(&line) {
            matches.push((index + 1, line));
        }
    }
    let _ = base;
    Ok(matches)
}

fn render_grep_output(
    output_mode: &str,
    matching_files: BTreeMap<String, usize>,
    content_matches: Vec<String>,
    head_limit: usize,
    _base: &Path,
) -> ToolResult {
    match output_mode {
        "content" => {
            let lines = limit_lines(content_matches, head_limit);
            if lines.is_empty() {
                ToolResult::text("No matches found")
            } else {
                ToolResult::text(lines.join("\n"))
            }
        }
        "count" => {
            let lines = matching_files
                .into_iter()
                .map(|(path, count)| format!("{path}:{count}"))
                .collect::<Vec<_>>();
            ToolResult::text(limit_lines(lines, head_limit).join("\n"))
        }
        _ => {
            let lines = matching_files.into_keys().collect::<Vec<_>>();
            ToolResult::text(format!(
                "Found {} file{}\n\n{}",
                lines.len(),
                if lines.len() == 1 { "" } else { "s" },
                limit_lines(lines, head_limit).join("\n")
            ))
        }
    }
}

fn limit_lines(lines: Vec<String>, head_limit: usize) -> Vec<String> {
    if head_limit == 0 {
        lines
    } else {
        lines.into_iter().take(head_limit).collect()
    }
}

fn collect_files(base: &Path, path: &Path, visit: &mut impl FnMut(&Path, &Path) -> bool) -> bool {
    let entries = match fs::read_dir(path) {
        Ok(entries) => entries,
        Err(_) => return true,
    };

    for entry in entries.flatten() {
        let full_path = entry.path();
        let relative = full_path.strip_prefix(base).unwrap_or(&full_path);
        if is_vcs_path(relative) {
            continue;
        }
        if full_path.is_dir() {
            if !collect_files(base, &full_path, visit) {
                return false;
            }
        } else if full_path.is_file() && !visit(relative, &full_path) {
            return false;
        }
    }
    true
}

fn is_vcs_path(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".git" | ".svn" | ".hg" | ".bzr" | ".jj" | ".sl")
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GlobMatcher {
    components: Vec<String>,
}

impl GlobMatcher {
    fn new(pattern: &str) -> Self {
        Self {
            components: pattern.split('/').map(ToOwned::to_owned).collect(),
        }
    }

    fn matches(&self, path: &Path) -> bool {
        let parts = path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .collect::<Vec<_>>();
        match_glob_components(&parts, &self.components)
    }
}

fn match_glob_components(path: &[&str], pattern: &[String]) -> bool {
    if pattern.is_empty() {
        return path.is_empty();
    }
    if pattern[0] == "**" {
        return match_glob_components(path, &pattern[1..])
            || (!path.is_empty() && match_glob_components(&path[1..], pattern));
    }
    if path.is_empty() {
        return false;
    }
    matches_glob_segment(path[0], &pattern[0]) && match_glob_components(&path[1..], &pattern[1..])
}

fn matches_glob_segment(name: &str, pattern: &str) -> bool {
    let name = name.as_bytes();
    let pattern = pattern.as_bytes();
    let (mut ni, mut pi) = (0usize, 0usize);
    let (mut star, mut match_i) = (None, 0usize);
    while ni < name.len() {
        if pi < pattern.len() && (pattern[pi] == b'?' || pattern[pi] == name[ni]) {
            ni += 1;
            pi += 1;
        } else if pi < pattern.len() && pattern[pi] == b'*' {
            star = Some(pi);
            match_i = ni;
            pi += 1;
        } else if let Some(star_i) = star {
            pi = star_i + 1;
            match_i += 1;
            ni = match_i;
        } else {
            return false;
        }
    }
    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }
    pi == pattern.len()
}

fn expand_home(path: &str) -> String {
    if path == "~" {
        return home_dir()
            .map(|home| home.display().to_string())
            .unwrap_or_else(|| path.to_owned());
    }
    if let Some(rest) = path.strip_prefix("~/") {
        return home_dir()
            .map(|home| home.join(rest).display().to_string())
            .unwrap_or_else(|| path.to_owned());
    }
    path.to_owned()
}

fn home_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("USERPROFILE").map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("HOME").map(PathBuf::from)
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    path.components().collect()
}
