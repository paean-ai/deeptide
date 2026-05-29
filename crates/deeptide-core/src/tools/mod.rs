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

// Tool groups extracted into submodules for maintainability. Each submodule
// holds the tool structs + their `impl Tool` blocks and reaches shared
// infrastructure (the `Tool` trait, `ToolResult`/`ToolContext`, helper
// functions) in this parent module via `use super::*`.
mod mcp;
mod planning;
mod tasks;
pub use mcp::*;
pub use planning::*;
pub use tasks::*;
mod vision;
pub use vision::*;
mod cron;
pub use cron::*;
mod lsp;
pub use lsp::*;
mod publish;
pub use publish::*;

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
        registry.register(Box::<AgentTool>::default());
        registry.register(Box::<McpTool>::default());
        registry.register(Box::<ListMcpResourcesTool>::default());
        registry.register(Box::<ReadMcpResourceTool>::default());
        registry.register(Box::<ListMcpPromptsTool>::default());
        registry.register(Box::<GetMcpPromptTool>::default());
        registry.register(Box::<BriefTool>::default());
        registry.register(Box::<CtxInspectTool>::default());
        registry.register(Box::<SnipTool>::default());
        registry.register(Box::<EnterPlanModeTool>::default());
        registry.register(Box::<ExitPlanModeTool>::default());
        registry.register(Box::<LspTool>::default());
        registry.register(Box::<ClipboardTool>::default());
        registry.register(Box::<AudioTranscribeTool>::default());
        registry.register(Box::<VideoTranscribeTool>::default());
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
        registry.register(Box::<DiscoverSkillsTool>::default());
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
        registry.register(Box::<BashOutputTool>::default());
        registry.register(Box::<KillBashTool>::default());
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
            if let Some((server, tool_name)) = parse_dynamic_mcp_tool_name(name) {
                return call_dynamic_mcp_tool(server, tool_name, input, context);
            }
            return ToolResult::error(format!("Unknown tool: {name}"));
        };
        tool.call(input, context)
    }

    pub fn names(&self) -> Vec<&'static str> {
        self.tools.keys().copied().collect()
    }

    /// Each registered tool paired with its read-only flag, in name order.
    /// Used to render the system prompt's tool index.
    pub fn tool_index(&self) -> Vec<(&'static str, bool)> {
        self.tools
            .values()
            .map(|tool| (tool.name(), tool.is_read_only()))
            .collect()
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
        if !crate::sensitive_file::is_allowed(&path) {
            return ToolResult::error(crate::sensitive_file::denial_message(&path));
        }
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
            if !crate::sensitive_file::is_allowed(&path) {
                sections.push(format!(
                    "===== {file_path} =====\n[Error: {}]",
                    crate::sensitive_file::denial_message(&path)
                ));
                continue;
            }
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
        collect_files(&base, &base, &mut |relative, full_path| {
            // Don't reveal the existence/paths of sensitive files (e.g. `.env`,
            // `id_rsa`) unless they have been explicitly opened this session.
            if matcher.matches(relative) && crate::sensitive_file::is_allowed(full_path) {
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

        let case_insensitive = input.get("-i").and_then(serde_json::Value::as_bool) == Some(true);
        // Multiline mode lets `.` span newlines and `^`/`$` anchor at line
        // boundaries, so a pattern can match across lines (Swift's -U
        // --multiline-dotall).
        let multiline = input.get("multiline").and_then(serde_json::Value::as_bool) == Some(true);
        let regex = match regex::RegexBuilder::new(pattern)
            .case_insensitive(case_insensitive)
            .dot_matches_new_line(multiline)
            .multi_line(multiline)
            .build()
        {
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
            .unwrap_or("files_with_matches")
            .to_owned();
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

        // Context lines: -C/context set both sides; -B/-A set each side. A
        // present -C/context overrides -B/-A, matching the Swift GrepTool.
        let grep_u64 = |key: &str| input.get(key).and_then(serde_json::Value::as_u64);
        let context = grep_u64("-C").or_else(|| grep_u64("context"));
        let before = context
            .or_else(|| grep_u64("-B"))
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let after = context
            .or_else(|| grep_u64("-A"))
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let line_numbers = input
            .get("-n")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let type_extensions = match input.get("type").and_then(serde_json::Value::as_str) {
            Some(name) if !name.trim().is_empty() => match file_type_extensions(name.trim()) {
                Some(extensions) => Some(
                    extensions
                        .iter()
                        .map(|ext| (*ext).to_owned())
                        .collect::<Vec<_>>(),
                ),
                None => return ToolResult::error(format!("Unknown file type: {}", name.trim())),
            },
            _ => None,
        };

        let options = GrepOptions {
            output_mode,
            head_limit,
            offset,
            before,
            after,
            line_numbers,
            type_extensions,
            multiline,
        };

        grep_path(&base, &base, &regex, glob.as_ref(), &options)
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

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
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

        let lower_query = query.to_ascii_lowercase();
        if lower_query.starts_with("select:") {
            return ToolResult::text(render_selected_tools(query, &entries));
        }

        let matches = search_tool_entries(query, &entries);
        let dynamic_mcp = if lower_query.contains("mcp") {
            render_dynamic_mcp_tool_search_entries(&context.cwd)
        } else {
            Vec::new()
        };
        if matches.is_empty() && dynamic_mcp.is_empty() {
            return ToolResult::text(
                "No matching tools. Try a tool name, an action, or `select:<ToolName>`.",
            );
        }

        let mut lines = matches
            .into_iter()
            .take(max_results)
            .map(render_tool_search_entry)
            .collect::<Vec<_>>();
        lines.extend(dynamic_mcp);
        ToolResult::text(lines.join("\n"))
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
        let memory_type = match input.get("type").and_then(serde_json::Value::as_str) {
            Some(raw) => {
                let Some(memory_type) = parse_memory_type(raw) else {
                    return ToolResult::error("type must be user, feedback, project, or reference");
                };
                memory_type
            }
            None => match scope {
                MemoryScope::Project => MemoryType::Project,
                MemoryScope::Global => MemoryType::User,
            },
        };

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
pub struct AgentTool;

impl Tool for AgentTool {
    fn name(&self) -> &'static str {
        "Agent"
    }

    fn description(&self) -> &'static str {
        "Launch a specialized sub-agent for multi-step exploration or planning."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let invocation = match parse_agent_invocation(&input) {
            Ok(invocation) => invocation,
            Err(error) => return ToolResult::error(error),
        };
        let model = invocation
            .model
            .as_deref()
            .unwrap_or("(inherit parent model)");
        let tool_policy = invocation.definition.tool_policy_label();

        // This fallback only fires when something dispatches `Agent`
        // straight through the `ToolRegistry` without going through
        // `AgentLoop::execute_tool_call`, which intercepts `Agent` calls
        // and routes them through the registered subagent backend factory.
        // The CLI wires one up automatically; if you see this message,
        // either a custom embedder is calling the registry directly or a
        // test forgot to call `AgentLoop::with_subagent_backend_factory`.
        ToolResult::error(format!(
            "Agent tool reached the registry fallback path. The hosting AgentLoop did not\n\
             register a subagent backend factory; call `AgentLoop::with_subagent_backend_factory(...)`\n\
             before `run()`, or dispatch this tool call through `AgentLoop::execute_tool_call`\n\
             instead of `ToolRegistry::call` directly.\n\n\
             Requested: {}\n\
             Type: {}\n\
             Model: {model}\n\
             Max turns: {}\n\
             Tools: {tool_policy}\n\
             Read-only: {}\n\n\
             Prompt:\n{prompt}",
            invocation.description,
            invocation.definition.kind,
            invocation.definition.max_turns,
            invocation.definition.is_read_only,
            prompt = invocation.prompt
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentDefinition {
    pub(crate) kind: &'static str,
    pub(crate) max_turns: usize,
    pub(crate) is_read_only: bool,
    pub(crate) allowed_tools: Option<&'static [&'static str]>,
    pub(crate) disallowed_tools: &'static [&'static str],
    /// Role instructions sent as the sub-agent's system prompt, mirroring the
    /// Swift `AgentDefinition.systemPrompt`.
    pub(crate) system_prompt: &'static str,
}

impl AgentDefinition {
    fn find(kind: &str) -> Option<Self> {
        Self::all()
            .into_iter()
            .find(|definition| definition.kind == kind)
    }

    fn all() -> Vec<Self> {
        vec![
            Self {
                kind: "general-purpose",
                max_turns: 15,
                is_read_only: false,
                allowed_tools: None,
                disallowed_tools: &["Agent", "EnterPlanMode", "ExitPlanMode", "MemoryWrite"],
                system_prompt: "You are a general-purpose sub-agent. Your task is to complete the user's request efficiently and accurately. You have access to the full set of tools and should use them as needed.\n\nGuidelines:\n- Break down complex tasks into clear steps\n- Use the best tool for each step (Read, Grep, Glob, Bash, etc.)\n- Be thorough — leave nothing assumed or unchecked\n- Return a clear, complete answer when done\n- If the task requires writing code, do so carefully and completely\n\nWhen finished, provide a concise summary of what you found or accomplished.",
            },
            Self {
                kind: "Explore",
                max_turns: 10,
                is_read_only: true,
                allowed_tools: Some(&[
                    "Read",
                    "Grep",
                    "Glob",
                    "LSP",
                    "WebFetch",
                    "WebSearch",
                    "Skill",
                    "ToolSearch",
                    "MemorySearch",
                    "ListMcpResources",
                    "ReadMcpResource",
                    "ListMcpPrompts",
                    "GetMcpPrompt",
                    "ReviewArtifact",
                    "CtxInspect",
                    "SpotlightSearch",
                    "TaskOutput",
                ]),
                disallowed_tools: &["Agent", "EnterPlanMode", "ExitPlanMode"],
                system_prompt: "You are a codebase exploration specialist. Your CRITICAL role:\n- SEARCH, READ, and ANALYZE code — NEVER modify it\n- You are in READ-ONLY MODE — no edits, no writes, no Bash, no shell\n- Use Glob for file patterns, Grep for content search, Read for file contents\n- Be fast and thorough — find what's needed quickly\n\nWhen you find relevant code, report:\n- Exact file paths and line numbers\n- Function/class signatures\n- Key implementation details\n- How the code connects to the broader architecture\n\nYou CANNOT: write files, edit files, run terminal commands, or create new code.\nYou CAN: read files, search for patterns, use LSP for symbols, and fetch docs via WebFetch when needed. WebSearch is available only when configured.",
            },
            Self {
                kind: "Plan",
                max_turns: 15,
                is_read_only: true,
                allowed_tools: None,
                disallowed_tools: &[
                    "Agent",
                    "EnterPlanMode",
                    "ExitPlanMode",
                    "Write",
                    "Edit",
                    "MemoryWrite",
                    "TaskStop",
                ],
                system_prompt: "You are a software architect and planning specialist. Your role:\n- EXPLORE the codebase to understand existing architecture\n- DESIGN implementation approaches for the given task\n- IDENTIFY all files, components, and patterns involved\n- CONSIDER trade-offs between different approaches\n- OUTPUT a detailed, actionable implementation plan\n\nYour plan should include:\n1. Understanding: what exists now and how it works\n2. Approach: step-by-step implementation strategy\n3. Files: every file that needs to be created or modified\n4. Dependencies: what must be done before what\n5. Risks: potential issues and how to mitigate them\n\nYou CANNOT: write code, edit files, or make any changes.\nYou CAN: explore the codebase thoroughly using Read, Grep, Glob, and Bash (git log/show/diff only).",
            },
        ]
    }

    fn all_types() -> Vec<&'static str> {
        Self::all()
            .into_iter()
            .map(|definition| definition.kind)
            .collect()
    }

    fn tool_policy_label(&self) -> String {
        if let Some(allowed_tools) = self.allowed_tools {
            return allowed_tools.join(", ");
        }
        if self.disallowed_tools.is_empty() {
            String::from("all registered tools")
        } else {
            format!(
                "all registered tools except {}",
                self.disallowed_tools.join(", ")
            )
        }
    }

    /// Assemble the sub-agent's full system prompt: the role instructions plus
    /// the working context (cwd, model, turn budget). Mirrors Swift's
    /// `fullSystemPrompt` assembly in `AgentTool.runSubagent`.
    pub(crate) fn full_system_prompt(&self, cwd: &Path, model: &str) -> String {
        format!(
            "{}\n\nYou are working in: {}\nModel: {}\nMax turns for this sub-task: {}",
            self.system_prompt,
            cwd.display(),
            model,
            self.max_turns
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AgentInvocation {
    pub(crate) description: String,
    pub(crate) prompt: String,
    pub(crate) definition: AgentDefinition,
    pub(crate) model: Option<String>,
}

pub(crate) fn parse_agent_invocation(input: &serde_json::Value) -> Result<AgentInvocation, String> {
    let Some(description) = input.get("description").and_then(serde_json::Value::as_str) else {
        return Err(String::from("Missing description parameter"));
    };
    let Some(prompt) = input.get("prompt").and_then(serde_json::Value::as_str) else {
        return Err(String::from("Missing prompt parameter"));
    };
    let description = description.trim();
    let prompt = prompt.trim();
    if description.is_empty() {
        return Err(String::from("description must not be empty"));
    }
    if prompt.chars().count() < 2 {
        return Err(String::from("prompt must be at least 2 characters"));
    }

    let subagent_type = input
        .get("subagent_type")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("general-purpose");
    let Some(definition) = AgentDefinition::find(subagent_type) else {
        return Err(format!(
            "Unknown agent type: {subagent_type}. Available: {}",
            AgentDefinition::all_types().join(", ")
        ));
    };

    if input
        .get("run_in_background")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        && input.get("isolation").and_then(serde_json::Value::as_str) == Some("worktree")
    {
        return Err(String::from(
            "Cannot combine run_in_background with isolation worktree in this version",
        ));
    }

    if let Some(isolation) = input.get("isolation").and_then(serde_json::Value::as_str)
        && isolation != "worktree"
    {
        return Err(String::from("isolation must be worktree when provided"));
    }

    let model = input
        .get("model")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    Ok(AgentInvocation {
        description: description.to_owned(),
        prompt: prompt.to_owned(),
        definition,
        model,
    })
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ClipboardTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct AudioTranscribeTool;

#[derive(Debug, Default, Clone, Copy)]
pub struct VideoTranscribeTool;

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
            "inspect" => match clipboard_snapshot() {
                Ok(snapshot) => ToolResult::text(snapshot.render_inspect()),
                Err(message) => ToolResult::error(message),
            },
            "read" => match clipboard_snapshot() {
                Ok(snapshot) => ToolResult::text(snapshot.render_read()),
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
    fn from_text_and_files(text: String, file_paths: Vec<String>) -> Self {
        let has_text = !text.is_empty();
        let has_files = !file_paths.is_empty();
        let mut type_names = Vec::new();
        if has_files {
            type_names.push(String::from("public.file-url"));
        }
        if has_text {
            type_names.push(String::from("text/plain"));
        }
        Self {
            type_names,
            text: has_text.then_some(text),
            file_paths,
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

impl Tool for AudioTranscribeTool {
    fn name(&self) -> &'static str {
        "AudioTranscribe"
    }

    fn description(&self) -> &'static str {
        "Transcribe a local audio file with a configured local speech backend."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        let language_hint = input
            .get("language_hint")
            .and_then(serde_json::Value::as_str);
        transcribe_media(
            MediaKind::Audio,
            &context.resolve_path(file_path),
            language_hint,
            false,
        )
    }
}

impl Tool for VideoTranscribeTool {
    fn name(&self) -> &'static str {
        "VideoTranscribe"
    }

    fn description(&self) -> &'static str {
        "Extract and transcribe the audio track from a local video file."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, context: &ToolContext) -> ToolResult {
        let Some(file_path) = input.get("file_path").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("file_path is required");
        };
        let language_hint = input
            .get("language_hint")
            .and_then(serde_json::Value::as_str);
        let allow_server = input
            .get("allow_server")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
        transcribe_media(
            MediaKind::Video,
            &context.resolve_path(file_path),
            language_hint,
            allow_server,
        )
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

/// Read-only catalog of every built-in skill. Mirrors zero-cli's
/// `DiscoverSkillsTool` so the model can proactively learn what skill
/// templates are available before deciding to call `Skill`.
///
/// The previous behavior surfaced the catalog only as a side effect of
/// calling `Skill` with an unknown name, which wastes a tool call and
/// pollutes the transcript with an error. `DiscoverSkills` lets the model
/// list the surface up front and pick the right skill on the first try.
#[derive(Debug, Default, Clone, Copy)]
pub struct DiscoverSkillsTool;

impl Tool for DiscoverSkillsTool {
    fn name(&self) -> &'static str {
        "DiscoverSkills"
    }

    fn description(&self) -> &'static str {
        "List every built-in Deeptide skill (name, one-line description, optional when-to-use hint). \
         Use before invoking Skill to pick the right template; no side effects."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, _input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let skills_json: Vec<serde_json::Value> = builtin_skills()
            .iter()
            .map(|skill| {
                let mut obj = serde_json::Map::with_capacity(3);
                obj.insert(
                    String::from("name"),
                    serde_json::Value::String(skill.name.to_owned()),
                );
                obj.insert(
                    String::from("description"),
                    serde_json::Value::String(skill.description.to_owned()),
                );
                if let Some(when) = skill.when_to_use {
                    obj.insert(
                        String::from("when_to_use"),
                        serde_json::Value::String(when.to_owned()),
                    );
                }
                serde_json::Value::Object(obj)
            })
            .collect();
        let payload = serde_json::json!({
            "count": skills_json.len(),
            "skills": skills_json,
            "invocation_hint": "Invoke a skill with Skill({\"skill\": \"<name>\", \"args\": \"<optional arguments>\"}).",
        });
        ToolResult::text(payload.to_string())
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
    // Stable de-dup by path before ranking (path order keeps it deterministic).
    hits.sort_by(|left, right| {
        left.scope
            .cmp(right.scope)
            .then_with(|| left.path.cmp(&right.path))
    });
    hits.dedup_by(|left, right| left.path == right.path);

    if hits.is_empty() {
        return ToolResult::text("No matching Deeptide memory files found.");
    }

    // Rank by BM25 + a mild recency nudge. Newest file → recency 1.0, oldest →
    // 0.0; single-file corpora get 1.0. The ranker drops zero-overlap docs, so
    // irrelevant files never surface — same guarantee as the old substring
    // filter, but now ordered by relevance instead of path.
    let (min_mtime, max_mtime) = hits.iter().fold((f64::MAX, f64::MIN), |(lo, hi), h| {
        (lo.min(h.mtime_secs), hi.max(h.mtime_secs))
    });
    let span = (max_mtime - min_mtime).max(1.0);
    let docs: Vec<crate::memory_rank::RankDoc> = hits
        .iter()
        .map(|h| crate::memory_rank::RankDoc {
            id: h.path.display().to_string(),
            text: h.text.clone(),
            recency: if max_mtime <= min_mtime {
                1.0
            } else {
                (h.mtime_secs - min_mtime) / span
            },
        })
        .collect();

    let ranked = crate::memory_rank::rank(query, &docs, max_results);
    if ranked.is_empty() {
        return ToolResult::text("No matching Deeptide memory files found.");
    }

    ToolResult::text(
        ranked
            .into_iter()
            .map(|(idx, _score)| render_memory_hit(hits[idx].clone()))
            .collect::<Vec<_>>()
            .join("\n\n"),
    )
}

#[derive(Debug, Clone, PartialEq)]
struct MemorySearchHit {
    scope: &'static str,
    path: PathBuf,
    title: String,
    description: Option<String>,
    matching_line: Option<String>,
    /// Full searchable text (title + description + body), for BM25 ranking.
    text: String,
    /// Modification time in seconds since the epoch, for the recency nudge.
    mtime_secs: f64,
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
        // No substring pre-filter: collect every candidate and let BM25 ranking
        // (in search_memory) decide relevance and order. Zero-overlap docs are
        // dropped by the ranker, so irrelevant files still never surface.
        let mtime_secs = entry
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0);
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
        let text = format!(
            "{title}\n{}\n{content}",
            description.as_deref().unwrap_or("")
        );
        hits.push(MemorySearchHit {
            scope,
            path,
            title,
            description,
            matching_line,
            text,
            mtime_secs,
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

pub(crate) fn model_context_window(model: &str) -> u64 {
    let lower = model.to_ascii_lowercase();
    if lower.contains("deepseek-v4-flash-q4") {
        1_000_000
    } else if lower.contains("v4-flash-mlx-q4q8") || lower.contains("v4-flash-q4q8") {
        1_048_576
    } else if lower.contains("deepseek-v4-flash") {
        512_000
    } else if lower.contains("deepseek-v4") {
        1_000_000
    } else if lower.contains("qwen3-coder-next")
        || lower.contains("qwen3.6-35b-a3b")
        || lower.contains("qwen3.6")
    {
        262_144
    } else if lower.contains("glm-4.7-flash") {
        131_072
    } else if lower.contains("deepseek-v3") || lower.contains("deepseek-chat") {
        128_000
    } else if lower.contains("claude-3-opus")
        || lower.contains("claude-3-5")
        || lower.contains("claude-3.5")
    {
        200_000
    } else if lower.contains("gemini") {
        1_000_000
    } else {
        128_000
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MediaKind {
    Audio,
    Video,
}

impl MediaKind {
    fn label(self) -> &'static str {
        match self {
            Self::Audio => "AudioTranscribe",
            Self::Video => "VideoTranscribe",
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }

    fn supported_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Audio => &[
                "mp3", "wav", "m4a", "aiff", "aif", "caf", "flac", "ogg", "opus",
            ],
            Self::Video => &["mp4", "mov", "m4v", "mkv", "webm", "avi"],
        }
    }
}

fn transcribe_media(
    kind: MediaKind,
    path: &Path,
    language_hint: Option<&str>,
    allow_server: bool,
) -> ToolResult {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("media");
    if !path.exists() {
        return ToolResult::error(format!("File not found: {}", path.display()));
    }
    if !path.is_file() {
        return ToolResult::error(format!("Path is not a file: {}", path.display()));
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !kind.supported_extensions().contains(&extension.as_str()) {
        return ToolResult::error(format!(
            "Unsupported {} format: .{}\nSupported formats: {}",
            kind.noun(),
            if extension.is_empty() {
                "(none)"
            } else {
                extension.as_str()
            },
            kind.supported_extensions().join(", ")
        ));
    }
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) => {
            return ToolResult::error(format!("File is not readable: {file_name}: {error}"));
        }
    };

    let duration = media_duration(path);
    let mut lines = vec![
        format!("[{}] {file_name}", kind.label()),
        String::new(),
        String::from("Transcription backend unavailable in this Rust build."),
        String::new(),
        String::from("--- Metadata ---"),
        format!("File size: {}", format_bytes(metadata.len())),
    ];
    if let Some(duration) = duration {
        lines.push(format!("Duration: {}", format_duration_seconds(duration)));
    } else {
        lines.push(String::from("Duration: unavailable"));
    }
    if let Some(language_hint) = language_hint.filter(|value| !value.trim().is_empty()) {
        lines.push(format!("Language hint: {}", language_hint.trim()));
    }
    if kind == MediaKind::Video {
        lines.push(format!(
            "Recognition mode: {}",
            if allow_server {
                "local or server fallback allowed by input"
            } else {
                "local only"
            }
        ));
        lines.push(String::from("Visual frames: not analyzed"));
    }
    lines.push(String::new());
    lines.push(String::from(
        "Install or configure a local speech-to-text backend, then retry. This tool currently provides safe validation and metadata parity for Rust while avoiding cloud uploads by default.",
    ));
    ToolResult::error(lines.join("\n"))
}

fn media_duration(path: &Path) -> Option<f64> {
    let ffprobe = which_binary("ffprobe")?;
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
        ])
        .arg(path)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite() && *value >= 0.0)
}

fn format_duration_seconds(seconds: f64) -> String {
    let total = seconds.round() as u64;
    let minutes = total / 60;
    let seconds = total % 60;
    if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
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

fn optional_trimmed_string(input: &serde_json::Value, key: &str) -> Option<String> {
    let value = input.get(key)?.as_str()?.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn json_usize(input: &serde_json::Value, key: &str) -> Option<usize> {
    input
        .get(key)?
        .as_u64()
        .and_then(|value| value.try_into().ok())
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

static REVIEW_ARTIFACTS: OnceLock<Mutex<BTreeMap<PathBuf, String>>> = OnceLock::new();

fn review_artifacts() -> &'static Mutex<BTreeMap<PathBuf, String>> {
    REVIEW_ARTIFACTS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Format a `SystemTime` as `YYYY-MM-DD HH:MM:SS` (UTC). Shared by the cron
/// "next fire" display and the publish state timestamps.
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
            prompt: "Bootstrap project memory for Deeptide in this repository.\n\nPhase 1 - explore (don't write anything yet):\n  * `ls -la` for the top-level layout.\n  * Read `README*` if present, plus whichever of these exist: `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, `Package.swift`, `Gemfile`, `pubspec.yaml`, `mix.exs`, `pom.xml`, `build.gradle*`, `Makefile`, `justfile`.\n  * Skim CI config (`.github/workflows/*`, `.circleci/*`, `.gitlab-ci.yml`) to learn the test/lint/release commands the project actually uses.\n  * Briefly look at one or two of the largest source dirs to understand structure - don't read every file.\n\nPhase 2 - check for existing project guides:\n  * `TIDE.md`, `AGENTS.md`, `CLAUDE.md`, `ZERO.md`, `CURSOR.md`, `.cursorrules`. If one exists with substantive content, do NOT overwrite - instead summarize what's there and stop. The user can delete it and re-run `/init` if they want a regeneration.\n\nPhase 3 - write `TIDE.md` at the repo root with these sections (skip any that don't apply, keep total length 60-120 lines):\n  * One-paragraph project overview (what it does, who uses it).\n  * Stack: language(s), framework, package manager, runtime.\n  * Build / test / lint / release commands - concrete, copy-pasteable.\n  * Code-layout map: top-level dirs and their purpose.\n  * Conventions worth remembering (naming, style, file org, error handling, testing).\n  * Load-bearing or fragile areas - the \"don't do\" list.\n\nKeep it tight: concrete commands and short bullets, not prose.\n\nPhase 4 - recap to the user (terminal text, not the file):\n  * Path of the file you wrote.\n  * One-line summary of the stack you detected.\n  * Reminder: this is a starting point - review, edit, re-run `/init` to re-scan; drop additional facts into `~/.config/tide/projects/<slug>/memory/` for cross-session recall.\n\n$ARGUMENTS",
        },
        BuiltinSkill {
            name: "batch",
            description: "Plan and execute large parallelizable changes across the codebase",
            when_to_use: None,
            prompt: "Orchestrate a large, parallelizable change across this repository.\n\nPhase 1 - Scope (use the Plan sub-agent or EnterPlanMode if helpful):\n- Map everything the user's instruction touches: files, symbols, conventions.\n- Break work into independent units (prefer per-directory or per-module slices).\n\nPhase 2 - Parallel execution:\n- Launch multiple Agent(subagent_type: Explore) calls in one message for read-only reconnaissance.\n- For implementations that must not corrupt each other's working tree, use Agent with `isolation: \"worktree\"` so each worker has its own git worktree under `.deeptide-worktrees/`.\n- Keep the parent coordinator focused on merging decisions and consistency; workers handle their slice only.\n\nPhase 3 - Verify:\n- Run the project's tests or build (Makefile targets, package scripts, `cargo test`, `npm test`, etc.).\n- Fix failures before reporting done.\n\nPhase 4 - Report:\n- Summarize units completed, key files changed, and verification commands run.\n- Note any worktrees left dirty so the user can merge or remove them (`git worktree list`).\n\n$ARGUMENTS",
        },
        BuiltinSkill {
            name: "publish",
            description: "Publish or delete a static frontend on clide.app with Paean credentials",
            when_to_use: Some(
                "Use when the user asks to publish, deploy, upload, remove, unpublish, or delete a static frontend app/site to or from Paean Clide hosting / clide.app. Examples: \"publish this frontend\", \"deploy this app\", \"unpublish the remote site\", \"delete the remote publish\".",
            ),
            prompt: "Publish or delete the current project's static frontend via Paean Clide hosting.\n\nCall the native Publish tool for the operation.\nDo NOT run `/publish` through Bash; slash commands are REPL-only.\nDo NOT run `tide publish`; the agent has the native Publish tool.\nDo NOT use WebSearch to discover how Clide publishing works.\n\nPublish tool input:\n  * For generic requests like \"$ARGUMENTS\" or \"publish to clide.app\", call Publish with no arguments.\n  * Set `dir` only when the user names a specific directory with top-level `index.html`.\n  * Set `handle` only when the user explicitly asks for a custom handle.\n  * Set `random` only when the user explicitly asks for a new random handle.\n  * Set `delete` only when the user asks to delete, remove, or unpublish remote content.\n\nBehavior:\n  * Requires a valid Paean AI login token from `tide auth login`, even if another provider is active.\n  * Prefers built output directories over source: `dist`, `build`, `out`, `.output/public`, then `public`.\n  * If no publishable output exists and `package.json` has a build script, run the project's build command first.\n  * The published directory must contain a top-level `index.html`.\n  * `.clideignore` is created or updated at the project root with safety defaults before zipping.\n  * Secrets, `.env`, credentials, `.git`, `node_modules`, `.clide/`, logs, source maps, editor files, and OS junk are excluded.\n  * If `.clide/publish.json` has a previous handle, reuse it unless `handle` or `random` is passed.\n  * Save the successful handle back to `.clide/publish.json` so the next publish overwrites the same site.\n\nAfter running, report the URL, whether it overwrote an existing site, file count, and state file path.\nIf authentication fails, tell the user to run `tide auth login`.\nIf no top-level `index.html` exists, explain that built apps should publish the build output directory, not the source directory.\n\n$ARGUMENTS",
        },
        BuiltinSkill {
            name: "update-config",
            description: "Configure Deeptide CLI settings",
            when_to_use: None,
            prompt: "Help the user configure their Deeptide CLI settings.\n\nAvailable settings:\n- model: The DeepSeek model to use (e.g., deepseek-v4-flash, deepseek-v4-pro)\n- api_key: DeepSeek API key\n- max_turns: Maximum agentic turns per session (default: 25)\n- max_tokens: Maximum output tokens per request (default: 4096)\n- permission_mode: default, accept-edits, plan, bypass\n- thinking: enabled, auto, disabled\n- effort: low, medium, high\n- fast_mode: true/false\n\nUse `tide config --set <key>=<value>` for global settings, or `tide config --set <key>=<value> --project` for project-specific settings.\nUse `tide config --show` to see current configuration.\n\n$ARGUMENTS",
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct McpServerConfig {
    name: String,
    source: PathBuf,
    command: Option<String>,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    framing: McpFraming,
    url: Option<String>,
    /// Extra HTTP headers sent with every request to an HTTP/SSE MCP server
    /// (e.g. `Authorization`). Ignored for stdio servers.
    headers: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpFraming {
    ContentLength,
    NewlineJson,
}

fn render_mcp_list(kind: &str, target: Option<&str>, cwd: &Path) -> ToolResult {
    let servers = match discover_mcp_servers(cwd) {
        Ok(servers) => servers,
        Err(error) => return ToolResult::error(error),
    };
    let selected = servers
        .iter()
        .filter(|server| target.is_none_or(|target| server.name == target))
        .collect::<Vec<_>>();

    if selected.is_empty() {
        return if let Some(target) = target {
            ToolResult::error(format!("MCP server not configured: {target}"))
        } else {
            ToolResult::error(
                "No MCP servers configured. Add servers under mcp_servers in .deeptide/settings.json or mcpServers in .mcp.json.",
            )
        };
    }

    let mut lines = Vec::new();
    for server in selected {
        lines.push(format!("[{}]", server.name));
        lines.push(format!("  source: {}", server.source.display()));
        if let Some(command) = &server.command {
            lines.push(format!("  command: {command}"));
        } else if let Some(url) = &server.url {
            lines.push(format!("  url: {url}"));
        } else {
            lines.push(String::from("  transport: configured"));
            lines.push(format!(
                "  {kind}: unavailable because no command or url is configured"
            ));
            continue;
        }
        let method = match kind {
            "resources" => "resources/list",
            "prompts" => "prompts/list",
            _ => unreachable!("MCP list kind should be known"),
        };
        match call_mcp_server(server, method, serde_json::json!({})) {
            Ok(result) => lines.extend(render_mcp_collection(kind, &result)),
            Err(error) => lines.push(format!("  error: {error}")),
        }
    }

    ToolResult::text(lines.join("\n"))
}

fn call_configured_mcp_server(
    cwd: &Path,
    server: &str,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let servers = discover_mcp_servers(cwd)?;
    let Some(config) = servers.iter().find(|configured| configured.name == server) else {
        return Err(format!("MCP server not configured: {server}"));
    };
    call_mcp_server(config, method, params)
}

fn parse_dynamic_mcp_tool_name(name: &str) -> Option<(&str, &str)> {
    let rest = name.strip_prefix("mcp__")?;
    let (server, tool) = rest.split_once("__")?;
    (!server.is_empty() && !tool.is_empty()).then_some((server, tool))
}

fn call_dynamic_mcp_tool(
    server: &str,
    tool_name: &str,
    input: serde_json::Value,
    context: &ToolContext,
) -> ToolResult {
    let arguments = input
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| input.clone());
    let params = serde_json::json!({
        "name": tool_name,
        "arguments": arguments
    });
    match call_configured_mcp_server(&context.cwd, server, "tools/call", params) {
        Ok(result) => ToolResult::text(format_json_value(&result)),
        Err(error) => ToolResult::error(error),
    }
}

fn discover_mcp_servers(cwd: &Path) -> Result<Vec<McpServerConfig>, String> {
    let mut servers = Vec::new();
    for path in mcp_config_paths(cwd) {
        if !path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read MCP settings {}: {error}", path.display()))?;
        let json: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|error| format!("Failed to parse MCP settings {}: {error}", path.display()))?;
        extract_mcp_servers_from_json(&json, &path, &mut servers);
    }
    servers.sort_by(|left, right| left.name.cmp(&right.name));
    servers.dedup_by(|left, right| left.name == right.name);
    Ok(servers)
}

fn mcp_config_paths(cwd: &Path) -> Vec<PathBuf> {
    let mut paths = vec![
        cwd.join(".mcp.json"),
        cwd.join(".deeptide").join("mcp.json"),
        cwd.join(".deeptide").join("settings.json"),
        cwd.join(".deeptide").join("config.json"),
    ];
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
    paths
}

fn extract_mcp_servers_from_json(
    json: &serde_json::Value,
    source: &Path,
    servers: &mut Vec<McpServerConfig>,
) {
    for key in ["mcp_servers", "mcpServers", "servers"] {
        if let Some(object) = json.get(key).and_then(serde_json::Value::as_object) {
            collect_mcp_servers(object, source, servers);
        }
    }
    if let Some(settings) = json.get("settings") {
        for key in ["mcp_servers", "mcpServers"] {
            if let Some(object) = settings.get(key).and_then(serde_json::Value::as_object) {
                collect_mcp_servers(object, source, servers);
            }
        }
    }
}

fn collect_mcp_servers(
    object: &serde_json::Map<String, serde_json::Value>,
    source: &Path,
    servers: &mut Vec<McpServerConfig>,
) {
    for (name, value) in object {
        let Some(config) = value.as_object() else {
            continue;
        };
        if config.get("disabled").and_then(serde_json::Value::as_bool) == Some(true) {
            continue;
        }
        let command = config
            .get("command")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let url = config
            .get("url")
            .or_else(|| config.get("endpoint"))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let args = config
            .get("args")
            .and_then(serde_json::Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let env = config
            .get("env")
            .and_then(serde_json::Value::as_object)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|(key, value)| {
                        value
                            .as_str()
                            .map(|value| (key.to_owned(), value.to_owned()))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        let framing = config
            .get("framing")
            .or_else(|| config.get("message_framing"))
            .and_then(serde_json::Value::as_str)
            .map(parse_mcp_framing)
            .unwrap_or(McpFraming::ContentLength);
        let headers = config
            .get("headers")
            .and_then(serde_json::Value::as_object)
            .map(|values| {
                values
                    .iter()
                    .filter_map(|(key, value)| {
                        value
                            .as_str()
                            .map(|value| (key.to_owned(), value.to_owned()))
                    })
                    .collect::<BTreeMap<_, _>>()
            })
            .unwrap_or_default();
        servers.push(McpServerConfig {
            name: name.to_owned(),
            source: source.to_path_buf(),
            command,
            args,
            env,
            framing,
            url,
            headers,
        });
    }
}

fn call_mcp_server(
    config: &McpServerConfig,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if config.command.is_some() {
        call_mcp_stdio_server(config, method, params)
    } else if config.url.is_some() {
        call_mcp_http_server(config, method, params)
    } else {
        Err(format!(
            "MCP server {} has no command or url configured.",
            config.name
        ))
    }
}

fn call_mcp_stdio_server(
    config: &McpServerConfig,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let command = config
        .command
        .as_ref()
        .expect("stdio MCP server must have a command");

    let mut child = Command::new(command)
        .args(&config.args)
        .envs(&config.env)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("Failed to start MCP server {}: {error}", config.name))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| format!("Failed to open stdin for MCP server {}", config.name))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("Failed to open stdout for MCP server {}", config.name))?;
    let stderr = child.stderr.take();
    let (tx, rx) = mpsc::channel();
    let framing = config.framing;
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_mcp_frame(&mut reader, framing) {
                Ok(message) => {
                    if tx.send(Ok(message)).is_err() {
                        break;
                    }
                }
                Err(error) => {
                    let _ = tx.send(Err(error.to_string()));
                    break;
                }
            }
        }
    });

    let initialize = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "deeptide-rs", "version": env!("CARGO_PKG_VERSION")}
        }
    });
    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": method,
        "params": params
    });
    write_mcp_frame(&mut stdin, &initialize, config.framing)
        .and_then(|_| write_mcp_frame(&mut stdin, &initialized, config.framing))
        .and_then(|_| write_mcp_frame(&mut stdin, &request, config.framing))
        .map_err(|error| format!("Failed to write MCP request to {}: {error}", config.name))?;
    drop(stdin);

    let deadline = Instant::now() + Duration::from_secs(10);
    let mut response = None;
    while Instant::now() < deadline {
        let timeout = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(timeout.min(Duration::from_millis(250))) {
            Ok(Ok(message)) => {
                if message.get("id").and_then(serde_json::Value::as_i64) == Some(2) {
                    response = Some(message);
                    break;
                }
            }
            Ok(Err(error)) => {
                return Err(format!("MCP server {} closed stdout: {error}", config.name));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(format!(
                    "MCP server {} stopped before replying",
                    config.name
                ));
            }
        }
    }

    let _ = child.kill();
    let _ = child.wait();

    let Some(response) = response else {
        let stderr_text = read_child_stderr(stderr);
        return Err(format!(
            "MCP server {} timed out waiting for {method}.{}",
            config.name, stderr_text
        ));
    };
    if let Some(error) = response.get("error") {
        return Err(format!(
            "MCP call failed on {}: {}",
            config.name,
            format_json_value(error)
        ));
    }
    Ok(response
        .get("result")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({})))
}

