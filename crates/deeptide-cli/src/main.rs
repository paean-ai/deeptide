use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use clap::{ArgAction, Parser, ValueEnum};
use deeptide_core::config::ConfigStore;
use deeptide_core::embedded_protocol::{EmbeddedProtocol, EmbeddedProtocolSpec};
use deeptide_core::permissions::PermissionMode;
use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentTerminalEvent, AnthropicBackend, AnthropicConfig,
    CommandCompletionSource, CompletionEngine, LocalEchoBackend, ModelPricing, ReplEvent,
    ReplSession, StreamingEvent, StreamingHandler, ThinkingConfig, tui,
};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context, Editor, Helper};

const DEFAULT_MODEL: &str = "deepseek-v4-pro";
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

struct ConfiguredBackend {
    backend: Box<dyn AgentBackend>,
    model: String,
    is_configured: bool,
    subagent_config: Option<AnthropicConfig>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
enum InputFormat {
    Text,
    StreamJson,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    StreamJson,
}

#[derive(Debug, Parser)]
#[command(
    name = "deeptide",
    version,
    about = "Cross-platform Rust implementation of the Deeptide CLI.",
    long_about = "Cross-platform Rust implementation of the Deeptide CLI.\n\nThis workspace is under active parity development against the Swift Deeptide app. The current Rust increment establishes slash-command, permission, and embedded protocol parity slices."
)]
struct Cli {
    #[arg(short = 'p', long, value_name = "TEXT")]
    prompt: Option<String>,

    #[arg(long = "print", action = ArgAction::SetTrue)]
    print_mode: bool,

    #[arg(long, value_enum, default_value_t = InputFormat::Text)]
    input_format: InputFormat,

    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    output_format: OutputFormat,

    #[arg(long, action = ArgAction::SetTrue)]
    embedded: bool,

    #[arg(long)]
    session_id: Option<String>,

    #[arg(long)]
    cwd: Option<PathBuf>,

