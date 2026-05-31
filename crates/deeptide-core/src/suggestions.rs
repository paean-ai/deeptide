//! Follow-up prompt suggestions, emitted when a task finishes.
//!
//! After an agent turn completes, the REPL offers a few next-step prompts the
//! user can run by number (à la Claude Code / Cursor's suggested actions). The
//! engine is **deterministic and free** — it reads signals from the turn that
//! just finished (which tools ran, whether anything errored, the active TODO,
//! the assistant's closing text) rather than spending another model call. That
//! keeps every turn cheap and the behaviour unit-testable offline.
//!
//! It is deliberately conservative: it returns nothing when there's no clear
//! next step, and stays silent when the assistant is the one waiting on the
//! user (a trailing question), so suggestions never become noise.

/// Signals gathered from the just-finished turn. The REPL fills `edited_files`,
/// `had_tool_error`, `bash_commands`, `next_todo`, and `assistant_text`; the
/// engine derives the rest (test/build/commit classification) so that logic
/// stays here and testable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TurnSignals {
    /// An Edit/Write/NotebookEdit/MultiEdit tool ran this turn.
    pub edited_files: bool,
    /// A tool result came back as an error.
    pub had_tool_error: bool,
    /// Shell commands the agent ran this turn (Bash tool inputs).
    pub bash_commands: Vec<String>,
    /// The next actionable TODO (in-progress, else first pending), if any.
    pub next_todo: Option<String>,
    /// The assistant's final natural-language text for the turn.
    pub assistant_text: String,
}

/// A single follow-up the user can run. `prompt` is what gets submitted if the
/// user picks it; `label` is the short display text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestion {
    pub label: String,
    pub prompt: String,
}

impl Suggestion {
    fn new(label: &str, prompt: &str) -> Self {
        Self {
            label: label.to_owned(),
            prompt: prompt.to_owned(),
        }
    }
}

/// Cap so the block stays a glanceable list, not a menu.
pub const MAX_SUGGESTIONS: usize = 3;

fn lc_any(cmd: &str, needles: &[&str]) -> bool {
    let lc = cmd.to_ascii_lowercase();
    needles.iter().any(|n| lc.contains(n))
}

fn is_test_cmd(cmd: &str) -> bool {
    lc_any(
        cmd,
        &[
            "test", "pytest", "jest", "vitest", "go test", "cargo test", "make test",
        ],
    )
}

fn is_build_cmd(cmd: &str) -> bool {
    lc_any(
        cmd,
        &["build", "cargo build", "make ", "tsc", "compile", "cargo check"],
    )
}

fn is_git_commit(cmd: &str) -> bool {
    lc_any(cmd, &["git commit"])
}

/// True when the assistant's closing text is itself a question — the ball is in
/// the user's court, so we don't pile suggestions on top of it.
fn assistant_is_asking(text: &str) -> bool {
    let last = text
        .lines()
        .map(str::trim)
        .rfind(|l| !l.is_empty())
        .unwrap_or("");
    last.ends_with('?')
}

/// Produce up to [`MAX_SUGGESTIONS`] follow-ups, highest-value first. Returns
/// empty when there's no clear next step or the assistant is awaiting an answer.
pub fn suggest(signals: &TurnSignals) -> Vec<Suggestion> {
    if assistant_is_asking(&signals.assistant_text) {
        return Vec::new();
    }

    let ran_tests = signals.bash_commands.iter().any(|c| is_test_cmd(c));
    let ran_build = signals.bash_commands.iter().any(|c| is_build_cmd(c));
    let committed = signals.bash_commands.iter().any(|c| is_git_commit(c));

    let mut out: Vec<Suggestion> = Vec::new();
    let mut push = |s: Suggestion| {
        if out.len() < MAX_SUGGESTIONS && !out.iter().any(|e| e.prompt == s.prompt) {
            out.push(s);
        }
    };

    // 1) Continue an explicit plan — the strongest "what's next" signal.
    if let Some(todo) = &signals.next_todo {
        let todo = todo.trim();
        if !todo.is_empty() {
            push(Suggestion::new(
                &format!("Continue: {}", truncate_label(todo, 60)),
                &format!("Continue with the next task: {todo}"),
            ));
        }
    }

    // 2) Something errored — fixing it almost always comes next.
    if signals.had_tool_error {
        push(Suggestion::new(
            "Fix the error from the last step",
            "Investigate and fix the error from the previous step.",
        ));
    }

    // 3) Code changed but wasn't verified — verify it.
    if signals.edited_files && !ran_tests {
        let label = if ran_build {
            "Run the tests"
        } else {
            "Build and run the tests"
        };
        push(Suggestion::new(
            label,
            "Run the project's tests to verify the change works.",
        ));
    }

    // 4) Code changed and isn't committed — review, then commit.
    if signals.edited_files && !committed {
        push(Suggestion::new(
            "Review the diff",
            "Show me a summary of the diff of everything changed so far.",
        ));
        push(Suggestion::new(
            "Commit the changes",
            "Commit the changes with a clear, conventional commit message.",
        ));
    }

    out
}

