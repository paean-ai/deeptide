use clap::Parser;
use deeptide_core::permissions::PermissionMode;

#[derive(Debug, Parser)]
#[command(
    name = "deeptide",
    version,
    about = "Cross-platform Rust implementation of the Deeptide CLI.",
    long_about = "Cross-platform Rust implementation of the Deeptide CLI.\n\nThis workspace is under active parity development against the Swift Deeptide app. The current Rust increment establishes the slash-command core and tests for /clear, /new, and /compact."
)]
struct Cli {
    #[arg(long, value_name = "TEXT")]
    prompt: Option<String>,

    #[arg(
        long,
        default_value = "default",
        help = "Permission mode: default, accept-edits, plan, bypass."
    )]
    permission_mode: String,
}

fn main() {
    let cli = Cli::parse();
    let Some(permission_mode) = PermissionMode::parse(&cli.permission_mode) else {
        eprintln!("invalid permission mode: {}", cli.permission_mode);
        std::process::exit(2);
    };

    if cli.prompt.is_some() {
        eprintln!(
            "Prompt execution is not wired up yet in the Rust port. Current parity slice covers internal slash-command behavior for /clear, /new, and /compact."
        );
        std::process::exit(2);
    }

    println!(
        "deeptide-rs is under active development.\nCurrent parity slice: slash-command core plus /clear, /new, and /compact tests."
    );
    println!("permission mode: {}", permission_mode.label());
}
