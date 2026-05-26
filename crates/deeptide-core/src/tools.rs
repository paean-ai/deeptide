use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write as IoWrite};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Mutex, OnceLock, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use image::codecs::jpeg::JpegEncoder;
use image::codecs::png::PngEncoder;
use image::imageops::{self, FilterType};
use image::{ColorType, DynamicImage, ImageEncoder, RgbaImage};
use reqwest::Url;
use serde::{Deserialize, Serialize};

use crate::memory::{
    MemoryScope, MemorySystem, MemoryType, add_to_memory_index, create_memory_file,
    extract_frontmatter, save_memory, strip_frontmatter,
};

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
        registry.register(Box::<FileMetadataTool>::default());
        registry.register(Box::<ReadFilesTool>::default());
        registry.register(Box::<GlobTool>::default());
        registry.register(Box::<GrepTool>::default());
        registry.register(Box::<WebFetchTool>::default());
        registry.register(Box::<WebSearchTool>::default());
        registry.register(Box::<ToolSearchTool>::default());
        registry.register(Box::<AskUserQuestionTool>::default());
        registry.register(Box::<MemorySearchTool>::default());
        registry.register(Box::<MemoryWriteTool>::default());
        registry.register(Box::<BriefTool>::default());
        registry.register(Box::<CtxInspectTool>::default());
        registry.register(Box::<SnipTool>::default());
        registry.register(Box::<EnterPlanModeTool>::default());
        registry.register(Box::<ExitPlanModeTool>::default());
        registry.register(Box::<LspTool>::default());
        registry.register(Box::<ClipboardTool>::default());
        registry.register(Box::<SpotlightSearchTool>::default());
        registry.register(Box::<ScreenCaptureTool>::default());
        registry.register(Box::<ImagePreprocessTool>::default());
        registry.register(Box::<VisionTool>::default());
        registry.register(Box::<CrashLogTool>::default());
        registry.register(Box::<MacLogTool>::default());
        registry.register(Box::<MacDiagnoseTool>::default());
        registry.register(Box::<CronCreateTool>::default());
        registry.register(Box::<CronListTool>::default());
        registry.register(Box::<CronDeleteTool>::default());
        registry.register(Box::<ReviewArtifactTool>::default());
        registry.register(Box::<SkillTool>::default());
        registry.register(Box::<PublishTool>::default());
        registry.register(Box::<RemoteTriggerTool>::default());
        registry.register(Box::<PushNotificationTool>::default());
        registry.register(Box::<NotebookEditTool>::default());
        registry.register(Box::<EnterWorktreeTool>::default());
        registry.register(Box::<ExitWorktreeTool>::default());
        registry.register(Box::<VerifyPlanExecutionTool>::default());
        registry.register(Box::<SleepTool>::default());
        registry.register(Box::<WriteTool>::default());
        registry.register(Box::<EditTool>::default());
        registry.register(Box::<BashTool>::default());
        registry.register(Box::<MonitorTool>::default());
        registry.register(Box::<TodoWriteTool>::default());
        registry.register(Box::<TaskCreateTool>::default());
        registry.register(Box::<TaskListTool>::default());
        registry.register(Box::<TaskGetTool>::default());
        registry.register(Box::<TaskUpdateTool>::default());
        registry.register(Box::<TaskStopTool>::default());
        registry.register(Box::<TaskOutputTool>::default());
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
pub struct FileMetadataTool;

impl Tool for FileMetadataTool {
    fn name(&self) -> &'static str {
        "FileMetadata"
    }

    fn description(&self) -> &'static str {
        "Inspect file metadata without reading file contents."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }

        let path = context.resolve_path(file_path);
        ToolResult::text(render_file_metadata(&path))
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
        let offset = input
            .get("offset")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let glob = input
            .get("glob")
            .and_then(serde_json::Value::as_str)
            .map(GlobMatcher::new);

        grep_path(
            &base,
            &base,
            &regex,
            output_mode,
            glob.as_ref(),
            head_limit,
            offset,
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WebFetchTool;

impl Tool for WebFetchTool {
    fn name(&self) -> &'static str {
        "WebFetch"
    }

    fn description(&self) -> &'static str {
        "Fetch web content over HTTP or HTTPS and return readable text."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(url_value) = input.get("url").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("url is required");
        };
        if url_value.trim().is_empty() {
            return ToolResult::error("url is required");
        }

        let requested_url = match Url::parse(url_value) {
            Ok(url) if matches!(url.scheme(), "http" | "https") => url,
            _ => return ToolResult::error(format!("Invalid URL: {url_value}")),
        };

        fetch_web_content(&requested_url)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct WebSearchTool;

impl WebSearchTool {
    pub fn call_with_environment(
        &self,
        input: serde_json::Value,
        env: &BTreeMap<String, String>,
    ) -> ToolResult {
        web_search_with_environment(input, env)
    }
}

impl Tool for WebSearchTool {
    fn name(&self) -> &'static str {
        "WebSearch"
    }

    fn description(&self) -> &'static str {
        "Search the web using configured Brave Search or Serper credentials."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let env = std::env::vars().collect::<BTreeMap<_, _>>();
        self.call_with_environment(input, &env)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ToolSearchTool;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ToolSearchEntry {
    name: &'static str,
    summary: String,
    is_read_only: bool,
    is_concurrency_safe: bool,
    keywords: Vec<&'static str>,
}

impl Tool for ToolSearchTool {
    fn name(&self) -> &'static str {
        "ToolSearch"
    }

    fn description(&self) -> &'static str {
        "Search available tools by name or capability."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(raw_query) = input.get("query").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("query is required");
        };
        let query = raw_query.trim();
        if query.is_empty() {
            return ToolResult::error("query is required");
        }

        let max_results = input
            .get("max_results")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(10)
            .clamp(1, 40);
        let entries = builtin_tool_search_entries();

        if query.to_ascii_lowercase().starts_with("select:") {
            return ToolResult::text(render_selected_tools(query, &entries));
        }

        let matches = search_tool_entries(query, &entries);
        if matches.is_empty() {
            return ToolResult::text(
                "No matching tools. Try a tool name, an action, or `select:<ToolName>`.",
            );
        }

        ToolResult::text(
            matches
                .into_iter()
                .take(max_results)
                .map(render_tool_search_entry)
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct AskUserQuestionTool;

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserQuestion {
    question: String,
    header: String,
    options: Vec<UserQuestionOption>,
    multi_select: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct UserQuestionOption {
    label: String,
    description: String,
}

impl Tool for AskUserQuestionTool {
    fn name(&self) -> &'static str {
        "AskUserQuestion"
    }

    fn description(&self) -> &'static str {
        "Ask the user clarifying questions to gather preferences or decisions."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let questions = match parse_user_questions(&input) {
            Ok(questions) => questions,
            Err(message) => return ToolResult::error(message),
        };

        let formatted = questions
            .iter()
            .enumerate()
            .map(|(index, question)| {
                let multi = if question.multi_select {
                    " (multi-select)"
                } else {
                    ""
                };
                let options = question
                    .options
                    .iter()
                    .map(|option| format!("  [{}] {}", option.label, option.description))
                    .collect::<Vec<_>>()
                    .join("\n");
                format!(
                    "Q{}: {}{}\nHeader: {}\n{}",
                    index + 1,
                    question.question,
                    multi,
                    question.header,
                    options
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        ToolResult::text(format!(
            "Questions for the user:\n\n{formatted}\n\nPlease answer each question to help me proceed."
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MemorySearchTool;

impl Tool for MemorySearchTool {
    fn name(&self) -> &'static str {
        "MemorySearch"
    }

    fn description(&self) -> &'static str {
        "Search Deeptide project and global memory files."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(query) = input.get("query").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("query is required");
        };
        let query = query.trim();
        if query.is_empty() {
            return ToolResult::error("query is required");
        }

        let scope = input
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("all");
        if !matches!(scope, "project" | "global" | "all") {
            return ToolResult::error("scope must be project, global, or all");
        }
        let max_results = input
            .get("max_results")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(10)
            .clamp(1, 50);

        search_memory(query, scope, max_results, &context.cwd)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryWriteTool;

impl Tool for MemoryWriteTool {
    fn name(&self) -> &'static str {
        "MemoryWrite"
    }

    fn description(&self) -> &'static str {
        "Persist a concise Deeptide memory shard for future sessions."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(title) = input.get("title").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("title is required");
        };
        let title = safe_inline(title.trim());
        if title.is_empty() {
            return ToolResult::error("title is required");
        }
        if title.chars().count() > 80 {
            return ToolResult::error("title must be 80 characters or fewer");
        }

        let Some(body) = input.get("body").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("body is required");
        };
        let body = body.trim();
        if body.is_empty() {
            return ToolResult::error("body is required");
        }
        if body.chars().count() > 2_000 {
            return ToolResult::error("body must be 2000 characters or fewer");
        }

        let Some(reason) = input.get("reason").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("reason is required");
        };
        let reason = safe_inline(reason.trim());
        if reason.is_empty() {
            return ToolResult::error("reason is required");
        }
        if reason.chars().count() > 240 {
            return ToolResult::error("reason must be 240 characters or fewer");
        }

        let scope = input
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("project");
        let Some(scope) = MemoryScope::parse(scope) else {
            return ToolResult::error("scope must be project or global");
        };
        let memory_type = input
            .get("type")
            .and_then(serde_json::Value::as_str)
            .and_then(parse_memory_type)
            .unwrap_or(match scope {
                MemoryScope::Project => MemoryType::Project,
                MemoryScope::Global => MemoryType::User,
            });

        let file_name = unique_memory_file_name(&title, &context.cwd, scope);
        let content = create_memory_file(&title, &reason, memory_type, body);
        let path = match save_memory(&file_name, &content, &context.cwd, scope) {
            Ok(path) => path,
            Err(error) => return ToolResult::error(format!("Failed to save memory: {error}")),
        };
        if let Err(error) = add_to_memory_index(
            &format!("- [{title}]({file_name}) - {reason}"),
            &context.cwd,
            scope,
        ) {
            return ToolResult::error(format!("Failed to update memory index: {error}"));
        }

        let scope_label = match scope {
            MemoryScope::Project => "project",
            MemoryScope::Global => "global",
        };
        ToolResult::text(format!(
            "Saved {scope_label} memory: {title}\n{}",
            path.display()
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct BriefTool;

impl Tool for BriefTool {
    fn name(&self) -> &'static str {
        "Brief"
    }

    fn description(&self) -> &'static str {
        "Request a context compaction summary of the conversation so far."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, _input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        ToolResult::text(
            "Context compaction triggered. The system will:\n\
             1. Summarize older messages into a compact form\n\
             2. Keep the most recent messages intact\n\
             3. Free context tokens for continued work\n\n\
             Key information (decisions, bugs, current task status) will be preserved in the summary. Continue your task after compaction completes.",
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct CtxInspectTool;

impl Tool for CtxInspectTool {
    fn name(&self) -> &'static str {
        "CtxInspect"
    }

    fn description(&self) -> &'static str {
        "Inspect the current context window budget and cache expectations."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let model = input
            .get("model")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unconfigured");
        let estimated_tokens = input
            .get("estimated_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let message_count = input
            .get("message_count")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let window = model_context_window(model);
        let used_pct = estimated_tokens
            .saturating_mul(100)
            .checked_div(window)
            .unwrap_or(0);
        let remaining = window.saturating_sub(estimated_tokens);

        let mut lines = vec![
            String::from("Context Window Report:"),
            String::new(),
            format!("Model: {model}"),
            format!("Context window: {} tokens", format_compact_number(window)),
            format!(
                "Estimated usage: {} tokens ({}%)",
                format_compact_number(estimated_tokens),
                used_pct
            ),
            format!("Remaining: {} tokens", format_compact_number(remaining)),
            format!("Active messages: {message_count}"),
        ];

        if used_pct > 90 {
            lines.push(String::new());
            lines.push(format!(
                "CRITICAL: Context at {used_pct}% - consider calling Brief or ending the task."
            ));
        } else if used_pct > 70 {
            lines.push(String::new());
            lines.push(format!(
                "WARNING: Context at {used_pct}% - monitor usage on upcoming tool calls."
            ));
        } else if used_pct < 30 {
            lines.push(String::new());
            lines.push(String::from(
                "Context usage is healthy - ample room for multi-step work.",
            ));
        }

        ToolResult::text(lines.join("\n"))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SnipTool;

impl Tool for SnipTool {
    fn name(&self) -> &'static str {
        "Snip"
    }

    fn description(&self) -> &'static str {
        "Request aggressive trimming of older conversation history."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let keep_last = input
            .get("keepLast")
            .or_else(|| input.get("keep_last"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(10)
            .clamp(1, 100);
        let explanation = input
            .get("explanation")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let reason = explanation
            .map(|value| format!("\nReason: {value}\n"))
            .unwrap_or_else(|| String::from("\n"));
        ToolResult::text(format!(
            "History trim requested: keeping last {keep_last} messages.{reason}\nThe system will remove older messages and insert a boundary marker. The agent should continue the task with the remaining context."
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct EnterPlanModeTool;

impl Tool for EnterPlanModeTool {
    fn name(&self) -> &'static str {
        "EnterPlanMode"
    }

    fn description(&self) -> &'static str {
        "Enter plan mode before making significant code changes."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, _input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        ToolResult::text(
            "Plan mode activated. You should now:\n\
             1. Explore the codebase to understand the existing architecture (Read, Grep, Glob, and read-only Bash are allowed)\n\
             2. Identify the files and components involved\n\
             3. Design an implementation approach\n\
             4. Present your plan to the user for approval using ExitPlanMode\n\
             File edits, writes, sub-agents, MCP calls, clipboard writes, and shell commands with side effects are blocked until you exit plan mode or the user changes permission mode. Do not modify project files until the user approves your plan.",
        )
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ExitPlanModeTool;

impl Tool for ExitPlanModeTool {
    fn name(&self) -> &'static str {
        "ExitPlanMode"
    }

    fn description(&self) -> &'static str {
        "Exit plan mode and present the implementation plan for user approval."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let prompts = extract_allowed_prompts(&input);
        let prompt_list = if prompts.is_empty() {
            String::from("(no additional permissions requested)")
        } else {
            prompts
                .into_iter()
                .map(|(tool, prompt)| format!("  - {tool}: {prompt}"))
                .collect::<Vec<_>>()
                .join("\n")
        };

        ToolResult::text(format!(
            "Plan is ready for review. Implementation will require:\n{prompt_list}\n\nThe plan has been written to the plan file. Please review and approve to begin implementation."
        ))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LspTool;

#[derive(Debug, Clone, PartialEq, Eq)]
struct LspLocation {
    uri: String,
    line: usize,
    character: usize,
}

impl Tool for LspTool {
    fn name(&self) -> &'static str {
        "LSP"
    }

    fn description(&self) -> &'static str {
        "Run code intelligence requests through a local Language Server Protocol server."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let valid_ops = [
            "goToDefinition",
            "findReferences",
            "hover",
            "documentSymbol",
        ];
        let Some(operation) = input.get("operation").and_then(serde_json::Value::as_str) else {
            return ToolResult::error(format!(
                "operation must be one of: {}",
                valid_ops.join(", ")
            ));
        };
        if !valid_ops.contains(&operation) {
            return ToolResult::error(format!(
                "operation must be one of: {}",
                valid_ops.join(", ")
            ));
        }
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }
        let Some(line) = json_usize(&input, "line") else {
            return ToolResult::error("line is required");
        };
        let character = json_usize(&input, "character").unwrap_or(1);

        let abs_path = context.resolve_path(file_path);
        if !abs_path.exists() {
            return ToolResult::error(format!("File not found: {}", abs_path.display()));
        }
        let Some(server_bin) = find_lsp_server(&abs_path) else {
            return ToolResult::error(
                "No LSP server found. For Swift files, install Xcode or Swift toolchain. Ensure the language server binary is on PATH.",
            );
        };
        let workspace = find_lsp_workspace_root(&abs_path).unwrap_or_else(|| context.cwd.clone());
        let source = match fs::read_to_string(&abs_path) {
            Ok(source) => source,
            Err(error) => {
                return ToolResult::error(format!(
                    "Failed to read {}: {error}",
                    abs_path.display()
                ));
            }
        };

        match run_lsp_request(
            operation,
            &server_bin,
            &workspace,
            &abs_path,
            line,
            character,
            source,
        ) {
            Ok(content) => ToolResult::text(content),
            Err(message) => ToolResult::error(format!("LSP error: {message}")),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ClipboardTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct SpotlightSearchTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct ScreenCaptureTool;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ClipboardSnapshot {
    type_names: Vec<String>,
    text: Option<String>,
    file_paths: Vec<String>,
    has_html: bool,
    has_rtf: bool,
    image_size: Option<(u32, u32)>,
}

impl Tool for ClipboardTool {
    fn name(&self) -> &'static str {
        "Clipboard"
    }

    fn description(&self) -> &'static str {
        "Read from or write to the system clipboard."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(operation) = input.get("operation").and_then(serde_json::Value::as_str) else {
            return ToolResult::error(
                "operation must be one of: inspect, read, files, finder_selection, write",
            );
        };

        match operation {
            "inspect" => match clipboard_read_text() {
                Ok(text) => ToolResult::text(ClipboardSnapshot::from_text(text).render_inspect()),
                Err(message) => ToolResult::error(message),
            },
            "read" => match clipboard_read_text() {
                Ok(text) => ToolResult::text(ClipboardSnapshot::from_text(text).render_read()),
                Err(message) => ToolResult::error(message),
            },
            "files" => match clipboard_file_paths() {
                Ok(paths) if paths.is_empty() => {
                    ToolResult::text("[Clipboard contains no file URLs]")
                }
                Ok(paths) => ToolResult::text(paths.join("\n")),
                Err(message) => ToolResult::error(message),
            },
            "finder_selection" => ToolResult::text(finder_selection()),
            "write" => {
                let Some(content) = input.get("content").and_then(serde_json::Value::as_str) else {
                    return ToolResult::error("write operation requires content");
                };
                match clipboard_write_text(content) {
                    Ok(()) => ToolResult::text(format!(
                        "Written {} characters to clipboard.",
                        content.chars().count()
                    )),
                    Err(message) => ToolResult::error(message),
                }
            }
            _ => ToolResult::error(
                "operation must be one of: inspect, read, files, finder_selection, write",
            ),
        }
    }
}

impl ClipboardSnapshot {
    fn from_text(text: String) -> Self {
        Self {
            type_names: if text.is_empty() {
                Vec::new()
            } else {
                vec![String::from("text/plain")]
            },
            text: (!text.is_empty()).then_some(text),
            file_paths: Vec::new(),
            has_html: false,
            has_rtf: false,
            image_size: None,
        }
    }

    fn render_inspect(&self) -> String {
        let mut lines = vec![String::from("[Clipboard.inspect]")];
        lines.push(format!(
            "types: {}",
            if self.type_names.is_empty() {
                String::from("none")
            } else {
                self.type_names.join(", ")
            }
        ));
        if let Some(text) = &self.text {
            let preview = truncate_chars(&text.replace('\n', "\\n"), 240);
            lines.push(format!("text: {} chars - {preview}", text.chars().count()));
        } else {
            lines.push(String::from("text: none"));
        }
        if self.file_paths.is_empty() {
            lines.push(String::from("files: none"));
        } else {
            lines.push(format!("files: {}", self.file_paths.len()));
            lines.extend(
                self.file_paths
                    .iter()
                    .take(20)
                    .map(|path| format!("  {path}")),
            );
            if self.file_paths.len() > 20 {
                lines.push(format!("  ... {} more", self.file_paths.len() - 20));
            }
        }
        lines.push(format!("html: {}", self.has_html));
        lines.push(format!("rtf: {}", self.has_rtf));
        if let Some((width, height)) = self.image_size {
            lines.push(format!("image: {width}x{height}"));
        } else {
            lines.push(String::from("image: none"));
        }
        lines.join("\n")
    }

    fn render_read(&self) -> String {
        if let Some(text) = &self.text {
            return text.clone();
        }
        if !self.file_paths.is_empty() {
            return format!("[Clipboard file URLs]\n{}", self.file_paths.join("\n"));
        }
        if let Some((width, height)) = self.image_size {
            return format!(
                "[Clipboard image]\nsize: {width}x{height}\nUse image attachment support to provide the image to the next prompt."
            );
        }
        if self.has_html || self.has_rtf {
            return String::from(
                "[Clipboard contains rich text but no plain text]\nUse inspect to view clipboard types.",
            );
        }
        String::from("[Clipboard is empty or contains unsupported content]")
    }
}

impl Tool for SpotlightSearchTool {
    fn name(&self) -> &'static str {
        "SpotlightSearch"
    }

    fn description(&self) -> &'static str {
        "Search files using the macOS Spotlight metadata index."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(query) = input.get("query").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("query is required");
        };
        if query.trim().is_empty() {
            return ToolResult::error("query is required");
        }

        let max_results = input
            .get("max_results")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(30);
        if max_results == 0 {
            return ToolResult::error("max_results must be >= 1");
        }

        let names_only = input
            .get("names_only")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        let scope = input
            .get("scope")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(|value| context.resolve_path(value))
            .unwrap_or_else(|| normalize_path(context.cwd.clone()));

        run_spotlight_search(query.trim(), &scope, names_only, max_results.min(200))
    }
}

impl Tool for ScreenCaptureTool {
    fn name(&self) -> &'static str {
        "ScreenCapture"
    }

    fn description(&self) -> &'static str {
        "List visible apps or capture a macOS window screenshot."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(operation) = input.get("operation").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("operation must be \"list\" or \"capture\"");
        };

        match operation {
            "list" => list_screen_windows(),
            "capture" => {
                let app_name = input.get("app_name").and_then(serde_json::Value::as_str);
                let window_id = input.get("window_id").and_then(serde_json::Value::as_u64);
                if app_name
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .is_none()
                    && window_id.is_none()
                {
                    return ToolResult::error("capture requires app_name or window_id");
                }

                let max_dimension = input
                    .get("max_dimension")
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|value| u32::try_from(value).ok())
                    .unwrap_or(1600)
                    .clamp(256, 4096);
                capture_screen_window(window_id, app_name, max_dimension)
            }
            _ => ToolResult::error("operation must be \"list\" or \"capture\""),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ImagePreprocessTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct VisionTool;

#[derive(Debug, Clone, PartialEq)]
struct ImageAnalysis {
    width: u32,
    height: u32,
    mean_luma: f64,
    luma_stddev: f64,
    edge_luma: f64,
    content_box: Option<NormalizedRect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct NormalizedRect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Tool for ImagePreprocessTool {
    fn name(&self) -> &'static str {
        "ImagePreprocess"
    }

    fn description(&self) -> &'static str {
        "Inspect, crop, resize, and enhance local images before visual analysis."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }
        let Some(operation) = input.get("operation").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("operation must be one of: inspect, preprocess");
        };
        if !matches!(operation, "inspect" | "preprocess") {
            return ToolResult::error("operation must be one of: inspect, preprocess");
        }

        let path = context.resolve_path(file_path);
        if !path.exists() {
            return ToolResult::error(format!("File not found: {}", path.display()));
        }
        let image = match image::open(&path) {
            Ok(image) => image,
            Err(error) => {
                return ToolResult::error(format!(
                    "Failed to load image: {}: {error}",
                    path.display()
                ));
            }
        };

        match operation {
            "inspect" => ToolResult::text(render_image_inspect(&image, &path)),
            "preprocess" => match preprocess_image(&image, &path, &input) {
                Ok(report) => ToolResult::text(report),
                Err(error) => ToolResult::error(error),
            },
            _ => unreachable!("validated operation"),
        }
    }
}

impl Tool for VisionTool {
    fn name(&self) -> &'static str {
        "Vision"
    }

    fn description(&self) -> &'static str {
        "Analyze local images and PDFs with OCR, layout extraction, or classification."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }
        let Some(operation) = input.get("operation").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("operation must be one of: ocr, layout, classify");
        };
        if !matches!(operation, "ocr" | "layout" | "classify") {
            return ToolResult::error("operation must be one of: ocr, layout, classify");
        }

        let language_hints = match parse_vision_language_hints(&input) {
            Ok(language_hints) => language_hints,
            Err(error) => return ToolResult::error(error),
        };
        let min_confidence = match parse_vision_min_confidence(&input) {
            Ok(min_confidence) => min_confidence,
            Err(error) => return ToolResult::error(error),
        };

        let path = context.resolve_path(file_path);
        if !path.exists() {
            return ToolResult::error(format!("File not found: {}", path.display()));
        }
        if !path.is_file() {
            return ToolResult::error(format!("Path is not a file: {}", path.display()));
        }

        if is_pdf_path(&path) {
            return match operation {
                "classify" => ToolResult::text(render_pdf_classification(&path)),
                "ocr" | "layout" => match run_pdftotext_vision(&path, operation, &input) {
                    Ok(output) => ToolResult::text(output),
                    Err(error) => ToolResult::error(error),
                },
                _ => unreachable!("validated operation"),
            };
        }

        let image = match image::open(&path) {
            Ok(image) => image,
            Err(error) => {
                return ToolResult::error(format!(
                    "Failed to load image: {}: {error}",
                    path.display()
                ));
            }
        };

        match operation {
            "classify" => ToolResult::text(render_vision_classification(&image, &path)),
            "ocr" => match run_tesseract_ocr(&path, &language_hints) {
                Ok(output) => ToolResult::text(output),
                Err(error) => ToolResult::error(error),
            },
            "layout" => {
                match run_tesseract_layout(&path, &image, &language_hints, min_confidence) {
                    Ok(output) => ToolResult::text(output),
                    Err(error) => ToolResult::error(error),
                }
            }
            _ => unreachable!("validated operation"),
        }
    }
}

