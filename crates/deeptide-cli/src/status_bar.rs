//! Bottom-anchored status bar via DECSTBM (scroll region).
//!
//! The terminal-control story:
//!
//!   * `engage()` sets the DEC top/bottom-margin (`\x1b[1;<rows-1>r`) so the
//!     LAST row stays outside the scroll region. Conversation output prints
//!     INSIDE the region and scrolls within rows 1..=rows-1; the bar at row
//!     `rows` stays put no matter how much output flows past.
//!   * `repaint(line)` saves the cursor (DECSC `\x1b7`), jumps to row `rows`,
//!     clears that row (`\x1b[2K`), writes the bar, then restores the cursor
//!     (DECRC `\x1b8`). The user's input position is untouched.
//!   * `Drop` MUST reset the region (`\x1b[r`) so the user's shell prompt
//!     isn't stuck below an invisible boundary after the REPL exits. We also
//!     install a panic hook that resets the region on any panic so a crash
//!     doesn't leave the terminal half-broken.
//!
//! Safe fallback: `try_engage` returns `None` whenever it's not safe to take
//! over the terminal — non-TTY stdout (piped, CI, `--output-format json`),
//! `TERM=dumb`, terminals shorter than [`MIN_ROWS`] or narrower than
//! [`MIN_COLS`], or environments where `terminal_size` can't probe a size.
//! The CLI's REPL loop falls back to the legacy "print the status line
//! inline above the prompt" behaviour in those cases.
//!
//! All pure helpers in this module (the ANSI sequence builders and the
//! styling wrappers) are unit-tested without touching the terminal so the
//! escape sequences are pinned and regressions surface early.

use std::io::{self, IsTerminal, Write};
use std::sync::Once;

/// Minimum terminal height we'll engage on. Below this the single-row bar
/// eats too much of the visible area; fall back to inline rendering.
const MIN_ROWS: u16 = 8;
/// Minimum terminal width. Anything narrower can't fit even one status
/// segment without truncation that would look broken.
const MIN_COLS: u16 = 24;

/// Owns the bottom-anchored status bar for the lifetime of one REPL session.
///
/// Construction is the only "engagement" point — there is no separate
/// `engage()` method to avoid the bug pattern where engagement is forgotten
/// or duplicated. `Drop` is the only path that resets the scroll region.
pub struct AnchoredStatusBar {
    rows: u16,
    cols: u16,
    last_painted: String,
}

impl AnchoredStatusBar {
    /// Attempt to take over the terminal. Returns `None` (rather than
    /// panicking or printing garbage) when stdout isn't a TTY, the terminal
    /// is too small, or `TERM` is missing/`dumb`.
    pub fn try_engage() -> Option<Self> {
        if !io::stdout().is_terminal() {
            return None;
        }
        let term = std::env::var("TERM").unwrap_or_default();
        if term.is_empty() || term == "dumb" {
            return None;
        }
        let (cols, rows) = terminal_size::terminal_size().map(|(w, h)| (w.0, h.0))?;
        if rows < MIN_ROWS || cols < MIN_COLS {
            return None;
        }

        install_panic_recovery(rows);

        let mut bar = Self {
            rows,
            cols,
            last_painted: String::new(),
        };
        bar.engage_terminal();
        Some(bar)
    }

    /// Width available for the bar text, after subtracting any future padding.
    pub fn cols(&self) -> usize {
        self.cols as usize
    }

    /// Paint `line` at row `rows`. Idempotent: a repaint with identical
    /// content does no IO so a tight REPL loop doesn't flood stdout.
    ///
    /// `lock` is an external mutex shared with any other thread that writes
    /// to stdout (e.g. the spinner thread). Holding it across the save/jump/
    /// write/restore sequence is what keeps the bar from interleaving with
    /// streamed model output.
    pub fn repaint(&mut self, line: &str, lock: &std::sync::Mutex<()>) {
        if self.detect_resize() {
            self.engage_terminal();
        }
        if line == self.last_painted {
            return;
        }
        if let Ok(_guard) = lock.lock() {
            let seq = paint_bar_seq(self.rows, line);
            let mut out = io::stdout();
            let _ = out.write_all(seq.as_bytes());
            let _ = out.flush();
        }
        self.last_painted = line.to_owned();
    }

