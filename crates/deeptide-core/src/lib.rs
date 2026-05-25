pub mod agent_loop;
pub mod api;
pub mod commands;
pub mod completion;
pub mod cost;
pub mod embedded_protocol;
pub mod memory;
pub mod permissions;
pub mod repl;
pub mod tool_result_summary;
pub mod tools;

pub use agent_loop::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentRequest, AgentResponse, AgentTerminalEvent,
    AgentUsage, ConversationMessage, LocalEchoBackend, MessageRole,
};
pub use api::{AnthropicBackend, AnthropicConfig};
pub use commands::{
    ClearCommand, CommandContext, CommandResult, CompactCommand, CostCommand, HelpCommand,
    MemoryCommand, NewCommand, RememberCommand, SlashCommand,
};
pub use completion::{
    CommandCompletionCandidate, CommandCompletionResult, CommandCompletionSource, CompletionEngine,
    Replacement,
};
pub use cost::{CacheHealth, CostSummary, CostTracker, ModelPricing, TurnRecord, TurnUsage};
pub use repl::{ReplEvent, ReplSession};
pub use tool_result_summary::ToolResultSummaryFormatter;
pub use tools::{ReadTool, Tool, ToolContext, ToolRegistry, ToolResult};