impl ImageAnalysis {
    fn is_likely_blank(&self) -> bool {
        self.luma_stddev < 2.0
    }

    fn content_box_description(&self) -> String {
        self.content_box.map_or_else(
            || String::from("none"),
            |rect| {
                format!(
                    "x={:.3} y={:.3} width={:.3} height={:.3}",
                    rect.x, rect.y, rect.width, rect.height
                )
            },
        )
    }
}

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

#[derive(Debug, Default, Clone, Copy)]
pub struct CronCreateTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct CronListTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct CronDeleteTool;

#[derive(Debug, Clone, PartialEq, Eq)]
struct CronJob {
    id: String,
    cron: String,
    prompt: String,
    recurring: bool,
    created_at: std::time::SystemTime,
    last_fired: Option<std::time::SystemTime>,
    fire_count: u64,
    last_status: String,
}

static CRON_JOBS: OnceLock<Mutex<BTreeMap<String, CronJob>>> = OnceLock::new();
static CRON_ID_COUNTER: OnceLock<Mutex<u64>> = OnceLock::new();
static REVIEW_ARTIFACTS: OnceLock<Mutex<BTreeMap<PathBuf, String>>> = OnceLock::new();

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
struct RemoteTriggerSettings {
    url: Option<String>,
    token: Option<String>,
    headers: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ReviewArtifactTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct SkillTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct PublishTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct RemoteTriggerTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct PushNotificationTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct NotebookEditTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct EnterWorktreeTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct ExitWorktreeTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct VerifyPlanExecutionTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct SleepTool;

#[derive(Debug, Clone, Copy)]
struct BuiltinSkill {
    name: &'static str,
    description: &'static str,
    when_to_use: Option<&'static str>,
    prompt: &'static str,
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

impl Tool for CronCreateTool {
    fn name(&self) -> &'static str {
        "CronCreate"
    }

    fn description(&self) -> &'static str {
        "Schedule a prompt using a 5-field cron expression."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(cron) = input.get("cron").and_then(serde_json::Value::as_str) else {
            return ToolResult::error(
                "cron must be a valid 5-field expression, for example `*/5 * * * *` for every 5 minutes",
            );
        };
        if let Some(error) = cron_validation_error(cron) {
            return ToolResult::error(error);
        }
        let Some(prompt) = input
            .get("prompt")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            return ToolResult::error("prompt is required");
        };

        let recurring = input
            .get("recurring")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_else(|| cron_should_default_to_recurring(cron));
        let id = next_cron_id();
        let created_at = std::time::SystemTime::now();
        let job = CronJob {
            id: id.clone(),
            cron: cron.to_owned(),
            prompt: prompt.to_owned(),
            recurring,
            created_at,
            last_fired: None,
            fire_count: 0,
            last_status: String::from("scheduled"),
        };
        cron_jobs()
            .lock()
            .expect("cron jobs lock")
            .insert(id.clone(), job);

        let schedule = cron_describe(cron);
        let task_type = if recurring { "Recurring" } else { "One-shot" };
        let next = cron_next_fire(cron, created_at)
            .map(|time| format!(" (next: {})", format_cron_datetime(time)))
            .unwrap_or_default();
        ToolResult::text(format!(
            "{task_type} task {id} scheduled: {schedule}{next}\nPermission mode switched to YOLO for unattended cron execution. Use Shift+Tab to leave YOLO when you no longer need scheduled tasks to run without prompts."
        ))
    }
}

impl Tool for CronListTool {
    fn name(&self) -> &'static str {
        "CronList"
    }

    fn description(&self) -> &'static str {
        "List all scheduled cron jobs with their IDs and schedules."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, _input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let mut jobs = cron_jobs()
            .lock()
            .expect("cron jobs lock")
            .values()
            .cloned()
            .collect::<Vec<_>>();
        jobs.sort_by(|left, right| left.id.cmp(&right.id));
        if jobs.is_empty() {
            return ToolResult::text("No scheduled cron jobs.");
        }
        let lines = jobs
            .into_iter()
            .map(|job| {
                let task_type = if job.recurring { "Recurring" } else { "One-shot" };
                let schedule = cron_describe(&job.cron);
                let fired = job
                    .last_fired
                    .map(|time| format!(" (last: {})", format_cron_time(time)))
                    .unwrap_or_default();
                let next = cron_next_fire(&job.cron, job.last_fired.unwrap_or(job.created_at))
                    .map(|time| format!(" (next: {})", format_cron_datetime(time)))
                    .unwrap_or_default();
                let prompt = truncate_chars(&job.prompt, 80);
                format!(
                    "[{}] {task_type}: {schedule}{fired}{next} (status: {}, fires: {})\n  Prompt: {prompt}",
                    job.id, job.last_status, job.fire_count
                )
            })
            .collect::<Vec<_>>();
        ToolResult::text(lines.join("\n\n"))
    }
}

impl Tool for CronDeleteTool {
    fn name(&self) -> &'static str {
        "CronDelete"
    }

    fn description(&self) -> &'static str {
        "Cancel a previously scheduled cron job by its ID."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(id) = input
            .get("id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
        else {
            return ToolResult::error("Missing job ID");
        };
        if cron_jobs()
            .lock()
            .expect("cron jobs lock")
            .remove(id)
            .is_some()
        {
            ToolResult::text(format!("Job {id} deleted."))
        } else {
            ToolResult::error(format!("Job {id} not found."))
        }
    }
}

impl Tool for ReviewArtifactTool {
    fn name(&self) -> &'static str {
        "ReviewArtifact"
    }

    fn description(&self) -> &'static str {
        "Mark a workspace file as needing human review."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        if file_path.trim().is_empty() {
            return ToolResult::error("file_path is required");
        }
        let reason = input
            .get("reason")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
            .trim();
        let path = context.resolve_path(file_path);
        review_artifacts()
            .lock()
            .expect("review artifacts lock")
            .insert(path.clone(), reason.to_owned());
        let suffix = if reason.is_empty() {
            String::new()
        } else {
            format!(" - {reason}")
        };
        ToolResult::text(format!("Marked {} for review.{suffix}", path.display()))
    }
}

impl Tool for SkillTool {
    fn name(&self) -> &'static str {
        "Skill"
    }

    fn description(&self) -> &'static str {
        "Invoke a named built-in skill by expanding its reusable prompt template."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(skill_name) = input.get("skill").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("skill name is required");
        };
        let Some(skill) = find_builtin_skill(skill_name) else {
            return ToolResult::error(format!(
                "Unknown skill: {skill_name}. Available:\n{}",
                builtin_skill_descriptions()
            ));
        };
        let args = input.get("args").and_then(serde_json::Value::as_str);
        let expanded = expand_skill_prompt(skill.prompt, args);
        ToolResult::text(format!(
            "Skill invoked: {skill_name}\n\n[SKILL PROMPT START]\n{expanded}\n[SKILL PROMPT END]\n\nFollow the instructions above. The skill specifies what to do and how to do it."
        ))
    }
}

impl Tool for PublishTool {
    fn name(&self) -> &'static str {
        "Publish"
    }

    fn description(&self) -> &'static str {
        "Prepare, inspect, or delete a static frontend publish on clide.app."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let options = match PublishOptions::from_input(&input) {
            Ok(options) => options,
            Err(message) => return ToolResult::error(message),
        };
        if options.status {
            return ToolResult::text(render_publish_status(context));
        }
        if options.delete {
            let handle = options
                .handle
                .clone()
                .or_else(|| load_publish_state(context).and_then(|state| state.handle));
            let Some(handle) = handle.filter(|value| !value.trim().is_empty()) else {
                return ToolResult::error(
                    "No handle specified and no saved `.clide/publish.json` handle found.",
                );
            };
            return match delete_publish(context, &handle) {
                Ok(result) => ToolResult::text(result),
                Err(message) => ToolResult::error(message),
            };
        }

        let publish_dir = match resolve_publish_dir(context, options.dir.as_deref()) {
            Ok(path) => path,
            Err(message) => return ToolResult::error(message),
        };
        if let Err(message) = ensure_clideignore_safety_defaults(&context.cwd) {
            return ToolResult::error(message);
        }
        let patterns = load_clideignore_patterns(&context.cwd);
        let files = match collect_publish_files(&context.cwd, &publish_dir, &patterns) {
            Ok(files) => files,
            Err(message) => return ToolResult::error(message),
        };
        if !files.iter().any(|file| file == "index.html") {
            return ToolResult::error(
                "Publish archive must include top-level `index.html` after `.clideignore` filtering.",
            );
        }
        if files.is_empty() {
            return ToolResult::error("Publish archive would contain no files.");
        }

        let state_handle = load_publish_state(context).and_then(|state| state.handle);
        let handle = if options.random {
            None
        } else {
            options.handle.clone().or(state_handle)
        };
        let plan =
            match build_publish_plan(&context.cwd, &publish_dir, &files, handle, options.random) {
                Ok(plan) => plan,
                Err(message) => return ToolResult::error(message),
            };
        if options.dry_run {
            return ToolResult::text(render_publish_dry_run(&plan));
        }

        match upload_publish(context, &publish_dir, &files, &plan) {
            Ok(result) => ToolResult::text(result),
            Err(message) => ToolResult::error(message),
        }
    }
}

impl Tool for RemoteTriggerTool {
    fn name(&self) -> &'static str {
        "RemoteTrigger"
    }

    fn description(&self) -> &'static str {
        "POST a JSON payload to a configured remote webhook endpoint."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(payload) = input.get("payload").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("payload is required");
        };
        let settings = match load_remote_trigger_settings(context) {
            Ok(Some(settings)) => settings,
            Ok(None) => {
                return ToolResult::error(
                    "Remote trigger not configured. Add settings.remote_trigger.url.",
                );
            }
            Err(error) => return ToolResult::error(error),
        };
        let Some(url) = settings
            .url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            return ToolResult::error(
                "Remote trigger not configured. Add settings.remote_trigger.url.",
            );
        };

        let body = if let Some(override_body) = input
            .get("override_body")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
        {
            override_body.to_owned()
        } else {
            match serde_json::to_string(&serde_json::json!({ "payload": payload })) {
                Ok(body) => body,
                Err(error) => {
                    return ToolResult::error(format!("Failed to encode payload: {error}"));
                }
            }
        };

        let client = match reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
        {
            Ok(client) => client,
            Err(error) => {
                return ToolResult::error(format!("Failed to create HTTP client: {error}"));
            }
        };
        let mut request = client
            .post(url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body);
        if let Some(token) = settings
            .token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            request = request.bearer_auth(token);
        }
        if let Some(headers) = settings.headers {
            for (key, value) in headers {
                if !key.trim().is_empty() {
                    request = request.header(key, value);
                }
            }
        }

        match request.send() {
            Ok(response) => {
                let status = response.status();
                let body = response
                    .text()
                    .unwrap_or_else(|error| format!("(failed to read response body: {error})"));
                let mut truncated: String = body.chars().take(2000).collect();
                if body.chars().count() > 2000 {
                    truncated.push_str("\n[response truncated]");
                }
                let content = format!("HTTP {}\n{truncated}", status.as_u16());
                if status.is_success() {
                    ToolResult::text(content)
                } else {
                    ToolResult::error(content)
                }
            }
            Err(error) => ToolResult::error(format!("Request failed: {error}")),
        }
    }
}

impl Tool for PushNotificationTool {
    fn name(&self) -> &'static str {
        "PushNotification"
    }

    fn description(&self) -> &'static str {
        "Post a native desktop notification when the user should be alerted."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(message) = input.get("message").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("message is required");
        };
        if message.is_empty() {
            return ToolResult::error("message is required");
        }
        if message.chars().count() > 500 {
            return ToolResult::error(format!(
                "message must be <= 500 chars (got {})",
                message.chars().count()
            ));
        }
        let title = input
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("deeptide");
        let subtitle = input.get("subtitle").and_then(serde_json::Value::as_str);
        let sound = input
            .get("sound")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        post_desktop_notification(title, subtitle, message, sound)
    }
}

impl Tool for NotebookEditTool {
    fn name(&self) -> &'static str {
        "NotebookEdit"
    }

    fn description(&self) -> &'static str {
        "Edit Jupyter notebook cells by id or insert new cells."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(notebook_path) = input
            .get("notebook_path")
            .and_then(serde_json::Value::as_str)
        else {
            return ToolResult::error("notebook_path must be a .ipynb file");
        };
        if !notebook_path.ends_with(".ipynb") {
            return ToolResult::error("notebook_path must be a .ipynb file");
        }
        let Some(new_source) = input.get("new_source").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("new_source is required");
        };
        let edit_mode = input
            .get("edit_mode")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("replace");
        if !matches!(edit_mode, "replace" | "insert" | "delete") {
            return ToolResult::error("edit_mode must be one of: replace, insert, delete");
        }
        let cell_type = input
            .get("cell_type")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("code");
        if !matches!(cell_type, "code" | "markdown") {
            return ToolResult::error("cell_type must be code or markdown");
        }
        let cell_id = input.get("cell_id").and_then(serde_json::Value::as_str);
        let path = context.resolve_path(notebook_path);
        if !path.exists() {
            return ToolResult::error(format!("Notebook not found: {}", path.display()));
        }

        match edit_notebook(&path, edit_mode, cell_id, new_source, cell_type) {
            Ok(message) => ToolResult::text(message),
            Err(error) => ToolResult::error(error),
        }
    }
}

impl Tool for EnterWorktreeTool {
    fn name(&self) -> &'static str {
        "EnterWorktree"
    }

    fn description(&self) -> &'static str {
        "Create an isolated git worktree for parallel work."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        if !is_git_repository(&context.cwd) {
            return ToolResult::error("Not a git repository. Worktrees require a git repo.");
        }
        let name = input
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(sanitize_worktree_name)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(default_worktree_name);
        let worktree_path = context
            .cwd
            .parent()
            .unwrap_or(context.cwd.as_path())
            .join(".deeptide-worktrees")
            .join(&name);
        if let Some(parent) = worktree_path.parent()
            && let Err(error) = fs::create_dir_all(parent)
        {
            return ToolResult::error(format!("Failed to create worktree parent: {error}"));
        }

        let output = run_git(
            &[
                "worktree",
                "add",
                worktree_path.to_string_lossy().as_ref(),
                "-b",
                &name,
            ],
            &context.cwd,
        );
        match output {
            Ok(output) if output.status.success() => ToolResult::text(format!(
                "Worktree created: {}\nBranch: {name}\n\nThe worktree is an isolated copy of the repository. Changes made here will not affect your main working directory. Use ExitWorktree to clean up.",
                worktree_path.display()
            )),
            Ok(output) => ToolResult::error(format!(
                "Failed to create worktree: {}",
                String::from_utf8_lossy(&output.stderr)
                    .trim()
                    .if_empty("unknown error")
            )),
            Err(error) => ToolResult::error(format!("Failed to run git worktree add: {error}")),
        }
    }
}

impl Tool for ExitWorktreeTool {
    fn name(&self) -> &'static str {
        "ExitWorktree"
    }

    fn description(&self) -> &'static str {
        "Keep or remove a git worktree and its branch."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let action = input
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("keep");
        if !matches!(action, "keep" | "remove") {
            return ToolResult::error("action must be keep or remove");
        }
        let worktree_path = input
            .get("path")
            .and_then(serde_json::Value::as_str)
            .map(|path| context.resolve_path(path))
            .unwrap_or_else(|| context.cwd.clone());

        if action == "keep" {
            return ToolResult::text(format!("Worktree kept at: {}", worktree_path.display()));
        }

        let branch = worktree_branch_for_path(&context.cwd, &worktree_path);
        let path_string = worktree_path.to_string_lossy();
        let remove = run_git(
            &["worktree", "remove", path_string.as_ref(), "--force"],
            &context.cwd,
        );
        match remove {
            Ok(output) if output.status.success() => {
                if let Some(branch) = branch.as_deref() {
                    let _ = run_git(&["branch", "-D", branch], &context.cwd);
                }
                ToolResult::text(format!("Worktree removed: {}", worktree_path.display()))
            }
            Ok(output) => ToolResult::error(format!(
                "Failed to remove worktree: {}",
                String::from_utf8_lossy(&output.stderr)
                    .trim()
                    .if_empty("unknown error")
            )),
            Err(error) => ToolResult::error(format!("Failed to run git worktree remove: {error}")),
        }
    }
}

impl Tool for VerifyPlanExecutionTool {
    fn name(&self) -> &'static str {
        "VerifyPlanExecution"
    }

    fn description(&self) -> &'static str {
        "Verify that planned file changes appear in git status."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        match render_plan_verification(&input, context) {
            Ok(report) => ToolResult::text(report),
            Err(error) => ToolResult::error(error),
        }
    }
}

impl Tool for SleepTool {
    fn name(&self) -> &'static str {
        "Sleep"
    }

    fn description(&self) -> &'static str {
        "Wait for a bounded duration without running a shell command."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(duration_ms) = input.get("duration_ms").and_then(serde_json::Value::as_f64) else {
            return ToolResult::error("duration_ms is required");
        };
        let clamped = duration_ms.clamp(0.0, 300_000.0);
        thread::sleep(Duration::from_millis(clamped as u64));
        ToolResult::text(format!("Slept {} ms", clamped as u64))
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

fn parse_user_questions(input: &serde_json::Value) -> Result<Vec<UserQuestion>, String> {
    let Some(items) = input.get("questions").and_then(serde_json::Value::as_array) else {
        return Err(String::from("At least one question is required"));
    };
    if items.is_empty() {
        return Err(String::from("At least one question is required"));
    }
    if items.len() > 4 {
        return Err(String::from("questions must contain at most 4 items"));
    }

    items
        .iter()
        .enumerate()
        .map(|(index, item)| parse_user_question(index, item))
        .collect()
}