/// Call an MCP server over the Streamable HTTP transport.
///
/// Mirrors the stdio handshake: POST `initialize`, then the
/// `notifications/initialized` notification, then the requested method. The
/// server may answer each POST with either a single `application/json`
/// JSON-RPC object or a `text/event-stream` (SSE) sequence; both are handled.
/// A `Mcp-Session-Id` header returned by `initialize` is echoed on subsequent
/// requests, as required by the MCP spec.
fn call_mcp_http_server(
    config: &McpServerConfig,
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = config
        .url
        .as_ref()
        .expect("HTTP MCP server must have a url");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| {
            format!(
                "Failed to build HTTP client for MCP server {}: {error}",
                config.name
            )
        })?;

    let initialize = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "deeptide-rs", "version": env!("CARGO_PKG_VERSION")}
        }
    });
    let init_response = post_mcp_http(&client, url, &config.headers, None, &initialize, config)?;
    let session_id = init_response
        .headers()
        .get("mcp-session-id")
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned);
    // Surface initialization errors early; the result body is otherwise unused.
    parse_mcp_http_message(init_response, 1, config)?;

    let initialized = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized"
    });
    post_mcp_http(
        &client,
        url,
        &config.headers,
        session_id.as_deref(),
        &initialized,
        config,
    )?;

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": method,
        "params": params
    });
    let response = post_mcp_http(
        &client,
        url,
        &config.headers,
        session_id.as_deref(),
        &request,
        config,
    )?;
    let message = parse_mcp_http_message(response, 2, config)?;

    if let Some(error) = message.get("error") {
        return Err(format!(
            "MCP call failed on {}: {}",
            config.name,
            format_json_value(error)
        ));
    }
    Ok(message
        .get("result")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({})))
}