    fn engage_terminal(&mut self) {
        let seq = engage_seq(self.rows);
        let mut out = io::stdout();
        let _ = out.write_all(seq.as_bytes());
        let _ = out.flush();
        // Drop the "last painted" cache: after re-engaging (e.g. after a
        // SIGWINCH), the bar row was cleared by engage_seq's `\x1b[2K`, so
        // any cached value would suppress the necessary first repaint.
        self.last_painted.clear();
    }

    fn detect_resize(&mut self) -> bool {
        let Some((w, h)) = terminal_size::terminal_size() else {
            return false;
        };
        let (cols, rows) = (w.0, h.0);
        if cols != self.cols || rows != self.rows {
            self.cols = cols;
            self.rows = rows;
            true
        } else {
            false
        }
    }
}

impl Drop for AnchoredStatusBar {
    fn drop(&mut self) {
        let seq = disengage_seq(self.rows);
        let mut out = io::stdout();
        let _ = out.write_all(seq.as_bytes());
        let _ = out.flush();
    }
}

/// Install a panic hook that resets the scroll region so a crashed REPL
/// doesn't strand the user in a half-broken terminal. Idempotent across
/// calls — only the first invocation registers anything, with the previous
/// hook preserved and chained.
fn install_panic_recovery(rows: u16) {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let seq = disengage_seq(rows);
            let mut err = io::stderr();
            let _ = err.write_all(seq.as_bytes());
            let _ = err.flush();
            previous(info);
        }));
    });
}

/// Build the ANSI sequence that engages the scroll region [1..=rows-1] and
/// clears the bar row so it starts visually empty.
fn engage_seq(rows: u16) -> String {
    let bottom = rows.saturating_sub(1).max(1);
    // 1. Save cursor (DECSC) so we can return the user's input position.
    // 2. Set DECSTBM [1; bottom] — reserves the LAST row outside the region.
    // 3. Jump to the reserved row and clear it (in case something was there).
    // 4. Move into the bottom of the region so subsequent prints scroll
    //    naturally upwards.
    // 5. Restore the saved cursor (DECRC) — most terminals honor this even
    //    after DECSTBM resets the cursor to (1,1) per spec.
    format!(
        "\x1b7\x1b[1;{bottom}r\x1b[{rows};1H\x1b[2K\x1b[{bottom};1H\x1b8",
        bottom = bottom,
        rows = rows
    )
}

/// Build the ANSI sequence that paints `text` at row `rows` (the reserved
/// bar row), then restores the cursor to wherever it was before the paint.
fn paint_bar_seq(rows: u16, text: &str) -> String {
    format!("\x1b7\x1b[{rows};1H\x1b[2K{text}\x1b8", rows = rows)
}

/// Build the ANSI sequence that releases the scroll region and clears the
/// stale bar row so the shell prompt can re-emerge cleanly when the REPL
/// exits.
fn disengage_seq(rows: u16) -> String {
    format!("\x1b[r\x1b[{rows};1H\x1b[2K\n", rows = rows)
}

// ─── Styling helpers ────────────────────────────────────────────────────────
//
// All helpers degrade to plain text when `color = false`, so `--no-color`,
// `NO_COLOR`, and `cli_config.json` toggles all reach the right output.