fn parse_user_question(index: usize, item: &serde_json::Value) -> Result<UserQuestion, String> {
    let Some(object) = item.as_object() else {
        return Err(format!("questions[{index}] must be an object"));
    };
    let question = required_trimmed_string(object.get("question"))
        .ok_or_else(|| format!("questions[{index}].question is required"))?;
    let header = required_trimmed_string(object.get("header"))
        .ok_or_else(|| format!("questions[{index}].header is required"))?;
    if header.chars().count() > 12 {
        return Err(format!(
            "questions[{index}].header must be 12 characters or fewer"
        ));
    }
    let Some(option_items) = object.get("options").and_then(serde_json::Value::as_array) else {
        return Err(format!("questions[{index}].options is required"));
    };
    if !(2..=4).contains(&option_items.len()) {
        return Err(format!(
            "questions[{index}].options must contain 2 to 4 options"
        ));
    }

    let options = option_items
        .iter()
        .enumerate()
        .map(|(option_index, option)| parse_user_question_option(index, option_index, option))
        .collect::<Result<Vec<_>, _>>()?;
    let multi_select = object
        .get("multiSelect")
        .or_else(|| object.get("multi_select"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    Ok(UserQuestion {
        question,
        header,
        options,
        multi_select,
    })
}

fn parse_user_question_option(
    question_index: usize,
    option_index: usize,
    item: &serde_json::Value,
) -> Result<UserQuestionOption, String> {
    let Some(object) = item.as_object() else {
        return Err(format!(
            "questions[{question_index}].options[{option_index}] must be an object"
        ));
    };
    let label = required_trimmed_string(object.get("label")).ok_or_else(|| {
        format!("questions[{question_index}].options[{option_index}].label is required")
    })?;
    let description = required_trimmed_string(object.get("description")).ok_or_else(|| {
        format!("questions[{question_index}].options[{option_index}].description is required")
    })?;
    Ok(UserQuestionOption { label, description })
}

fn required_trimmed_string(value: Option<&serde_json::Value>) -> Option<String> {
    value
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn search_memory(query: &str, scope: &str, max_results: usize, cwd: &Path) -> ToolResult {
    let dirs = memory_search_dirs(scope, cwd)
        .into_iter()
        .filter(|(dir_scope, dir)| {
            let _ = dir_scope;
            dir.exists()
        })
        .collect::<Vec<_>>();
    if dirs.is_empty() {
        return ToolResult::text(format!(
            "No Deeptide memory directories found for scope: {scope}"
        ));
    }

    let needle = query.to_ascii_lowercase();
    let mut hits = Vec::new();
    for (scope_label, dir) in dirs {
        collect_memory_hits(scope_label, &dir, &needle, &mut hits);
    }
    hits.sort_by(|left, right| {
        left.scope
            .cmp(right.scope)
            .then_with(|| left.path.cmp(&right.path))
    });
    hits.dedup_by(|left, right| left.path == right.path);

    if hits.is_empty() {
        return ToolResult::text("No matching Deeptide memory files found.");
    }

    ToolResult::text(
        hits.into_iter()
            .take(max_results)
            .map(render_memory_hit)
            .collect::<Vec<_>>()
            .join("\n\n"),
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MemorySearchHit {
    scope: &'static str,
    path: PathBuf,
    title: String,
    description: Option<String>,
    matching_line: Option<String>,
}

fn memory_search_dirs(scope: &str, cwd: &Path) -> Vec<(&'static str, PathBuf)> {
    match scope {
        "project" => vec![
            ("project", MemorySystem::project_memory_dir(cwd)),
            ("project", MemorySystem::legacy_project_memory_dir(cwd)),
        ],
        "global" => vec![
            ("global", MemorySystem::global_memory_dir()),
            ("global", MemorySystem::legacy_global_memory_dir()),
        ],
        _ => vec![
            ("project", MemorySystem::project_memory_dir(cwd)),
            ("project", MemorySystem::legacy_project_memory_dir(cwd)),
            ("global", MemorySystem::global_memory_dir()),
            ("global", MemorySystem::legacy_global_memory_dir()),
        ],
    }
}

fn collect_memory_hits(
    scope: &'static str,
    dir: &Path,
    needle: &str,
    hits: &mut Vec<MemorySearchHit>,
) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().and_then(|name| name.to_str()) == Some("MEMORY.md") {
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("md") {
            continue;
        }
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        if !content.to_ascii_lowercase().contains(needle) {
            continue;
        }
        let frontmatter = extract_frontmatter(&path);
        let title = frontmatter
            .iter()
            .find(|(key, _)| key == "name")
            .map(|(_, value)| value.to_owned())
            .or_else(|| first_markdown_heading(&content))
            .or_else(|| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .map(ToOwned::to_owned)
            })
            .unwrap_or_default();
        let description = frontmatter
            .iter()
            .find(|(key, _)| key == "description")
            .map(|(_, value)| value.to_owned());
        let matching_line = first_matching_memory_line(needle, &content);
        hits.push(MemorySearchHit {
            scope,
            path,
            title,
            description,
            matching_line,
        });
    }
}

fn render_memory_hit(hit: MemorySearchHit) -> String {
    let mut lines = vec![
        format!("- {}", hit.path.display()),
        format!("  scope: {}", hit.scope),
        format!("  title: {}", hit.title),
    ];
    if let Some(description) = hit.description {
        lines.push(format!("  description: {description}"));
    }
    if let Some(matching_line) = hit.matching_line {
        lines.push(format!("  match: {}", truncate_chars(&matching_line, 180)));
    }
    lines.join("\n")
}

fn first_markdown_heading(content: &str) -> Option<String> {
    strip_frontmatter(content)
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn first_matching_memory_line(needle: &str, content: &str) -> Option<String> {
    strip_frontmatter(content).lines().find_map(|line| {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "---" {
            return None;
        }
        trimmed
            .to_ascii_lowercase()
            .contains(needle)
            .then(|| trimmed.to_owned())
    })
}

fn parse_memory_type(raw: &str) -> Option<MemoryType> {
    match raw {
        "user" => Some(MemoryType::User),
        "feedback" => Some(MemoryType::Feedback),
        "project" => Some(MemoryType::Project),
        "reference" => Some(MemoryType::Reference),
        _ => None,
    }
}

fn unique_memory_file_name(title: &str, cwd: &Path, scope: MemoryScope) -> String {
    let dir = match scope {
        MemoryScope::Project => MemorySystem::project_memory_dir(cwd),
        MemoryScope::Global => MemorySystem::global_memory_dir(),
    };
    let base = memory_slug(title);
    let mut candidate = format!("{base}.md");
    let mut suffix = 2usize;
    while dir.join(&candidate).exists() {
        candidate = format!("{base}-{suffix}.md");
        suffix += 1;
    }
    candidate
}

fn memory_slug(title: &str) -> String {
    let words = title
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .take(8)
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if words.is_empty() {
        String::from("memory")
    } else {
        words.join("-")
    }
}

fn safe_inline(value: &str) -> String {
    value
        .replace(['\n', '\r'], " ")
        .replace('[', "(")
        .replace(']', ")")
        .replace(':', " -")
        .trim()
        .to_owned()
}

fn model_context_window(model: &str) -> u64 {
    let lower = model.to_ascii_lowercase();
    if lower.contains("deepseek-v4-pro") || lower.contains("deepseek-v4-flash-q4") {
        1_000_000
    } else if lower.contains("deepseek-v4-flash") {
        512_000
    } else if lower.contains("qwen3-coder") || lower.contains("qwen3.6") || lower.contains("claude")
    {
        262_144
    } else {
        200_000
    }
}

fn format_compact_number(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn extract_allowed_prompts(input: &serde_json::Value) -> Vec<(String, String)> {
    input
        .get("allowedPrompts")
        .or_else(|| input.get("allowed_prompts"))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let tool = item.get("tool")?.as_str()?.trim();
                    let prompt = item.get("prompt")?.as_str()?.trim();
                    if tool.is_empty() || prompt.is_empty() {
                        None
                    } else {
                        Some((tool.to_owned(), prompt.to_owned()))
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

fn json_usize(input: &serde_json::Value, key: &str) -> Option<usize> {
    input
        .get(key)?
        .as_u64()
        .and_then(|value| value.try_into().ok())
}

fn run_lsp_request(
    operation: &str,
    server_bin: &Path,
    workspace: &Path,
    file_path: &Path,
    line: usize,
    character: usize,
    source: String,
) -> Result<String, String> {
    let mut session = LspSession::new(server_bin, workspace)?;
    let file_uri = lsp_file_uri(file_path);
    session.initialize(workspace)?;
    session.did_open(&file_uri, language_id_for_path(file_path), source)?;
    thread::sleep(Duration::from_millis(300));

    let lsp_line = line.saturating_sub(1);
    let lsp_character = character.saturating_sub(1);
    let result = match operation {
        "goToDefinition" => {
            let value = session.request(
                "textDocument/definition",
                serde_json::json!({
                    "textDocument": {"uri": file_uri},
                    "position": {"line": lsp_line, "character": lsp_character}
                }),
                Duration::from_secs(8),
            )?;
            format_lsp_locations(parse_lsp_locations(value.as_ref()), "Definition", workspace)
        }
        "findReferences" => {
            let value = session.request(
                "textDocument/references",
                serde_json::json!({
                    "textDocument": {"uri": file_uri},
                    "position": {"line": lsp_line, "character": lsp_character},
                    "context": {"includeDeclaration": true}
                }),
                Duration::from_secs(10),
            )?;
            format_lsp_locations(parse_lsp_locations(value.as_ref()), "References", workspace)
        }
        "hover" => {
            let value = session.request(
                "textDocument/hover",
                serde_json::json!({
                    "textDocument": {"uri": file_uri},
                    "position": {"line": lsp_line, "character": lsp_character}
                }),
                Duration::from_secs(8),
            )?;
            format_lsp_hover(value.as_ref()).unwrap_or_else(|| {
                format!(
                    "No hover information available at {}:{line}:{character}",
                    file_path.display()
                )
            })
        }
        "documentSymbol" => {
            let value = session.request(
                "textDocument/documentSymbol",
                serde_json::json!({"textDocument": {"uri": file_uri}}),
                Duration::from_secs(8),
            )?;
            let output = format_lsp_document_symbols(value.as_ref());
            if output.is_empty() {
                format!(
                    "No symbols found in {}",
                    file_path
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("file")
                )
            } else {
                output
            }
        }
        _ => return Err(format!("Unknown operation: {operation}")),
    };
    session.shutdown();
    Ok(result)
}

struct LspSession {
    child: Child,
    stdin: ChildStdin,
    responses: mpsc::Receiver<Result<serde_json::Value, String>>,
    next_id: u64,
}

impl LspSession {
    fn new(server_bin: &Path, workspace: &Path) -> Result<Self, String> {
        let mut child = Command::new(server_bin)
            .current_dir(workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("Failed to start {}: {error}", server_bin.display()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| String::from("LSP server stdin was unavailable"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| String::from("LSP server stdout was unavailable"))?;
        let (sender, responses) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            loop {
                match read_lsp_message(&mut reader) {
                    Ok(Some(value)) => {
                        if sender.send(Ok(value)).is_err() {
                            break;
                        }
                    }
                    Ok(None) => break,
                    Err(error) => {
                        let _ = sender.send(Err(error));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            stdin,
            responses,
            next_id: 1,
        })
    }

    fn initialize(&mut self, workspace: &Path) -> Result<(), String> {
        self.request(
            "initialize",
            serde_json::json!({
                "processId": std::process::id(),
                "rootUri": lsp_file_uri(workspace),
                "capabilities": {
                    "textDocument": {
                        "definition": {"dynamicRegistration": false},
                        "references": {"dynamicRegistration": false},
                        "hover": {"dynamicRegistration": false},
                        "documentSymbol": {
                            "dynamicRegistration": false,
                            "hierarchicalDocumentSymbolSupport": false
                        }
                    }
                },
                "trace": "off"
            }),
            Duration::from_secs(10),
        )?;
        self.notification("initialized", serde_json::json!({}))
    }

    fn did_open(&mut self, uri: &str, language_id: &str, text: String) -> Result<(), String> {
        self.notification(
            "textDocument/didOpen",
            serde_json::json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": language_id,
                    "version": 1,
                    "text": text
                }
            }),
        )
    }

    fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> Result<Option<serde_json::Value>, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))?;

        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(String::from("LSP server timed out"));
            }
            let message = self
                .responses
                .recv_timeout(remaining)
                .map_err(|_| String::from("LSP server timed out"))??;
            if message.get("id").and_then(serde_json::Value::as_u64) != Some(id) {
                continue;
            }
            if let Some(error) = message.get("error") {
                return Err(format!("LSP server error: {error}"));
            }
            return Ok(message.get("result").cloned());
        }
    }

    fn notification(&mut self, method: &str, params: serde_json::Value) -> Result<(), String> {
        self.send(serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
    }

    fn send(&mut self, value: serde_json::Value) -> Result<(), String> {
        let body = serde_json::to_vec(&value)
            .map_err(|error| format!("Failed to encode LSP message: {error}"))?;
        write!(self.stdin, "Content-Length: {}\r\n\r\n", body.len())
            .map_err(|error| format!("Failed to write LSP header: {error}"))?;
        self.stdin
            .write_all(&body)
            .map_err(|error| format!("Failed to write LSP body: {error}"))?;
        self.stdin
            .flush()
            .map_err(|error| format!("Failed to flush LSP body: {error}"))
    }

    fn shutdown(&mut self) {
        let _ = self.request("shutdown", serde_json::json!(null), Duration::from_secs(2));
        let _ = self.notification("exit", serde_json::json!({}));
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for LspSession {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn read_lsp_message(
    reader: &mut BufReader<impl Read>,
) -> Result<Option<serde_json::Value>, String> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("Failed to read LSP header: {error}"))?;
        if bytes == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed
            .strip_prefix("Content-Length:")
            .or_else(|| trimmed.strip_prefix("content-length:"))
        {
            content_length = Some(
                value
                    .trim()
                    .parse::<usize>()
                    .map_err(|error| format!("Invalid LSP Content-Length: {error}"))?,
            );
        }
    }
    let Some(length) = content_length else {
        return Err(String::from("LSP message did not include Content-Length"));
    };
    let mut body = vec![0u8; length];
    reader
        .read_exact(&mut body)
        .map_err(|error| format!("Failed to read LSP body: {error}"))?;
    serde_json::from_slice(&body).map(Some).map_err(|error| {
        format!(
            "Failed to parse LSP JSON: {error}: {}",
            String::from_utf8_lossy(&body)
        )
    })
}

fn find_lsp_server(file_path: &Path) -> Option<PathBuf> {
    let ext = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let candidates: &[&str] = match ext.as_str() {
        "swift" | "m" | "mm" | "h" | "c" | "cpp" | "cc" | "cxx" => &[
            "/Applications/Xcode.app/Contents/Developer/Toolchains/XcodeDefault.xctoolchain/usr/bin/sourcekit-lsp",
            "/usr/bin/sourcekit-lsp",
            "sourcekit-lsp",
        ],
        "py" => &["pyright-langserver", "pylsp", "python-lsp-server"],
        "js" | "ts" | "tsx" | "jsx" => &["typescript-language-server", "tsserver"],
        "rs" => &["rust-analyzer"],
        "go" => &["gopls"],
        _ => &[],
    };

    candidates.iter().find_map(|candidate| {
        let path = PathBuf::from(candidate);
        if path.is_absolute() {
            path.is_file().then_some(path)
        } else {
            which_binary(candidate)
        }
    })
}

fn which_binary(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    #[cfg(windows)]
    let extensions = ["", ".exe", ".cmd", ".bat"];
    #[cfg(not(windows))]
    let extensions = [""];
    for dir in std::env::split_paths(&paths) {
        for extension in extensions {
            let candidate = dir.join(format!("{name}{extension}"));
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

fn find_lsp_workspace_root(file_path: &Path) -> Option<PathBuf> {
    let mut dir = file_path.parent()?;
    for _ in 0..12 {
        for marker in [
            "Package.swift",
            ".xcodeproj",
            ".xcworkspace",
            ".git",
            "Cargo.toml",
            "pyproject.toml",
            "package.json",
            "go.mod",
        ] {
            if dir.join(marker).exists() {
                return Some(dir.to_path_buf());
            }
        }
        dir = dir.parent()?;
    }
    None
}

fn lsp_file_uri(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    #[cfg(windows)]
    let raw = if raw.starts_with('/') {
        raw
    } else {
        format!("/{raw}")
    };
    format!("file://{}", percent_encode_path(&raw))
}

fn path_from_lsp_uri(uri: &str) -> String {
    percent_decode_path(uri.strip_prefix("file://").unwrap_or(uri))
}

fn percent_encode_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' | b':' => {
                encoded.push(char::from(*byte))
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn percent_decode_path(path: &str) -> String {
    let bytes = path.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) =
                (hex_value(bytes[index + 1]), hex_value(bytes[index + 2]))
        {
            decoded.push((high << 4) | low);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8_lossy(&decoded).into_owned()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn language_id_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "swift" => "swift",
        "m" | "mm" => "objective-c",
        "h" | "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "py" => "python",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" => "javascript",
        "rs" => "rust",
        "go" => "go",
        _ => "plaintext",
    }
}

fn parse_lsp_locations(value: Option<&serde_json::Value>) -> Vec<LspLocation> {
    fn parse_one(value: &serde_json::Value) -> Option<LspLocation> {
        let (uri, range) = if let Some(target_uri) = value.get("targetUri") {
            (target_uri.as_str()?, value.get("targetSelectionRange")?)
        } else {
            (value.get("uri")?.as_str()?, value.get("range")?)
        };
        let start = range.get("start")?;
        Some(LspLocation {
            uri: uri.to_owned(),
            line: start.get("line")?.as_u64()?.try_into().ok()?,
            character: start.get("character")?.as_u64()?.try_into().ok()?,
        })
    }

    match value {
        Some(serde_json::Value::Array(items)) => items.iter().filter_map(parse_one).collect(),
        Some(serde_json::Value::Object(_)) => value.and_then(parse_one).into_iter().collect(),
        _ => Vec::new(),
    }
}

fn format_lsp_locations(locations: Vec<LspLocation>, label: &str, workspace: &Path) -> String {
    if locations.is_empty() {
        return format!("No {} found.", label.to_ascii_lowercase());
    }
    let mut lines = vec![format!("{label}s ({}):", locations.len())];
    for location in locations {
        let path = PathBuf::from(path_from_lsp_uri(&location.uri));
        let display_path = path
            .strip_prefix(workspace)
            .unwrap_or(&path)
            .display()
            .to_string();
        lines.push(format!(
            "  {}:{}:{}",
            display_path,
            location.line + 1,
            location.character + 1
        ));
    }
    lines.join("\n")
}

fn format_lsp_hover(value: Option<&serde_json::Value>) -> Option<String> {
    let contents = value?.get("contents")?;
    if let Some(text) = contents.as_str() {
        return Some(text.to_owned());
    }
    if let Some(value) = contents.get("value").and_then(serde_json::Value::as_str) {
        return Some(value.to_owned());
    }
    if let Some(items) = contents.as_array() {
        let output = items
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .or_else(|| item.get("value").and_then(serde_json::Value::as_str))
            })
            .collect::<Vec<_>>()
            .join("\n");
        return (!output.is_empty()).then_some(output);
    }
    None
}

fn format_lsp_document_symbols(value: Option<&serde_json::Value>) -> String {
    let Some(serde_json::Value::Array(symbols)) = value else {
        return String::new();
    };
    symbols
        .iter()
        .filter_map(|symbol| {
            let name = symbol.get("name").and_then(serde_json::Value::as_str)?;
            let kind = symbol_kind_name(symbol.get("kind").and_then(serde_json::Value::as_u64));
            if let Some(line) = symbol
                .pointer("/location/range/start/line")
                .or_else(|| symbol.pointer("/range/start/line"))
                .and_then(serde_json::Value::as_u64)
            {
                Some(format!("{kind} {name} (line {})", line + 1))
            } else {
                Some(format!("{kind} {name}"))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn symbol_kind_name(kind: Option<u64>) -> &'static str {
    match kind {
        Some(1) => "File",
        Some(2) => "Module",
        Some(3) => "Namespace",
        Some(4) => "Package",
        Some(5) => "Class",
        Some(6) => "Method",
        Some(7) => "Property",
        Some(8) => "Field",
        Some(9) => "Constructor",
        Some(10) => "Enum",
        Some(11) => "Interface",
        Some(12) => "Function",
        Some(13) => "Variable",
        Some(14) => "Constant",
        Some(15) => "String",
        Some(16) => "Number",
        Some(17) => "Boolean",
        Some(18) => "Array",
        Some(23) => "Struct",
        Some(25) => "EnumMember",
        _ => "Symbol",
    }
}

fn render_image_inspect(image: &DynamicImage, path: &Path) -> String {
    let analysis = analyse_image(image, 192);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    format!(
        "[ImagePreprocess.inspect] {name}\nsize: {}x{}\nmean_luma: {:.1}\nluma_stddev: {:.1}\nedge_luma: {:.1}\nlikely_blank: {}\ncontent_box: {}",
        analysis.width,
        analysis.height,
        analysis.mean_luma,
        analysis.luma_stddev,
        analysis.edge_luma,
        analysis.is_likely_blank(),
        analysis.content_box_description()
    )
}

fn preprocess_image(
    image: &DynamicImage,
    path: &Path,
    input: &serde_json::Value,
) -> Result<String, String> {
    let original_analysis = analyse_image(image, 192);
    let mut processed = image.to_rgba8();
    let mut steps = Vec::new();

    if input
        .get("auto_trim")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        && let Some(rect) = original_analysis.content_box
    {
        processed = crop_normalized(&processed, rect);
        steps.push(String::from("auto_trim"));
    }

    if let Some(rect) = parse_normalized_crop(input.get("crop")) {
        processed = crop_normalized(&processed, rect);
        steps.push(String::from("crop"));
    }

    if input
        .get("enhance_text")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        processed = enhance_text_image(&processed);
        steps.push(String::from("enhance_text"));
    }

    let max_dimension = input
        .get("max_dimension")
        .and_then(serde_json::Value::as_u64)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(1600)
        .clamp(256, 4096);
    let longest_side = processed.width().max(processed.height());
    if longest_side > max_dimension {
        let scale = f64::from(max_dimension) / f64::from(longest_side);
        let width = (f64::from(processed.width()) * scale).round().max(1.0) as u32;
        let height = (f64::from(processed.height()) * scale).round().max(1.0) as u32;
        processed = imageops::resize(&processed, width, height, FilterType::Lanczos3);
        steps.push(format!("resize_{max_dimension}"));
    }

    let format = input
        .get("format")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("jpeg")
        .to_ascii_lowercase();
    let format = if format == "png" { "png" } else { "jpeg" };
    let encoded = encode_processed_image(&processed, format)?;
    let media_type = if format == "png" {
        "image/png"
    } else {
        "image/jpeg"
    };
    let final_analysis = analyse_rgba_image(&processed, 192);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    let encoded_base64 = base64_encode(&encoded);

    Ok(format!(
        "[ImagePreprocess.preprocess] {name}\nsteps: {}\ninput_size: {}x{}\noutput_size: {}x{}\nlikely_blank: {}\ncontent_box: {}\nformat: {media_type}\nimage_base64: {encoded_base64}",
        if steps.is_empty() {
            String::from("none")
        } else {
            steps.join(", ")
        },
        original_analysis.width,
        original_analysis.height,
        final_analysis.width,
        final_analysis.height,
        final_analysis.is_likely_blank(),
        final_analysis.content_box_description()
    ))
}

fn parse_vision_language_hints(input: &serde_json::Value) -> Result<Vec<String>, String> {
    let Some(value) = input.get("language_hints") else {
        return Ok(Vec::new());
    };
    let Some(values) = value.as_array() else {
        return Err(String::from("language_hints must be an array of strings"));
    };
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
                .ok_or_else(|| String::from("language_hints must be an array of strings"))
        })
        .collect()
}

fn parse_vision_min_confidence(input: &serde_json::Value) -> Result<f64, String> {
    let Some(value) = input.get("min_confidence") else {
        return Ok(0.5);
    };
    let Some(confidence) = value.as_f64() else {
        return Err(String::from(
            "min_confidence must be a number from 0.0 to 1.0",
        ));
    };
    if !(0.0..=1.0).contains(&confidence) {
        return Err(String::from(
            "min_confidence must be a number from 0.0 to 1.0",
        ));
    }
    Ok(confidence)
}

fn is_pdf_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pdf"))
}

fn render_vision_classification(image: &DynamicImage, path: &Path) -> String {
    let analysis = analyse_image(image, 192);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    let aspect_ratio = f64::from(analysis.width) / f64::from(analysis.height.max(1));
    let mut labels = vec![String::from("image")];
    if analysis.is_likely_blank() {
        labels.push(String::from("blank"));
    } else if analysis.luma_stddev > 40.0 && analysis.edge_luma > 35.0 {
        labels.push(String::from("document-like"));
    } else if analysis.edge_luma > 25.0 {
        labels.push(String::from("screenshot-like"));
    } else {
        labels.push(String::from("photo-like"));
    }
    if aspect_ratio > 1.25 {
        labels.push(String::from("landscape"));
    } else if aspect_ratio < 0.8 {
        labels.push(String::from("portrait"));
    }

    format!(
        "[Vision.classify] {name}\nsize: {}x{}\naspect_ratio: {:.2}\nlabels: {}\nmean_luma: {:.1}\nluma_stddev: {:.1}\nedge_luma: {:.1}\nlikely_blank: {}\ncontent_box: {}",
        analysis.width,
        analysis.height,
        aspect_ratio,
        labels.join(", "),
        analysis.mean_luma,
        analysis.luma_stddev,
        analysis.edge_luma,
        analysis.is_likely_blank(),
        analysis.content_box_description()
    )
}

fn render_pdf_classification(path: &Path) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.pdf");
    let size = fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    format!("[Vision.classify] {name}\nlabels: pdf, document\nsize_bytes: {size}")
}

fn run_tesseract_ocr(path: &Path, language_hints: &[String]) -> Result<String, String> {
    let output = run_tesseract(path, language_hints, None)?;
    let text = output.trim();
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    Ok(format!(
        "[Vision.ocr] {name}\n{}",
        if text.is_empty() {
            "[No text detected]"
        } else {
            text
        }
    ))
}

fn run_tesseract_layout(
    path: &Path,
    image: &DynamicImage,
    language_hints: &[String],
    min_confidence: f64,
) -> Result<String, String> {
    let output = run_tesseract(path, language_hints, Some("tsv"))?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("image");
    let (width, height) = (
        f64::from(image.width().max(1)),
        f64::from(image.height().max(1)),
    );
    let min_confidence = min_confidence * 100.0;
    let mut lines = vec![format!("[Vision.layout] {name}")];
    for line in output.lines().skip(1) {
        let columns: Vec<&str> = line.split('\t').collect();
        if columns.len() < 12 {
            continue;
        }
        let confidence = columns[10].parse::<f64>().unwrap_or(-1.0);
        let text = columns[11..].join("\t").trim().to_owned();
        if text.is_empty() || confidence < min_confidence {
            continue;
        }
        let left = columns[6].parse::<f64>().unwrap_or(0.0) / width;
        let top = columns[7].parse::<f64>().unwrap_or(0.0) / height;
        let word_width = columns[8].parse::<f64>().unwrap_or(0.0) / width;
        let word_height = columns[9].parse::<f64>().unwrap_or(0.0) / height;
        lines.push(format!(
            "conf={:.0} x={left:.3} y={top:.3} width={word_width:.3} height={word_height:.3} text={text}",
            confidence
        ));
    }
    if lines.len() == 1 {
        lines.push(String::from("[No text regions detected]"));
    }
    Ok(lines.join("\n"))
}

fn run_tesseract(
    path: &Path,
    language_hints: &[String],
    output_format: Option<&str>,
) -> Result<String, String> {
    let mut command = Command::new("tesseract");
    command.arg(path).arg("stdout");
    if !language_hints.is_empty() {
        command.arg("-l").arg(language_hints.join("+"));
    }
    if let Some(output_format) = output_format {
        command.arg(output_format);
    }
    let output = command.output().map_err(|error| {
        format!(
            "Vision OCR requires tesseract on this platform. Install tesseract or use ImagePreprocess/classify only: {error}"
        )
    })?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Vision OCR failed: {}",
            stderr.trim().if_empty("unknown error")
        ))
    }
}

