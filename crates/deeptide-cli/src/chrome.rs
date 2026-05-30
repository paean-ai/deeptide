//! Conversation "chrome" — the small visual elements (role
//! prefixes, separators, headers, the welcome banner, the
//! during-streaming ghost text) that give the REPL its Codex /
//! Claude-Code-style polish.
//!
//! Every function in this module is pure: it takes the data it
//! needs plus a `use_color` toggle and returns a `String`. That
//! makes the renderers trivially unit-testable and means the
//! caller (the CLI main loop, the streaming handler) just `write_all`s
//! the output.  Color sequences degrade to plain text whenever
//! `use_color = false`, matching the workspace convention used by
//! [`crate::status_bar::dim`] / [`crate::status_bar::colorize`].
//!
//! ## Palette
//!
//! Roles are color-coded for scannability, with all picks chosen to
//! stay legible on both light and dark terminal themes:
//!
//! | role        | glyph | SGR        | rationale                                                         |
//! | ----------- | ----- | ---------- | ----------------------------------------------------------------- |
//! | `you`       | `▎`   | `1;33`     | warm yellow — matches Claude Code / Codex user lines              |
//! | `deeptide`  | `▎`   | `1;36`     | bright cyan — same hue as the input prompt prefix                 |
//! | `system`    | `▎`   | `1;35`     | magenta — distinguishes meta from real conversation               |
//! | `hint`      | n/a   | `2;3` dim+ | dim italic for inline tips that should recede visually            |
//! | `separator` | `─`   | `2` dim    | faint horizontal rule between turns                               |
//!
//! The `▎` glyph (U+258E LEFT ONE QUARTER BLOCK) is the same one
//! Claude Code uses; it reads as a "speaker bar" rather than the
//! generic vertical bar `│` that we already use for tool-call body
//! indentation. Keeping the two glyphs distinct lets the eye tell
//! "who said this" apart from "what did the tool print".

/// Common glyph for the role-prefix bar. Centralised so a future
/// theme swap (e.g. emoji bullets, or plain ASCII for restrictive
/// terminals) only touches one constant.
pub const ROLE_BAR: &str = "▎";

/// Faint rule character used by [`render_separator`].
const SEPARATOR_GLYPH: char = '─';

// ─── Role SGR codes ────────────────────────────────────────────────
//
// We keep these as `&'static str` rather than enum variants because
// (a) all consumers want the raw SGR for inline formatting, and
// (b) it makes the unit tests trivial — assert on the literal code.

const SGR_YOU: &str = "1;33";
const SGR_ASSISTANT: &str = "1;36";
const SGR_SYSTEM: &str = "1;35";
const SGR_DIM: &str = "2";
const SGR_DIM_ITALIC: &str = "2;3";

