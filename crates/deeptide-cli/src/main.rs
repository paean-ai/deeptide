use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use clap::{ArgAction, Parser, ValueEnum};
use deeptide_core::config::ConfigStore;
use deeptide_core::embedded_protocol::{EmbeddedProtocol, EmbeddedProtocolSpec};
use deeptide_core::permissions::PermissionMode;
use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentTerminalEvent, AnthropicBackend, AnthropicConfig,
    AskOutcome, CommandCompletionSource, CompletionEngine, LocalEchoBackend, MarkdownRenderOptions,
    ModelPricing, ReplEvent, ReplSession, StatusSegment, StreamingEvent, StreamingHandler,
    StreamingMarkdownRenderer, ThinkingConfig, ToolCall, tui,
};
use rustyline::completion::{Completer, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{
    Cmd, ConditionalEventHandler, Context, Editor, Event, EventContext, EventHandler, Helper,
    KeyCode, KeyEvent, Modifiers, RepeatCount,
};

mod status_bar;

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
        long = "list-models",
        action = ArgAction::SetTrue,
        help = "List models with built-in pricing data and exit (no API key required)."
    )]
    list_models: bool,

    #[arg(
        long = "doctor",
        action = ArgAction::SetTrue,
        help = "Print a diagnostic report (config paths, env, model, optional tooling) and exit."
    )]
    doctor: bool,

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

    // --list-models is another no-API-key early exit so users can discover
    // which models have built-in pricing data and which the CLI defaults
    // to, without provisioning an API key first.
    if cli.list_models {
        println!("{}", format_model_list(DEFAULT_MODEL));
        return Ok(());
    }

    // --doctor is a no-network, no-API-key diagnostic that surfaces the
    // configuration the CLI would use on a real run plus environment
    // health checks. Designed to be the first thing a user runs when
    // something doesn't behave the way they expect.
    if cli.doctor {
        println!("{}", run_doctor(&cli, &cwd));
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
    models: Vec<String>,
    use_color: bool,
}

impl ReplHelper {
    fn new(commands: Vec<CommandCompletionSource>, use_color: bool) -> Self {
        // Argument completion for `/model <name>`: the built-in catalog plus the
        // two shorthand aliases the command accepts.
        let mut models = vec![String::from("flash"), String::from("pro")];
        models.extend(
            deeptide_core::known_models()
                .into_iter()
                .map(|model| model.name.to_owned()),
        );
        Self {
            commands,
            models,
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
            // No command-name match: try completing a known command argument,
            // currently `/model <name>`.
            let model_refs: Vec<&str> = self.models.iter().map(String::as_str).collect();
            if let Some(values) =
                CompletionEngine::value_completions(line, pos, "model", &model_refs, 8)
            {
                let pairs = values
                    .candidates
                    .iter()
                    .map(|value| Pair {
                        display: value.clone(),
                        replacement: value.clone(),
                    })
                    .collect();
                return Ok((values.token_start, pairs));
            }
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

    // Activity spinner coordination. While a turn runs synchronously (model
    // thinking + tool execution), a background thread animates a spinner so the
    // terminal isn't silent. `spinner_stop` halts it; `output_started` records
    // that real output has taken over the current line (so neither the handler
    // nor the spinner's exit-clear erases streamed text); `spinner_lock`
    // serializes the two threads' writes to stdout.
    let spinner_stop: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    let output_started: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let spinner_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));

    let did_stream_handler = Arc::clone(&did_stream);
    let spinner_stop_handler = Arc::clone(&spinner_stop);
    let output_started_handler = Arc::clone(&output_started);
    let spinner_lock_handler = Arc::clone(&spinner_lock);
    // Line-buffered markdown renderer for streamed output. Sits inside an
    // `Arc<Mutex<…>>` because the streaming handler is invoked from whatever
    // thread the backend uses, and we may also need to drain (flush) it
    // from the REPL thread between turns.
    let streaming_md = Arc::new(Mutex::new(StreamingMarkdownRenderer::new(
        MarkdownRenderOptions { color: use_color },
    )));
    let streaming_md_handler = Arc::clone(&streaming_md);
    let streaming_handler: StreamingHandler = Arc::new(move |event: &StreamingEvent| {
        if let StreamingEvent::TextDelta { delta, .. } = event {
            let first = !did_stream_handler.swap(true, Ordering::Relaxed);
            // Stop the spinner under the lock so it cannot draw a frame over
            // the text we are about to print.
            if let Ok(_guard) = spinner_lock_handler.lock() {
                spinner_stop_handler.store(true, Ordering::Relaxed);
                output_started_handler.store(true, Ordering::Relaxed);
                let mut out = io::stdout();
                if first {
                    // Erase the spinner line before the first streamed token.
                    let _ = out.write_all(b"\r\x1b[2K");
                }
                // Apply incremental markdown rendering. Bold, lists, headers,
                // links, and inline code now style correctly even though the
                // model is emitting one chunk at a time. Code fences are
                // passed through verbatim while open (the body remains
                // readable as it streams in).
                let rendered = match streaming_md_handler.lock() {
                    Ok(mut renderer) => renderer.push(delta),
                    // If the mutex is poisoned, fall back to raw output so the
                    // user still sees the model response.
                    Err(_) => delta.clone(),
                };
                let _ = out.write_all(rendered.as_bytes());
                let _ = out.flush();
            }
        }
    });

    let configured = configured_backend_with_handler(cli, Some(streaming_handler))?;
    let is_configured = configured.is_configured;
    let (allowed_tools, disallowed_tools) = parse_tool_restrictions(
        cli.allowed_tools.as_deref(),
        cli.disallowed_tools.as_deref(),
    );
    // Permission ask callback. Invoked synchronously from inside the agent
    // loop whenever a tool requires interactive approval; reads y/n/a from
    // stdin and returns the user's choice. Built BEFORE the REPL so we can
    // pass it as a builder argument; uses the same spinner mutex so the
    // prompt isn't clobbered by an in-flight frame.
    let ask_spinner_stop = Arc::clone(&spinner_stop);
    let ask_spinner_lock = Arc::clone(&spinner_lock);
    let ask_output_started = Arc::clone(&output_started);
    let ask_use_color = use_color;
    let ask_callback: deeptide_core::PermissionAskCallback =
        Arc::new(move |tool_call: &ToolCall| {
            handle_permission_prompt(
                tool_call,
                &ask_spinner_lock,
                &ask_spinner_stop,
                &ask_output_started,
                ask_use_color,
            )
        });

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
        .with_subagent_backend_factory(subagent_backend_factory(configured.subagent_config))
        .with_ask_callback(ask_callback);
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

    // Shift+Tab: cycle the session permission mode. The conditional handler
    // sets `pending_mode_cycle` and asks rustyline to interrupt readline so
    // the REPL loop can observe the flag and react with visible feedback.
    let pending_mode_cycle: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    rl.bind_sequence(
        Event::KeySeq(vec![KeyEvent(KeyCode::BackTab, Modifiers::NONE)]),
        EventHandler::Conditional(Box::new(ShiftTabCycleHandler {
            pending: Arc::clone(&pending_mode_cycle),
        })),
    );

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
            "No API key configured; using local echo backend. Set DEEPTIDE_API_KEY (or ANTHROPIC_API_KEY / ZERO_API_KEY) or pass --api-key to call a model. Run `deeptide --doctor` to inspect the full resolution chain."
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