/// Dim grey — used for the status bar text itself so it visually recedes
/// behind the conversation.
pub fn dim(text: &str, color: bool) -> String {
    if color {
        format!("\x1b[2m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

/// Apply an arbitrary SGR sequence around `text`, with safe degradation.
///
/// Currently used only by tests but kept on the public surface for the next
/// iteration (role-prefix coloring in the transcript: `assistant>` blue,
/// `you>` green, `tool>` yellow, `system>` magenta). Removing it now would
/// just mean re-deriving the same wrapper a week later.
#[allow(dead_code)]
pub fn colorize(text: &str, sgr: &str, color: bool) -> String {
    if color {
        format!("\x1b[{sgr}m{text}\x1b[0m")
    } else {
        text.to_owned()
    }
}

/// Wrap `text` in a rustyline-safe SGR pair. Rustyline needs invisible escape
/// sequences delimited by `\x01...\x02` so the line editor counts only the
/// visible glyphs when positioning the cursor. Using `colorize` for the
/// rustyline prompt would mis-place the cursor after every keystroke.
pub fn rustyline_safe(text: &str, sgr: &str, color: bool) -> String {
    if color {
        format!("\x01\x1b[{sgr}m\x02{text}\x01\x1b[0m\x02")
    } else {
        text.to_owned()
    }
}

/// SGR codes used elsewhere in the CLI. Centralised so the palette stays
/// consistent across the prompt, spinner, role prefixes, and status bar.
pub mod palette {
    /// Bright cyan + bold — used for the interactive prompt prefix.
    pub const PROMPT: &str = "1;36";
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engage_seq_reserves_bottom_row_and_returns_to_caller_position() {
        let seq = engage_seq(24);
        // DECSC at the start so the caller's cursor is preserved across the
        // engage sequence.
        assert!(seq.starts_with("\x1b7"), "expected DECSC first: {seq:?}");
        // DECSTBM with bottom = rows - 1.
        assert!(
            seq.contains("\x1b[1;23r"),
            "expected DECSTBM [1;23]: {seq:?}"
        );
        // We jump to and clear the reserved row so it isn't holding stale
        // content from before engagement.
        assert!(
            seq.contains("\x1b[24;1H"),
            "expected jump to row 24: {seq:?}"
        );
        assert!(seq.contains("\x1b[2K"), "expected line clear: {seq:?}");
        // DECRC at the end restores the caller's cursor.
        assert!(seq.ends_with("\x1b8"), "expected DECRC at the end: {seq:?}");
    }

    #[test]
    fn engage_seq_pins_minimum_two_row_region_on_a_very_short_terminal() {
        // rows = 2 → bottom must clamp to 1, not 0 (which would be invalid).
        let seq = engage_seq(2);
        assert!(
            seq.contains("\x1b[1;1r"),
            "DECSTBM must use a 1-based bottom even when rows-1 underflows: {seq:?}"
        );
    }

    #[test]
    fn paint_bar_seq_writes_text_at_the_reserved_row_around_a_cursor_save() {
        let seq = paint_bar_seq(30, "model deepseek-v4-pro | cost $0.01");
        assert_eq!(
            seq,
            "\x1b7\x1b[30;1H\x1b[2Kmodel deepseek-v4-pro | cost $0.01\x1b8"
        );
    }

    #[test]
    fn disengage_seq_releases_the_region_and_clears_the_bar_row() {
        let seq = disengage_seq(40);
        assert!(
            seq.starts_with("\x1b[r"),
            "DECSTBM reset must come first so the move-to-row is allowed: {seq:?}"
        );
        assert!(seq.contains("\x1b[40;1H"), "expected row-40 jump: {seq:?}");
        assert!(seq.contains("\x1b[2K"), "expected line clear: {seq:?}");
        // Trailing newline gives the shell prompt a clean line to land on.
        assert!(seq.ends_with('\n'), "expected trailing newline: {seq:?}");
    }

    #[test]
    fn dim_brackets_text_in_sgr_when_color_is_on_and_is_a_noop_otherwise() {
        assert_eq!(dim("hi", true), "\x1b[2mhi\x1b[0m");
        assert_eq!(dim("hi", false), "hi");
    }

    #[test]
    fn colorize_emits_the_sgr_pair_and_degrades_cleanly() {
        assert_eq!(colorize("ok", "1;36", true), "\x1b[1;36mok\x1b[0m");
        assert_eq!(colorize("ok", "1;36", false), "ok");
    }

    #[test]
    fn rustyline_safe_wraps_escapes_in_zero_width_markers() {
        // `\x01` and `\x02` tell rustyline "the bytes between these are
        // invisible" so cursor positioning is computed from the glyphs only.
        let out = rustyline_safe("deeptide> ", "1;36", true);
        assert!(
            out.starts_with("\x01\x1b[1;36m\x02"),
            "expected `\\x01` + SGR + `\\x02` prefix: {out:?}"
        );
        assert!(
            out.ends_with("\x01\x1b[0m\x02"),
            "expected `\\x01` + reset + `\\x02` suffix: {out:?}"
        );
        // The visible glyphs are sandwiched between the markers, unchanged.
        assert!(out.contains("\x02deeptide> \x01"));
    }

    #[test]
    fn rustyline_safe_is_a_noop_when_color_is_off_so_prompt_width_is_exact() {
        assert_eq!(rustyline_safe("> ", "1;36", false), "> ");
    }
}