/// Wrap `text` in `sgr…reset` when `color` is on; passthrough
/// otherwise. Private wrapper so each renderer below doesn't repeat
/// the same conditional. Mirrors
/// [`crate::status_bar::colorize`] but kept module-local because
/// the chrome module shouldn't reach across crates for a one-line
/// helper.
fn sgr(text: &str, code: &str, color: bool) -> String {
    if color {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

// ─── Public renderers ──────────────────────────────────────────────

/// Render the user's submitted message as a single block that lands
/// in the scroll region right before the agent starts streaming.
/// The message stays in the scrollback so the user can later see
/// the conversation flow ("you said X → deeptide replied Y") at a
/// glance, mirroring how Codex / Claude Code echo user turns.
///
/// Multi-line bodies are prefixed on every visual line so the bar
/// reads as one continuous "speaker block":
///
/// ```text
/// ▎ you ▾
/// ▎ first paragraph
/// ▎ second paragraph
/// ```
///
/// A trailing newline is included so the next `println!` starts on
/// a fresh line.
pub fn render_user_block(text: &str, color: bool) -> String {
    let header = sgr(&format!("{ROLE_BAR} you ▾"), SGR_YOU, color);
    let mut lines = vec![header];
    let bar = sgr(ROLE_BAR, SGR_YOU, color);
    for line in text.lines() {
        lines.push(format!("{bar} {line}"));
    }
    // Trailing newline: callers immediately print the separator or
    // the assistant header, both of which expect to start at col 1.
    format!("{}\n", lines.join("\n"))
}

/// Render the assistant's role header that goes out RIGHT BEFORE
/// the first streamed delta of a turn. Streaming output then lands
/// on the next line, visually grouped under the `▎ deeptide ▾`
/// banner.
///
/// The returned string ends with a newline so the streaming
/// handler's first `write_all` can stay byte-for-byte unchanged —
/// the deltas just print on the next line.
pub fn render_assistant_header(color: bool) -> String {
    let bar = sgr(&format!("{ROLE_BAR} deeptide ▾"), SGR_ASSISTANT, color);
    format!("{bar}\n")
}

/// Render a single-line meta notice ("system speaks"). Used for
/// the auto-compact echo, queue dispatches, mode cycles, and any
/// other framework-originated message that should look distinct
/// from the model's prose.
///
/// Returns a non-newline-terminated string — the caller decides
/// whether to `writeln!` or `write!` based on context.
#[allow(dead_code)]
pub fn render_system_line(text: &str, color: bool) -> String {
    sgr(&format!("{ROLE_BAR} {text}"), SGR_SYSTEM, color)
}

/// Render the soft separator emitted between turns. It's a dim
/// half-width rule (defaults to 20 cells, capped to the terminal
/// width when the caller passes one) with a blank line above so
/// turns are visually breathing. The rule itself doubles as a
/// "you've reached the end of this turn" cue, especially helpful
/// when the user scrolls back through long sessions.
pub fn render_separator(width: usize, color: bool) -> String {
    // Half the column width feels right at common terminal sizes
    // (80, 120). Hard cap at 40 so very wide terminals don't get
    // an attention-grabbing line stretching half the screen.
    let glyphs = width.saturating_sub(4).clamp(8, 40);
    let rule: String = std::iter::repeat_n(SEPARATOR_GLYPH, glyphs).collect();
    let dim_rule = sgr(&rule, SGR_DIM, color);
    format!("\n{dim_rule}\n")
}

/// Compact dim-italic hint (`thinking…`, `Read(/path)`, `Bash(cmd)`).
/// Used by the pinned input row as a "ghost prompt" while the
/// agent is busy, so the bottom of the screen always shows
/// something — never a stale user-typed line and never just empty
/// space. Output is plain text with no trailing newline; the
/// caller positions the cursor.
pub fn render_thinking(label: &str, color: bool) -> String {
    let body = if label.trim().is_empty() {
        "thinking…"
    } else {
        label
    };
    sgr(&format!("{ROLE_BAR} {body}"), SGR_DIM_ITALIC, color)
}

/// Render the welcome banner shown when the REPL starts.  Codex
/// prints a two-line greeting; we go slightly richer because we
/// have more dimensions to surface (model, mode, cwd, auth state).
///
/// Layout:
///
/// ```text
/// ░ Deeptide  v0.x.y (rev abc1234)
/// ░ model deepseek-v4-pro · mode default · cwd path/to/repo · key ok
/// ░ /help for commands · Shift+Tab cycles mode · Ctrl+D exits
/// ```
///
/// `version` should already be the formatted "0.1.0 (rev abc, on main)"
/// string the CLI produces elsewhere — this function doesn't try to
/// compute it.
pub fn render_welcome(
    version: &str,
    model: &str,
    mode: &str,
    cwd: &str,
    auth_summary: &str,
    color: bool,
) -> String {
    let head_glyph = "░";
    let head = sgr(
        &format!("{head_glyph} Deeptide  {version}"),
        SGR_ASSISTANT,
        color,
    );
    let body = sgr(
        &format!("{head_glyph} model {model} · mode {mode} · cwd {cwd} · {auth_summary}"),
        SGR_DIM,
        color,
    );
    let tip = sgr(
        &format!("{head_glyph} /help for commands · Shift+Tab cycles mode · Ctrl+D exits"),
        SGR_DIM_ITALIC,
        color,
    );
    format!("{head}\n{body}\n{tip}")
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_block_includes_role_header_and_prefixes_each_line() {
        let out = render_user_block("hello\nworld", true);
        // Header line uses the warm-yellow SGR + the speaker bar.
        assert!(
            out.contains("\x1b[1;33m▎ you ▾\x1b[0m"),
            "missing colored you header: {out:?}"
        );
        // Each body line should be prefixed by a colored bar.
        assert_eq!(
            out.matches("\x1b[1;33m▎\x1b[0m").count(),
            2,
            "expected one bar per body line in: {out:?}"
        );
        // Trailing newline so the next print starts fresh.
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn user_block_in_no_color_mode_is_plain_text() {
        let out = render_user_block("hello", false);
        assert!(
            !out.contains("\x1b["),
            "no-color render must not embed SGR: {out:?}"
        );
        assert!(out.starts_with("▎ you ▾"));
        assert!(out.contains("\n▎ hello"));
    }

    #[test]
    fn assistant_header_is_one_colored_line_terminated_by_newline() {
        let out = render_assistant_header(true);
        assert!(out.starts_with("\x1b[1;36m▎ deeptide ▾"));
        assert!(out.ends_with('\n'));
        // Exactly one newline — anything more would create a blank
        // line between the header and the first streamed delta.
        assert_eq!(out.matches('\n').count(), 1);
    }

    #[test]
    fn separator_emits_blank_line_above_dim_rule_with_clamped_width() {
        let out = render_separator(200, true);
        assert!(out.starts_with('\n'), "expected blank prefix line: {out:?}");
        // Width should be capped at 40 even though we passed 200.
        let glyph_count = out.matches(SEPARATOR_GLYPH).count();
        assert!(
            (8..=40).contains(&glyph_count),
            "rule width must be clamped to [8, 40], got {glyph_count}: {out:?}"
        );
        assert!(out.contains("\x1b[2m"), "rule must be dim-styled: {out:?}");
    }

    #[test]
    fn separator_with_narrow_terminal_falls_back_to_minimum_width() {
        let out = render_separator(6, false);
        // Narrow terminal: still get at least 8 glyphs so the rule
        // is visually noticeable.
        let glyph_count = out.matches(SEPARATOR_GLYPH).count();
        assert_eq!(glyph_count, 8);
    }

    #[test]
    fn thinking_default_label_is_thinking_ellipsis() {
        let out = render_thinking("", true);
        assert!(out.contains("thinking…"), "unexpected default: {out:?}");
        assert!(out.contains("\x1b[2;3m"), "must be dim-italic: {out:?}");
        assert!(out.contains(ROLE_BAR));
    }

    #[test]
    fn thinking_custom_label_overrides_the_default() {
        let out = render_thinking("Read(/etc/hosts)", true);
        assert!(out.contains("Read(/etc/hosts)"), "label missing: {out:?}");
        assert!(
            !out.contains("thinking…"),
            "custom label should replace default: {out:?}"
        );
    }

    #[test]
    fn welcome_includes_version_model_cwd_mode_and_auth_summary() {
        let out = render_welcome(
            "0.1.0 (rev abc1234)",
            "deepseek-v4-pro",
            "default",
            "demo/path",
            "key ok",
            true,
        );
        for needle in [
            "Deeptide",
            "0.1.0 (rev abc1234)",
            "deepseek-v4-pro",
            "default",
            "demo/path",
            "key ok",
            "/help",
            "Shift+Tab",
            "Ctrl+D",
        ] {
            assert!(
                out.contains(needle),
                "welcome banner missing `{needle}`: {out}"
            );
        }
    }

    #[test]
    fn welcome_in_no_color_mode_is_plain_text_and_keeps_the_glyph() {
        let out = render_welcome("0.1.0", "m", "default", ".", "key ok", false);
        assert!(!out.contains("\x1b["));
        // The head glyph must still appear so the layout doesn't
        // collapse into a wall of text on terminals that disabled
        // color.
        assert!(out.contains("░"));
    }
}
