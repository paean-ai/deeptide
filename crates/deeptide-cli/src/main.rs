use std::io::{self, Read, Write};
use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};
use deeptide_core::embedded_protocol::{EmbeddedProtocol, EmbeddedProtocolSpec};
use deeptide_core::permissions::PermissionMode;
use deeptide_core::{
    AgentBackend, AgentLoop, AgentLoopEvent, AgentTerminalEvent, AnthropicBackend, AnthropicConfig,
    LocalEchoBackend, MarkdownRenderer, ReplEvent, ReplSession,
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

    #[arg(long, env = "DEEPTIDE_MODEL", default_value = "deepseek-v4-pro")]
    model: String,

    #[arg(
        long,
        env = "DEEPTIDE_BASE_URL",
        default_value = "https://api.anthropic.com"
    )]
    base_url: String,

    #[arg(long, env = "DEEPTIDE_API_KEY", hide_env_values = true)]
    api_key: Option<String>,

    #[arg(long, default_value_t = 4096)]
    max_output_tokens: usize,
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
    let (backend, model, is_configured) = configured_backend(cli)?;
    let mut repl = ReplSession::new(backend)
        .with_model(model)
        .with_permission_mode(permission_mode);

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
                    writeln!(stdout, "{}", MarkdownRenderer::render(&text))
                        .map_err(|error| error.to_string())?;
                }
                ReplEvent::Exit => return Ok(()),
            }
        }
    }
}

fn emit_output(cli: &Cli, prompt: &str, permission_mode: PermissionMode) -> Result<(), String> {
    let response = run_prompt(cli, prompt, permission_mode)?;

    match cli.output_format {
        OutputFormat::Text => {
            println!("{}", MarkdownRenderer::render(&response));
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
                }))
                .map_err(|error| error.to_string())?
            );
        }
    }

    Ok(())
}

fn run_prompt(cli: &Cli, prompt: &str, permission_mode: PermissionMode) -> Result<String, String> {
    let (backend, model, _) = configured_backend(cli)?;
    let mut loop_ = AgentLoop::new(backend)
        .with_model(model)
        .with_permission_mode(permission_mode);

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

fn configured_backend(cli: &Cli) -> Result<(Box<dyn AgentBackend>, String, bool), String> {
    let api_key = cli
        .api_key
        .clone()
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok());
    let Some(api_key) = api_key.filter(|key| !key.trim().is_empty()) else {
        return Ok((
            Box::<LocalEchoBackend>::default(),
            String::from("unconfigured"),
            false,
        ));
    };

    let base_url = std::env::var("ANTHROPIC_BASE_URL").unwrap_or_else(|_| cli.base_url.clone());
    let mut config = AnthropicConfig::new(base_url, api_key, cli.model.clone());
    config.max_tokens = cli.max_output_tokens;
    let backend = AnthropicBackend::new(config)?;
    Ok((Box::new(backend), cli.model.clone(), true))
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
        Cli, InputFormat, OutputFormat, collect_prompt, normalize_embedded_mode, validate_formats,
    };

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
}