fn post_mcp_http(
    client: &reqwest::blocking::Client,
    url: &str,
    headers: &BTreeMap<String, String>,
    session_id: Option<&str>,
    body: &serde_json::Value,
    config: &McpServerConfig,
) -> Result<reqwest::blocking::Response, String> {
    let mut request = client
        .post(url)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        )
        .json(body);
    if let Some(session_id) = session_id {
        request = request.header("Mcp-Session-Id", session_id);
    }
    for (key, value) in headers {
        request = request.header(key, value);
    }
    request
        .send()
        .map_err(|error| format!("MCP HTTP request to {} failed: {error}", config.name))
}

/// Read a JSON-RPC message from an HTTP response, accepting both a single JSON
/// object and an SSE stream of `data:` events.
fn parse_mcp_http_message(
    response: reqwest::blocking::Response,
    expected_id: i64,
    config: &McpServerConfig,
) -> Result<serde_json::Value, String> {
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let body = response
        .text()
        .map_err(|error| format!("Failed to read MCP response from {}: {error}", config.name))?;

    if content_type.contains("text/event-stream") {
        return extract_jsonrpc_from_sse(&body, expected_id).ok_or_else(|| {
            format!(
                "MCP server {} returned no JSON-RPC message for id {expected_id}",
                config.name
            )
        });
    }

    if body.trim().is_empty() {
        return Err(format!(
            "MCP server {} returned an empty response (HTTP {})",
            config.name,
            status.as_u16()
        ));
    }

    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| format!("MCP server {} returned invalid JSON: {error}", config.name))?;
    Ok(select_jsonrpc_message(value, expected_id))
}