    #[arg(
        long,
        default_value = "default",
        help = "Permission mode: default, accept-edits, plan, bypass."
    )]
    permission_mode: String,

    #[arg(long, env = "DEEPTIDE_MODEL", default_value = DEFAULT_MODEL)]
    model: String,

    #[arg(long, env = "DEEPTIDE_BASE_URL", default_value = DEFAULT_BASE_URL)]
    base_url: String,

    #[arg(long, env = "DEEPTIDE_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    #[arg(
        long,
        env = "DEEPTIDE_PROFILE",
        value_name = "NAME",
        help = "Active provider profile from settings.json `providers`. Falls back to TIDE_PROFILE, then the config's active_profile."
    )]
    profile: Option<String>,

    #[arg(
        long = "fallback-model",
        env = "DEEPTIDE_FALLBACK_MODEL",
        value_name = "MODEL",
        help = "Model to retry with once when the primary model is transiently overloaded (HTTP 529/503)."
    )]
    fallback_model: Option<String>,

    #[arg(
        long,
        env = "DEEPTIDE_THINKING",
        value_name = "LEVEL",
        help = "Extended thinking: low, medium/enabled, high, disabled, or auto (omit to let the provider decide)."
    )]
    thinking: Option<String>,

    #[arg(
        long,
        env = "DEEPTIDE_EFFORT",
        value_name = "LEVEL",
        help = "Reasoning effort (low, medium, high); used when --thinking is unset."
    )]
    effort: Option<String>,

    #[arg(long, default_value_t = 4096)]
    max_output_tokens: usize,

    #[arg(
        long,
        env = "DEEPTIDE_MAX_TURNS",
        default_value_t = 25,
        help = "Safety cap on agentic turns per prompt."
    )]
    max_turns: usize,

    #[arg(
        long = "system-prompt",
        env = "DEEPTIDE_SYSTEM_PROMPT",
        value_name = "TEXT",
        help = "Optional system prompt sent on every model request. Cached server-side when prompt caching is enabled."
    )]
    system_prompt: Option<String>,

    #[arg(
        long = "system-prompt-file",
        env = "DEEPTIDE_SYSTEM_PROMPT_FILE",
        value_name = "PATH",
        help = "Read the system prompt from a file. Overrides --system-prompt when both are supplied.",
        conflicts_with = "system_prompt"
    )]
    system_prompt_file: Option<PathBuf>,

    #[arg(
        long = "append-system-prompt",
        env = "DEEPTIDE_APPEND_SYSTEM_PROMPT",
        value_name = "TEXT",
        help = "Text appended after the base system prompt (project + memory, or --system-prompt). Combine with the default prompt rather than replacing it."
    )]
    append_system_prompt: Option<String>,

    #[arg(
        long = "no-prompt-cache",
        env = "DEEPTIDE_NO_PROMPT_CACHE",
        action = ArgAction::SetTrue,
        help = "Disable Anthropic prompt caching of system prompt and tool schemas."
    )]
    no_prompt_cache: bool,

    #[arg(
        long = "no-color",
        env = "DEEPTIDE_NO_COLOR",
        action = ArgAction::SetTrue,
        help = "Disable ANSI color output. Also honored via the NO_COLOR environment variable and settings.json `no_color`."
    )]
    no_color: bool,

    #[arg(
        long,
        env = "DEEPTIDE_DEBUG",
        action = ArgAction::SetTrue,
        help = "Start the REPL with debug diagnostics (per-turn token/cost) enabled. Toggle at runtime with /debug."
    )]
    debug: bool,

    #[arg(
        long,
        env = "DEEPTIDE_FAST",
        action = ArgAction::SetTrue,
        help = "Fast mode: same model, biased toward faster/terser output (adds a system-prompt hint)."
    )]
    fast: bool,

    #[arg(
        long = "stream",
        env = "DEEPTIDE_STREAM",
        action = ArgAction::SetTrue,
        help = "Request streamed SSE responses from the Anthropic Messages API. Required by some proxy providers (openrouter, custom relays)."
    )]
    stream: bool,

    #[arg(
        long = "allowed-tools",
        value_name = "NAMES",
        help = "Restrict the agent to these tools (comma-separated). Only the listed tools are advertised and callable."
    )]
    allowed_tools: Option<String>,

    #[arg(
        long = "disallowed-tools",
        value_name = "NAMES",
        help = "Forbid these tools (comma-separated). Takes precedence over --allowed-tools."
    )]
    disallowed_tools: Option<String>,

    #[arg(
        short = 'y',
        long = "yolo",
        action = ArgAction::SetTrue,
        help = "Bypass all permission checks (equivalent to --permission-mode bypass)."
    )]
    yolo: bool,

    #[arg(
        short = 'c',
        long = "continue",
        action = ArgAction::SetTrue,
        help = "Resume the most recent session in this directory."
    )]
    continue_session: bool,

    #[arg(
        short = 'r',
        long = "resume",
        value_name = "SESSION_ID",
        help = "Resume a saved session by id, loading its history into context."
    )]
    resume: Option<String>,

    #[arg(
        long = "list-sessions",
        action = ArgAction::SetTrue,
        help = "List saved sessions for this directory and exit (no API key required)."
    )]
    list_sessions: bool,

    #[arg(
        long = "no-session-persistence",
        env = "DEEPTIDE_NO_SESSION_PERSISTENCE",
        action = ArgAction::SetTrue,
        help = "Do not autosave conversation turns to disk (privacy / scratch sessions)."
    )]
    no_session_persistence: bool,

    #[arg(
        long = "settings",
        value_name = "PATH",
        help = "Merge an explicit settings.json file on top of the global/project/local scopes."
    )]
    settings: Option<PathBuf>,

    #[arg(
        long = "add-dir",
        value_name = "PATH",
        action = ArgAction::Append,
        help = "Register an additional directory in the session context (repeatable), like /add-dir in the REPL."
    )]
    add_dir: Vec<PathBuf>,
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run(mut cli: Cli) -> Result<(), String> {
    normalize_embedded_mode(&mut cli);

    if let Some(cwd) = cli.cwd.as_ref() {
        std::env::set_current_dir(cwd)
            .map_err(|error| format!("invalid --cwd {}: {error}", cwd.display()))?;
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // --list-sessions is a no-API-key early exit: print the session list and stop.
    if cli.list_sessions {
        println!(
            "{}",
            format_session_list(&deeptide_core::SessionStore::list(&cwd))
        );
        return Ok(());
    }

    // Load settings.json (global ← project ← local), plus an explicit
    // --settings file on top, then apply as fallbacks. Explicit CLI flags and
    // environment variables always take precedence.
    if let Some(path) = cli.settings.as_ref()
        && !path.exists()
    {
        return Err(format!("--settings file not found: {}", path.display()));
    }
    let cfg = ConfigStore::load_with_override(&cwd, cli.settings.as_deref());
    cfg.apply_env();
    apply_config_fallbacks(&mut cli, &cfg);

    let permission_mode = if cli.yolo {
        // --yolo is a convenience alias for the bypass permission mode.
        PermissionMode::Bypass
    } else {
        match PermissionMode::parse(&cli.permission_mode) {
            Some(mode) => mode,
            None => return Err(format!("invalid permission mode: {}", cli.permission_mode)),
        }
    };

    validate_formats(&cli)?;

    // Lifecycle hooks (settings.json `hooks`) fire around the agent loop in
    // both interactive and print modes; PreToolUse hooks can block a tool.
    let hooks = deeptide_core::HookEngine::new(cfg.hooks.clone().unwrap_or_default(), &cwd);

    if !cli.print_mode && cli.input_format == InputFormat::Text {
        // Per-model pricing overrides from settings.json, converted to the
        // per-token rates the cost tracker consumes. Only the interactive REPL
        // surfaces cost (`/cost`, status line), so print mode skips this.
        let pricing_overrides = cfg.pricing_overrides();
        return run_interactive(&cli, permission_mode, pricing_overrides, hooks);
    }

    let stdin = read_stdin_if_needed(&cli)?;
    let prompt = collect_prompt(&cli, stdin.as_deref())?;
    emit_output(&cli, &prompt, permission_mode, &cwd, hooks)
}

/// Apply `settings.json` values as fallbacks for CLI fields that were not
/// explicitly set by the user.  CLI flags and env vars always win.
fn apply_config_fallbacks(cli: &mut Cli, cfg: &deeptide_core::ConfigData) {
    // Resolve the active provider profile. The profile name comes from
    // --profile (or DEEPTIDE_PROFILE via clap), then TIDE_PROFILE, then the
    // config's active_profile. A selected provider supplies model/base_url/
    // api_key that take precedence over the top-level config fields, matching
    // the Swift implementation's `provider ?? fileConfig` resolution order.
    let profile_name = cli
        .profile
        .clone()
        .filter(|name| !name.is_empty())
        .or_else(|| env_first_non_empty(&["TIDE_PROFILE"]));
    let provider = cfg.active_provider(profile_name.as_deref()).map(|(_, p)| p);
    let cfg_model = provider
        .and_then(|p| p.model.as_ref())
        .or(cfg.model.as_ref());
    let cfg_base_url = provider
        .and_then(|p| p.base_url.as_ref())
        .or(cfg.base_url.as_ref());
    let cfg_api_key = provider
        .and_then(|p| p.api_key.as_ref())
        .or(cfg.api_key.as_ref());

    // model: CLI default is DEFAULT_MODEL; treat that as "not set" and let
    // config override it.
    if cli.model == DEFAULT_MODEL
        && let Some(m) = cfg_model
    {
        cli.model = m.clone();
    }
    if cli.base_url == DEFAULT_BASE_URL
        && let Some(u) = cfg_base_url
    {
        cli.base_url = u.clone();
    }
    if cli.max_turns == 25
        && let Some(t) = cfg.max_turns
    {
        cli.max_turns = t;
    }
    if cli.max_output_tokens == 4096
        && let Some(t) = cfg.max_tokens
    {
        cli.max_output_tokens = t;
    }
    if cli.permission_mode == "default"
        && let Some(ref m) = cfg.permission_mode
    {
        cli.permission_mode = m.clone();
    }
    if cli.api_key.is_none()
        && let Some(key) = cfg_api_key
    {
        cli.api_key = Some(key.clone());
    }
    if cli.fallback_model.is_none()
        && let Some(fallback) = cfg.fallback_model.as_ref()
    {
        cli.fallback_model = Some(fallback.clone());
    }
    if cli.thinking.is_none()
        && let Some(thinking) = cfg.thinking.as_ref()
    {
        cli.thinking = Some(thinking.clone());
    }
    if cli.effort.is_none()
        && let Some(effort) = cfg.effort.as_ref()
    {
        cli.effort = Some(effort.clone());
    }
    if let Some(false) = cfg.prompt_cache {
        cli.no_prompt_cache = true;
    }
    if let Some(true) = cfg.no_color {
        cli.no_color = true;
    }
    if let Some(true) = cfg.debug {
        cli.debug = true;
    }
    if let Some(true) = cfg.fast_mode {
        cli.fast = true;
    }
}

/// Whether ANSI color output should be emitted. Color is disabled by the
/// `--no-color` flag (which also absorbs settings.json `no_color` and the
/// `DEEPTIDE_NO_COLOR` env var) or by the conventional `NO_COLOR` env var.
fn use_color(cli: &Cli) -> bool {
    !(cli.no_color || std::env::var_os("NO_COLOR").is_some())
}

fn normalize_embedded_mode(cli: &mut Cli) {
    if cli.embedded {
        cli.print_mode = true;
        cli.input_format = InputFormat::StreamJson;
        cli.output_format = OutputFormat::StreamJson;
    }
}

fn validate_formats(cli: &Cli) -> Result<(), String> {
    if cli.output_format != OutputFormat::Text && !cli.print_mode {
        return Err(format!(
            "--output-format={} requires --print",
            cli.output_format.as_ref()
        ));
    }

    if cli.input_format == InputFormat::StreamJson {
        if !cli.print_mode || cli.output_format != OutputFormat::StreamJson {
            return Err(
                "--input-format=stream-json requires --print --output-format=stream-json"
                    .to_owned(),
            );
        }
        if cli.prompt.is_some() {
            return Err(
                "--input-format=stream-json reads prompts from stdin; omit --prompt/-p".to_owned(),
            );
        }
    }

    Ok(())
}

fn collect_prompt(cli: &Cli, stdin: Option<&str>) -> Result<String, String> {
    match cli.input_format {
        InputFormat::StreamJson => {
            let raw = stdin.unwrap_or_default();
            let prompt = EmbeddedProtocol::parse_prompt(raw).map_err(|error| error.to_string())?;
            if prompt.trim().is_empty() {
                return Err("no user messages found in stream-json stdin".to_owned());
            }
            Ok(prompt)
        }
        InputFormat::Text => {
            if let Some(prompt) = cli.prompt.as_ref() {
                return Ok(prompt.clone());
            }

            if cli.print_mode {
                let prompt = stdin.unwrap_or_default();
                if prompt.trim().is_empty() {
                    return Err(
                        "no prompt provided and stdin is empty; use --prompt or pipe input"
                            .to_owned(),
                    );
                }
                return Ok(prompt.trim().to_owned());
            }

            Err("interactive REPL mode requires a terminal; use --print or --embedded".to_owned())
        }
    }
}

/// Rustyline helper that provides tab-completion for `/command` prefixes
/// using `CompletionEngine` and the REPL's registered command list.
struct ReplHelper {
    commands: Vec<CommandCompletionSource>,
    use_color: bool,
}

impl ReplHelper {
    fn new(commands: Vec<CommandCompletionSource>, use_color: bool) -> Self {
        Self {
            commands,
            use_color,
        }
    }
}

impl Helper for ReplHelper {}
impl Highlighter for ReplHelper {}

impl Validator for ReplHelper {
    fn validate(&self, ctx: &mut ValidationContext<'_>) -> rustyline::Result<ValidationResult> {
        // Allow multi-line continuation when the line ends with a backslash.
        // The user types `hello \<Enter>` and rustyline asks for more input.
        if ctx.input().trim_end().ends_with('\\') {
            Ok(ValidationResult::Incomplete)
        } else {
            Ok(ValidationResult::Valid(None))
        }
    }
    fn validate_while_typing(&self) -> bool {
        false
    }
}

impl Hinter for ReplHelper {
    type Hint = String;
    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Completer for ReplHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let Some(result) = CompletionEngine::command_completions(line, pos, &self.commands, 8)
        else {
            return Ok((pos, vec![]));
        };

        let pairs = result
            .candidates
            .iter()
            .map(|c| {
                let repl = c.replacement();
                let display = if self.use_color {
                    format!("{repl:<26}\x1b[90m {}\x1b[0m", c.description)
                } else {
                    format!("{repl:<26} {}", c.description)
                };
                Pair {
                    display,
                    replacement: repl,
                }
            })
            .collect();

        Ok((result.token_start, pairs))
    }
}

