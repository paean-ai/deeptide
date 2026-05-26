use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentTerminalEvent, ClearCommand,
    CommandCompletionSource, CommandContext, CommandResult, CompactCommand, CostCommand,
    HelpCommand, MemoryCommand, NewCommand, PermissionMode, RememberCommand, SlashCommand, Tool,
    ToolContext, ToolRegistry, WriteTool,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplEvent {
    Output(String),
    Exit,
}

pub struct ReplSession {
    agent_loop: AgentLoop,
    cost_display_enabled: Arc<AtomicBool>,
    tool_registry: ToolRegistry,
    tool_context: ToolContext,
}

impl ReplSession {
    pub fn new(backend: Box<dyn AgentBackend>) -> Self {
        Self {
            agent_loop: AgentLoop::new(backend),
            cost_display_enabled: Arc::new(AtomicBool::new(false)),
            tool_registry: ToolRegistry::with_builtin_tools(),
            tool_context: ToolContext::new(
                std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from(".")),
            ),
        }
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.agent_loop = self.agent_loop.with_model(model);
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<std::path::PathBuf>) -> Self {
        self.tool_context = ToolContext::new(cwd);
        self.agent_loop = self.agent_loop.with_cwd(self.tool_context.cwd.clone());
        self
    }

    pub fn with_permission_mode(mut self, mode: PermissionMode) -> Self {
        self.agent_loop = self.agent_loop.with_permission_mode(mode);
        self
    }

    pub fn submit(&mut self, line: &str) -> Vec<ReplEvent> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Vec::new();
        }

        if let Some(command_line) = trimmed.strip_prefix('/') {
            return self.execute_command(command_line);
        }

        self.agent_loop
            .run(trimmed)
            .into_iter()
            .filter_map(agent_event_to_repl_event)
            .collect()
    }

    pub fn banner(&self) -> String {
        String::from("Deeptide Rust REPL")
    }

    pub fn prompt(&self) -> String {
        String::from("deeptide> ")
    }

    pub fn agent_loop(&self) -> &AgentLoop {
        &self.agent_loop
    }

    fn execute_command(&mut self, command_line: &str) -> Vec<ReplEvent> {
        let mut parts = command_line.splitn(2, char::is_whitespace);
        let name = parts.next().unwrap_or_default().to_ascii_lowercase();
        let args = parts.next().unwrap_or_default();

        if matches!(name.as_str(), "exit" | "quit" | "q") {
            return vec![ReplEvent::Exit];
        }

        let context = self.command_context();
        let result = match name.as_str() {
            "help" | "h" | "?" => HelpCommand.execute(args, &context),
            "clear" | "cls" => {
                self.agent_loop.reset();
                ClearCommand.execute(args, &context)
            }
            "new" => {
                self.agent_loop.reset();
                NewCommand.execute(args, &context)
            }
            "compact" | "compress" => CompactCommand.execute(args, &context),
            "cost" => CostCommand.execute(args, &context),
            "read" => self.execute_read_command(args),
            "write" => self.execute_write_command(args),
            "memory" | "mem" => MemoryCommand.execute(args, &context),
            "remember" => RememberCommand.execute(args, &context),
            _ => CommandResult::Text(format!(
                "Unknown command: /{name}\nType /help for the full list."
            )),
        };

        command_result_to_repl_events(result)
    }

    fn execute_read_command(&self, args: &str) -> CommandResult {
        let parsed = match parse_read_args(args) {
            Ok(parsed) => parsed,
            Err(message) => return CommandResult::Text(message),
        };

        let mut input = serde_json::Map::new();
        input.insert(
            String::from("file_path"),
            serde_json::Value::String(parsed.file_path),
        );
        if let Some(offset) = parsed.offset {
            input.insert(String::from("offset"), serde_json::json!(offset));
        }
        if let Some(limit) = parsed.limit {
            input.insert(String::from("limit"), serde_json::json!(limit));
        }

        let result =
            self.tool_registry
                .call("Read", serde_json::Value::Object(input), &self.tool_context);
        CommandResult::Text(result.content)
    }

    fn execute_write_command(&self, args: &str) -> CommandResult {
        let parsed = match parse_write_args(args) {
            Ok(parsed) => parsed,
            Err(message) => return CommandResult::Text(message),
        };

        let result = WriteTool.call(
            serde_json::json!({
                "file_path": parsed.file_path,
                "content": parsed.content,
            }),
            &self.tool_context,
        );
        CommandResult::Text(result.content)
    }

    fn command_context(&self) -> CommandContext {
        let cost_display_enabled = Arc::clone(&self.cost_display_enabled);
        let set_cost_display_enabled = Arc::clone(&self.cost_display_enabled);
        let summary = self.agent_loop.cost_tracker().summary();

        CommandContext::builder()
            .clear_conversation(|| Some(String::new()))
            .compact_conversation(|| {})
            .all_commands(repl_command_sources)
            .cost_summary(move || summary.clone())
            .cost_display_enabled(move || cost_display_enabled.load(Ordering::SeqCst))
            .set_cost_display_enabled(move |enabled| {
                set_cost_display_enabled.store(enabled, Ordering::SeqCst);
            })
            .build()
    }
}

