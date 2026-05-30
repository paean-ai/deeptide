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
use std::sync::atomic::{AtomicBool, Ordering};

/// Minimum terminal height we'll engage on. Below this the single-row bar
/// eats too much of the visible area; fall back to inline rendering.
const MIN_ROWS: u16 = 8;
/// Minimum terminal width. Anything narrower can't fit even one status
/// segment without truncation that would look broken.
const MIN_COLS: u16 = 24;

/// Default number of rows the pinned footer reserves at the bottom of
/// the terminal. Layout:
///
/// * row `rows`     → status bar (model / mode / cwd / ctx / …)
/// * row `rows - 1` → input prompt (the rustyline edit line)
///
/// Increasing this would give the input a multi-row edit area but
/// rustyline only knows how to manage a single growing line, so
/// reserving more rows just leaves blank space; 2 is the sweet spot.
pub const DEFAULT_FOOTER_ROWS: u16 = 2;

/// Set by the SIGWINCH handler. The next `repaint` reads + clears it
/// to force a fresh `terminal_size` probe + re-engage of the scroll
/// region. Atomic because the signal handler runs on a separate
/// async-signal-safe context and must not lock anything.
static RESIZE_REQUESTED: AtomicBool = AtomicBool::new(false);

/// `true` once we've installed the SIGWINCH handler. We only register
/// it once per process; subsequent engages just re-use it.
static SIGWINCH_INSTALLED: Once = Once::new();

/// Owns the bottom-anchored status bar for the lifetime of one REPL session.
///
/// Construction is the only "engagement" point — there is no separate
/// `engage()` method to avoid the bug pattern where engagement is forgotten
/// or duplicated. `Drop` is the only path that resets the scroll region.
pub struct AnchoredStatusBar {
    rows: u16,
    cols: u16,
    footer_rows: u16,
    last_painted: String,
}

impl AnchoredStatusBar {
    /// Attempt to take over the terminal with the default 2-row pinned
    /// footer (input prompt + status bar). Returns `None` (rather than
    /// panicking or printing garbage) when stdout isn't a TTY, the
    /// terminal is too small, or `TERM` is missing/`dumb`.
    pub fn try_engage() -> Option<Self> {
        Self::try_engage_with_footer(DEFAULT_FOOTER_ROWS)
    }

    /// Same as [`try_engage`] but with a caller-chosen footer height
    /// (in rows). Use this when the embedder wants a richer pinned
    /// area — e.g. tests pass `footer_rows = 1` to drive the
    /// pre-existing single-row layout.
    ///
    /// [`try_engage`]: AnchoredStatusBar::try_engage
    pub fn try_engage_with_footer(footer_rows: u16) -> Option<Self> {
        if !io::stdout().is_terminal() {
            return None;
        }
        let term = std::env::var("TERM").unwrap_or_default();
        if term.is_empty() || term == "dumb" {
            return None;
        }
        let (cols, rows) = terminal_size::terminal_size().map(|(w, h)| (w.0, h.0))?;
        // A footer that swallows half the terminal would be a UX bug;
        // a 1-row footer is the bare minimum (status bar only).
        let footer_rows = footer_rows.max(1).min(rows.saturating_sub(MIN_ROWS / 2));
        if rows < MIN_ROWS || cols < MIN_COLS {
            return None;
        }

        install_panic_recovery(rows);
        install_sigwinch_handler();

        let mut bar = Self {
            rows,
            cols,
            footer_rows,
            last_painted: String::new(),
        };
        bar.engage_terminal();
        Some(bar)
    }

    /// Width available for the bar text, after subtracting any future padding.
    pub fn cols(&self) -> usize {
        self.cols as usize
    }

    /// Number of rows reserved at the bottom (input + status, etc.).
    pub fn footer_rows(&self) -> u16 {
        self.footer_rows
    }

    /// 1-based row index where the input prompt should be rendered.
    /// When the footer is 2+ rows tall, this is the row directly above
    /// the status bar; for a 1-row footer it coincides with the status
    /// bar row (legacy behaviour — input renders inline above).
    pub fn input_row(&self) -> u16 {
        if self.footer_rows >= 2 {
            self.rows.saturating_sub(self.footer_rows - 1)
        } else {
            self.rows
        }
    }

    /// 1-based row index for the bottom-most line of the scroll region
    /// (i.e. the last row INTO which conversation output may scroll).
    pub fn scroll_region_bottom(&self) -> u16 {
        self.rows.saturating_sub(self.footer_rows).max(1)
    }