/// Extract the JSON-RPC message matching `expected_id` from an SSE body.
///
/// Falls back to the first event carrying a `result` or `error` when no id
/// matches, which tolerates servers that omit the id on their final event.
fn extract_jsonrpc_from_sse(body: &str, expected_id: i64) -> Option<serde_json::Value> {
    let mut data = String::new();
    let mut fallback = None;
    let consider = |data: &str, fallback: &mut Option<serde_json::Value>| {
        if data.is_empty() {
            return None;
        }
        let value = serde_json::from_str::<serde_json::Value>(data).ok()?;
        if value.get("id").and_then(serde_json::Value::as_i64) == Some(expected_id) {
            return Some(value);
        }
        if fallback.is_none() && (value.get("result").is_some() || value.get("error").is_some()) {
            *fallback = Some(value);
        }
        None
    };

    for line in body.lines() {
        if let Some(rest) = line.strip_prefix("data:") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            if !data.is_empty() {
                data.push('\n');
            }
            data.push_str(rest);
        } else if line.trim().is_empty() {
            if let Some(found) = consider(&data, &mut fallback) {
                return Some(found);
            }
            data.clear();
        }
    }
    if let Some(found) = consider(&data, &mut fallback) {
        return Some(found);
    }
    fallback
}

/// Pick the JSON-RPC message matching `expected_id` from a value that may be a
/// single object or a batch array.
fn select_jsonrpc_message(value: serde_json::Value, expected_id: i64) -> serde_json::Value {
    let serde_json::Value::Array(items) = value else {
        return value;
    };
    let mut fallback = None;
    for item in items {
        if item.get("id").and_then(serde_json::Value::as_i64) == Some(expected_id) {
            return item;
        }
        if fallback.is_none() && (item.get("result").is_some() || item.get("error").is_some()) {
            fallback = Some(item);
        }
    }
    fallback.unwrap_or_else(|| serde_json::json!({}))
}