fn truncate_label(text: &str, cap: usize) -> String {
    let one_line = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if one_line.chars().count() <= cap {
        return one_line;
    }
    let kept: String = one_line.chars().take(cap.saturating_sub(1)).collect();
    format!("{kept}…")
}

/// Render a suggestion list as a compact, numbered block for the REPL. Returns
/// `None` for an empty list so callers can skip emitting anything.
pub fn render_block(suggestions: &[Suggestion]) -> Option<String> {
    if suggestions.is_empty() {
        return None;
    }
    let mut lines = vec![String::from("Next steps:")];
    for (i, s) in suggestions.iter().enumerate() {
        lines.push(format!("  {}. {}", i + 1, s.label));
    }
    lines.push(format!(
        "  (type 1–{} to run · /suggest off to hide)",
        suggestions.len()
    ));
    Some(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals() -> TurnSignals {
        TurnSignals::default()
    }

    #[test]
    fn edited_without_tests_suggests_verify_and_review() {
        let s = TurnSignals {
            edited_files: true,
            ..signals()
        };
        let out = suggest(&s);
        let labels: Vec<&str> = out.iter().map(|s| s.label.as_str()).collect();
        assert!(out.iter().any(|x| x.label.contains("test")), "{labels:?}");
        assert!(out.iter().any(|x| x.label.contains("diff")), "{labels:?}");
        assert!(out.len() <= MAX_SUGGESTIONS);
    }

    #[test]
    fn ran_tests_suppresses_the_run_tests_suggestion() {
        let s = TurnSignals {
            edited_files: true,
            bash_commands: vec!["cargo test -p foo".into()],
            ..signals()
        };
        let out = suggest(&s);
        assert!(
            !out.iter().any(|x| x.label.to_lowercase().contains("run the tests")),
            "should not re-suggest tests that just ran: {out:?}"
        );
    }

    #[test]
    fn committed_changes_suppress_commit_suggestion() {
        let s = TurnSignals {
            edited_files: true,
            bash_commands: vec!["git commit -m x".into()],
            ..signals()
        };
        let out = suggest(&s);
        assert!(!out.iter().any(|x| x.label.contains("Commit")), "{out:?}");
    }

    #[test]
    fn pending_todo_leads_the_list() {
        let s = TurnSignals {
            edited_files: true,
            next_todo: Some("migrate the queue worker".into()),
            ..signals()
        };
        let out = suggest(&s);
        assert_eq!(out[0].prompt, "Continue with the next task: migrate the queue worker");
    }

    #[test]
    fn tool_error_suggests_a_fix() {
        let s = TurnSignals {
            had_tool_error: true,
            ..signals()
        };
        let out = suggest(&s);
        assert!(out.iter().any(|x| x.label.contains("Fix the error")), "{out:?}");
    }

    #[test]
    fn trailing_question_suppresses_all_suggestions() {
        let s = TurnSignals {
            edited_files: true,
            had_tool_error: true,
            assistant_text: "I changed the file. Which database should I target?".into(),
            ..signals()
        };
        assert!(
            suggest(&s).is_empty(),
            "no suggestions while the assistant is awaiting an answer"
        );
    }

    #[test]
    fn no_signal_yields_no_suggestions() {
        let s = TurnSignals {
            assistant_text: "Here is the explanation you asked for.".into(),
            ..signals()
        };
        assert!(suggest(&s).is_empty());
    }

    #[test]
    fn render_block_numbers_items_and_caps_pick_range() {
        let out = suggest(&TurnSignals {
            edited_files: true,
            ..signals()
        });
        let block = render_block(&out).expect("non-empty");
        assert!(block.contains("Next steps:"));
        assert!(block.contains("1. "));
        assert!(block.contains("type 1–"));
    }
}