    /// Paint `line` at the status row. Idempotent: a repaint with
    /// identical content does no IO so a tight REPL loop doesn't flood
    /// stdout.
    ///
    /// `lock` is an external mutex shared with any other thread that
    /// writes to stdout (e.g. the spinner thread). Holding it across
    /// the save/jump/write/restore sequence is what keeps the bar from
    /// interleaving with streamed model output.
    pub fn repaint(&mut self, line: &str, lock: &std::sync::Mutex<()>) {
        // Resize sources, in priority order:
        //   1. SIGWINCH flag (set asynchronously by the kernel) —
        //      authoritative trigger that something just changed.
        //   2. Polled `terminal_size` probe — catches resizes that
        //      happened before SIGWINCH was installed, or in
        //      environments where signals aren't delivered.
        let sigwinch_dirty = RESIZE_REQUESTED.swap(false, Ordering::Relaxed);
        if sigwinch_dirty {
            // Force a re-probe regardless of cached dims.
            self.refresh_dimensions();
            self.engage_terminal();
        } else if self.detect_resize() {
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

    /// Position the cursor at the input row (`rows - footer_rows + 1`)
    /// and clear it, so the next `rl.readline(...)` call writes its
    /// prompt + edit buffer there. Must be called BEFORE invoking
    /// rustyline; the cursor stays at the cleared column until
    /// rustyline's prompt write moves it.
    ///
    /// Holding `lock` keeps the cursor jump atomic against the spinner
    /// thread — without it, a spinner tick could squeeze in between the
    /// jump and the readline call.
    pub fn prepare_input_row(&self, lock: &std::sync::Mutex<()>) {
        if let Ok(_guard) = lock.lock() {
            let seq = jump_and_clear_seq(self.input_row());
            let mut out = io::stdout();
            let _ = out.write_all(seq.as_bytes());
            let _ = out.flush();
        }
    }

    /// Paint a single-line "ghost" text at the input row WITHOUT
    /// moving the caller's logical cursor — used to keep the input
    /// area visually occupied (e.g. `▎ thinking…`) while the agent
    /// is busy. The sequence:
    ///
    /// 1. Save cursor (DECSC `\x1b7`).
    /// 2. Jump to the input row + clear it.
    /// 3. Write `text` (caller is responsible for any color codes
    ///    or truncation — we don't attempt to width-clamp because
    ///    the chrome module already produces ANSI-styled content
    ///    that's hard to slice safely).
    /// 4. Restore cursor (DECRC `\x1b8`).
    ///
    /// Returning the cursor lets the caller continue streaming
    /// model output into the scroll region with no perceived
    /// re-positioning. Holding `lock` keeps the paint atomic
    /// against the spinner / streaming threads.
    pub fn paint_input_ghost(&self, text: &str, lock: &std::sync::Mutex<()>) {
        if self.footer_rows < 2 {
            return;
        }
        if let Ok(_guard) = lock.lock() {
            let mut out = io::stdout();
            let _ = out.write_all(paint_ghost_seq(self.input_row(), text).as_bytes());
            let _ = out.flush();
        }
    }

    /// Recover from `rl.readline` returning. The user just pressed
    /// Enter so rustyline emitted a final newline; depending on
    /// whether the input wrapped, the cursor may be on the status
    /// row or below it.  This method:
    ///
    /// 1. Clears the input row so it appears empty during
    ///    streaming — the submitted text already lives in the agent
    ///    transcript that gets printed into the scroll region, so
    ///    leaving it stale at the bottom would just be visual
    ///    noise. (This is the "your message moves up" feel of
    ///    Codex / Claude Code.)
    /// 2. Repaints the status bar (in case input overflow scribbled
    ///    on it).
    /// 3. Moves the cursor to the bottom of the scroll region so
    ///    any subsequent `println!` (streamed model output,
    ///    slash-command results) lands INSIDE the scroll region
    ///    rather than clobbering the pinned footer.
    pub fn recover_to_scroll_region(&mut self, status_line: &str, lock: &std::sync::Mutex<()>) {
        if let Ok(_guard) = lock.lock() {
            let mut out = io::stdout();
            // (1) Clear the input row when we actually reserved one
            //     (footer >= 2). For the single-row legacy footer
            //     there is no separate input row to wipe.
            if self.footer_rows >= 2 {
                let _ = out.write_all(jump_and_clear_seq(self.input_row()).as_bytes());
            }
            // (2) Repaint the status row. We bypass the
            //     last_painted cache: input overflow could have
            //     overwritten the status row WITHOUT changing the
            //     intended status string, so the cache would
            //     suppress the necessary re-paint.
            let _ = out.write_all(paint_bar_seq(self.rows, status_line).as_bytes());
            // (3) Drop the cursor into the bottom of the scroll
            //     region so downstream prints scroll naturally
            //     upwards.
            let scroll_bottom = self.scroll_region_bottom();
            let _ = out.write_all(format!("\x1b[{scroll_bottom};1H").as_bytes());
            let _ = out.flush();
        }
        // Sync the cache so the next idempotent `repaint` is a
        // no-op for the same status string.
        self.last_painted = status_line.to_owned();
    }

    fn engage_terminal(&mut self) {
        let seq = engage_seq(self.rows, self.footer_rows);
        let mut out = io::stdout();
        let _ = out.write_all(seq.as_bytes());
        let _ = out.flush();
        // Drop the "last painted" cache: after re-engaging (e.g. after
        // a SIGWINCH), the footer rows were cleared by engage_seq, so
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

    fn refresh_dimensions(&mut self) {
        if let Some((w, h)) = terminal_size::terminal_size() {
            self.cols = w.0;
            self.rows = h.0;
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

/// Build the ANSI sequence that engages the scroll region `[1..=rows -
/// footer_rows]` and clears every reserved footer row so the pinned
/// area starts visually empty.
fn engage_seq(rows: u16, footer_rows: u16) -> String {
    let footer = footer_rows.max(1);
    let bottom = rows.saturating_sub(footer).max(1);

    // 1. Save cursor (DECSC) so we can return the user's input
    //    position.
    // 2. Set DECSTBM [1; bottom] — reserves the LAST `footer` rows
    //    outside the scroll region.
    // 3. Clear each reserved row (footer can be 1..N) — in case
    //    something was already on them when we engaged.
    // 4. Move into the bottom of the scroll region so subsequent
    //    prints scroll naturally upwards.
    // 5. Restore the saved cursor (DECRC) — most terminals honor this
    //    even after DECSTBM resets the cursor to (1,1) per spec.
    let mut out = format!("\x1b7\x1b[1;{bottom}r", bottom = bottom);
    for i in 0..footer {
        let row = rows.saturating_sub(i).max(1);
        out.push_str(&format!("\x1b[{row};1H\x1b[2K"));
    }
    out.push_str(&format!("\x1b[{bottom};1H\x1b8", bottom = bottom));
    out
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

/// Build the ANSI sequence that jumps the cursor to `row`, column 1,
/// and clears that row. Used by `prepare_input_row` to position the
/// rustyline edit area at the bottom-most non-status row.
fn jump_and_clear_seq(row: u16) -> String {
    format!("\x1b[{row};1H\x1b[2K", row = row.max(1))
}

/// Build the ANSI sequence that paints `text` at `row` without
/// disturbing the caller's logical cursor: save-cursor (DECSC),
/// jump-and-clear-row, write text, restore-cursor (DECRC).
///
/// Extracted as a free function so we can unit-test the exact byte
/// sequence we emit — `paint_input_ghost` itself does live IO and
/// would be hard to assert against directly.
fn paint_ghost_seq(row: u16, text: &str) -> String {
    let row = row.max(1);
    format!("\x1b7\x1b[{row};1H\x1b[2K{text}\x1b8")
}

/// Install a SIGWINCH handler that sets [`RESIZE_REQUESTED`] so the
/// next [`AnchoredStatusBar::repaint`] re-probes terminal dimensions
/// and re-engages the scroll region. Idempotent across calls; the
/// handler does not allocate or take locks (async-signal-safe).
///
/// On non-Unix targets this is a no-op — the polled `detect_resize`
/// path is the only resize signal we get.
fn install_sigwinch_handler() {
    #[cfg(unix)]
    {
        SIGWINCH_INSTALLED.call_once(|| {
            // SAFETY: libc::signal is async-signal-safe; the handler
            // we install only writes to an AtomicBool, which is also
            // async-signal-safe.
            //
            // The intermediate `*const ()` cast keeps clippy's
            // `function_casts_as_integer` lint happy on newer
            // toolchains — casting a fn item straight to an integer
            // type is now an error under `-D warnings`.
            unsafe {
                libc::signal(
                    libc::SIGWINCH,
                    sigwinch_handler as *const () as libc::sighandler_t,
                );
            }
        });
    }
    #[cfg(not(unix))]
    {
        // SIGWINCH doesn't exist on Windows; we rely entirely on the
        // polled `terminal_size` probe inside `repaint`.
        let _ = &SIGWINCH_INSTALLED;
    }
}

#[cfg(unix)]
extern "C" fn sigwinch_handler(_sig: libc::c_int) {
    // Only writes to an atomic — no allocation, no locks, fully
    // async-signal-safe.
    RESIZE_REQUESTED.store(true, Ordering::Relaxed);
}

/// Test-only helper: force the resize flag so unit tests can drive
/// the `repaint` re-engage path without spinning up a real terminal.
#[cfg(test)]
pub(crate) fn signal_resize_for_test() {
    RESIZE_REQUESTED.store(true, Ordering::Relaxed);
}

/// Test-only helper: snapshot + clear the resize flag.
#[cfg(test)]
pub(crate) fn take_resize_flag_for_test() -> bool {
    RESIZE_REQUESTED.swap(false, Ordering::Relaxed)
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
    fn engage_seq_reserves_one_row_footer_by_default_layout() {
        let seq = engage_seq(24, 1);
        // DECSC at the start so the caller's cursor is preserved
        // across the engage sequence.
        assert!(seq.starts_with("\x1b7"), "expected DECSC first: {seq:?}");
        // DECSTBM with bottom = rows - footer = 23.
        assert!(
            seq.contains("\x1b[1;23r"),
            "expected DECSTBM [1;23]: {seq:?}"
        );
        // Jump to + clear the single reserved status row.
        assert!(
            seq.contains("\x1b[24;1H"),
            "expected jump to row 24: {seq:?}"
        );
        assert!(seq.contains("\x1b[2K"), "expected line clear: {seq:?}");
        assert!(seq.ends_with("\x1b8"), "expected DECRC at the end: {seq:?}");
    }

    #[test]
    fn engage_seq_two_row_footer_clears_both_input_and_status_rows() {
        let seq = engage_seq(24, 2);
        // Bottom of scroll region drops by another row: rows-2 = 22.
        assert!(
            seq.contains("\x1b[1;22r"),
            "two-row footer must reserve rows 23+24, leaving region [1;22]: {seq:?}"
        );
        // Both reserved rows must be cleared.
        assert!(
            seq.contains("\x1b[24;1H"),
            "must clear status row 24: {seq:?}"
        );
        assert!(
            seq.contains("\x1b[23;1H"),
            "must clear input row 23: {seq:?}"
        );
    }

    #[test]
    fn engage_seq_pins_minimum_one_row_region_on_a_very_short_terminal() {
        // rows = 2, footer = 1 → bottom = 1.
        let seq = engage_seq(2, 1);
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

    #[test]
    fn jump_and_clear_seq_places_cursor_at_row_one_column_and_erases_line() {
        let seq = jump_and_clear_seq(23);
        // Sequence: CUP to row 23 column 1, then EL2 (erase whole line).
        assert_eq!(seq, "\x1b[23;1H\x1b[2K");
    }

    #[test]
    fn jump_and_clear_seq_clamps_zero_row_to_one() {
        // Row 0 is not a valid VT cursor coordinate; the helper must
        // not emit `\x1b[0;1H` (which some terminals reject).
        let seq = jump_and_clear_seq(0);
        assert!(
            seq.starts_with("\x1b[1;1H"),
            "expected row clamped to 1: {seq:?}"
        );
    }

    #[test]
    fn paint_ghost_seq_saves_cursor_paints_at_row_and_restores() {
        let seq = paint_ghost_seq(23, "▎ thinking…");
        // DECSC opens the sequence so the caller's cursor is
        // preserved.
        assert!(seq.starts_with("\x1b7"), "expected DECSC at start: {seq:?}");
        // Cursor must jump to row 23 column 1 and clear the row
        // before writing the ghost text.
        assert!(
            seq.contains("\x1b[23;1H\x1b[2K"),
            "expected jump + clear for row 23: {seq:?}"
        );
        // The ghost text itself is embedded verbatim.
        assert!(
            seq.contains("▎ thinking…"),
            "ghost text must appear verbatim: {seq:?}"
        );
        // DECRC closes the sequence so streaming output continues
        // at the saved position.
        assert!(seq.ends_with("\x1b8"), "expected DECRC at end: {seq:?}");
    }

    #[test]
    fn paint_ghost_seq_clamps_zero_row_to_one() {
        // Row 0 is not a valid VT cursor coordinate — verify the
        // helper degrades rather than emitting an invalid escape.
        let seq = paint_ghost_seq(0, "hi");
        assert!(
            seq.contains("\x1b[1;1H"),
            "expected row clamped to 1: {seq:?}"
        );
    }

    #[test]
    fn resize_flag_round_trips_through_helper_accessors() {
        // The flag is process-global; clear first so a previous test
        // doesn't poison this assertion.
        let _ = take_resize_flag_for_test();
        assert!(!take_resize_flag_for_test());
        signal_resize_for_test();
        assert!(
            take_resize_flag_for_test(),
            "expected the flag to be high after signal"
        );
        assert!(
            !take_resize_flag_for_test(),
            "take must clear the flag — otherwise repaint would re-engage forever"
        );
    }
}