    // Anchor the status bar to the bottom row of the terminal so it doesn't
    // scroll with conversation content. Falls back (returns None) on non-
    // TTY stdout, `TERM=dumb`, or terminals too small to host the bar — in
    // which case the loop falls back to printing the bar inline above the
    // prompt every turn (the pre-anchored behaviour).
    let mut anchored = if use_color {
        status_bar::AnchoredStatusBar::try_engage()
    } else {
        None
    };

    // Pre-rendered styled prompt. Computed once: the raw `repl.prompt()`
    // value is stable for the lifetime of the session, and rustyline needs
    // `\x01...\x02` markers around invisible escape sequences so cursor
    // positioning stays correct (see status_bar::rustyline_safe).
    let raw_prompt = repl.prompt();
    let styled_prompt = if let Some(stripped) = raw_prompt.strip_suffix(' ') {
        format!(
            "{} ",
            status_bar::rustyline_safe(stripped, status_bar::palette::PROMPT, use_color)
        )
    } else {
        status_bar::rustyline_safe(&raw_prompt, status_bar::palette::PROMPT, use_color)
    };

    // Auth indicator state. `is_configured` (computed from
    // `effective_credential`) is the inference-side API key result and is
    // stable for the lifetime of the session. The Paean publish token is
    // probed each repaint because it's environmental and could in principle
    // change mid-session; the call is just env-var reads and is cheap.
    let api_key_resolved = is_configured;
    let mut auth_paint_tick: u64 = 0;