fn read_child_stderr(stderr: Option<std::process::ChildStderr>) -> String {
    let Some(mut stderr) = stderr else {
        return String::new();
    };
    let mut text = String::new();
    let _ = stderr.read_to_string(&mut text);
    let text = text.trim();
    if text.is_empty() {
        String::new()
    } else {
        format!(" stderr: {text}")
    }
}

fn parse_mcp_framing(raw: &str) -> McpFraming {
    match raw.trim().to_ascii_lowercase().as_str() {
        "newline" | "json-lines" | "jsonl" | "ndjson" => McpFraming::NewlineJson,
        _ => McpFraming::ContentLength,
    }
}

fn write_mcp_frame(
    writer: &mut impl IoWrite,
    message: &serde_json::Value,
    framing: McpFraming,
) -> io::Result<()> {
    let body = serde_json::to_vec(message).map_err(io::Error::other)?;
    match framing {
        McpFraming::ContentLength => {
            write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
            writer.write_all(&body)?;
        }
        McpFraming::NewlineJson => {
            writer.write_all(&body)?;
            writer.write_all(b"\n")?;
        }
    }
    writer.flush()
}

fn read_mcp_frame(reader: &mut impl BufRead, framing: McpFraming) -> io::Result<serde_json::Value> {
    match framing {
        McpFraming::ContentLength => read_mcp_content_length_frame(reader),
        McpFraming::NewlineJson => read_mcp_newline_json_frame(reader),
    }
}

fn read_mcp_newline_json_frame(reader: &mut impl BufRead) -> io::Result<serde_json::Value> {
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected EOF while reading MCP JSON line",
            ));
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        return serde_json::from_str(trimmed).map_err(io::Error::other);
    }
}

fn read_mcp_content_length_frame(reader: &mut impl BufRead) -> io::Result<serde_json::Value> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "unexpected EOF while reading MCP headers",
            ));
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = Some(value.trim().parse::<usize>().map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("invalid MCP content length: {error}"),
                )
            })?);
        }
    }

    let Some(length) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "missing MCP Content-Length header",
        ));
    };
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(io::Error::other)
}

fn render_mcp_collection(kind: &str, result: &serde_json::Value) -> Vec<String> {
    let key = match kind {
        "resources" => "resources",
        "prompts" => "prompts",
        _ => return vec![format!("  {kind}: {}", format_json_value(result))],
    };
    let Some(items) = result.get(key).and_then(serde_json::Value::as_array) else {
        return vec![format!("  {kind}: (none)")];
    };
    if items.is_empty() {
        return vec![format!("  {kind}: (none)")];
    }

    let mut lines = vec![format!("  {kind}:")];
    for item in items {
        if let Some(object) = item.as_object() {
            let primary = if kind == "prompts" {
                object.get("name")
            } else {
                object.get("uri").or_else(|| object.get("name"))
            }
            .and_then(serde_json::Value::as_str)
            .unwrap_or("?");
            let secondary = if kind == "prompts" {
                object.get("description")
            } else {
                object.get("name").or_else(|| object.get("description"))
            }
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
            if secondary.is_empty() || secondary == primary {
                lines.push(format!("    - {primary}"));
            } else {
                lines.push(format!("    - {primary} - {secondary}"));
            }
        }
    }
    lines
}

fn render_dynamic_mcp_tool_search_entries(cwd: &Path) -> Vec<String> {
    let Ok(servers) = discover_mcp_servers(cwd) else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    for server in servers {
        let Some(command) = &server.command else {
            continue;
        };
        match call_mcp_server(&server, "tools/list", serde_json::json!({})) {
            Ok(result) => {
                let Some(tools) = result.get("tools").and_then(serde_json::Value::as_array) else {
                    continue;
                };
                if !tools.is_empty() && !lines.iter().any(|line| line == "[MCP tools]") {
                    lines.push(String::from("[MCP tools]"));
                }
                for tool in tools {
                    let Some(object) = tool.as_object() else {
                        continue;
                    };
                    let Some(name) = object.get("name").and_then(serde_json::Value::as_str) else {
                        continue;
                    };
                    let description = object
                        .get("description")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("MCP server tool");
                    lines.push(format!(
                        "- mcp__{}__{} [external] - {}",
                        server.name,
                        name,
                        truncate_chars(description, 160)
                    ));
                }
            }
            Err(error) => {
                lines.push(format!(
                    "- MCP server {} ({command}) - tools/list unavailable: {}",
                    server.name,
                    truncate_chars(&error, 120)
                ));
            }
        }
    }
    lines
}

