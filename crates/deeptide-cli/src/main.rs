use std::io::{self, Read, Write};
use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};
use deeptide_core::embedded_protocol::{EmbeddedProtocol, EmbeddedProtocolSpec};
use deeptide_core::permissions::PermissionMode;
use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentTerminalEvent, AnthropicBackend, AnthropicConfig,
    LocalEchoBackend, ReplEvent, ReplSession, tui,
};

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
        long = "no-prompt-cache",
        env = "DEEPTIDE_NO_PROMPT_CACHE",
        action = ArgAction::SetTrue,
        help = "Disable Anthropic prompt caching of system prompt and tool schemas."
    )]
    no_prompt_cache: bool,

    #[arg(
        long = "stream",
        env = "DEEPTIDE_STREAM",
        action = ArgAction::SetTrue,
        help = "Request streamed SSE responses from the Anthropic Messages API. Required by some proxy providers (openrouter, custom relays)."
    )]
    stream: bool,
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run(mut cli: Cli) -> Result<(), String> {
    normalize_embedded_mode(&mut cli);

    let Some(permission_mode) = PermissionMode::parse(&cli.permission_mode) else {
        return Err(format!("invalid permission mode: {}", cli.permission_mode));
    };

    if let Some(cwd) = cli.cwd.as_ref() {
        std::env::set_current_dir(cwd)
            .map_err(|error| format!("invalid --cwd {}: {error}", cwd.display()))?;
    }

    validate_formats(&cli)?;
    if !cli.print_mode && cli.input_format == InputFormat::Text {
        return run_interactive(&cli, permission_mode);
    }

    let stdin = read_stdin_if_needed(&cli)?;
    let prompt = collect_prompt(&cli, stdin.as_deref())?;
    emit_output(&cli, &prompt, permission_mode)
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

fn run_interactive(cli: &Cli, permission_mode: PermissionMode) -> Result<(), String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let configured = configured_backend(cli)?;
    let is_configured = configured.is_configured;
    let mut repl = ReplSession::new(configured.backend)
        .with_model(configured.model)
        .with_permission_mode(permission_mode)
        .with_max_turns(cli.max_turns)
        .with_subagent_backend_factory(subagent_backend_factory(configured.subagent_config));

    writeln!(stdout, "{}", repl.banner()).map_err(|error| error.to_string())?;
    writeln!(
        stdout,
        "Permission mode: {}. Type /help for commands, /exit to quit.",
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

    loop {
        writeln!(
            stdout,
            "{}",
            repl.status_line().render(terminal_width().unwrap_or(100))
        )
        .map_err(|error| error.to_string())?;
        write!(stdout, "{}", repl.prompt()).map_err(|error| error.to_string())?;
        stdout.flush().map_err(|error| error.to_string())?;

        let mut line = String::new();
        let bytes = stdin
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if bytes == 0 {
            writeln!(stdout).map_err(|error| error.to_string())?;
            return Ok(());
        }

        for event in repl.submit(&line) {
            match event {
                ReplEvent::Output(text) => {
                    writeln!(
                        stdout,
                        "{}",
                        tui::render_output_panel(
                            &text,
                            terminal_width().unwrap_or(100),
                            std::env::var_os("NO_COLOR").is_none(),
                        )
                    )
                    .map_err(|error| error.to_string())?;
                }
                ReplEvent::Exit => return Ok(()),
            }
        }
    }
}

fn terminal_width() -> Option<usize> {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width >= 20)
}

fn emit_output(cli: &Cli, prompt: &str, permission_mode: PermissionMode) -> Result<(), String> {
    let response = run_prompt(cli, prompt, permission_mode)?;

    match cli.output_format {
        OutputFormat::Text => {
            println!(
                "{}",
                tui::render_output_panel(
                    &response,
                    terminal_width().unwrap_or(100),
                    std::env::var_os("NO_COLOR").is_none(),
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

fn run_prompt(cli: &Cli, prompt: &str, permission_mode: PermissionMode) -> Result<String, String> {
    let configured = configured_backend(cli)?;
    let mut loop_ = AgentLoop::new(configured.backend)
        .with_model(configured.model)
        .with_permission_mode(permission_mode)
        .with_max_turns(cli.max_turns)
        .with_subagent_backend_factory(subagent_backend_factory(configured.subagent_config));

    let events = loop_.run(prompt);
    let mut assistant = None;
    for event in events {
        match event {
            AgentLoopEvent::Assistant(message) => assistant = Some(message.content),
            AgentLoopEvent::Terminal(AgentTerminalEvent::ModelError(error)) => return Err(error),
            AgentLoopEvent::Terminal(AgentTerminalEvent::MaxTurnsReached) => {
                return Err(String::from("maximum turns reached"));
            }
            AgentLoopEvent::User(_)
            | AgentLoopEvent::ToolBatchSummary { .. }
            | AgentLoopEvent::ToolResult { .. }
            | AgentLoopEvent::Terminal(AgentTerminalEvent::Complete) => {}
        }
    }

    assistant.ok_or_else(|| String::from("model returned no assistant message"))
}

fn configured_backend(cli: &Cli) -> Result<ConfiguredBackend, String> {
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
    config.enable_streaming = cli.stream;
    if let Some(system_prompt) = resolve_system_prompt(cli)? {
        config = config.with_system_prompt(system_prompt);
    }
    let backend = AnthropicBackend::new(config.clone())?;
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
        Cli, DEFAULT_BASE_URL, DEFAULT_MODEL, InputFormat, OutputFormat, collect_prompt,
        configured_backend, effective_base_url, effective_model, normalize_embedded_mode,
        validate_formats,
    };
    use clap::Parser;
    use deeptide_core::AnthropicAuthMode;
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
            max_output_tokens: 4096,
            max_turns: 25,
            system_prompt: None,
            system_prompt_file: None,
            no_prompt_cache: false,
            stream: false,
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