fn run_pdftotext_vision(
    path: &Path,
    operation: &str,
    input: &serde_json::Value,
) -> Result<String, String> {
    let (first_page, last_page) = parse_vision_pages(input.get("pages"))?;
    let mut command = Command::new("pdftotext");
    if operation == "layout" {
        command.arg("-layout");
    }
    command
        .arg("-f")
        .arg(first_page.to_string())
        .arg("-l")
        .arg(last_page.to_string())
        .arg(path)
        .arg("-");
    let output = command.output().map_err(|error| {
        format!(
            "Vision PDF OCR requires pdftotext (poppler). Install poppler or convert pages to images: {error}"
        )
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "Vision PDF OCR failed: {}",
            stderr.trim().if_empty("unknown error")
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("document.pdf");
    Ok(format!(
        "[Vision.{operation}] {name}\npages: {first_page}-{last_page}\n{}",
        if text.trim().is_empty() {
            "[No text detected]"
        } else {
            text.trim()
        }
    ))
}

fn parse_vision_pages(value: Option<&serde_json::Value>) -> Result<(usize, usize), String> {
    let Some(value) = value else {
        return Ok((1, 1));
    };
    if let Some(page) = value.as_u64() {
        let page = usize::try_from(page).map_err(|_| String::from("pages is too large"))?;
        if page == 0 {
            return Err(String::from("pages must start at 1"));
        }
        return Ok((page, page));
    }
    let Some(pages) = value
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(String::from(
            "pages must be a page number or range like 1-3",
        ));
    };
    if let Some((start, end)) = pages.split_once('-') {
        let start = start
            .trim()
            .parse::<usize>()
            .map_err(|_| String::from("pages must be a page number or range like 1-3"))?;
        let end = end
            .trim()
            .parse::<usize>()
            .map_err(|_| String::from("pages must be a page number or range like 1-3"))?;
        if start == 0 || end < start {
            return Err(String::from(
                "pages must be a page number or range like 1-3",
            ));
        }
        return Ok((start, end.min(start + 4)));
    }
    let page = pages
        .parse::<usize>()
        .map_err(|_| String::from("pages must be a page number or range like 1-3"))?;
    if page == 0 {
        return Err(String::from("pages must start at 1"));
    }
    Ok((page, page))
}

fn analyse_image(image: &DynamicImage, sample_limit: u32) -> ImageAnalysis {
    analyse_rgba_image(&image.to_rgba8(), sample_limit)
}

fn analyse_rgba_image(image: &RgbaImage, sample_limit: u32) -> ImageAnalysis {
    let (width, height) = image.dimensions();
    let longest_side = width.max(height).max(1);
    let scale = (f64::from(sample_limit) / f64::from(longest_side)).min(1.0);
    let sample_width = (f64::from(width) * scale).round().max(1.0) as u32;
    let sample_height = (f64::from(height) * scale).round().max(1.0) as u32;
    let sample = if sample_width == width && sample_height == height {
        image.clone()
    } else {
        imageops::resize(image, sample_width, sample_height, FilterType::Triangle)
    };
    let mut lumas = Vec::with_capacity((sample_width * sample_height) as usize);
    for pixel in sample.pixels() {
        let [r, g, b, _] = pixel.0;
        lumas.push(0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b));
    }
    let count = lumas.len().max(1) as f64;
    let mean_luma = lumas.iter().sum::<f64>() / count;
    let variance = lumas
        .iter()
        .map(|luma| (luma - mean_luma).powi(2))
        .sum::<f64>()
        / count;
    let edge_luma = estimate_edge_luma(&lumas, sample_width as usize, sample_height as usize);
    let content_box = estimate_content_box(
        &lumas,
        sample_width as usize,
        sample_height as usize,
        edge_luma,
    );

    ImageAnalysis {
        width,
        height,
        mean_luma,
        luma_stddev: variance.sqrt(),
        edge_luma,
        content_box,
    }
}

fn estimate_edge_luma(lumas: &[f64], width: usize, height: usize) -> f64 {
    if width == 0 || height == 0 {
        return 0.0;
    }
    let mut samples = Vec::with_capacity(width * 2 + height * 2);
    for x in 0..width {
        samples.push(lumas[x]);
        samples.push(lumas[(height - 1) * width + x]);
    }
    for y in 0..height {
        samples.push(lumas[y * width]);
        samples.push(lumas[y * width + width - 1]);
    }
    samples.iter().sum::<f64>() / samples.len().max(1) as f64
}

fn estimate_content_box(
    lumas: &[f64],
    width: usize,
    height: usize,
    background: f64,
) -> Option<NormalizedRect> {
    let threshold = 10.0_f64.max(32.0_f64.min((background - 127.5).abs() * 0.20));
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, None, None);
    for y in 0..height {
        for x in 0..width {
            let luma = lumas[y * width + x];
            if (luma - background).abs() > threshold {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = Some(max_x.map_or(x, |value: usize| value.max(x)));
                max_y = Some(max_y.map_or(y, |value: usize| value.max(y)));
            }
        }
    }
    let (Some(max_x), Some(max_y)) = (max_x, max_y) else {
        return None;
    };
    let min_x = min_x.saturating_sub(2);
    let min_y = min_y.saturating_sub(2);
    let max_x = (max_x + 2).min(width.saturating_sub(1));
    let max_y = (max_y + 2).min(height.saturating_sub(1));
    Some(NormalizedRect {
        x: min_x as f64 / width as f64,
        y: min_y as f64 / height as f64,
        width: (max_x - min_x + 1) as f64 / width as f64,
        height: (max_y - min_y + 1) as f64 / height as f64,
    })
}

fn parse_normalized_crop(value: Option<&serde_json::Value>) -> Option<NormalizedRect> {
    let object = value?.as_object()?;
    let x = object.get("x")?.as_f64()?.clamp(0.0, 1.0);
    let y = object.get("y")?.as_f64()?.clamp(0.0, 1.0);
    let width = object.get("width")?.as_f64()?;
    let height = object.get("height")?.as_f64()?;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some(NormalizedRect {
        x,
        y,
        width: width.clamp(0.0, 1.0 - x),
        height: height.clamp(0.0, 1.0 - y),
    })
}

fn crop_normalized(image: &RgbaImage, rect: NormalizedRect) -> RgbaImage {
    let x = (rect.x * f64::from(image.width())).floor() as u32;
    let y = (rect.y * f64::from(image.height())).floor() as u32;
    let width = (rect.width * f64::from(image.width())).ceil().max(1.0) as u32;
    let height = (rect.height * f64::from(image.height())).ceil().max(1.0) as u32;
    let width = width.min(image.width().saturating_sub(x).max(1));
    let height = height.min(image.height().saturating_sub(y).max(1));
    imageops::crop_imm(image, x, y, width, height).to_image()
}

fn enhance_text_image(image: &RgbaImage) -> RgbaImage {
    let mut enhanced = RgbaImage::new(image.width(), image.height());
    for (x, y, pixel) in image.enumerate_pixels() {
        let [r, g, b, a] = pixel.0;
        let gray = (0.2126 * f64::from(r) + 0.7152 * f64::from(g) + 0.0722 * f64::from(b)) / 255.0;
        let adjusted = ((gray - 0.5) * 1.28 + 0.52).clamp(0.0, 1.0);
        let byte = (adjusted * 255.0).round() as u8;
        enhanced.put_pixel(x, y, image::Rgba([byte, byte, byte, a]));
    }
    imageops::unsharpen(&enhanced, 0.8, 4)
}