    loop {
        // Paint the status bar (anchored at row N, OR inline above the
        // prompt as a safe fallback when anchoring isn't available).
        let bar_width = anchored
            .as_ref()
            .map(status_bar::AnchoredStatusBar::cols)
            .unwrap_or_else(|| terminal_width().unwrap_or(100));
        let auth_segment =
            build_auth_segment(api_key_resolved, paean_token_resolved(), auth_paint_tick);
        auth_paint_tick = auth_paint_tick.wrapping_add(1);
        let bar_text = repl
            .status_line_with_auth(Some(auth_segment))
            .render(bar_width);
        let bar_styled = status_bar::dim(&bar_text, use_color);
        if let Some(bar) = anchored.as_mut() {
            bar.repaint(&bar_styled, &spinner_lock);
        } else {
            writeln!(stdout, "{bar_styled}").map_err(|error| error.to_string())?;
            stdout.flush().map_err(|error| error.to_string())?;
        }

        let readline = rl.readline(&styled_prompt);
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

                // Reset per-turn streaming + spinner state.
                did_stream.store(false, Ordering::Relaxed);
                output_started.store(false, Ordering::Relaxed);
                spinner_stop.store(false, Ordering::Relaxed);

                // Animate an activity spinner on a background thread while the
                // synchronous turn runs, so model thinking and tool execution
                // aren't silent. Skipped when color is off (plain/log output).
                let spinner_handle = if use_color {
                    let stop = Arc::clone(&spinner_stop);
                    let started = Arc::clone(&output_started);
                    let lock = Arc::clone(&spinner_lock);
                    Some(thread::spawn(move || {
                        run_spinner(&stop, &started, &lock, true)
                    }))
                } else {
                    None
                };

                let events = repl.submit(&content);

                // Halt the spinner and reclaim its line before printing results.
                spinner_stop.store(true, Ordering::Relaxed);
                if let Some(handle) = spinner_handle {
                    let _ = handle.join();
                }

                // Drain the streaming markdown buffer. The model often
                // produces a trailing line without a final newline; without
                // this flush that fragment would stay buffered and reappear
                // on the next turn's first delta with stale state. Resetting
                // the renderer also clears any fence state so a turn that
                // ended mid-code-block doesn't bleed into the next response.
                if let Ok(mut renderer) = streaming_md.lock() {
                    let trailing = renderer.flush();
                    if !trailing.is_empty() {
                        let _ = stdout.write_all(trailing.as_bytes());
                    }
                    *renderer =
                        StreamingMarkdownRenderer::new(MarkdownRenderOptions { color: use_color });
                }

                for event in events {
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
                // Shift+Tab piggy-backs on rustyline's Interrupt command to
                // break out of readline cleanly. If the cycle handler set the
                // flag, this isn't a real Ctrl+C — it's the user asking to
                // rotate the permission mode.
                if pending_mode_cycle.swap(false, Ordering::Relaxed) {
                    let next = next_permission_mode(repl.agent_loop().permission_mode());
                    repl.set_permission_mode(next);
                    let label = next.label();
                    let line = if use_color {
                        format!("  → mode \x1b[1m{label}\x1b[0m")
                    } else {
                        format!("  → mode {label}")
                    };
                    writeln!(stdout, "{line}").map_err(|error| error.to_string())?;
                    continue;
                }
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

/// Animate the activity spinner until `stop` is set, redrawing one line in
/// place. Writes are serialized with `lock` (shared with the streaming handler)
/// so a frame never interleaves with — or paints over — streamed text. The
/// spinner line is cleared on exit only when real output has not already taken
/// over the current line (`output_started`).
///
/// `color` controls whether the spinner line is wrapped in a dim ANSI SGR
/// pair so it visually recedes behind upcoming streamed text. The setting
/// must match the rest of the CLI's color policy (`use_color(cli)`) so a
/// piped or `--no-color` run never emits stray escapes.
fn run_spinner(stop: &AtomicBool, output_started: &AtomicBool, lock: &Mutex<()>, color: bool) {
    // Grace period so an instant response never flashes a spinner.
    thread::sleep(Duration::from_millis(150));
    let start = Instant::now();
    let mut tick = 0usize;
    while !stop.load(Ordering::Relaxed) {
        if let Ok(_guard) = lock.lock() {
            // Re-check under the lock: the handler flips `stop` while holding it,
            // so this guarantees we never draw after output has begun.
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let raw = tui::render_spinner_line(tick, start.elapsed().as_secs());
            let line = status_bar::dim(&raw, color);
            let mut out = io::stdout();
            let _ = write!(out, "\r{line}\x1b[K");
            let _ = out.flush();
        }
        tick = tick.wrapping_add(1);
        thread::sleep(Duration::from_millis(120));
    }
    if !output_started.load(Ordering::Relaxed)
        && let Ok(_guard) = lock.lock()
    {
        let mut out = io::stdout();
        let _ = write!(out, "\r\x1b[2K");
        let _ = out.flush();
    }
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
    match cli.output_format {
        OutputFormat::Text => {
            // Plain text propagates errors to stderr / non-zero exit.
            let response = run_prompt(cli, prompt, permission_mode, cwd, hooks, None)?.response;
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
            // Emit the result envelope (success or error) as a single JSON
            // object so machine consumers always get a parseable result.
            let outcome = run_prompt(cli, prompt, permission_mode, cwd, hooks, None);
            let body = one_shot_result_json(&outcome, &cli.model);
            println!(
                "{}",
                serde_json::to_string_pretty(&body).map_err(|error| error.to_string())?
            );
        }
        OutputFormat::StreamJson => {
            // Spec first, then assistant_delta events stream live during the
            // run, then the final result envelope — mirroring Swift's embedded
            // event sequence.
            println!(
                "{}",
                serde_json::to_string(&EmbeddedProtocolSpec::default())
                    .map_err(|error| error.to_string())?
            );
            let delta_handler: StreamingHandler = Arc::new(|event: &StreamingEvent| {
                if let StreamingEvent::TextDelta { delta, .. } = event {
                    let mut out = io::stdout().lock();
                    let _ = writeln!(out, "{}", assistant_delta_event(delta));
                    let _ = out.flush();
                }
            });
            let outcome = run_prompt(
                cli,
                prompt,
                permission_mode,
                cwd,
                hooks,
                Some(delta_handler),
            );
            let body = one_shot_result_json(&outcome, &cli.model);
            println!(
                "{}",
                serde_json::to_string(&body).map_err(|error| error.to_string())?
            );
        }
    }

    Ok(())
}

/// Returns the stderr warning that should be printed when a `--print`
/// run starts without a resolved credential. Returns `None` when no
/// warning is needed (either credentials resolved fine, or we're in
/// interactive REPL mode where the user already sees an in-band
/// banner). Extracted as a pure function so unit tests can pin the
/// trigger condition + the wording.
fn unconfigured_print_mode_warning(is_configured: bool, print_mode: bool) -> Option<String> {
    if is_configured || !print_mode {
        return None;
    }
    Some(String::from(
        "warning: no API key resolved; --print is using the local echo backend and will NOT call a model.\n         set DEEPTIDE_API_KEY / ANTHROPIC_API_KEY / ZERO_API_KEY (or pass --api-key), or run `deeptide --doctor` to inspect the resolution chain.",
    ))
}

/// The outcome of a one-shot (`--print`) run: the assistant's final text plus
/// the run's accumulated cost/token usage, used to build the result envelope.
struct PromptOutcome {
    response: String,
    cost: deeptide_core::CostSummary,
}

fn run_prompt(
    cli: &Cli,
    prompt: &str,
    permission_mode: PermissionMode,
    cwd: &Path,
    hooks: deeptide_core::HookEngine,
    streaming_handler: Option<StreamingHandler>,
) -> Result<PromptOutcome, String> {
    let configured = configured_backend_with_handler(cli, streaming_handler)?;

    // In one-shot (`--print`) mode the user can't see the interactive
    // "No API key configured" banner that REPL emits to stdout. The
    // print path was silently swapping in LocalEchoBackend and shipping
    // a canned echo back as if it were a real model response — which
    // looked plausible in `stream-json` output and could be mistaken
    // for a real model run in CI logs. Surface the degradation loudly
    // on stderr so the failure mode is obvious without breaking
    // existing scripts that depend on exit code 0.
    if let Some(warning) = unconfigured_print_mode_warning(configured.is_configured, cli.print_mode)
    {
        eprintln!("{warning}");
    }

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

    let response = assistant.ok_or_else(|| String::from("model returned no assistant message"))?;
    Ok(PromptOutcome {
        response,
        cost: loop_.cost_tracker().summary(),
    })
}

/// Serialize a single `assistant_delta` stream-json event, matching Swift's
/// `{"type":"assistant_delta","delta":...}` (JSON-escaped).
fn assistant_delta_event(delta: &str) -> String {
    serde_json::json!({ "type": "assistant_delta", "delta": delta }).to_string()
}

/// Build the one-shot result envelope, matching the Swift `OneShotJSONResult`
/// shape so embedded/stream-json consumers see identical fields. On error a
/// `type: "error"` result carries the message and zeroed usage.
fn one_shot_result_json(outcome: &Result<PromptOutcome, String>, model: &str) -> serde_json::Value {
    match outcome {
        Ok(outcome) => serde_json::json!({
            "type": "result",
            "status": "completed",
            "response": outcome.response,
            "model": model,
            "cost_usd": outcome.cost.total_cost_usd,
            "input_tokens": outcome.cost.total_input,
            "output_tokens": outcome.cost.total_output,
            "cache_create_tokens": outcome.cost.total_cache_create,
            "cache_read_tokens": outcome.cost.total_cache_read,
        }),
        Err(error) => serde_json::json!({
            "type": "error",
            "status": "fatal",
            "response": "",
            "model": model,
            "cost_usd": 0.0,
            "input_tokens": 0,
            "output_tokens": 0,
            "cache_create_tokens": 0,
            "cache_read_tokens": 0,
            "error": error,
        }),
    }
}

/// Non-streaming backend helper, used by tests; production paths call
/// `configured_backend_with_handler` directly with the appropriate handler.
#[cfg(test)]
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

/// Render the diagnostic report for `--doctor`.
///
/// Designed to be the first thing a user runs when something doesn't
/// behave the way they expect. Does no network I/O and never reveals
/// credential values — API keys are reduced to a presence flag plus
/// their string length so users can confirm they're set without
/// leaking secrets into a pasted bug report.
///
/// `cwd` is passed in (rather than re-read inside the function) so the
/// test suite can drive the report under a tempdir.
fn run_doctor(cli: &Cli, cwd: &Path) -> String {
    let mut lines = Vec::with_capacity(64);

    lines.push(format!("Deeptide doctor  v{}", env!("CARGO_PKG_VERSION")));
    lines.push(String::from(
        "================================================",
    ));
    lines.push(format!("workspace : {}", cwd.display()));
    lines.push(format!(
        "platform  : {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    lines.push(String::new());

    lines.push(String::from("[ config files ]"));
    for (label, path) in [
        ("global ", ConfigStore::global_path()),
        ("project", ConfigStore::project_path(cwd)),
        ("local  ", ConfigStore::local_path(cwd)),
    ] {
        let status = if path.exists() { "exists" } else { "missing" };
        lines.push(format!("  {label}  {}  [{status}]", path.display()));
    }
    match cli.settings.as_ref() {
        Some(path) => {
            let status = if path.exists() { "exists" } else { "MISSING" };
            lines.push(format!("  --settings  {}  [{status}]", path.display()));
        }
        None => lines.push(String::from("  --settings  (not provided)")),
    }
    lines.push(String::new());

    lines.push(String::from("[ model ]"));
    lines.push(format!("  effective  : {}", effective_model(cli)));
    lines.push(format!("  --model    : {}", cli.model));
    lines.push(format!(
        "  env override (DEEPTIDE_MODEL): {}",
        env_first_non_empty(&["DEEPTIDE_MODEL", "ZERO_CLI_MODEL", "ANTHROPIC_MODEL"])
            .unwrap_or_else(|| String::from("(not set)"))
    ));
    match cli.fallback_model.as_deref() {
        Some(fallback) => lines.push(format!("  fallback   : {fallback}")),
        None => lines.push(String::from("  fallback   : (none)")),
    }
    lines.push(String::new());

    lines.push(String::from("[ auth ]"));
    lines.push(format!("  base_url   : {}", effective_base_url(cli)));
    let credential = effective_credential(cli);
    match &credential {
        Some(CloudCredential::ApiKey(_)) => lines.push(String::from(
            "  credential : api key resolved (--api-key or DEEPTIDE_API_KEY; legacy ZERO_API_KEY / ANTHROPIC_API_KEY also accepted)",
        )),
        Some(CloudCredential::BearerToken(_)) => lines.push(String::from(
            "  credential : bearer token resolved (DEEPTIDE_AUTH_TOKEN; legacy ZERO_CLI_AUTH_TOKEN / ANTHROPIC_AUTH_TOKEN also accepted)",
        )),
        None => lines.push(String::from(
            "  credential : NOT RESOLVED — set DEEPTIDE_API_KEY (or pass --api-key) before running a turn. Legacy ZERO_API_KEY / ANTHROPIC_API_KEY also accepted for backward compatibility.",
        )),
    }
    // Env var presence breakdown — never reveals the value, just shows
    // length so users can confirm they exported the right variable. The
    // canonical DEEPTIDE_* names are listed first so users new to the
    // project see them prominently; legacy names follow for migrators.
    for name in [
        "DEEPTIDE_API_KEY",
        "DEEPTIDE_AUTH_TOKEN",
        "ZERO_API_KEY",
        "ZERO_CLI_API_KEY",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_AUTH_TOKEN",
        "ZERO_CLI_AUTH_TOKEN",
    ] {
        match std::env::var(name) {
            Ok(value) if !value.trim().is_empty() => {
                lines.push(format!(
                    "    env {name}  = present (len={})",
                    value.trim().len()
                ));
            }
            _ => lines.push(format!("    env {name}  = (unset)")),
        }
    }
    lines.push(String::new());

    lines.push(String::from("[ baseline ]"));
    lines.push(format!("  permission_mode : {}", cli.permission_mode));
    lines.push(format!(
        "  yolo            : {}",
        if cli.yolo { "ON" } else { "off" }
    ));
    lines.push(format!(
        "  stream          : {}",
        if cli.stream { "ON" } else { "off" }
    ));
    lines.push(format!(
        "  prompt cache    : {}",
        if cli.no_prompt_cache {
            "DISABLED"
        } else {
            "enabled"
        }
    ));
    lines.push(format!("  max_output_tokens: {}", cli.max_output_tokens));
    lines.push(format!("  max_turns        : {}", cli.max_turns));
    lines.push(String::new());

    lines.push(String::from("[ sessions ]"));
    let sessions = deeptide_core::SessionStore::list(cwd);
    lines.push(format!("  saved for this cwd: {}", sessions.len()));
    if let Some(most_recent) = sessions.first() {
        lines.push(format!(
            "  most-recent       : {} ({} messages)",
            most_recent.session_id, most_recent.message_count
        ));
    }
    lines.push(String::new());

    lines.push(String::from("[ optional tools detected on PATH ]"));
    // Each entry is `(binary, platform-note)`. We surface the platform
    // note so a Linux user doesn't see "pbcopy ✗" as a problem.
    let probes: &[(&str, &str)] = &[
        ("git", ""),
        ("rg", "(recommended for fast grep)"),
        ("pbpaste", "(macOS clipboard read)"),
        ("pbcopy", "(macOS clipboard write)"),
        ("wl-paste", "(Wayland clipboard read)"),
        ("xclip", "(X11 clipboard)"),
        ("xsel", "(X11 clipboard fallback)"),
        ("notify-send", "(Linux desktop notifications)"),
        ("osascript", "(macOS scripting / notifications)"),
        ("screencapture", "(macOS screen capture)"),
        ("ffmpeg", "(audio/video transcribe)"),
        ("powershell", "(Windows clipboard / notifications)"),
    ];
    for (bin, note) in probes {
        let mark = if which_on_path(bin).is_some() {
            "✓"
        } else {
            "✗"
        };
        let suffix = if note.is_empty() {
            String::new()
        } else {
            format!(" {note}")
        };
        lines.push(format!("  [{mark}] {bin}{suffix}"));
    }
    lines.push(String::new());

    if credential.is_none() {
        lines.push(String::from(
            "WARNING: no credential resolved. The CLI will refuse to make API calls until one is set.",
        ));
    } else {
        lines.push(String::from(
            "Looks healthy. Run `deeptide -p 'hello'` to verify end-to-end connectivity.",
        ));
    }

    lines.join("\n")
}

/// Lightweight `which`-style probe that walks `$PATH` and reports the
/// first absolute path of an executable named `binary`, or `None`. We
/// deliberately don't shell out to system `which` because Windows
/// PowerShell's `which` alias is `Get-Command` with different semantics
/// — rolling our own keeps `--doctor` self-contained.
fn which_on_path(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        // On Windows the binary may end with `.exe`/`.cmd`/`.bat`; on
        // other platforms a bare name is enough. We try the bare name
        // first because that's what 90% of users have on Unix.
        let candidate = dir.join(binary);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(target_os = "windows")]
        {
            for ext in ["exe", "cmd", "bat", "ps1"] {
                let candidate = dir.join(format!("{binary}.{ext}"));
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Render the catalog of models for `--list-models`. Each row shows
/// per-million-token prices for input, output, cache-create, and
/// cache-read (USD), plus a flag marking the default. Aligned by name
/// so the table stays readable as the catalog grows.
fn format_model_list(default_model: &str) -> String {
    let models = deeptide_core::known_models();
    if models.is_empty() {
        return String::from("No models with built-in pricing data are registered.");
    }
    let name_width = models
        .iter()
        .map(|m| m.name.len())
        .max()
        .unwrap_or(0)
        .max(5);
    let mut lines = vec![format!("Built-in models ({}):", models.len())];
    lines.push(format!(
        "  {:<name_width$}   input      output     cache_create   cache_read   default",
        "name"
    ));
    for model in &models {
        let p = model.pricing;
        let is_default = if model.name == default_model { "*" } else { "" };
        lines.push(format!(
            "  {:<name_width$}   ${:>5.2}/M   ${:>5.2}/M    ${:>5.2}/M      ${:>5.2}/M       {}",
            model.name,
            p.input * 1_000_000.0,
            p.output * 1_000_000.0,
            p.cache_create * 1_000_000.0,
            p.cache_read * 1_000_000.0,
            is_default,
        ));
    }
    lines.push(String::new());
    lines.push(format!("Default: {default_model} (override with --model <name>, env DEEPTIDE_MODEL, or settings.model)."));
    lines.push(String::from(
        "Other model names are still accepted on the wire; only listed ones get accurate cost tracking.",
    ));
    lines.join("\n")
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
    // Canonical Deeptide env names take priority. The legacy ZERO_* /
    // ANTHROPIC_* names are preserved as fallbacks so users migrating from
    // zero-cli or other Anthropic-compatible tooling don't have to re-export
    // anything.
    //
    // Note: `cli.api_key` is also clap-bound to `DEEPTIDE_API_KEY` (see the
    // `#[arg(... env = "DEEPTIDE_API_KEY")]` declaration), so the explicit
    // `DEEPTIDE_API_KEY` entry below is only consulted on code paths that
    // bypass clap (unit tests building `Cli` directly, library embeds).
    let api_key = cli.api_key.clone().or_else(|| {
        env_first_non_empty(&[
            "DEEPTIDE_API_KEY",
            "ZERO_API_KEY",
            "ZERO_CLI_API_KEY",
            "ANTHROPIC_API_KEY",
        ])
    });
    if let Some(api_key) = api_key.filter(|key| !key.trim().is_empty()) {
        return Some(CloudCredential::ApiKey(api_key));
    }

    if has_provider_base_url(cli) {
        return env_first_non_empty(&[
            "DEEPTIDE_AUTH_TOKEN",
            "ZERO_CLI_AUTH_TOKEN",
            "ANTHROPIC_AUTH_TOKEN",
        ])
        .map(CloudCredential::BearerToken);
    }

    None
}

/// Advance the permission mode by one step around the cycle
/// `Default → AcceptEdits → Plan → Bypass → Default`. Bound to Shift+Tab so
/// users can switch into YOLO mid-session without restarting the REPL.
fn next_permission_mode(current: PermissionMode) -> PermissionMode {
    match current {
        PermissionMode::Default => PermissionMode::AcceptEdits,
        PermissionMode::AcceptEdits => PermissionMode::Plan,
        PermissionMode::Plan => PermissionMode::Bypass,
        PermissionMode::Bypass => PermissionMode::Default,
    }
}

/// Conditional readline handler that records "Shift+Tab was pressed" via an
/// atomic flag and asks rustyline to interrupt the current readline session.
///
/// The interrupt is the cleanest way out of `rl.readline()` from inside a
/// custom binding: it returns `ReadlineError::Interrupted` which our REPL
/// already handles as a benign restart point. The REPL then observes the
/// flag, cycles the permission mode, paints feedback, and presents a fresh
/// prompt — losing any partial input the user had typed, but that's a fair
/// trade for a binding that only fires on an explicit modifier keystroke.
struct ShiftTabCycleHandler {
    pending: Arc<AtomicBool>,
}

impl ConditionalEventHandler for ShiftTabCycleHandler {
    fn handle(
        &self,
        _evt: &Event,
        _n: RepeatCount,
        _positive: bool,
        _ctx: &EventContext<'_>,
    ) -> Option<Cmd> {
        self.pending.store(true, Ordering::Relaxed);
        // Force rustyline to exit with `Interrupted` so the REPL loop can
        // observe the flag and react. Without this rustyline would just sit
        // in `readline()` and the mode switch would be invisible until the
        // user pressed Enter on their own.
        Some(Cmd::Interrupt)
    }
}

/// Interactive permission prompt invoked by the agent loop when a tool
/// call needs the user's approval. Stops the spinner, prints a one-line
/// summary of the pending tool call, reads y/n/a from stdin, and returns
/// the resulting [`AskOutcome`].
///
/// Input grammar:
/// - empty / `y` / `yes`   → allow this call only
/// - `n` / `no`            → deny this call
/// - `a` / `all` / `yolo`  → allow this call AND flip the session to YOLO
///   (Bypass) so subsequent risky calls don't re-prompt.
fn handle_permission_prompt(
    tool_call: &ToolCall,
    spinner_lock: &Arc<Mutex<()>>,
    spinner_stop: &Arc<AtomicBool>,
    output_started: &Arc<AtomicBool>,
    use_color: bool,
) -> AskOutcome {
    // Halt the spinner first; the lock then prevents it from racing the
    // prompt write before it observes the stop flag.
    spinner_stop.store(true, Ordering::Relaxed);
    let _spinner_guard = spinner_lock.lock().ok();

    let mut stdout = io::stdout();
    // Clear whatever the spinner left on the current line.
    let _ = stdout.write_all(b"\r\x1b[2K");

    let summary = summarize_tool_call_for_prompt(tool_call);
    let header = if use_color {
        format!(
            "\n\x1b[33m  ⏸  Permission required\x1b[0m  \x1b[1m{}\x1b[0m  {}\n",
            tool_call.name, summary
        )
    } else {
        format!(
            "\n  ⏸  Permission required  {}  {}\n",
            tool_call.name, summary
        )
    };
    let prompt = if use_color {
        "  \x1b[2m[y]es / [n]o / [a]ll-yolo\x1b[0m  > "
    } else {
        "  [y]es / [n]o / [a]ll-yolo  > "
    };
    let _ = stdout.write_all(header.as_bytes());
    let _ = stdout.write_all(prompt.as_bytes());
    let _ = stdout.flush();

    output_started.store(true, Ordering::Relaxed);

    let mut response = String::new();
    if io::stdin().read_line(&mut response).is_err() {
        return AskOutcome::Deny {
            reason: String::from("failed to read approval from stdin"),
        };
    }

    match response.trim().to_ascii_lowercase().as_str() {
        "" | "y" | "yes" => AskOutcome::Allow,
        "n" | "no" => AskOutcome::Deny {
            reason: String::from("user declined"),
        },
        "a" | "all" | "yolo" | "all-yolo" => {
            let _ = writeln!(
                stdout,
                "  (session switched to YOLO mode — Shift+Tab to cycle back)"
            );
            AskOutcome::AllowAndSetMode(PermissionMode::Bypass)
        }
        other => AskOutcome::Deny {
            reason: format!("user declined ({other})"),
        },
    }
}

/// Produce a one-line summary of the tool call's most relevant input field
/// for the permission prompt. Bash → command, Write/Edit → file_path, etc.
/// Falls back to a truncated JSON dump for unknown tools so the user can
/// still see what's being requested.
fn summarize_tool_call_for_prompt(tool_call: &ToolCall) -> String {
    const MAX_LEN: usize = 140;

    let extracted = match tool_call.name.as_str() {
        "Bash" => tool_call
            .input
            .get("command")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        "Write" | "Edit" => tool_call
            .input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        "Read" => tool_call
            .input
            .get("file_path")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        "WebFetch" => tool_call
            .input
            .get("url")
            .and_then(|v| v.as_str())
            .map(str::to_owned),
        _ => None,
    };

    let raw =
        extracted.unwrap_or_else(|| serde_json::to_string(&tool_call.input).unwrap_or_default());

    // Single-line, trimmed to MAX_LEN characters (not bytes) for the prompt.
    let collapsed: String = raw
        .lines()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    if collapsed.chars().count() <= MAX_LEN {
        collapsed
    } else {
        let mut truncated: String = collapsed.chars().take(MAX_LEN).collect();
        truncated.push('…');
        truncated
    }
}

/// Returns `true` when a Paean publishing token is exported in the
/// environment. Paean today only powers `/publish` (skill upload to
/// api.paean.ai); inference still routes through Anthropic-compatible
/// providers via the regular API key chain. The status bar surfaces this so
/// users can see at a glance whether `/publish` will work without re-reading
/// `--doctor`.
fn paean_token_resolved() -> bool {
    env_first_non_empty(&["PAEAN_API_TOKEN", "PAEAN_TOKEN", "CLIDE_API_TOKEN"]).is_some()
}

/// Build the status-bar auth segment, rotating between the inference API key
/// indicator and the Paean publish-token indicator when both are present.
///
/// Rotation logic:
/// - both keys resolved → alternate every paint between `key ok` / `paean ok`
/// - only API key      → `key ok`
/// - only Paean token  → `paean ok`
/// - neither           → `auth —`
///
/// `tick` is a monotonically increasing counter (one per status-bar paint).
/// We don't tie it to wall-clock time because the bar only repaints between
/// turns; tying to paints keeps the cycle smooth and predictable from the
/// user's perspective.
fn build_auth_segment(api_key_resolved: bool, paean_resolved: bool, tick: u64) -> StatusSegment {
    match (api_key_resolved, paean_resolved) {
        (true, true) => {
            if tick.is_multiple_of(2) {
                StatusSegment::new("key", "ok")
            } else {
                StatusSegment::new("paean", "ok")
            }
        }
        (true, false) => StatusSegment::new("key", "ok"),
        (false, true) => StatusSegment::new("paean", "ok"),
        (false, false) => StatusSegment::new("auth", "—"),
    }
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
        build_auth_segment, collect_prompt, configured_backend, effective_base_url,
        effective_model, next_permission_mode, normalize_embedded_mode, paean_token_resolved,
        summarize_tool_call_for_prompt, use_color, validate_formats,
    };
    use clap::Parser;
    use deeptide_core::permissions::PermissionMode;
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
            list_models: false,
            doctor: false,
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
                "DEEPTIDE_API_KEY",
                "DEEPTIDE_AUTH_TOKEN",
                "DEEPTIDE_BASE_URL",
                "DEEPTIDE_MODEL",
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
                "PAEAN_API_TOKEN",
                "PAEAN_TOKEN",
                "CLIDE_API_TOKEN",
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
    fn assistant_delta_event_matches_swift_shape_and_escapes() {
        let event = super::assistant_delta_event("hello \"world\"\n");
        let parsed: serde_json::Value = serde_json::from_str(&event).expect("valid JSON");
        assert_eq!(parsed["type"], "assistant_delta");
        assert_eq!(parsed["delta"], "hello \"world\"\n");
    }

    #[test]
    fn one_shot_result_json_success_matches_swift_envelope() {
        let outcome = Ok(super::PromptOutcome {
            response: String::from("done"),
            cost: deeptide_core::CostSummary {
                total_input: 10,
                total_output: 5,
                total_cache_create: 2,
                total_cache_read: 8,
                total_cost_usd: 0.0123,
                ..Default::default()
            },
        });
        let json = super::one_shot_result_json(&outcome, "claude-x");
        assert_eq!(json["type"], "result");
        assert_eq!(json["status"], "completed");
        assert_eq!(json["response"], "done");
        assert_eq!(json["model"], "claude-x");
        assert_eq!(json["input_tokens"], 10);
        assert_eq!(json["output_tokens"], 5);
        assert_eq!(json["cache_create_tokens"], 2);
        assert_eq!(json["cache_read_tokens"], 8);
        assert!(json.get("cost_usd").is_some());
        assert!(json.get("error").is_none(), "success result omits error");
    }

    #[test]
    fn one_shot_result_json_error_carries_message_and_zeroed_usage() {
        let outcome: Result<super::PromptOutcome, String> = Err(String::from("boom"));
        let json = super::one_shot_result_json(&outcome, "claude-x");
        assert_eq!(json["type"], "error");
        assert_eq!(json["status"], "fatal");
        assert_eq!(json["error"], "boom");
        assert_eq!(json["response"], "");
        assert_eq!(json["input_tokens"], 0);
        assert_eq!(json["output_tokens"], 0);
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

    #[test]
    fn format_model_list_marks_default_and_shows_every_known_model() {
        let out = super::format_model_list("deepseek-v4-pro");
        // Every entry in known_models() must appear in the table; if a
        // future commit adds a new pricing entry, this test forces the
        // change to also surface through `--list-models`.
        let count = deeptide_core::known_models().len();
        assert!(
            out.starts_with(&format!("Built-in models ({count}):")),
            "header should announce row count; got: {out}"
        );
        for model in deeptide_core::known_models() {
            assert!(
                out.contains(model.name),
                "missing {} from model table",
                model.name
            );
        }
        // The chosen default must end with the asterisk marker, and a
        // non-default model must not.
        for line in out.lines() {
            if line.starts_with("  deepseek-v4-pro ") {
                assert!(
                    line.trim_end().ends_with('*'),
                    "default row should end with '*'; got: {line}"
                );
            }
            if line.starts_with("  deepseek-v4-flash ") {
                assert!(
                    !line.trim_end().ends_with('*'),
                    "non-default row must not end with '*'; got: {line}"
                );
            }
        }
        // Footer points users at the override paths.
        assert!(out.contains("--model"));
        assert!(out.contains("DEEPTIDE_MODEL"));
    }

    #[test]
    fn doctor_report_does_not_leak_credential_values() {
        let _g = env_guard();
        // Use a unique unlikely-to-collide secret. We want to verify it
        // appears nowhere in the doctor report even if the env var is set.
        let secret = "sk-deeptide-doctor-test-XYZZY-DO-NOT-LEAK-12345";
        unsafe {
            std::env::set_var("ANTHROPIC_API_KEY", secret);
        }
        let cli = sample_cli();
        let dir = tempfile::tempdir().expect("tempdir");
        let report = super::run_doctor(&cli, dir.path());
        unsafe {
            std::env::remove_var("ANTHROPIC_API_KEY");
        }

        assert!(
            !report.contains(secret),
            "doctor report leaked the secret value: {report}"
        );
        // The report must still confirm the credential is present and
        // surface a useful length so the user can sanity-check it.
        assert!(
            report.contains("ANTHROPIC_API_KEY  = present (len="),
            "doctor must report ANTHROPIC_API_KEY presence + length"
        );
        assert!(
            report.contains(&format!("len={})", secret.len())),
            "doctor must report the exact length of the env var; got: {report}"
        );
    }

    #[test]
    fn doctor_report_announces_missing_credential_when_no_env_set() {
        let _g = env_guard();
        clear_api_env();
        let mut cli = sample_cli();
        cli.api_key = None;
        let dir = tempfile::tempdir().expect("tempdir");
        let report = super::run_doctor(&cli, dir.path());
        assert!(
            report.contains("credential : NOT RESOLVED"),
            "doctor must call out missing credential prominently; got: {report}"
        );
        assert!(
            report.contains("WARNING: no credential resolved"),
            "doctor must emit a closing warning when no credential is set"
        );
        // The recovery hint must lead with the canonical Deeptide env name,
        // not the legacy ones the project inherited from zero-cli — that
        // was the prior bug where users following --help (which advertises
        // DEEPTIDE_API_KEY) saw a doctor message that only suggested the
        // legacy names.
        assert!(
            report.contains("set DEEPTIDE_API_KEY"),
            "doctor's recovery hint must lead with DEEPTIDE_API_KEY; got: {report}"
        );
    }

    #[test]
    fn deeptide_auth_token_resolves_a_bearer_credential_against_provider_base_url() {
        let _g = env_guard();
        clear_api_env();
        unsafe {
            std::env::set_var("DEEPTIDE_AUTH_TOKEN", "deeptide-bearer-token");
        }
        let mut cli = sample_cli();
        cli.api_key = None;
        // Bearer-token resolution only applies when the base URL has been
        // pointed at a custom provider (not the Anthropic default). Use the
        // standard custom-provider shape we test elsewhere.
        cli.base_url = "https://api.example.test".to_owned();

        let credential = super::effective_credential(&cli);
        match credential {
            Some(super::CloudCredential::BearerToken(token)) => {
                assert_eq!(token, "deeptide-bearer-token");
            }
            Some(super::CloudCredential::ApiKey(_)) => {
                panic!("DEEPTIDE_AUTH_TOKEN should yield BearerToken, not ApiKey")
            }
            None => panic!("DEEPTIDE_AUTH_TOKEN should yield a BearerToken credential, got None"),
        }

        clear_api_env();
    }

    #[test]
    fn deeptide_api_key_takes_priority_over_legacy_env_names() {
        let _g = env_guard();
        clear_api_env();
        unsafe {
            std::env::set_var("DEEPTIDE_API_KEY", "deeptide-canonical");
            std::env::set_var("ANTHROPIC_API_KEY", "legacy-anthropic");
            std::env::set_var("ZERO_API_KEY", "legacy-zero");
        }
        let mut cli = sample_cli();
        // Force the test through the env-fallback branch (clap normally
        // populates cli.api_key from DEEPTIDE_API_KEY itself).
        cli.api_key = None;

        let credential = super::effective_credential(&cli);
        match credential {
            Some(super::CloudCredential::ApiKey(key)) => {
                assert_eq!(
                    key, "deeptide-canonical",
                    "DEEPTIDE_API_KEY must win over legacy ZERO_/ANTHROPIC_ names"
                );
            }
            Some(super::CloudCredential::BearerToken(_)) => {
                panic!("expected ApiKey credential; got BearerToken")
            }
            None => panic!("expected ApiKey credential; got None"),
        }

        clear_api_env();
    }

    #[test]
    fn doctor_report_probes_deeptide_env_namespace() {
        let _g = env_guard();
        clear_api_env();
        let cli = sample_cli();
        let dir = tempfile::tempdir().expect("tempdir");
        let report = super::run_doctor(&cli, dir.path());

        // Every probed env var must appear in the report — both the
        // canonical DEEPTIDE_* names AND the legacy aliases — so a user
        // who exported the "wrong" variable can spot which one they
        // actually set.
        for name in [
            "DEEPTIDE_API_KEY",
            "DEEPTIDE_AUTH_TOKEN",
            "ZERO_API_KEY",
            "ZERO_CLI_API_KEY",
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_AUTH_TOKEN",
            "ZERO_CLI_AUTH_TOKEN",
        ] {
            assert!(
                report.contains(name),
                "doctor probe table is missing {name}; report:\n{report}"
            );
        }

        // DEEPTIDE_* names must appear before any legacy name so they read
        // as the canonical choice in the report.
        let deeptide_pos = report
            .find("DEEPTIDE_API_KEY")
            .expect("DEEPTIDE_API_KEY must be in the report");
        let legacy_pos = report
            .find("ZERO_API_KEY")
            .expect("ZERO_API_KEY must be in the report");
        assert!(
            deeptide_pos < legacy_pos,
            "DEEPTIDE_API_KEY must be listed before ZERO_API_KEY so the report leads with the canonical name"
        );
    }

    #[test]
    fn doctor_report_surfaces_each_config_scope_with_existence_state() {
        let _g = env_guard();
        let dir = tempfile::tempdir().expect("tempdir");
        // Create only the project scope; global and local should report
        // as missing, project as exists.
        let project_dir = dir.path().join(".deeptide");
        std::fs::create_dir_all(&project_dir).expect("project dir");
        std::fs::write(project_dir.join("settings.json"), "{}").expect("project file");

        let cli = sample_cli();
        let report = super::run_doctor(&cli, dir.path());

        // Look at the project line specifically.
        let project_line = report
            .lines()
            .find(|line| line.contains(".deeptide/settings.json") && line.contains("project"))
            .expect("project line present");
        assert!(
            project_line.ends_with("[exists]"),
            "project scope must be marked existing: {project_line}"
        );
        let local_line = report
            .lines()
            .find(|line| line.contains("settings.local.json"))
            .expect("local line present");
        assert!(
            local_line.ends_with("[missing]"),
            "local scope must be marked missing: {local_line}"
        );
    }

    #[test]
    fn doctor_report_marks_settings_override_path_existence() {
        let _g = env_guard();
        let dir = tempfile::tempdir().expect("tempdir");
        let settings = dir.path().join("custom.json");
        std::fs::write(&settings, "{}").expect("write");
        let mut cli = sample_cli();
        cli.settings = Some(settings.clone());
        let report = super::run_doctor(&cli, dir.path());
        assert!(
            report.contains(&format!("--settings  {}  [exists]", settings.display())),
            "doctor must mark an existing --settings file as [exists]; got: {report}"
        );

        // Now point at a non-existent file — the report should say MISSING
        // (uppercase) so it stands out as a likely misconfiguration.
        let mut cli2 = sample_cli();
        cli2.settings = Some(dir.path().join("nope.json"));
        let report2 = super::run_doctor(&cli2, dir.path());
        assert!(
            report2.contains("[MISSING]"),
            "doctor must mark a bad --settings path as [MISSING] (uppercase)"
        );
    }

    #[test]
    fn doctor_report_probes_path_for_known_optional_tools() {
        let _g = env_guard();
        let cli = sample_cli();
        let dir = tempfile::tempdir().expect("tempdir");
        let report = super::run_doctor(&cli, dir.path());

        // Every probe must appear; their detection state varies by host
        // and is not asserted here, but the *catalog* must be exhaustive
        // so users can't be surprised by a missing entry.
        for probe in [
            "git",
            "rg",
            "pbpaste",
            "pbcopy",
            "wl-paste",
            "xclip",
            "xsel",
            "notify-send",
            "osascript",
            "screencapture",
            "ffmpeg",
            "powershell",
        ] {
            assert!(
                report.contains(probe),
                "doctor PATH-probe section missing entry for {probe}; got: {report}"
            );
        }
    }

    #[test]
    fn unconfigured_print_mode_emits_actionable_warning() {
        // The whole point: --print mode without credentials must yield a
        // loud stderr warning. The text must call out the failure mode
        // ("local echo backend"), every env var the user can set, and
        // the `--doctor` escape hatch.
        let warning = super::unconfigured_print_mode_warning(false, true)
            .expect("warning must fire when print mode is unconfigured");
        assert!(warning.contains("local echo backend"));
        assert!(warning.contains("DEEPTIDE_API_KEY"));
        assert!(warning.contains("ANTHROPIC_API_KEY"));
        assert!(warning.contains("ZERO_API_KEY"));
        assert!(warning.contains("--api-key"));
        assert!(warning.contains("deeptide --doctor"));
    }

    #[test]
    fn configured_print_mode_does_not_warn() {
        // is_configured = true → no warning (a real backend will run).
        assert!(super::unconfigured_print_mode_warning(true, true).is_none());
    }

    #[test]
    fn interactive_mode_relies_on_in_band_banner_not_stderr() {
        // Interactive REPL prints its own visible banner on stdout when
        // unconfigured. The stderr warning is a print-mode-only safety
        // net to keep `--print` output trustworthy in CI logs.
        assert!(super::unconfigured_print_mode_warning(false, false).is_none());
        assert!(super::unconfigured_print_mode_warning(true, false).is_none());
    }

    #[test]
    fn which_on_path_finds_a_known_binary_or_returns_none() {
        // `sh` is present on every Unix host the CI matrix supports;
        // on Windows we don't run this assertion at all.
        #[cfg(unix)]
        {
            assert!(
                super::which_on_path("sh").is_some(),
                "PATH probe failed to find sh — env may be misconfigured"
            );
        }
        // A guaranteed-absent binary must return None.
        assert!(
            super::which_on_path("deeptide-doctor-definitely-does-not-exist-xyz").is_none(),
            "PATH probe must return None for clearly absent binaries"
        );
    }

    #[test]
    fn format_model_list_renders_pricing_per_million_tokens() {
        // The cost.rs table stores prices as USD per token (e.g. 0.27 /
        // 1_000_000.0). The CLI surfaces them scaled to per-million so
        // they're human-readable. Spot-check the known v4-pro row.
        let out = super::format_model_list("deepseek-v4-pro");
        assert!(
            out.contains("$ 0.27/M") || out.contains("$0.27/M"),
            "expected input price scaled to per-million tokens; got: {out}"
        );
        // Output price for v4-pro is $1.10/M.
        assert!(
            out.contains("$ 1.10/M") || out.contains("$1.10/M"),
            "expected output price scaled to per-million tokens; got: {out}"
        );
    }

    #[test]
    fn auth_segment_shows_key_ok_when_only_api_key_resolved() {
        let segment = build_auth_segment(true, false, 0);
        assert_eq!(segment.label, "key");
        assert_eq!(segment.value, "ok");
        // Same shape regardless of tick: with no paean token, there's
        // nothing to rotate to.
        let segment_later = build_auth_segment(true, false, 999);
        assert_eq!(segment_later.label, "key");
    }

    #[test]
    fn auth_segment_shows_paean_when_only_paean_resolved() {
        let segment = build_auth_segment(false, true, 0);
        assert_eq!(segment.label, "paean");
        assert_eq!(segment.value, "ok");
    }

    #[test]
    fn auth_segment_warns_when_neither_resolved() {
        let segment = build_auth_segment(false, false, 0);
        assert_eq!(segment.label, "auth");
        // Em-dash, not the regular minus.
        assert_eq!(segment.value, "—");
    }

    #[test]
    fn auth_segment_rotates_when_both_resolved() {
        let even = build_auth_segment(true, true, 0);
        let odd = build_auth_segment(true, true, 1);
        assert_eq!(even.label, "key");
        assert_eq!(odd.label, "paean");
        // Wrap-around at u64::MAX preserves the alternating contract.
        assert_eq!(build_auth_segment(true, true, u64::MAX).label, "paean");
        assert_eq!(build_auth_segment(true, true, 2).label, "key");
    }

    fn tool_call(name: &str, input: serde_json::Value) -> deeptide_core::ToolCall {
        deeptide_core::ToolCall::new("call_test", name, input)
    }

    #[test]
    fn shift_tab_cycles_modes_in_a_full_loop() {
        // Default → AcceptEdits → Plan → Bypass → Default. Each step is a
        // single Shift+Tab press in the running REPL.
        assert_eq!(
            next_permission_mode(PermissionMode::Default),
            PermissionMode::AcceptEdits
        );
        assert_eq!(
            next_permission_mode(PermissionMode::AcceptEdits),
            PermissionMode::Plan
        );
        assert_eq!(
            next_permission_mode(PermissionMode::Plan),
            PermissionMode::Bypass
        );
        assert_eq!(
            next_permission_mode(PermissionMode::Bypass),
            PermissionMode::Default
        );
    }

    #[test]
    fn permission_prompt_summary_extracts_bash_command() {
        let summary = summarize_tool_call_for_prompt(&tool_call(
            "Bash",
            serde_json::json!({"command": "ls -la /tmp"}),
        ));
        assert_eq!(summary, "ls -la /tmp");
    }

    #[test]
    fn permission_prompt_summary_extracts_file_path_for_writes() {
        let summary = summarize_tool_call_for_prompt(&tool_call(
            "Write",
            serde_json::json!({"file_path": "/tmp/foo.txt", "content": "secret"}),
        ));
        assert_eq!(summary, "/tmp/foo.txt");
    }

    #[test]
    fn permission_prompt_summary_truncates_long_commands_with_ellipsis() {
        let long_cmd = "a".repeat(500);
        let summary = summarize_tool_call_for_prompt(&tool_call(
            "Bash",
            serde_json::json!({"command": long_cmd}),
        ));
        assert!(
            summary.ends_with('…'),
            "expected ellipsis, got: {summary:?}"
        );
        assert!(summary.chars().count() <= 141);
    }

    #[test]
    fn permission_prompt_summary_collapses_newlines_in_command() {
        let summary = summarize_tool_call_for_prompt(&tool_call(
            "Bash",
            serde_json::json!({"command": "echo line1\n  echo line2\necho line3"}),
        ));
        // Prompt must stay single-line; whitespace collapses to single spaces.
        assert_eq!(summary, "echo line1 echo line2 echo line3");
    }

    #[test]
    fn permission_prompt_summary_falls_back_to_json_for_unknown_tools() {
        let summary = summarize_tool_call_for_prompt(&tool_call(
            "MysteryTool",
            serde_json::json!({"k": "v"}),
        ));
        assert!(summary.contains("\"k\""), "got: {summary:?}");
        assert!(summary.contains("\"v\""), "got: {summary:?}");
    }

    #[test]
    fn paean_token_resolved_detects_any_supported_env_name() {
        let _guard = env_guard();
        clear_api_env();
        assert!(!paean_token_resolved());

        unsafe {
            std::env::set_var("PAEAN_TOKEN", "fixture");
        }
        assert!(paean_token_resolved());

        clear_api_env();
        unsafe {
            std::env::set_var("CLIDE_API_TOKEN", "fixture");
        }
        assert!(paean_token_resolved());

        clear_api_env();
        unsafe {
            std::env::set_var("PAEAN_API_TOKEN", "fixture");
        }
        assert!(paean_token_resolved());

        clear_api_env();
        // Empty string should not be treated as a resolved token.
        unsafe {
            std::env::set_var("PAEAN_API_TOKEN", "");
        }
        assert!(!paean_token_resolved());
        clear_api_env();
    }
}