/// Path to the persistent readline history file.
fn history_file_path() -> Option<PathBuf> {
    let base = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)?;
    Some(base.join(".deeptide").join("history"))
}

/// Fires `SessionEnd` hooks when dropped, so they run on every way the
/// interactive loop can exit (return, break, or error) without duplicating the
/// call at each site.
struct SessionEndGuard(deeptide_core::HookEngine);

impl Drop for SessionEndGuard {
    fn drop(&mut self) {
        if self.0.has_hooks(deeptide_core::HookEvent::SessionEnd) {
            let _ = self.0.run(deeptide_core::HookEvent::SessionEnd, None, None);
        }
    }
}

fn run_interactive(
    cli: &Cli,
    permission_mode: PermissionMode,
    pricing_overrides: HashMap<String, ModelPricing>,
    hooks: deeptide_core::HookEngine,
) -> Result<(), String> {
    let mut stdout = io::stdout();

    // SessionStart fires once when the interactive session begins; SessionEnd
    // fires on every exit path via the RAII guard below (normal exit, /exit,
    // Ctrl+D, or an error return). The Swift implementation models these events
    // but does not yet fire them; Rust completes the configured hook lifecycle.
    if hooks.has_hooks(deeptide_core::HookEvent::SessionStart) {
        let _ = hooks.run(deeptide_core::HookEvent::SessionStart, None, None);
    }
    let _session_end = SessionEndGuard(hooks.clone());

    // In interactive mode, always enable streaming so text appears live rather
    // than appearing all-at-once after the full response is assembled.  Track
    // whether anything was streamed so we can suppress the duplicate full-text
    // print from `ReplEvent::Output`.
    let use_color = use_color(cli);
    let did_stream: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let did_stream_handler = Arc::clone(&did_stream);
    let streaming_handler: StreamingHandler = Arc::new(move |event: &StreamingEvent| {
        if let StreamingEvent::TextDelta { delta, .. } = event {
            // Write the delta directly; no buffering so text appears immediately.
            let _ = io::stdout().write_all(delta.as_bytes());
            let _ = io::stdout().flush();
            did_stream_handler.store(true, Ordering::Relaxed);
        }
    });

    let configured = configured_backend_with_handler(cli, Some(streaming_handler))?;
    let is_configured = configured.is_configured;
    let (allowed_tools, disallowed_tools) = parse_tool_restrictions(
        cli.allowed_tools.as_deref(),
        cli.disallowed_tools.as_deref(),
    );
    let mut repl = ReplSession::new(configured.backend)
        .with_model(configured.model)
        .with_permission_mode(permission_mode)
        .with_max_turns(cli.max_turns)
        .with_pricing_overrides(pricing_overrides)
        .with_debug(cli.debug)
        .with_fast_mode(cli.fast)
        .with_hooks(hooks)
        .with_tool_restrictions(allowed_tools, disallowed_tools)
        .with_session_persistence(!cli.no_session_persistence)
        .with_additional_dirs(&cli.add_dir)
        .with_tps_store_dir(deeptide_core::tps::default_store_dir())
        .with_subagent_backend_factory(subagent_backend_factory(configured.subagent_config));
    if let Some(append) = cli.append_system_prompt.as_deref() {
        repl = repl.with_appended_system_prompt(append);
    }

    let rl_config = rustyline::config::Config::builder()
        .history_ignore_space(true)
        .completion_type(rustyline::config::CompletionType::List)
        .build();
    let helper = ReplHelper::new(repl.command_sources(), use_color);
    let mut rl: Editor<ReplHelper, rustyline::history::DefaultHistory> =
        Editor::with_config(rl_config).map_err(|error| error.to_string())?;
    rl.set_helper(Some(helper));

    let history_path = history_file_path();
    if let Some(ref path) = history_path {
        // Non-fatal if the file doesn't exist yet
        let _ = rl.load_history(path);
    }

    writeln!(stdout, "{}", repl.banner()).map_err(|error| error.to_string())?;
    writeln!(
        stdout,
        "Permission mode: {}. Type /help for commands, Ctrl+D to exit.",
        permission_mode.label()
    )
    .map_err(|error| error.to_string())?;
    if !is_configured {
        writeln!(
            stdout,
            "No API key configured; using local echo backend. Set DEEPTIDE_API_KEY or --api-key to call a model."
        )
        .map_err(|error| error.to_string())?;
    }

    // --resume / --continue: restore a prior conversation before the first prompt.
    let resume_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Some(session_id) = resolve_resume_id(cli, &resume_cwd) {
        let count = repl.resume_session(&session_id)?;
        writeln!(
            stdout,
            "Resumed session {session_id}: {count} message(s) restored."
        )
        .map_err(|error| error.to_string())?;
    }

    loop {
        // Print the status bar above the prompt line.
        writeln!(
            stdout,
            "{}",
            repl.status_line().render(terminal_width().unwrap_or(100))
        )
        .map_err(|error| error.to_string())?;
        stdout.flush().map_err(|error| error.to_string())?;

        let readline = rl.readline(&repl.prompt());
        match readline {
            Ok(line) => {
                let trimmed = line.trim().to_owned();
                if trimmed.is_empty() {
                    continue;
                }
                // Join multi-line continuation: "hello \\\nworld" → "hello world"
                let content = trimmed
                    .split("\\\n")
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(" ");
                if content.is_empty() {
                    continue;
                }

                if let Err(err) = rl.add_history_entry(content.as_str()) {
                    let _ = err;
                }

                // Reset the streaming flag before each submission.
                did_stream.store(false, Ordering::Relaxed);

                for event in repl.submit(&content) {
                    match event {
                        ReplEvent::Output(text) => {
                            if did_stream.swap(false, Ordering::Relaxed) {
                                // The streaming handler already printed the text
                                // incrementally.  Print a blank line to visually
                                // separate the streamed output from the next prompt.
                                writeln!(stdout).map_err(|error| error.to_string())?;
                            } else {
                                writeln!(
                                    stdout,
                                    "{}",
                                    tui::render_output_panel(
                                        &text,
                                        terminal_width().unwrap_or(100),
                                        use_color,
                                    )
                                )
                                .map_err(|error| error.to_string())?;
                            }
                        }
                        ReplEvent::Exit => {
                            if let Some(ref path) = history_path {
                                save_history(&mut rl, path);
                            }
                            return Ok(());
                        }
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl+C — echo ^C and continue (matches Swift/zero-cli behaviour)
                writeln!(stdout, "^C").map_err(|error| error.to_string())?;
                continue;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl+D on empty line — exit gracefully
                writeln!(stdout).map_err(|error| error.to_string())?;
                break;
            }
            Err(error) => return Err(error.to_string()),
        }
    }

    if let Some(ref path) = history_path {
        save_history(&mut rl, path);
    }
    Ok(())
}

fn save_history(rl: &mut Editor<ReplHelper, rustyline::history::DefaultHistory>, path: &PathBuf) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = rl.save_history(path);
}

fn terminal_width() -> Option<usize> {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 20)
}

