use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use deeptide_core::{
    ClearCommand, CommandContext, CommandResult, CompactCommand, NewCommand, SlashCommand,
};

#[test]
fn clear_command_uses_clear_hook_when_provided() {
    let reset_called = Arc::new(AtomicBool::new(false));
    let clear_called = Arc::new(AtomicBool::new(false));

    let context = CommandContext::builder()
        .reset_conversation({
            let reset_called = Arc::clone(&reset_called);
            move || {
                reset_called.store(true, Ordering::SeqCst);
            }
        })
        .clear_conversation({
            let clear_called = Arc::clone(&clear_called);
            move || {
                clear_called.store(true, Ordering::SeqCst);
                Some(String::from("Started a new local cache branch."))
            }
        })
        .build();

    let output = text(ClearCommand.execute("", &context));

    assert!(clear_called.load(Ordering::SeqCst));
    assert!(!reset_called.load(Ordering::SeqCst));
    assert!(output.contains("Conversation cleared."));
    assert!(output.contains("Started a new local cache branch."));
}

#[test]
fn clear_command_falls_back_to_reset_conversation() {
    let reset_called = Arc::new(AtomicBool::new(false));
    let context = CommandContext::builder()
        .reset_conversation({
            let reset_called = Arc::clone(&reset_called);
            move || {
                reset_called.store(true, Ordering::SeqCst);
            }
        })
        .build();

    let output = text(ClearCommand.execute("", &context));

    assert!(reset_called.load(Ordering::SeqCst));
    assert!(output.contains("Conversation cleared."));
}

#[test]
fn local_clear_requires_explicit_confirmation() {
    let clear_called = Arc::new(AtomicBool::new(false));
    let context = CommandContext::builder()
        .clear_conversation({
            let clear_called = Arc::clone(&clear_called);
            move || {
                clear_called.store(true, Ordering::SeqCst);
                Some(String::from("Started a new local cache branch."))
            }
        })
        .is_local_mode(|| true)
        .local_warmup_estimate(|| Some(String::from("12.3s from recent local cache records")))
        .build();

    let output = text(ClearCommand.execute("", &context));

    assert!(!clear_called.load(Ordering::SeqCst));
    assert!(output.contains("requires confirmation"));
    assert!(output.contains("/clear --yes"));
    assert!(output.contains("12.3s"));
}

#[test]
fn local_clear_yes_bypasses_confirmation() {
    let clear_called = Arc::new(AtomicBool::new(false));
    let context = CommandContext::builder()
        .clear_conversation({
            let clear_called = Arc::clone(&clear_called);
            move || {
                clear_called.store(true, Ordering::SeqCst);
                Some(String::new())
            }
        })
        .is_local_mode(|| true)
        .build();

    let _ = ClearCommand.execute("--yes", &context);

    assert!(clear_called.load(Ordering::SeqCst));
}

#[test]
fn new_command_reuses_clear_hook_and_primer() {
    let context = CommandContext::builder()
        .clear_conversation(|| Some(String::from("Started a new local cache branch.")))
        .prime_local_cache_branch(|| Some(String::from(" Prompt prefix primed.")))
        .build();

    let output = text(NewCommand.execute("", &context));

    assert!(output.contains("New conversation started."));
    assert!(output.contains("Started a new local cache branch."));
    assert!(output.contains("Prompt prefix primed."));
}

#[test]
fn compact_command_requires_confirmation_in_local_mode() {
    let compact_called = Arc::new(AtomicBool::new(false));
    let context = CommandContext::builder()
        .compact_conversation({
            let compact_called = Arc::clone(&compact_called);
            move || {
                compact_called.store(true, Ordering::SeqCst);
            }
        })
        .is_local_mode(|| true)
        .context_tokens(|| 512)
        .build();

    let output = text(CompactCommand.execute("", &context));

    assert!(!compact_called.load(Ordering::SeqCst));
    assert!(output.contains("requires confirmation"));
    assert!(output.contains("refill the current prompt prefix"));
}

#[test]
fn compact_command_runs_when_confirmed() {
    let compact_called = Arc::new(AtomicBool::new(false));
    let context = CommandContext::builder()
        .compact_conversation({
            let compact_called = Arc::clone(&compact_called);
            move || {
                compact_called.store(true, Ordering::SeqCst);
            }
        })
        .is_local_mode(|| true)
        .build();

    let output = text(CompactCommand.execute("-y", &context));

    assert!(compact_called.load(Ordering::SeqCst));
    assert_eq!(output, "Context compacted.");
}

fn text(result: CommandResult) -> String {
    match result {
        CommandResult::Text(value) => value,
        other => panic!("expected text command result, got {other:?}"),
    }
}
