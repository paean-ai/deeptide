use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use clap::{ArgAction, Parser, ValueEnum};
use deeptide_core::config::ConfigStore;
use deeptide_core::embedded_protocol::{
    EmbeddedProtocol, EmbeddedProtocolSpec, PermissionResponse,
};
use deeptide_core::permissions::PermissionMode;
use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentTerminalEvent, AnthropicBackend, AskOutcome,
    CommandCompletionCandidate, CommandCompletionSource, CompletionEngine, LocalEchoBackend,
    MarkdownRenderOptions, ModelPricing, ReplEvent, ReplSession, StatusSegment, StreamingEvent,
    StreamingHandler, StreamingMarkdownRenderer, SystemMessage, ToolCall, tui,
};
/// A single completion candidate produced by the slash-command / `@path`
/// / argument completers. `replacement` is the text spliced into the
/// buffer by the custom [`prompt_editor`] (which owns candidate-list
/// rendering, so no separate display string is needed). Formerly
/// `rustyline::completion::Pair`.
#[derive(Debug, Clone)]
struct Pair {
    replacement: String,
}

mod chrome;
mod line_edit;
mod picker;
mod prompt_editor;
use deeptide_host::provider;
mod queue_editor;
mod queue_input;
mod status_bar;

const DEFAULT_MODEL: &str = "deepseek-v4-flash";
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

// Backend assembly (provider presets, protocol dispatch, sub-agent config) lives
// in `deeptide-host` so the CLI and the GUI build backends identically.
use deeptide_host::backend::{
    BackendParams, CloudCredential, ConfiguredBackend, SubagentConfig, build_backend,
};

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

/// Compile-time short version string for `-V`. Combines the static crate
/// version with the git commit short hash and commit date captured by
/// `build.rs`. When the binary is built outside a git checkout (e.g. from
/// a crates.io tarball) the build script substitutes "unknown" for the
/// git pieces; we strip those out so `--version` stays clean and doesn't
/// announce missing metadata.
const VERSION_SHORT: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("DEEPTIDE_GIT_HASH"),
    " ",
    env!("DEEPTIDE_GIT_DATE"),
    ")"
);

/// Multi-line `--version` output. Exposes every build-provenance datum so
/// a user filing a bug report can paste the output and unambiguously
/// identify which binary they're running.
const VERSION_LONG: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("DEEPTIDE_GIT_HASH"),
    " ",
    env!("DEEPTIDE_GIT_DATE"),
    ")\n",
    "commit:  ",
    env!("DEEPTIDE_GIT_HASH"),
    "\n",
    "date:    ",
    env!("DEEPTIDE_GIT_DATE"),
    "\n",
    "branch:  ",
    env!("DEEPTIDE_GIT_BRANCH"),
    "\n",
    "rustc:   ",
    env!("DEEPTIDE_RUSTC"),
);

/// Worked examples appended to the bottom of `--help`. They focus on the
/// headless / scripted surface so the binary is self-documenting when wired
/// into other tools or used non-interactively.
const HEADLESS_EXAMPLES: &str = concat!(
    "Examples:\n",
    "  # Interactive REPL (default)\n",
    "  deeptide\n",
    "\n",
    "  # Headless one-shot: print the answer as plain text and exit\n",
    "  deeptide --print -p \"explain src/main.rs\"\n",
    "\n",
    "  # Pipe the prompt in; emit a single JSON result object\n",
    "  echo \"summarise the staged diff\" | deeptide --print --output-format json\n",
    "\n",
    "  # Stream structured events (NDJSON) for live consumption\n",
    "  deeptide --print --output-format stream-json -p \"refactor the parser\"\n",
    "\n",
    "  # Combine a prompt with piped file contents\n",
    "  deeptide --print --read-stdin -p \"review this patch\" < change.diff\n",
    "\n",
    "  # Full autonomy with a restricted toolset (good for CI)\n",
    "  deeptide --print -y --allowed-tools Read,Edit,Bash -p \"fix the failing test\"\n",
    "\n",
    "  # Resume a saved session headlessly\n",
    "  deeptide --print -r <session-id> -p \"continue\"\n",
    "\n",
    "  # Embedded NDJSON protocol, driven by a host application\n",
    "  deeptide --embedded\n",
);

#[derive(Debug, Parser)]
#[command(
    name = "deeptide-rs",
    version = VERSION_SHORT,
    long_version = VERSION_LONG,
    about = "Agentic coding CLI. Interactive REPL by default; headless with --print or --embedded.",
    long_about = "Deeptide is an agentic coding assistant. Run it with no arguments for an \
interactive REPL, or drive it non-interactively as a scriptable CLI.\n\
\n\
Execution modes:\n\
  - Interactive REPL  Default when no headless flag is set and stdin is a terminal.\n\
  - Headless one-shot  --print (alias --no-tui): run a single prompt, print the result,\n\
                       and exit. Pick the wire format with --output-format\n\
                       (text | json | stream-json). The prompt comes from --prompt/-p, or\n\
                       from stdin when omitted (echo \"...\" | deeptide --print).\n\
  - Embedded protocol  --embedded: line-delimited JSON (NDJSON) over stdin/stdout for host\n\
                       apps that drive the agent programmatically.\n\
\n\
Compatibility:\n\
  The headless surface is wire-compatible with the Swift Deeptide app and zero-cli: the same\n\
  --print / --output-format / --input-format / --embedded contract and stream-json event\n\
  shapes, so existing `tide --print` scripts work unchanged. On-device local-model flags\n\
  (-L/--local and friends) are macOS-only and handled by the `tide local` launcher, not this\n\
  binary; everything else in the headless contract is supported here.",
    after_long_help = HEADLESS_EXAMPLES,
)]
struct Cli {
    #[arg(
        short = 'p',
        long,
        value_name = "TEXT",
        help_heading = "Headless & scripting",
        help = "Prompt to run. In --print mode, the prompt is read from stdin when this is omitted."
    )]
    prompt: Option<String>,

    #[arg(
        long = "print",
        visible_alias = "no-tui",
        action = ArgAction::SetTrue,
        help_heading = "Headless & scripting",
        help = "Non-interactive one-shot: run the prompt, print the result, and exit (no REPL)."
    )]
    print_mode: bool,

    #[arg(
        long,
        value_enum,
        default_value_t = InputFormat::Text,
        help_heading = "Headless & scripting",
        help = "Prompt input format for --print: text, or stream-json (NDJSON messages on stdin)."
    )]
    input_format: InputFormat,

    #[arg(
        long,
        value_enum,
        default_value_t = OutputFormat::Text,
        help_heading = "Headless & scripting",
        help = "Result format for --print: text (default), json (one result object), or stream-json (NDJSON events)."
    )]
    output_format: OutputFormat,

    #[arg(
        long,
        action = ArgAction::SetTrue,
        help_heading = "Headless & scripting",
        help = "Run the Clide/DeepClide embedded NDJSON protocol over stdin/stdout (implies --print and stream-json in/out)."
    )]
    embedded: bool,

    #[arg(
        long = "read-stdin",
        action = ArgAction::SetTrue,
        help_heading = "Headless & scripting",
        help = "Read stdin and append it to --prompt. Lets you combine a fixed instruction with piped content."
    )]
    read_stdin: bool,

    #[arg(
        long,
        value_name = "ID",
        help_heading = "Headless & scripting",
        help = "Session id to use for embedded or scripted runs."
    )]
    session_id: Option<String>,

    #[arg(
        long,
        value_name = "DIR",
        help_heading = "Headless & scripting",
        help = "Working directory for the run (changes into it before starting)."
    )]
    cwd: Option<PathBuf>,

    #[arg(
        long,
        default_value = "default",
        help = "Permission mode: default, accept-edits, plan, bypass."
    )]
    permission_mode: String,

    #[arg(
        long,
        env = "DEEPTIDE_THEME",
        default_value = "dark",
        help = "Colour theme for syntax highlighting: dark, light, high-contrast."
    )]
    theme: String,

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
        long,
        env = "DEEPTIDE_PROVIDER",
        value_name = "NAME",
        help = "Model provider preset: anthropic, openai, deepseek, moonshot, zhipu, openrouter, groq, ollama, lmstudio, vllm, mlx — or any name for a custom OpenAI-compatible endpoint. Selects the wire protocol and default base-url/model; explicit --base-url/--model/--api-key still win. See --list-providers."
    )]
    provider: Option<String>,

    #[arg(
        long = "list-providers",
        help = "List the built-in provider presets (protocol + default endpoint/model) and exit."
    )]
    list_providers: bool,

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

    #[arg(
        long,
        visible_alias = "max-tokens",
        default_value_t = 65_536,
        env = "DEEPTIDE_MAX_OUTPUT_TOKENS",
        help = "Maximum tokens the model may produce per turn. 64K matches the practical output \
                cap of Claude 4.5 Sonnet and is safely clamped server-side for smaller-capacity \
                models. DeepSeek V4 supports up to 384K — raise this for very large one-shot \
                outputs, or use chunked Write+Edit patterns for arbitrary file sizes."
    )]
    max_output_tokens: usize,

    #[arg(
        long,
        env = "DEEPTIDE_MAX_TURNS",
        default_value_t = 200,
        help = "Safety cap on agentic turns per prompt. Default 200 covers \
                long multi-file refactors; raise via --max-turns N or set \
                `max_turns` in settings.json (or DEEPTIDE_MAX_TURNS env)."
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
        short = 'n',
        long = "name",
        value_name = "NAME",
        help = "Session display name. Sets the terminal window title for this interactive session."
    )]
    name: Option<String>,

    #[arg(
        long = "list-sessions",
        action = ArgAction::SetTrue,
        help = "List saved sessions for this directory and exit (no API key required)."
    )]
    list_sessions: bool,

    #[arg(
        long = "import",
        value_name = "TOOL",
        help = "Import a prior session from another agent (claude|codex|deeptide) for this directory before the first prompt."
    )]
    import: Option<String>,

    #[arg(
        long = "import-session",
        value_name = "ID",
        help = "Which session to import (id prefix). Defaults to the most recent for this directory."
    )]
    import_session: Option<String>,

    #[arg(
        long = "import-as",
        value_name = "MODE",
        default_value = "context",
        value_parser = ["context", "memory"],
        help = "How to bring the imported session in: `context` (splice a live handoff) or `memory` (distil durable facts)."
    )]
    import_as: String,

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

    // Env fallback for these `no_*` toggles is applied manually in `run` via
    // `env_flag` rather than clap's `env=`: clap's bool+env treats the var as
    // set for ANY non-"false" value, so e.g. `DEEPTIDE_NO_SUGGESTIONS=0` would
    // wrongly *enable* the flag. `env_flag` gives intuitive 0/false/no/off→off.
    #[arg(
        long = "gui",
        action = ArgAction::SetTrue,
        help = "Launch the native desktop GUI (the `deeptide-gui` binary), sharing this config + sessions."
    )]
    gui: bool,

    #[arg(
        long = "no-session-persistence",
        action = ArgAction::SetTrue,
        help = "Do not autosave conversation turns to disk (privacy / scratch sessions) (env DEEPTIDE_NO_SESSION_PERSISTENCE=1; 0/false/no/off/empty/unset = persist)."
    )]
    no_session_persistence: bool,

    #[arg(
        long = "no-session-capture",
        action = ArgAction::SetTrue,
        help = "Skip the end-of-session memory consolidation pass so /exit and Ctrl-D return immediately (env DEEPTIDE_NO_SESSION_CAPTURE=1; 0/false/no/off/empty/unset = capture)."
    )]
    no_session_capture: bool,

    #[arg(
        long = "no-suggestions",
        action = ArgAction::SetTrue,
        help = "Do not show follow-up prompt suggestions after a task finishes (env DEEPTIDE_NO_SUGGESTIONS=1; 0/false/no/off/empty/unset = shown)."
    )]
    no_suggestions: bool,

    #[arg(
        long = "settings",
        value_name = "PATH",
        help = "Merge an explicit settings.json file on top of the global/project/local scopes."
    )]
    settings: Option<PathBuf>,

    #[arg(
        long = "isolated",
        env = "DEEPTIDE_ISOLATED",
        action = ArgAction::SetTrue,
        help = "Ignore the global/project/local settings.json scopes; use only --settings (or built-in defaults). Makes per-invocation config reproducible for testing alternate keys/endpoints."
    )]
    isolated: bool,

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

/// Local UTC offset, snapshotted once at startup. `OnceLock` because we
/// only ever set it from the single-threaded entry point and read it
/// from the (later, multithreaded) REPL loop.
static LOCAL_UTC_OFFSET: OnceLock<time::UtcOffset> = OnceLock::new();

/// Snapshot the local UTC offset while the process is still
/// single-threaded. Falls back to UTC if the platform can't resolve a
/// local offset. Idempotent — only the first call wins.
fn capture_local_offset() {
    let offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    let _ = LOCAL_UTC_OFFSET.set(offset);
}

/// Current wall-clock time in the user's local zone, using the offset
/// captured at startup by [`capture_local_offset`]. Safe to call from
/// any thread.
fn local_now() -> time::OffsetDateTime {
    let offset = LOCAL_UTC_OFFSET
        .get()
        .copied()
        .unwrap_or(time::UtcOffset::UTC);
    time::OffsetDateTime::now_utc().to_offset(offset)
}

/// Launch the native desktop GUI (`deeptide-gui`). The CLI deliberately does NOT
/// depend on the GUI crate (that would drag a webview/graphics stack into the
/// lean headless binary), so we exec the separate binary — preferring one next
/// to this executable, then falling back to a PATH lookup by name.
fn launch_gui() -> Result<(), String> {
    let name = if cfg!(windows) {
        "deeptide-gui.exe"
    } else {
        "deeptide-gui"
    };
    let beside = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
        .filter(|path| path.exists());
    let program = beside.unwrap_or_else(|| std::path::PathBuf::from(name));

    let status = std::process::Command::new(&program)
        .status()
        .map_err(|error| {
            format!(
                "could not launch the desktop GUI ({name}): {error}.\n\
             Build/install it with `cargo build -p deeptide-gui` (or place `{name}` on PATH)."
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("the desktop GUI exited with {status}"))
    }
}

/// Truthiness of a boolean env var with well-defined semantics: unset, empty,
/// or `0`/`false`/`no`/`off` (case-insensitive) → `false`; any other value →
/// `true`. Avoids clap's `env`+bool footgun where `=0` could enable a flag.
fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let v = v.trim().to_ascii_lowercase();
            !(v.is_empty() || v == "0" || v == "false" || v == "no" || v == "off")
        }
        Err(_) => false,
    }
}

fn run(mut cli: Cli) -> Result<(), String> {
    // Resolve the local UTC offset now, at the single-threaded entry
    // point. `time::UtcOffset::current_local_offset()` deliberately
    // refuses to compute an offset once the process is multithreaded
    // (the spinner / queue-editor threads we spawn per turn would trip
    // it), so we snapshot it here and reuse the cached value for the
    // rest of the session via `local_now`.
    capture_local_offset();

    normalize_embedded_mode(&mut cli);

    // Fold env-var overrides in explicitly (see `env_flag`): the CLI flag OR a
    // truthy env var enables the toggle; a falsey env var never forces it on.
    cli.no_suggestions = cli.no_suggestions || env_flag("DEEPTIDE_NO_SUGGESTIONS");
    cli.no_session_capture = cli.no_session_capture || env_flag("DEEPTIDE_NO_SESSION_CAPTURE");
    cli.no_session_persistence =
        cli.no_session_persistence || env_flag("DEEPTIDE_NO_SESSION_PERSISTENCE");

    if let Some(cwd) = cli.cwd.as_ref() {
        std::env::set_current_dir(cwd)
            .map_err(|error| format!("invalid --cwd {}: {error}", cwd.display()))?;
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));

    // --gui hands off to the native desktop app. It shares this process's cwd
    // (so `--cwd` carries through) and reads the same settings.json / sessions.
    if cli.gui {
        return launch_gui();
    }

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

    // --list-providers: print the built-in provider presets (protocol +
    // default endpoint/model) so users can discover `--provider` values
    // without reading docs.
    if cli.list_providers {
        println!("{}", format_provider_list());
        return Ok(());
    }

    // --doctor is a no-network, no-API-key diagnostic that surfaces the
    // configuration the CLI would use on a real run plus environment
    // health checks. Designed to be the first thing a user runs when
    // something doesn't behave the way they expect. Resolve settings.json
    // (incl. an explicit --settings file) and provider fallbacks *first* so
    // the [model]/[auth] sections report the effective values a real run
    // would use — this is what makes `--doctor --settings X` (or --profile)
    // a reliable way to test alternate api keys/endpoints from the CLI.
    if cli.doctor {
        let cfg = load_config(&cli, &cwd);
        cfg.apply_env();
        apply_config_fallbacks(&mut cli, &cfg);
        println!("{}", run_doctor(&cli, &cwd));
        return Ok(());
    }

    // Load settings.json (global ← project ← local), plus an explicit
    // --settings file on top, then apply as fallbacks. Explicit CLI flags and
    // environment variables always take precedence. With --isolated the
    // file scopes are skipped entirely (see `load_config`).
    if let Some(path) = cli.settings.as_ref()
        && !path.exists()
    {
        return Err(format!("--settings file not found: {}", path.display()));
    }
    let cfg = load_config(&cli, &cwd);
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

    // Install the colour theme process-wide before any rendering. Done once,
    // here, so the syntax highlighter (and future themed consumers) read a
    // stable palette for the whole run. Unknown names fail fast with the valid
    // set rather than silently falling back.
    match deeptide_core::theme::by_name(&cli.theme) {
        Some(theme) => {
            let _ = deeptide_core::theme::set_active(theme);
        }
        None => {
            return Err(format!(
                "invalid theme: {} (choose one of: {})",
                cli.theme,
                deeptide_core::theme::theme_names().join(", ")
            ));
        }
    }

    // Lifecycle hooks (settings.json `hooks`) fire around the agent loop in
    // both interactive and print modes; PreToolUse hooks can block a tool.
    let hooks = deeptide_core::HookEngine::new(cfg.hooks.clone().unwrap_or_default(), &cwd);

    if !cli.print_mode && !cli.read_stdin && cli.input_format == InputFormat::Text {
        // Per-model pricing overrides from settings.json, converted to the
        // per-token rates the cost tracker consumes. Only the interactive REPL
        // surfaces cost (`/cost`, status line), so print mode skips this.
        let pricing_overrides = cfg.pricing_overrides();
        return run_interactive(&cli, permission_mode, pricing_overrides, hooks);
    }

    let stdin = read_stdin_if_needed(&cli)?;
    let prompt = collect_prompt(&cli, stdin.as_deref())?;
    let permission_responses = collect_permission_responses(&cli, stdin.as_deref());
    emit_output(
        &cli,
        &prompt,
        permission_mode,
        &cwd,
        hooks,
        permission_responses,
    )
}

/// Apply `settings.json` values as fallbacks for CLI fields that were not
/// explicitly set by the user.  CLI flags and env vars always win.
/// Resolve the merged configuration for this run. Normally this is
/// `global ← project ← local ← --settings`, but `--isolated` skips the
/// persistent scopes and uses only the explicit `--settings` file (or
/// built-in defaults), so a single invocation is fully reproducible.
fn load_config(cli: &Cli, cwd: &Path) -> deeptide_core::ConfigData {
    if cli.isolated {
        ConfigStore::load_isolated(cli.settings.as_deref())
    } else {
        ConfigStore::load_with_override(cwd, cli.settings.as_deref())
    }
}

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
    // Only adopt the config value when the CLI flag is still on the
    // *current* clap default — otherwise an explicit `--max-turns N` on
    // the command line would silently lose to a stale settings.json.
    // 200 mirrors the clap default just above; bump both together if it
    // changes.
    if cli.max_turns == 200
        && let Some(t) = cfg.max_turns
    {
        cli.max_turns = t;
    }
    // Only override the CLI flag from config when the user is still on
    // the *current default* — otherwise an explicit `--max-output-tokens`
    // would silently lose to a stale settings.json. 65_536 mirrors the
    // clap default just above; bump both together if it changes.
    if cli.max_output_tokens == 65_536
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
    // settings.json `session_end_capture: false` opts out of the
    // end-of-session consolidation pass, unless the user already passed
    // the flag (in which case it's a no-op) or left it enabled.
    if let Some(false) = cfg.session_end_capture {
        cli.no_session_capture = true;
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
            let piped = stdin.map(str::trim).filter(|chunk| !chunk.is_empty());

            match (cli.prompt.as_deref(), piped) {
                // `--read-stdin` appends piped content after the fixed prompt.
                (Some(prompt), Some(extra)) if cli.read_stdin => Ok(format!("{prompt}\n\n{extra}")),
                (Some(prompt), _) => Ok(prompt.to_owned()),
                // No `--prompt`: in any non-interactive path the prompt is the
                // piped stdin.
                (None, Some(extra)) => Ok(extra.to_owned()),
                (None, None) if cli.print_mode || cli.read_stdin => Err(
                    "no prompt provided and stdin is empty; use --prompt or pipe input".to_owned(),
                ),
                (None, None) => Err(
                    "interactive REPL mode requires a terminal; use --print or --embedded"
                        .to_owned(),
                ),
            }
        }
    }
}

/// Rustyline helper that provides tab-completion for `/command` prefixes
/// using `CompletionEngine` and the REPL's registered command list.
struct ReplHelper {
    commands: Vec<CommandCompletionSource>,
    models: Vec<String>,
}

/// Static argument-completion table for slash commands whose first argument
/// is drawn from a small fixed value set. Each entry pairs *all accepted
/// heads* (canonical name + aliases) with the candidate values, so e.g.
/// typing `/perm --` and `/permissions --` both yield the same suggestions.
///
/// New commands with closed-set first arguments should be added here rather
/// than special-cased in `Completer::complete`.
///
/// `/model` and `/help` are handled separately because their value sets are
/// dynamic (built from `known_models()` and the live command registry,
/// respectively).
const FIXED_ARG_SUGGESTIONS: &[(&[&str], &[&str])] = &[
    (
        &["permission", "perm", "permissions"],
        &["--allow", "--deny", "--remove", "--list"],
    ),
    (&["cost"], &["show", "hide", "breakdown"]),
    (&["provider", "profiles"], &["list", "use", "status"]),
    (&["tps", "speed"], &["--json", "--reset"]),
    (&["update", "upgrade"], &["--check", "--force"]),
    (&["dream"], &["run", "status"]),
    (&["cron"], &["list", "delete"]),
    (&["goal", "objective"], &["status", "clear"]),
    (&["config"], &["show"]),
    (&["clear", "cls"], &["--yes"]),
    (&["new"], &["--yes"]),
    (&["compact", "compress"], &["--yes"]),
    (&["reminder", "anchor", "reorient"], &["show", "send"]),
    (&["sessions", "session"], &["latest", "today"]),
    (&["context", "ctx"], &["files", "all"]),
    (&["debug", "dbg"], &["on", "off"]),
    (&["branch"], &["-b"]),
    (
        &["queue"],
        &["list", "add", "pop", "clear", "mode", "single", "batch"],
    ),
    (
        &["tools", "tool"],
        &["--read-only", "--writes", "--all", "--details", "--help"],
    ),
    (
        &["think", "thinking", "reason", "reasoning"],
        &[
            "on", "off", "auto", "low", "medium", "high", "status", "budget",
        ],
    ),
    (&["search", "find", "grep-chat"], &["--regex", "-r"]),
    (
        &["checkpoint", "snap", "snapshot"],
        &[
            "save", "list", "restore", "drop", "clear", "latest", "--help",
        ],
    ),
    (&["rewind", "undo-turn"], &["latest", "last"]),
    (
        &["usage", "tooltime", "telemetry"],
        &["show", "--json", "reset", "--help"],
    ),
    (&["test", "tests"], &["--run", "-r", "--help"]),
    (&["lint", "check"], &["--run", "-r", "--help"]),
    (
        &["auto-compact", "autocompact", "auto_compact"],
        &["on", "off", "status", "threshold", "reset", "--help"],
    ),
];