fn emit_output(
    cli: &Cli,
    prompt: &str,
    permission_mode: PermissionMode,
    cwd: &Path,
    hooks: deeptide_core::HookEngine,
) -> Result<(), String> {
    let response = run_prompt(cli, prompt, permission_mode, cwd, hooks)?;

    match cli.output_format {
        OutputFormat::Text => {
            println!(
                "{}",
                tui::render_output_panel(
                    &response,
                    terminal_width().unwrap_or(100),
                    use_color(cli),
                )
            );
        }
        OutputFormat::Json => {
            let body = serde_json::json!({
                "prompt": prompt,
                "response": response,
                "input_format": cli.input_format.as_ref(),
                "output_format": cli.output_format.as_ref(),
                "embedded": cli.embedded,
                "session_id": cli.session_id,
                "permission_mode": permission_mode.label(),
                "model": cli.model,
                "max_turns": cli.max_turns,
            });
            println!(
                "{}",
                serde_json::to_string_pretty(&body).map_err(|error| error.to_string())?
            );
        }
        OutputFormat::StreamJson => {
            println!(
                "{}",
                serde_json::to_string(&EmbeddedProtocolSpec::default())
                    .map_err(|error| error.to_string())?
            );
            println!(
                "{}",
                serde_json::to_string(&serde_json::json!({
                    "type": "result",
                    "subtype": "assistant_response",
                    "prompt": prompt,
                    "response": response,
                    "session_id": cli.session_id,
                    "permission_mode": permission_mode.label(),
                    "model": cli.model,
                    "max_turns": cli.max_turns,
                }))
                .map_err(|error| error.to_string())?
            );
        }
    }

    Ok(())
}

fn run_prompt(
    cli: &Cli,
    prompt: &str,
    permission_mode: PermissionMode,
    cwd: &Path,
    hooks: deeptide_core::HookEngine,
) -> Result<String, String> {
    let configured = configured_backend(cli)?;
    let (allowed_tools, disallowed_tools) = parse_tool_restrictions(
        cli.allowed_tools.as_deref(),
        cli.disallowed_tools.as_deref(),
    );
    let mut loop_ = AgentLoop::new(configured.backend)
        .with_model(configured.model)
        .with_permission_mode(permission_mode)
        .with_max_turns(cli.max_turns)
        .with_hooks(hooks)
        .with_tool_restrictions(allowed_tools, disallowed_tools)
        .with_subagent_backend_factory(subagent_backend_factory(configured.subagent_config));

    // Give print mode the same project context (CLAUDE.md/TIDE.md/AGENTS.md +
    // memory) as the interactive REPL. An explicit --system-prompt flag still
    // wins: it is applied to the backend config and used when the loop carries
    // no system prompt of its own.
    if let Some(system_prompt) = print_mode_system_prompt(cli, cwd)? {
        loop_ = loop_.with_system_prompt(system_prompt);
    }

    // --resume / --continue: run the prompt on top of a prior conversation.
    if let Some(session_id) = resolve_resume_id(cli, cwd) {
        let messages = deeptide_core::SessionStore::load(cwd, &session_id)?;
        loop_.restore_messages(messages);
    }

    let events = loop_.run(prompt);
    let mut assistant = None;
    for event in events {
        match event {
            AgentLoopEvent::Assistant(message) => assistant = Some(message.content),
            AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(error)) => return Err(error),
            AgentLoopEvent::Terminal(AgentTerminalEvent::MaxTurnsReached) => {
                return Err(String::from("maximum turns reached"));
            }
            AgentLoopEvent::Terminal(AgentTerminalEvent::Blocked) => {
                return Err(String::from(
                    "context window full: transcript exceeds the model's limit even after compaction",
                ));
            }
            AgentLoopEvent::User(_)
            | AgentLoopEvent::ToolBatchSummary { .. }
            | AgentLoopEvent::ToolResult { .. }
            | AgentLoopEvent::Compaction(_)
            | AgentLoopEvent::Terminal(AgentTerminalEvent::Complete) => {}
        }
    }

    assistant.ok_or_else(|| String::from("model returned no assistant message"))
}

fn configured_backend(cli: &Cli) -> Result<ConfiguredBackend, String> {
    configured_backend_with_handler(cli, None)
}

