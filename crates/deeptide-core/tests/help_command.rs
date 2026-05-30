use deeptide_core::{
    CommandCompletionSource, CommandContext, CommandResult, HelpCommand, SlashCommand,
};

fn commands() -> Vec<CommandCompletionSource> {
    vec![
        CommandCompletionSource::new(
            "help",
            ["h", "?"],
            "Show available commands and keybindings",
            "/help [command]",
        ),
        CommandCompletionSource::new("exit", ["quit", "q"], "Exit the REPL", "/exit"),
        CommandCompletionSource::new("new", Vec::<&str>::new(), "Start a new branch", "/new"),
        CommandCompletionSource::new(
            "clear",
            ["cls"],
            "Clear conversation and reset context",
            "/clear [--yes]",
        ),
        CommandCompletionSource::new("model", ["m"], "Switch model", "/model <name>"),
        CommandCompletionSource::new("memory", Vec::<&str>::new(), "Manage memory", "/memory"),
        CommandCompletionSource::new("commit", Vec::<&str>::new(), "Commit changes", "/commit"),
    ]
}

#[test]
fn help_command_lists_commands_by_swift_category_order() {
    let context = CommandContext::builder().all_commands(commands).build();
    let output = text(HelpCommand.execute("", &context));

    assert!(output.contains("Deeptide commands (7):"));
    assert!(output.contains("Core"));
    assert!(output.contains("/help, /h, /?"));
    assert!(
        output.contains("/exit, /quit, /q"),
        "Core section must surface /exit so users discover how to leave the REPL, got:\n{output}"
    );
    assert!(
        output.contains("/new"),
        "/new must appear in Core (not exiled to Other), got:\n{output}"
    );
    assert!(output.contains("/clear, /cls"));
    assert!(output.contains("Model"));
    assert!(output.contains("/model, /m"));
    assert!(output.contains("Memory"));
    assert!(output.contains("/memory"));
    assert!(output.contains("Git"));
    assert!(output.contains("/commit"));
    assert!(output.contains("Keybindings: Enter=submit"));
    assert!(output.contains("Type /help <command> for details"));

    // /new used to fall through to the "Other" section because it wasn't
    // listed in help_categories(). Guard against that regression.
    if let Some(other_idx) = output.find("\nOther\n") {
        let other_block = &output[other_idx..];
        assert!(
            !other_block.contains("/new"),
            "/new must not appear in the 'Other' section, got:\n{output}"
        );
    }
}

#[test]
fn help_command_renders_detail_for_name() {
    let context = CommandContext::builder().all_commands(commands).build();
    let output = text(HelpCommand.execute("model", &context));

    assert!(output.contains("/model \u{00b7} /m"));
    assert!(output.contains("Switch model"));
    assert!(output.contains("Usage:   /model <name>"));
    assert!(output.contains("Aliases: /m"));
}

#[test]
fn help_command_renders_detail_for_alias_with_leading_slash() {
    let context = CommandContext::builder().all_commands(commands).build();
    let output = text(HelpCommand.execute("/cls", &context));

    assert!(output.contains("/clear \u{00b7} /cls"));
    assert!(output.contains("Clear conversation and reset context"));
}

#[test]
fn help_command_suggests_similar_unknown_commands() {
    let context = CommandContext::builder().all_commands(commands).build();
    let output = text(HelpCommand.execute("me", &context));

    assert!(output.contains("Unknown command: /me"));
    assert!(output.contains("Did you mean: /memory?"));
}

#[test]
fn help_command_suggests_typo_via_edit_distance() {
    let context = CommandContext::builder().all_commands(commands).build();
    // "commti" neither starts with nor contains any command, but is one
    // transposition away from "commit" — the fuzzy fallback should catch it.
    let output = text(HelpCommand.execute("commti", &context));

    assert!(output.contains("Unknown command: /commti"));
    assert!(
        output.contains("Did you mean: /commit?"),
        "expected /commit suggestion, got: {output}"
    );
}

#[test]
fn help_command_reports_no_match_for_unrelated_command() {
    let context = CommandContext::builder().all_commands(commands).build();
    let output = text(HelpCommand.execute("zzzzzz", &context));

    assert!(output.contains("Unknown command: /zzzzzz"));
    assert!(output.contains("Type /help for the full list."));
    assert!(!output.contains("Did you mean"));
}

fn text(result: CommandResult) -> String {
    match result {
        CommandResult::Text(value) => value,
        other => panic!("expected text command result, got {other:?}"),
    }
}