impl ReplHelper {
    fn new(commands: Vec<CommandCompletionSource>) -> Self {
        // Argument completion for `/model <name>`: the built-in catalog plus the
        // two shorthand aliases the command accepts.
        let mut models = vec![String::from("flash"), String::from("pro")];
        models.extend(
            deeptide_core::known_models()
                .into_iter()
                .map(|model| model.name.to_owned()),
        );
        Self { commands, models }
    }

    /// Walk the dynamic argument-completion specs (model, help) and the
    /// static `FIXED_ARG_SUGGESTIONS` table, returning the first match.
    /// Each `command` head is tried against `value_completions`, which
    /// already enforces that the typed token sits in the first-arg slot.
    fn arg_completion(&self, line: &str, pos: usize) -> Option<(usize, Vec<Pair>)> {
        // Dynamic: /model <name>
        let model_refs: Vec<&str> = self.models.iter().map(String::as_str).collect();
        for head in ["model", "m"] {
            if let Some(values) =
                CompletionEngine::value_completions(line, pos, head, &model_refs, 8)
            {
                return Some(pairs_from_values(values.token_start, &values.candidates));
            }
        }

        // Dynamic: /help <command-name> — surfaces every registered command
        // (and its aliases) so /help <Tab> works as a "list everything"
        // affordance distinct from /help showing categorised output.
        let mut help_targets: Vec<String> = Vec::with_capacity(self.commands.len() * 2);
        for cmd in &self.commands {
            help_targets.push(cmd.name.clone());
            help_targets.extend(cmd.aliases.clone());
        }
        let help_refs: Vec<&str> = help_targets.iter().map(String::as_str).collect();
        for head in ["help", "h", "?"] {
            if let Some(values) =
                CompletionEngine::value_completions(line, pos, head, &help_refs, 8)
            {
                return Some(pairs_from_values(values.token_start, &values.candidates));
            }
        }

        // Static fixed-value table.
        for (heads, values) in FIXED_ARG_SUGGESTIONS {
            for head in *heads {
                if let Some(result) =
                    CompletionEngine::value_completions(line, pos, head, values, 8)
                {
                    return Some(pairs_from_values(result.token_start, &result.candidates));
                }
            }
        }

        None
    }

    /// Compute the inline ghost-text hint for a partially-typed slash
    /// command. Returns the *tail* (the characters that would be inserted
    /// if the user accepted the hint) when there is exactly one prefix
    /// match in the registered command set. Multiple matches → no hint
    /// (the user should press Tab to see the list and disambiguate);
    /// substring/fuzzy matches → no hint (would be visually misleading);
    /// cursor not at end of line → no hint (mid-edit cursor noise).
    ///
    /// Exposed at module scope as `compute_hint` so it's unit-testable
    /// without a full rustyline `Context`.
    fn compute_hint(&self, line: &str, pos: usize) -> Option<String> {
        compute_hint(line, pos, &self.commands)
    }
}

/// Tab completion for `@<path>` references. Returns `Some((start, pairs))`
/// when the cursor is currently inside an `@`-prefixed token; returns
/// `None` to delegate back to the slash-command / argument completers.
///
/// Behaviour mirrors what the user gets from Cursor / Claude Code:
///   * `@<Tab>` lists everything in the cwd
///   * `@src/<Tab>` lists everything in `src/`
///   * `@src/ma<Tab>` filters `src/` entries by the `ma` prefix
///   * Hidden + heavyweight build dirs are skipped so the listing
///     stays tractable on large monorepos
///
/// The replacement value is the path WITHOUT the leading `@`, since
/// rustyline's `Completer` API replaces from `token_start` and the
/// caller has already eaten the `@` (we set `token_start = at_pos + 1`).
fn at_path_completion(line: &str, pos: usize) -> Option<(usize, Vec<Pair>)> {
    // Identify the @-token under the cursor. Scan backward from `pos`
    // until we hit a whitespace, start of line, or another `@`.
    let line_bytes = line.as_bytes();
    if pos > line_bytes.len() {
        return None;
    }
    let mut start = pos;
    while start > 0 {
        let c = line_bytes[start - 1];
        if matches!(c, b' ' | b'\t' | b'\n' | b'\r') {
            break;
        }
        start -= 1;
        if c == b'@' {
            // Found the @ that anchors this token. Make sure the @ is
            // at a word boundary, otherwise we don't treat this as a
            // file reference (matches the parser semantics in core).
            if start > 0 {
                let before = line_bytes[start - 1];
                let is_boundary = matches!(
                    before,
                    b' ' | b'\t' | b'\n' | b'\r' | b'(' | b'[' | b'{' | b'\'' | b'"' | b'`' | b','
                );
                if !is_boundary {
                    return None;
                }
            }
            let at_pos = start;
            let path_prefix = &line[at_pos + 1..pos];
            let candidates = list_filesystem_candidates(path_prefix);
            if candidates.is_empty() {
                return Some((at_pos + 1, vec![]));
            }
            let pairs = candidates
                .into_iter()
                .map(|(repl, _display)| Pair { replacement: repl })
                .collect();
            return Some((at_pos + 1, pairs));
        }
    }
    None
}

/// Enumerate filesystem entries matching `prefix`, returned as
/// `(replacement, display)` pairs sorted by relevance (score asc,
/// mtime desc, alpha). The replacement is the path fragment that
/// should go in after the `@`; the display has trailing-`/` for
/// directories so the user can see at a glance what will be inlined
/// vs what needs further navigation.
///
/// Matching tiers (T1.3)
/// =====================
///
/// We keep three relevance bands so the most expected hits land at
/// the top of the Tab list and the inline `@` palette hint:
///
/// 1. **Prefix match** (score 0) — `name_lower.starts_with(prefix_lower)`.
///    Backwards-compatible with the old prefix-only behaviour: typing
///    `@RE<Tab>` still surfaces `README.md` first.
/// 2. **Substring match** (score 1) — `name_lower.contains(prefix_lower)`.
///    `@modelcfg` finds `agent_modelcfg.rs`, `@.test` finds `unit.test.ts`.
/// 3. **Subsequence match** (score 2) — every char of the prefix
///    appears in `name` in order (with gaps). `@modtest` finds
///    `models/test.rs`. This is the "fzf-style" tier and intentionally
///    last so it doesn't shadow cleaner matches.
///
/// Empty `name_part` skips matching entirely and just lists the
/// directory.
fn list_filesystem_candidates(prefix: &str) -> Vec<(String, String)> {
    list_filesystem_candidates_ranked(prefix)
        .into_iter()
        .map(|m| (m.replacement, m.display))
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FsCandidate {
    replacement: String,
    display: String,
    /// 0 = prefix, 1 = substring, 2 = subsequence. Lower is better.
    score: u8,
    /// Seconds since UNIX epoch from the entry's mtime, 0 when
    /// metadata read failed. Used as a tiebreaker so recently-edited
    /// files float above stale ones at the same score.
    mtime_secs: i64,
    /// Lower-cased name without the directory prefix — used for the
    /// final alphabetical tiebreaker.
    sort_key: String,
}

fn list_filesystem_candidates_ranked(prefix: &str) -> Vec<FsCandidate> {
    use std::path::Path;

    const MAX_CANDIDATES: usize = 64;
    // Static deny-list for directory bases we never want to surface
    // as completion candidates — they're huge, transient, or never
    // useful to inline. Matching is by name only at the immediate
    // directory level.
    const SKIP_NAMES: &[&str] = &[
        "node_modules",
        "target",
        ".git",
        ".clone",
        ".claire",
        "dist",
        "build",
        ".next",
        ".cache",
        ".turbo",
        "venv",
        ".venv",
        "__pycache__",
        ".DS_Store",
    ];

    // Split the prefix into directory portion + filename prefix.
    // `prefix == "src/foo"` → dir = "src", filename_prefix = "foo".
    // `prefix == "foo"` → dir = "", filename_prefix = "foo".
    // `prefix == "src/"` → dir = "src", filename_prefix = "".
    let (dir_part, name_part) = match prefix.rfind('/') {
        Some(idx) => (&prefix[..=idx], &prefix[idx + 1..]),
        None => ("", prefix),
    };

    let cwd = std::env::current_dir().ok();
    let search_dir: std::path::PathBuf = if dir_part.is_empty() {
        cwd.clone().unwrap_or_else(|| Path::new(".").to_path_buf())
    } else {
        let candidate = Path::new(dir_part);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            cwd.clone()
                .unwrap_or_else(|| Path::new(".").to_path_buf())
                .join(candidate)
        }
    };

    let entries = match std::fs::read_dir(&search_dir) {
        Ok(it) => it,
        Err(_) => return Vec::new(),
    };

    let mut out: Vec<FsCandidate> = Vec::new();
    let name_prefix_lower = name_part.to_ascii_lowercase();

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        let name_str = match file_name.to_str() {
            Some(s) => s.to_owned(),
            None => continue,
        };
        // Filter: skip names in the deny list outright, and skip
        // hidden files unless the user explicitly typed a leading dot.
        if SKIP_NAMES.contains(&name_str.as_str()) {
            continue;
        }
        if name_str.starts_with('.') && !name_part.starts_with('.') {
            continue;
        }

        // Score this entry against the typed name prefix. Empty
        // prefix → score 0 (everything is a match). Otherwise pick
        // the BEST tier the name qualifies for.
        let name_lower = name_str.to_ascii_lowercase();
        let score = if name_prefix_lower.is_empty() || name_lower.starts_with(&name_prefix_lower) {
            0
        } else if name_lower.contains(&name_prefix_lower) {
            1
        } else if is_subsequence(&name_prefix_lower, &name_lower) {
            2
        } else {
            continue;
        };

        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        let mtime_secs = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut replacement = String::with_capacity(dir_part.len() + name_str.len() + 1);
        replacement.push_str(dir_part);
        replacement.push_str(&name_str);
        if is_dir {
            replacement.push('/');
        }

        let display = if is_dir {
            format!("{name_str}/")
        } else {
            name_str.clone()
        };
        out.push(FsCandidate {
            replacement,
            display,
            score,
            mtime_secs,
            sort_key: name_lower,
        });

        if out.len() >= MAX_CANDIDATES {
            break;
        }
    }

    out.sort_by(|a, b| {
        // Score ascending (0 prefix > 1 substring > 2 subsequence).
        a.score
            .cmp(&b.score)
            // Then most-recently-modified first — this nudges files
            // the user is actively editing to the top of the list
            // within their relevance band.
            .then_with(|| b.mtime_secs.cmp(&a.mtime_secs))
            // Finally alphabetical for deterministic output.
            .then_with(|| a.sort_key.cmp(&b.sort_key))
    });

    out
}

/// Subsequence check: every char of `needle` appears in `haystack`
/// in order (gaps allowed). Both inputs are expected pre-lowercased
/// by the caller. Empty needle → always matches.
fn is_subsequence(needle: &str, haystack: &str) -> bool {
    let mut h_iter = haystack.chars();
    'outer: for nc in needle.chars() {
        for hc in h_iter.by_ref() {
            if hc == nc {
                continue 'outer;
            }
        }
        return false;
    }
    true
}

fn pairs_from_values(token_start: usize, values: &[String]) -> (usize, Vec<Pair>) {
    let pairs = values
        .iter()
        .map(|value| Pair {
            replacement: value.clone(),
        })
        .collect();
    (token_start, pairs)
}

/// Pure-function inline-hint logic; see `ReplHelper::compute_hint` for the
/// behavioural contract. Lives outside the impl block so unit tests can
/// drive it with a synthetic command list and no rustyline state.
fn compute_hint(line: &str, pos: usize, commands: &[CommandCompletionSource]) -> Option<String> {
    if pos != line.chars().count() {
        return None;
    }

    // ── @path file-mention palette hint (T1.3) ──────────────────────
    // Detect a `@<prefix>` token under the cursor and, if it has
    // multiple filesystem matches, surface them as a dim inline
    // palette — same pattern as the slash-command palette (T1.2) so
    // users learn one convention. We deliberately run this BEFORE
    // the slash-command lookup so an `@`-anchored line never falls
    // through to "no candidates" because of the `/` prefix check.
    if let Some(hint) = compute_at_path_hint(line, pos) {
        return Some(hint);
    }

    let result = CompletionEngine::command_completions(line, pos, commands, 64)?;

    // Only hint on clean *prefix* matches against either the canonical name
    // (score == 0) or an alias (score == 1). Substring/contains hits (scores
    // 2 / 3) would jump the hint to a non-adjacent character range which is
    // visually misleading.
    let prefix_hits: Vec<&CommandCompletionCandidate> =
        result.candidates.iter().filter(|c| c.score <= 1).collect();

    // ── Single match path ────────────────────────────────────────────
    // Exactly one prefix match → return the *tail* of the completion
    // so rustyline renders it as ghost text after the cursor
    // (e.g. typing `/exi` shows a dim `t`).
    if prefix_hits.len() == 1 {
        let candidate = prefix_hits[0];
        let target = &candidate.matched_text;
        let typed_len = result.typed.chars().count();
        let target_len = target.chars().count();
        if target_len <= typed_len {
            return None;
        }
        let tail: String = target.chars().skip(typed_len).collect();
        return Some(tail);
    }

    // ── Multi-match discovery path (T1.2 slash-command palette) ─────
    // Two or more prefix matches → render a compact in-line palette
    // so the user can SEE what's available without pressing Tab. The
    // palette is appended after the cursor as dim ghost text, the
    // same surface rustyline already uses for completion hints — no
    // overlay / scroll-region acrobatics needed for the inline tier.
    //
    // We deliberately keep this single-line: rustyline's hint surface
    // is positioned inline and a multi-line value would break layout
    // mid-prompt. A future iteration will pair this with a proper
    // pop-up panel above the input row once we have the native
    // raw-mode line editor in T2.2.
    if prefix_hits.len() >= 2 {
        return Some(multi_command_palette_hint(&prefix_hits));
    }

    None
}

/// Build an inline palette hint for an `@<prefix>` file mention.
/// Returns `None` when the cursor isn't inside an `@`-anchored token
/// or when there's nothing useful to surface (zero / exactly one
/// match — Tab handles the single-match case fine).
///
/// Format (dim ghost-text after the cursor):
///
/// ```text
///   main.rs · queue_editor.rs · diff_preview.rs  +5 more (Tab)
/// ```
///
/// Note the items here are *file names* not slash commands — the
/// `·` separator and `(Tab)` teach-text intentionally mirror the
/// slash-command palette (T1.2) so users learn one convention.
fn compute_at_path_hint(line: &str, pos: usize) -> Option<String> {
    // We re-implement the @-token boundary scan instead of calling
    // `at_path_completion` because the latter performs filesystem
    // I/O for every keystroke and we want to avoid that when the
    // cursor *isn't* in an @-token. Cheap byte scan first; bail
    // before any syscalls when there's no @-anchor.
    let line_bytes = line.as_bytes();
    if pos > line_bytes.len() {
        return None;
    }
    let mut at_pos: Option<usize> = None;
    let mut idx = pos;
    while idx > 0 {
        let c = line_bytes[idx - 1];
        if matches!(c, b' ' | b'\t' | b'\n' | b'\r') {
            break;
        }
        idx -= 1;
        if c == b'@' {
            if idx > 0 {
                let before = line_bytes[idx - 1];
                let is_boundary = matches!(
                    before,
                    b' ' | b'\t' | b'\n' | b'\r' | b'(' | b'[' | b'{' | b'\'' | b'"' | b'`' | b','
                );
                if !is_boundary {
                    return None;
                }
            }
            at_pos = Some(idx);
            break;
        }
    }
    let at_pos = at_pos?;

    let path_prefix = &line[at_pos + 1..pos];
    let candidates = list_filesystem_candidates_ranked(path_prefix);

    // No candidates → silent (consistent with slash-command behaviour
    // when no commands match). Single candidate → also silent because
    // rustyline's Tab will autocomplete immediately, and a one-item
    // palette is just noise.
    if candidates.len() < 2 {
        return None;
    }

    Some(at_path_palette_hint(&candidates))
}

/// Render up to four candidate display names as a dim inline
/// palette. Mirrors [`multi_command_palette_hint`]'s format so users
/// who learn the slash-command palette get the same convention here.
fn at_path_palette_hint(candidates: &[FsCandidate]) -> String {
    const MAX_INLINE: usize = 4;
    let total = candidates.len();
    let take = total.min(MAX_INLINE);
    let names: Vec<&str> = candidates
        .iter()
        .take(take)
        .map(|c| c.display.as_str())
        .collect();
    let head = names.join(" · ");
    if total > MAX_INLINE {
        format!(
            "  {head}  +{remaining} more (Tab)",
            remaining = total - take
        )
    } else {
        format!("  {head}  (Tab)")
    }
}

/// Build an inline "discovery palette" hint summarising N matching
/// slash commands. Format (rendered dim by [`Highlighter::highlight_hint`]):
///
/// ```text
///   /model · /mode · /mcp · /mac-diagnose  +N more (Tab)
/// ```
///
/// The cursor sits at the END of the user's typed input; this hint
/// is appended after a two-space buffer so the typed prefix and the
/// suggestion list don't visually mash together. The first space is
/// intentionally part of the returned string so callers don't need
/// to know about the spacing convention.
fn multi_command_palette_hint(prefix_hits: &[&CommandCompletionCandidate]) -> String {
    // Show at most this many command names in the inline palette.
    // Above this it'd wrap on standard 80-column terminals and the
    // last entries would scroll off the right edge.
    const MAX_INLINE: usize = 4;
    let total = prefix_hits.len();
    let take = total.min(MAX_INLINE);

    let names: Vec<String> = prefix_hits
        .iter()
        .take(take)
        .map(|c| format!("/{}", c.name))
        .collect();
    let head = names.join(" · ");

    if total > MAX_INLINE {
        format!(
            "  {head}  +{remaining} more (Tab)",
            remaining = total - take
        )
    } else {
        format!("  {head}  (Tab)")
    }
}

/// Completion provider for the custom raw-mode [`prompt_editor`]. Mirrors
/// the rustyline [`Completer::complete`] resolution order — `@path` first,
/// then slash-command names, then the argument-completion table — but
/// returns plain replacement strings (the editor owns the
/// longest-common-prefix insert and the candidate listing).
fn editor_completion(
    helper: &ReplHelper,
    line: &str,
    pos: usize,
) -> prompt_editor::CompletionResult {
    if let Some((start, pairs)) = at_path_completion(line, pos) {
        return prompt_editor::CompletionResult {
            start,
            candidates: pairs.into_iter().map(|p| p.replacement).collect(),
        };
    }
    if let Some(result) = CompletionEngine::command_completions(line, pos, &helper.commands, 64) {
        let candidates = result.candidates.iter().map(|c| c.replacement()).collect();
        return prompt_editor::CompletionResult {
            start: result.token_start,
            candidates,
        };
    }
    if let Some((start, pairs)) = helper.arg_completion(line, pos) {
        return prompt_editor::CompletionResult {
            start,
            candidates: pairs.into_iter().map(|p| p.replacement).collect(),
        };
    }
    prompt_editor::CompletionResult {
        start: pos,
        candidates: Vec::new(),
    }
}