/// Build the configured backend, optionally installing a streaming handler.
///
/// When `streaming_handler` is `Some`, the backend is forced into streaming
/// mode regardless of the `--stream` flag (the handler is only useful when
/// streaming is active).  Callers that supply a handler typically also want
/// to suppress re-printing the assembled text from `ReplEvent::Output` because
/// the handler already wrote it incrementally.
fn configured_backend_with_handler(
    cli: &Cli,
    streaming_handler: Option<StreamingHandler>,
) -> Result<ConfiguredBackend, String> {
    let credential = effective_credential(cli);
    let Some(credential) = credential else {
        return Ok(ConfiguredBackend {
            backend: Box::<LocalEchoBackend>::default(),
            model: String::from("unconfigured"),
            is_configured: false,
            subagent_config: None,
        });
    };

    let base_url = effective_base_url(cli);
    let model = effective_model(cli);
    let mut config = match credential {
        CloudCredential::ApiKey(api_key) => AnthropicConfig::new(base_url, api_key, model.clone()),
        CloudCredential::BearerToken(token) => {
            AnthropicConfig::new_with_bearer_token(base_url, token, model.clone())
        }
    };
    config.max_tokens = cli.max_output_tokens;
    config.enable_prompt_caching = !cli.no_prompt_cache;
    config.enable_streaming = cli.stream || streaming_handler.is_some();
    config.fallback_model = cli.fallback_model.clone();
    // --thinking takes precedence over --effort; both already absorb config
    // fallbacks. An unset/`auto` value leaves thinking omitted from requests.
    config.thinking = cli
        .thinking
        .as_deref()
        .or(cli.effort.as_deref())
        .and_then(ThinkingConfig::from_label);
    if let Some(system_prompt) = resolve_system_prompt(cli)? {
        config = config.with_system_prompt(system_prompt);
    }
    let mut backend = AnthropicBackend::new(config.clone())?;
    if let Some(handler) = streaming_handler {
        backend = backend.with_streaming_handler(handler);
    }
    Ok(ConfiguredBackend {
        backend: Box::new(backend),
        model,
        is_configured: true,
        subagent_config: Some(config),
    })
}

enum CloudCredential {
    ApiKey(String),
    BearerToken(String),
}

/// Read the effective system prompt from `--system-prompt-file` (preferred)
/// or `--system-prompt`. Whitespace-only files/values are treated as "no
/// prompt" — `AnthropicConfig::with_system_prompt` does the same upstream,
/// but reporting it here gives clearer error semantics for typo-ed paths.
/// Build the system prompt to install on the print-mode agent loop.
///
/// The base is the explicit `--system-prompt` / `--system-prompt-file` when
/// given, otherwise the project prompt (CLAUDE.md/memory) so non-interactive
/// runs match the interactive REPL. `--append-system-prompt` text, when set, is
/// appended after the base. Always returns `Some` so the loop carries the
/// combined prompt directly.
fn print_mode_system_prompt(cli: &Cli, cwd: &Path) -> Result<Option<String>, String> {
    let mut prompt = match resolve_system_prompt(cli)? {
        Some(explicit) => explicit,
        None => deeptide_core::build_system_prompt(cwd),
    };
    if let Some(append) = cli
        .append_system_prompt
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        prompt.push_str("\n\n");
        prompt.push_str(append);
    }
    Ok(Some(prompt))
}

/// Parse the `--allowed-tools` / `--disallowed-tools` comma-separated flags into
/// the form `AgentLoop::with_tool_restrictions` expects: an optional allowlist
/// and a denylist. Whitespace is trimmed and empty entries dropped.
fn parse_tool_restrictions(
    allowed: Option<&str>,
    disallowed: Option<&str>,
) -> (Option<Vec<String>>, Vec<String>) {
    fn split(value: &str) -> Vec<String> {
        value
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }
    let allowed = allowed.map(split).filter(|list| !list.is_empty());
    let disallowed = disallowed.map(split).unwrap_or_default();
    (allowed, disallowed)
}

/// Render the saved-session listing for `--list-sessions` (most-recent first).
fn format_session_list(entries: &[deeptide_core::SessionEntry]) -> String {
    if entries.is_empty() {
        return String::from(
            "No saved sessions for this project. Sessions are saved automatically on each turn.",
        );
    }
    let mut lines = vec![format!("Saved sessions ({}):", entries.len())];
    for entry in entries.iter().take(50) {
        let project = std::path::Path::new(&entry.cwd)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&entry.cwd);
        let preview = if entry.preview.trim().is_empty() {
            "(empty)"
        } else {
            entry.preview.trim()
        };
        lines.push(format!(
            "  {}  {}  \"{}\"  ({} messages)",
            entry.session_id, project, preview, entry.message_count
        ));
    }
    lines.push(String::new());
    lines.push(String::from("Use --resume <session-id> to resume one."));
    lines.join("\n")
}

/// Resolve which prior session (if any) to resume. `--resume <id>` passes the
/// id straight through; `--continue` selects the most recently updated session
/// in `cwd` (or `None` when there is none). Validation happens at load time.
fn resolve_resume_id(cli: &Cli, cwd: &Path) -> Option<String> {
    if let Some(id) = cli
        .resume
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        return Some(id.to_owned());
    }
    if cli.continue_session {
        return deeptide_core::SessionStore::list(cwd)
            .into_iter()
            .next()
            .map(|entry| entry.session_id);
    }
    None
}

fn resolve_system_prompt(cli: &Cli) -> Result<Option<String>, String> {
    if let Some(path) = cli.system_prompt_file.as_ref() {
        let content = std::fs::read_to_string(path)
            .map_err(|error| format!("--system-prompt-file {}: {error}", path.display()))?;
        if content.trim().is_empty() {
            return Ok(None);
        }
        return Ok(Some(content));
    }
    Ok(cli.system_prompt.clone())
}

fn effective_credential(cli: &Cli) -> Option<CloudCredential> {
    let api_key = cli.api_key.clone().or_else(|| {
        env_first_non_empty(&["ZERO_API_KEY", "ZERO_CLI_API_KEY", "ANTHROPIC_API_KEY"])
    });
    if let Some(api_key) = api_key.filter(|key| !key.trim().is_empty()) {
        return Some(CloudCredential::ApiKey(api_key));
    }

    if has_provider_base_url(cli) {
        return env_first_non_empty(&["ZERO_CLI_AUTH_TOKEN", "ANTHROPIC_AUTH_TOKEN"])
            .map(CloudCredential::BearerToken);
    }

    None
}

fn subagent_backend_factory(
    config: Option<AnthropicConfig>,
) -> impl Fn(&str) -> Box<dyn AgentBackend> + Send + Sync + 'static {
    move |model| {
        let Some(mut config) = config.clone() else {
            return Box::<LocalEchoBackend>::default();
        };
        config.model = model.to_owned();
        match AnthropicBackend::new(config) {
            Ok(backend) => Box::new(backend),
            Err(_) => Box::<LocalEchoBackend>::default(),
        }
    }
}