fn format_json_value(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
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
    title: &str,
    subtitle: Option<&str>,
    message: &str,
    sound: bool,
) -> ToolResult {
    let script = build_windows_notification_script(title, subtitle, message, sound);
    let output = Command::new("powershell")
        .args(["-NoProfile", "-WindowStyle", "Hidden", "-Command", &script])
        .output();
    match output {
        Ok(output) if output.status.success() => {
            ToolResult::text(format!("Notification posted: {title}"))
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            ToolResult::error(format!(
                "powershell NotifyIcon exited with code {}. {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            ))
        }
        Err(error) => ToolResult::error(format!(
            "Failed to spawn powershell for Windows notification: {error}"
        )),
    }
}

/// Build a single-line PowerShell script that pops a Windows tray balloon
/// notification using `System.Windows.Forms.NotifyIcon`.
///
/// Implementation choice: NotifyIcon ships with .NET Framework on every
/// supported Windows release, so we don't depend on third-party modules
/// (BurntToast etc.) or `msg.exe` (missing on Home editions). The balloon
/// auto-dismisses; we sleep just long enough to keep the icon alive while
/// Windows renders the popup, then `Dispose()` to release the tray slot.
///
/// Compiled on Windows and in tests on every host so the escape logic
/// stays unit-testable from a developer machine that can't actually
/// cross-compile cargo test to Windows (`ring` needs a Windows linker).
/// Only the wiring that shells out to `powershell` is `target_os =
/// "windows"`-gated; everything pure is testable everywhere.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn build_windows_notification_script(
    title: &str,
    subtitle: Option<&str>,
    message: &str,
    sound: bool,
) -> String {
    // NotifyIcon has no distinct subtitle slot, so we prepend any subtitle
    // into the body separated by a blank line — same shape macOS uses
    // when subtitles fall through to its osascript path.
    let combined_message = match subtitle.filter(|value| !value.is_empty()) {
        Some(subtitle) => format!("{subtitle}\n\n{message}"),
        None => message.to_owned(),
    };
    format!(
        "Add-Type -AssemblyName System.Windows.Forms;\
         $n = New-Object System.Windows.Forms.NotifyIcon;\
         $n.Icon = [System.Drawing.SystemIcons]::Information;\
         $n.Visible = $true;\
         $n.BalloonTipTitle = {title};\
         $n.BalloonTipText  = {body};\
         $n.BalloonTipIcon  = [System.Windows.Forms.ToolTipIcon]::Info;\
         if ({sound_flag}) {{ [System.Console]::Beep() }};\
         $n.ShowBalloonTip(5000);\
         Start-Sleep -Milliseconds 5500;\
         $n.Dispose();",
        title = powershell_string(title),
        body = powershell_string(&combined_message),
        sound_flag = if sound { "$true" } else { "$false" },
    )
}

/// Escape a string for safe embedding inside a PowerShell single-quoted
/// literal. PowerShell single quotes do not interpret backslashes, but a
/// literal single quote must be doubled (`''`). Carriage returns and
/// newlines are stripped so the inline command stays a single statement.
#[cfg(any(target_os = "windows", test))]
pub(crate) fn powershell_string(value: &str) -> String {
    let escaped = value
        .replace('\'', "''")
        .replace('\r', "")
        .replace('\n', " ");
    format!("'{escaped}'")
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

fn clipboard_snapshot() -> Result<ClipboardSnapshot, String> {
    let text = clipboard_read_text()?;
    let file_paths = clipboard_file_paths().unwrap_or_default();
    Ok(ClipboardSnapshot::from_text_and_files(text, file_paths))
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
        "Agent" => vec![
            "subagent",
            "delegate",
            "explore",
            "plan",
            "parallel",
            "multi step",
            "research",
        ],
        "MCP" => vec![
            "mcp",
            "model context protocol",
            "json rpc",
            "server",
            "tool bridge",
        ],
        "ListMcpResources" | "ReadMcpResource" => vec![
            "mcp",
            "model context protocol",
            "resource",
            "server",
            "context",
        ],
        "ListMcpPrompts" | "GetMcpPrompt" => vec![
            "mcp",
            "model context protocol",
            "prompt",
            "template",
            "server",
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
        "AudioTranscribe" => vec![
            "audio",
            "speech",
            "transcribe",
            "transcription",
            "voice",
            "recording",
        ],
        "VideoTranscribe" => vec![
            "video",
            "speech",
            "transcribe",
            "transcription",
            "audio track",
            "subtitles",
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
        if let Some(reason) = crate::sensitive_file::shell_command_block_reason(command, |token| {
            context.resolve_path(token)
        }) {
            return ToolResult::error(reason);
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
pub struct BashOutputTool;

impl Tool for BashOutputTool {
    fn name(&self) -> &'static str {
        "BashOutput"
    }

    fn description(&self) -> &'static str {
        "Read the accumulated stdout/stderr of a background Bash invocation. \
         Pass shell_id from the original Bash run_in_background response. \
         Optional stdout_cursor/stderr_cursor return only lines produced after the given counter."
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(shell_id) = input.get("shell_id").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("shell_id is required");
        };
        let shell_id = shell_id.trim();
        if shell_id.is_empty() {
            return ToolResult::error("shell_id is required");
        }
        let since_stdout = input
            .get("stdout_cursor")
            .and_then(serde_json::Value::as_u64);
        let since_stderr = input
            .get("stderr_cursor")
            .and_then(serde_json::Value::as_u64);

        match crate::background_shell::read_output(shell_id, since_stdout, since_stderr) {
            Ok(snapshot) => ToolResult::text(render_background_snapshot(&snapshot)),
            Err(error) => ToolResult::error(error),
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct KillBashTool;

impl Tool for KillBashTool {
    fn name(&self) -> &'static str {
        "KillBash"
    }

    fn description(&self) -> &'static str {
        "Send SIGKILL to a background Bash invocation registered by Bash run_in_background. \
         Returns the final accumulated output. Idempotent — calling on an already-exited shell returns its recorded exit."
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn call(&self, input: serde_json::Value, _context: &ToolContext) -> ToolResult {
        let Some(shell_id) = input.get("shell_id").and_then(serde_json::Value::as_str) else {
            return ToolResult::error("shell_id is required");
        };
        let shell_id = shell_id.trim();
        if shell_id.is_empty() {
            return ToolResult::error("shell_id is required");
        }
        match crate::background_shell::stop(shell_id) {
            Ok(snapshot) => ToolResult::text(render_background_snapshot(&snapshot)),
            Err(error) => ToolResult::error(error),
        }
    }
}

fn render_background_snapshot(
    snapshot: &crate::background_shell::BackgroundOutputSnapshot,
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "shell_id={} running={} elapsed={}s",
        snapshot.shell_id, snapshot.running, snapshot.elapsed_seconds
    ));
    if let Some(code) = snapshot.exit_code {
        out.push_str(&format!(" exit_code={code}"));
    }
    out.push('\n');
    out.push_str(&format!(
        "stdout_cursor={} stderr_cursor={}",
        snapshot.stdout_cursor, snapshot.stderr_cursor
    ));
    if snapshot.stdout_truncated || snapshot.stderr_truncated {
        out.push_str(" (truncated)");
    }
    out.push('\n');
    out.push_str(&format!("command: {}\n", snapshot.command));
    out.push_str("--- stdout ---\n");
    if snapshot.stdout.is_empty() {
        out.push_str("(no new lines)\n");
    } else {
        out.push_str(&snapshot.stdout);
        if !snapshot.stdout.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str("--- stderr ---\n");
    if snapshot.stderr.is_empty() {
        out.push_str("(no new lines)\n");
    } else {
        out.push_str(&snapshot.stderr);
        if !snapshot.stderr.ends_with('\n') {
            out.push('\n');
        }
    }
    out
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
        if !crate::sensitive_file::is_allowed(&path) {
            return ToolResult::error(crate::sensitive_file::denial_message(&path));
        }
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
        if !crate::sensitive_file::is_allowed(&path) {
            return ToolResult::error(crate::sensitive_file::denial_message(&path));
        }
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

/// Default number of lines a single `Read` returns when no explicit limit is
/// given, matching the Swift `FileReadTool.defaultLineLimit`.
const DEFAULT_READ_LINES: usize = 2_000;
/// Maximum estimated tokens a single `Read` will return. Above this the tool
/// reports an error with guidance rather than flooding the context, matching
/// the Swift `FileReadTool.maxTextOutputTokens`.
const MAX_READ_OUTPUT_TOKENS: usize = 25_000;

/// Validate that `path` is a readable regular file and open it, returning the
/// same not-found / directory / special-file errors the Read tools surface.
fn open_readable_file(path: &Path) -> Result<fs::File, String> {
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

    fs::File::open(path).map_err(|error| format!("Failed to open {}: {error}", path.display()))
}

fn read_text_file(path: &Path, offset: Option<usize>, limit: Option<usize>) -> ToolResult {
    let file = match open_readable_file(path) {
        Ok(file) => file,
        Err(message) => return ToolResult::error(message),
    };

    let start = offset.unwrap_or(1).saturating_sub(1);
    let requested_limit = limit.unwrap_or(DEFAULT_READ_LINES).max(1);

    let mut selected = Vec::new();
    let mut line_number = 0usize;
    let mut has_more = false;
    for line in io::BufReader::new(file).lines() {
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                return ToolResult::error(format!("Failed to read {}: {error}", path.display()));
            }
        };
        line_number += 1;
        if line_number <= start {
            continue;
        }
        if selected.len() < requested_limit {
            selected.push((line_number, line));
        } else {
            // We read one line past the window, so more content remains.
            has_more = true;
            break;
        }
    }

    let output = selected
        .iter()
        .map(|(line_number, line)| format!("{line_number}\t{line}"))
        .collect::<Vec<_>>()
        .join("\n");

    // Never silently crop: if the selected range is too large, tell the model
    // exactly how to narrow the read instead of returning a flood of tokens.
    let tokens = estimate_tokens(&output);
    if tokens > MAX_READ_OUTPUT_TOKENS {
        let end_hint = start + selected.len().max(1);
        return ToolResult::error(format!(
            "File content for {} is too large to return safely ({tokens} estimated tokens; max {MAX_READ_OUTPUT_TOKENS}).\nUse offset and limit to read a smaller range, or use Grep to locate specific content first.\nAttempted range: lines {}-{end_hint}.",
            path.display(),
            start + 1
        ));
    }

    // Only advise continuation when the default limit kicked in. An explicit
    // `limit` is a deliberate truncation, so the model already knows more may
    // remain and a trailing notice would only distort result summaries.
    if has_more && limit.is_none() {
        let next_offset = start + selected.len() + 1;
        return ToolResult::text(format!(
            "{output}\n\n[Read stopped at the default {DEFAULT_READ_LINES}-line limit. More content may exist. Continue with offset: {next_offset}, limit: {DEFAULT_READ_LINES}, or use Grep to search within the file.]"
        ));
    }

    ToolResult::text(output)
}

fn read_text_file_limited(path: &Path, line_limit: Option<usize>) -> Result<String, String> {
    let file = open_readable_file(path)?;

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
    match crate::background_shell::spawn(command, &context.cwd) {
        Ok(spawned) => {
            let pid_note = spawned
                .pid
                .map(|pid| format!(", pid={pid}"))
                .unwrap_or_default();
            ToolResult::text(format!(
                "Background shell started: shell_id={}{pid_note}. Use BashOutput {{\"shell_id\":\"{}\"}} to read its stdout/stderr or KillBash to stop it.\nCommand: {}",
                spawned.shell_id,
                spawned.shell_id,
                truncate_chars(command, 100)
            ))
        }
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

    let output = truncate_command_output(&output, bash_max_output_chars(), 500);
    if failed {
        ToolResult::error(output)
    } else {
        ToolResult::text(output)
    }
}

/// Default character ceiling for Bash output, matching the Swift BashTool.
const DEFAULT_BASH_MAX_OUTPUT: usize = 30_000;

/// Clamp a configured Bash output ceiling to the supported range, or fall back
/// to the default. Mirrors Swift's `min(150_000, max(10_000, envMax)) ?? 30_000`.
fn clamp_bash_max(value: Option<usize>) -> usize {
    value
        .map(|chars| chars.clamp(10_000, 150_000))
        .unwrap_or(DEFAULT_BASH_MAX_OUTPUT)
}

/// Resolve the Bash output ceiling, honoring the `TIDE_BASH_MAX_OUTPUT`
/// environment override (clamped to [10K, 150K]).
fn bash_max_output_chars() -> usize {
    clamp_bash_max(
        std::env::var("TIDE_BASH_MAX_OUTPUT")
            .ok()
            .and_then(|raw| raw.trim().parse::<usize>().ok()),
    )
}

fn truncate_command_output(output: &str, max_chars: usize, max_lines: usize) -> String {
    let total_lines = output.lines().count();
    let mut truncated = output
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n");
    if output.ends_with('\n') && total_lines <= max_lines {
        truncated.push('\n');
    }
    let mut did_truncate = total_lines > max_lines;
    if truncated.chars().count() > max_chars {
        truncated = truncate_chars(&truncated, max_chars);
        did_truncate = true;
    }
    if did_truncate {
        // Report how many whole lines were dropped so the model knows to
        // re-run narrowed (head/tail/grep). Matches the Swift truncation marker.
        let elided = total_lines.saturating_sub(truncated.lines().count());
        truncated.push_str(&format!("\n... [{elided} lines truncated]"));
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

/// Search options resolved from the `Grep` tool input.
struct GrepOptions {
    output_mode: String,
    head_limit: usize,
    offset: usize,
    /// Context lines before each match (content mode only).
    before: usize,
    /// Context lines after each match (content mode only).
    after: usize,
    /// Whether to prefix content lines with their line number.
    line_numbers: bool,
    /// When `Some`, only files with one of these (lowercase, dotless)
    /// extensions are searched. Resolved from the `type` parameter.
    type_extensions: Option<Vec<String>>,
    /// Whether the pattern may span lines (the regex was built with
    /// dot-matches-newline + multiline anchoring).
    multiline: bool,
}

impl GrepOptions {
    fn is_content_mode(&self) -> bool {
        self.output_mode == "content"
    }

    /// Whether `path` passes the `type` filter (always true when no filter set).
    fn matches_type(&self, path: &Path) -> bool {
        let Some(extensions) = &self.type_extensions else {
            return true;
        };
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) => {
                let ext = ext.to_ascii_lowercase();
                extensions.iter().any(|candidate| candidate == &ext)
            }
            None => false,
        }
    }
}

/// Map a `type` name to the file extensions it covers. Mirrors the common
/// ripgrep `--type` definitions Swift relies on. Returns `None` for unknown
/// types so the tool can report an error like ripgrep does.
fn file_type_extensions(name: &str) -> Option<&'static [&'static str]> {
    let extensions: &'static [&'static str] = match name.to_ascii_lowercase().as_str() {
        "rust" | "rs" => &["rs"],
        "js" | "javascript" => &["js", "jsx", "mjs", "cjs"],
        "ts" | "typescript" => &["ts", "tsx", "mts", "cts"],
        "py" | "python" => &["py", "pyi"],
        "go" => &["go"],
        "swift" => &["swift"],
        "java" => &["java"],
        "kotlin" | "kt" => &["kt", "kts"],
        "c" => &["c", "h"],
        "cpp" | "c++" | "cxx" => &["cpp", "cc", "cxx", "hpp", "hh", "hxx"],
        "rb" | "ruby" => &["rb"],
        "php" => &["php"],
        "html" => &["html", "htm"],
        "css" => &["css"],
        "json" => &["json"],
        "md" | "markdown" => &["md", "markdown"],
        "sh" | "shell" | "bash" => &["sh", "bash"],
        "yaml" | "yml" => &["yaml", "yml"],
        "toml" => &["toml"],
        _ => return None,
    };
    Some(extensions)
}

fn grep_path(
    base: &Path,
    path: &Path,
    regex: &regex::Regex,
    glob: Option<&GlobMatcher>,
    options: &GrepOptions,
) -> ToolResult {
    if path.is_file() {
        // Don't surface matching lines from a sensitive file (e.g. `.env`)
        // unless it has been explicitly opened — return as if it had no matches.
        // The `type` filter is likewise honoured for a single explicit file.
        if !crate::sensitive_file::is_allowed(path) || !options.matches_type(path) {
            return render_grep_output(options, BTreeMap::new(), Vec::new(), base);
        }
        return grep_file(path.parent().unwrap_or(base), path, regex, options);
    }
    if !path.is_dir() {
        return ToolResult::error(format!("Path does not exist: {}", path.display()));
    }

    let collect_limit = collect_limit(options.head_limit, options.offset);
    let mut matching_files = BTreeMap::<String, usize>::new();
    let mut content_matches = Vec::new();
    collect_files(path, path, &mut |relative, full_path| {
        // Skip sensitive files (unless opened) so secret contents never leak
        // into grep results; keep walking the rest of the tree.
        if !crate::sensitive_file::is_allowed(full_path) {
            return true;
        }
        if let Some(glob) = glob
            && !glob.matches(relative)
        {
            return true;
        }
        if !options.matches_type(full_path) {
            return true;
        }
        let relative = relative.to_string_lossy().replace('\\', "/");
        if let Ok((count, lines)) = process_grep_file(&relative, full_path, regex, options)
            && count > 0
        {
            matching_files.insert(relative, count);
            content_matches.extend(lines);
        }
        matching_files.len().max(content_matches.len()) < collect_limit
    });

    render_grep_output(options, matching_files, content_matches, base)
}

fn grep_file(base: &Path, path: &Path, regex: &regex::Regex, options: &GrepOptions) -> ToolResult {
    let relative = path
        .strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    match process_grep_file(&relative, path, regex, options) {
        Ok((count, lines)) => {
            let mut matching_files = BTreeMap::new();
            if count > 0 {
                matching_files.insert(relative, count);
            }
            render_grep_output(options, matching_files, lines, base)
        }
        Err(error) => ToolResult::error(format!("Failed to search {}: {error}", path.display())),
    }
}

/// Read `path`, returning the number of matching lines and — in content mode —
/// the formatted output lines (matches plus any requested context).
/// Match `regex` against the whole file (lines joined with `\n`) and return the
/// sorted, de-duplicated indices of every line spanned by a match. Used for
/// multiline searches where a single match can cover several lines.
fn multiline_match_indices(all_lines: &[String], regex: &regex::Regex) -> Vec<usize> {
    let content = all_lines.join("\n");
    // Byte offset at which each line begins within `content`.
    let mut line_starts = Vec::with_capacity(all_lines.len());
    let mut offset = 0usize;
    for line in all_lines {
        line_starts.push(offset);
        offset += line.len() + 1; // +1 for the joining '\n'
    }
    let line_of = |byte: usize| -> usize {
        // Last line whose start is <= byte.
        line_starts
            .partition_point(|&start| start <= byte)
            .saturating_sub(1)
    };

    let mut indices = std::collections::BTreeSet::new();
    for m in regex.find_iter(&content) {
        let start_line = line_of(m.start());
        // Use the last byte of the match so a match ending exactly at a line
        // boundary doesn't pull in the following line.
        let last_byte = m.end().saturating_sub(1).max(m.start());
        let end_line = line_of(last_byte);
        for line in start_line..=end_line {
            indices.insert(line);
        }
    }
    indices.into_iter().collect()
}

fn process_grep_file(
    relative: &str,
    path: &Path,
    regex: &regex::Regex,
    options: &GrepOptions,
) -> io::Result<(usize, Vec<String>)> {
    let file = fs::File::open(path)?;
    let mut all_lines = Vec::new();
    for line in io::BufReader::new(file).lines() {
        all_lines.push(line?);
    }
    let match_indices = if options.multiline {
        multiline_match_indices(&all_lines, regex)
    } else {
        all_lines
            .iter()
            .enumerate()
            .filter(|(_, line)| regex.is_match(line))
            .map(|(index, _)| index)
            .collect()
    };
    if match_indices.is_empty() {
        return Ok((0, Vec::new()));
    }
    let lines = if options.is_content_mode() {
        format_grep_content(relative, &all_lines, &match_indices, options)
    } else {
        Vec::new()
    };
    Ok((match_indices.len(), lines))
}

/// Format content-mode output: matching lines as `path:line:text`, context
/// lines as `path-line-text`, with `--` separators between non-contiguous
/// groups (ripgrep convention). `--` separators are only emitted when context
/// is requested.
fn format_grep_content(
    relative: &str,
    all_lines: &[String],
    match_indices: &[usize],
    options: &GrepOptions,
) -> Vec<String> {
    let context_mode = options.before > 0 || options.after > 0;
    let match_set: std::collections::HashSet<usize> = match_indices.iter().copied().collect();

    let mut to_emit: Vec<usize> = Vec::new();
    let last_index = all_lines.len().saturating_sub(1);
    for &index in match_indices {
        let start = index.saturating_sub(options.before);
        let end = index.saturating_add(options.after).min(last_index);
        for i in start..=end {
            to_emit.push(i);
        }
    }
    to_emit.sort_unstable();
    to_emit.dedup();

    let mut out = Vec::with_capacity(to_emit.len());
    let mut previous: Option<usize> = None;
    for &index in &to_emit {
        if context_mode
            && let Some(prev) = previous
            && index > prev + 1
        {
            out.push(String::from("--"));
        }
        let separator = if match_set.contains(&index) { ':' } else { '-' };
        if options.line_numbers {
            out.push(format!(
                "{relative}{separator}{}{separator}{}",
                index + 1,
                all_lines[index]
            ));
        } else {
            out.push(format!("{relative}{separator}{}", all_lines[index]));
        }
        previous = Some(index);
    }
    out
}

fn render_grep_output(
    options: &GrepOptions,
    matching_files: BTreeMap<String, usize>,
    content_matches: Vec<String>,
    _base: &Path,
) -> ToolResult {
    let head_limit = options.head_limit;
    let offset = options.offset;
    match options.output_mode.as_str() {
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

/// Locate an executable on `PATH` (shared by the LSP server lookup and media
/// duration probing). Honors Windows executable extensions.
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
        let snapshot = ClipboardSnapshot::from_text_and_files(
            String::new(),
            vec![String::from("/tmp/a.txt"), String::from("/tmp/b.txt")],
        );

        assert_eq!(
            snapshot.render_read(),
            "[Clipboard file URLs]\n/tmp/a.txt\n/tmp/b.txt"
        );
        assert!(snapshot.render_inspect().contains("files: 2"));
    }

    #[test]
    fn clipboard_snapshot_read_prefers_text_but_inspects_files() {
        let snapshot = ClipboardSnapshot::from_text_and_files(
            String::from("hello"),
            vec![String::from("/tmp/a.txt")],
        );

        assert_eq!(snapshot.render_read(), "hello");
        let rendered = snapshot.render_inspect();
        assert!(rendered.contains("types: public.file-url, text/plain"));
        assert!(rendered.contains("text: 5 chars"));
        assert!(rendered.contains("files: 1"));
        assert!(rendered.contains("  /tmp/a.txt"));
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

#[cfg(test)]
mod agent_definition_tests {
    use super::AgentDefinition;
    use std::path::Path;

    #[test]
    fn every_agent_definition_has_a_role_system_prompt() {
        for kind in AgentDefinition::all_types() {
            let definition = AgentDefinition::find(kind).expect("definition exists");
            assert!(
                !definition.system_prompt.trim().is_empty(),
                "{kind} should carry a role system prompt"
            );
        }
    }

    #[test]
    fn full_system_prompt_includes_role_and_working_context() {
        let definition = AgentDefinition::find("Explore").expect("Explore definition");
        // The Explore role is read-only; its prompt must say so.
        assert!(definition.system_prompt.contains("READ-ONLY"));

        let prompt = definition.full_system_prompt(Path::new("/tmp/project"), "claude-test");
        assert!(prompt.contains("READ-ONLY"), "role text is preserved");
        assert!(
            prompt.contains("/tmp/project"),
            "working directory is included"
        );
        assert!(prompt.contains("claude-test"), "model is included");
        assert!(
            prompt.contains(&definition.max_turns.to_string()),
            "turn budget is included"
        );
    }
}

#[cfg(test)]
mod bash_output_tests {
    use super::{clamp_bash_max, truncate_command_output};

    #[test]
    fn short_output_is_untouched() {
        let out = truncate_command_output("one\ntwo\nthree", 30_000, 500);
        assert_eq!(out, "one\ntwo\nthree");
        assert!(!out.contains("lines truncated"));
    }

    #[test]
    fn line_cap_reports_elided_count() {
        let body = (1..=600)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let out = truncate_command_output(&body, 30_000, 500);
        assert!(out.contains("line 1"));
        assert!(out.contains("line 500"));
        assert!(!out.contains("line 501"));
        // 600 total - 500 kept = 100 dropped whole lines.
        assert!(
            out.contains("... [100 lines truncated]"),
            "marker should report the dropped line count: {out}"
        );
    }

    #[test]
    fn char_cap_truncates_a_single_huge_line() {
        let body = "x".repeat(50_000);
        let out = truncate_command_output(&body, 30_000, 500);
        assert!(out.chars().count() <= 30_000 + 40);
        assert!(out.contains("lines truncated"));
    }

    #[test]
    fn clamp_bash_max_applies_range_and_default() {
        assert_eq!(clamp_bash_max(None), 30_000);
        assert_eq!(clamp_bash_max(Some(50_000)), 50_000);
        assert_eq!(clamp_bash_max(Some(1_000)), 10_000);
        assert_eq!(clamp_bash_max(Some(999_999)), 150_000);
    }
}

#[cfg(test)]
mod windows_notification_tests {
    //! Unit tests for the Windows PushNotification PowerShell builder.
    //! These exercise the pure script/escape helpers on any host so we
    //! don't have to wait for Windows CI to catch escape regressions.
    use super::{build_windows_notification_script, powershell_string};

    #[test]
    fn powershell_string_wraps_in_single_quotes_and_doubles_internal_quotes() {
        assert_eq!(powershell_string(""), "''");
        assert_eq!(powershell_string("hello"), "'hello'");
        // Single quotes inside the value must be doubled — that's the only
        // escape PowerShell single-quoted literals honor.
        assert_eq!(powershell_string("it's fine"), "'it''s fine'");
        assert_eq!(powershell_string("don't 'do' that"), "'don''t ''do'' that'");
    }

    #[test]
    fn powershell_string_strips_newlines_so_the_script_stays_one_statement() {
        // Newlines would terminate the PowerShell command — replace with
        // spaces. Carriage returns are stripped entirely so `\r\n`
        // collapses to a single space, not two characters.
        assert_eq!(powershell_string("a\nb"), "'a b'");
        assert_eq!(powershell_string("a\r\nb"), "'a b'");
        assert_eq!(powershell_string("a\rb"), "'ab'");
    }

    #[test]
    fn powershell_string_passes_backslashes_through_unchanged() {
        // Single-quoted PowerShell literals do NOT interpret backslashes.
        // Path-like inputs must round-trip untouched.
        assert_eq!(
            powershell_string(r"C:\Users\test\file"),
            r"'C:\Users\test\file'"
        );
    }

    #[test]
    fn build_windows_notification_script_includes_title_body_and_sound() {
        let script =
            build_windows_notification_script("Build Finished", None, "Tests are green.", false);
        assert!(script.contains("System.Windows.Forms.NotifyIcon"));
        assert!(script.contains("'Build Finished'"));
        assert!(script.contains("'Tests are green.'"));
        // sound = false → the conditional branch must not auto-Beep.
        assert!(script.contains("if ($false)"));
        // Dispose must always run so we don't leak tray icons.
        assert!(script.contains(".Dispose()"));
    }

    #[test]
    fn build_windows_notification_script_inlines_subtitle_before_body() {
        let script = build_windows_notification_script(
            "Deploy",
            Some("staging-us-east"),
            "Rolled out to 47 hosts.",
            true,
        );
        // Subtitle + blank line + body must collapse to a single
        // PowerShell-safe literal once escaped (newlines → spaces).
        assert!(
            script.contains("'staging-us-east  Rolled out to 47 hosts.'"),
            "expected subtitle then body in single literal; got: {script}"
        );
        // sound = true → Beep branch must be reachable.
        assert!(script.contains("if ($true)"));
    }

    #[test]
    fn build_windows_notification_script_omits_subtitle_when_blank() {
        // Empty subtitles should be treated like absent ones — no
        // leading "  " on the body literal.
        let script = build_windows_notification_script("T", Some(""), "M", false);
        assert!(script.contains("'M'"));
        assert!(!script.contains("'  M'"));
    }
}
