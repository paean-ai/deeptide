use deeptide_core::{CommandCompletionSource, CompletionEngine, NewCommand, SlashCommand};

fn commands() -> Vec<CommandCompletionSource> {
    vec![
        CommandCompletionSource::new("help", ["h", "?"], "Show help", "/help [command]"),
        CommandCompletionSource::new("history", Vec::<&str>::new(), "Show history", "/history"),
        CommandCompletionSource::new("model", ["m"], "Switch model", "/model <name>"),
        CommandCompletionSource::new("memory", ["remember"], "Manage memory", "/memory"),
        CommandCompletionSource::new("commit", Vec::<&str>::new(), "Commit changes", "/commit"),
    ]
}

#[test]
fn command_completions_rank_prefix_before_substring() {
    let result = must_complete("/m", 2);
    let names: Vec<&str> = result
        .candidates
        .iter()
        .take(2)
        .map(|candidate| candidate.name.as_str())
        .collect();

    assert_eq!(names, ["model", "memory"]);
}

#[test]
fn command_completions_match_aliases() {
    let result = must_complete("/rem", 4);
    let first = match result.candidates.first() {
        Some(candidate) => candidate,
        None => panic!("expected first completion candidate"),
    };

    assert_eq!(first.name, "memory");
    assert_eq!(first.matched_text, "remember");
}

#[test]
fn command_completion_only_applies_to_leading_slash_token() {
    assert!(CompletionEngine::command_completions("ask /he", 7, &commands(), 8).is_none());
    assert!(CompletionEngine::command_completions("@he", 3, &commands(), 8).is_none());
}

#[test]
fn replacing_token_adds_trailing_space_for_command_arguments() {
    let result = must_complete("/he", 3);
    let accepted = CompletionEngine::replacing_token("/he", &result, &result.candidates[0]);

    assert_eq!(accepted.text, "/help ");
    assert_eq!(accepted.cursor, 6);
}

#[test]
fn replacing_token_preserves_existing_suffix() {
    let result = must_complete("/he argument", 3);
    let accepted =
        CompletionEngine::replacing_token("/he argument", &result, &result.candidates[0]);

    assert_eq!(accepted.text, "/help argument");
    assert_eq!(accepted.cursor, 5);
}

#[test]
fn command_completion_source_can_be_built_from_slash_command() {
    let source = CommandCompletionSource::from_command(&NewCommand);

    assert_eq!(source.name, NewCommand.name());
    assert_eq!(source.description, NewCommand.description());
    assert_eq!(source.usage, NewCommand.usage());
}

const MODELS: &[&str] = &["flash", "pro", "deepseek-v4-pro", "deepseek-v4-flash"];

#[test]
fn value_completions_complete_model_argument_prefix_first() {
    let result = match CompletionEngine::value_completions("/model fl", 9, "model", MODELS, 8) {
        Some(result) => result,
        None => panic!("expected model-argument completions"),
    };
    // Prefix match ("flash") ranks ahead of the substring match
    // ("deepseek-v4-flash"); both are offered.
    assert_eq!(result.candidates.first().map(String::as_str), Some("flash"));
    assert!(result.candidates.iter().any(|c| c == "deepseek-v4-flash"));
}

#[test]
fn value_completions_empty_argument_lists_all_models() {
    let result = match CompletionEngine::value_completions("/model ", 7, "model", MODELS, 8) {
        Some(result) => result,
        None => panic!("expected all models for an empty argument"),
    };
    assert_eq!(result.candidates.len(), MODELS.len());
}

#[test]
fn value_completions_only_apply_to_the_named_command() {
    // Wrong command, a bare word, and an unmatched value all decline.
    assert!(CompletionEngine::value_completions("/help fl", 8, "model", MODELS, 8).is_none());
    assert!(CompletionEngine::value_completions("fl", 2, "model", MODELS, 8).is_none());
    assert!(CompletionEngine::value_completions("/model zzz", 10, "model", MODELS, 8).is_none());
}

#[test]
fn replacing_value_substitutes_the_argument_token() {
    let result = CompletionEngine::value_completions("/model fl", 9, "model", MODELS, 8)
        .expect("model completions");
    let accepted = CompletionEngine::replacing_value("/model fl", &result, "flash");

    assert_eq!(accepted.text, "/model flash");
    assert_eq!(accepted.cursor, 12);
}

fn must_complete(input: &str, cursor: usize) -> deeptide_core::CommandCompletionResult {
    match CompletionEngine::command_completions(input, cursor, &commands(), 8) {
        Some(result) => result,
        None => panic!("expected command completions for {input:?} at cursor {cursor}"),
    }
}
