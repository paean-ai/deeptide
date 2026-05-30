pub mod agent_loop;
pub mod api;
pub mod at_references;
pub mod background_shell;
pub mod checkpoints;
pub mod commands;
pub mod completion;
pub mod config;
pub mod context_window;
pub mod cost;
pub mod diff_preview;
pub mod embedded_protocol;
pub mod hooks;
pub mod markdown;
pub mod memory;
pub mod memory_capture;
pub mod memory_hygiene;
pub mod memory_rank;
pub mod message_queue;
pub mod permissions;
pub mod project_toolchain;
pub mod prompt;
pub mod repl;
pub mod routing;
pub mod safety_guard;
pub mod sensitive_file;
pub mod session;
pub mod streaming;
pub mod tool_batch_labeler;
pub mod tool_result_summary;
pub mod tool_usage;
pub mod tools;
pub mod tps;
pub mod tui;
pub mod width;

pub use agent_loop::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentRequest, AgentResponse, AgentTerminalEvent,
    AgentUsage, AskOutcome, ConversationMessage, LocalEchoBackend, MessageRole,
    PermissionAskCallback, ToolCall, ToolResultBlock,
};
pub use api::{AnthropicAuthMode, AnthropicBackend, AnthropicConfig, ThinkingConfig, ToolChoice};
pub use at_references::{
    AtExpansionOptions, AtReference, AttachedFile, ExpansionResult, ExpansionStatus, ImageKind,
    expand_at_references, parse_at_references,
};
pub use checkpoints::{Checkpoint, CheckpointStore, MAX_CHECKPOINTS, SelectorOutcome};
pub use commands::{
    ClearCommand, CommandContext, CommandResult, CompactCommand, CostCommand, ExitCommand,
    HelpCommand, MemoryCommand, NewCommand, RememberCommand, SlashCommand,
};
pub use completion::{
    CommandCompletionCandidate, CommandCompletionResult, CommandCompletionSource, CompletionEngine,
    Replacement, ValueCompletionResult,
};
pub use config::{
    ConfigData, ConfigScope, ConfigStore, HookEntry, McpServerConfig, ModelPricingConfig,
    ProviderProfile, SettingsHooks, SettingsPermissions, SettingsRule,
};
pub use context_window::{
    CompressionReport, ContextWindowConfig, ContextWindowManager, HeuristicSummarizer, Summarizer,
    estimate_tokens,
};
pub use cost::{
    CacheHealth, CostSummary, CostTracker, KnownModel, ModelPricing, TurnRecord, TurnUsage,
    known_models,
};
pub use diff_preview::{DiffPreview, DiffPreviewOptions, render_tool_call_diff};
pub use hooks::{HookEngine, HookEvent, HookOutcome};
pub use markdown::{MarkdownRenderOptions, MarkdownRenderer, StreamingMarkdownRenderer};
pub use message_queue::{MessageQueue, QueueMode};
pub use permissions::{
    PermissionDecision, PermissionManager, PermissionMode, PermissionRules, Rule, RuleResult,
    ToolInput,
};
pub use project_toolchain::{
    NodePackageManager, ProjectKind, PythonMarker, ToolchainCommand, detect_toolchains,
};
pub use prompt::build_system_prompt;
pub use repl::{ReplEvent, ReplSession, SystemMessage};
pub use routing::{EscalationPolicy, Route, RoutingBackend};
pub use safety_guard::{
    AuditInput, AuditReport, Auditor, CodeAuditor, Finding, SafetyGuard, ShellAuditor, Verdict,
};
pub use session::{SessionEntry, SessionStore, new_session_id};
pub use streaming::{
    STREAM_RETRY_SIGNAL_PREFIX, StreamError, StreamRetryNotice, StreamingEvent, StreamingHandler,
    parse_stream_retry_signal, parse_streaming_response,
};
pub use tool_batch_labeler::{ToolBatchFailureClassifier, ToolBatchItem, ToolBatchLabeler};
pub use tool_result_summary::ToolResultSummaryFormatter;
pub use tool_usage::{ToolUsageEntry, ToolUsageTracker};
pub use tools::{
    AgentTool, AppendFileTool, AskUserQuestionTool, AudioTranscribeTool, BashTool, BriefTool,
    ClipboardTool, CrashLogTool, CronCreateTool, CronDeleteTool, CronListTool, CtxInspectTool,
    EditTool, EnterPlanModeTool, EnterWorktreeTool, ExitPlanModeTool, ExitWorktreeTool,
    FileMetadataTool, GetMcpPromptTool, GlobTool, GrepTool, ImagePreprocessTool,
    ListMcpPromptsTool, ListMcpResourcesTool, LspTool, MacDiagnoseTool, MacLogTool, McpTool,
    MemorySearchTool, MemoryWriteTool, MonitorTool, NotebookEditTool, PublishTool,
    PushNotificationTool, ReadFilesTool, ReadMcpResourceTool, ReadTool, RemoteTriggerTool,
    ReviewArtifactTool, ScreenCaptureTool, SkillTool, SleepTool, SnipTool, SpotlightSearchTool,
    TaskCreateTool, TaskGetTool, TaskListTool, TaskOutputTool, TaskStopTool, TaskUpdateTool,
    TodoSummary, TodoWriteTool, Tool, ToolContext, ToolMetadata, ToolRegistry, ToolResult,
    ToolSearchTool, VerifyPlanExecutionTool, VideoTranscribeTool, VisionTool, WebFetchTool,
    WebSearchTool, WriteTool, todo_summary,
};
pub use tps::{TpsRecord, TpsSample};
pub use tui::{
    InputBar, InputLayout, StatusLine, StatusSegment, TranscriptItem, TranscriptKind, TuiFrame,
};