fn encode_processed_image(image: &RgbaImage, format: &str) -> Result<Vec<u8>, String> {
    let mut encoded = Vec::new();
    if format == "png" {
        PngEncoder::new(&mut encoded)
            .write_image(
                image.as_raw(),
                image.width(),
                image.height(),
                ColorType::Rgba8.into(),
            )
            .map_err(|error| format!("Failed to encode PNG: {error}"))?;
    } else {
        let rgb = DynamicImage::ImageRgba8(image.clone()).to_rgb8();
        JpegEncoder::new_with_quality(&mut encoded, 86)
            .encode(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                ColorType::Rgb8.into(),
            )
            .map_err(|error| format!("Failed to encode JPEG: {error}"))?;
    }
    Ok(encoded)
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = *chunk.get(1).unwrap_or(&0);
        let b2 = *chunk.get(2).unwrap_or(&0);
        output.push(TABLE[(b0 >> 2) as usize] as char);
        output.push(TABLE[(((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            output.push(TABLE[(((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(TABLE[(b2 & 0b0011_1111) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
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

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
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

fn cron_jobs() -> &'static Mutex<BTreeMap<String, CronJob>> {
    CRON_JOBS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn next_cron_id() -> String {
    let mut counter = CRON_ID_COUNTER
        .get_or_init(|| Mutex::new(0))
        .lock()
        .expect("cron id lock");
    *counter += 1;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    format!("{:08x}", (nanos as u64) ^ *counter)
}

fn cron_validation_error(cron: &str) -> Option<String> {
    let fields = cron.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return Some(String::from(
            "cron must be a valid 5-field expression, for example `*/5 * * * *` for every 5 minutes",
        ));
    }
    for (field, range, name) in [
        (fields[0], 0..60, "minute"),
        (fields[1], 0..24, "hour"),
        (fields[2], 1..32, "day-of-month"),
        (fields[3], 1..13, "month"),
        (fields[4], 0..7, "day-of-week"),
    ] {
        if !cron_field_is_valid(field, range.clone()) {
            return Some(format!(
                "invalid {name} field `{field}` in cron expression `{cron}`"
            ));
        }
    }
    None
}

fn cron_field_is_valid(field: &str, range: std::ops::Range<i32>) -> bool {
    if field == "*" {
        return true;
    }
    if field.contains(',') {
        return field
            .split(',')
            .all(|part| cron_field_is_valid(part, range.clone()));
    }
    if field.contains('/') {
        let parts = field.split('/').collect::<Vec<_>>();
        let [base, step] = parts.as_slice() else {
            return false;
        };
        let Ok(step) = step.parse::<i32>() else {
            return false;
        };
        if step <= 0 {
            return false;
        }
        if *base == "*" {
            return true;
        }
        return base
            .parse::<i32>()
            .is_ok_and(|value| range.contains(&value));
    }
    if field.contains('-') {
        let parts = field
            .split('-')
            .filter_map(|part| part.parse::<i32>().ok())
            .collect::<Vec<_>>();
        return parts.len() == 2
            && range.contains(&parts[0])
            && range.contains(&parts[1])
            && parts[0] <= parts[1];
    }
    field
        .parse::<i32>()
        .is_ok_and(|value| range.contains(&value))
}

fn cron_field_matches(field: &str, value: i32, range: std::ops::Range<i32>) -> bool {
    if field == "*" {
        return true;
    }
    if field.contains(',') {
        return field
            .split(',')
            .any(|part| cron_field_matches(part, value, range.clone()));
    }
    if field.contains('/') {
        let parts = field.split('/').collect::<Vec<_>>();
        let [base, step] = parts.as_slice() else {
            return false;
        };
        let base = if *base == "*" {
            range.start
        } else {
            base.parse::<i32>().unwrap_or(range.start)
        };
        let step = step.parse::<i32>().unwrap_or(1);
        return step > 0 && range.contains(&base) && value >= base && (value - base) % step == 0;
    }
    if field.contains('-') {
        let parts = field
            .split('-')
            .filter_map(|part| part.parse::<i32>().ok())
            .collect::<Vec<_>>();
        return parts.len() == 2 && value >= parts[0] && value <= parts[1];
    }
    field.parse::<i32>().is_ok_and(|number| number == value)
}

fn cron_next_fire(cron: &str, from: std::time::SystemTime) -> Option<std::time::SystemTime> {
    if cron_validation_error(cron).is_some() {
        return None;
    }
    let fields = cron.split_whitespace().collect::<Vec<_>>();
    let mut candidate = time::OffsetDateTime::from(from)
        .replace_second(0)
        .ok()?
        .replace_nanosecond(0)
        .ok()?
        + time::Duration::minutes(1);
    for _ in 0..(366 * 24 * 60) {
        let weekday = match candidate.weekday() {
            time::Weekday::Sunday => 0,
            time::Weekday::Monday => 1,
            time::Weekday::Tuesday => 2,
            time::Weekday::Wednesday => 3,
            time::Weekday::Thursday => 4,
            time::Weekday::Friday => 5,
            time::Weekday::Saturday => 6,
        };
        if cron_field_matches(fields[0], i32::from(candidate.minute()), 0..60)
            && cron_field_matches(fields[1], i32::from(candidate.hour()), 0..24)
            && cron_field_matches(fields[2], i32::from(candidate.day()), 1..32)
            && cron_field_matches(fields[3], i32::from(candidate.month() as u8), 1..13)
            && cron_field_matches(fields[4], weekday, 0..7)
        {
            return Some(candidate.into());
        }
        candidate += time::Duration::minutes(1);
    }
    None
}

fn cron_should_default_to_recurring(cron: &str) -> bool {
    let fields = cron.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return false;
    }
    let [minute, hour, day, month, weekday] = fields.as_slice() else {
        return false;
    };
    if fields
        .iter()
        .any(|field| field.contains('/') || field.contains(',') || field.contains('-'))
    {
        return true;
    }
    if *minute == "*" || *hour == "*" {
        return true;
    }
    if *day == "*" || *month == "*" {
        return true;
    }
    *weekday != "*"
}

fn cron_describe(cron: &str) -> String {
    let fields = cron.split_whitespace().collect::<Vec<_>>();
    let [minute, hour, day, month, weekday] = fields.as_slice() else {
        return format!("invalid cron: {cron}");
    };
    if *minute == "*" && *hour == "*" && *day == "*" && *month == "*" && *weekday == "*" {
        return String::from("every minute");
    }
    if let Some(mins) = minute.strip_prefix("*/")
        && *hour == "*"
        && *day == "*"
        && *month == "*"
    {
        return format!("every {mins} minutes");
    }
    if *minute == "0" && *hour == "*" && *day == "*" && *month == "*" {
        return String::from("every hour");
    }
    if *month == "*" && *weekday == "*" {
        return format!("at {hour}:{} daily", pad_left(minute, 2));
    }
    if *weekday != "*" {
        return format!(
            "at {hour}:{} on {}",
            pad_left(minute, 2),
            cron_describe_day_of_week(weekday)
        );
    }
    format!("cron: {cron}")
}

fn cron_describe_day_of_week(field: &str) -> String {
    let names = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    match field {
        "*" => String::from("every day"),
        "1-5" => String::from("weekdays"),
        "0,6" => String::from("weekends"),
        _ => field
            .parse::<usize>()
            .ok()
            .and_then(|day| names.get(day).copied())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| field.to_owned()),
    }
}

fn pad_left(value: &str, width: usize) -> String {
    if value.len() >= width {
        value.to_owned()
    } else {
        format!("{}{value}", "0".repeat(width - value.len()))
    }
}

fn format_cron_datetime(time: std::time::SystemTime) -> String {
    let datetime = time::OffsetDateTime::from(time);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        datetime.year(),
        u8::from(datetime.month()),
        datetime.day(),
        datetime.hour(),
        datetime.minute(),
        datetime.second()
    )
}

fn format_cron_time(time: std::time::SystemTime) -> String {
    let datetime = time::OffsetDateTime::from(time);
    format!(
        "{:02}:{:02}:{:02}",
        datetime.hour(),
        datetime.minute(),
        datetime.second()
    )
}

fn review_artifacts() -> &'static Mutex<BTreeMap<PathBuf, String>> {
    REVIEW_ARTIFACTS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn builtin_skills() -> &'static [BuiltinSkill] {
    &[
        BuiltinSkill {
            name: "commit",
            description: "Generate a git commit message for staged changes",
            when_to_use: None,
            prompt: "Generate a concise, descriptive git commit message for the staged changes.\n\nFollow these rules:\n1. Run `git diff --cached` to see what's staged\n2. Run `git log --oneline -10` to see recent commit style\n3. Write a one-line summary (under 72 chars) followed by a blank line, then details\n4. Use imperative mood: \"Add feature\" not \"Added feature\"\n5. Reference related issues with owner/repo#123 format\n6. End with: Co-authored-by: Deeptide <ds@deeptide.sh>\n\n$ARGUMENTS",
        },
        BuiltinSkill {
            name: "simplify",
            description: "Review code for reuse, quality, and efficiency",
            when_to_use: None,
            prompt: "Review the changed code for reuse opportunities, quality issues, and efficiency improvements. Then fix any issues found.\n\nCheck for:\n1. Duplicated logic that could be extracted into shared utilities\n2. Overly complex code that could be simplified\n3. Inefficient patterns (unnecessary allocations, repeated work)\n4. Dead code or unused imports\n5. Violations of existing codebase conventions\n\nAfter analysis, make minimal, targeted fixes. Do not refactor for the sake of refactoring.\n\n$ARGUMENTS",
        },
        BuiltinSkill {
            name: "review-pr",
            description: "Review a GitHub pull request",
            when_to_use: None,
            prompt: "Review the GitHub pull request provided.\n\nSteps:\n1. Run `gh pr view <number>` to get PR details\n2. Run `gh pr diff <number>` to see the changes\n3. Analyze the changes for: correctness, security, performance, test coverage, style\n4. Optionally run `gh pr review <number>` to submit your review\n\nProvide a clear, structured review with:\n- Summary of changes\n- Issues found (by severity)\n- Suggestions for improvement\n\n$ARGUMENTS",
        },
        BuiltinSkill {
            name: "init",
            description: "Scan this repo and write a TIDE.md project guide",
            when_to_use: None,
            prompt: "Bootstrap project memory for Deeptide in this repository.\n\nPhase 1 - explore without writing: inspect top-level layout, README, package manifests, CI config, and source structure.\nPhase 2 - check for existing project guides such as TIDE.md, AGENTS.md, CLAUDE.md, ZERO.md, CURSOR.md, or .cursorrules. Do not overwrite substantive existing guidance.\nPhase 3 - write TIDE.md with concise project-specific commands, architecture notes, conventions, and safety guidance.\nPhase 4 - summarize what was learned and what changed.\n\n$ARGUMENTS",
        },
        BuiltinSkill {
            name: "batch",
            description: "Plan and execute large parallelizable changes across the codebase",
            when_to_use: None,
            prompt: "Orchestrate a large, parallelizable change across this repository.\n\nPhase 1 - scope the files, symbols, and conventions touched by the request.\nPhase 2 - split work into independent units and use sub-agents or plan mode where useful.\nPhase 3 - verify with the project's tests or build commands.\nPhase 4 - report completed units, key files changed, and verification commands.\n\n$ARGUMENTS",
        },
        BuiltinSkill {
            name: "publish",
            description: "Publish or delete a static frontend on clide.app with Paean credentials",
            when_to_use: Some(
                "Use when the user asks to publish, deploy, upload, remove, unpublish, or delete a static frontend app/site to or from Paean Clide hosting / clide.app.",
            ),
            prompt: "Publish or delete the current project's static frontend via Paean Clide hosting.\n\nCall the native Publish tool for the operation. Do not run slash commands through Bash. Prefer built output directories over source: dist, build, out, .output/public, then public. Ensure a top-level index.html exists and report the published URL or authentication/build issue.\n\n$ARGUMENTS",
        },
        BuiltinSkill {
            name: "update-config",
            description: "Configure Deeptide CLI settings",
            when_to_use: None,
            prompt: "Help the user configure their Deeptide CLI settings.\n\nAvailable settings include model, api_key, max_turns, max_tokens, permission_mode, thinking, effort, and fast_mode. Use `tide config --set <key>=<value>` for global settings or add `--project` for project-specific settings. Use `tide config --show` to inspect current configuration.\n\n$ARGUMENTS",
        },
    ]
}

fn find_builtin_skill(name: &str) -> Option<&'static BuiltinSkill> {
    builtin_skills().iter().find(|skill| skill.name == name)
}

fn builtin_skill_descriptions() -> String {
    builtin_skills()
        .iter()
        .map(|skill| {
            let mut line = format!("  - {}: {}", skill.name, skill.description);
            if let Some(when) = skill.when_to_use {
                line.push_str(&format!("\n    When to use: {when}"));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn expand_skill_prompt(prompt: &str, args: Option<&str>) -> String {
    let Some(args) = args.filter(|value| !value.is_empty()) else {
        return prompt.replace("$ARGUMENTS", "");
    };
    let parts = args.splitn(11, ' ').collect::<Vec<_>>();
    let mut expanded = prompt.replace("$ARGUMENTS", args);
    for (index, part) in parts.iter().enumerate() {
        expanded = expanded.replace(&format!("${}", index + 1), part);
    }
    expanded
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishOptions {
    dir: Option<String>,
    handle: Option<String>,
    random: bool,
    delete: bool,
    dry_run: bool,
    status: bool,
}

impl PublishOptions {
    fn from_input(input: &serde_json::Value) -> Result<Self, String> {
        let option = |key: &str| {
            input
                .get(key)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned)
        };
        let options = Self {
            dir: option("dir"),
            handle: option("handle"),
            random: input
                .get("random")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            delete: input
                .get("delete")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            dry_run: input
                .get("dry_run")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
            status: input
                .get("status")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(false),
        };
        if options.random && options.handle.is_some() {
            return Err(String::from("Use either random or handle, not both."));
        }
        if options.delete && options.random {
            return Err(String::from("Use either delete or random, not both."));
        }
        if options.delete && options.dir.is_some() {
            return Err(String::from(
                "dir is only valid when publishing, not deleting.",
            ));
        }
        if options.status
            && (options.delete
                || options.dry_run
                || options.dir.is_some()
                || options.handle.is_some()
                || options.random)
        {
            return Err(String::from(
                "status cannot be combined with publish/delete options.",
            ));
        }
        Ok(options)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct PublishState {
    #[serde(skip_serializing_if = "Option::is_none")]
    handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(rename = "publishDir")]
    #[serde(skip_serializing_if = "Option::is_none")]
    publish_dir: Option<String>,
    #[serde(rename = "fileCount")]
    #[serde(skip_serializing_if = "Option::is_none")]
    file_count: Option<u64>,
    #[serde(rename = "totalBytes")]
    #[serde(skip_serializing_if = "Option::is_none")]
    total_bytes: Option<u64>,
    #[serde(rename = "publishedAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    published_at: Option<String>,
    #[serde(rename = "deletedAt")]
    #[serde(skip_serializing_if = "Option::is_none")]
    deleted_at: Option<String>,
    #[serde(rename = "lastDeletedHandle")]
    #[serde(skip_serializing_if = "Option::is_none")]
    last_deleted_handle: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishPlan {
    publish_dir_relative: String,
    handle_description: String,
    handle: Option<String>,
    random: bool,
    file_count: usize,
    total_bytes: u64,
    has_index: bool,
    sample_files: Vec<String>,
    notes: Vec<String>,
}

const CLIDEIGNORE_FILE: &str = ".clideignore";
const CLIDEIGNORE_HEADER: &str = "# Added by Clide publish safety defaults";
const CLIDEIGNORE_SAFETY_PATTERNS: &[&str] = &[
    ".clide/",
    ".env",
    ".env.*",
    "*.pem",
    "*.key",
    "*.p12",
    "*.pfx",
    "id_rsa*",
    ".npmrc",
    ".aws/",
    ".gcloud/",
    "service-account*.json",
    "secrets/",
    "private/",
    "node_modules/",
    ".git/",
    ".next/cache/",
    "dist/*.map",
    ".vscode/",
    ".idea/",
    "coverage/",
    "*.log",
    ".DS_Store",
    "Thumbs.db",
    "*.tmp",
];

fn render_publish_status(context: &ToolContext) -> String {
    let Some(state) = load_publish_state(context) else {
        return String::from("No saved clide.app publish state at .clide/publish.json");
    };
    let mut lines = vec![String::from("Clide publish status")];
    if let Some(handle) = state.handle {
        lines.push(format!("  Handle:      {handle}"));
    }
    if let Some(url) = state.url {
        lines.push(format!("  URL:         {url}"));
    }
    if let Some(dir) = state.publish_dir {
        lines.push(format!("  Directory:   {dir}"));
    }
    if let Some(file_count) = state.file_count {
        lines.push(format!("  Files:       {file_count}"));
    }
    if let Some(total_bytes) = state.total_bytes {
        lines.push(format!("  Bytes:       {total_bytes}"));
    }
    if let Some(published_at) = state.published_at {
        lines.push(format!("  Published:   {published_at}"));
    }
    if let Some(deleted_at) = state.deleted_at {
        lines.push(format!("  Deleted:     {deleted_at}"));
    }
    if let Some(handle) = state.last_deleted_handle {
        lines.push(format!("  Last deleted handle: {handle}"));
    }
    lines.push(String::from("  State:       .clide/publish.json"));
    lines.join("\n")
}

fn load_publish_state(context: &ToolContext) -> Option<PublishState> {
    let path = context.cwd.join(".clide").join("publish.json");
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn resolve_publish_dir(
    context: &ToolContext,
    explicit_dir: Option<&str>,
) -> Result<PathBuf, String> {
    if let Some(dir) = explicit_dir {
        let path = context.resolve_path(dir);
        if !path.is_dir() {
            return Err(format!("Publish directory does not exist: {dir}"));
        }
        if !path.join("index.html").is_file() {
            return Err(format!(
                "Publish directory must contain a top-level `index.html`: {dir}"
            ));
        }
        return Ok(path);
    }

    for candidate in ["dist", "build", "out", ".output/public", "public"] {
        let path = context.cwd.join(candidate);
        if path.is_dir() && path.join("index.html").is_file() {
            return Ok(normalize_path(path));
        }
    }
    if context.cwd.join("index.html").is_file() {
        return Ok(normalize_path(context.cwd.clone()));
    }
    if package_json_has_build_script(&context.cwd) {
        return Err(String::from(
            "No built output with top-level `index.html` found. This project has a package.json build script; run the build first, then publish dist, build, out, .output/public, public, or project root.",
        ));
    }
    Err(String::from(
        "No publishable static directory found. Expected top-level `index.html` in `dist`, `build`, `out`, `.output/public`, `public`, or project root.",
    ))
}

fn package_json_has_build_script(project_root: &Path) -> bool {
    let Ok(text) = fs::read_to_string(project_root.join("package.json")) else {
        return false;
    };
    serde_json::from_str::<serde_json::Value>(&text)
        .ok()
        .and_then(|value| value.get("scripts").cloned())
        .and_then(|scripts| scripts.get("build").cloned())
        .is_some()
}

fn ensure_clideignore_safety_defaults(project_root: &Path) -> Result<(), String> {
    let path = project_root.join(CLIDEIGNORE_FILE);
    let exists = path.exists();
    let mut text = if exists {
        fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read .clideignore: {error}"))?
    } else {
        String::from(
            "# .clideignore - files and folders excluded from Clide publish.\n# Gitignore-style: one pattern per line, # for comments.\n# The .clideignore file itself is never published.\n\n",
        )
    };
    let existing = load_clideignore_patterns_from_text(&text)
        .into_iter()
        .map(|pattern| normalize_publish_ignore_pattern(&pattern))
        .collect::<std::collections::BTreeSet<_>>();
    let missing = CLIDEIGNORE_SAFETY_PATTERNS
        .iter()
        .filter(|pattern| !existing.contains(normalize_publish_ignore_pattern(pattern).as_str()))
        .copied()
        .collect::<Vec<_>>();
    if exists && missing.is_empty() {
        return Ok(());
    }
    if !text.is_empty() && !text.ends_with('\n') {
        text.push('\n');
    }
    if !text.contains(CLIDEIGNORE_HEADER) {
        if !text.ends_with("\n\n") {
            text.push('\n');
        }
        text.push_str(CLIDEIGNORE_HEADER);
        text.push('\n');
    }
    for pattern in missing {
        text.push_str(pattern);
        text.push('\n');
    }
    fs::write(&path, text).map_err(|error| format!("Failed to write .clideignore: {error}"))
}

fn load_clideignore_patterns(project_root: &Path) -> Vec<String> {
    fs::read_to_string(project_root.join(CLIDEIGNORE_FILE))
        .map(|text| load_clideignore_patterns_from_text(&text))
        .unwrap_or_default()
}

fn load_clideignore_patterns_from_text(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(normalize_publish_ignore_pattern)
        .filter(|line| !line.is_empty())
        .collect()
}

fn normalize_publish_ignore_pattern(pattern: &str) -> String {
    pattern.trim().trim_end_matches('/').to_owned()
}

fn collect_publish_files(
    project_root: &Path,
    publish_dir: &Path,
    patterns: &[String],
) -> Result<Vec<String>, String> {
    let mut files = Vec::new();
    collect_publish_files_inner(project_root, publish_dir, publish_dir, patterns, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_publish_files_inner(
    project_root: &Path,
    publish_root: &Path,
    dir: &Path,
    patterns: &[String],
    files: &mut Vec<String>,
) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|error| {
        format!(
            "Failed to read publish directory {}: {error}",
            dir.display()
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {error}"))?;
        let path = entry.path();
        let rel_publish = relative_path_string(publish_root, &path);
        let rel_project = relative_path_string(project_root, &path);
        if rel_publish.contains('\n') {
            return Err(format!(
                "Cannot publish files with newline in path: {rel_publish}"
            ));
        }
        if publish_path_is_ignored(&rel_project, &rel_publish, patterns) {
            continue;
        }
        let metadata = entry
            .metadata()
            .map_err(|error| format!("Failed to inspect {}: {error}", path.display()))?;
        if metadata.is_dir() {
            collect_publish_files_inner(project_root, publish_root, &path, patterns, files)?;
        } else if metadata.is_file() {
            files.push(rel_publish);
        }
    }
    Ok(())
}

fn publish_path_is_ignored(rel_project: &str, rel_publish: &str, patterns: &[String]) -> bool {
    if rel_project == CLIDEIGNORE_FILE || rel_publish == CLIDEIGNORE_FILE {
        return true;
    }
    if rel_project == ".clide" || rel_project.starts_with(".clide/") {
        return true;
    }
    patterns.iter().any(|pattern| {
        publish_pattern_matches(pattern, rel_project)
            || publish_pattern_matches(pattern, rel_publish)
    })
}

fn publish_pattern_matches(pattern: &str, relative_path: &str) -> bool {
    let pattern = normalize_publish_ignore_pattern(pattern)
        .trim_matches('/')
        .to_owned();
    let relative_path = relative_path
        .replace('\\', "/")
        .trim_matches('/')
        .to_owned();
    if pattern.is_empty() || relative_path.is_empty() {
        return false;
    }
    if pattern.contains('/') {
        return GlobMatcher::new(&pattern).matches(Path::new(&relative_path))
            || relative_path.starts_with(&(pattern.clone() + "/"))
            || GlobMatcher::new(&(pattern + "/*")).matches(Path::new(&relative_path));
    }
    relative_path
        .split('/')
        .any(|part| matches_glob_segment(part, &pattern))
        || matches_glob_segment(&relative_path, &pattern)
        || relative_path.starts_with(&(pattern + "/"))
}

fn build_publish_plan(
    project_root: &Path,
    publish_dir: &Path,
    files: &[String],
    handle: Option<String>,
    random: bool,
) -> Result<PublishPlan, String> {
    let mut total_bytes = 0u64;
    for file in files {
        let metadata = fs::metadata(publish_dir.join(file))
            .map_err(|error| format!("Failed to inspect {file}: {error}"))?;
        total_bytes = total_bytes.saturating_add(metadata.len());
    }
    let mut notes = Vec::new();
    if random {
        notes.push(String::from(
            "A new random handle will be assigned by the server.",
        ));
    } else if handle.is_none() {
        notes.push(String::from(
            "No saved or explicit handle; server will assign a handle.",
        ));
    } else {
        notes.push(String::from(
            "Existing or explicit handle will be overwritten if it already exists.",
        ));
    }
    if files.iter().any(|file| file.ends_with(".map")) {
        notes.push(String::from(
            "Source maps are included; add a .clideignore rule if they should stay private.",
        ));
    }
    Ok(PublishPlan {
        publish_dir_relative: relative_path_string(project_root, publish_dir),
        handle_description: if random {
            String::from("(random)")
        } else {
            handle
                .clone()
                .unwrap_or_else(|| String::from("(server assigned)"))
        },
        handle,
        random,
        file_count: files.len(),
        total_bytes,
        has_index: files.iter().any(|file| file == "index.html"),
        sample_files: files.iter().take(6).cloned().collect(),
        notes,
    })
}

fn render_publish_dry_run(plan: &PublishPlan) -> String {
    let rel_dir = if plan.publish_dir_relative.is_empty() {
        "."
    } else {
        &plan.publish_dir_relative
    };
    let mut lines = vec![
        String::from("Publish dry run: ready"),
        format!("  Directory:  {rel_dir}"),
        format!("  Handle:     {}", plan.handle_description),
        format!("  Files:      {}", plan.file_count),
        format!("  Bytes:      {}", plan.total_bytes),
        format!(
            "  Index:      {}",
            if plan.has_index { "yes" } else { "no" }
        ),
        String::from("  Ignore:     .clideignore safety defaults present"),
    ];
    if !plan.sample_files.is_empty() {
        lines.push(format!("  Sample:     {}", plan.sample_files.join(", ")));
    }
    if !plan.notes.is_empty() {
        lines.push(String::from("  Notes:"));
        lines.extend(plan.notes.iter().map(|note| format!("    - {note}")));
    }
    lines.join("\n")
}

fn relative_path_string(base: &Path, child: &Path) -> String {
    let base = normalize_path(base.to_path_buf());
    let child = normalize_path(child.to_path_buf());
    child
        .strip_prefix(&base)
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| child.to_string_lossy().replace('\\', "/"))
        .trim_start_matches('/')
        .to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PublishAuth {
    base_url: String,
    token: String,
}

#[derive(Debug, Deserialize)]
struct PublishUploadResponse {
    success: bool,
    data: Option<PublishResult>,
    error: Option<String>,
    reason: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PublishResult {
    handle: String,
    #[serde(rename = "assignedHandle")]
    assigned_handle: Option<String>,
    url: String,
    #[serde(rename = "shortUrl")]
    short_url: Option<String>,
    #[serde(rename = "fileCount")]
    file_count: u64,
    #[serde(rename = "totalBytes")]
    total_bytes: u64,
    #[serde(default)]
    overwritten: bool,
    #[serde(rename = "archiveUrl")]
    archive_url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DeletePublishResult {
    success: bool,
    handle: Option<String>,
    #[serde(rename = "deletedObjects")]
    deleted_objects: Option<u64>,
    error: Option<String>,
    reason: Option<String>,
}

fn upload_publish(
    context: &ToolContext,
    publish_dir: &Path,
    files: &[String],
    plan: &PublishPlan,
) -> Result<String, String> {
    let auth = resolve_publish_auth()?;
    let zip_path = create_publish_zip(publish_dir, files)?;
    let result = (|| {
        validate_publish_login(&auth)?;
        let response = publish_zip(&auth, &zip_path, publish_dir, plan)?;
        save_publish_state(context, &response, publish_dir)?;
        Ok(render_publish_success(&response, context, publish_dir))
    })();
    let _ = fs::remove_file(&zip_path);
    result
}

fn delete_publish(context: &ToolContext, handle: &str) -> Result<String, String> {
    let auth = resolve_publish_auth()?;
    validate_publish_login(&auth)?;
    let result = unpublish_handle(&auth, handle)?;
    mark_publish_deleted(context, handle)?;
    Ok(render_delete_success(&result, handle))
}

fn resolve_publish_auth() -> Result<PublishAuth, String> {
    let token = ["PAEAN_API_TOKEN", "PAEAN_TOKEN", "CLIDE_API_TOKEN"]
        .into_iter()
        .find_map(|key| std::env::var(key).ok())
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            String::from(
                "Paean login is missing or expired. Set PAEAN_API_TOKEN, PAEAN_TOKEN, or CLIDE_API_TOKEN, then retry Publish.",
            )
        })?;
    let base_url = std::env::var("PAEAN_API_BASE_URL")
        .or_else(|_| std::env::var("CLIDE_API_BASE_URL"))
        .unwrap_or_else(|_| String::from("https://api.paean.ai"));
    Ok(PublishAuth {
        base_url: normalize_publish_base_url(&base_url),
        token,
    })
}

fn normalize_publish_base_url(raw: &str) -> String {
    let mut base = raw.trim().trim_end_matches('/').to_owned();
    if base.ends_with("/zero") {
        base.truncate(base.len() - "/zero".len());
    }
    if base.is_empty() {
        String::from("https://api.paean.ai")
    } else {
        base
    }
}

fn validate_publish_login(auth: &PublishAuth) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| format!("Failed to build publish HTTP client: {error}"))?;
    let url = format!("{}/publish/clide/check?handle=", auth.base_url);
    let response = client
        .get(url)
        .bearer_auth(&auth.token)
        .send()
        .map_err(|error| format!("Publish login check failed: {error}"))?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(String::from(
            "Paean login is missing or expired. Refresh the publish token, then retry Publish.",
        ));
    }
    if !response.status().is_success() {
        let status = response.status().as_u16();
        let message = publish_response_message(response);
        return Err(format!(
            "Publish API {status}: {}",
            message.unwrap_or_else(|| String::from("login check failed"))
        ));
    }
    Ok(())
}

fn publish_zip(
    auth: &PublishAuth,
    zip_path: &Path,
    publish_dir: &Path,
    plan: &PublishPlan,
) -> Result<PublishResult, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| format!("Failed to build publish HTTP client: {error}"))?;
    let file = fs::File::open(zip_path)
        .map_err(|error| format!("Failed to open publish archive: {error}"))?;
    let file_name = publish_dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("site");
    let part = reqwest::blocking::multipart::Part::reader(file)
        .file_name(format!("{file_name}.zip"))
        .mime_str("application/zip")
        .map_err(|error| format!("Failed to prepare publish archive part: {error}"))?;
    let mut form = reqwest::blocking::multipart::Form::new().part("archive", part);
    if !plan.random
        && let Some(handle) = plan.handle.as_ref().filter(|value| !value.is_empty())
    {
        form = form.text("handle", handle.clone());
    }
    let response = client
        .post(format!("{}/publish/clide", auth.base_url))
        .bearer_auth(&auth.token)
        .multipart(form)
        .send()
        .map_err(|error| format!("Publish upload failed: {error}"))?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(String::from(
            "Paean login is missing or expired. Refresh the publish token, then retry Publish.",
        ));
    }
    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|error| format!("Failed to read publish response: {error}"))?;
    if !(200..=299).contains(&status) {
        return Err(format!(
            "Publish API {status}: {}",
            parse_publish_error_message(&body).unwrap_or_else(|| body.trim().to_owned())
        ));
    }
    let decoded: PublishUploadResponse = serde_json::from_str(&body)
        .map_err(|error| format!("Unexpected publish response: {error}"))?;
    if !decoded.success {
        return Err(format!(
            "Publish API {status}: {}",
            decoded
                .error
                .or(decoded.reason)
                .unwrap_or_else(|| String::from("Publish failed"))
        ));
    }
    decoded
        .data
        .ok_or_else(|| String::from("Unexpected publish response: missing data"))
}

fn unpublish_handle(auth: &PublishAuth, handle: &str) -> Result<DeletePublishResult, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| format!("Failed to build publish HTTP client: {error}"))?;
    let encoded = percent_encode_path_segment(handle);
    let response = client
        .delete(format!("{}/publish/{encoded}", auth.base_url))
        .bearer_auth(&auth.token)
        .send()
        .map_err(|error| format!("Publish delete failed: {error}"))?;
    if matches!(response.status().as_u16(), 401 | 403) {
        return Err(String::from(
            "Paean login is missing or expired. Refresh the publish token, then retry Publish.",
        ));
    }
    let status = response.status().as_u16();
    let body = response
        .text()
        .map_err(|error| format!("Failed to read publish delete response: {error}"))?;
    if !(200..=299).contains(&status) {
        return Err(format!(
            "Publish API {status}: {}",
            parse_publish_error_message(&body).unwrap_or_else(|| body.trim().to_owned())
        ));
    }
    let decoded: DeletePublishResult = serde_json::from_str(&body)
        .map_err(|error| format!("Unexpected publish delete response: {error}"))?;
    if !decoded.success {
        return Err(format!(
            "Publish API {status}: {}",
            decoded
                .error
                .or(decoded.reason)
                .unwrap_or_else(|| String::from("Delete failed"))
        ));
    }
    Ok(decoded)
}

fn create_publish_zip(publish_dir: &Path, files: &[String]) -> Result<PathBuf, String> {
    let tmp_dir = std::env::temp_dir().join(format!(
        "deeptide-publish-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0)
    ));
    fs::create_dir_all(&tmp_dir)
        .map_err(|error| format!("Failed to create publish archive directory: {error}"))?;
    let zip_path = tmp_dir.join("site.zip");
    let file = fs::File::create(&zip_path)
        .map_err(|error| format!("Failed to create publish archive: {error}"))?;
    let mut writer = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    for relative in files {
        let path = publish_dir.join(relative);
        writer
            .start_file(relative.replace('\\', "/"), options)
            .map_err(|error| format!("Failed to add {relative} to publish archive: {error}"))?;
        let mut input = fs::File::open(&path)
            .map_err(|error| format!("Failed to read publish file {relative}: {error}"))?;
        io::copy(&mut input, &mut writer)
            .map_err(|error| format!("Failed to write {relative} to publish archive: {error}"))?;
    }
    writer
        .finish()
        .map_err(|error| format!("Failed to finalize publish archive: {error}"))?;
    Ok(zip_path)
}

fn save_publish_state(
    context: &ToolContext,
    result: &PublishResult,
    publish_dir: &Path,
) -> Result<(), String> {
    let handle = result
        .assigned_handle
        .clone()
        .unwrap_or_else(|| result.handle.clone());
    let state = PublishState {
        handle: Some(handle),
        url: Some(result.url.clone()),
        publish_dir: Some({
            let rel = relative_path_string(&context.cwd, publish_dir);
            if rel.is_empty() {
                String::from(".")
            } else {
                rel
            }
        }),
        file_count: Some(result.file_count),
        total_bytes: Some(result.total_bytes),
        published_at: Some(format_cron_datetime(std::time::SystemTime::now())),
        deleted_at: None,
        last_deleted_handle: None,
    };
    write_publish_state(context, &state)
}

fn mark_publish_deleted(context: &ToolContext, handle: &str) -> Result<(), String> {
    let Some(mut state) = load_publish_state(context) else {
        return Ok(());
    };
    if state.handle.as_deref() != Some(handle) {
        return Ok(());
    }
    state.last_deleted_handle = Some(handle.to_owned());
    state.deleted_at = Some(format_cron_datetime(std::time::SystemTime::now()));
    state.handle = None;
    write_publish_state(context, &state)
}

fn write_publish_state(context: &ToolContext, state: &PublishState) -> Result<(), String> {
    let dir = context.cwd.join(".clide");
    fs::create_dir_all(&dir).map_err(|error| format!("Failed to create .clide: {error}"))?;
    let data = serde_json::to_string_pretty(state)
        .map_err(|error| format!("Failed to encode publish state: {error}"))?;
    fs::write(dir.join("publish.json"), format!("{data}\n"))
        .map_err(|error| format!("Failed to write .clide/publish.json: {error}"))
}

fn render_publish_success(
    result: &PublishResult,
    context: &ToolContext,
    publish_dir: &Path,
) -> String {
    let handle = result
        .assigned_handle
        .as_deref()
        .unwrap_or(result.handle.as_str());
    let rel_dir = {
        let rel = relative_path_string(&context.cwd, publish_dir);
        if rel.is_empty() {
            String::from(".")
        } else {
            rel
        }
    };
    [
        format!("Published: {}", result.url),
        format!("  Handle:     {handle}"),
        format!("  Directory:  {rel_dir}"),
        format!("  Files:      {}", result.file_count),
        format!("  Bytes:      {}", result.total_bytes),
        format!(
            "  Overwrote:  {}",
            if result.overwritten { "yes" } else { "no" }
        ),
        String::from("  State:      .clide/publish.json"),
    ]
    .join("\n")
}

fn render_delete_success(result: &DeletePublishResult, fallback_handle: &str) -> String {
    let handle = result.handle.as_deref().unwrap_or(fallback_handle);
    let mut lines = vec![format!("Deleted remote publish: {handle}.clide.app")];
    if let Some(count) = result.deleted_objects {
        lines.push(format!("  Deleted objects: {count}"));
    }
    lines.push(String::from("  State: .clide/publish.json"));
    lines.join("\n")
}

fn publish_response_message(response: reqwest::blocking::Response) -> Option<String> {
    response
        .text()
        .ok()
        .and_then(|body| {
            parse_publish_error_message(&body).or_else(|| Some(body.trim().to_owned()))
        })
        .filter(|message| !message.is_empty())
}

fn parse_publish_error_message(body: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|value| {
            value
                .get("error")
                .or_else(|| value.get("reason"))
                .and_then(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
        })
}

fn edit_notebook(
    path: &Path,
    edit_mode: &str,
    cell_id: Option<&str>,
    new_source: &str,
    cell_type: &str,
) -> Result<String, String> {
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read notebook {}: {error}", path.display()))?;
    let mut notebook: serde_json::Value =
        serde_json::from_str(&raw).map_err(|error| format!("Invalid notebook format: {error}"))?;
    let Some(cells) = notebook
        .get_mut("cells")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return Err(String::from("Invalid notebook format"));
    };

    let result = match edit_mode {
        "delete" => {
            let Some(cell_id) = cell_id else {
                return Err(String::from("cell_id required for delete mode"));
            };
            let Some(index) = cells.iter().position(|cell| {
                cell.get("id").and_then(serde_json::Value::as_str) == Some(cell_id)
            }) else {
                return Err(format!("Cell not found: {cell_id}"));
            };
            cells.remove(index);
            format!("Cell {cell_id} deleted.")
        }
        "insert" => {
            let insert_index = cell_id
                .and_then(|cell_id| {
                    cells
                        .iter()
                        .position(|cell| {
                            cell.get("id").and_then(serde_json::Value::as_str) == Some(cell_id)
                        })
                        .map(|index| index + 1)
                })
                .unwrap_or(0);
            let new_cell = serde_json::json!({
                "id": generated_cell_id(),
                "cell_type": cell_type,
                "source": [new_source],
                "metadata": {},
                "outputs": [],
                "execution_count": serde_json::Value::Null,
            });
            cells.insert(insert_index.min(cells.len()), new_cell);
            format!("Cell inserted at position {insert_index}.")
        }
        _ => {
            let Some(cell_id) = cell_id else {
                return Err(String::from("cell_id required for replace mode"));
            };
            let Some(cell) = cells
                .iter_mut()
                .find(|cell| cell.get("id").and_then(serde_json::Value::as_str) == Some(cell_id))
            else {
                return Err(format!("Cell not found: {cell_id}"));
            };
            cell["source"] = serde_json::json!([new_source]);
            cell["cell_type"] = serde_json::Value::String(cell_type.to_owned());
            format!("Cell {cell_id} replaced.")
        }
    };

    let formatted = serde_json::to_string_pretty(&notebook)
        .map_err(|error| format!("Failed to encode notebook: {error}"))?;
    fs::write(path, format!("{formatted}\n"))
        .map_err(|error| format!("Failed to write notebook {}: {error}", path.display()))?;
    Ok(result)
}

fn generated_cell_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("deeptide-{nanos:x}")
}

fn is_git_repository(cwd: &Path) -> bool {
    run_git(&["rev-parse", "--is-inside-work-tree"], cwd)
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn run_git(args: &[&str], cwd: &Path) -> io::Result<std::process::Output> {
    Command::new("git").args(args).current_dir(cwd).output()
}

fn default_worktree_name() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("deeptide-worktree-{millis:x}")
}

fn sanitize_worktree_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn worktree_branch_for_path(cwd: &Path, worktree_path: &Path) -> Option<String> {
    let output = run_git(&["worktree", "list", "--porcelain"], cwd).ok()?;
    if !output.status.success() {
        return None;
    }
    let target = normalize_path(worktree_path.to_path_buf());
    let mut found = false;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            found = normalize_path(PathBuf::from(path)) == target;
            continue;
        }
        if found && let Some(branch) = line.strip_prefix("branch refs/heads/") {
            return Some(branch.to_owned());
        }
    }
    None
}

fn render_plan_verification(
    input: &serde_json::Value,
    context: &ToolContext,
) -> Result<String, String> {
    let diff = run_git(&["diff", "--name-status", "HEAD"], &context.cwd)
        .map_err(|error| format!("Failed to run git diff: {error}"))?;
    if !diff.status.success() {
        return Err(format!(
            "Failed to run git diff: {}",
            String::from_utf8_lossy(&diff.stderr)
                .trim()
                .if_empty("unknown error")
        ));
    }
    let untracked = run_git(
        &["ls-files", "--others", "--exclude-standard"],
        &context.cwd,
    )
    .map_err(|error| format!("Failed to run git ls-files: {error}"))?;
    if !untracked.status.success() {
        return Err(format!(
            "Failed to run git ls-files: {}",
            String::from_utf8_lossy(&untracked.stderr)
                .trim()
                .if_empty("unknown error")
        ));
    }

    let mut lines = vec![String::from("Plan Verification Report:")];
    let mut changed = std::collections::BTreeSet::new();
    for line in String::from_utf8_lossy(&diff.stdout).lines() {
        let mut parts = line.splitn(2, '\t');
        let status = parts.next().unwrap_or_default();
        let file = parts.next().unwrap_or_default();
        if file.is_empty() {
            continue;
        }
        changed.insert(file.to_owned());
        let label = match status.chars().next() {
            Some('A') => "ADDED",
            Some('M') => "MODIFIED",
            Some('D') => "DELETED",
            Some('R') => "RENAMED",
            _ => status,
        };
        lines.push(format!("  [{label}] {file}"));
    }
    for file in String::from_utf8_lossy(&untracked.stdout)
        .lines()
        .filter(|line| !line.trim().is_empty())
    {
        changed.insert(file.to_owned());
        lines.push(format!("  [NEW] {file}"));
    }

    let expected = input
        .get("expected_files")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !expected.is_empty() {
        lines.push(String::new());
        lines.push(String::from("Expected files:"));
        let mut all_found = true;
        for file in expected {
            let normalized = context
                .resolve_path(&file)
                .strip_prefix(&context.cwd)
                .ok()
                .and_then(|path| path.to_str())
                .map(ToOwned::to_owned)
                .unwrap_or_else(|| file.clone());
            if changed.contains(&file) || changed.contains(&normalized) {
                lines.push(format!("  OK {file}"));
            } else {
                lines.push(format!("  MISSING {file} - NOT FOUND in changes"));
                all_found = false;
            }
        }
        if all_found {
            lines.push(String::new());
            lines.push(String::from("All expected changes verified."));
        }
    }

    if lines.len() == 1 {
        lines.push(String::from("  (no changes detected)"));
    }
    Ok(lines.join("\n"))
}

fn load_remote_trigger_settings(
    context: &ToolContext,
) -> Result<Option<RemoteTriggerSettings>, String> {
    let mut settings = RemoteTriggerSettings::default();
    for path in remote_trigger_settings_paths(&context.cwd) {
        let Some(layer) = read_remote_trigger_settings(&path)? else {
            continue;
        };
        if layer.url.is_some() {
            settings.url = layer.url;
        }
        if layer.token.is_some() {
            settings.token = layer.token;
        }
        if layer.headers.is_some() {
            settings.headers = layer.headers;
        }
    }

    if let Ok(url) = std::env::var("DEEPTIDE_REMOTE_TRIGGER_URL")
        && !url.trim().is_empty()
    {
        settings.url = Some(url);
    }
    if let Ok(token) = std::env::var("DEEPTIDE_REMOTE_TRIGGER_TOKEN")
        && !token.trim().is_empty()
    {
        settings.token = Some(token);
    }

    if settings.url.is_none() && settings.token.is_none() && settings.headers.is_none() {
        Ok(None)
    } else {
        Ok(Some(settings))
    }
}

fn read_remote_trigger_settings(path: &Path) -> Result<Option<RemoteTriggerSettings>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path)
        .map_err(|error| format!("Failed to read settings file {}: {error}", path.display()))?;
    let json: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Failed to parse settings file {}: {error}", path.display()))?;
    let Some(remote_trigger) = json.get("remote_trigger") else {
        return Ok(None);
    };
    serde_json::from_value(remote_trigger.clone())
        .map(Some)
        .map_err(|error| {
            format!(
                "Failed to parse settings.remote_trigger in {}: {error}",
                path.display()
            )
        })
}

fn remote_trigger_settings_paths(cwd: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(config_dir) = tide_config_dir() {
        let project = project_settings_slug(cwd);
        paths.push(config_dir.join("settings.json"));
        paths.push(
            config_dir
                .join("projects")
                .join(&project)
                .join("settings.json"),
        );
        paths.push(
            config_dir
                .join("projects")
                .join(project)
                .join("settings.local.json"),
        );
    }
    if let Some(home) = home_dir() {
        paths.push(home.join(".deeptide").join("settings.json"));
        paths.push(home.join(".deeptide").join("config.json"));
    }
    paths.push(cwd.join(".deeptide").join("settings.json"));
    paths.push(cwd.join(".deeptide").join("config.json"));
    paths
}

fn tide_config_dir() -> Option<PathBuf> {
    std::env::var_os("TIDE_CONFIG_DIR")
        .map(PathBuf::from)
        .or_else(|| home_dir().map(|home| home.join(".config").join("tide")))
}

fn project_settings_slug(cwd: &Path) -> String {
    let normalized = cwd.to_string_lossy().replace('\\', "/");
    let mut slug = normalized.replace('/', "-");
    while slug.starts_with('-') {
        slug.remove(0);
    }
    if slug.len() > 64 {
        slug = slug.chars().take(64).collect();
    }
    slug
}

fn post_desktop_notification(
    title: &str,
    subtitle: Option<&str>,
    message: &str,
    sound: bool,
) -> ToolResult {
    post_desktop_notification_impl(title, subtitle, message, sound)
}

#[cfg(target_os = "macos")]
fn post_desktop_notification_impl(
    title: &str,
    subtitle: Option<&str>,
    message: &str,
    sound: bool,
) -> ToolResult {
    let mut script = format!(
        "display notification {} with title {}",
        applescript_string(message),
        applescript_string(title)
    );
    if let Some(subtitle) = subtitle.filter(|value| !value.is_empty()) {
        script.push_str(" subtitle ");
        script.push_str(&applescript_string(subtitle));
    }
    if sound {
        script.push_str(" sound name \"Submarine\"");
    }
    let output = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .output();
    match output {
        Ok(output) if output.status.success() => {
            ToolResult::text(format!("Notification posted: {title}"))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            ToolResult::error(format!(
                "osascript exited with code {}. {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            ))
        }
        Err(error) => ToolResult::error(format!("Failed to spawn osascript: {error}")),
    }
}

#[cfg(target_os = "linux")]
fn post_desktop_notification_impl(
    title: &str,
    subtitle: Option<&str>,
    message: &str,
    _sound: bool,
) -> ToolResult {
    let mut command = Command::new("notify-send");
    command.arg(title).arg(message);
    if let Some(subtitle) = subtitle.filter(|value| !value.is_empty()) {
        command
            .arg("--hint")
            .arg(format!("string:deeptide-subtitle:{subtitle}"));
    }
    let output = command.output();
    match output {
        Ok(output) if output.status.success() => {
            ToolResult::text(format!("Notification posted: {title}"))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            ToolResult::error(format!(
                "notify-send exited with code {}. {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            ))
        }
        Err(error) => ToolResult::error(format!(
            "Failed to spawn notify-send. Install libnotify-bin or equivalent: {error}"
        )),
    }
}

#[cfg(target_os = "windows")]
fn post_desktop_notification_impl(
    _title: &str,
    _subtitle: Option<&str>,
    _message: &str,
    _sound: bool,
) -> ToolResult {
    ToolResult::error(
        "PushNotification is not implemented on Windows yet. Use normal assistant output or a RemoteTrigger webhook.",
    )
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn post_desktop_notification_impl(
    _title: &str,
    _subtitle: Option<&str>,
    _message: &str,
    _sound: bool,
) -> ToolResult {
    ToolResult::error("PushNotification is not supported on this platform.")
}

#[cfg(target_os = "macos")]
fn applescript_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace(['\n', '\r'], " ");
    format!("\"{escaped}\"")
}

fn percent_encode_path_segment(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(char::from(*byte));
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn run_spotlight_search(
    query: &str,
    scope: &Path,
    names_only: bool,
    max_results: usize,
) -> ToolResult {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("/usr/bin/mdfind");
        if names_only {
            command.arg("-name");
        }
        let scope = scope.to_string_lossy().into_owned();
        command.args(["-onlyin", scope.as_str(), query]);
        match command.output() {
            Ok(output) if output.status.success() => {
                let raw = String::from_utf8_lossy(&output.stdout);
                let mut paths = raw
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>();
                let total = paths.len();
                paths.truncate(max_results);
                if paths.is_empty() {
                    return ToolResult::text(
                        "[SpotlightSearch] No results. The index may be building or the query may need adjustment.",
                    );
                }
                if total > max_results {
                    paths.push(format!(
                        "... ({} more results - narrow the query or increase max_results)",
                        total - max_results
                    ));
                }
                ToolResult::text(paths.join("\n"))
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                ToolResult::error(format!(
                    "[SpotlightSearch] mdfind error: {}",
                    if stderr.is_empty() {
                        format!("exit status {}", output.status)
                    } else {
                        stderr
                    }
                ))
            }
            Err(error) => {
                ToolResult::error(format!("[SpotlightSearch] failed to run mdfind: {error}"))
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (query, scope, names_only, max_results);
        ToolResult::error(
            "SpotlightSearch is only available on macOS because it uses mdfind. Use Glob or Grep for cross-platform file discovery.",
        )
    }
}

fn list_screen_windows() -> ToolResult {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("osascript");
        command.args([
            "-e",
            "tell application \"System Events\" to get the name of every process whose visible is true",
        ]);
        match command.output() {
            Ok(output) if output.status.success() => {
                let apps = String::from_utf8_lossy(&output.stdout);
                let lines = apps
                    .split(',')
                    .map(str::trim)
                    .filter(|app| !app.is_empty())
                    .map(|app| format!("app:{app}"))
                    .collect::<Vec<_>>();
                if lines.is_empty() {
                    ToolResult::text("Visible apps:\n(no visible apps found)")
                } else {
                    ToolResult::text(format!("Visible apps:\n{}", lines.join("\n")))
                }
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                ToolResult::error(format!(
                    "[ScreenCapture] window listing failed: {}",
                    if stderr.is_empty() {
                        format!("exit status {}", output.status)
                    } else {
                        stderr
                    }
                ))
            }
            Err(error) => {
                ToolResult::error(format!("[ScreenCapture] failed to run osascript: {error}"))
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        ToolResult::error(
            "ScreenCapture is only available on macOS in this Rust build. Attach a screenshot manually or use platform-native screenshot tooling.",
        )
    }
}

fn capture_screen_window(
    window_id: Option<u64>,
    app_name: Option<&str>,
    max_dimension: u32,
) -> ToolResult {
    #[cfg(target_os = "macos")]
    {
        let Some(window_id) = window_id else {
            let app = app_name.unwrap_or("the requested app").trim();
            return ToolResult::error(format!(
                "ScreenCapture.capture by app_name ({app}) requires native ScreenCaptureKit parity that is not implemented in this Rust fallback yet. Pass window_id to capture with screencapture."
            ));
        };
        let destination = std::env::temp_dir().join(format!(
            "deeptide-screen-{window_id}-{}.png",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0)
        ));
        let mut command = Command::new("/usr/sbin/screencapture");
        let window_arg = format!("-l{window_id}");
        let destination_arg = destination.to_string_lossy().into_owned();
        command.args(["-x", window_arg.as_str(), destination_arg.as_str()]);
        match command.output() {
            Ok(output) if output.status.success() && destination.exists() => {
                ToolResult::text(format!(
                    "[ScreenCapture.capture]\nfile_path: {}\nmax_dimension: {max_dimension}\nUse ImagePreprocess with this file_path if resizing, trimming, or text enhancement is needed.",
                    destination.display()
                ))
            }
            Ok(output) => {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
                ToolResult::error(format!(
                    "[ScreenCapture] capture failed: {}",
                    if stderr.is_empty() {
                        format!("exit status {}", output.status)
                    } else {
                        stderr
                    }
                ))
            }
            Err(error) => ToolResult::error(format!(
                "[ScreenCapture] failed to run screencapture: {error}"
            )),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (window_id, app_name, max_dimension);
        ToolResult::error(
            "ScreenCapture is only available on macOS in this Rust build. Attach a screenshot manually or use platform-native screenshot tooling.",
        )
    }
}

fn clipboard_read_text() -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("pbpaste");
        command_stdout(&mut command)
    }
    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("powershell");
        command.args(["-NoProfile", "-Command", "Get-Clipboard -Raw -Format Text"]);
        command_stdout(&mut command)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for args in [
            ("wl-paste", vec!["--no-newline"]),
            ("xclip", vec!["-selection", "clipboard", "-out"]),
            ("xsel", vec!["--clipboard", "--output"]),
        ] {
            let mut command = Command::new(args.0);
            command.args(args.1);
            if let Ok(output) = command_stdout(&mut command) {
                return Ok(output);
            }
        }
        Err(String::from(
            "Clipboard read is unavailable. Install wl-clipboard, xclip, or xsel.",
        ))
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        Err(String::from(
            "Clipboard read is unsupported on this platform.",
        ))
    }
}

fn clipboard_write_text(content: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        write_command_stdin("pbcopy", &[], content)
    }
    #[cfg(target_os = "windows")]
    {
        let mut child = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "Set-Clipboard -Value ([Console]::In.ReadToEnd())",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("Failed to start clipboard writer: {error}"))?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(content.as_bytes())
                .map_err(|error| format!("Failed to write clipboard content: {error}"))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|error| format!("Failed to wait for clipboard writer: {error}"))?;
        if output.status.success() {
            Ok(())
        } else {
            Err(format!(
                "Clipboard write failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        }
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for (program, args) in [
            ("wl-copy", Vec::<&str>::new()),
            ("xclip", vec!["-selection", "clipboard"]),
            ("xsel", vec!["--clipboard", "--input"]),
        ] {
            if write_command_stdin(program, &args, content).is_ok() {
                return Ok(());
            }
        }
        Err(String::from(
            "Clipboard write is unavailable. Install wl-clipboard, xclip, or xsel.",
        ))
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        let _ = content;
        Err(String::from(
            "Clipboard write is unsupported on this platform.",
        ))
    }
}

fn clipboard_file_paths() -> Result<Vec<String>, String> {
    #[cfg(target_os = "macos")]
    {
        let script = r#"
        use framework "AppKit"
        use scripting additions
        set pb to current application's NSPasteboard's generalPasteboard()
        set urls to pb's readObjectsForClasses:{current application's NSURL} options:{current application's NSPasteboardURLReadingFileURLsOnlyKey:true}
        if urls is missing value then return ""
        set out to ""
        repeat with u in urls
          set out to out & (u's |path|() as text) & linefeed
        end repeat
        return out
        "#;
        let mut command = Command::new("osascript");
        command.args(["-l", "AppleScript", "-e", script]);
        command_stdout(&mut command).map(|output| {
            output
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        Ok(Vec::new())
    }
}

fn finder_selection() -> String {
    #[cfg(target_os = "macos")]
    {
        let script = r#"
        tell application "Finder"
          set selectedItems to selection
          if selectedItems is {} then return ""
          set output to ""
          repeat with itemRef in selectedItems
            set output to output & POSIX path of (itemRef as alias) & linefeed
          end repeat
          return output
        end tell
        "#;
        let mut command = Command::new("osascript");
        command.args(["-e", script]);
        match command_stdout(&mut command) {
            Ok(output) if output.trim().is_empty() => {
                String::from("[Finder has no selected files]")
            }
            Ok(output) => output.trim().to_owned(),
            Err(message) => format!(
                "[Finder selection unavailable: {message}]\nGrant Automation permission to your terminal app if macOS prompts."
            ),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        String::from("[Finder selection is only available on macOS]")
    }
}

fn command_stdout(command: &mut Command) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("Failed to run clipboard command: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!(
            "Clipboard command failed: {}",
            stderr.trim().if_empty("unknown error")
        ))
    }
}

fn write_command_stdin(program: &str, args: &[&str], content: &str) -> Result<(), String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start clipboard writer: {error}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(content.as_bytes())
            .map_err(|error| format!("Failed to write clipboard content: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("Failed to wait for clipboard writer: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Clipboard write failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

trait EmptyFallback {
    fn if_empty(&self, fallback: &'static str) -> &str;
}

impl EmptyFallback for str {
    fn if_empty(&self, fallback: &'static str) -> &str {
        if self.is_empty() { fallback } else { self }
    }
}

fn builtin_tool_search_entries() -> Vec<ToolSearchEntry> {
    ToolRegistry::with_builtin_tools()
        .tools
        .values()
        .map(|tool| ToolSearchEntry {
            name: tool.name(),
            summary: tool.description().to_owned(),
            is_read_only: tool.is_read_only(),
            is_concurrency_safe: tool.is_read_only(),
            keywords: tool_search_keywords(tool.name()),
        })
        .collect()
}

fn search_tool_entries(query: &str, entries: &[ToolSearchEntry]) -> Vec<ToolSearchEntry> {
    let lower = query.trim().to_ascii_lowercase();
    if let Some(exact) = entries
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(&lower))
    {
        return vec![exact.clone()];
    }

    let terms = lower.split_whitespace().collect::<Vec<_>>();
    let required = terms
        .iter()
        .filter_map(|term| term.strip_prefix('+').filter(|term| !term.is_empty()))
        .collect::<Vec<_>>();
    let optional = terms
        .iter()
        .copied()
        .filter(|term| !term.starts_with('+'))
        .collect::<Vec<_>>();
    let scoring_terms = if required.is_empty() {
        optional
    } else {
        required
            .iter()
            .copied()
            .chain(optional.iter().copied())
            .collect()
    };
    if scoring_terms.is_empty() {
        return Vec::new();
    }

    let mut scored = entries
        .iter()
        .filter_map(|entry| {
            let searchable = SearchableToolEntry::new(entry);
            if !required.iter().all(|term| searchable.matches(term)) {
                return None;
            }

            let mut score = 0usize;
            if searchable.name == lower {
                score += 1_000;
            }
            if searchable.name.starts_with(&lower) {
                score += 350;
            }
            if searchable.name.contains(&lower) {
                score += 180;
            }
            if searchable.full_name_parts == lower {
                score += 120;
            }

            for term in &scoring_terms {
                if searchable.name_parts.iter().any(|part| part == term) {
                    score += 90;
                }
                if searchable
                    .name_parts
                    .iter()
                    .any(|part| part.starts_with(term))
                {
                    score += 55;
                }
                if searchable.name_parts.iter().any(|part| part.contains(term)) {
                    score += 25;
                }
                if searchable
                    .keyword_parts
                    .iter()
                    .any(|keyword| keyword == term)
                {
                    score += 45;
                }
                if searchable.summary_words.iter().any(|word| word == term) {
                    score += 18;
                }
                if searchable.summary.contains(term) {
                    score += 8;
                }
            }

            (score > 0).then_some((entry.clone(), score))
        })
        .collect::<Vec<_>>();

    scored.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.name.cmp(right.0.name))
    });
    scored.into_iter().map(|(entry, _)| entry).collect()
}

fn render_tool_search_entry(entry: ToolSearchEntry) -> String {
    let mut traits = Vec::new();
    traits.push(if entry.is_read_only {
        "read-only"
    } else {
        "writes"
    });
    if entry.is_concurrency_safe {
        traits.push("parallel");
    }
    format!(
        "- {} [{}] - {}",
        entry.name,
        traits.join(", "),
        truncate_chars(&entry.summary, 160)
    )
}

fn render_selected_tools(query: &str, entries: &[ToolSearchEntry]) -> String {
    let selected = query
        .get("select:".len()..)
        .unwrap_or_default()
        .split(|character: char| character == ',' || character.is_whitespace())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return String::from("No tools selected. Use `select:Read,Edit,Grep`.");
    }

    selected
        .into_iter()
        .map(|name| {
            entries
                .iter()
                .find(|entry| entry.name.eq_ignore_ascii_case(name))
                .cloned()
                .map(render_tool_search_entry)
                .unwrap_or_else(|| format!("- {name} - not found"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchableToolEntry {
    name: String,
    name_parts: Vec<String>,
    full_name_parts: String,
    summary: String,
    summary_words: Vec<String>,
    keyword_parts: Vec<String>,
}

impl SearchableToolEntry {
    fn new(entry: &ToolSearchEntry) -> Self {
        let name_parts = split_search_words(entry.name);
        let summary = entry.summary.to_ascii_lowercase();
        Self {
            name: entry.name.to_ascii_lowercase(),
            full_name_parts: name_parts.join(" "),
            name_parts,
            summary_words: split_search_words(&entry.summary),
            keyword_parts: entry
                .keywords
                .iter()
                .flat_map(|keyword| split_search_words(keyword))
                .collect(),
            summary,
        }
    }

    fn matches(&self, term: &str) -> bool {
        self.name.contains(term)
            || self.name_parts.iter().any(|part| part == term)
            || self.name_parts.iter().any(|part| part.contains(term))
            || self.keyword_parts.iter().any(|keyword| keyword == term)
            || self.summary_words.iter().any(|word| word == term)
            || self.summary.contains(term)
    }
}

fn split_search_words(value: &str) -> Vec<String> {
    let mut spaced = String::with_capacity(value.len());
    let mut previous: Option<char> = None;
    for character in value.chars() {
        if matches!(character, '_' | '-') {
            spaced.push(' ');
        } else {
            if let Some(prev) = previous
                && (prev.is_ascii_lowercase() || prev.is_ascii_digit())
                && character.is_ascii_uppercase()
            {
                spaced.push(' ');
            }
            spaced.push(character);
        }
        previous = Some(character);
    }

    spaced
        .to_ascii_lowercase()
        .split(|character: char| !character.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn tool_search_keywords(name: &str) -> Vec<&'static str> {
    match name {
        "Bash" => vec![
            "shell",
            "command",
            "terminal",
            "build",
            "test",
            "git",
            "package manager",
        ],
        "Monitor" => vec![
            "long running",
            "logs",
            "watch",
            "server",
            "until",
            "tail",
            "process",
        ],
        "AskUserQuestion" => vec![
            "ask user", "clarify", "question", "choice", "decision", "blocked",
        ],
        "MemorySearch" => vec![
            "memory",
            "remember",
            "recall",
            "project memory",
            "global memory",
            "preference",
        ],
        "MemoryWrite" => vec![
            "memory",
            "remember",
            "save memory",
            "preference",
            "project memory",
            "global memory",
        ],
        "Brief" => vec!["context", "summary", "compact", "compaction", "tokens"],
        "CtxInspect" => vec!["context", "tokens", "window", "cache", "budget"],
        "Snip" => vec!["context", "trim", "history", "messages", "tokens"],
        "EnterPlanMode" => vec!["plan", "planning", "explore", "approval"],
        "ExitPlanMode" => vec!["plan", "approval", "implementation", "permissions"],
        "Clipboard" => vec![
            "clipboard",
            "pasteboard",
            "copy",
            "paste",
            "finder",
            "selection",
        ],
        "SpotlightSearch" => vec![
            "spotlight",
            "mdfind",
            "metadata",
            "file discovery",
            "macos search",
        ],
        "ScreenCapture" => vec![
            "screen",
            "screenshot",
            "window",
            "capture",
            "visible apps",
            "ui inspection",
        ],
        "LSP" => vec![
            "language server",
            "definition",
            "references",
            "hover",
            "symbols",
            "code intelligence",
        ],
        "ImagePreprocess" => vec![
            "image",
            "screenshot",
            "crop",
            "resize",
            "ocr",
            "visual",
            "blank",
        ],
        "Vision" => vec![
            "vision",
            "ocr",
            "layout",
            "classify",
            "image",
            "pdf",
            "screenshot",
            "text recognition",
        ],
        "CrashLog" => vec!["crash", "diagnostic reports", "ips", "hang", "panic"],
        "MacLog" => vec!["macos", "unified logging", "tcc", "permission", "logs"],
        "MacDiagnose" => vec!["macos", "diagnose", "doctor", "permission", "crash"],
        "CronCreate" | "CronList" | "CronDelete" => {
            vec!["cron", "schedule", "recurring", "reminder", "automation"]
        }
        "ReviewArtifact" => vec!["review", "human review", "flag", "artifact", "edited file"],
        "Skill" => vec![
            "skill", "prompt", "template", "workflow", "publish", "commit",
        ],
        "Publish" => vec![
            "publish",
            "deploy",
            "clide.app",
            "static site",
            "dry run",
            "unpublish",
        ],
        "RemoteTrigger" => vec!["webhook", "remote", "trigger", "post", "automation"],
        "PushNotification" => vec![
            "notification",
            "alert",
            "notify",
            "desktop",
            "afk",
            "reminder",
        ],
        "NotebookEdit" => vec!["notebook", "jupyter", "ipynb", "cell", "data science"],
        "EnterWorktree" | "ExitWorktree" => {
            vec!["git", "worktree", "parallel", "branch", "isolation"]
        }
        "VerifyPlanExecution" => vec!["verify", "plan", "git diff", "expected files"],
        "Sleep" => vec!["sleep", "wait", "delay", "pause", "timer"],
        "Edit" => vec!["replace", "patch", "modify", "string replacement"],
        "FileMetadata" => vec!["xattr", "quarantine", "binary", "mime", "file type"],
        "Glob" => vec!["find files", "discover", "pattern"],
        "Grep" => vec!["search", "regex", "ripgrep", "content search"],
        "Read" => vec!["open", "inspect", "file contents"],
        "ReadFiles" => vec!["multi read", "batch read", "several files"],
        "TaskCreate" | "TaskGet" | "TaskList" | "TaskOutput" | "TaskStop" | "TaskUpdate"
        | "TodoWrite" => vec!["todo", "task", "plan", "progress"],
        "ToolSearch" => vec!["tool", "capability", "search tools", "find tool", "select"],
        "WebFetch" => vec!["url", "http", "fetch", "page"],
        "WebSearch" => vec!["web", "internet", "search", "brave", "serper"],
        "Write" => vec!["create", "overwrite", "new file"],
        _ => Vec::new(),
    }
}

fn web_search_with_environment(
    input: serde_json::Value,
    env: &BTreeMap<String, String>,
) -> ToolResult {
    let Some(query) = input.get("query").and_then(serde_json::Value::as_str) else {
        return ToolResult::error("Missing query parameter");
    };
    if query.chars().count() < 2 {
        return ToolResult::error("query must be at least 2 characters");
    }
    if input.get("allowed_domains").is_some() && input.get("blocked_domains").is_some() {
        return ToolResult::error("Cannot specify both allowed_domains and blocked_domains");
    }

    let allowed_domains = extract_string_array(input.get("allowed_domains"));
    let blocked_domains = extract_string_array(input.get("blocked_domains"));

    if let Some(api_key) = env
        .get("BRAVE_SEARCH_API_KEY")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        && let Ok(results) = search_brave(query, api_key, &allowed_domains, &blocked_domains)
    {
        return ToolResult::text(results);
    }

    if let Some(api_key) = env
        .get("SERPER_API_KEY")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        && let Ok(results) = search_serper(query, api_key, &allowed_domains, &blocked_domains)
    {
        return ToolResult::text(results);
    }

    let encoded = encode_query_component(query);
    ToolResult::error(format!(
        "WebSearch requires an API key. Set one of:\n  export BRAVE_SEARCH_API_KEY=<key>   # https://search.brave.com (2000 free/month)\n  export SERPER_API_KEY=<key>          # https://serper.dev (Google results)\n\nAlternative: use WebFetch to retrieve search results directly:\n  - https://html.duckduckgo.com/html/?q={encoded}\n  - https://www.google.com/search?q={encoded}"
    ))
}

#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveWebResult>,
}

#[derive(Debug, Clone, Deserialize)]
struct BraveWebResult {
    title: String,
    url: String,
    description: Option<String>,
    age: Option<String>,
}

fn search_brave(
    query: &str,
    api_key: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<String, String> {
    let mut url =
        Url::parse("https://api.search.brave.com/res/v1/web/search").map_err(|e| e.to_string())?;
    url.query_pairs_mut()
        .append_pair("q", query)
        .append_pair("count", "10")
        .append_pair("text_decorations", "false");

    let client = web_search_client()?;
    let response = client
        .get(url)
        .header("X-Subscription-Token", api_key)
        .header(reqwest::header::ACCEPT, "application/json")
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Brave Search failed with HTTP {}",
            response.status()
        ));
    }

    let parsed = response
        .json::<BraveSearchResponse>()
        .map_err(|error| error.to_string())?;
    let results = parsed.web.map(|web| web.results).unwrap_or_default();
    let results = apply_domain_filters(
        results
            .into_iter()
            .map(|result| (result.url.clone(), result)),
        allowed_domains,
        blocked_domains,
    );

    Ok(format_brave_results(query, &results))
}

fn format_brave_results(query: &str, results: &[BraveWebResult]) -> String {
    if results.is_empty() {
        return format!("No results found for: \"{query}\"");
    }

    let mut lines = vec![format!(
        "Web search results for: \"{query}\" (Brave Search)"
    )];
    for (index, result) in results.iter().take(8).enumerate() {
        lines.push(String::new());
        lines.push(format!("{}. {}", index + 1, result.title));
        lines.push(format!("   URL: {}", result.url));
        if let Some(description) = result
            .description
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            lines.push(format!("   {description}"));
        }
        if let Some(age) = result.age.as_ref().filter(|value| !value.is_empty()) {
            lines.push(format!("   {age}"));
        }
    }
    lines.push(String::new());
    lines.push(String::from(
        "Use WebFetch on any URL above to read the full page content.",
    ));
    lines.join("\n")
}

#[derive(Debug, Deserialize)]
struct SerperResponse {
    organic: Option<Vec<SerperOrganic>>,
}

#[derive(Debug, Clone, Deserialize)]
struct SerperOrganic {
    title: String,
    link: String,
    snippet: Option<String>,
    date: Option<String>,
}

fn search_serper(
    query: &str,
    api_key: &str,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Result<String, String> {
    let client = web_search_client()?;
    let response = client
        .post("https://google.serper.dev/search")
        .header("X-API-KEY", api_key)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&serde_json::json!({"q": query, "num": 10}))
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!(
            "Serper search failed with HTTP {}",
            response.status()
        ));
    }

    let parsed = response
        .json::<SerperResponse>()
        .map_err(|error| error.to_string())?;
    let results = apply_domain_filters(
        parsed
            .organic
            .unwrap_or_default()
            .into_iter()
            .map(|result| (result.link.clone(), result)),
        allowed_domains,
        blocked_domains,
    );

    Ok(format_serper_results(query, &results))
}

fn format_serper_results(query: &str, results: &[SerperOrganic]) -> String {
    if results.is_empty() {
        return format!("No results found for: \"{query}\"");
    }

    let mut lines = vec![format!(
        "Web search results for: \"{query}\" (Google via Serper)"
    )];
    for (index, result) in results.iter().take(8).enumerate() {
        lines.push(String::new());
        lines.push(format!("{}. {}", index + 1, result.title));
        lines.push(format!("   URL: {}", result.link));
        if let Some(snippet) = result.snippet.as_ref().filter(|value| !value.is_empty()) {
            lines.push(format!("   {snippet}"));
        }
        if let Some(date) = result.date.as_ref().filter(|value| !value.is_empty()) {
            lines.push(format!("   {date}"));
        }
    }
    lines.push(String::new());
    lines.push(String::from(
        "Use WebFetch on any URL above to read the full page content.",
    ));
    lines.join("\n")
}

fn web_search_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| error.to_string())
}

fn apply_domain_filters<T>(
    items: impl IntoIterator<Item = (String, T)>,
    allowed_domains: &[String],
    blocked_domains: &[String],
) -> Vec<T> {
    items
        .into_iter()
        .filter(|(url, _)| domain_allowed(url, allowed_domains, blocked_domains))
        .take(8)
        .map(|(_, value)| value)
        .collect()
}

fn domain_allowed(url: &str, allowed_domains: &[String], blocked_domains: &[String]) -> bool {
    let Some(host) = Url::parse(url)
        .ok()
        .and_then(|url| url.host_str().map(ToOwned::to_owned))
    else {
        return true;
    };
    if !allowed_domains.is_empty() {
        return allowed_domains
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
    }
    if !blocked_domains.is_empty() {
        return !blocked_domains
            .iter()
            .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
    }
    true
}

fn extract_string_array(value: Option<&serde_json::Value>) -> Vec<String> {
    value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn encode_query_component(query: &str) -> String {
    let Ok(mut url) = Url::parse("https://example.invalid/") else {
        return query.to_owned();
    };
    url.query_pairs_mut().append_pair("q", query);
    url.query()
        .and_then(|query| query.strip_prefix("q="))
        .unwrap_or(query)
        .to_owned()
}

fn fetch_web_content(url: &Url) -> ToolResult {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 \
             (KHTML, like Gecko) Deeptide/1.0 Safari/537.36",
        )
        .build()
    {
        Ok(client) => client,
        Err(error) => return ToolResult::error(format!("Failed to create HTTP client: {error}")),
    };

    let response = match client
        .get(url.clone())
        .header(
            reqwest::header::ACCEPT,
            "text/html,application/xhtml+xml,application/xml;q=0.9,text/plain;q=0.8,*/*;q=0.5",
        )
        .header(reqwest::header::ACCEPT_LANGUAGE, "en-US,en;q=0.9")
        .send()
    {
        Ok(response) => response,
        Err(error) => return ToolResult::error(format!("Failed to fetch URL: {error}")),
    };

    let status = response.status();
    let final_url = response.url().clone();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_owned();
    let selected_headers = selected_response_headers(&response);
    let bytes = match response.bytes() {
        Ok(bytes) => bytes,
        Err(error) => return ToolResult::error(format!("Failed to read response body: {error}")),
    };
    let size = bytes.len();

    let body = decode_response_body(&bytes);
    let is_http_error = !status.is_success();
    let mut output = response_header(
        status.as_u16(),
        size,
        url.as_str(),
        final_url.as_str(),
        &content_type,
        &selected_headers,
    );

    let Some(body) = body else {
        output.push_str("\n\nCould not decode response body (binary content).");
        return if is_http_error {
            ToolResult::error(output)
        } else {
            ToolResult::text(output)
        };
    };

    if is_http_error {
        output.push_str("\n\n--- HTTP diagnostics ---\n");
        output.push_str(&http_diagnostic(
            status.as_u16(),
            &selected_headers,
            final_url.as_str(),
            url.as_str(),
        ));
    }

    let lower_content_type = content_type.to_lowercase();
    let is_html_like = lower_content_type.contains("html") || body.to_lowercase().contains("<html");
    let mut text = if is_html_like {
        html_to_text(&body, Some(&final_url))
    } else {
        decode_entities(&body)
    };
    text = text.trim().to_owned();
    let total_chars = text.chars().count();
    if total_chars > 50_000 {
        text = format!(
            "{}\n\n[Content truncated: {total_chars} total chars; fetch a narrower URL or linked resource if needed]",
            truncate_chars(&text, 50_000)
        );
    }

    output.push_str("\n\n");
    output.push_str(&text);

    if is_http_error {
        ToolResult::error(output)
    } else {
        ToolResult::text(output)
    }
}

fn selected_response_headers(response: &reqwest::blocking::Response) -> Vec<(String, String)> {
    ["Location", "Retry-After", "WWW-Authenticate", "Server"]
        .into_iter()
        .filter_map(|name| {
            response
                .headers()
                .get(name)
                .and_then(|value| value.to_str().ok())
                .filter(|value| !value.is_empty())
                .map(|value| (name.to_owned(), value.to_owned()))
        })
        .collect()
}

fn decode_response_body(bytes: &[u8]) -> Option<String> {
    String::from_utf8(bytes.to_vec()).ok().or_else(|| {
        if bytes.contains(&0) {
            None
        } else {
            Some(bytes.iter().map(|byte| char::from(*byte)).collect())
        }
    })
}

fn response_header(
    code: u16,
    size: usize,
    requested_url: &str,
    final_url: &str,
    content_type: &str,
    selected_headers: &[(String, String)],
) -> String {
    let mut lines = vec![
        format!("HTTP {code} | {}", format_byte_count(size)),
        format!("URL: {requested_url}"),
    ];
    if final_url != requested_url {
        lines.push(format!("Final URL: {final_url}"));
    }
    lines.push(format!("Content-Type: {content_type}"));
    lines.extend(
        selected_headers
            .iter()
            .map(|(name, value)| format!("{name}: {value}")),
    );
    lines.join("\n")
}

fn http_diagnostic(
    code: u16,
    selected_headers: &[(String, String)],
    final_url: &str,
    requested_url: &str,
) -> String {
    let mut lines = Vec::new();
    if final_url != requested_url {
        lines.push(String::from(
            "Request followed redirects from the requested URL; verify the final URL when diagnosing.",
        ));
    }

    match code {
        400 => lines.push(String::from("400 Bad Request: inspect query encoding, required parameters, and whether the endpoint expects a different method or content type.")),
        401 => lines.push(String::from("401 Unauthorized: authentication is missing, expired, or not accepted by this endpoint.")),
        403 => lines.push(String::from("403 Forbidden: access may require login, a token, different permissions, or the site may be blocking automated clients.")),
        404 => lines.push(String::from("404 Not Found: verify the path, slug, version, and redirects; the host is reachable but this resource was not served.")),
        408 => lines.push(String::from("408 Request Timeout: retry later and check whether the server expects a smaller or more specific request.")),
        409 => lines.push(String::from("409 Conflict: the request reached the endpoint but conflicts with current resource state.")),
        410 => lines.push(String::from("410 Gone: the resource has been intentionally removed or archived.")),
        429 => {
            let retry = selected_headers
                .iter()
                .find(|(name, _)| name == "Retry-After")
                .map(|(_, value)| format!(" Retry-After: {value}."))
                .unwrap_or_default();
            lines.push(format!("429 Rate Limited: wait before retrying, reduce request frequency, or use an authenticated API.{retry}"));
        }
        500..=599 => lines.push(format!("{code} Server Error: the upstream service failed; capture the body, final URL, and relevant headers before retrying.")),
        _ => lines.push(format!("{code} HTTP status: use the response body plus headers above as the primary evidence.")),
    }

    lines.join("\n")
}

fn html_to_text(html: &str, base_url: Option<&Url>) -> String {
    let mut text = regex_replace(html, r"(?is)<!--.*?-->", "");
    text = regex_replace(&text, r"(?is)<script\b[^>]*>.*?</script>", "");
    text = regex_replace(&text, r"(?is)<style\b[^>]*>.*?</style>", "");
    text = replace_anchor_tags(&text, base_url);
    text = regex_replace(&text, r"(?i)<br\s*/?>", "\n");
    text = regex_replace(
        &text,
        r"(?i)</(p|div|section|article|header|footer|main|nav|tr|table|ul|ol|h[1-6])\s*>",
        "\n",
    );
    text = regex_replace(&text, r"(?i)<li\b[^>]*>", "\n- ");
    text = regex_replace(&text, r"(?i)</(td|th)\s*>", "\t");
    text = regex_replace(&text, r"<[^>]+>", "");

    let decoded = decode_entities(&text);
    let decoded = regex_replace(&decoded, r"[ \t\u{00A0}]+", " ");
    let decoded = regex_replace(&decoded, r" *\n *", "\n");
    regex_replace(&decoded, r"\n{3,}", "\n\n")
}

fn replace_anchor_tags(html: &str, base_url: Option<&Url>) -> String {
    let Ok(regex) = regex::Regex::new(r#"(?is)<a\b([^>]*)>(.*?)</a>"#) else {
        return html.to_owned();
    };
    regex
        .replace_all(html, |captures: &regex::Captures<'_>| {
            let attrs = captures.get(1).map(|value| value.as_str()).unwrap_or("");
            let inner = captures.get(2).map(|value| value.as_str()).unwrap_or("");
            let label = regex_replace(inner, r"<[^>]+>", "");
            if let Some(href) = extract_href(attrs).filter(|href| !href.is_empty()) {
                let resolved = base_url
                    .and_then(|base| base.join(&href).ok())
                    .map(|url| url.to_string())
                    .unwrap_or(href);
                format!("{label} [{resolved}]")
            } else {
                label
            }
        })
        .into_owned()
}

fn extract_href(attrs: &str) -> Option<String> {
    let regex = regex::Regex::new(r#"(?is)\bhref\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))"#).ok()?;
    let captures = regex.captures(attrs)?;
    (1..=3)
        .find_map(|group| captures.get(group).map(|value| value.as_str()))
        .map(decode_entities)
}

fn decode_entities(text: &str) -> String {
    let mut output = String::new();
    let mut chars = text.char_indices().peekable();
    while let Some((index, ch)) = chars.next() {
        if ch != '&' {
            output.push(ch);
            continue;
        }

        let Some((semicolon_index, _)) = text[index..]
            .char_indices()
            .take_while(|(_, candidate)| *candidate != '\n')
            .find(|(_, candidate)| *candidate == ';')
        else {
            output.push(ch);
            continue;
        };
        if semicolon_index > 12 {
            output.push(ch);
            continue;
        }

        let entity = &text[index + 1..index + semicolon_index];
        if let Some(replacement) = decode_entity(entity) {
            output.push(replacement);
            while chars
                .peek()
                .is_some_and(|(next_index, _)| *next_index <= index + semicolon_index)
            {
                chars.next();
            }
        } else {
            output.push(ch);
        }
    }
    output
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" | "#39" => Some('\''),
        "nbsp" => Some(' '),
        value if value.starts_with("#x") || value.starts_with("#X") => {
            u32::from_str_radix(&value[2..], 16)
                .ok()
                .and_then(char::from_u32)
        }
        value if value.starts_with('#') => value[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
    }
}

fn regex_replace(input: &str, pattern: &str, replacement: &str) -> String {
    regex::Regex::new(pattern)
        .map(|regex| regex.replace_all(input, replacement).into_owned())
        .unwrap_or_else(|_| input.to_owned())
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
pub struct MonitorTool;

impl Tool for MonitorTool {
    fn name(&self) -> &'static str {
        "Monitor"
    }

    fn description(&self) -> &'static str {
        "Run a long command and return recent output after a timeout or regex match."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(command) = input.get("command").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("command is required");
        };
        if command.trim().is_empty() {
            return ToolResult::error("command is required");
        }

        let max_seconds = input
            .get("max_seconds")
            .or_else(|| input.get("timeout"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(30)
            .clamp(5, 300);
        let until = input
            .get("until")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let regex = match until {
            Some(pattern) => match regex::Regex::new(pattern) {
                Ok(regex) => Some(regex),
                Err(error) => return ToolResult::error(format!("Invalid until regex: {error}")),
            },
            None => None,
        };

        monitor_shell_command(
            command,
            context,
            Duration::from_secs(max_seconds),
            regex,
            until,
        )
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

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskCreateTool;

impl Tool for TaskCreateTool {
    fn name(&self) -> &'static str {
        "TaskCreate"
    }

    fn description(&self) -> &'static str {
        "Create one in-memory task with a subject and description."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(subject) = input.get("subject").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing subject or description");
        };
        let Some(description) = input.get("description").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing subject or description");
        };
        if subject.trim().is_empty() || description.trim().is_empty() {
            return ToolResult::error("Missing subject or description");
        }

        add_todo(TodoItem {
            content: subject.to_owned(),
            status: TodoStatus::Pending,
            active_form: Some(description.to_owned()),
        });

        ToolResult::text(format!("Task created: {subject}"))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskListTool;

impl Tool for TaskListTool {
    fn name(&self) -> &'static str {
        "TaskList"
    }

    fn description(&self) -> &'static str {
        "List the current in-memory todo tasks."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, _input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let todos = list_todos();
        if todos.is_empty() {
            return ToolResult::text("No tasks.");
        }

        let lines = todos
            .iter()
            .enumerate()
            .map(|(index, item)| format!("#{} {} {}", index + 1, item.status.icon(), item.content))
            .collect::<Vec<_>>()
            .join("\n");
        ToolResult::text(lines)
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskGetTool;

impl Tool for TaskGetTool {
    fn name(&self) -> &'static str {
        "TaskGet"
    }

    fn description(&self) -> &'static str {
        "Get full details for one in-memory todo task by ID."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(task_id) = input.get("taskId").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing taskId parameter");
        };
        let Some(task) = get_todo(task_id) else {
            return ToolResult::error(format!("Task not found: {task_id}"));
        };

        let mut lines = vec![
            format!("Task: {}", task.content),
            format!("ID: {task_id}"),
            format!("Status: {}", task.status.as_str()),
        ];
        if let Some(active_form) = task.active_form.filter(|value| !value.is_empty()) {
            lines.push(format!("Description: {active_form}"));
        }
        ToolResult::text(lines.join("\n"))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskUpdateTool;

impl Tool for TaskUpdateTool {
    fn name(&self) -> &'static str {
        "TaskUpdate"
    }

    fn description(&self) -> &'static str {
        "Update an in-memory todo task by ID."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(task_id) = input.get("taskId").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing taskId");
        };

        if input.get("status").and_then(serde_json::Value::as_str) == Some("deleted") {
            return if delete_todo(task_id) {
                ToolResult::text(format!("Task #{task_id} deleted"))
            } else {
                ToolResult::error(format!("Task #{task_id} not found"))
            };
        }

        let status = input
            .get("status")
            .and_then(serde_json::Value::as_str)
            .map(TodoStatus::parse);
        let subject = input
            .get("subject")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);
        let description = input
            .get("description")
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned);

        let changes = update_todo(task_id, status, subject, description);
        if changes.is_empty() {
            return ToolResult::text(format!("Task #{task_id}: no changes (task may not exist)"));
        }

        ToolResult::text(format!("Task #{task_id} updated: {}", changes.join(", ")))
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskStopTool;