/// Inline ghost-text hint for the custom prompt editor. Delegates to the
/// shared [`compute_hint`] so the slash-command tail / palette and the
/// `@path` palette behave identically to the rustyline hinter.
fn editor_hint(helper: &ReplHelper, line: &str, pos: usize) -> Option<String> {
    helper.compute_hint(line, pos)
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

    // Set the terminal window/icon title from --name/-n (OSC 0). Best-effort:
    // terminals that don't understand the sequence ignore it. Mirrors the
    // Swift app's `--name`, which also titles the session.
    if let Some(name) = cli
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        let _ = write!(stdout, "\x1b]0;{name}\x07");
        let _ = stdout.flush();
    }

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
    // Tracks whether we've printed the one-time "thinking" header for a
    // reasoning model's chain-of-thought this turn (DeepSeek-reasoner et al.).
    let did_think: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

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
    let did_think_handler = Arc::clone(&did_think);
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
    let streaming_md_for_retry = Arc::clone(&streaming_md);
    let did_stream_for_retry = Arc::clone(&did_stream);
    let spinner_stop_for_retry = Arc::clone(&spinner_stop);
    let spinner_lock_for_retry = Arc::clone(&spinner_lock);
    let output_started_for_retry = Arc::clone(&output_started);
    let use_color_for_retry = use_color;

    // Late-bound slot for the per-session message queue. We can't capture
    // `ReplSession::message_queue_handle()` directly here because the REPL
    // is created several blocks below; instead, the streaming handler holds
    // a `OnceLock` and pulls the queue out the first time it sees one. The
    // CLI fills the slot right after constructing the REPL.
    //
    // Wrapping in `Arc` so the handler closure can hold one clone and the
    // post-construction "set" site holds another.
    let queue_slot: Arc<OnceLock<Arc<Mutex<deeptide_core::MessageQueue>>>> =
        Arc::new(OnceLock::new());
    let queue_slot_for_handler = Arc::clone(&queue_slot);
    let spinner_lock_for_queue = Arc::clone(&spinner_lock);
    let use_color_for_queue = use_color;

    // Mutex-free signal that the raw-mode queue editor currently
    // owns stdin (set by the REPL loop at the start of each turn,
    // cleared when the editor thread joins). When this is true the
    // streaming handler skips the legacy cooked-mode
    // `drain_pending_stdin_into_queue` so the two readers don't race
    // for the same bytes. The Arc lets the handler see updates we
    // make from the main thread later.
    let raw_editor_active: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let raw_editor_active_for_handler = Arc::clone(&raw_editor_active);

    // Cooperative cancellation flag for the current turn. Shared four ways:
    //   * the agent loop (between-step / post-tool checks),
    //   * the tool context (shell/monitor poll loops kill their child),
    //   * the streaming backend (aborts an in-flight SSE read),
    //   * the raw-mode queue editor pump, which flips it on an empty-draft
    //     Ctrl-C during a turn.
    // Reset to `false` at the start of every turn so a previous cancellation
    // never bleeds into the next one. Ctrl-C cancels the turn (single press);
    // exiting the REPL is the double-press gesture at the idle prompt.
    let turn_interrupt: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    // Suspend/ack pair for the raw-mode queue editor. Set from the
    // permission-prompt path (and any other site that needs
    // exclusive cooked-mode stdin) to make the pump release raw
    // mode without joining the thread; the pump raises `suspended`
    // once raw mode is actually back to cooked so the caller can
    // read safely. Both flags reset when the editor thread exits
    // for the turn.
    let editor_suspend: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    let editor_suspended: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    // Shift+Tab pressed inside the raw-mode editor sets this flag.
    // The CLI consumes it in two places:
    //   1. In `ask_callback` (next permission prompt) — return
    //      `AllowAndSetMode(Bypass)` so the prompt resolves as
    //      a fast "yes + YOLO".
    //   2. After `repl.submit()` returns (turn boundary) — cycle
    //      to the next mode like the between-turn rustyline
    //      handler does, in case no permission prompt fired this
    //      turn.
    // Whichever consumer fires first wins; both call `.swap(false)`
    // so the flag can't be applied twice.
    let editor_mode_cycle: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));

    // Streamed-character counter for the live spinner stats panel.
    // The streaming handler atomically adds each TextDelta /
    // ToolUseInputDelta payload's char count; `run_spinner` divides
    // by 4 and renders it as `↓ N tokens`. Reset to 0 at the start
    // of each turn (just before spawning the spinner) so the
    // counter shows *this turn's* throughput, not cumulative.
    //
    // `chars` (Unicode codepoints) rather than `bytes` because the
    // estimate we care about is "perceived progress", and UTF-8
    // byte count over-weights CJK by 3x relative to ASCII. Chars
    // are wrong for tokens too, but in a consistent direction.
    let stream_chars_received: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let stream_chars_handler = Arc::clone(&stream_chars_received);

    // Uploaded-context token count for the spinner's `↑` segment. Set once per
    // turn from the model's `message_start` echo (Anthropic reports the prompt
    // size — input + cache — the moment the stream opens), so the user sees how
    // much context this turn shipped, alongside the `↓` output that follows.
    // Stays 0 for protocols that don't report input until completion (OpenAI),
    // in which case the spinner simply shows `↓` only — no phantom `↑ 0`.
    let prompt_tokens_total: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let prompt_tokens_handler = Arc::clone(&prompt_tokens_total);

    // Shared "current tool name / preview" state. Two writers:
    //   * the streaming handler sets it to `Preparing <name>` on
    //     `ToolUseStart` (model committed to a tool, args streaming);
    //   * the agent_loop's `tool_progress_callback` (further down)
    //     overwrites it with the rich preview when the tool actually
    //     runs (`⠦ Bash(npm test)`).
    // The spinner thread reads it as `phase`. `None` = no tool in
    // flight; both writers clear back to `None` on their respective
    // exit events.
    let current_tool_phase: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let current_tool_phase_for_handler = Arc::clone(&current_tool_phase);
    let current_tool_phase_for_callback = Arc::clone(&current_tool_phase);

    // Turn-scoped count of tools that have FINISHED this turn. Surfaced in the
    // progress line ("· N done") so a long, multi-tool turn never reads as idle
    // between tool rounds. Reset to 0 at the start of each turn.
    let tools_done: Arc<AtomicUsize> = Arc::new(AtomicUsize::new(0));
    let tools_done_for_callback = Arc::clone(&tools_done);
    let tools_done_for_handler = Arc::clone(&tools_done);

    // Live "preparing tool" ticker — see [`LiveToolArgsTicker`] for
    // the full rationale. Mid-turn, when the model has already
    // streamed some preamble text (which kills the main spinner) and
    // then commits to a tool_use, we spawn a small dedicated thread
    // here to keep the elapsed/tokens panel ticking while args
    // stream in.
    //
    // We share it via Arc<Mutex<Option<_>>> because:
    //   * `streaming_handler` (called from the backend's SSE thread)
    //     needs to spawn / stop / replace it across ToolUseStart and
    //     BlockStop events.
    //   * The post-stream cleanup path (on the REPL thread) also
    //     needs access so it can drain a leftover ticker if the
    //     stream was truncated without a matching BlockStop.
    let live_tool_args: Arc<Mutex<Option<LiveToolArgsTicker>>> = Arc::new(Mutex::new(None));
    let live_tool_args_handler = Arc::clone(&live_tool_args);
    let live_tool_args_lock_for_handler = Arc::clone(&spinner_lock);

    // Execution-phase ticker wiring for `tool_progress_callback`. The
    // streaming handler above only animates while the model *streams tool
    // args*; once `BlockStop` fires the args are in and the tool actually
    // runs — which for a shell / Chrome PDF print can be minutes of total
    // silence after the main spinner died on the first streamed token.
    // These clones let the callback print a live "▶ <tool>" line and drive
    // a heartbeat ticker during that execution window.
    let did_stream_for_callback = Arc::clone(&did_stream);
    let spinner_lock_for_callback = Arc::clone(&spinner_lock);
    let stream_chars_for_callback = Arc::clone(&stream_chars_received);
    let live_tool_args_for_callback = Arc::clone(&live_tool_args);
    let use_color_for_callback = use_color;
    let live_tool_args_chars_for_handler = Arc::clone(&stream_chars_received);
    let live_tool_args_use_color = use_color;

    let streaming_handler: StreamingHandler = Arc::new(move |event: &StreamingEvent| {
        match event {
            StreamingEvent::TextDelta { delta, .. } => {
                // Live progress counter for the spinner's stats
                // panel. Counted before we touch the spinner /
                // markdown locks so a contended lock never costs
                // us a visible "tokens received" tick.
                stream_chars_handler.fetch_add(delta.chars().count(), Ordering::Relaxed);
                let first = !did_stream_handler.swap(true, Ordering::Relaxed);
                // Stop the spinner under the lock so it cannot draw a frame
                // over the text we are about to print.
                if let Ok(_guard) = spinner_lock_handler.lock() {
                    spinner_stop_handler.store(true, Ordering::Relaxed);
                    output_started_handler.store(true, Ordering::Relaxed);
                    let mut out = io::stdout();
                    if first {
                        // If a reasoning model streamed "thinking" text just
                        // above, terminate that dim block with a newline first —
                        // otherwise the spinner-line erase below would wipe the
                        // last (non-newline-terminated) line of reasoning.
                        if did_think_handler.load(Ordering::Relaxed) {
                            let _ = out.write_all(b"\x1b[0m\n");
                        }
                        // Erase the spinner line before the first streamed
                        // token, then drop in the assistant role header so
                        // the streaming output reads as a labelled block.
                        let _ = out.write_all(b"\r\x1b[2K");
                        let _ = out.write_all(
                            chrome::render_assistant_header(use_color_for_retry).as_bytes(),
                        );
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
            StreamingEvent::ToolUseInputDelta { partial_json, .. } => {
                // Tool-argument JSON is still model output — count
                // its chars so the spinner's "↓ N tokens" panel
                // keeps ticking during long tool-args generation
                // (e.g. a multi-kilobyte Write payload). Without
                // this the user sees the elapsed clock climb but
                // the token counter freeze, which reads as
                // "stuck".
                stream_chars_handler.fetch_add(partial_json.chars().count(), Ordering::Relaxed);
            }
            StreamingEvent::ToolUseStart { index, name, .. } => {
                // Two-headed response to "model is about to stream
                // tool args":
                //
                //  (a) Update the *shared* `current_tool_phase` so
                //      the main spinner (when still alive — i.e. no
                //      text streamed yet this turn) flips its
                //      activity label from a generic "Thinking" to
                //      "Preparing <name>". This is the cheap case:
                //      a single mutex write, no new threads.
                //
                //  (b) If text already streamed this turn, the main
                //      spinner is dead (we stopped it on the first
                //      `TextDelta`). The user would otherwise see
                //      total silence while a multi-kilobyte tool
                //      payload streams in. Drop a one-line live
                //      ticker on a fresh row below the streamed
                //      text and start animating it with the rich
                //      "Preparing <name> · Ns · ↓ N tokens" panel.
                if let Ok(mut phase) = current_tool_phase_for_handler.lock() {
                    *phase = Some(format!("Preparing {name}"));
                }
                if did_stream_handler.load(Ordering::Relaxed) {
                    // Acquire the same lock the spinner & text writers use, then:
                    //  1. Flush any partial narration line held by the markdown
                    //     renderer, so the model's between-tool text (which often
                    //     arrives without a trailing newline) doesn't run on into
                    //     the next text segment after the tool.
                    //  2. Write a single `\n` to drop us onto a fresh line below
                    //     the assistant markdown, then spawn the ticker.
                    if let Ok(_guard) = live_tool_args_lock_for_handler.lock() {
                        let mut out = io::stdout();
                        if let Ok(mut renderer) = streaming_md_handler.lock() {
                            let pending = renderer.flush();
                            if !pending.is_empty() {
                                let _ = out.write_all(pending.as_bytes());
                            }
                        }
                        let _ = out.write_all(b"\n");
                        let _ = out.flush();
                    }
                    let stop = Arc::new(AtomicBool::new(false));
                    let stop_for_thread = Arc::clone(&stop);
                    let lock_for_thread = Arc::clone(&live_tool_args_lock_for_handler);
                    let chars_for_thread = Arc::clone(&live_tool_args_chars_for_handler);
                    let baseline = live_tool_args_chars_for_handler.load(Ordering::Relaxed);
                    let phase_for_thread = format!("Preparing {name}");
                    let color = live_tool_args_use_color;
                    let done_for_thread = Arc::clone(&tools_done_for_handler);
                    let handle = thread::spawn(move || {
                        run_tool_args_ticker(
                            &stop_for_thread,
                            &lock_for_thread,
                            &chars_for_thread,
                            baseline,
                            &phase_for_thread,
                            Instant::now(),
                            color,
                            &done_for_thread,
                        );
                    });
                    if let Ok(mut slot) = live_tool_args_handler.lock() {
                        // If a previous ticker is still around
                        // (shouldn't be — BlockStop is the canonical
                        // exit) tear it down before replacing so we
                        // don't leak a thread.
                        if let Some(old) = slot.take() {
                            old.stop.store(true, Ordering::Relaxed);
                            if let Some(h) = old.handle {
                                let _ = h.join();
                            }
                        }
                        *slot = Some(LiveToolArgsTicker {
                            stop,
                            handle: Some(handle),
                            block_index: *index,
                        });
                    }
                }
            }
            StreamingEvent::BlockStop { index } => {
                // Whichever content block just closed, clear the
                // matching ticker. Index-matching keeps us from
                // accidentally killing the ticker for tool_use #2
                // when text-block #1 closes (the model emits text
                // blocks too, each with its own BlockStop).
                let to_join = if let Ok(mut slot) = live_tool_args_handler.lock() {
                    match slot.as_ref() {
                        Some(active) if active.block_index == *index => slot.take(),
                        _ => None,
                    }
                } else {
                    None
                };
                if let Some(mut ticker) = to_join {
                    ticker.stop.store(true, Ordering::Relaxed);
                    if let Some(h) = ticker.handle.take() {
                        let _ = h.join();
                    }
                    // Clear the "Preparing <name>" phase that the
                    // ToolUseStart arm installed so the main
                    // spinner (if it ever resumes) doesn't render a
                    // stale tool name. The tool_progress callback
                    // will repopulate it when the tool actually
                    // runs.
                    if let Ok(mut phase) = current_tool_phase_for_handler.lock() {
                        *phase = None;
                    }
                }
            }
            StreamingEvent::MessageDelta {
                stop_reason: Some(reason),
                ..
            } => {
                // Surface our synthetic `deeptide:stream-retry:N/M` signal
                // as a visible REPL notice. Without this the user sees a
                // ~400 ms+ silent backoff with no clue why; with it they
                // get "↻ retry 1/3 — stream cut, reconnecting" and a
                // fresh spinner cycle.
                //
                // Real Anthropic stop_reasons (end_turn, tool_use, …)
                // bypass via `parse_stream_retry_signal` returning None.
                if let Some(notice) = deeptide_core::parse_stream_retry_signal(reason) {
                    render_retry_notice(
                        &notice,
                        use_color_for_retry,
                        &did_stream_for_retry,
                        &output_started_for_retry,
                        &spinner_stop_for_retry,
                        &spinner_lock_for_retry,
                        &streaming_md_for_retry,
                    );
                }
            }
            StreamingEvent::MessageStart {
                input_tokens,
                cache_read_tokens,
                cache_creation_tokens,
                ..
            } => {
                // Record the uploaded-context size for the spinner's `↑`
                // segment. Sum the full prompt the turn shipped: fresh input +
                // cache-read + cache-creation tokens (all count as "uploaded
                // context", just priced differently). Reported immediately by
                // Anthropic at stream open.
                prompt_tokens_handler.store(
                    input_tokens + cache_read_tokens + cache_creation_tokens,
                    Ordering::Relaxed,
                );
            }
            StreamingEvent::ThinkingDelta { delta } => {
                // Reasoning-model chain-of-thought (DeepSeek-reasoner et al.),
                // shown dimmed and ABOVE the answer so it reads as the model
                // "thinking out loud" before responding. Rendered as raw dim
                // text — deliberately NOT through the markdown renderer, whose
                // line-buffered state belongs to the answer stream that follows.
                // Each delta is wrapped in its own dim run (and closed) so no
                // open SGR leaks into the answer's TextDelta that comes next.
                stream_chars_handler.fetch_add(delta.chars().count(), Ordering::Relaxed);
                let first = !did_think_handler.swap(true, Ordering::Relaxed);
                if let Ok(_guard) = spinner_lock_handler.lock() {
                    spinner_stop_handler.store(true, Ordering::Relaxed);
                    output_started_handler.store(true, Ordering::Relaxed);
                    let mut out = io::stdout();
                    if first {
                        let _ = out.write_all(b"\r\x1b[2K");
                        let header = if use_color_for_retry {
                            "\x1b[2m\x1b[3m\u{1f4ad} thinking\x1b[0m\n"
                        } else {
                            "\u{1f4ad} thinking\n"
                        };
                        let _ = out.write_all(header.as_bytes());
                    }
                    if use_color_for_retry {
                        let _ = out.write_all(b"\x1b[2m");
                        let _ = out.write_all(delta.as_bytes());
                        let _ = out.write_all(b"\x1b[0m");
                    } else {
                        let _ = out.write_all(delta.as_bytes());
                    }
                    let _ = out.flush();
                }
            }
            _ => {}
        }

        // Mid-turn type-ahead capture for the `/queue` feature. After
        // every SSE event, opportunistically peek stdin (non-blocking,
        // libc::poll with timeout=0 on Unix; no-op on other platforms)
        // and consume any complete line the user typed during the turn.
        // The cost of the peek is microseconds even when stdin is idle,
        // so we don't gate it on the event kind.
        //
        // Suppressed when the raw-mode queue editor is active for the
        // turn: that path owns stdin under different termios settings
        // and would race with us byte-for-byte.
        if !raw_editor_active_for_handler.load(Ordering::Relaxed)
            && let Some(queue) = queue_slot_for_handler.get()
        {
            drain_pending_stdin_into_queue(queue, &spinner_lock_for_queue, use_color_for_queue);
        }
    });

    let configured = configured_backend_with_handler(
        cli,
        Some(streaming_handler),
        Some(Arc::clone(&turn_interrupt)),
    )?;
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
    //
    // Suspends the raw-mode queue editor (if active) for the duration of
    // the prompt — otherwise the editor's pump thread reads stdin in raw
    // mode and the user's `y` / `a` keystroke is captured as part of the
    // queue buffer instead of reaching the prompt's `read_line`.
    let ask_spinner_stop = Arc::clone(&spinner_stop);
    let ask_spinner_lock = Arc::clone(&spinner_lock);
    let ask_output_started = Arc::clone(&output_started);
    let ask_use_color = use_color;
    let ask_editor_suspend = Arc::clone(&editor_suspend);
    let ask_editor_suspended = Arc::clone(&editor_suspended);
    let ask_raw_editor_active = Arc::clone(&raw_editor_active);
    let ask_editor_mode_cycle = Arc::clone(&editor_mode_cycle);
    let ask_callback: deeptide_core::PermissionAskCallback = Arc::new(
        move |tool_call: &ToolCall| {
            // Shift+Tab pressed mid-turn before this prompt fired
            // — resolve as a fast "yes + YOLO" without showing the
            // [y]es/[n]o/[t]/[a] menu. This matches the
            // single-key UX users expect: hit Shift+Tab, prompt
            // disappears, mode flips to Bypass for the rest of
            // the session.
            //
            // We print a one-line notice so the user sees the
            // mode change instead of just observing tool calls
            // suddenly stop prompting.
            if ask_editor_mode_cycle.swap(false, Ordering::Relaxed) {
                let mut out = io::stdout();
                let line = if ask_use_color {
                    "  \x1b[2m→ Shift+Tab: allowed this call and switched session to YOLO (Bypass)\x1b[0m"
                } else {
                    "  → Shift+Tab: allowed this call and switched session to YOLO (Bypass)"
                };
                let _ = writeln!(out, "{line}");
                let _ = out.flush();
                return AskOutcome::AllowAndSetMode(PermissionMode::Bypass);
            }
            // If the raw-mode editor is live, take exclusive stdin
            // for the prompt. We:
            //   1. Raise `editor_suspend` so the pump observes the
            //      handshake on its next loop tick (≤ ~50ms).
            //   2. Wait up to 250ms for the pump to ack via
            //      `editor_suspended`. If the pump is slow / not
            //      running we fall through after the timeout — the
            //      prompt still works in that case, the only
            //      risk is a transient race where the first
            //      keystroke goes to the editor instead.
            //   3. Run the prompt normally.
            //   4. Lower `editor_suspend`; the pump will re-enter
            //      raw mode and repaint within ~20ms.
            let suspending = ask_raw_editor_active.load(Ordering::Relaxed);
            if suspending {
                ask_editor_suspend.store(true, Ordering::Relaxed);
                let deadline = std::time::Instant::now() + std::time::Duration::from_millis(250);
                while !ask_editor_suspended.load(Ordering::Relaxed)
                    && std::time::Instant::now() < deadline
                {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }

            let outcome = handle_permission_prompt(
                tool_call,
                &ask_spinner_lock,
                &ask_spinner_stop,
                &ask_output_started,
                ask_use_color,
            );

            if suspending {
                ask_editor_suspend.store(false, Ordering::Relaxed);
            }

            outcome
        },
    );

    // `current_tool_phase` / `current_tool_phase_for_callback` are
    // declared further up alongside the streaming handler's
    // captures, because the handler also writes to the phase on
    // `ToolUseStart` (model committed to a tool, args streaming).
    // The `Finished` arm below clears it for the symmetric exit.
    let tool_progress_callback: deeptide_core::ToolProgressCallback = Arc::new(
        move |event: &deeptide_core::ToolProgressEvent| match event {
            deeptide_core::ToolProgressEvent::Started {
                preview,
                name,
                index,
                ..
            } => {
                // Prefer the rich preview; fall back to the bare
                // tool name when the input was opaque (custom MCP
                // tool with no string fields, etc.). Never leave
                // the phase as an empty string — that's the
                // "collapse to bare verb" signal the spinner uses
                // for clearing state.
                let label = if preview.trim().is_empty() {
                    name.clone()
                } else {
                    preview.clone()
                };
                // (1) Drive the *main* spinner's activity label. It's only
                //     alive when no preamble text streamed this turn; when
                //     it is, this flips "Crunching…" → "Running <tool>".
                if let Ok(mut phase) = current_tool_phase_for_callback.lock() {
                    *phase = Some(format!("Running {label}"));
                }
                // (2) Drop a persistent "▶ <tool>" line into the scroll
                //     region the moment the tool starts. Without this the
                //     user sees no tool calls at all until `submit()`
                //     returns at end-of-turn and the whole batch of result
                //     lines prints at once — which on a long turn reads as
                //     "stuck, nothing happening". Held under `spinner_lock`
                //     so it can't interleave with a spinner frame.
                if let Ok(_guard) = spinner_lock_for_callback.lock() {
                    let mut out = io::stdout();
                    let _ = write!(
                        out,
                        "\r\x1b[2K{}\n",
                        render_tool_start_line(&label, use_color_for_callback)
                    );
                    let _ = out.flush();
                }
                // (3) If the main spinner already died (text streamed before
                //     the tool batch), nothing animates during the tool's
                //     *execution* — a shell / Chrome PDF run can sit silent
                //     for minutes. Spawn a heartbeat ticker so the UI keeps
                //     a live "⠹ Running <tool> · Ns" pulse. When the main
                //     spinner is still alive it already provides the pulse,
                //     so we don't stack a second animator on the same line.
                if did_stream_for_callback.load(Ordering::Relaxed) {
                    let stop = Arc::new(AtomicBool::new(false));
                    let stop_for_thread = Arc::clone(&stop);
                    let lock_for_thread = Arc::clone(&spinner_lock_for_callback);
                    let chars_for_thread = Arc::clone(&stream_chars_for_callback);
                    let baseline = stream_chars_for_callback.load(Ordering::Relaxed);
                    let phase_for_thread = format!("Running {label}");
                    let color = use_color_for_callback;
                    let done_for_thread = Arc::clone(&tools_done_for_callback);
                    let handle = thread::spawn(move || {
                        run_tool_args_ticker(
                            &stop_for_thread,
                            &lock_for_thread,
                            &chars_for_thread,
                            baseline,
                            &phase_for_thread,
                            Instant::now(),
                            color,
                            &done_for_thread,
                        );
                    });
                    if let Ok(mut slot) = live_tool_args_for_callback.lock() {
                        // Defensive: tear down any leftover ticker before
                        // replacing so a thread can never leak (the stream's
                        // BlockStop should already have cleared the slot).
                        if let Some(old) = slot.take() {
                            old.stop.store(true, Ordering::Relaxed);
                            if let Some(h) = old.handle {
                                let _ = h.join();
                            }
                        }
                        *slot = Some(LiveToolArgsTicker {
                            stop,
                            handle: Some(handle),
                            block_index: *index,
                        });
                    }
                }
            }
            deeptide_core::ToolProgressEvent::Finished { .. } => {
                // Stop the execution heartbeat (its exit wipes its own
                // animated line) before updating the progress label.
                drain_live_tool_args(&live_tool_args_for_callback);
                // Count the completed tool and keep a running "N done" label in
                // the phase, so between tools the spinner shows progress instead
                // of reverting to a bare "Working".
                let n = tools_done_for_callback.fetch_add(1, Ordering::Relaxed) + 1;
                if let Ok(mut phase) = current_tool_phase_for_callback.lock() {
                    *phase = Some(format!("{n} tool{} done", if n == 1 { "" } else { "s" }));
                }
            }
        },
    );

    let mut repl = ReplSession::new(configured.backend)
        .with_interrupt_flag(Arc::clone(&turn_interrupt))
        .with_model(configured.model)
        .with_version(format!("deeptide-rs {VERSION_LONG}"))
        .with_permission_mode(permission_mode)
        .with_max_turns(cli.max_turns)
        .with_pricing_overrides(pricing_overrides)
        .with_debug(cli.debug)
        .with_fast_mode(cli.fast)
        .with_hooks(hooks)
        .with_tool_restrictions(allowed_tools, disallowed_tools)
        .with_session_persistence(!cli.no_session_persistence)
        .with_session_end_capture(!cli.no_session_capture)
        .with_suggestions(!cli.no_suggestions)
        .with_additional_dirs(&cli.add_dir)
        .with_tps_store_dir(deeptide_core::tps::default_store_dir())
        .with_subagent_backend_factory(subagent_backend_factory(configured.subagent_config))
        .with_ask_callback(ask_callback)
        .with_tool_progress_callback(tool_progress_callback);
    if let Some(append) = cli.append_system_prompt.as_deref() {
        repl = repl.with_appended_system_prompt(append);
    }

    // Hand the streaming handler a stable reference to the per-session
    // queue. Must happen AFTER `ReplSession::new` so the handle exists;
    // setting fails iff some other code path already filled the slot
    // (it shouldn't) — we ignore the result either way to stay
    // forward-compatible if future code wants to pre-seed the queue.
    let _ = queue_slot.set(repl.message_queue_handle());

    // Completion / hint provider for the custom raw-mode prompt editor.
    // Holds the REPL's command + model tables; the read loop borrows it
    // each turn to drive Tab completion and the inline ghost-text hint.
    // (Shift+Tab mode-cycling is now handled inside the editor, which
    // surfaces `PromptOutcome::ModeCycle` instead of rustyline's
    // interrupt-binding hack.)
    let editor_helper = ReplHelper::new(repl.command_sources());

    // Persistent, multi-line-safe command history — replaces rustyline's
    // `load_history` / `add_history_entry` / `save_history`.
    let history_path = history_file_path();
    let mut history = match history_path.as_ref() {
        Some(path) => prompt_editor::History::load(path),
        None => prompt_editor::History::new(),
    };

    // Rich welcome banner: three styled lines that summarise model,
    // mode, cwd, auth state, plus the key shortcuts. Replaces the
    // older two-line plain banner without changing the
    // `repl.banner()` API — the legacy version is still available
    // and matters for non-interactive code paths (tests, JSON
    // output) that consume it directly.
    let welcome = {
        let banner_cwd = std::env::current_dir()
            .map(|p| {
                p.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| p.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|_| String::from("."));
        let auth_short = if is_configured {
            "key ok"
        } else {
            "no key (echo backend)"
        };
        chrome::render_welcome(
            VERSION_SHORT,
            repl.agent_loop().model(),
            permission_mode.label(),
            &banner_cwd,
            auth_short,
            use_color,
        )
    };
    writeln!(stdout, "{welcome}").map_err(|error| error.to_string())?;
    if !is_configured {
        writeln!(
            stdout,
            "No API key configured; using local echo backend. Set DEEPTIDE_API_KEY (or ANTHROPIC_API_KEY / ZERO_API_KEY) or pass --api-key to call a model. Run `deeptide --doctor` to inspect the full resolution chain."
        )
        .map_err(|error| error.to_string())?;
    }

    // First-run onboarding: on a fresh install with prior Claude Code / Codex
    // sessions in this project, nudge the user to import them (especially
    // `/import all`). Mark onboarded UNCONDITIONALLY on the first run so the
    // discovery walk (which scans ~/.claude and the whole ~/.codex tree) runs
    // at most once, not on every startup until a session-bearing project is
    // opened.
    if deeptide_core::is_first_run() {
        if let Some(hint) = repl.first_run_import_hint() {
            writeln!(stdout, "{}", status_bar::dim(&hint, use_color))
                .map_err(|error| error.to_string())?;
        }
        if !deeptide_core::mark_onboarded() {
            writeln!(
                stdout,
                "{}",
                status_bar::dim(
                    "(could not persist the onboarding marker; set HOME or XDG_CONFIG_HOME to a \
                     writable dir to stop this hint from reappearing)",
                    use_color
                )
            )
            .map_err(|error| error.to_string())?;
        }
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

    // --import <tool>: bring in a prior session from another agent (Claude Code,
    // Codex) before the first prompt. Renders the same events the `/import`
    // slash command would.
    if let Some(tool) = cli.import.as_deref() {
        let selector = cli.import_session.as_deref().unwrap_or("--latest");
        let import_args = format!("{tool} {selector} --as {}", cli.import_as);
        for event in repl.run_import(&import_args) {
            if let deeptide_core::ReplEvent::Output(text) = event {
                writeln!(stdout, "{text}").map_err(|error| error.to_string())?;
            }
        }
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

    // Pre-rendered styled prompt for the custom editor. Computed once:
    // the raw `repl.prompt()` value is stable for the session. The editor
    // owns the cursor and measures width via `deeptide_core::width` (which
    // strips ANSI), so we use a plain SGR wrapper here rather than the
    // rustyline `\x01...\x02` invisible-marker form.
    let raw_prompt = repl.prompt();
    let editor_prompt = if let Some(stripped) = raw_prompt.strip_suffix(' ') {
        format!(
            "{} ",
            status_bar::colorize(stripped, status_bar::palette::PROMPT, use_color)
        )
    } else {
        status_bar::colorize(&raw_prompt, status_bar::palette::PROMPT, use_color)
    };

    // Auth indicator state. `is_configured` (computed from
    // `effective_credential`) is the inference-side API key result and is
    // stable for the lifetime of the session. The Paean publish token is
    // probed each repaint because it's environmental and could in principle
    // change mid-session; the call is just env-var reads and is cheap.
    let api_key_resolved = is_configured;
    let mut auth_paint_tick: u64 = 0;

    // When a turn finishes with messages remaining in the queue, we
    // populate this with the drained content so the next loop iteration
    // submits it immediately, bypassing rustyline. Cleared as soon as it
    // is consumed. Holds at most one entry — `drain_next_queued_prompt`
    // already decides how to combine multiple queued messages based on
    // `QueueMode`.
    let mut pending_queued_input: Option<String> = None;

    // Tracks the last time the user pressed Ctrl-C *at the idle prompt* (empty
    // draft). The first press warns; a second within `CTRL_C_EXIT_WINDOW`
    // exits — the Codex / Claude Code "press Ctrl-C again to exit" guard
    // against quitting on a stray keystroke. (Cancelling a *running* turn is a
    // single press, handled by the raw-mode editor pump, which doesn't reach
    // this idle path.)
    let mut last_idle_ctrl_c: Option<Instant> = None;
    const CTRL_C_EXIT_WINDOW: Duration = Duration::from_millis(1500);

    // Completion + inline-hint providers for the custom editor, borrowing
    // `editor_helper` for the lifetime of the loop. Built once so each
    // turn's `read_prompt` can hand them straight to the editor.
    let comp = |line: &str, pos: usize| editor_completion(&editor_helper, line, pos);
    let hintf = |line: &str, pos: usize| editor_hint(&editor_helper, line, pos);

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
        // `render_styled` emits per-segment SGR so `mode bypass`
        // / `ctx 96%` / missing-auth pop against the otherwise
        // dim bar. With color off it returns the same plain
        // string as `render`, so the previous `status_bar::dim`
        // wrap-everything path is no longer needed.
        let bar_styled = repl
            .status_line_with_auth(Some(auth_segment))
            .render_styled(bar_width, use_color);
        if let Some(bar) = anchored.as_mut() {
            bar.repaint(&bar_styled, &spinner_lock);
            // Pin the input prompt to the row directly above the
            // status bar. This is what makes the input box appear
            // "fixed" at the bottom of the terminal even while
            // streaming output flows above: rustyline's prompt write
            // lands at row `rows - 1`, not at the post-stream cursor
            // position (which would scroll with conversation).
            if bar.footer_rows() >= 2 {
                bar.prepare_input_row(&spinner_lock);
            }
        } else {
            writeln!(stdout, "{bar_styled}").map_err(|error| error.to_string())?;
            stdout.flush().map_err(|error| error.to_string())?;
        }

        // If the previous turn drained a queued prompt, fire it immediately
        // without going through rustyline. This is what makes the queue
        // feature feel automatic: the user typed during turn N, the queue
        // auto-fires turn N+1 with that content. Skip the status-bar repaint
        // loop body's `rl.readline` and synthesize an `Ok(line)` instead.
        let outcome = match pending_queued_input.take() {
            Some(queued) => {
                writeln!(
                    stdout,
                    "{}",
                    render_queue_dispatch_notice(&queued, use_color)
                )
                .map_err(|error| error.to_string())?;
                prompt_editor::PromptOutcome::Line(queued)
            }
            None => {
                let cfg = prompt_editor::PromptConfig {
                    prompt: &editor_prompt,
                    use_color,
                    history: history.entries(),
                    completion: &comp,
                    hint: &hintf,
                };
                // The custom raw-mode editor owns the cursor + dynamic
                // footer when the anchored status bar is engaged and color
                // is on; otherwise fall back to a cooked single-line read.
                match anchored.as_mut() {
                    Some(bar) if use_color && bar.footer_rows() >= 2 => {
                        prompt_editor::read_prompt(bar, &spinner_lock, &cfg)
                    }
                    _ => prompt_editor::read_prompt_cooked(&cfg),
                }
            }
        };
        match outcome {
            prompt_editor::PromptOutcome::Line(line) => {
                // Any successful input cancels a pending Ctrl-C double-tap so a
                // stale first tap from minutes ago can't pair with a fresh one
                // and accidentally quit (matches #132's idle-exit guard).
                last_idle_ctrl_c = None;
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

                history.add(&content);

                // After rustyline returned, the cursor is wherever
                // the user's Enter left it — possibly below the input
                // row if their text wrapped, possibly on the status
                // row. Snap back into the scroll region so the
                // upcoming streamed output lands above the pinned
                // footer (not on top of the status bar). The status
                // bar is also repainted in case input overflow
                // scribbled across row `rows`.
                if let Some(bar) = anchored.as_mut()
                    && bar.footer_rows() >= 2
                {
                    bar.recover_to_scroll_region(&bar_styled, &spinner_lock);
                }

                // Interactive selection menu: if this line opens one (e.g. a
                // bare `/import`), run the fuzzy picker and replace `content`
                // with the chosen row's action. Cancel skips the turn; an
                // unsupported terminal falls through to the numbered-text menu
                // that `submit` prints.
                let content = if use_color && let Some(menu) = repl.menu_for(&content) {
                    match picker::run(&menu, use_color) {
                        picker::PickResult::Selected(action) => action,
                        picker::PickResult::Cancelled => continue,
                        picker::PickResult::Unsupported => content,
                    }
                } else {
                    content
                };

                // Echo the user's submitted text into the scrollback as a
                // styled `▎ you ▾` block. Without this the conversation
                // looks one-sided once the input row is wiped — the
                // model's reply appears with no context for what was
                // asked. Skipped for slash commands because the
                // command's own output is the user-visible echo (and a
                // double-prefix would just be noise).
                if !content.starts_with('/') {
                    let echo = chrome::render_user_block(&content, use_color);
                    let _ = stdout.write_all(echo.as_bytes());
                }

                // Classify the input: local-only slash commands (/exit,
                // /clear, /new, /help, /status, …) get a strictly
                // synchronous path with no spinner, no queue editor,
                // no ghost paint. The previous flow always span up
                // the full agent-turn apparatus before discovering
                // submit had nothing to do, which the user perceived
                // as `/exit` "first talking to the model".
                //
                // `slash_command_invokes_agent` is the source of
                // truth for which slash commands actually trigger
                // an `agent_loop.run` (and therefore deserve the
                // spinner/editor). Non-slash input is always
                // agent-invoking.
                let invokes_agent = !content.starts_with('/')
                    || deeptide_core::slash_command_invokes_agent(&content);

                // `/exit` / `/quit` / `/q` (and Ctrl-D) now quit immediately
                // — they no longer run the model-backed memory consolidation
                // pass that used to make exiting hang for minutes. So there's
                // no "consolidating memory" spinner to arm here.

                // Paint a dim ghost prompt at the input row so the
                // bottom of the screen never reads "empty" while the
                // agent is thinking. The next loop iteration's
                // `prepare_input_row` will clear it before rustyline
                // takes over. We *also* paint this as a fallback for
                // sessions where the raw-mode queue editor refuses
                // to start (non-TTY stdin, termios failure, etc.);
                // the editor thread's first paint will overwrite it
                // a few milliseconds later when the editor is up.
                //
                // Suppressed for local-only slash commands: there's
                // no waiting period to fill, the result text comes
                // back synchronously in microseconds.
                if invokes_agent
                    && let Some(bar) = anchored.as_ref()
                    && bar.footer_rows() >= 2
                {
                    bar.paint_input_ghost(&chrome::render_thinking("", use_color), &spinner_lock);
                }

                // Reset per-turn streaming + spinner state.
                did_stream.store(false, Ordering::Relaxed);
                did_think.store(false, Ordering::Relaxed);
                output_started.store(false, Ordering::Relaxed);
                spinner_stop.store(false, Ordering::Relaxed);
                // Clear any cancellation left from a previous turn so a stale
                // Ctrl-C never aborts this fresh one before it starts. The
                // raw-editor pump (re)sets it on an empty-draft Ctrl-C.
                turn_interrupt.store(false, Ordering::Relaxed);

                // Spin up the raw-mode queue editor thread so the
                // user can type follow-ups straight into the pinned
                // input row while the agent is streaming. Skips
                // entirely when:
                //   * the request doesn't invoke the agent (local-only
                //     slash command → no streaming, no need to queue),
                //   * we couldn't reserve a footer row (no
                //     anchored bar or only the legacy 1-row variant),
                //   * stdin isn't a TTY / termios refuses to flip,
                //   * color is off (likely a piped session where
                //     cooked-mode stdin is fine).
                let editor_stop = Arc::new(AtomicBool::new(false));
                let editor_handle = if invokes_agent
                    && use_color
                    && anchored.as_ref().map(|b| b.footer_rows()).unwrap_or(0) >= 2
                {
                    match queue_editor::enter_raw_mode() {
                        Some(guard) => {
                            let input_row = anchored.as_ref().map(|b| b.input_row()).unwrap_or(0);
                            // Per-turn fresh suspend/ack pair so a
                            // suspend from a previous turn that wasn't
                            // cleanly cleared can't bleed into this
                            // one. `editor_suspend` / `editor_suspended`
                            // outside this block are cloned into the
                            // ask_callback's closure once at startup,
                            // so we reset them rather than swap.
                            editor_suspend.store(false, Ordering::Relaxed);
                            editor_suspended.store(false, Ordering::Relaxed);
                            // editor_mode_cycle survives across turns
                            // intentionally — if the user presses
                            // Shift+Tab and we don't manage to apply
                            // it this turn (e.g. the editor failed
                            // to spawn), we still pick it up at the
                            // turn-boundary check below.
                            let ctx = queue_editor::EditorContext {
                                queue: repl.message_queue_handle(),
                                stop: Arc::clone(&editor_stop),
                                paint_lock: Arc::clone(&spinner_lock),
                                repaint: Box::new(move |line: &str| {
                                    status_bar::write_ghost_at_row(input_row, line);
                                }),
                                use_color,
                                line_width: terminal_width().unwrap_or(80),
                                suspend: Arc::clone(&editor_suspend),
                                suspended: Arc::clone(&editor_suspended),
                                mode_cycle: Arc::clone(&editor_mode_cycle),
                                interrupt: Arc::clone(&turn_interrupt),
                            };
                            // Flip the suppression flag BEFORE spawning
                            // so the streaming handler's first tick sees
                            // it; flip it back when the editor thread
                            // joins so the cooked path resumes for the
                            // next turn (if raw mode happens to fail
                            // then).
                            raw_editor_active.store(true, Ordering::Relaxed);
                            Some(thread::spawn(move || queue_editor::run_pump(ctx, guard)))
                        }
                        None => None,
                    }
                } else {
                    None
                };

                // Animate an activity spinner on a background thread while the
                // synchronous turn runs, so model thinking and tool execution
                // aren't silent. Skipped when:
                //   * color is off (plain/log output), or
                //   * the request is a local-only slash command (no
                //     model round-trip, the spinner flash would be
                //     pure noise — this is the fix for the "`/exit`
                //     looks like it's talking to the model" bug).
                //
                // Reset the per-turn streamed-char counter just before
                // we spawn so the spinner's `↓ N tokens` stat shows
                // *this turn's* throughput. The streaming handler will
                // start adding to it as soon as the first
                // `TextDelta` / `ToolUseInputDelta` arrives.
                stream_chars_received.store(0, Ordering::Relaxed);
                prompt_tokens_total.store(0, Ordering::Relaxed);
                tools_done.store(0, Ordering::Relaxed);
                let spinner_handle = if invokes_agent && use_color {
                    let stop = Arc::clone(&spinner_stop);
                    let started = Arc::clone(&output_started);
                    let lock = Arc::clone(&spinner_lock);
                    let phase = Arc::clone(&current_tool_phase);
                    let chars = Arc::clone(&stream_chars_received);
                    let prompt = Arc::clone(&prompt_tokens_total);
                    Some(thread::spawn(move || {
                        run_spinner(&stop, &started, &lock, true, &phase, &chars, &prompt)
                    }))
                } else {
                    None
                };

                // Stamp the turn start so we can report how long the
                // agent worked once it finishes. Monotonic `Instant` so
                // it's immune to wall-clock adjustments mid-turn.
                let turn_started = Instant::now();
                let events = repl.submit(&content);

                // Defensive: tear down any live tool-args ticker
                // still in flight. The streaming handler joins its
                // own ticker on `BlockStop`, but a truncated stream
                // or network blip can leave one orphaned. Without
                // this drain it would keep printing into the next
                // turn's render area.
                drain_live_tool_args(&live_tool_args);

                // Halt the spinner and reclaim its line before printing results.
                spinner_stop.store(true, Ordering::Relaxed);
                if let Some(handle) = spinner_handle {
                    let _ = handle.join();
                }

                // Stop the queue editor thread; the pump owns its
                // `RawModeGuard` now and drops it on the way out of
                // `run_pump`, restoring termios to cooked mode
                // before the join returns. We don't need a separate
                // restore here.
                //
                // Also lower the suspend flag in case the pump
                // exited while parked in a permission-prompt
                // suspend — that would leave `editor_suspend=true`
                // for the next turn, mis-triggering the ack
                // handshake before the pump has even started.
                editor_stop.store(true, Ordering::Relaxed);
                if let Some(handle) = editor_handle {
                    let _ = handle.join();
                }
                editor_suspend.store(false, Ordering::Relaxed);
                editor_suspended.store(false, Ordering::Relaxed);
                raw_editor_active.store(false, Ordering::Relaxed);

                // Apply any Shift+Tab the user pressed during the
                // turn that wasn't already eaten by a permission
                // prompt. Mirrors the between-turn rustyline arm so
                // mid-turn Shift+Tab cycles modes identically to
                // between-turn Shift+Tab: Default → AcceptEdits →
                // Plan → Bypass → Default.
                if editor_mode_cycle.swap(false, Ordering::Relaxed) {
                    let next = next_permission_mode(repl.agent_loop().permission_mode());
                    repl.set_permission_mode(next);
                    let label = next.label();
                    let line = if use_color {
                        format!("  → mode \x1b[1m{label}\x1b[0m")
                    } else {
                        format!("  → mode {label}")
                    };
                    writeln!(stdout, "{line}").map_err(|error| error.to_string())?;
                }

                // Drain the streaming markdown buffer. The model often
                // produces a trailing line without a final newline; without
                // this flush that fragment would stay buffered and reappear
                // on the next turn's first delta with stale state. Resetting
                // the renderer also clears any fence state so a turn that
                // ended mid-code-block doesn't bleed into the next response.
                let streamed = did_stream.load(Ordering::Relaxed);
                if let Ok(mut renderer) = streaming_md.lock() {
                    let trailing = renderer.flush();
                    if !trailing.is_empty() {
                        let _ = stdout.write_all(trailing.as_bytes());
                    }
                    *renderer =
                        StreamingMarkdownRenderer::new(MarkdownRenderOptions { color: use_color });
                }
                // Soft separator between turns. We only emit it when
                // the agent actually streamed something (otherwise
                // the turn was likely a slash-command echo whose
                // output is self-contained) and when we have a
                // terminal width we can clamp the rule against.
                if streamed {
                    let width = terminal_width().unwrap_or(80);
                    let _ = stdout.write_all(chrome::render_separator(width, use_color).as_bytes());
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
                        ReplEvent::System(message) => {
                            let rendered = render_system_message(&message, use_color, cli.debug);
                            writeln!(stdout, "{rendered}").map_err(|error| error.to_string())?;
                        }
                        ReplEvent::Exit => {
                            if let Some(ref path) = history_path {
                                history.save(path);
                            }
                            return Ok(());
                        }
                    }
                }

                // Per-turn timing footer: how long the agent worked plus
                // the local wall-clock time it finished, so long-running
                // tasks are easy to review after the fact. Gated on
                // `invokes_agent` so local-only slash commands (/help,
                // /status, …) — whose work is instantaneous — don't get a
                // noisy "worked for 0s" line.
                if invokes_agent {
                    let footer =
                        chrome::render_turn_timing(turn_started.elapsed(), local_now(), use_color);
                    writeln!(stdout, "{footer}").map_err(|error| error.to_string())?;
                }

                // The turn has finished. If the user typed any messages
                // during it (captured via mid-turn stdin polling) OR
                // explicitly enqueued via `/queue add`, drain the queue
                // and stage the result for automatic submission on the
                // next iteration. `drain_next_queued_prompt` honours the
                // configured mode (single = pop head, batch = join all).
                pending_queued_input = repl.drain_next_queued_prompt();
            }
            prompt_editor::PromptOutcome::ModeCycle => {
                // Shift+Tab inside the editor rotates the session
                // permission mode: Default → AcceptEdits → Plan → Bypass →
                // Default. (Previously piggy-backed on rustyline's
                // interrupt binding; the custom editor surfaces it as a
                // dedicated outcome instead.)
                // A deliberate mode switch also clears the Ctrl-C double-tap
                // timer so it can't pair with a later tap to exit.
                last_idle_ctrl_c = None;
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
            prompt_editor::PromptOutcome::Interrupted => {
                // Ctrl+C on an empty draft at the idle prompt. Double-press to
                // exit: the first press warns, a second within the window
                // quits. (A non-empty draft is cleared inside the editor and
                // never reaches here; cancelling a running turn is a single
                // press handled by the raw-mode pump.)
                let now = Instant::now();
                let double = last_idle_ctrl_c
                    .is_some_and(|prev| now.duration_since(prev) <= CTRL_C_EXIT_WINDOW);
                if double {
                    writeln!(stdout, "^C").map_err(|error| error.to_string())?;
                    break;
                }
                last_idle_ctrl_c = Some(now);
                writeln!(stdout, "^C  (press Ctrl-C again to exit)")
                    .map_err(|error| error.to_string())?;
                continue;
            }
            prompt_editor::PromptOutcome::Eof => {
                // Ctrl+D on an empty line — exit immediately, exactly like a
                // typed `/exit`. We deliberately do NOT run the
                // end-of-session memory consolidation pass here: it drove a
                // model round-trip (a full agent loop, no timeout) that made
                // quitting hang for many seconds — sometimes appearing never
                // to return. Durable facts are still captured by the
                // scheduled dream loop *during* the session and can be forced
                // any time with `/dream`.
                writeln!(stdout).map_err(|error| error.to_string())?;
                break;
            }
        }
    }

    if let Some(ref path) = history_path {
        history.save(path);
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
/// Live indicator shown while the model is streaming a tool_use's
/// JSON arguments **after** the main spinner has already been killed
/// by an earlier `TextDelta`.
///
/// Without it the user sees the spinner disappear after the model's
/// preamble text and then long silence while a multi-kilobyte tool
/// payload streams in — reads as "stuck", which is exactly the
/// symptom the user reported. With it, a single line below the
/// streamed text repaints every 120 ms with the verb, tool name,
/// elapsed seconds, and estimated tokens received.
///
/// ## Lifecycle
///
/// Spawned by the streaming handler on `ToolUseStart` (only when
/// `did_stream == true`, i.e. text already ran and the main spinner
/// is dead). Stopped + joined + line-cleared on `BlockStop` for the
/// matching block index. Also cleaned up defensively at turn end so
/// an aborted ticker (truncated stream, network blip) can't leak.
///
/// `baseline_chars` snapshots `stream_chars_received` at start so the
/// "↓ N tokens" panel reports tokens **for this tool's args**, not
/// cumulative-since-turn-start (which would mix in the preceding
/// text's char count and read as wrong).
struct LiveToolArgsTicker {
    /// Set to `true` to end the ticker's animation loop.
    stop: Arc<AtomicBool>,
    /// `Some` until the owning handler joins the thread; taken
    /// out on stop.
    handle: Option<std::thread::JoinHandle<()>>,
    /// Anthropic content-block index this ticker is tracking. The
    /// matching `BlockStop` carries the same index.
    block_index: usize,
}

/// Defensive teardown for a [`LiveToolArgsTicker`] left behind by an
/// abnormal stream termination (truncated SSE, network blip, model
/// emitting `ToolUseStart` without a matching `BlockStop`). Called
/// at turn boundaries so the orphan can't keep painting into the
/// next turn.
///
/// No-op when the slot is already empty, so calling it
/// unconditionally is cheap.
fn drain_live_tool_args(slot: &Mutex<Option<LiveToolArgsTicker>>) {
    let to_join = slot.lock().ok().and_then(|mut s| s.take());
    if let Some(mut ticker) = to_join {
        ticker.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = ticker.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Animation loop for [`LiveToolArgsTicker`]. Mirrors `run_spinner`
/// but draws under a separate stop flag and reports tokens
/// (subtracting `baseline_chars` from the shared turn counter, so the
/// `↓ N tokens` panel is scoped to whatever started after the
/// baseline snapshot). `phase` is the full activity label to render —
/// `"Preparing Bash(…)"` while the model streams tool args, or
/// `"Running Bash(…)"` while the tool itself executes.
#[allow(clippy::too_many_arguments)] // internal progress-ticker helper; args are all distinct primitives
fn run_tool_args_ticker(
    stop: &AtomicBool,
    lock: &Mutex<()>,
    stream_chars: &AtomicUsize,
    baseline_chars: usize,
    phase: &str,
    started_at: Instant,
    color: bool,
    tools_done: &AtomicUsize,
) {
    // Match `run_spinner`'s 150 ms grace so a quick BlockStop / fast
    // tool never produces a flash. The model often emits the full
    // tool_use in one SSE frame for short inputs (e.g. TodoWrite), and
    // reads/greps finish in milliseconds.
    thread::sleep(Duration::from_millis(150));
    if stop.load(Ordering::Relaxed) {
        return;
    }
    let mut tick = 0usize;
    while !stop.load(Ordering::Relaxed) {
        if let Ok(_guard) = lock.lock() {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let total_chars = stream_chars.load(Ordering::Relaxed);
            // Saturate so a stale snapshot (counter reset mid-turn)
            // never produces a negative token count.
            let scoped_chars = total_chars.saturating_sub(baseline_chars);
            let tokens_estimate = scoped_chars / 4;
            // Append a running "· N done" so a multi-tool turn shows cumulative
            // progress, not just the current tool. `phase` is the pre-built
            // activity label ("Preparing Bash(…)" / "Running Bash(…)"); we only
            // tack the cumulative counter onto it here.
            let done = tools_done.load(Ordering::Relaxed);
            let phase_label = if done > 0 {
                format!("{phase} · {done} done")
            } else {
                phase.to_owned()
            };
            let raw = tui::render_spinner_line_rich(
                tick,
                started_at.elapsed().as_secs(),
                Some(&phase_label),
                // The tool-args panel is scoped to this tool's downloaded args;
                // the uploaded-context (↑) figure belongs to the main spinner.
                0,
                tokens_estimate,
            );
            let line = status_bar::dim(&raw, color);
            let mut out = io::stdout();
            // Same redraw pattern as `run_spinner`: carriage return
            // to column 0, paint, then EL to wipe stale glyphs from
            // the previous (longer) frame.
            let _ = write!(out, "\r{line}\x1b[K");
            let _ = out.flush();
        }
        tick = tick.wrapping_add(1);
        thread::sleep(Duration::from_millis(120));
    }
    // Wipe our line on exit so whatever renders next (tool header,
    // assistant text continuation) lands on a clean row.
    if let Ok(_guard) = lock.lock() {
        let mut out = io::stdout();
        let _ = write!(out, "\r\x1b[2K");
        let _ = out.flush();
    }
}

fn run_spinner(
    stop: &AtomicBool,
    output_started: &AtomicBool,
    lock: &Mutex<()>,
    color: bool,
    phase: &Mutex<Option<String>>,
    stream_chars: &AtomicUsize,
    prompt_tokens: &AtomicUsize,
) {
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
            // Take a snapshot of the phase under its own short-lived
            // lock so the painter doesn't hold both `lock` and
            // `phase` at the same time — keeps the locking order
            // shallow.
            let phase_snapshot = phase.lock().ok().and_then(|p| p.clone());
            // Estimated tokens received this turn. Chars-to-tokens
            // is approximated as `chars / 4` (the conventional
            // English heuristic); the value is intentionally an
            // estimate for progress feedback, not billing.
            let chars = stream_chars.load(Ordering::Relaxed);
            let tokens_estimate = chars / 4;
            // Uploaded-context size (↑), reported by the model at stream start
            // (Anthropic `message_start`). Already a token count, not chars, so
            // it is shown verbatim rather than divided by the chars heuristic.
            let prompt = prompt_tokens.load(Ordering::Relaxed);
            let raw = tui::render_spinner_line_rich(
                tick,
                start.elapsed().as_secs(),
                phase_snapshot.as_deref(),
                prompt,
                tokens_estimate,
            );
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
    permission_responses: Vec<PermissionResponse>,
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
            // object so machine consumers always get a parseable result —
            // THEN propagate a non-zero exit on failure. A machine consumer
            // parses the envelope (which carries the `error` field); a shell
            // script checks `$?`. Printing the envelope first, erroring second,
            // serves both: the JSON is always on stdout, and the exit code is
            // 0 only when the run actually succeeded.
            let outcome = run_prompt(cli, prompt, permission_mode, cwd, hooks, None);
            let body = one_shot_result_json(&outcome, &cli.model);
            println!(
                "{}",
                serde_json::to_string_pretty(&body).map_err(|error| error.to_string())?
            );
            outcome.map(|_| ())?;
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
                let line = match event {
                    StreamingEvent::TextDelta { delta, .. } => Some(assistant_delta_event(delta)),
                    // Reasoning-model chain-of-thought (DeepSeek-reasoner et al.).
                    // The spec advertises `thinking_delta`; surface it live so
                    // headless consumers can show the model thinking, kept
                    // separate from the answer `assistant_delta` stream.
                    StreamingEvent::ThinkingDelta { delta } => Some(thinking_delta_event(delta)),
                    _ => None,
                };
                if let Some(line) = line {
                    let mut out = io::stdout().lock();
                    let _ = writeln!(out, "{line}");
                    let _ = out.flush();
                }
            });
            // Forward tool batches / results / compaction as stream-json lines as
            // they happen, so consumers see the agent's tool activity mid-run
            // (previously the stream was only `assistant_delta` + final envelope).
            let tool_sink = |event: &AgentLoopEvent| {
                if let Some(line) = stream_json_event_line(event) {
                    let mut out = io::stdout().lock();
                    let _ = writeln!(out, "{line}");
                    let _ = out.flush();
                }
            };
            // Install the headless permission callback when this is the host
            // protocol (`--embedded`) or the host pre-queued responses — so
            // gated tools emit a `permission_request` and resolve from those
            // responses rather than erroring. Plain `--print` stream-json with
            // no responses keeps the prior behaviour (loop rules decide).
            let ask_callback = if cli.embedded || !permission_responses.is_empty() {
                Some(headless_ask_callback(permission_responses))
            } else {
                None
            };
            let outcome = run_prompt_with_events(
                cli,
                prompt,
                permission_mode,
                cwd,
                hooks,
                Some(delta_handler),
                Some(&tool_sink),
                ask_callback,
            );
            let body = one_shot_result_json(&outcome, &cli.model);
            println!(
                "{}",
                serde_json::to_string(&body).map_err(|error| error.to_string())?
            );
            // Same exit-code contract as the Json path: the `result`/`error`
            // envelope is always emitted, but the process exits non-zero when
            // the run failed so automation can detect it via `$?`.
            outcome.map(|_| ())?;
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
    run_prompt_with_events(
        cli,
        prompt,
        permission_mode,
        cwd,
        hooks,
        streaming_handler,
        None,
        None,
    )
}

/// Like [`run_prompt`] but invokes `event_sink` (when given) for every
/// `AgentLoopEvent` as it is consumed, so the stream-json output path can
/// forward tool batches / tool results / compaction notices to stdout as they
/// happen. The sink sees events in emission order, interleaved with the live
/// `assistant_delta` stream the `streaming_handler` already emits.
#[allow(clippy::too_many_arguments)] // one-shot run wiring; each arg is a distinct, named concern
fn run_prompt_with_events(
    cli: &Cli,
    prompt: &str,
    permission_mode: PermissionMode,
    cwd: &Path,
    hooks: deeptide_core::HookEngine,
    streaming_handler: Option<StreamingHandler>,
    event_sink: Option<&dyn Fn(&AgentLoopEvent)>,
    ask_callback: Option<deeptide_core::PermissionAskCallback>,
) -> Result<PromptOutcome, String> {
    let configured = configured_backend_with_handler(cli, streaming_handler, None)?;

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

    // Headless permission handling: with a callback installed, gated tools emit
    // a `permission_request` and are resolved from host-supplied responses
    // instead of erroring out. Without one (the default), the loop's own
    // permission rules apply as before.
    if let Some(ask) = ask_callback {
        loop_ = loop_.with_ask_callback(ask);
    }

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

    // --import <tool> --import-as context: prepend a framed handoff from a
    // foreign session so the one-shot prompt continues prior work. (Memory-mode
    // import is interactive-only; it needs its own agent pass.)
    if let Some(tool) = cli.import.as_deref() {
        if cli.import_as != "memory" {
            if let Some(source) = deeptide_core::import::SourceTool::parse(tool) {
                let selector = cli.import_session.as_deref().unwrap_or("--latest");
                match deeptide_core::import::resolve_ref(cwd, source, selector)
                    .and_then(|r| deeptide_core::import::parse_file(&r.path, source))
                {
                    Ok(t) if t.message_turns() > 0 => {
                        let handoff = deeptide_core::import::handoff_message(&t, 8);
                        let mut combined = vec![handoff];
                        combined.extend(loop_.messages().to_vec());
                        loop_.restore_messages(combined);
                    }
                    Ok(_) => eprintln!("[import] session had no conversational content; skipped."),
                    Err(e) => eprintln!("[import] {e}"),
                }
            } else {
                eprintln!("[import] unknown source `{tool}`; use claude, codex, or deeptide.");
            }
        } else {
            eprintln!(
                "[import] --import-as memory is interactive-only; run deeptide without --print."
            );
        }
    }

    let events = loop_.run(prompt);
    let mut assistant = None;
    for event in events {
        // Forward each event to the sink (stream-json tool-event emitter) before
        // we consume it, so consumers see tool batches / results in real order.
        if let Some(sink) = event_sink {
            sink(&event);
        }
        match event {
            AgentLoopEvent::Assistant(message) => assistant = Some(message.content),
            AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(error)) => return Err(error),
            AgentLoopEvent::Terminal(AgentTerminalEvent::MaxTurnsReached { cap }) => {
                return Err(format!(
                    "maximum turns reached ({cap}). Raise the cap with --max-turns N or set `max_turns` in settings.json."
                ));
            }
            AgentLoopEvent::Terminal(AgentTerminalEvent::Blocked) => {
                return Err(String::from(
                    "context window full: transcript exceeds the model's limit even after compaction",
                ));
            }
            // Print mode is non-interactive — there is no Ctrl-C path that
            // would set the interrupt flag, so `Interrupted` is unreachable
            // here in practice; treat it as a benign no-op for exhaustiveness.
            AgentLoopEvent::User(_)
            | AgentLoopEvent::ToolBatchSummary { .. }
            | AgentLoopEvent::ToolResult { .. }
            | AgentLoopEvent::Compaction(_)
            | AgentLoopEvent::Terminal(AgentTerminalEvent::Complete)
            | AgentLoopEvent::Terminal(AgentTerminalEvent::Interrupted) => {}
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

/// Serialize a single `thinking_delta` stream-json event for reasoning-model
/// chain-of-thought. Distinct `type` from `assistant_delta` so consumers can
/// render the model's reasoning separately (dimmed, collapsible) from its
/// answer. The spec advertises this output type.
fn thinking_delta_event(delta: &str) -> String {
    serde_json::json!({ "type": "thinking_delta", "delta": delta }).to_string()
}

/// Serialize the `permission_request` stream-json event a headless host sees
/// when a permission-gated tool needs approval. Carries enough for the host to
/// decide: the tool name, the `tool_use_id` (so a reply can be correlated), and
/// the full input. The spec advertises this output type; it is emitted from the
/// headless ask-callback below.
fn permission_request_event(tool_call: &ToolCall) -> String {
    serde_json::json!({
        "type": "permission_request",
        "tool_use_id": tool_call.id,
        "tool": tool_call.name,
        "input": tool_call.input,
    })
    .to_string()
}

/// Map a host-supplied [`PermissionResponse`] to the agent loop's [`AskOutcome`].
/// `allowed=false` denies; `allowed && remember` whitelists the whole tool for
/// the run (so a batch of same-tool calls isn't re-asked); plain `allowed`
/// approves just this one. Pure so the mapping is unit-testable.
fn headless_permission_outcome(response: &PermissionResponse, tool_name: &str) -> AskOutcome {
    if !response.allowed {
        return AskOutcome::Deny {
            reason: String::from("denied by host (permission_response allowed=false)"),
        };
    }
    if response.remember {
        AskOutcome::AllowAllSession {
            tool_name: tool_name.to_owned(),
        }
    } else {
        AskOutcome::Allow
    }
}

/// Decide a single gated tool call from a queue of pre-supplied responses,
/// emitting the `permission_request` line via `emit` first (so the host's log
/// records what was asked, correlated by `tool_use_id`). Pops the next response
/// — matching `tool_use_id` when the host set one, else FIFO — and maps it; an
/// empty queue denies with an actionable reason. Pure over `emit`/`queue` so the
/// whole decision is testable without real stdout.
fn decide_headless_permission(
    tool_call: &ToolCall,
    queue: &mut std::collections::VecDeque<PermissionResponse>,
    emit: &mut dyn FnMut(&str),
) -> AskOutcome {
    emit(&permission_request_event(tool_call));

    // Prefer a response the host explicitly addressed to this tool_use_id;
    // otherwise take the next unaddressed one in order.
    let idx = queue
        .iter()
        .position(|r| r.tool_use_id.as_deref() == Some(tool_call.id.as_str()))
        .or_else(|| {
            queue
                .iter()
                .position(|r| r.tool_use_id.is_none() || r.tool_use_id.as_deref() == Some(""))
        });
    match idx.and_then(|i| queue.remove(i)) {
        Some(response) => headless_permission_outcome(&response, &tool_call.name),
        None => AskOutcome::Deny {
            reason: format!(
                "no permission_response supplied for {} (tool_use_id {}); pre-queue a permission_response in the stream-json input, or run with --permission-mode accept-edits/bypass",
                tool_call.name, tool_call.id
            ),
        },
    }
}

/// Build the headless permission ask-callback: emits a `permission_request` per
/// gated tool and resolves it from `responses` (the `permission_response`
/// messages parsed from the stream-json input). Shared `Mutex` because the
/// agent loop may invoke the callback from a worker thread.
fn headless_ask_callback(
    responses: Vec<PermissionResponse>,
) -> deeptide_core::PermissionAskCallback {
    let queue = Arc::new(Mutex::new(std::collections::VecDeque::from(responses)));
    Arc::new(move |tool_call: &ToolCall| {
        let mut emit = |line: &str| {
            let mut out = io::stdout().lock();
            let _ = writeln!(out, "{line}");
            let _ = out.flush();
        };
        match queue.lock() {
            Ok(mut q) => decide_headless_permission(tool_call, &mut q, &mut emit),
            Err(_) => AskOutcome::Deny {
                reason: String::from("permission queue poisoned"),
            },
        }
    })
}

/// Extract any `permission_response` messages a host pre-queued in the
/// stream-json stdin batch (empty for text input or when none were sent). These
/// pre-authorise gated tools in headless runs — see [`headless_ask_callback`].
fn collect_permission_responses(cli: &Cli, stdin: Option<&str>) -> Vec<PermissionResponse> {
    if cli.input_format != InputFormat::StreamJson {
        return Vec::new();
    }
    EmbeddedProtocol::parse(stdin.unwrap_or_default())
        .map(|parsed| parsed.permission_responses)
        .unwrap_or_default()
}

/// Encode an [`AgentLoopEvent`] as a stream-json line, or `None` for events the
/// protocol doesn't surface (the live `assistant_delta` text already streams via
/// the `streaming_handler`, so `Assistant`/`User`/terminal events are skipped
/// here to avoid duplicating it). This is what fills the previously-empty middle
/// of the stream-json output: consumers now see tool batches and results as the
/// agent works, not just the final envelope.
fn stream_json_event_line(event: &AgentLoopEvent) -> Option<String> {
    let value = match event {
        AgentLoopEvent::ToolBatchSummary {
            label,
            tool_calls,
            failed_count,
        } => serde_json::json!({
            "type": "tool_batch",
            "label": label,
            "tools": tool_calls.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
            "failed_count": failed_count,
        }),
        AgentLoopEvent::ToolResult {
            tool_call,
            content,
            is_error,
        } => serde_json::json!({
            "type": "tool_result",
            "tool": tool_call.name,
            "tool_use_id": tool_call.id,
            "is_error": is_error,
            "content": content,
        }),
        AgentLoopEvent::Compaction(report) => serde_json::json!({
            "type": "compact",
            "compressed_messages": report.compressed_messages,
            "tokens_after": report.tokens_after,
        }),
        // Assistant text is already streamed as assistant_delta; user echoes and
        // terminal markers don't need a separate line here.
        _ => return None,
    };
    Some(value.to_string())
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
    configured_backend_with_handler(cli, None, None)
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
    interrupt: Option<Arc<AtomicBool>>,
) -> Result<ConfiguredBackend, String> {
    // The CLI resolves provider / credential / overrides from clap flags + env,
    // then hands a front-end-neutral `BackendParams` to the shared host builder,
    // so the GUI assembles backends through the identical path (no drift).
    let params = BackendParams {
        provider: cli
            .provider
            .clone()
            .unwrap_or_else(|| String::from("anthropic")),
        base_url_override: base_url_override(cli),
        model_override: model_override(cli),
        credential: effective_credential(cli),
        max_output_tokens: cli.max_output_tokens,
        enable_prompt_caching: !cli.no_prompt_cache,
        stream: cli.stream,
        fallback_model: cli.fallback_model.clone(),
        thinking_label: cli.thinking.clone().or_else(|| cli.effort.clone()),
        system_prompt: resolve_system_prompt(cli)?,
    };
    build_backend(&params, streaming_handler, interrupt)
}

/// The explicit base-URL override (flag or env), or `None` to defer to the
/// preset default (the host builder applies that fallback).
fn base_url_override(cli: &Cli) -> Option<String> {
    if cli.base_url != DEFAULT_BASE_URL {
        return Some(cli.base_url.clone());
    }
    env_first_non_empty(&["ZERO_CLI_BASE_URL", "ZERO_API_BASE", "ANTHROPIC_BASE_URL"])
}

/// The explicit model override (flag or env), or `None` to defer to the preset
/// default.
fn model_override(cli: &Cli) -> Option<String> {
    if cli.model != DEFAULT_MODEL {
        return Some(cli.model.clone());
    }
    env_first_non_empty(&["ZERO_CLI_MODEL", "ANTHROPIC_MODEL"])
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

    lines.push(format!("Deeptide doctor  v{}", VERSION_SHORT));
    lines.push(String::from(
        "================================================",
    ));
    lines.push(format!("workspace : {}", cwd.display()));
    lines.push(format!(
        "platform  : {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    ));
    lines.push(format!("commit    : {}", env!("DEEPTIDE_GIT_HASH")));
    lines.push(format!("built     : {}", env!("DEEPTIDE_GIT_DATE")));
    lines.push(format!("branch    : {}", env!("DEEPTIDE_GIT_BRANCH")));
    lines.push(format!("rustc     : {}", env!("DEEPTIDE_RUSTC")));
    lines.push(String::new());

    lines.push(String::from("[ config files ]"));
    if cli.isolated {
        lines.push(String::from(
            "  isolated  : ON — global/project/local scopes ignored; using --settings only",
        ));
    }
    for (label, path) in [
        ("global ", ConfigStore::global_path()),
        ("project", ConfigStore::project_path(cwd)),
        ("local  ", ConfigStore::local_path(cwd)),
    ] {
        let exists = path.exists();
        let status = if cli.isolated {
            if exists {
                "exists, IGNORED"
            } else {
                "missing, ignored"
            }
        } else if exists {
            "exists"
        } else {
            "missing"
        };
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
/// Render the `--list-providers` table: each built-in preset's id, protocol,
/// default endpoint, default model, and whether a key is required. The custom
/// fallback (any unknown name → OpenAI-compatible localhost) is documented in a
/// trailing note rather than as a row.
fn format_provider_list() -> String {
    let ids = provider::known_provider_ids();
    let id_width = ids.iter().map(|id| id.len()).max().unwrap_or(8).max(8);
    let mut lines = vec![format!("Provider presets ({}):", ids.len())];
    lines.push(format!(
        "  {:<id_width$}  {:<9}  {:<8}  endpoint / model",
        "name", "protocol", "auth"
    ));
    for id in ids {
        let p = provider::resolve_provider(id);
        let protocol = match p.protocol {
            provider::Protocol::Anthropic => "anthropic",
            provider::Protocol::OpenAi => "openai",
            provider::Protocol::Gemini => "gemini",
        };
        let auth = if p.requires_key { "key" } else { "keyless" };
        lines.push(format!(
            "  {:<id_width$}  {protocol:<9}  {auth:<8}  {} · {}",
            p.id, p.base_url, p.model
        ));
    }
    lines.push(String::new());
    lines.push(String::from(
        "Any other --provider name → a custom OpenAI-compatible endpoint (default http://localhost:8000/v1).",
    ));
    lines.push(String::from(
        "Explicit --base-url / --model / --api-key always override a preset's defaults.",
    ));
    lines.join("\n")
}

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

/// Print a one-line "retrying…" notice when the API layer's auto-retry
/// loop fires between SSE attempts.
///
/// The notice appears as `↻ retry N/M — <reason>` in dimmed cyan so it
/// reads as a transient status message rather than competing with the
/// model's actual answer. Operates under the same spinner lock the
/// `TextDelta` arm uses so the spinner thread cannot draw a frame on
/// top of (or under) the notice.
///
/// Side effects beyond printing:
///
/// 1. Resets the in-flight `StreamingMarkdownRenderer` so any partial
///    line buffered from the doomed attempt doesn't leak into the
///    retry's output.
/// 2. Clears `did_stream` and `output_started` so the next `TextDelta`
///    re-erases the spinner line cleanly (otherwise the first
///    post-retry token would chain directly onto our notice with no
///    separator).
///
/// `use_color=false` flips ANSI escapes to plain text so the notice
/// stays readable on terminals without colour or under `--no-color`.
#[allow(clippy::too_many_arguments)]
fn render_retry_notice(
    notice: &deeptide_core::StreamRetryNotice,
    use_color: bool,
    did_stream: &Arc<AtomicBool>,
    output_started: &Arc<AtomicBool>,
    spinner_stop: &Arc<AtomicBool>,
    spinner_lock: &Arc<Mutex<()>>,
    streaming_md: &Arc<Mutex<deeptide_core::StreamingMarkdownRenderer>>,
) {
    let Ok(_guard) = spinner_lock.lock() else {
        return;
    };
    // Stop the spinner, clear its line, print the notice, then let the
    // outer loop restart the spinner during the backoff. The next
    // TextDelta (or another retry) will re-claim stdout under the lock.
    spinner_stop.store(true, Ordering::Relaxed);
    let mut out = io::stdout();
    let _ = out.write_all(b"\r\x1b[2K");

    // Drain any buffered markdown state — the partial line from the
    // doomed attempt is now meaningless because we'll get a fresh
    // delta stream on the retry.
    let drained = match streaming_md.lock() {
        Ok(mut renderer) => {
            let leftover = renderer.flush();
            renderer.reset();
            leftover
        }
        Err(_) => String::new(),
    };
    if !drained.is_empty() {
        // If the previous attempt had emitted some text already, print
        // it before the notice so the user can see how far we got.
        let _ = out.write_all(drained.as_bytes());
        if !drained.ends_with('\n') {
            let _ = out.write_all(b"\n");
        }
    }

    let line = format_retry_notice_line(notice, use_color);
    let _ = out.write_all(line.as_bytes());
    let _ = out.flush();

    // Reset the "we have streamed output" flags so the next real
    // TextDelta re-erases the spinner line and starts fresh.
    did_stream.store(false, Ordering::Relaxed);
    output_started.store(false, Ordering::Relaxed);
}

/// Truncate a single-line string to at most `max_chars` *display*
/// characters with a trailing `…` if it had to cut. Uses the unified
/// `display_width` so CJK / fullwidth content gets accounted at 2
/// cells per char rather than 1.
fn truncate_inline(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_owned();
    }
    let truncated: String = text.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{truncated}…")
}

/// Build the rendered retry-notice line (without writing it). Split out
/// from [`render_retry_notice`] so we can unit-test the formatting
/// without needing a real spinner thread or stdout capture.
///
/// Output shape:
///
/// * With reason:     `  ↻ retry N/M — <reason>\n`
/// * Without reason:  `  ↻ retry N/M\n`
/// * With colour:     wrapped in dim-cyan ANSI escapes
///
/// The leading two-space indent is intentional: it matches the
/// `└─` left-bar indent of the structured tool-event renderer so
/// transient status messages visually align with executed-tool output
/// rather than competing with the model's prose.
fn format_retry_notice_line(notice: &deeptide_core::StreamRetryNotice, use_color: bool) -> String {
    let label = if notice.reason.is_empty() {
        format!("  ↻ retry {}/{}", notice.attempt, notice.max_attempts)
    } else {
        // Bound the reason length so a verbose truncation message
        // doesn't push the spinner off-screen. 80 chars covers the
        // canonical "streaming response cut before message_stop (N
        // chars assembled…)" template comfortably.
        format!(
            "  ↻ retry {}/{} — {}",
            notice.attempt,
            notice.max_attempts,
            truncate_inline(&notice.reason, 80)
        )
    };
    if use_color {
        // Dim cyan: visible but signals "transient status, not the
        // actual answer". Reset at end so subsequent tokens aren't
        // accidentally coloured.
        format!("\x1b[2;36m{label}\x1b[0m\n")
    } else {
        format!("{label}\n")
    }
}

/// Bounded single-line preview used by both the mid-turn "✚ queued"
/// notice and the post-turn "▶ queued" dispatch notice. Collapses
/// internal newlines to ` / ` so multi-line input doesn't break the
/// status line, and truncates at `max_chars` characters (not bytes) so
/// CJK content fits cleanly.
fn queue_preview(message: &str, max_chars: usize) -> String {
    let collapsed: String = message
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" / ");
    truncate_inline(&collapsed, max_chars)
}

/// Read every pending complete line off stdin and push each into the
/// queue. Called from the streaming handler after every SSE event.
///
/// Visibility: after consuming a line we print a `✚ queued (#N): …`
/// notice on its own row so the user has positive confirmation their
/// input was captured. The notice acquires `spinner_lock` so it can't
/// tear a concurrent streaming write — the rest of the streaming
/// handler uses the same lock for that reason.
///
/// Failure modes are swallowed (we re-try on the next event tick):
///   * lock poisoning → can't render notice, skip
///   * stdin EOF → poll will no longer trigger, harmless
///   * transient read error → return; the next poll picks up the rest
fn drain_pending_stdin_into_queue(
    queue: &Arc<Mutex<deeptide_core::MessageQueue>>,
    spinner_lock: &Arc<Mutex<()>>,
    use_color: bool,
) {
    // Cap the per-event drain so a runaway producer can't starve the
    // streaming handler. The loop exits naturally when stdin reports
    // no more pending data.
    const MAX_LINES_PER_TICK: usize = 8;

    for _ in 0..MAX_LINES_PER_TICK {
        if !queue_input::stdin_has_pending_line() {
            break;
        }
        let line = match queue_input::read_pending_line() {
            Ok(Some(line)) => line,
            Ok(None) => break, // stdin EOF
            Err(_) => break,
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let depth = match queue.lock() {
            Ok(mut q) => {
                if !q.push(trimmed) {
                    continue;
                }
                q.len()
            }
            Err(_) => break,
        };

        // Render the per-line confirmation under the spinner lock so a
        // concurrent stream write doesn't interleave with our newline.
        // The leading `\r\x1b[2K` clears whatever fragment of streaming
        // output is on the current row, then we re-emit the notice on
        // its own row. The streaming text that arrives after this
        // continues from the next row naturally.
        let preview = queue_preview(trimmed, 60);
        let line_text = if use_color {
            format!("\r\x1b[2K\x1b[2;32m  ✚ queued (#{depth}): {preview}\x1b[0m\n")
        } else {
            format!("\r  ✚ queued (#{depth}): {preview}\n")
        };

        if let Ok(_guard) = spinner_lock.lock() {
            let mut out = io::stdout();
            let _ = out.write_all(line_text.as_bytes());
            let _ = out.flush();
        }
    }
}

/// Notice printed on the row above an auto-dispatched queued prompt, so
/// the user sees "this prompt was popped off the queue, not freshly
/// typed". Shown immediately before the REPL submits the queued content
/// on the next iteration.
fn render_queue_dispatch_notice(queued: &str, use_color: bool) -> String {
    let preview = queue_preview(queued, 80);
    if use_color {
        format!("\x1b[2;36m  ▶ queued → submit: {preview}\x1b[0m")
    } else {
        format!("  ▶ queued → submit: {preview}")
    }
}

/// Render a [`SystemMessage`] for the interactive REPL. Color is applied
/// when the user hasn't opted out via `--no-color` / `NO_COLOR`; in plain
/// mode the output is just the structured fields joined together.
///
/// Styling vocabulary:
///
/// - tool batch summary       → dim `· Tools …`
/// - tool success             → green `✓ <Name>` + dim summary
/// - tool failure             → red `✗ <Name>` + bright summary
/// - small successful body    → dim `│ <line>` left-bar (claude-code style)
/// - compaction notice        → dim `· Context auto-compacted …`
/// - notice (alert)           → yellow `⚠ <message>`
/// - call-id                  → only shown under `--debug` (was always-on)
fn render_system_message(message: &SystemMessage, use_color: bool, debug: bool) -> String {
    match message {
        SystemMessage::ToolBatch {
            label,
            failed_count,
        } => {
            let verb = if *failed_count == 0 {
                "Tools completed"
            } else {
                "Tools completed with failures"
            };
            let raw = format!("· {verb}: {label}");
            status_bar::dim(&raw, use_color)
        }
        SystemMessage::Tool {
            name,
            call_id,
            summary,
            is_error,
            body,
            subject,
        } => render_tool_event(
            name,
            call_id,
            summary,
            *is_error,
            body.as_deref(),
            subject.as_deref(),
            use_color,
            debug,
        ),
        SystemMessage::Compaction {
            compressed_messages,
            tokens_after,
        } => {
            let raw = format!(
                "· Context auto-compacted: folded {compressed_messages} earlier message(s); ~{tokens_after} tokens now."
            );
            status_bar::dim(&raw, use_color)
        }
        SystemMessage::Notice(text) => {
            if use_color {
                format!("\x1b[33m⚠ {text}\x1b[0m")
            } else {
                format!("! {text}")
            }
        }
    }
}

/// Format a single tool event line. Call IDs are hidden by default
/// (they're long and noisy) and only re-surfaced under `--debug` for
/// correlation with backend logs.
///
/// Layout (T2.1):
///
/// ```text
///   ✓ Read  src/main.rs  · 966 lines (29.8 KB)
///   │  └─ tool name  └─ subject (input)        └─ summary (output)
///   ✗ Bash  cargo build  · exit 1, 12 lines (1.4 KB)
/// ```
///
/// The `subject` slot pulls the most useful field of the call's
/// INPUT (file path, command, URL, …) so the row is self-describing
/// without expanding the body. Falls back gracefully to the old
/// `name · summary` shape when `subject` is `None`.
/// Render the live "▶ <tool>" line printed the instant a tool starts
/// executing — before its result is known. Mirrors `render_tool_event`'s
/// marker layout but uses a cyan ▶ so an in-flight call is visually
/// distinct from the ✓/✗ result line that lands when the turn ends. The
/// `label` is the pre-rendered input preview (e.g. `"Bash(chrome …)"`).
fn render_tool_start_line(label: &str, use_color: bool) -> String {
    if use_color {
        // Cyan ▶ marker, dimmed preview so the marker stays the anchor.
        format!("\x1b[36m▶\x1b[0m \x1b[2m{label}\x1b[0m")
    } else {
        format!("▶ {label}")
    }
}

/// Render a tool-call event row: a ✓/✗ marker, the tool name, an
/// optional **subject** — the single most useful field from the tool's
/// INPUT (file path, command, URL, …) so the row is self-describing
/// without expanding the body. Falls back gracefully to the old
/// `name · summary` shape when `subject` is `None`.
#[allow(clippy::too_many_arguments)]
fn render_tool_event(
    name: &str,
    call_id: &str,
    summary: &str,
    is_error: bool,
    body: Option<&str>,
    subject: Option<&str>,
    use_color: bool,
    debug: bool,
) -> String {
    let id_suffix = if debug {
        format!(" ({call_id})")
    } else {
        String::new()
    };

    // The subject is the most informative thing on the row, so it's
    // rendered in *plain* (un-dimmed) text on success and bright red
    // on failure — both stand out next to the dim summary.
    let subject_segment = match subject {
        Some(s) if !s.is_empty() && use_color => {
            if is_error {
                format!("  \x1b[31m{s}\x1b[0m")
            } else {
                format!("  {s}")
            }
        }
        Some(s) if !s.is_empty() => format!("  {s}"),
        _ => String::new(),
    };

    let header = if is_error {
        if use_color {
            format!(
                "\x1b[31m✗\x1b[0m \x1b[1m{name}\x1b[0m{id_dim}{subject_segment}  \x1b[31m{summary}\x1b[0m",
                id_dim = if debug {
                    format!("\x1b[2m{id_suffix}\x1b[0m")
                } else {
                    String::new()
                }
            )
        } else {
            format!("✗ {name}{id_suffix}{subject_segment}  {summary}")
        }
    } else if use_color {
        format!(
            "\x1b[32m✓\x1b[0m \x1b[1m{name}\x1b[0m{id_dim}{subject_segment}  \x1b[2m{summary}\x1b[0m",
            id_dim = if debug {
                format!("\x1b[2m{id_suffix}\x1b[0m")
            } else {
                String::new()
            }
        )
    } else {
        format!("✓ {name}{id_suffix}{subject_segment}  {summary}")
    };

    match body {
        Some(body) if !body.is_empty() => {
            // Indent body lines with a dim left bar so they're visually
            // grouped with the tool header without overwhelming the bar.
            let prefix = if use_color {
                "\x1b[2m│\x1b[0m "
            } else {
                "│ "
            };
            let body_block = body
                .lines()
                .map(|line| format!("{prefix}{line}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{header}\n{body_block}")
        }
        _ => header,
    }
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

/// Interactive permission prompt invoked by the agent loop when a tool
/// call needs the user's approval. Stops the spinner, prints a one-line
/// summary of the pending tool call, reads the user's choice from stdin,
/// and returns the resulting [`AskOutcome`].
///
/// Input grammar (see [`parse_permission_response`]):
/// - empty / `y` / `yes`      → allow this call only
/// - `n` / `no`               → deny this call
/// - `t` / `this` / `tool`    → allow this call AND auto-approve every
///   subsequent call to the **same tool** for the rest of the session
///   (session-scoped rule, NOT persisted to disk).
/// - `a` / `all` / `yolo`     → allow this call AND flip the session to
///   YOLO (Bypass) so subsequent risky calls don't re-prompt either.
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

    // For file-modifying tools (Write / Edit / AppendFile), render a
    // unified diff preview so the user can see what's about to change
    // before approving. The preview is bounded to a few dozen lines so
    // even a large rewrite doesn't dominate the prompt. Unknown tools
    // (Bash, Grep, ...) return None and we fall back to the one-line
    // summary alone.
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let diff_block = match deeptide_core::render_tool_call_diff(
        &tool_call.name,
        &tool_call.input,
        &cwd,
        deeptide_core::DiffPreviewOptions::default(),
    ) {
        Some(preview) => render_diff_preview_block(&preview, use_color),
        None => String::new(),
    };

    let prompt = if use_color {
        "  \x1b[2m[y]es / [n]o / [t]his-tool / [a]ll-yolo\x1b[0m  > "
    } else {
        "  [y]es / [n]o / [t]his-tool / [a]ll-yolo  > "
    };
    let _ = stdout.write_all(header.as_bytes());
    if !diff_block.is_empty() {
        let _ = stdout.write_all(diff_block.as_bytes());
    }
    let _ = stdout.write_all(prompt.as_bytes());
    let _ = stdout.flush();

    output_started.store(true, Ordering::Relaxed);

    let mut response = String::new();
    if io::stdin().read_line(&mut response).is_err() {
        return AskOutcome::Deny {
            reason: String::from("failed to read approval from stdin"),
        };
    }

    let outcome = parse_permission_response(&response, &tool_call.name);

    // Echo a hint when a session-wide effect was triggered so the user
    // understands why subsequent calls stop prompting.
    match &outcome {
        AskOutcome::AllowAndSetMode(_) => {
            let _ = writeln!(
                stdout,
                "  (session switched to YOLO mode — Shift+Tab to cycle back)"
            );
        }
        AskOutcome::AllowAllSession { tool_name } => {
            let _ = writeln!(
                stdout,
                "  (session-allowing every {tool_name} call until exit)"
            );
        }
        _ => {}
    }

    outcome
}

/// Pure parser for the permission-prompt response. Splitting the I/O
/// (`handle_permission_prompt`) from the decision logic (this function)
/// keeps the keyword matching unit-testable without faking stdin.
fn parse_permission_response(raw: &str, tool_name: &str) -> AskOutcome {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "y" | "yes" => AskOutcome::Allow,
        "n" | "no" => AskOutcome::Deny {
            reason: String::from("user declined"),
        },
        "t" | "this" | "tool" | "this-tool" | "always" => AskOutcome::AllowAllSession {
            tool_name: tool_name.to_owned(),
        },
        "a" | "all" | "yolo" | "all-yolo" => AskOutcome::AllowAndSetMode(PermissionMode::Bypass),
        other => AskOutcome::Deny {
            reason: format!("user declined ({other})"),
        },
    }
}

/// Format a [`DiffPreview`] for display inside the permission ask.
/// Adds a left bar (`│`) so the preview visually nests under the
/// permission header, colourises `-`/`+` lines when the terminal
/// supports it, and bookends the block with `───` rules so it's clearly
/// delimited from the prompt the user types into below.
/// If `lines[start]` begins a run of deletion lines (`-…`, excluding the `---`
/// file header) immediately followed by an EQUAL-length run of insertion lines
/// (`+…`, excluding `+++`), return `(deletions, insertions, index_past_block)`
/// so the caller can pair them for word-level highlighting. Returns `None` for
/// any other shape (unequal runs, a lone deletion, context lines), in which
/// case the caller renders those lines individually.
fn paired_change_block<'a>(
    lines: &'a [&'a str],
    start: usize,
) -> Option<(&'a [&'a str], &'a [&'a str], usize)> {
    let is_del = |l: &str| l.starts_with('-') && !l.starts_with("---");
    let is_ins = |l: &str| l.starts_with('+') && !l.starts_with("+++");

    if !lines.get(start).is_some_and(|l| is_del(l)) {
        return None;
    }
    let mut del_end = start;
    while lines.get(del_end).is_some_and(|l| is_del(l)) {
        del_end += 1;
    }
    let mut ins_end = del_end;
    while lines.get(ins_end).is_some_and(|l| is_ins(l)) {
        ins_end += 1;
    }
    let dels = &lines[start..del_end];
    let inss = &lines[del_end..ins_end];
    // Only pair when the two runs line up 1:1 AND there's an insertion side —
    // a pure deletion (no following `+`) has nothing to word-diff against.
    if inss.is_empty() || dels.len() != inss.len() {
        return None;
    }
    Some((dels, inss, ins_end))
}

/// Render one side of a word-diffed line: `marker` (`-`/`+`) + body, painted in
/// `base_sgr` (red/green) with the CHANGED spans additionally bold + reverse-
/// video so the actual edit pops while the common text stays calm. Each changed
/// run toggles the emphasis on and back off, staying within the base colour.
fn render_word_diff_line(
    marker: char,
    base_sgr: &str,
    spans: &[deeptide_core::word_diff::Span],
) -> String {
    use deeptide_core::word_diff::Span;
    let mut body = String::new();
    for span in spans {
        match span {
            Span::Common(text) => body.push_str(text),
            // Bold + reverse for the changed run, then clear just those two
            // attributes (22 = not-bold, 27 = not-reverse) so we fall back to
            // the line's base colour rather than a full reset.
            Span::Changed(text) => {
                body.push_str("\x1b[1m\x1b[7m");
                body.push_str(text);
                body.push_str("\x1b[27m\x1b[22m");
            }
        }
    }
    format!("  │ {base_sgr}{marker}{body}\x1b[0m")
}

fn render_diff_preview_block(preview: &deeptide_core::DiffPreview, use_color: bool) -> String {
    let mut out = String::new();
    let rule = if use_color {
        "\x1b[2m──────── proposed change ────────\x1b[0m"
    } else {
        "──────── proposed change ────────"
    };
    out.push_str("  ");
    out.push_str(rule);
    out.push('\n');
    if use_color {
        out.push_str(&format!("  \x1b[1m{}\x1b[0m\n", preview.summary));
    } else {
        out.push_str(&format!("  {}\n", preview.summary));
    }
    // Diff +/- colours come from the active theme (default `dark` reproduces
    // the prior hard-coded green/red, so existing snapshots are unchanged;
    // `high-contrast` bolds them).
    let diff = &deeptide_core::theme::active().diff;
    let lines: Vec<&str> = preview.body.lines().collect();
    let mut idx = 0;
    while idx < lines.len() {
        // Word-level highlighting: when a run of deletion lines is immediately
        // followed by an equal-length run of insertion lines, the edit is an
        // in-place rewrite — pair them positionally and brighten only the
        // changed words on each side (the rest of the line stays calm), the
        // `git --word-diff` / zero-cli experience. Any other shape falls
        // through to the per-line styling below.
        if use_color && let Some((dels, inss, next)) = paired_change_block(&lines, idx) {
            // Compute each pair's word-level spans once, then emit all the
            // deletions followed by all the insertions (unified-diff order).
            let paired: Vec<_> = dels
                .iter()
                .zip(inss.iter())
                .map(|(del, ins)| deeptide_core::word_diff::word_diff(&del[1..], &ins[1..]))
                .collect();
            for (old_spans, _new_spans) in &paired {
                out.push_str(&render_word_diff_line('-', &diff.removed, old_spans));
                out.push('\n');
            }
            for (_old_spans, new_spans) in &paired {
                out.push_str(&render_word_diff_line('+', &diff.added, new_spans));
                out.push('\n');
            }
            idx = next;
            continue;
        }

        let line = lines[idx];
        let styled = if !use_color {
            format!("  │ {line}")
        } else if let Some(rest) = line.strip_prefix("$ ") {
            // Bash command body line — render like a code block:
            // dim cyan prefix + plain command text.
            format!("  │ \x1b[36m$\x1b[0m \x1b[1m{rest}\x1b[0m")
        } else if let Some(rest) = line.strip_prefix("! ") {
            // Bash risk warning emitted by detect_bash_risks. Bold
            // red prefix makes it impossible to miss when scanning.
            format!("  │ \x1b[1;31m⚠\x1b[0m  \x1b[31m{rest}\x1b[0m")
        } else if let Some(rest) = line.strip_prefix('+') {
            format!("  │ {}+{rest}\x1b[0m", diff.added)
        } else if let Some(rest) = line.strip_prefix('-') {
            format!("  │ {}-{rest}\x1b[0m", diff.removed)
        } else if line.starts_with("@@") {
            format!("  │ \x1b[36m{line}\x1b[0m")
        } else if line.starts_with("---") || line.starts_with("+++") {
            format!("  │ \x1b[2m{line}\x1b[0m")
        } else {
            format!("  │ {line}")
        };
        out.push_str(&styled);
        out.push('\n');
        idx += 1;
    }
    out.push_str("  ");
    out.push_str(rule);
    out.push_str("\n\n");
    out
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
        // Bold-red `auth —` so the user immediately spots a
        // mis-configured session instead of wondering why every
        // turn fails with an auth error.
        (false, false) => {
            StatusSegment::new("auth", "—").with_severity(deeptide_core::Severity::Alert)
        }
    }
}

fn subagent_backend_factory(
    config: Option<SubagentConfig>,
) -> impl Fn(&str) -> Box<dyn AgentBackend> + Send + Sync + 'static {
    move |model| match config.clone() {
        // Spawn sub-agents on the SAME protocol/endpoint as the main loop, just
        // re-pointed at the requested model.
        Some(SubagentConfig::Anthropic(mut config)) => {
            config.model = model.to_owned();
            match AnthropicBackend::new(config) {
                Ok(backend) => Box::new(backend),
                Err(_) => Box::<LocalEchoBackend>::default(),
            }
        }
        Some(SubagentConfig::OpenAi(mut config)) => {
            config.model = model.to_owned();
            match deeptide_core::OpenAiBackend::new(config) {
                Ok(backend) => Box::new(backend) as Box<dyn AgentBackend>,
                Err(_) => Box::<LocalEchoBackend>::default(),
            }
        }
        Some(SubagentConfig::Gemini(mut config)) => {
            config.model = model.to_owned();
            match deeptide_core::GeminiBackend::new(config) {
                Ok(backend) => Box::new(backend) as Box<dyn AgentBackend>,
                Err(_) => Box::<LocalEchoBackend>::default(),
            }
        }
        None => Box::<LocalEchoBackend>::default(),
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
    if cli.input_format == InputFormat::StreamJson
        || cli.read_stdin
        || (cli.print_mode && cli.prompt.is_none())
    {
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
    // Test fixtures use std::fs / tempfile setup where failure means
    // the test infrastructure is broken, not the code under test —
    // `.unwrap()` is fine for that. Opt-out is local to this module.
    #![allow(clippy::unwrap_used)]

    use super::{
        Cli, DEFAULT_BASE_URL, DEFAULT_MODEL, FIXED_ARG_SUGGESTIONS, FsCandidate, InputFormat,
        LiveToolArgsTicker, OutputFormat, ReplHelper, SubagentConfig, VERSION_LONG, VERSION_SHORT,
        apply_config_fallbacks, at_path_completion, build_auth_segment, collect_prompt,
        compute_at_path_hint, compute_hint, configured_backend, drain_live_tool_args,
        editor_completion, editor_hint, effective_base_url, effective_model,
        format_retry_notice_line, is_subsequence, next_permission_mode, normalize_embedded_mode,
        paean_token_resolved, parse_permission_response, render_system_message,
        summarize_tool_call_for_prompt, truncate_inline, use_color, validate_formats,
    };
    use clap::Parser;
    use deeptide_core::AskOutcome;
    use deeptide_core::CommandCompletionSource;
    use deeptide_core::permissions::PermissionMode;
    use deeptide_core::{
        AnthropicAuthMode, ConfigData, ProviderProfile, SystemMessage, ThinkingConfig,
    };
    use std::collections::HashMap;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    fn sample_cli() -> Cli {
        Cli {
            prompt: None,
            print_mode: false,
            input_format: InputFormat::Text,
            output_format: OutputFormat::Text,
            embedded: false,
            read_stdin: false,
            session_id: None,
            cwd: None,
            permission_mode: "default".to_owned(),
            theme: "dark".to_owned(),
            model: "deepseek-v4-flash".to_owned(),
            base_url: "https://api.anthropic.com".to_owned(),
            api_key: None,
            profile: None,
            provider: None,
            list_providers: false,
            fallback_model: None,
            thinking: None,
            effort: None,
            max_output_tokens: 65_536,
            max_turns: 200,
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
            name: None,
            list_sessions: false,
            import: None,
            import_session: None,
            import_as: "context".to_owned(),
            list_models: false,
            doctor: false,
            gui: false,
            no_session_persistence: false,
            no_session_capture: false,
            no_suggestions: false,
            settings: None,
            isolated: false,
            add_dir: Vec::new(),
        }
    }

    #[test]
    fn retry_notice_renders_compact_label_without_color() {
        let notice = deeptide_core::StreamRetryNotice {
            attempt: 1,
            max_attempts: 3,
            reason: "stream cut".to_owned(),
        };
        let line = format_retry_notice_line(&notice, false);
        // Plain mode: no ANSI escapes, single newline, indented label.
        assert_eq!(line, "  ↻ retry 1/3 — stream cut\n");
        assert!(
            !line.contains('\x1b'),
            "plain mode must have no escapes: {line:?}"
        );
    }

    #[test]
    fn retry_notice_wraps_label_in_dim_cyan_when_color_on() {
        let notice = deeptide_core::StreamRetryNotice {
            attempt: 2,
            max_attempts: 3,
            reason: "upstream proxy cut".to_owned(),
        };
        let line = format_retry_notice_line(&notice, true);
        // Dim (2) + cyan (36) opening, reset (0) closing. Sole newline
        // at the very end so subsequent TextDelta lands on its own row.
        assert!(
            line.starts_with("\x1b[2;36m"),
            "missing dim-cyan opener: {line:?}"
        );
        assert!(
            line.ends_with("\x1b[0m\n"),
            "missing reset+newline: {line:?}"
        );
        assert!(line.contains("retry 2/3"));
        assert!(line.contains("upstream proxy cut"));
    }

    #[test]
    fn retry_notice_omits_separator_when_reason_is_empty() {
        // Empty-reason path: the em-dash separator and trailing text
        // must vanish entirely so we don't render "↻ retry 1/3 —" with
        // a dangling dash that looks like a truncated message.
        let notice = deeptide_core::StreamRetryNotice {
            attempt: 1,
            max_attempts: 3,
            reason: String::new(),
        };
        let line = format_retry_notice_line(&notice, false);
        assert_eq!(line, "  ↻ retry 1/3\n");
        assert!(
            !line.contains('—'),
            "no separator allowed when reason is empty: {line:?}"
        );
    }

    #[test]
    fn retry_notice_truncates_long_reasons_inline() {
        let long_reason: String = "x".repeat(200);
        let notice = deeptide_core::StreamRetryNotice {
            attempt: 3,
            max_attempts: 3,
            reason: long_reason,
        };
        let line = format_retry_notice_line(&notice, false);
        // Hard cap: the inline truncator keeps reasons at ≤80 chars +
        // ellipsis so the notice fits on a typical terminal row even
        // with the "  ↻ retry N/M — " preamble. Total line stays well
        // under 100 columns.
        assert!(line.contains('…'), "long reason must be elided: {line:?}");
        assert!(
            line.chars().count() < 100,
            "notice too long: {line:?} ({} chars)",
            line.chars().count()
        );
    }

    #[test]
    fn truncate_inline_is_a_no_op_for_short_text() {
        assert_eq!(truncate_inline("hello", 80), "hello");
        assert_eq!(truncate_inline("", 80), "");
    }

    #[test]
    fn truncate_inline_respects_char_count_not_byte_count_for_cjk() {
        // 中文每字符占 1 个 char 但占 2 个显示格。本函数以 char 计数，
        // 所以 5 个中文 ≤ 5 不裁剪；6 个 + 上限 5 → 4 个中文 + …。
        let cjk = "你好世界编程";
        assert_eq!(truncate_inline(cjk, 6), cjk);
        let cut = truncate_inline(cjk, 5);
        assert!(cut.ends_with('…'));
        assert_eq!(cut.chars().count(), 5);
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

        let configured = configured_backend(&cli).expect("backend");
        let config = configured
            .subagent_config
            .as_ref()
            .and_then(SubagentConfig::as_anthropic)
            .expect("anthropic config");
        assert_eq!(config.thinking, Some(ThinkingConfig::high()));
    }

    #[test]
    fn configured_backend_uses_effort_when_thinking_unset() {
        let _guard = env_guard();
        clear_api_env();
        let mut cli = sample_cli();
        cli.api_key = Some("k".to_owned());
        cli.effort = Some("low".to_owned());

        let configured = configured_backend(&cli).expect("backend");
        let config = configured
            .subagent_config
            .as_ref()
            .and_then(SubagentConfig::as_anthropic)
            .expect("anthropic config");
        assert_eq!(config.thinking, Some(ThinkingConfig::low()));
    }

    #[test]
    fn configured_backend_omits_thinking_for_auto() {
        let _guard = env_guard();
        clear_api_env();
        let mut cli = sample_cli();
        cli.api_key = Some("k".to_owned());
        cli.thinking = Some("auto".to_owned());

        let configured = configured_backend(&cli).expect("backend");
        let config = configured
            .subagent_config
            .as_ref()
            .and_then(SubagentConfig::as_anthropic)
            .expect("anthropic config");
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
    fn read_stdin_appends_piped_content_to_prompt() {
        let mut cli = sample_cli();
        cli.print_mode = true;
        cli.read_stdin = true;
        cli.prompt = Some("review this patch".to_owned());

        let prompt = collect_prompt(&cli, Some("  diff --git a b\n"))
            .unwrap_or_else(|error| panic!("read-stdin should append: {error}"));

        assert_eq!(prompt, "review this patch\n\ndiff --git a b");
    }

    #[test]
    fn no_tui_alias_enables_print_mode() {
        let cli = Cli::try_parse_from(["deeptide", "--no-tui", "-p", "hello"])
            .expect("--no-tui alias should parse");

        assert!(cli.print_mode);
    }

    #[test]
    fn max_tokens_alias_maps_to_max_output_tokens() {
        let cli = Cli::try_parse_from(["deeptide", "--max-tokens", "1234", "--print", "-p", "hi"])
            .expect("--max-tokens alias should parse");

        assert_eq!(cli.max_output_tokens, 1234);
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
        let subagent_config = configured
            .subagent_config
            .as_ref()
            .and_then(SubagentConfig::as_anthropic)
            .expect("anthropic subagent config");
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
        let subagent_config = configured
            .subagent_config
            .as_ref()
            .and_then(SubagentConfig::as_anthropic)
            .expect("anthropic subagent config");
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
        let cfg = configured
            .subagent_config
            .as_ref()
            .and_then(SubagentConfig::as_anthropic)
            .expect("anthropic subagent config");
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
        let cfg = configured
            .subagent_config
            .as_ref()
            .and_then(SubagentConfig::as_anthropic)
            .expect("anthropic subagent config");
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
        let cfg = configured
            .subagent_config
            .as_ref()
            .and_then(SubagentConfig::as_anthropic)
            .expect("anthropic subagent config");
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
    fn diff_preview_word_highlights_only_changed_words_in_paired_lines() {
        use deeptide_core::DiffPreview;
        // An equal-length -/+ block is an in-place rewrite: only the changed
        // word gets reverse-video (`\x1b[7m`), the common text does not.
        let preview = DiffPreview {
            summary: "edit f.rs".to_owned(),
            body: "--- f.rs  (1)\n+++ f.rs  (1)\n@@ -1,1 +1,1 @@\n-let x = 1;\n+let x = 2;"
                .to_owned(),
        };
        let out = super::render_diff_preview_block(&preview, true);
        // The changed digits are wrapped in bold+reverse.
        assert!(
            out.contains("\x1b[1m\x1b[7m1\x1b[27m\x1b[22m"),
            "old `1` should be reverse-highlighted: {out:?}"
        );
        assert!(
            out.contains("\x1b[1m\x1b[7m2\x1b[27m\x1b[22m"),
            "new `2` should be reverse-highlighted: {out:?}"
        );
        // The common prefix `let x = ` must NOT be inside a reverse-video run.
        assert!(
            !out.contains("\x1b[7mlet"),
            "common text must stay calm: {out:?}"
        );
    }

    #[test]
    fn diff_preview_falls_back_to_plain_coloring_for_unequal_blocks() {
        use deeptide_core::DiffPreview;
        // 1 deletion, 2 insertions → not a 1:1 rewrite → no word-diff, just the
        // existing per-line red/green (no reverse-video anywhere).
        let preview = DiffPreview {
            summary: "edit f.rs".to_owned(),
            body: "@@ -1,1 +1,2 @@\n-old line\n+new line one\n+new line two".to_owned(),
        };
        let out = super::render_diff_preview_block(&preview, true);
        assert!(
            !out.contains("\x1b[7m"),
            "no word-diff on unequal runs: {out:?}"
        );
        assert!(
            out.contains("\x1b[31m-old line\x1b[0m"),
            "plain red deletion: {out:?}"
        );
        assert!(
            out.contains("\x1b[32m+new line one\x1b[0m"),
            "plain green insertion: {out:?}"
        );
    }

    #[test]
    fn headless_permission_outcome_maps_allowed_remember_and_denied() {
        use deeptide_core::AskOutcome;
        use deeptide_core::embedded_protocol::PermissionResponse;
        let resp = |allowed, remember| PermissionResponse {
            request_id: None,
            tool_use_id: None,
            allowed,
            remember,
        };
        // allow once
        assert!(matches!(
            super::headless_permission_outcome(&resp(true, false), "Write"),
            AskOutcome::Allow
        ));
        // allow + remember → whitelist the whole tool
        match super::headless_permission_outcome(&resp(true, true), "Bash") {
            AskOutcome::AllowAllSession { tool_name } => assert_eq!(tool_name, "Bash"),
            other => panic!("expected AllowAllSession, got {other:?}"),
        }
        // denied
        assert!(matches!(
            super::headless_permission_outcome(&resp(false, false), "Write"),
            AskOutcome::Deny { .. }
        ));
    }

    #[test]
    fn decide_headless_permission_emits_request_and_consumes_queue_by_id_then_fifo() {
        use deeptide_core::embedded_protocol::PermissionResponse;
        use deeptide_core::{AskOutcome, ToolCall};
        use std::collections::VecDeque;

        let call = |id: &str, name: &str| ToolCall {
            id: id.to_owned(),
            name: name.to_owned(),
            input: serde_json::json!({"file_path": "a"}),
        };
        // One response addressed to tool-2, one unaddressed.
        let mut queue = VecDeque::from(vec![
            PermissionResponse {
                request_id: None,
                tool_use_id: Some("tool-2".to_owned()),
                allowed: true,
                remember: false,
            },
            PermissionResponse {
                request_id: None,
                tool_use_id: None,
                allowed: false,
                remember: false,
            },
        ]);

        let mut emitted = Vec::new();
        let mut emit = |line: &str| emitted.push(line.to_owned());

        // tool-1 has no addressed response → takes the unaddressed (deny).
        let o1 = super::decide_headless_permission(&call("tool-1", "Write"), &mut queue, &mut emit);
        assert!(matches!(o1, AskOutcome::Deny { .. }), "tool-1: {o1:?}");
        // tool-2's addressed response remains and is matched by id → allow.
        let o2 = super::decide_headless_permission(&call("tool-2", "Read"), &mut queue, &mut emit);
        assert!(matches!(o2, AskOutcome::Allow), "tool-2: {o2:?}");
        assert!(queue.is_empty(), "both responses consumed");

        // A permission_request was emitted per call, carrying the tool_use_id.
        assert_eq!(emitted.len(), 2);
        let first: serde_json::Value = serde_json::from_str(&emitted[0]).expect("json");
        assert_eq!(first["type"], "permission_request");
        assert_eq!(first["tool_use_id"], "tool-1");
        assert_eq!(first["tool"], "Write");
    }

    #[test]
    fn decide_headless_permission_denies_with_reason_when_queue_empty() {
        use deeptide_core::{AskOutcome, ToolCall};
        use std::collections::VecDeque;
        let mut queue = VecDeque::new();
        let mut emit = |_: &str| {};
        let outcome = super::decide_headless_permission(
            &ToolCall {
                id: "t1".to_owned(),
                name: "Bash".to_owned(),
                input: serde_json::json!({}),
            },
            &mut queue,
            &mut emit,
        );
        match outcome {
            AskOutcome::Deny { reason } => {
                assert!(reason.contains("Bash") && reason.contains("permission_response"));
            }
            other => panic!("expected Deny, got {other:?}"),
        }
    }

    #[test]
    fn thinking_delta_event_is_a_distinct_type_from_assistant_delta() {
        // Reasoning chain-of-thought rides its own `thinking_delta` type so
        // headless consumers can render it separately from the answer.
        let event = super::thinking_delta_event("let me reason: 2+2");
        let parsed: serde_json::Value = serde_json::from_str(&event).expect("valid JSON");
        assert_eq!(parsed["type"], "thinking_delta");
        assert_eq!(parsed["delta"], "let me reason: 2+2");
        // Must NOT masquerade as an answer delta.
        assert_ne!(parsed["type"], "assistant_delta");
    }

    #[test]
    fn stream_json_event_line_encodes_tool_result_and_batch() {
        use deeptide_core::{AgentLoopEvent, ToolCall};

        // A tool result → a `tool_result` line carrying name/id/error/content.
        let result = AgentLoopEvent::ToolResult {
            tool_call: ToolCall::new("call_1", "Bash", serde_json::json!({"command": "ls"})),
            content: "file.txt\n".to_owned(),
            is_error: false,
        };
        let line = super::stream_json_event_line(&result).expect("tool_result encodes");
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["type"], "tool_result");
        assert_eq!(v["tool"], "Bash");
        assert_eq!(v["tool_use_id"], "call_1");
        assert_eq!(v["is_error"], false);
        assert_eq!(v["content"], "file.txt\n");

        // A batch summary → a `tool_batch` line with the tool names + failures.
        let batch = AgentLoopEvent::ToolBatchSummary {
            label: "Ran 2 tools".to_owned(),
            tool_calls: vec![
                ToolCall::new("a", "Read", serde_json::json!({})),
                ToolCall::new("b", "Bash", serde_json::json!({})),
            ],
            failed_count: 1,
        };
        let line = super::stream_json_event_line(&batch).expect("tool_batch encodes");
        let v: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(v["type"], "tool_batch");
        assert_eq!(v["tools"], serde_json::json!(["Read", "Bash"]));
        assert_eq!(v["failed_count"], 1);
    }

    #[test]
    fn stream_json_event_line_skips_text_and_terminal_events() {
        use deeptide_core::{AgentLoopEvent, AgentTerminalEvent, ConversationMessage};
        // Assistant text already streams as assistant_delta; don't double it.
        assert!(
            super::stream_json_event_line(&AgentLoopEvent::Assistant(
                ConversationMessage::assistant("hi")
            ))
            .is_none()
        );
        assert!(
            super::stream_json_event_line(&AgentLoopEvent::Terminal(AgentTerminalEvent::Complete))
                .is_none()
        );
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
    fn permission_response_empty_or_yes_allows() {
        for input in ["", "\n", "y", "Y\n", "yes", "  YES  "] {
            assert!(
                matches!(parse_permission_response(input, "Write"), AskOutcome::Allow),
                "{input:?} should map to Allow"
            );
        }
    }

    #[test]
    fn permission_response_no_denies_with_user_declined_reason() {
        for input in ["n", "N\n", "no", "  NO  "] {
            let outcome = parse_permission_response(input, "Bash");
            match outcome {
                AskOutcome::Deny { reason } => assert_eq!(reason, "user declined"),
                other => panic!("{input:?} should Deny, got {other:?}"),
            }
        }
    }

    #[test]
    fn permission_response_this_tool_installs_session_allow_for_named_tool() {
        for input in ["t", "T\n", "this", "tool", "this-tool", "  always  "] {
            match parse_permission_response(input, "Write") {
                AskOutcome::AllowAllSession { tool_name } => {
                    assert_eq!(tool_name, "Write", "input={input:?}");
                }
                other => panic!("{input:?} should AllowAllSession, got {other:?}"),
            }
        }
    }

    #[test]
    fn permission_response_all_flips_session_into_bypass() {
        for input in ["a", "A\n", "all", "yolo", "all-yolo"] {
            match parse_permission_response(input, "Write") {
                AskOutcome::AllowAndSetMode(mode) => {
                    assert_eq!(mode, PermissionMode::Bypass, "input={input:?}");
                }
                other => panic!("{input:?} should AllowAndSetMode, got {other:?}"),
            }
        }
    }

    #[test]
    fn permission_response_unknown_token_safely_denies_with_echoed_token() {
        // Fail closed: an unrecognised response must NOT default to Allow.
        // The echoed token helps the user retry without re-reading the
        // tool summary.
        match parse_permission_response("maybe", "Bash") {
            AskOutcome::Deny { reason } => {
                assert!(reason.contains("maybe"), "got: {reason}");
            }
            other => panic!("unknown input should Deny, got {other:?}"),
        }
    }

    // Reuse the canonical helper from the core width module so this test
    // observes exactly the same ANSI semantics as production code.
    use deeptide_core::width::strip_ansi;

    #[test]
    fn tool_batch_summary_dims_with_dot_prefix() {
        let plain = strip_ansi(&render_system_message(
            &SystemMessage::ToolBatch {
                label: "Read 2 files in src/".into(),
                failed_count: 0,
            },
            true,
            false,
        ));
        assert_eq!(plain, "· Tools completed: Read 2 files in src/");
        // ANSI codes present for color-on: dim escape \x1b[2m must appear.
        let colored = render_system_message(
            &SystemMessage::ToolBatch {
                label: "Read 2 files in src/".into(),
                failed_count: 0,
            },
            true,
            false,
        );
        assert!(colored.contains("\x1b[2m"), "got: {colored:?}");
    }

    #[test]
    fn tool_batch_failure_uses_failed_verb() {
        let plain = strip_ansi(&render_system_message(
            &SystemMessage::ToolBatch {
                label: "Wrote 1 write, 1 failed".into(),
                failed_count: 1,
            },
            false,
            false,
        ));
        assert_eq!(
            plain,
            "· Tools completed with failures: Wrote 1 write, 1 failed"
        );
    }

    #[test]
    fn tool_success_shows_green_check_and_hides_call_id_by_default() {
        let plain = strip_ansi(&render_system_message(
            &SystemMessage::Tool {
                name: "Read".into(),
                call_id: "toolu_abc123".into(),
                summary: "1 lines".into(),
                is_error: false,
                body: None,
                subject: None,
            },
            true,
            false,
        ));
        // Visible glyphs only — call_id must not leak.
        assert!(plain.starts_with("✓ Read"), "got: {plain:?}");
        assert!(plain.contains("1 lines"), "got: {plain:?}");
        assert!(!plain.contains("toolu_abc123"), "got: {plain:?}");
    }

    #[test]
    fn tool_success_surfaces_call_id_under_debug() {
        let plain = strip_ansi(&render_system_message(
            &SystemMessage::Tool {
                name: "Read".into(),
                call_id: "toolu_abc123".into(),
                summary: "1 lines".into(),
                is_error: false,
                body: None,
                subject: None,
            },
            false,
            true,
        ));
        assert!(plain.contains("toolu_abc123"), "got: {plain:?}");
    }

    #[test]
    fn tool_failure_shows_red_cross_and_summary() {
        let colored = render_system_message(
            &SystemMessage::Tool {
                name: "Write".into(),
                call_id: "toolu_w".into(),
                summary: "Permission required for Write.".into(),
                is_error: true,
                body: None,
                subject: None,
            },
            true,
            false,
        );
        // Red escape \x1b[31m must appear for the failure glyph and summary.
        assert!(colored.contains("\x1b[31m"), "got: {colored:?}");
        let plain = strip_ansi(&colored);
        assert!(plain.starts_with("✗ Write"), "got: {plain:?}");
        assert!(plain.contains("Permission required"), "got: {plain:?}");
    }

    #[test]
    fn tool_subject_appears_between_name_and_summary() {
        // T2.1: the per-tool row should now self-describe what was
        // acted on — the subject (file path / command / URL) is
        // spliced between the tool name and the result summary.
        let plain = strip_ansi(&render_system_message(
            &SystemMessage::Tool {
                name: "Read".into(),
                call_id: "toolu_read".into(),
                summary: "966 lines (29.8 KB)".into(),
                is_error: false,
                body: None,
                subject: Some("src/main.rs".into()),
            },
            true,
            false,
        ));
        // Expect: ✓ Read  src/main.rs  966 lines (29.8 KB)
        assert!(plain.starts_with("✓ Read"), "got: {plain:?}");
        let read_pos = plain.find("Read").unwrap();
        let subject_pos = plain
            .find("src/main.rs")
            .unwrap_or_else(|| panic!("subject missing: {plain:?}"));
        let summary_pos = plain
            .find("966 lines")
            .unwrap_or_else(|| panic!("summary missing: {plain:?}"));
        assert!(
            read_pos < subject_pos && subject_pos < summary_pos,
            "ordering must be name → subject → summary: {plain:?}"
        );
    }

    #[test]
    fn tool_no_subject_keeps_old_two_space_separator_layout() {
        // When subject is None, the row should look identical to the
        // pre-T2.1 layout (name then summary, two spaces).
        let plain = strip_ansi(&render_system_message(
            &SystemMessage::Tool {
                name: "Bash".into(),
                call_id: "toolu_x".into(),
                summary: "ok".into(),
                is_error: false,
                body: None,
                subject: None,
            },
            true,
            false,
        ));
        assert_eq!(plain, "✓ Bash  ok");
    }

    #[test]
    fn tool_body_renders_with_dim_left_bar() {
        let plain = strip_ansi(&render_system_message(
            &SystemMessage::Tool {
                name: "Read".into(),
                call_id: "x".into(),
                summary: "2 lines".into(),
                is_error: false,
                body: Some("1\talpha\n2\tbeta".into()),
                subject: None,
            },
            true,
            false,
        ));
        let lines: Vec<&str> = plain.lines().collect();
        assert!(lines[0].starts_with("✓ Read"), "got: {lines:?}");
        assert_eq!(lines[1], "│ 1\talpha");
        assert_eq!(lines[2], "│ 2\tbeta");
    }

    #[test]
    fn compaction_message_dims_with_dot_prefix() {
        let plain = strip_ansi(&render_system_message(
            &SystemMessage::Compaction {
                compressed_messages: 4,
                tokens_after: 12_345,
            },
            true,
            false,
        ));
        assert_eq!(
            plain,
            "· Context auto-compacted: folded 4 earlier message(s); ~12345 tokens now."
        );
    }

    #[test]
    fn notice_uses_alert_glyph_and_yellow_color() {
        let colored = render_system_message(
            &SystemMessage::Notice("Maximum turns reached.".into()),
            true,
            false,
        );
        assert!(colored.contains("\x1b[33m"), "got: {colored:?}");
        let plain = strip_ansi(&colored);
        assert_eq!(plain, "⚠ Maximum turns reached.");

        // Plain mode falls back to ASCII for portability.
        let plain_mode =
            render_system_message(&SystemMessage::Notice("Boom.".into()), false, false);
        assert_eq!(plain_mode, "! Boom.");
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

    // ---------- Slash-command completion & ghost-text hint ----------
    //
    // These tests cover the user-reported gaps:
    //   * `/exit` must appear in the registered command list (regression
    //     against accidentally removing it from `repl_command_sources`)
    //   * Tab completion for partial command names must surface candidates
    //   * Inline hint shows only on unambiguous prefix matches
    //   * The expanded argument-completion table (FIXED_ARG_SUGGESTIONS)
    //     covers /permission, /cost, /provider, /tps, /update, /dream,
    //     /cron, /goal, /config, /clear --yes, /new --yes, /compact --yes,
    //     /reminder, /sessions, /context, /debug, /branch.

    fn sample_commands() -> Vec<CommandCompletionSource> {
        vec![
            CommandCompletionSource::new(
                "help",
                ["h", "?"],
                "Show available commands and keybindings",
                "/help [command]",
            ),
            CommandCompletionSource::new("exit", ["quit", "q"], "Exit the REPL", "/exit"),
            CommandCompletionSource::new("export", Vec::<&str>::new(), "Export", "/export"),
            CommandCompletionSource::new("memory", ["mem"], "Manage memory", "/memory"),
            CommandCompletionSource::new("model", ["m"], "Switch model", "/model <name>"),
            CommandCompletionSource::new(
                "permission",
                ["perm", "permissions"],
                "Manage permissions",
                "/permission",
            ),
            CommandCompletionSource::new("cost", Vec::<&str>::new(), "Cost", "/cost"),
            CommandCompletionSource::new(
                "provider",
                ["profiles"],
                "Provider profiles",
                "/provider",
            ),
        ]
    }

    fn helper() -> ReplHelper {
        ReplHelper::new(sample_commands())
    }

    #[test]
    fn editor_completion_resolves_slash_command_tail() {
        // `/exi` resolves to the `/exit` command via the editor bridge,
        // with `start` at the line head so the editor replaces from `/`.
        let helper = helper();
        let line = "/exi";
        let result = editor_completion(&helper, line, line.len());
        assert_eq!(result.start, 0);
        assert!(
            result.candidates.iter().any(|c| c == "/exit"),
            "expected /exit in {:?}",
            result.candidates
        );
    }

    #[test]
    fn editor_completion_resolves_argument_values() {
        // No command-name match for `/cost s`, so the bridge falls
        // through to the argument-completion table.
        let helper = helper();
        let line = "/cost s";
        let result = editor_completion(&helper, line, line.len());
        assert!(
            result.candidates.iter().any(|c| c == "show"),
            "expected 'show' arg suggestion, got {:?}",
            result.candidates
        );
    }

    #[test]
    fn editor_hint_matches_compute_hint() {
        let helper = helper();
        let line = "/exi";
        assert_eq!(
            editor_hint(&helper, line, line.len()),
            compute_hint(line, line.len(), &sample_commands())
        );
    }

    #[test]
    fn hint_offers_completion_tail_when_prefix_is_unambiguous() {
        // `/exi` only matches `/exit` as a prefix → hint should be "t".
        let line = "/exi";
        assert_eq!(
            compute_hint(line, line.len(), &sample_commands()).as_deref(),
            Some("t")
        );
    }

    #[test]
    fn hint_shows_palette_for_ambiguous_prefix() {
        // `/ex` matches both /exit and /export → the inline palette
        // surfaces both names so the user can discover what's
        // available without first pressing Tab. T1.2 behaviour.
        let line = "/ex";
        let hint = compute_hint(line, line.len(), &sample_commands())
            .expect("/ex with multiple prefix matches should yield a palette hint");
        assert!(
            hint.contains("/exit"),
            "palette should mention /exit: {hint}"
        );
        assert!(
            hint.contains("/export"),
            "palette should mention /export: {hint}"
        );
        assert!(
            hint.contains("(Tab)"),
            "palette should teach the Tab action: {hint}"
        );
    }

    #[test]
    fn hint_is_empty_for_complete_command() {
        // Already typed the full name → no trailing ghost text.
        let line = "/exit";
        assert!(compute_hint(line, line.len(), &sample_commands()).is_none());
    }

    #[test]
    fn hint_is_empty_when_cursor_not_at_end() {
        // Cursor mid-word: hint would visually clash with the typed tail.
        let line = "/exit";
        // pos=3 means cursor sits after "/ex" — hint would still be unambiguous
        // for the prefix, but rendering it past in-progress edits is jarring.
        assert!(compute_hint(line, 3, &sample_commands()).is_none());
    }

    #[test]
    fn hint_shows_palette_for_bare_slash() {
        // Just `/` → every command prefix-matches with score 0. The
        // palette surfaces the first MAX_INLINE names so the user
        // can SEE what commands exist without having to press Tab.
        // T1.2 behaviour.
        let line = "/";
        let hint = compute_hint(line, line.len(), &sample_commands())
            .expect("bare slash should yield a palette hint");
        // sample_commands() has more than MAX_INLINE (=4) entries, so
        // the palette must include a "+N more" suffix.
        assert!(
            hint.contains("+") && hint.contains("more"),
            "palette should truncate with `+N more` when there are extra commands: {hint}"
        );
        assert!(hint.contains("(Tab)"));
    }

    #[test]
    fn palette_caps_inline_entries_at_four_and_summarises_rest() {
        // With many matches, the palette must (a) include at most
        // four names inline so it doesn't wrap on an 80-column
        // terminal and (b) suffix with "+N more (Tab)" pointing at
        // the remaining count.
        let line = "/";
        let hint =
            compute_hint(line, line.len(), &sample_commands()).expect("bare slash yields palette");
        let dot_separator_count = hint.matches(" · ").count();
        // four names → three separators between them.
        assert_eq!(
            dot_separator_count, 3,
            "expected exactly 3 inline separators (= 4 names), got: {hint}"
        );
        // sample_commands() has 8 commands; palette shows 4 inline so
        // "+4 more" should appear in the suffix.
        assert!(
            hint.contains("+4 more"),
            "expected `+4 more` suffix for 8 commands - 4 inline: {hint}"
        );
    }

    #[test]
    fn palette_omits_more_suffix_when_all_fit() {
        // Only 2 matches (`/exit`, `/export`) → both fit inline, no
        // "+N more" needed.
        let line = "/ex";
        let hint = compute_hint(line, line.len(), &sample_commands()).expect("/ex yields palette");
        assert!(
            !hint.contains("+"),
            "no extra-count suffix expected: {hint}"
        );
        assert!(!hint.contains("more"), "no `more` suffix expected: {hint}");
        assert!(hint.contains("(Tab)"));
    }

    #[test]
    fn hint_is_empty_for_non_slash_input() {
        let line = "hello";
        assert!(compute_hint(line, line.len(), &sample_commands()).is_none());
    }

    #[test]
    fn hint_completes_aliases_when_uniquely_typed() {
        // `/quit` is an alias for `/exit` and shouldn't appear as a hint —
        // the alias is itself a complete command. `/qui` should hint `t`
        // (matched_text is the alias "quit").
        let line = "/qui";
        let hint = compute_hint(line, line.len(), &sample_commands());
        assert_eq!(
            hint.as_deref(),
            Some("t"),
            "hint should complete /quit → /quit (alias path)"
        );
    }

    #[test]
    fn arg_completion_table_contains_critical_commands() {
        // Sanity check: the table the user expects to feed argument
        // suggestions doesn't silently shrink.
        let heads: Vec<&str> = FIXED_ARG_SUGGESTIONS
            .iter()
            .flat_map(|(heads, _)| heads.iter().copied())
            .collect();
        for required in [
            "permission",
            "cost",
            "provider",
            "tps",
            "update",
            "dream",
            "cron",
            "goal",
            "config",
            "clear",
            "new",
            "compact",
            "reminder",
            "sessions",
            "context",
            "debug",
            "branch",
        ] {
            assert!(
                heads.contains(&required),
                "FIXED_ARG_SUGGESTIONS dropped /{required}; table is now: {heads:?}"
            );
        }
    }

    #[test]
    fn arg_completion_resolves_permission_flag() {
        let helper = helper();
        let line = "/permission --a";
        let (token_start, pairs) = helper
            .arg_completion(line, line.len())
            .expect("/permission --a must suggest --allow");
        assert_eq!(token_start, "/permission ".len());
        let suggestions: Vec<String> = pairs.iter().map(|p| p.replacement.clone()).collect();
        assert!(
            suggestions.iter().any(|s| s == "--allow"),
            "expected --allow in suggestions, got {suggestions:?}"
        );
    }

    #[test]
    fn arg_completion_resolves_permission_via_short_alias() {
        let helper = helper();
        // Critical: aliases must trigger the same arg completion as the
        // canonical name. Previous implementation only matched canonical.
        let line = "/perm --d";
        let pairs = helper
            .arg_completion(line, line.len())
            .expect("/perm alias must surface the permission arg list");
        let suggestions: Vec<String> = pairs.1.iter().map(|p| p.replacement.clone()).collect();
        assert!(
            suggestions.iter().any(|s| s == "--deny"),
            "expected --deny via alias /perm, got {suggestions:?}"
        );
    }

    #[test]
    fn arg_completion_resolves_cost_action() {
        let helper = helper();
        let line = "/cost s";
        let pairs = helper
            .arg_completion(line, line.len())
            .expect("/cost s must suggest 'show'");
        let suggestions: Vec<String> = pairs.1.iter().map(|p| p.replacement.clone()).collect();
        assert!(
            suggestions.contains(&"show".to_owned()),
            "got {suggestions:?}"
        );
    }

    #[test]
    fn arg_completion_resolves_provider_subverb() {
        let helper = helper();
        let line = "/provider l";
        let pairs = helper
            .arg_completion(line, line.len())
            .expect("/provider l must suggest 'list'");
        let suggestions: Vec<String> = pairs.1.iter().map(|p| p.replacement.clone()).collect();
        assert!(
            suggestions.contains(&"list".to_owned()),
            "got {suggestions:?}"
        );
    }

    #[test]
    fn arg_completion_resolves_help_for_registered_command() {
        let helper = helper();
        // `/help mem` should surface "memory" from the dynamic registered
        // command list (not the static FIXED_ARG_SUGGESTIONS table).
        let line = "/help mem";
        let pairs = helper
            .arg_completion(line, line.len())
            .expect("/help mem must surface registered command names");
        let suggestions: Vec<String> = pairs.1.iter().map(|p| p.replacement.clone()).collect();
        assert!(
            suggestions.iter().any(|s| s == "memory"),
            "expected memory in /help completion, got {suggestions:?}"
        );
    }

    #[test]
    fn arg_completion_for_unknown_command_returns_none() {
        let helper = helper();
        let line = "/notacommand foo";
        assert!(
            helper.arg_completion(line, line.len()).is_none(),
            "no completion should be returned for an unknown command head"
        );
    }

    // ---------- Version provenance ----------
    //
    // The user complaint that motivated build.rs was:
    //   $ deeptide-rs --version
    //   deeptide 0.1.0
    // ...with no way to tell *which* commit they had installed locally.
    // These tests guard the contract that --version output remains
    // meaningful even when build.rs falls back to "unknown" hashes.

    #[test]
    fn version_short_contains_pkg_version() {
        let pkg = env!("CARGO_PKG_VERSION");
        assert!(
            VERSION_SHORT.starts_with(pkg),
            "VERSION_SHORT must lead with the crate version, got: {VERSION_SHORT}"
        );
    }

    #[test]
    fn version_short_includes_provenance_envelope() {
        // The wrapped "(hash date)" envelope is what makes the short form
        // bug-report friendly. It must be present even when the build is
        // outside a git checkout (in which case both fields are "unknown").
        assert!(
            VERSION_SHORT.contains('('),
            "VERSION_SHORT must contain a (hash date) envelope, got: {VERSION_SHORT}"
        );
        assert!(VERSION_SHORT.contains(')'));
    }

    #[test]
    fn version_long_carries_each_label() {
        for label in ["commit:", "date:", "branch:", "rustc:"] {
            assert!(
                VERSION_LONG.contains(label),
                "VERSION_LONG missing `{label}`, got:\n{VERSION_LONG}"
            );
        }
    }

    #[test]
    fn version_long_starts_with_short_form() {
        // The first line of --version (long) is the same as -V (short) so
        // grep / pipe-to-head workflows show the same headline either way.
        let first_line = VERSION_LONG.lines().next().unwrap_or_default();
        assert_eq!(
            first_line, VERSION_SHORT,
            "VERSION_LONG's first line must match VERSION_SHORT verbatim"
        );
    }

    #[test]
    fn version_env_vars_are_non_empty() {
        // build.rs guarantees a fallback string for every variable, so even
        // a crates.io tarball build produces a usable --version. Catch a
        // future regression where someone removes the fallback.
        for name in [
            "DEEPTIDE_GIT_HASH",
            "DEEPTIDE_GIT_DATE",
            "DEEPTIDE_GIT_BRANCH",
            "DEEPTIDE_RUSTC",
        ] {
            let value = match name {
                "DEEPTIDE_GIT_HASH" => env!("DEEPTIDE_GIT_HASH"),
                "DEEPTIDE_GIT_DATE" => env!("DEEPTIDE_GIT_DATE"),
                "DEEPTIDE_GIT_BRANCH" => env!("DEEPTIDE_GIT_BRANCH"),
                "DEEPTIDE_RUSTC" => env!("DEEPTIDE_RUSTC"),
                _ => unreachable!(),
            };
            assert!(
                !value.is_empty(),
                "{name} must always be set to at least 'unknown' by build.rs, got empty"
            );
        }
    }

    // ---- @-file tab completion ----------------------------------------

    #[test]
    fn at_path_completion_returns_none_when_no_at_token_under_cursor() {
        // Plain text with no `@`: completer should defer back to the
        // slash-command path.
        let line = "hello world";
        let result = at_path_completion(line, line.len());
        assert!(result.is_none(), "expected None");
    }

    #[test]
    fn at_path_completion_lists_matching_entries_in_cwd() {
        // Stage a tempdir with two files, chdir into it, then drive
        // `at_path_completion` against `@<Tab>`. We can't easily mock
        // `current_dir`, so we set it for the duration of the test
        // (single-threaded test guard required).
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("alpha.txt"), "a").unwrap();
        std::fs::write(dir.path().join("beta.txt"), "b").unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let line = "look at @";
        let (start, pairs) = at_path_completion(line, line.len()).expect("at-completion fired");
        assert_eq!(start, "look at @".len());
        let names: Vec<&str> = pairs.iter().map(|p| p.replacement.as_str()).collect();
        assert!(names.contains(&"alpha.txt"), "missing alpha.txt: {names:?}");
        assert!(names.contains(&"beta.txt"), "missing beta.txt: {names:?}");

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn at_path_completion_filters_by_prefix() {
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("alpha.txt"), "a").unwrap();
        std::fs::write(dir.path().join("beta.txt"), "b").unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let line = "see @alp";
        let (_start, pairs) = at_path_completion(line, line.len()).expect("fired");
        let names: Vec<&str> = pairs.iter().map(|p| p.replacement.as_str()).collect();
        assert_eq!(names, vec!["alpha.txt"], "prefix filter: {names:?}");

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn at_path_completion_marks_directories_with_trailing_slash() {
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("README.md"), "x").unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let line = "@";
        let (_start, pairs) = at_path_completion(line, line.len()).expect("fired");
        let displays: Vec<&str> = pairs.iter().map(|p| p.replacement.as_str()).collect();
        assert!(
            displays.contains(&"src/"),
            "dir lacks trailing /: {displays:?}"
        );
        assert!(displays.contains(&"README.md"));

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn at_path_completion_skips_heavy_build_dirs() {
        // node_modules and target are on the deny list; they should
        // not surface even when matching a prefix.
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("node_modules")).unwrap();
        std::fs::create_dir(dir.path().join("target")).unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let (_start, pairs) = at_path_completion("@", 1).expect("fired");
        let displays: Vec<&str> = pairs.iter().map(|p| p.replacement.as_str()).collect();
        assert!(displays.contains(&"src/"));
        assert!(
            !displays.contains(&"node_modules/"),
            "deny-list miss: {displays:?}"
        );
        assert!(
            !displays.contains(&"target/"),
            "deny-list miss: {displays:?}"
        );

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn at_path_completion_handles_subdirectory_paths() {
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("crates");
        std::fs::create_dir(&sub).unwrap();
        std::fs::write(sub.join("foo.rs"), "f").unwrap();
        std::fs::write(sub.join("bar.rs"), "b").unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let line = "@crates/";
        let (_start, pairs) = at_path_completion(line, line.len()).expect("fired");
        let replacements: Vec<&str> = pairs.iter().map(|p| p.replacement.as_str()).collect();
        assert!(replacements.contains(&"crates/bar.rs"));
        assert!(replacements.contains(&"crates/foo.rs"));

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn at_path_completion_skips_when_at_inside_word() {
        // `someone@`: char before `@` is `e`, not on the boundary
        // allow-list → completer must not fire.
        let line = "someone@";
        assert!(at_path_completion(line, line.len()).is_none());
    }

    // ── T1.3 fuzzy `@path` + inline palette hint ───────────────────

    #[test]
    fn is_subsequence_recognises_chars_in_order_with_gaps() {
        assert!(is_subsequence("mdt", "models/test.rs"));
        assert!(is_subsequence("rst", "robust"));
        assert!(is_subsequence("", "anything"));
        assert!(!is_subsequence("xyz", "abcdef"));
        assert!(!is_subsequence("tsr", "robust"), "order matters");
    }

    #[test]
    fn at_path_completion_substring_match_finds_inner_token() {
        // `modelcfg` doesn't prefix-match `agent_modelcfg.rs`, but it
        // is a clean substring → score 1 → should still surface.
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("agent_modelcfg.rs"), "x").unwrap();
        std::fs::write(dir.path().join("unrelated.rs"), "x").unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let line = "@modelcfg";
        let (_start, pairs) = at_path_completion(line, line.len()).expect("fired");
        let displays: Vec<&str> = pairs.iter().map(|p| p.replacement.as_str()).collect();
        assert!(
            displays.contains(&"agent_modelcfg.rs"),
            "expected substring match to surface agent_modelcfg.rs: {displays:?}"
        );

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn at_path_completion_subsequence_match_finds_scattered_chars() {
        // `modtst` → m..o..d..t..s..t with gaps in `models_test.rs`.
        // Score 2 (subsequence) — the lowest band, but still a match.
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("models_test.rs"), "x").unwrap();
        std::fs::write(dir.path().join("completely_unrelated.rs"), "x").unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let line = "@modtst";
        let (_start, pairs) = at_path_completion(line, line.len()).expect("fired");
        let displays: Vec<&str> = pairs.iter().map(|p| p.replacement.as_str()).collect();
        assert!(
            displays.contains(&"models_test.rs"),
            "expected subsequence match: {displays:?}"
        );
        assert!(
            !displays.contains(&"completely_unrelated.rs"),
            "non-match should NOT surface: {displays:?}"
        );

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn at_path_completion_ranks_prefix_above_substring_above_subsequence() {
        // For the prefix `te`:
        //   * `test.rs`      → prefix (score 0)
        //   * `latest.rs`    → substring (score 1, contains "te")
        //   * `tale.rs`      → subsequence (score 2, t,a,l,e contains t..e)
        // All three should appear, in that score order.
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("test.rs"), "x").unwrap();
        std::fs::write(dir.path().join("latest.rs"), "x").unwrap();
        std::fs::write(dir.path().join("tale.rs"), "x").unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let line = "@te";
        let (_start, pairs) = at_path_completion(line, line.len()).expect("fired");
        let order: Vec<&str> = pairs.iter().map(|p| p.replacement.as_str()).collect();
        // test.rs (prefix) must come before latest.rs (substring),
        // which must come before tale.rs (subsequence).
        let test_idx = order.iter().position(|d| *d == "test.rs");
        let latest_idx = order.iter().position(|d| *d == "latest.rs");
        let tale_idx = order.iter().position(|d| *d == "tale.rs");
        assert!(
            test_idx.is_some() && latest_idx.is_some() && tale_idx.is_some(),
            "missing entries: {order:?}"
        );
        assert!(
            test_idx < latest_idx,
            "prefix should rank above substring: {order:?}"
        );
        assert!(
            latest_idx < tale_idx,
            "substring should rank above subsequence: {order:?}"
        );

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn fs_candidate_sort_orders_by_score_then_mtime_then_alpha() {
        // Pure test of the sort comparator — no filesystem mtime
        // surgery required. Construct candidates with controlled
        // (score, mtime, sort_key) tuples and verify they end up in
        // the expected order after `sort_by`.
        let mut items = [
            FsCandidate {
                replacement: "zebra.rs".into(),
                display: "zebra.rs".into(),
                score: 1,
                mtime_secs: 9000,
                sort_key: "zebra.rs".into(),
            },
            FsCandidate {
                replacement: "alpha_old.rs".into(),
                display: "alpha_old.rs".into(),
                score: 0,
                mtime_secs: 1000,
                sort_key: "alpha_old.rs".into(),
            },
            FsCandidate {
                replacement: "alpha_new.rs".into(),
                display: "alpha_new.rs".into(),
                score: 0,
                mtime_secs: 9000,
                sort_key: "alpha_new.rs".into(),
            },
            FsCandidate {
                replacement: "subseq.rs".into(),
                display: "subseq.rs".into(),
                score: 2,
                mtime_secs: 9000,
                sort_key: "subseq.rs".into(),
            },
        ];
        items.sort_by(|a, b| {
            a.score
                .cmp(&b.score)
                .then_with(|| b.mtime_secs.cmp(&a.mtime_secs))
                .then_with(|| a.sort_key.cmp(&b.sort_key))
        });
        let order: Vec<&str> = items.iter().map(|c| c.display.as_str()).collect();
        // Expected:
        //   score 0 / mtime 9000 → alpha_new.rs
        //   score 0 / mtime 1000 → alpha_old.rs
        //   score 1 / mtime 9000 → zebra.rs
        //   score 2 / mtime 9000 → subseq.rs
        assert_eq!(
            order,
            vec!["alpha_new.rs", "alpha_old.rs", "zebra.rs", "subseq.rs"]
        );
    }

    #[test]
    fn at_path_hint_renders_palette_for_multiple_matches() {
        // When typing `@a` with multiple files matching, the inline
        // hint should preview them as a dim palette (T1.3 mirrors
        // T1.2's slash-command palette).
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("alpha.txt"), "x").unwrap();
        std::fs::write(dir.path().join("aria.txt"), "x").unwrap();
        std::fs::write(dir.path().join("apex.txt"), "x").unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let line = "tell me about @a";
        let hint = compute_at_path_hint(line, line.len()).expect("palette hint expected");
        // Mentions at least two of the matching files in the inline
        // palette format with `(Tab)` teach-text.
        assert!(hint.contains("(Tab)"), "missing teach-text: {hint}");
        let mentioned = ["alpha.txt", "aria.txt", "apex.txt"]
            .iter()
            .filter(|n| hint.contains(*n))
            .count();
        assert!(
            mentioned >= 2,
            "expected ≥2 of {{alpha/aria/apex}}.txt in hint: {hint}"
        );

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn at_path_hint_silent_for_single_match() {
        // Single candidate: Tab autocompletes immediately, so the
        // inline palette would just be noise.
        let _guard = SERIAL_CWD.get_or_init(|| Mutex::new(())).lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("only_one.txt"), "x").unwrap();
        let prev = std::env::current_dir().ok();
        std::env::set_current_dir(dir.path()).unwrap();

        let line = "look @on";
        assert!(
            compute_at_path_hint(line, line.len()).is_none(),
            "single match should not produce inline palette"
        );

        if let Some(prev) = prev {
            let _ = std::env::set_current_dir(prev);
        }
    }

    #[test]
    fn at_path_hint_silent_when_not_in_at_token() {
        // No `@` under cursor → must defer back to slash-command path
        // by returning None.
        assert!(compute_at_path_hint("plain text", 10).is_none());
        // `@` inside a word (e.g. email) → boundary check fails.
        assert!(compute_at_path_hint("alice@example", 13).is_none());
    }

    // Serial guard for tests that mutate process-wide cwd. `OnceLock`
    // + `Mutex<()>` gives us cross-test exclusion without a
    // `lazy_static` dep.
    static SERIAL_CWD: OnceLock<Mutex<()>> = OnceLock::new();

    // ── LiveToolArgsTicker lifecycle ────────────────────────────────
    //
    // The ticker thread itself is hard to unit test (it animates
    // against `io::stdout()` and times its own sleep). Instead we
    // exercise the `drain_live_tool_args` helper and the public
    // structural fields it depends on, since that's the only public
    // surface other code interacts with.

    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[test]
    fn drain_live_tool_args_is_noop_when_slot_is_empty() {
        let slot: Mutex<Option<LiveToolArgsTicker>> = Mutex::new(None);
        drain_live_tool_args(&slot);
        assert!(slot.lock().unwrap().is_none(), "slot must remain empty");
    }

    #[test]
    fn drain_live_tool_args_stops_and_joins_ticker_thread() {
        // Spawn a tiny stub thread that just polls the stop flag.
        // Standing in for `run_tool_args_ticker` keeps the test out
        // of stdout / lock territory while still exercising the
        // shutdown handshake the real path uses.
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            while !stop_for_thread.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        });

        let slot: Mutex<Option<LiveToolArgsTicker>> = Mutex::new(Some(LiveToolArgsTicker {
            stop: Arc::clone(&stop),
            handle: Some(handle),
            block_index: 7,
        }));

        drain_live_tool_args(&slot);

        assert!(
            stop.load(Ordering::Relaxed),
            "drain must flip the stop flag so the worker can exit"
        );
        assert!(
            slot.lock().unwrap().is_none(),
            "drain must clear the slot after joining"
        );
    }

    #[test]
    fn drain_live_tool_args_drops_handle_so_slot_is_safe_to_refill() {
        // After drain the slot must be `None`, not `Some(_)` with
        // an already-joined handle — otherwise the next
        // `ToolUseStart` would replace a ghost ticker and double
        // up on stop signals.
        let stop = Arc::new(AtomicBool::new(false));
        let stop_for_thread = Arc::clone(&stop);
        let handle = std::thread::spawn(move || {
            while !stop_for_thread.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        let slot: Mutex<Option<LiveToolArgsTicker>> = Mutex::new(Some(LiveToolArgsTicker {
            stop,
            handle: Some(handle),
            block_index: 0,
        }));

        drain_live_tool_args(&slot);
        // Re-arm to confirm the slot is genuinely re-usable.
        let stop2 = Arc::new(AtomicBool::new(false));
        let stop2_for_thread = Arc::clone(&stop2);
        let handle2 = std::thread::spawn(move || {
            while !stop2_for_thread.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        });
        {
            let mut guard = slot.lock().unwrap();
            *guard = Some(LiveToolArgsTicker {
                stop: stop2,
                handle: Some(handle2),
                block_index: 42,
            });
        }
        drain_live_tool_args(&slot);
        assert!(slot.lock().unwrap().is_none());
    }

    #[test]
    fn render_tool_start_line_uses_cyan_marker_and_dim_label_with_color() {
        let line = crate::render_tool_start_line("Bash(chrome --headless …)", true);
        // Cyan ▶ marker, then the dimmed preview.
        assert_eq!(
            line,
            "\x1b[36m▶\x1b[0m \x1b[2mBash(chrome --headless …)\x1b[0m"
        );
    }

    #[test]
    fn render_tool_start_line_degrades_to_plain_marker_without_color() {
        let line = crate::render_tool_start_line("Read(src/main.rs)", false);
        assert_eq!(line, "▶ Read(src/main.rs)");
        // No ANSI escapes when color is off.
        assert_eq!(strip_ansi(&line), line);
    }
}