fn agent_event_to_repl_event(event: AgentLoopEvent) -> Option<ReplEvent> {
    match event {
        AgentLoopEvent::Assistant(message) => Some(ReplEvent::Output(message.content)),
        AgentLoopEvent::ToolBatchSummary {
            label,
            failed_count,
            ..
        } => {
            let status = if failed_count == 0 {
                "completed"
            } else {
                "completed with failures"
            };
            Some(ReplEvent::Output(format!("Tools {status}: {label}")))
        }
        AgentLoopEvent::Terminal(AgentTerminalEvent::MaxTurnsReached) => {
            Some(ReplEvent::Output(String::from("Maximum turns reached.")))
        }
        AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(error)) => {
            Some(ReplEvent::Output(format!("Model error: {error}")))
        }
        AgentLoopEvent::ToolResult {
            tool_call,
            content,
            is_error,
        } => Some(ReplEvent::Output(format!(
            "Tool {} ({}) {}\n{}",
            tool_call.name,
            tool_call.id,
            if is_error { "failed:" } else { "completed:" },
            content
        ))),
        AgentLoopEvent::User(_) | AgentLoopEvent::Terminal(AgentTerminalEvent::Complete) => None,
    }
}

fn command_result_to_repl_events(result: CommandResult) -> Vec<ReplEvent> {
    match result {
        CommandResult::Text(text) => vec![ReplEvent::Output(text)],
        CommandResult::Exit => vec![ReplEvent::Exit],
        CommandResult::Compact => vec![ReplEvent::Output(String::from("Context compacted."))],
        CommandResult::Skip => Vec::new(),
    }
}

fn repl_command_sources() -> Vec<CommandCompletionSource> {
    vec![
        CommandCompletionSource::from_command(&HelpCommand),
        CommandCompletionSource::new("exit", ["quit", "q"], "Exit the REPL", "/exit"),
        CommandCompletionSource::from_command(&ClearCommand),
        CommandCompletionSource::from_command(&NewCommand),
        CommandCompletionSource::from_command(&CompactCommand),
        CommandCompletionSource::from_command(&CostCommand),
        CommandCompletionSource::new(
            "read",
            Vec::<&str>::new(),
            "Read a text file with optional line range",
            "/read <path> [--offset N] [--limit N]",
        ),
        CommandCompletionSource::new(
            "write",
            Vec::<&str>::new(),
            "Write complete text to a file",
            "/write <path> <content>",
        ),
        CommandCompletionSource::from_command(&MemoryCommand),
        CommandCompletionSource::from_command(&RememberCommand),
    ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadArgs {
    file_path: String,
    offset: Option<usize>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WriteArgs {
    file_path: String,
    content: String,
}

fn parse_read_args(args: &str) -> Result<ReadArgs, String> {
    let parts = split_shell_like(args);
    if parts.is_empty() {
        return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
    }

    let mut file_path = None;
    let mut offset = None;
    let mut limit = None;
    let mut index = 0;
    while index < parts.len() {
        match parts[index].as_str() {
            "--offset" => {
                index += 1;
                let Some(raw) = parts.get(index) else {
                    return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
                };
                offset = Some(
                    raw.parse::<usize>()
                        .map_err(|_| String::from("--offset must be a positive number"))?,
                );
            }
            "--limit" => {
                index += 1;
                let Some(raw) = parts.get(index) else {
                    return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
                };
                limit = Some(
                    raw.parse::<usize>()
                        .map_err(|_| String::from("--limit must be a positive number"))?,
                );
            }
            value if value.starts_with("--") => {
                return Err(format!("Unknown /read option: {value}"));
            }
            value => {
                if file_path.is_some() {
                    return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
                }
                file_path = Some(value.to_owned());
            }
        }
        index += 1;
    }

    let Some(file_path) = file_path else {
        return Err(String::from("Usage: /read <path> [--offset N] [--limit N]"));
    };

    Ok(ReadArgs {
        file_path,
        offset,
        limit,
    })
}

fn parse_write_args(args: &str) -> Result<WriteArgs, String> {
    let trimmed = args.trim_start();
    if trimmed.is_empty() {
        return Err(String::from("Usage: /write <path> <content>"));
    }

    let mut split_at = None;
    for (index, character) in trimmed.char_indices() {
        if character.is_whitespace() {
            split_at = Some(index);
            break;
        }
    }

    let Some(split_at) = split_at else {
        return Err(String::from("Usage: /write <path> <content>"));
    };
    let file_path = trimmed[..split_at].to_owned();
    let content = trimmed[split_at..].trim_start().to_owned();
    if content.is_empty() {
        return Err(String::from("Usage: /write <path> <content>"));
    }

    Ok(WriteArgs { file_path, content })
}

fn split_shell_like(input: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;

    for character in input.chars() {
        if Some(character) == quote {
            quote = None;
        } else if quote.is_none() && matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if quote.is_none() && character.is_whitespace() {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts
}