impl Tool for TaskStopTool {
    fn name(&self) -> &'static str {
        "TaskStop"
    }

    fn description(&self) -> &'static str {
        "Stop a task by marking it completed."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(task_id) = input.get("taskId").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("Missing taskId parameter");
        };
        let explanation = input
            .get("explanation")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty());

        if complete_todo(task_id) {
            if let Some(explanation) = explanation {
                ToolResult::text(format!("Task stopped: {explanation}"))
            } else {
                ToolResult::text(format!("Task {task_id} stopped"))
            }
        } else {
            ToolResult::error(format!("Task not found or already completed: {task_id}"))
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct TaskOutputTool;

impl Tool for TaskOutputTool {
    fn name(&self) -> &'static str {
        "TaskOutput"
    }

    fn description(&self) -> &'static str {
        "Retrieve recorded metadata and output for one task."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(task_id) = input.get("task_id").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("task_id is required");
        };
        if task_id.trim().is_empty() {
            return ToolResult::error("task_id is required");
        }

        let Some(task) = get_todo(task_id) else {
            return ToolResult::error(
                serde_json::json!({
                    "retrieval_status": "not_ready",
                    "task": null,
                    "note": format!("no task with id {task_id} (TaskStorage tracks the agent's todo list; background output capture is not yet implemented)")
                })
                .to_string(),
            );
        };

        ToolResult::text(
            serde_json::json!({
                "retrieval_status": "success",
                "task": {
                    "task_id": task_id,
                    "task_type": "todo",
                    "status": task.status.as_str(),
                    "description": task.active_form.unwrap_or_default(),
                    "output": task.content
                }
            })
            .to_string(),
        )
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

    fn icon(self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::InProgress => "◉",
            Self::Completed => "⌬",
            Self::Deleted => "✕",
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Deleted => "deleted",
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

fn add_todo(item: TodoItem) {
    if let Ok(mut todos) = todo_storage().lock() {
        todos.push(item);
    }
}

fn list_todos() -> Vec<TodoItem> {
    todo_storage()
        .lock()
        .map(|todos| todos.clone())
        .unwrap_or_default()
}

fn get_todo(task_id: &str) -> Option<TodoItem> {
    let index = task_id.parse::<usize>().ok()?.checked_sub(1)?;
    todo_storage()
        .lock()
        .ok()
        .and_then(|todos| todos.get(index).cloned())
}

fn complete_todo(task_id: &str) -> bool {
    let Some(index) = task_id
        .parse::<usize>()
        .ok()
        .and_then(|value| value.checked_sub(1))
    else {
        return false;
    };
    let Ok(mut todos) = todo_storage().lock() else {
        return false;
    };
    let Some(todo) = todos.get_mut(index) else {
        return false;
    };
    if todo.status == TodoStatus::Completed {
        return false;
    }
    todo.status = TodoStatus::Completed;
    true
}

fn update_todo(
    task_id: &str,
    status: Option<TodoStatus>,
    subject: Option<String>,
    description: Option<String>,
) -> Vec<String> {
    let Some(index) = task_id
        .parse::<usize>()
        .ok()
        .and_then(|value| value.checked_sub(1))
    else {
        return Vec::new();
    };

    let Ok(mut todos) = todo_storage().lock() else {
        return Vec::new();
    };
    let Some(todo) = todos.get_mut(index) else {
        return Vec::new();
    };

    let mut changes = Vec::new();
    if let Some(status) = status {
        todo.status = status;
        changes.push(format!("status -> {}", status.as_str()));
    }
    if let Some(subject) = subject {
        todo.content = subject;
        changes.push(String::from("subject updated"));
    }
    if let Some(description) = description {
        todo.active_form = Some(description);
        changes.push(String::from("description updated"));
    }
    changes
}

fn delete_todo(task_id: &str) -> bool {
    let Some(index) = task_id
        .parse::<usize>()
        .ok()
        .and_then(|value| value.checked_sub(1))
    else {
        return false;
    };
    let Ok(mut todos) = todo_storage().lock() else {
        return false;
    };
    if index >= todos.len() {
        return false;
    }
    todos.remove(index);
    true
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
    if let Some(kind) = special_file_kind(path) {
        return Err(unsupported_special_file_message(path, kind, metadata.len()));
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

fn render_file_metadata(path: &Path) -> String {
    let mut lines = vec![format!("[FileMetadata] {}", path.display())];
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            lines.push(String::from("exists: false"));
            return lines.join("\n");
        }
        Err(error) => {
            lines.push(String::from("exists: unknown"));
            lines.push(format!("error: {error}"));
            return lines.join("\n");
        }
    };

    lines.push(String::from("exists: true"));
    lines.push(format!(
        "kind: {}",
        if metadata.is_dir() {
            "directory"
        } else if metadata.is_file() {
            "file"
        } else {
            "other"
        }
    ));
    if metadata.is_file() {
        lines.push(format!(
            "size: {}",
            format_byte_count(metadata.len() as usize)
        ));
    }
    if let Ok(created) = metadata.created() {
        lines.push(format!("created: {}", format_system_time(created)));
    }
    if let Ok(modified) = metadata.modified() {
        lines.push(format!("modified: {}", format_system_time(modified)));
    }
    lines.push(format!("readonly: {}", metadata.permissions().readonly()));

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        lines.push(format!(
            "mode: {:o}",
            metadata.permissions().mode() & 0o7777
        ));
    }

    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    lines.push(format!(
        "extension: {}",
        if ext.is_empty() { "none" } else { ext.as_str() }
    ));
    let (type_name, mime) = file_type_hint(&ext, &metadata);
    lines.push(format!("type: {type_name}"));
    lines.push(format!("mime: {mime}"));

    #[cfg(target_os = "macos")]
    append_macos_metadata(path, &mut lines);
    #[cfg(not(target_os = "macos"))]
    {
        lines.push(String::from("xattrs: unsupported on this platform"));
        lines.push(String::from("spotlight: unavailable"));
    }

    lines.join("\n")
}