fn effective_base_url(cli: &Cli) -> String {
    if cli.base_url != DEFAULT_BASE_URL {
        return cli.base_url.clone();
    }

    env_first_non_empty(&["ZERO_CLI_BASE_URL", "ZERO_API_BASE", "ANTHROPIC_BASE_URL"])
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned())
}

fn has_provider_base_url(cli: &Cli) -> bool {
    cli.base_url != DEFAULT_BASE_URL
        || env_first_non_empty(&["ZERO_CLI_BASE_URL", "ZERO_API_BASE", "ANTHROPIC_BASE_URL"])
            .is_some()
}

fn effective_model(cli: &Cli) -> String {
    if cli.model != DEFAULT_MODEL {
        return cli.model.clone();
    }

    env_first_non_empty(&["ZERO_CLI_MODEL", "ANTHROPIC_MODEL"])
        .unwrap_or_else(|| DEFAULT_MODEL.to_owned())
}

fn env_first_non_empty(names: &[&str]) -> Option<String> {
    names.iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    })
}

fn read_stdin_if_needed(cli: &Cli) -> Result<Option<String>, String> {
    if cli.input_format == InputFormat::StreamJson || cli.print_mode && cli.prompt.is_none() {
        return read_stdin().map(Some);
    }

    Ok(None)
}

fn read_stdin() -> Result<String, String> {
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .map_err(|error| error.to_string())?;
    Ok(buf)
}

trait EnumValue {
    fn as_ref(&self) -> &'static str;
}

impl EnumValue for InputFormat {
    fn as_ref(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::StreamJson => "stream-json",
        }
    }
}

impl EnumValue for OutputFormat {
    fn as_ref(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::StreamJson => "stream-json",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Cli, DEFAULT_BASE_URL, DEFAULT_MODEL, InputFormat, OutputFormat, apply_config_fallbacks,
        collect_prompt, configured_backend, effective_base_url, effective_model,
        normalize_embedded_mode, use_color, validate_formats,
    };
    use clap::Parser;
    use deeptide_core::{AnthropicAuthMode, ConfigData, ProviderProfile, ThinkingConfig};
    use std::collections::HashMap;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn sample_cli() -> Cli {
        Cli {
            prompt: None,
            print_mode: false,
            input_format: InputFormat::Text,
            output_format: OutputFormat::Text,
            embedded: false,
            session_id: None,
            cwd: None,
            permission_mode: "default".to_owned(),
            model: "deepseek-v4-pro".to_owned(),
            base_url: "https://api.anthropic.com".to_owned(),
            api_key: None,
            profile: None,
            fallback_model: None,
            thinking: None,
            effort: None,
            max_output_tokens: 4096,
            max_turns: 25,
            system_prompt: None,
            system_prompt_file: None,
            append_system_prompt: None,
            no_prompt_cache: false,
            no_color: false,
            debug: false,
            fast: false,
            stream: false,
            allowed_tools: None,
            disallowed_tools: None,
            yolo: false,
            continue_session: false,
            resume: None,
            list_sessions: false,
            no_session_persistence: false,
            settings: None,
            add_dir: Vec::new(),
        }
    }

