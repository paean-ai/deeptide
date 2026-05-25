use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

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

fn read_text_file(path: &Path, offset: Option<usize>, limit: Option<usize>) -> ToolResult {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or_default();
            return ToolResult::error(format!(
                "File does not exist: {}\nHint: the file may be at a different location. Use Glob(pattern: \"**/{filename}\") to locate it.",
                path.display()
            ));
        }
        Err(error) => {
            return ToolResult::error(format!("Failed to inspect {}: {error}", path.display()));
        }
    };

    if metadata.is_dir() {
        return ToolResult::error(format!(
            "Path is a directory: {}\nUse Glob to list files or Grep to search within it.",
            path.display()
        ));
    }

    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return ToolResult::error(format!("Failed to open {}: {error}", path.display()));
        }
    };

    let start = offset.unwrap_or(1).saturating_sub(1);
    let needed = limit.unwrap_or(usize::MAX);
    let mut selected = Vec::new();
    let mut total_lines = 0usize;

    for line in io::BufReader::new(file).lines() {
        total_lines += 1;
        let line = match line {
            Ok(line) => line,
            Err(error) => {
                return ToolResult::error(format!("Failed to read {}: {error}", path.display()));
            }
        };

        if total_lines > start && selected.len() < needed {
            selected.push((total_lines, line));
        }
        if selected.len() >= needed {
            break;
        }
    }

    let output = selected
        .into_iter()
        .map(|(line_number, line)| format!("{line_number}\t{line}"))
        .collect::<Vec<_>>()
        .join("\n");

    ToolResult::text(output)
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