fn format_system_time(value: std::time::SystemTime) -> String {
    let datetime = time::OffsetDateTime::from(value);
    datetime
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| format!("{datetime}"))
}

fn file_type_hint(ext: &str, metadata: &fs::Metadata) -> (&'static str, &'static str) {
    if metadata.is_dir() {
        return ("directory", "inode/directory");
    }
    match ext {
        "txt" | "md" | "rs" | "swift" | "js" | "ts" | "tsx" | "jsx" | "py" | "go" | "java"
        | "c" | "h" | "cpp" | "hpp" | "json" | "toml" | "yaml" | "yml" | "csv" | "tsv" | "html"
        | "css" | "xml" | "sh" | "zsh" | "bash" => ("text", "text/plain"),
        "png" => ("image", "image/png"),
        "jpg" | "jpeg" => ("image", "image/jpeg"),
        "gif" => ("image", "image/gif"),
        "webp" => ("image", "image/webp"),
        "bmp" => ("image", "image/bmp"),
        "tif" | "tiff" => ("image", "image/tiff"),
        "pdf" => ("PDF document", "application/pdf"),
        "zip" => ("archive", "application/zip"),
        "gz" | "tgz" => ("archive", "application/gzip"),
        "tar" => ("archive", "application/x-tar"),
        "doc" | "docx" | "rtf" | "odt" => ("office or rich document", "application/octet-stream"),
        "ppt" | "pptx" | "xls" | "xlsx" => ("office document", "application/octet-stream"),
        "sqlite" | "sqlite3" | "db" => ("database", "application/octet-stream"),
        "" => ("unknown", "unknown"),
        _ => ("unknown", "application/octet-stream"),
    }
}