    fn env_guard() -> MutexGuard<'static, ()> {
        static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        ENV_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|error| error.into_inner())
    }

    fn clear_api_env() {
        unsafe {
            for name in [
                "ZERO_API_KEY",
                "ZERO_CLI_API_KEY",
                "ANTHROPIC_API_KEY",
                "ZERO_CLI_BASE_URL",
                "ZERO_API_BASE",
                "ANTHROPIC_BASE_URL",
                "ZERO_CLI_AUTH_TOKEN",
                "ANTHROPIC_AUTH_TOKEN",
                "ZERO_CLI_MODEL",
                "ANTHROPIC_MODEL",
            ] {
                std::env::remove_var(name);
            }
        }
    }

    #[test]
    fn config_fallbacks_apply_active_provider() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::remove_var("TIDE_PROFILE");
        }

        let mut cli = sample_cli();
        // Simulate "unset" CLI values so config/provider fallbacks apply.
        cli.model = DEFAULT_MODEL.to_owned();
        cli.base_url = DEFAULT_BASE_URL.to_owned();

        let cfg = ConfigData {
            active_profile: Some(String::from("deepseek")),
            base_url: Some(String::from("https://top-level.example")),
            providers: Some(HashMap::from([(
                String::from("deepseek"),
                ProviderProfile {
                    base_url: Some(String::from("https://api.deepseek.com/anthropic")),
                    api_key: Some(String::from("provider-key")),
                    model: Some(String::from("provider-model-xyz")),
                    ..Default::default()
                },
            )])),
            ..Default::default()
        };

        apply_config_fallbacks(&mut cli, &cfg);

        // The active provider's values win over the top-level config base_url.
        assert_eq!(cli.base_url, "https://api.deepseek.com/anthropic");
        assert_eq!(cli.model, "provider-model-xyz");
        assert_eq!(cli.api_key.as_deref(), Some("provider-key"));
    }

    #[test]
    fn config_fallbacks_explicit_profile_overrides_active_profile() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::remove_var("TIDE_PROFILE");
        }

        let mut cli = sample_cli();
        cli.model = DEFAULT_MODEL.to_owned();
        cli.base_url = DEFAULT_BASE_URL.to_owned();
        cli.profile = Some(String::from("anthropic"));

        let cfg = ConfigData {
            active_profile: Some(String::from("deepseek")),
            providers: Some(HashMap::from([
                (
                    String::from("deepseek"),
                    ProviderProfile {
                        base_url: Some(String::from("https://api.deepseek.com/anthropic")),
                        ..Default::default()
                    },
                ),
                (
                    String::from("anthropic"),
                    ProviderProfile {
                        base_url: Some(String::from("https://api.anthropic.com")),
                        ..Default::default()
                    },
                ),
            ])),
            ..Default::default()
        };

        apply_config_fallbacks(&mut cli, &cfg);
        assert_eq!(cli.base_url, "https://api.anthropic.com");
    }

    #[test]
    fn config_fallbacks_explicit_cli_flags_beat_provider() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::remove_var("TIDE_PROFILE");
        }

        let mut cli = sample_cli();
        // A user-supplied --base-url (non-default) must not be overridden.
        cli.base_url = "https://user-supplied.example".to_owned();

        let cfg = ConfigData {
            active_profile: Some(String::from("deepseek")),
            providers: Some(HashMap::from([(
                String::from("deepseek"),
                ProviderProfile {
                    base_url: Some(String::from("https://api.deepseek.com/anthropic")),
                    ..Default::default()
                },
            )])),
            ..Default::default()
        };

        apply_config_fallbacks(&mut cli, &cfg);
        assert_eq!(cli.base_url, "https://user-supplied.example");
    }

    #[test]
    fn use_color_respects_flag_and_env() {
        let _guard = env_guard();
        unsafe {
            std::env::remove_var("NO_COLOR");
        }

        let mut cli = sample_cli();
        // Default: color enabled.
        assert!(use_color(&cli));

        // --no-color flag disables color.
        cli.no_color = true;
        assert!(!use_color(&cli));

        // NO_COLOR env disables color even without the flag.
        cli.no_color = false;
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
        assert!(!use_color(&cli));
        unsafe {
            std::env::remove_var("NO_COLOR");
        }
    }

    #[test]
    fn config_no_color_folds_into_cli_flag() {
        let _guard = env_guard();
        unsafe {
            std::env::remove_var("TIDE_PROFILE");
        }
        let mut cli = sample_cli();
        assert!(!cli.no_color);

        let cfg = ConfigData {
            no_color: Some(true),
            ..Default::default()
        };
        apply_config_fallbacks(&mut cli, &cfg);
        assert!(cli.no_color);
    }

    #[test]
    fn config_debug_folds_into_cli_flag() {
        let _guard = env_guard();
        unsafe {
            std::env::remove_var("TIDE_PROFILE");
        }
        let mut cli = sample_cli();
        assert!(!cli.debug);

        let cfg = ConfigData {
            debug: Some(true),
            ..Default::default()
        };
        apply_config_fallbacks(&mut cli, &cfg);
        assert!(cli.debug);
    }

    #[test]
    fn configured_backend_resolves_thinking_from_flag() {
        let _guard = env_guard();
        clear_api_env();
        let mut cli = sample_cli();
        cli.api_key = Some("k".to_owned());
        cli.thinking = Some("high".to_owned());

        let config = configured_backend(&cli)
            .expect("backend")
            .subagent_config
            .expect("config");
        assert_eq!(config.thinking, Some(ThinkingConfig::high()));
    }

    #[test]
    fn configured_backend_uses_effort_when_thinking_unset() {
        let _guard = env_guard();
        clear_api_env();
        let mut cli = sample_cli();
        cli.api_key = Some("k".to_owned());
        cli.effort = Some("low".to_owned());

        let config = configured_backend(&cli)
            .expect("backend")
            .subagent_config
            .expect("config");
        assert_eq!(config.thinking, Some(ThinkingConfig::low()));
    }

    #[test]
    fn configured_backend_omits_thinking_for_auto() {
        let _guard = env_guard();
        clear_api_env();
        let mut cli = sample_cli();
        cli.api_key = Some("k".to_owned());
        cli.thinking = Some("auto".to_owned());

        let config = configured_backend(&cli)
            .expect("backend")
            .subagent_config
            .expect("config");
        assert_eq!(config.thinking, None);
    }

    #[test]
    fn embedded_mode_forces_stream_json_printing() {
        let mut cli = sample_cli();
        cli.embedded = true;

        normalize_embedded_mode(&mut cli);

        assert!(cli.print_mode);
        assert_eq!(cli.input_format, InputFormat::StreamJson);
        assert_eq!(cli.output_format, OutputFormat::StreamJson);
    }

    #[test]
    fn stream_json_requires_print_and_stream_json_output() {
        let mut cli = sample_cli();
        cli.input_format = InputFormat::StreamJson;

        let error = validate_formats(&cli)
            .expect_err("stream-json input should require stream-json print output");

        assert_eq!(
            error,
            "--input-format=stream-json requires --print --output-format=stream-json"
        );
    }

    #[test]
    fn stream_json_rejects_explicit_prompt_flag() {
        let mut cli = sample_cli();
        cli.print_mode = true;
        cli.input_format = InputFormat::StreamJson;
        cli.output_format = OutputFormat::StreamJson;
        cli.prompt = Some("hello".to_owned());

        let error = validate_formats(&cli).expect_err("stream-json input should reject --prompt");

        assert_eq!(
            error,
            "--input-format=stream-json reads prompts from stdin; omit --prompt/-p"
        );
    }

    #[test]
    fn print_mode_text_input_trims_stdin_prompt() {
        let mut cli = sample_cli();
        cli.print_mode = true;

        let prompt = collect_prompt(&cli, Some("  hello from stdin\n"))
            .unwrap_or_else(|error| panic!("stdin prompt should be accepted: {error}"));

        assert_eq!(prompt, "hello from stdin");
    }

    #[test]
    fn collects_prompt_from_stream_json_stdin() {
        let mut cli = sample_cli();
        cli.print_mode = true;
        cli.input_format = InputFormat::StreamJson;
        cli.output_format = OutputFormat::StreamJson;

        let prompt = collect_prompt(
            &cli,
            Some("{\"type\":\"user\",\"text\":\"first\"}\n{\"type\":\"user_text\",\"text\":\"second\"}\n"),
        )
        .unwrap_or_else(|error| panic!("stream-json prompt should parse: {error}"));

        assert_eq!(prompt, "first\n\nsecond");
    }

    #[test]
    fn parses_max_turns_option() {
        let cli = Cli::try_parse_from(["deeptide", "--max-turns", "7", "--print", "-p", "hello"])
            .expect("max turns option should parse");

        assert_eq!(cli.max_turns, 7);
    }

    #[test]
    fn zero_cli_api_env_takes_priority_for_cloud_backend() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", "anthropic-key");
            std::env::set_var("ZERO_CLI_API_KEY", "zero-cli-key");
            std::env::set_var("ZERO_CLI_BASE_URL", "https://zero.example.test");
            std::env::set_var("ZERO_CLI_MODEL", "zero-model");
        }

        let cli = sample_cli();
        let configured = configured_backend(&cli).expect("configured backend");

        assert!(configured.is_configured);
        let subagent_config = configured.subagent_config.expect("subagent config");
        assert_eq!(configured.model, "zero-model");
        assert_eq!(subagent_config.model, "zero-model");
        assert_eq!(subagent_config.base_url, "https://zero.example.test");
        assert_eq!(subagent_config.api_key, "zero-cli-key");
        assert_eq!(subagent_config.auth_mode, AnthropicAuthMode::ApiKey);

        clear_api_env();
    }

    #[test]
    fn zero_cli_auth_token_uses_bearer_auth_with_provider_base_url() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::set_var("ZERO_CLI_AUTH_TOKEN", "provider-token");
            std::env::set_var("ZERO_CLI_BASE_URL", "https://provider.example.test");
        }

        let cli = sample_cli();
        let configured = configured_backend(&cli).expect("configured backend");

        assert!(configured.is_configured);
        let subagent_config = configured.subagent_config.expect("subagent config");
        assert_eq!(subagent_config.api_key, "provider-token");
        assert_eq!(subagent_config.base_url, "https://provider.example.test");
        assert_eq!(subagent_config.auth_mode, AnthropicAuthMode::BearerToken);

        clear_api_env();
    }

    #[test]
    fn zero_cli_auth_token_without_provider_base_url_falls_back_to_local() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::set_var("ZERO_CLI_AUTH_TOKEN", "provider-token");
        }

        let cli = sample_cli();
        let configured = configured_backend(&cli).expect("configured backend");

        assert!(!configured.is_configured);
        assert!(configured.subagent_config.is_none());

        clear_api_env();
    }

    #[test]
    fn explicit_deeptide_options_still_override_zero_cli_env() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::set_var("ZERO_CLI_BASE_URL", "https://zero.example.test");
            std::env::set_var("ZERO_CLI_MODEL", "zero-model");
        }
        let mut cli = sample_cli();
        cli.base_url = "https://deeptide.example.test".to_owned();
        cli.model = "deeptide-model".to_owned();

        assert_eq!(effective_base_url(&cli), "https://deeptide.example.test");
        assert_eq!(effective_model(&cli), "deeptide-model");

        clear_api_env();
    }

    #[test]
    fn defaults_apply_when_no_compatible_api_env_is_set() {
        let _guard = env_guard();
        clear_api_env();
        let cli = sample_cli();

        assert_eq!(effective_base_url(&cli), DEFAULT_BASE_URL);
        assert_eq!(effective_model(&cli), DEFAULT_MODEL);

        clear_api_env();
    }

    #[test]
    fn system_prompt_flag_reaches_anthropic_config() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        }
        let mut cli = sample_cli();
        cli.system_prompt = Some("you are deeptide".to_owned());

        let configured = configured_backend(&cli).expect("configured backend");
        let cfg = configured.subagent_config.expect("subagent config");
        assert_eq!(cfg.system_prompt.as_deref(), Some("you are deeptide"));
        assert!(cfg.enable_prompt_caching);

        clear_api_env();
    }

    #[test]
    fn no_prompt_cache_flag_disables_caching() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        }
        let mut cli = sample_cli();
        cli.no_prompt_cache = true;

        let configured = configured_backend(&cli).expect("configured backend");
        let cfg = configured.subagent_config.expect("subagent config");
        assert!(!cfg.enable_prompt_caching);

        clear_api_env();
    }

    #[test]
    fn stream_flag_flips_anthropic_config_enable_streaming() {
        let _guard = env_guard();
        clear_api_env();
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", "test-key");
        }
        let mut cli = sample_cli();
        cli.stream = true;

        let configured = configured_backend(&cli).expect("configured backend");
        let cfg = configured.subagent_config.expect("subagent config");
        assert!(cfg.enable_streaming, "--stream must enable streaming");

        clear_api_env();
    }

    #[test]
    fn system_prompt_file_takes_priority_over_inline_text() {
        use std::io::Write as _;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("prompt.txt");
        let mut file = std::fs::File::create(&path).expect("create prompt file");
        write!(file, "from-file body").expect("write");

        let mut cli = sample_cli();
        cli.system_prompt = Some("inline body".to_owned());
        cli.system_prompt_file = Some(path);

        let resolved = super::resolve_system_prompt(&cli)
            .expect("resolve should succeed")
            .expect("file body should resolve to Some");
        assert_eq!(resolved, "from-file body");
    }

    #[test]
    fn system_prompt_file_whitespace_only_resolves_to_none() {
        use std::io::Write as _;
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("blank.txt");
        let mut file = std::fs::File::create(&path).expect("create");
        write!(file, "   \n\t\n").expect("write");

        let mut cli = sample_cli();
        cli.system_prompt_file = Some(path);
        assert!(super::resolve_system_prompt(&cli).expect("ok").is_none());
    }

    #[test]
    fn parse_tool_restrictions_splits_trims_and_drops_empties() {
        let (allowed, disallowed) =
            super::parse_tool_restrictions(Some(" Read , Grep ,,"), Some("Bash"));
        assert_eq!(
            allowed,
            Some(vec![String::from("Read"), String::from("Grep")])
        );
        assert_eq!(disallowed, vec![String::from("Bash")]);
    }

    #[test]
    fn parse_tool_restrictions_absent_flags_yield_no_restriction() {
        let (allowed, disallowed) = super::parse_tool_restrictions(None, None);
        assert!(allowed.is_none());
        assert!(disallowed.is_empty());

        // An all-whitespace allowlist collapses to no allowlist rather than an
        // empty one that would forbid every tool.
        let (allowed, _) = super::parse_tool_restrictions(Some("  , "), None);
        assert!(allowed.is_none());
    }

    #[test]
    fn format_session_list_handles_empty_and_populated() {
        assert!(super::format_session_list(&[]).contains("No saved sessions"));

        let entry = deeptide_core::SessionEntry {
            session_id: "2024-session-abc".to_owned(),
            cwd: "/home/user/project".to_owned(),
            model: "deepseek-v4-pro".to_owned(),
            started_at: "2024-01-01T00:00:00Z".to_owned(),
            updated_at: "2024-01-01T00:05:00Z".to_owned(),
            preview: "fix the parser".to_owned(),
            message_count: 7,
        };
        let listing = super::format_session_list(&[entry]);
        assert!(listing.contains("Saved sessions (1):"));
        assert!(listing.contains("2024-session-abc"));
        assert!(listing.contains("project"));
        assert!(listing.contains("fix the parser"));
        assert!(listing.contains("7 messages"));
        assert!(listing.contains("Use --resume"));
    }

    #[test]
    fn print_mode_uses_project_prompt_when_no_flag() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cli = sample_cli();
        let resolved = super::print_mode_system_prompt(&cli, dir.path()).expect("ok");
        let prompt = resolved.expect("project prompt should be installed when no flag is set");
        assert!(
            !prompt.trim().is_empty(),
            "build_system_prompt always carries at least the identity preamble"
        );
    }

    #[test]
    fn print_mode_uses_explicit_system_prompt_flag() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut cli = sample_cli();
        cli.system_prompt = Some("custom override".to_owned());
        let prompt = super::print_mode_system_prompt(&cli, dir.path())
            .expect("ok")
            .expect("explicit prompt is installed on the loop");
        assert_eq!(prompt, "custom override");
        // The project identity preamble is replaced, not present.
        assert!(!prompt.contains("You are Deeptide"));
    }

    #[test]
    fn print_mode_appends_system_prompt_text() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut cli = sample_cli();
        cli.append_system_prompt = Some("Always reply in JSON.".to_owned());
        let prompt = super::print_mode_system_prompt(&cli, dir.path())
            .expect("ok")
            .expect("prompt installed");
        // The project prompt remains, with the appended directive after it.
        assert!(prompt.contains("You are Deeptide"));
        assert!(prompt.trim_end().ends_with("Always reply in JSON."));

        // Appends on top of an explicit --system-prompt too.
        let mut cli2 = sample_cli();
        cli2.system_prompt = Some("Base prompt.".to_owned());
        cli2.append_system_prompt = Some("Extra rule.".to_owned());
        let prompt2 = super::print_mode_system_prompt(&cli2, dir.path())
            .expect("ok")
            .expect("prompt installed");
        assert_eq!(prompt2, "Base prompt.\n\nExtra rule.");
    }

    #[test]
    fn system_prompt_file_missing_path_returns_clear_error() {
        let mut cli = sample_cli();
        cli.system_prompt_file = Some(std::path::PathBuf::from(
            "/nonexistent/deeptide-system-prompt.txt",
        ));
        let err = super::resolve_system_prompt(&cli).expect_err("missing path should error");
        assert!(
            err.contains("--system-prompt-file"),
            "error should name the flag: {err}"
        );
    }
}