#[cfg(target_os = "macos")]
fn append_macos_metadata(path: &Path, lines: &mut Vec<String>) {
    let xattrs = Command::new("/usr/bin/xattr").arg("-l").arg(path).output();
    match xattrs {
        Ok(output) if output.status.success() && !output.stdout.is_empty() => {
            let text = String::from_utf8_lossy(&output.stdout);
            let names = text
                .lines()
                .filter_map(|line| line.split_once(": ").map(|(name, _)| name.to_owned()))
                .collect::<Vec<_>>();
            if names.is_empty() {
                lines.push(String::from("xattrs: present"));
            } else {
                lines.push(format!("xattrs: {}", names.join(", ")));
            }
            if let Some(quarantine) = text
                .lines()
                .find(|line| line.starts_with("com.apple.quarantine:"))
            {
                lines.push(format!(
                    "quarantine: {}",
                    quarantine
                        .strip_prefix("com.apple.quarantine:")
                        .unwrap_or("")
                        .trim()
                ));
            } else {
                lines.push(String::from("quarantine: none"));
            }
        }
        _ => lines.push(String::from("xattrs: none")),
    }

    let spotlight = Command::new("/usr/bin/mdls")
        .args([
            "-name",
            "kMDItemKind",
            "-name",
            "kMDItemContentType",
            "-name",
            "kMDItemFSName",
        ])
        .arg(path)
        .output();
    match spotlight {
        Ok(output) if output.status.success() && !output.stdout.is_empty() => {
            lines.push(String::from("spotlight:"));
            for line in String::from_utf8_lossy(&output.stdout).lines() {
                let trimmed = line.trim();
                if !trimmed.is_empty() && !trimmed.ends_with("= (null)") {
                    lines.push(format!("  {trimmed}"));
                }
            }
        }
        _ => lines.push(String::from("spotlight: unavailable")),
    }
}

fn special_file_kind(path: &Path) -> Option<&'static str> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tiff" | "tif" | "heic" | "avif" => {
            Some("image")
        }
        "pdf" => Some("PDF document"),
        "doc" | "docx" | "rtf" | "rtfd" | "odt" | "ppt" | "pptx" | "xls" | "xlsx" | "ods"
        | "odp" | "webarchive" => Some("office or rich document"),
        "zip" | "tar" | "gz" | "tgz" | "bz2" | "xz" | "7z" | "rar" | "dmg" | "pkg" => {
            Some("archive or package")
        }
        "mp3" | "wav" | "m4a" | "flac" | "mp4" | "mov" | "avi" | "mkv" | "webm" => {
            Some("media file")
        }
        "sqlite" | "sqlite3" | "db" => Some("database file"),
        _ => None,
    }
}

fn unsupported_special_file_message(path: &Path, kind: &str, bytes: u64) -> String {
    let file = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<unknown>");
    format!(
        "reason: unsupported binary or package-like file\nfile: {file}\ntype: {kind} · size: {}\nlikely_cause: this path appears to be {kind}, and Read avoids dumping binary data into context.\nnext_action: use a dedicated tool for images, PDFs, Office files, archives, media, or app bundles; if this is actually source text, report the extension/filename so Read can classify it safely.",
        format_byte_count(bytes as usize)
    )
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

#[derive(Debug)]
enum MonitorEvent {
    Stdout(String),
    Stderr(String),
}

fn monitor_shell_command(
    command: &str,
    context: &ToolContext,
    timeout: Duration,
    until_regex: Option<regex::Regex>,
    until_pattern: Option<&str>,
) -> ToolResult {
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
        Err(error) => return ToolResult::error(format!("Failed to launch: {error}")),
    };

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let (sender, receiver) = std::sync::mpsc::channel();
    if let Some(stdout) = stdout {
        spawn_monitor_reader(stdout, sender.clone(), true);
    }
    if let Some(stderr) = stderr {
        spawn_monitor_reader(stderr, sender, false);
    }

    let started = Instant::now();
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut matched = false;
    let mut timed_out = false;
    let exit_code;

    loop {
        while let Ok(event) = receiver.try_recv() {
            match event {
                MonitorEvent::Stdout(line) => {
                    if until_regex
                        .as_ref()
                        .is_some_and(|regex| regex.is_match(&line))
                    {
                        matched = true;
                    }
                    stdout_lines.push(line);
                }
                MonitorEvent::Stderr(line) => stderr_lines.push(line),
            }
        }

        if matched {
            let _ = child.kill();
            exit_code = child.wait().ok().and_then(|status| status.code());
            break;
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                exit_code = status.code();
                break;
            }
            Ok(None) if started.elapsed() >= timeout => {
                timed_out = true;
                let _ = child.kill();
                exit_code = child.wait().ok().and_then(|status| status.code());
                break;
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => return ToolResult::error(format!("Failed to monitor command: {error}")),
        }
    }

    let drain_deadline = Instant::now() + Duration::from_millis(250);
    while Instant::now() < drain_deadline {
        match receiver.try_recv() {
            Ok(MonitorEvent::Stdout(line)) => stdout_lines.push(line),
            Ok(MonitorEvent::Stderr(line)) => stderr_lines.push(line),
            Err(std::sync::mpsc::TryRecvError::Empty) => thread::sleep(Duration::from_millis(10)),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
        }
    }

    render_monitor_output(
        &stdout_lines,
        &stderr_lines,
        exit_code,
        timed_out,
        matched,
        until_pattern,
        timeout,
    )
}

fn spawn_monitor_reader<R>(
    reader: R,
    sender: std::sync::mpsc::Sender<MonitorEvent>,
    is_stdout: bool,
) where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut reader = io::BufReader::new(reader);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => break,
                Ok(_) => {
                    let trimmed = line.trim_end_matches(['\r', '\n']).to_owned();
                    let event = if is_stdout {
                        MonitorEvent::Stdout(trimmed)
                    } else {
                        MonitorEvent::Stderr(trimmed)
                    };
                    if sender.send(event).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
}

fn render_monitor_output(
    stdout_lines: &[String],
    stderr_lines: &[String],
    exit_code: Option<i32>,
    timed_out: bool,
    matched: bool,
    until_pattern: Option<&str>,
    timeout: Duration,
) -> ToolResult {
    let header = if matched {
        format!(
            "(matched `{}` after {} lines)",
            until_pattern.unwrap_or_default(),
            stdout_lines.len()
        )
    } else if timed_out {
        format!(
            "(captured {} lines, timed out after {}s)",
            stdout_lines.len(),
            timeout.as_secs()
        )
    } else {
        format!(
            "(captured {} lines, exit={})",
            stdout_lines.len(),
            exit_code.unwrap_or(-1)
        )
    };

    let mut body = header;
    let stdout_tail = stdout_lines
        .iter()
        .rev()
        .take(200)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    if !stdout_tail.is_empty() {
        body.push('\n');
        body.push_str(&stdout_tail);
    }
    if !stderr_lines.is_empty() {
        let stderr_tail = stderr_lines
            .iter()
            .rev()
            .take(100)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
        body.push_str("\n--- stderr ---\n");
        body.push_str(&truncate_chars(&stderr_tail, 2_000));
    }

    if !matched && exit_code.unwrap_or(0) != 0 && !timed_out {
        ToolResult::error(body)
    } else {
        ToolResult::text(body)
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
    offset: usize,
) -> ToolResult {
    if path.is_file() {
        return grep_file(
            path.parent().unwrap_or(base),
            path,
            regex,
            output_mode,
            head_limit,
            offset,
        );
    }
    if !path.is_dir() {
        return ToolResult::error(format!("Path does not exist: {}", path.display()));
    }

    let collect_limit = collect_limit(head_limit, offset);
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
        matching_files.len().max(content_matches.len()) < collect_limit
    });

    render_grep_output(
        output_mode,
        matching_files,
        content_matches,
        head_limit,
        offset,
        base,
    )
}

fn grep_file(
    base: &Path,
    path: &Path,
    regex: &regex::Regex,
    output_mode: &str,
    head_limit: usize,
    offset: usize,
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
                offset,
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
    offset: usize,
    _base: &Path,
) -> ToolResult {
    match output_mode {
        "content" => {
            let (lines, truncated) = page_lines(content_matches, head_limit, offset);
            if lines.is_empty() {
                ToolResult::text("No matches found")
            } else if truncated {
                ToolResult::text(format!(
                    "{}\n\n[Results truncated: use offset to paginate]",
                    lines.join("\n")
                ))
            } else {
                ToolResult::text(lines.join("\n"))
            }
        }
        "count" => {
            let lines = matching_files
                .into_iter()
                .map(|(path, count)| format!("{path}:{count}"))
                .collect::<Vec<_>>();
            let (lines, truncated) = page_lines(lines, head_limit, offset);
            if truncated {
                ToolResult::text(format!(
                    "{}\n\n[Results truncated: use offset to paginate]",
                    lines.join("\n")
                ))
            } else {
                ToolResult::text(lines.join("\n"))
            }
        }
        _ => {
            let lines = matching_files.into_keys().collect::<Vec<_>>();
            let (paged, truncated) = page_lines(lines, head_limit, offset);
            let suffix = if truncated {
                "\n\n[Results truncated: use offset to paginate]"
            } else {
                ""
            };
            ToolResult::text(
                format!(
                    "Found {} file{}\n\n{}",
                    paged.len(),
                    if paged.len() == 1 { "" } else { "s" },
                    paged.join("\n")
                ) + suffix,
            )
        }
    }
}

fn collect_limit(head_limit: usize, offset: usize) -> usize {
    if head_limit == 0 {
        usize::MAX
    } else {
        offset.saturating_add(head_limit)
    }
}

fn page_lines(lines: Vec<String>, head_limit: usize, offset: usize) -> (Vec<String>, bool) {
    let start = offset.min(lines.len());
    let tail_len = lines.len().saturating_sub(start);
    if head_limit == 0 {
        (lines.into_iter().skip(start).collect(), false)
    } else {
        (
            lines.into_iter().skip(start).take(head_limit).collect(),
            tail_len > head_limit,
        )
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

#[cfg(test)]
mod image_preprocess_tests {
    use super::{analyse_image, parse_normalized_crop};
    use image::{DynamicImage, Rgba, RgbaImage};

    #[test]
    fn image_analysis_detects_blank_image() {
        let image =
            DynamicImage::ImageRgba8(RgbaImage::from_pixel(64, 64, Rgba([255, 255, 255, 255])));

        let analysis = analyse_image(&image, 64);

        assert!(analysis.is_likely_blank());
        assert!(analysis.content_box.is_none());
    }

    #[test]
    fn image_analysis_finds_content_box() {
        let mut image = RgbaImage::from_pixel(100, 80, Rgba([255, 255, 255, 255]));
        for y in 20..50 {
            for x in 30..70 {
                image.put_pixel(x, y, Rgba([0, 0, 0, 255]));
            }
        }
        let image = DynamicImage::ImageRgba8(image);

        let analysis = analyse_image(&image, 100);
        let box_rect = analysis.content_box.expect("content box");

        assert!(!analysis.is_likely_blank());
        assert!(box_rect.width > 0.35);
        assert!(box_rect.width < 0.5);
        assert!(box_rect.height > 0.30);
        assert!(box_rect.height < 0.45);
    }

    #[test]
    fn normalized_crop_clamps_to_image_bounds() {
        let crop = parse_normalized_crop(Some(&serde_json::json!({
            "x": 0.8,
            "y": -1.0,
            "width": 0.5,
            "height": 2.0
        })))
        .expect("crop");

        assert_eq!(crop.x, 0.8);
        assert_eq!(crop.y, 0.0);
        assert!((crop.width - 0.2).abs() < f64::EPSILON);
        assert_eq!(crop.height, 1.0);
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

#[cfg(test)]
mod cron_tests {
    use super::{
        cron_describe, cron_next_fire, cron_should_default_to_recurring, cron_validation_error,
    };
    use time::{Date, Month, OffsetDateTime, Time, UtcOffset};

    #[test]
    fn every_five_minutes_cron_finds_next_boundary() {
        let from = OffsetDateTime::new_in_offset(
            Date::from_calendar_date(2026, Month::May, 11).expect("date"),
            Time::from_hms(10, 1, 0).expect("time"),
            UtcOffset::UTC,
        );

        let next =
            OffsetDateTime::from(cron_next_fire("*/5 * * * *", from.into()).expect("next fire"));

        assert_eq!(next.hour(), 10);
        assert_eq!(next.minute(), 5);
    }

    #[test]
    fn cron_list_field_can_contain_ranges() {
        let from = OffsetDateTime::new_in_offset(
            Date::from_calendar_date(2026, Month::May, 11).expect("date"),
            Time::from_hms(10, 0, 0).expect("time"),
            UtcOffset::UTC,
        );

        let next = OffsetDateTime::from(
            cron_next_fire("0,15-20 * * * *", from.into()).expect("next fire"),
        );

        assert_eq!(next.hour(), 10);
        assert_eq!(next.minute(), 15);
    }

    #[test]
    fn recurring_inference_matches_swift_reference() {
        assert!(cron_should_default_to_recurring("*/5 * * * *"));
        assert!(cron_should_default_to_recurring("0 * * * *"));
        assert!(cron_should_default_to_recurring("0 9 * * *"));
        assert!(!cron_should_default_to_recurring("0 9 11 5 *"));
    }

    #[test]
    fn cron_validation_and_descriptions_match_reference_shapes() {
        let error = cron_validation_error("/5 * * *").expect("error");
        assert!(error.contains("5-field"));
        assert!(error.contains("*/5 * * * *"));

        assert_eq!(cron_describe("* * * * *"), "every minute");
        assert_eq!(cron_describe("*/5 * * * *"), "every 5 minutes");
        assert_eq!(cron_describe("0 * * * *"), "every hour");
        assert_eq!(cron_describe("30 9 * * 1-5"), "at 9:30 on weekdays");
    }
}

#[cfg(test)]
mod lsp_tests {
    use super::{
        LspLocation, format_lsp_document_symbols, format_lsp_hover, format_lsp_locations,
        lsp_file_uri, parse_lsp_locations, path_from_lsp_uri,
    };
    use std::path::Path;

    #[test]
    fn lsp_file_uris_escape_spaces_and_decode_for_display() {
        let uri = lsp_file_uri(Path::new("/repo/with space/main.rs"));

        assert_eq!(uri, "file:///repo/with%20space/main.rs");
        assert_eq!(path_from_lsp_uri(&uri), "/repo/with space/main.rs");
    }

    #[test]
    fn lsp_locations_parse_location_and_location_link_shapes() {
        let value = serde_json::json!([
            {
                "uri": "file:///repo/src/lib.rs",
                "range": {"start": {"line": 2, "character": 4}}
            },
            {
                "targetUri": "file:///repo/src/main.rs",
                "targetSelectionRange": {"start": {"line": 9, "character": 1}}
            }
        ]);

        let locations = parse_lsp_locations(Some(&value));

        assert_eq!(
            locations,
            vec![
                LspLocation {
                    uri: String::from("file:///repo/src/lib.rs"),
                    line: 2,
                    character: 4,
                },
                LspLocation {
                    uri: String::from("file:///repo/src/main.rs"),
                    line: 9,
                    character: 1,
                },
            ]
        );
    }

    #[test]
    fn lsp_locations_render_relative_editor_positions() {
        let rendered = format_lsp_locations(
            vec![LspLocation {
                uri: String::from("file:///repo/src/lib.rs"),
                line: 2,
                character: 4,
            }],
            "Definition",
            Path::new("/repo"),
        );

        assert_eq!(rendered, "Definitions (1):\n  src/lib.rs:3:5");
    }

    #[test]
    fn lsp_hover_extracts_plain_marked_string_and_arrays() {
        assert_eq!(
            format_lsp_hover(Some(&serde_json::json!({"contents": "let value: String"}))),
            Some(String::from("let value: String"))
        );
        assert_eq!(
            format_lsp_hover(Some(&serde_json::json!({
                "contents": [{"language": "rust", "value": "fn main()"}, "plain"]
            }))),
            Some(String::from("fn main()\nplain"))
        );
    }

    #[test]
    fn lsp_document_symbols_render_flat_symbol_lists() {
        let rendered = format_lsp_document_symbols(Some(&serde_json::json!([
            {
                "name": "run",
                "kind": 12,
                "location": {"range": {"start": {"line": 7}}}
            },
            {
                "name": "State",
                "kind": 23,
                "range": {"start": {"line": 1}}
            }
        ])));

        assert_eq!(rendered, "Function run (line 8)\nStruct State (line 2)");
    }
}

#[cfg(test)]
mod clipboard_tests {
    use super::ClipboardSnapshot;

    #[test]
    fn clipboard_snapshot_read_prefers_plain_text() {
        let snapshot = ClipboardSnapshot {
            type_names: vec![String::from("text/plain"), String::from("public.file-url")],
            text: Some(String::from("hello")),
            file_paths: vec![String::from("/tmp/a.txt")],
            has_html: false,
            has_rtf: false,
            image_size: None,
        };

        assert_eq!(snapshot.render_read(), "hello");
    }

    #[test]
    fn clipboard_snapshot_read_reports_file_urls() {
        let snapshot = ClipboardSnapshot {
            type_names: vec![String::from("public.file-url")],
            text: None,
            file_paths: vec![String::from("/tmp/a.txt"), String::from("/tmp/b.txt")],
            has_html: false,
            has_rtf: false,
            image_size: None,
        };

        assert_eq!(
            snapshot.render_read(),
            "[Clipboard file URLs]\n/tmp/a.txt\n/tmp/b.txt"
        );
    }

    #[test]
    fn clipboard_snapshot_inspect_reports_rich_content() {
        let snapshot = ClipboardSnapshot {
            type_names: vec![String::from("public.html"), String::from("public.rtf")],
            text: None,
            file_paths: Vec::new(),
            has_html: true,
            has_rtf: true,
            image_size: Some((120, 80)),
        };

        let rendered = snapshot.render_inspect();

        assert!(rendered.contains("types: public.html, public.rtf"));
        assert!(rendered.contains("html: true"));
        assert!(rendered.contains("rtf: true"));
        assert!(rendered.contains("image: 120x80"));
    }
}
