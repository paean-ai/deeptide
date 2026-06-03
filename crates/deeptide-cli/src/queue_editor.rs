//! Raw-mode mid-turn input editor.
//!
//! Why this exists
//! ===============
//!
//! While the agent is busy (`repl.submit()` is blocked streaming
//! tokens, running tools, etc.) the user has nothing to type into —
//! rustyline is idle, and stdin is in the kernel's default *cooked*
//! line discipline. Cooked mode means every keystroke is echoed
//! immediately to whatever the cursor happens to be on (usually the
//! spinner / streaming row), so anything the user types becomes
//! interleaved noise instead of input. They can't see what they're
//! typing, can't see how to commit it, and can't see a queue
//! depth — the existing `MessageQueue` is a phantom feature with
//! no on-screen affordance.
//!
//! Codex / Claude Code / Cursor CLI solve this by running stdin in
//! raw mode for the entire session and rendering their own line
//! editor. We take the same approach but scoped to a *single turn*:
//!
//! 1. When the turn starts we drop stdin into raw mode and spawn a
//!    short-lived input thread.
//! 2. The thread reads one byte at a time, accumulates into an
//!    in-memory buffer, and repaints the pinned input row whenever
//!    the buffer changes.
//! 3. On Enter, the buffer is pushed into the shared
//!    `MessageQueue` and cleared. The pinned row immediately shows
//!    the new queue depth so the user knows the line landed.
//! 4. When the agent's turn ends, the thread is signalled to stop
//!    and termios is restored (Drop on `RawModeGuard`).
//!
//! Out of scope (by design, for the first iteration)
//! -------------------------------------------------
//!
//! - Up/Down history recall during the turn. rustyline owns
//!   history and is idle; resurrecting it would require shared
//!   state.
//!
//! Supported in-line editing (added after first iteration):
//! - Left/Right arrow keys move the cursor one grapheme cluster at
//!   a time within the current draft.
//! - Home (Ctrl-A) / End (Ctrl-E) jump to the buffer ends.
//! - Delete (`CSI 3 ~`) removes the grapheme cluster at the cursor.
//! - Insert + Backspace both operate at the cursor position rather
//!   than at the buffer's end, so the editor behaves like every
//!   other modern shell line-editor.
//! - Multi-line / soft-newline. Pressing Enter always submits.
//! - Windows. The raw-mode termios machinery is Unix-only; on other
//!   platforms `take_old_termios` returns `None` and the caller
//!   should fall back to the legacy cooked-mode peek path.
//!
//! Testing strategy
//! ----------------
//!
//! Everything that *can* be tested as a pure function is — the
//! line painter, the UTF-8 boundary scanner, the Ctrl-code
//! recogniser. The raw-mode + thread orchestration is exercised by
//! the smoke test in the binary's integration suite.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use deeptide_core::MessageQueue;
use deeptide_core::width::{char_width, display_width};

use crate::line_edit::{
    backspace_grapheme, delete_grapheme_at_cursor, next_grapheme_boundary, prev_grapheme_boundary,
};

/// Foreground color used for the "type to queue" prompt prefix and
/// the pending count badge. Bright cyan keeps it consistent with
/// the rest of the assistant-themed UI; the body of the buffer
/// itself is rendered in default-foreground so the user's typing
/// stands out.
const PROMPT_COLOR: &str = "\x1b[1;36m";
/// Dim sequence for the hint text after the buffer, and for the
/// queue-depth pill.
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

// ─── Key recognisers ───────────────────────────────────────────────
//
// Constants centralise the "which byte means what" mapping so the
// pump loop reads as a flat match instead of a thicket of magic
// numbers. Multi-byte sequences (arrow keys, bracketed paste) are
// recognised by the state machine in `PumpState` below.

const ETX: u8 = 0x03; // Ctrl-C
const NAK: u8 = 0x15; // Ctrl-U: clear line
const BS_LEGACY: u8 = 0x08; // Ctrl-H, some terminals send this for Backspace
const DEL: u8 = 0x7f; // standard Backspace on most terminals
const CR: u8 = b'\r';
const LF: u8 = b'\n';
const ESC: u8 = 0x1b;

/// Result of a single keystroke being consumed. Lets the caller
/// decide what to repaint (or whether to push the buffer to the
/// queue without painting at all).
#[derive(Debug, PartialEq, Eq)]
pub enum KeyOutcome {
    /// Buffer state changed; the input row should be repainted.
    Repaint,
    /// User pressed Enter; `pending` contains the trimmed line to
    /// enqueue. The buffer has already been cleared.
    Submit { pending: String },
    /// User pressed Ctrl-C; treat as a "cancel my draft" gesture.
    /// The buffer has been cleared but nothing should be submitted.
    Cancelled,
    /// User pressed Shift+Tab. The editor doesn't itself know how to
    /// cycle the permission mode (it has no access to the agent
    /// loop), so it forwards the intent up to the pump, which sets
    /// the shared `mode_cycle` flag. The CLI then applies the
    /// cycle either inside the next permission prompt (as a
    /// quick-yolo shortcut) or at turn boundary.
    ModeCycle,
    /// Byte was consumed but had no visible effect (escape sequence
    /// progress, unrecognised key, etc.). No repaint needed.
    Nothing,
}

/// Single-line input editor with an internal UTF-8-safe `String`
/// buffer. Drives the pinned input row during a streaming turn.
///
/// Threading: the editor is held by exactly one thread (the
/// per-turn input thread). The shared *queue* is `Arc<Mutex<_>>`;
/// the editor itself doesn't need to be `Send + Sync`.
#[derive(Debug, Default)]
pub struct QueueEditor {
    buf: String,
    /// Byte offset of the caret within `buf`. Always sits on a
    /// UTF-8 codepoint boundary; navigation moves it by grapheme
    /// clusters so a single arrow keypress steps over one
    /// user-perceived character (base + any combining marks /
    /// variation selectors / ZWJ chains).
    cursor: usize,
    /// Active multi-byte sequence parser. Most bytes hit the
    /// "ground" state and are dispatched as ordinary input; the
    /// state machine just handles the escape-sequence detours.
    state: PumpState,
    /// In-progress UTF-8 codepoint assembly. When the terminal
    /// delivers a multi-byte char (e.g. CJK input) we get the
    /// bytes one at a time via `libc::read`; we can't naively cast
    /// each byte to `char` because that produces Latin-1
    /// codepoints. Bytes accumulate here until the codepoint is
    /// complete, then we decode it as a `&str` and push it into
    /// the visible buffer in one atomic step.
    utf8_pending: Vec<u8>,
    utf8_expected: usize,
    /// Accumulator for the leading numeric parameter of a CSI
    /// sequence (e.g. the `3` in `ESC [ 3 ~` for the Delete key).
    /// Reset every time we enter `PumpState::Csi`. We only track
    /// the first parameter — every key we currently recognise
    /// either has zero parameters (arrows, Home, End, Shift+Tab)
    /// or one trailing parameter (the `~` family). Multi-param
    /// sequences (modifier keys, mouse reporting) are still
    /// swallowed cleanly because we ignore everything after the
    /// first `;`.
    csi_param: Option<u32>,
    csi_saw_semi: bool,
}

/// Tracks where we are inside a multi-byte input sequence. Kept
/// deliberately tiny — we only need to *skip* CSI-style sequences
/// (so arrow keys / function keys don't pollute the buffer) and to
/// gate the body of a bracketed-paste block.
#[derive(Debug, Default, PartialEq, Eq)]
enum PumpState {
    /// Default: each byte is regular input or a control code.
    #[default]
    Ground,
    /// Just saw ESC; next byte tells us what kind of sequence
    /// we're in.
    Escape,
    /// Inside a CSI (`ESC [`) sequence; consume bytes until we see
    /// a final byte in `0x40..=0x7e` which closes the sequence.
    Csi,
    /// Inside a bracketed-paste block (`ESC [ 200 ~ … ESC [ 201 ~`);
    /// every byte is literal input until the end marker. Reserved
    /// for the future refinement noted in `consume_csi`; currently
    /// unreachable but kept for clarity of intent.
    #[allow(dead_code)]
    Paste,
}

impl QueueEditor {
    /// Fresh editor with an empty buffer in the ground state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Buffer contents. Used by the painter to render what the
    /// user has typed so far.
    pub fn buffer(&self) -> &str {
        &self.buf
    }

    /// Byte offset of the caret within `buffer()`. Always on a
    /// codepoint boundary; the painter uses this to split the
    /// buffer into prefix / suffix slices around the cursor block.
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// Test helper: insert raw text as if it had come from the
    /// terminal in ground state and place the cursor at the end.
    /// Useful for unit tests that want to set up a non-empty
    /// buffer without driving every byte through `consume`.
    #[cfg(test)]
    fn seed(&mut self, text: &str) {
        self.buf.push_str(text);
        self.cursor = self.buf.len();
    }

    /// Test helper: same as [`seed`] but lets the test position
    /// the caret somewhere inside the buffer (in byte offset).
    /// Panics if `cursor` is not on a UTF-8 boundary — that's a
    /// test bug.
    #[cfg(test)]
    fn seed_with_cursor(&mut self, text: &str, cursor: usize) {
        self.buf.push_str(text);
        assert!(
            self.buf.is_char_boundary(cursor),
            "seed cursor {cursor} not on char boundary of {text:?}"
        );
        self.cursor = cursor;
    }

    /// Feed one byte into the editor. Returns a [`KeyOutcome`]
    /// describing what (if anything) the caller needs to do
    /// downstream. The byte stream is assumed to be UTF-8 — we
    /// only inspect ASCII control codes in `Ground`, every other
    /// byte is appended verbatim and `String::pop` is left to
    /// handle codepoint boundaries on Backspace.
    pub fn consume(&mut self, byte: u8) -> KeyOutcome {
        match self.state {
            PumpState::Ground => self.consume_ground(byte),
            PumpState::Escape => self.consume_escape(byte),
            PumpState::Csi => self.consume_csi(byte),
            PumpState::Paste => self.consume_paste(byte),
        }
    }

    fn consume_ground(&mut self, byte: u8) -> KeyOutcome {
        // Mnemonic control codes that don't have shared
        // constants. Defined inline (instead of at module top)
        // because they're only meaningful inside the ground-state
        // match and naming them would imply a wider contract.
        //   SOH (Ctrl-A): jump to start of line (Home).
        //   ENQ (Ctrl-E): jump to end of line   (End).
        //   ACK (Ctrl-F): cursor right one grapheme.
        //   STX (Ctrl-B): cursor left  one grapheme.
        //   EOT (Ctrl-D): forward delete (only when buffer non-empty;
        //                  empty-buffer Ctrl-D would conventionally
        //                  exit, but the queue editor delegates exit
        //                  to the rustyline path, so we no-op
        //                  rather than risk losing a streaming turn).
        const SOH: u8 = 0x01;
        const STX: u8 = 0x02;
        const EOT: u8 = 0x04;
        const ENQ: u8 = 0x05;
        const ACK: u8 = 0x06;
        match byte {
            ESC => {
                self.state = PumpState::Escape;
                KeyOutcome::Nothing
            }
            CR | LF => {
                let trimmed = self.buf.trim().to_owned();
                self.buf.clear();
                self.cursor = 0;
                if trimmed.is_empty() {
                    // Empty Enter is a no-op visually but still
                    // needs a repaint in case the buffer had
                    // trailing whitespace that we just stripped.
                    KeyOutcome::Repaint
                } else {
                    KeyOutcome::Submit { pending: trimmed }
                }
            }
            ETX => {
                // Ctrl-C semantics, matching Codex / Claude Code:
                //   * a non-empty draft is cleared first (like Ctrl-U) and
                //     stays in the editor — the running turn is NOT cancelled,
                //     so a stray Ctrl-C while typing ahead doesn't kill the
                //     agent;
                //   * an empty draft is the "cancel the running turn" gesture,
                //     reported up as `Cancelled` so the pump flips the shared
                //     interrupt flag.
                if self.buf.is_empty() {
                    KeyOutcome::Cancelled
                } else {
                    self.buf.clear();
                    self.cursor = 0;
                    KeyOutcome::Repaint
                }
            }
            NAK => {
                if self.buf.is_empty() {
                    KeyOutcome::Nothing
                } else {
                    self.buf.clear();
                    self.cursor = 0;
                    KeyOutcome::Repaint
                }
            }
            DEL | BS_LEGACY => {
                if backspace_grapheme(&mut self.buf, &mut self.cursor) {
                    KeyOutcome::Repaint
                } else {
                    KeyOutcome::Nothing
                }
            }
            SOH => self.cursor_home(),
            ENQ => self.cursor_end(),
            STX => self.cursor_left(),
            ACK => self.cursor_right(),
            EOT => {
                if delete_grapheme_at_cursor(&mut self.buf, self.cursor) {
                    KeyOutcome::Repaint
                } else {
                    KeyOutcome::Nothing
                }
            }
            // Other control codes (0x00..=0x1f minus the ones
            // handled above) are dropped silently. They'd produce
            // unreadable junk in the painted line and most users
            // don't intentionally type them.
            byte if byte < 0x20 => KeyOutcome::Nothing,
            byte => self.consume_byte_for_codepoint(byte),
        }
    }

    /// Move the caret one grapheme cluster to the left, returning
    /// `Repaint` if anything changed (so the painter re-renders
    /// with the cursor block in its new position) or `Nothing`
    /// when we're already at the start.
    fn cursor_left(&mut self) -> KeyOutcome {
        if self.cursor == 0 {
            return KeyOutcome::Nothing;
        }
        self.cursor = prev_grapheme_boundary(&self.buf, self.cursor);
        KeyOutcome::Repaint
    }

    /// Move the caret one grapheme cluster to the right.
    fn cursor_right(&mut self) -> KeyOutcome {
        if self.cursor >= self.buf.len() {
            return KeyOutcome::Nothing;
        }
        self.cursor = next_grapheme_boundary(&self.buf, self.cursor);
        KeyOutcome::Repaint
    }

    /// Jump to the start of the buffer.
    fn cursor_home(&mut self) -> KeyOutcome {
        if self.cursor == 0 {
            KeyOutcome::Nothing
        } else {
            self.cursor = 0;
            KeyOutcome::Repaint
        }
    }

    /// Jump to the end of the buffer.
    fn cursor_end(&mut self) -> KeyOutcome {
        if self.cursor >= self.buf.len() {
            KeyOutcome::Nothing
        } else {
            self.cursor = self.buf.len();
            KeyOutcome::Repaint
        }
    }

    /// Feed one byte into the in-progress UTF-8 codepoint and, if
    /// a codepoint is complete, append it to the visible buffer.
    /// Handles both ASCII (`0x20..=0x7f`) and multi-byte sequences
    /// (`0x80..=0xff`). Invalid sequences are dropped silently —
    /// the terminal occasionally drops a stray high byte (e.g.
    /// when a paste hits us mid-codepoint) and surfacing those
    /// would replace clean rendering with U+FFFD garbage.
    fn consume_byte_for_codepoint(&mut self, byte: u8) -> KeyOutcome {
        // ASCII fast path.
        if byte < 0x80 {
            if self.utf8_expected > 0 {
                // ASCII byte mid-codepoint is malformed input
                // (e.g. a paste glitch dropped a continuation
                // byte). Drop both the half-built codepoint AND
                // the offending ASCII byte rather than smuggling
                // either half into the visible buffer.
                self.utf8_pending.clear();
                self.utf8_expected = 0;
                return KeyOutcome::Nothing;
            }
            self.insert_str(&(byte as char).to_string());
            return KeyOutcome::Repaint;
        }

        if self.utf8_expected == 0 {
            // First byte of a multi-byte sequence.
            let expected = match byte {
                0xC2..=0xDF => 2,
                0xE0..=0xEF => 3,
                0xF0..=0xF4 => 4,
                _ => {
                    // Continuation byte without a lead byte, or
                    // an invalid lead (0xC0/0xC1, 0xF5..=0xFF).
                    // Drop and stay in ground.
                    return KeyOutcome::Nothing;
                }
            };
            self.utf8_expected = expected;
            self.utf8_pending.clear();
            self.utf8_pending.push(byte);
            return KeyOutcome::Nothing;
        }

        // Continuation byte expected. Reject anything outside the
        // continuation range — a paste glitch or backend hiccup
        // shouldn't pollute the buffer.
        if !(0x80..=0xBF).contains(&byte) {
            self.utf8_pending.clear();
            self.utf8_expected = 0;
            return KeyOutcome::Nothing;
        }
        self.utf8_pending.push(byte);
        if self.utf8_pending.len() < self.utf8_expected {
            return KeyOutcome::Nothing;
        }

        // Codepoint assembled. Decode and insert at the caret.
        // Copy the decoded bytes into an owned `String` first so
        // we can drop the immutable borrow of `utf8_pending`
        // before calling `insert_str`, which needs `&mut self`.
        let outcome = match std::str::from_utf8(&self.utf8_pending) {
            Ok(decoded) => {
                let owned = decoded.to_owned();
                self.insert_str(&owned);
                KeyOutcome::Repaint
            }
            Err(_) => KeyOutcome::Nothing,
        };
        self.utf8_pending.clear();
        self.utf8_expected = 0;
        outcome
    }

    /// Insert `s` at the caret and advance the caret past it.
    /// Centralised so the ASCII path and the UTF-8 codepoint path
    /// share a single mutation site — otherwise it's easy to bump
    /// the buffer in one and forget the cursor in the other.
    fn insert_str(&mut self, s: &str) {
        debug_assert!(
            self.buf.is_char_boundary(self.cursor),
            "cursor must always sit on a codepoint boundary",
        );
        self.buf.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    fn consume_escape(&mut self, byte: u8) -> KeyOutcome {
        match byte {
            b'[' => {
                self.state = PumpState::Csi;
                // Fresh CSI: reset the param accumulator so we
                // don't carry state from a previous sequence.
                self.csi_param = None;
                self.csi_saw_semi = false;
                KeyOutcome::Nothing
            }
            // ESC ESC: drop back to ground but ignore the buffered
            // ESC. Treat the second ESC as a fresh prefix.
            ESC => {
                self.state = PumpState::Escape;
                KeyOutcome::Nothing
            }
            _ => {
                // Single-byte ESC sequence (e.g. ESC <alpha>): the
                // alt-modified key, which we don't bind. Return to
                // ground without emitting anything.
                self.state = PumpState::Ground;
                KeyOutcome::Nothing
            }
        }
    }

    fn consume_csi(&mut self, byte: u8) -> KeyOutcome {
        // CSI parameter accumulation phase. Parameters are ASCII
        // digits separated by `;`. We only track the FIRST param
        // (everything after `;` is ignored) because the keys we
        // care about — Delete (`3~`), Home (`1~`), End (`4~`),
        // PgUp/PgDn (`5~` / `6~`) — all live in single-param
        // sequences. Multi-param sequences (modifier keys like
        // Shift+Arrow `1;2D`, mouse reports) are still swallowed
        // cleanly because we never look at later params.
        if byte.is_ascii_digit() {
            if !self.csi_saw_semi {
                // Lossy clamp: terminals don't send more than 3
                // digits in practice. `saturating_*` keeps malformed
                // input from triggering an integer overflow.
                let digit = (byte - b'0') as u32;
                self.csi_param = Some(
                    self.csi_param
                        .unwrap_or(0)
                        .saturating_mul(10)
                        .saturating_add(digit),
                );
            }
            return KeyOutcome::Nothing;
        }
        if byte == b';' {
            self.csi_saw_semi = true;
            return KeyOutcome::Nothing;
        }
        // Intermediate bytes (`0x20..=0x2F`, e.g. ` ` / `!`) are
        // legal in CSI sequences but never appear in the keys we
        // handle. Pass through silently so they don't trip the
        // final-byte branch.
        if (0x20..=0x2F).contains(&byte) {
            return KeyOutcome::Nothing;
        }
        // CSI terminator: any byte in the `@…~` range closes the
        // sequence. Dispatch based on the final byte (and the
        // accumulated leading param when the final byte is `~`).
        if (0x40..=0x7e).contains(&byte) {
            let param = self.csi_param.unwrap_or(0);
            self.state = PumpState::Ground;
            self.csi_param = None;
            self.csi_saw_semi = false;
            return match byte {
                b'A' | b'B' => {
                    // Up / Down: no semantics in a single-line
                    // editor with no history. Swallow cleanly so
                    // the bytes don't pollute the buffer.
                    KeyOutcome::Nothing
                }
                b'C' => self.cursor_right(),
                b'D' => self.cursor_left(),
                b'H' => self.cursor_home(),
                b'F' => self.cursor_end(),
                b'Z' => KeyOutcome::ModeCycle,
                b'~' => match param {
                    1 | 7 => self.cursor_home(),
                    3 => {
                        if delete_grapheme_at_cursor(&mut self.buf, self.cursor) {
                            KeyOutcome::Repaint
                        } else {
                            KeyOutcome::Nothing
                        }
                    }
                    4 | 8 => self.cursor_end(),
                    // 2~ = Insert (we don't switch overwrite mode),
                    // 5~/6~ = PgUp/PgDn (no scrollback in this
                    // editor), 11..15~ = F1..F4 (no bindings).
                    _ => KeyOutcome::Nothing,
                },
                _ => KeyOutcome::Nothing,
            };
        }
        KeyOutcome::Nothing
    }

    fn consume_paste(&mut self, byte: u8) -> KeyOutcome {
        // Reserved for the bracketed-paste body. Today this state
        // is never entered (see comment in `consume_csi`); the
        // logic lives here so the future refinement is a
        // single-line change.
        match byte {
            ESC => {
                self.state = PumpState::Escape;
                KeyOutcome::Nothing
            }
            byte => {
                self.buf.push(byte as char);
                KeyOutcome::Repaint
            }
        }
    }
}

/// Render the input-row content for the current editor state.
/// Layout (color on, with non-empty buffer + 2 pending, caret at end):
///
/// ```text
/// ▎ ✎ hello world█  (Enter to queue · queue 2)
/// ```
///
/// `cursor` is the byte offset of the caret within `buf`. The
/// painter splits the buffer into prefix / suffix slices around
/// it and renders the reverse-video block in between, so moving
/// the caret with Left / Right / Home / End shows up visibly on
/// the pinned row without us having to move the real terminal
/// cursor.
///
/// When `cursor == buf.len()` the suffix is empty and the layout
/// degrades to the original "block at end" rendering. When the
/// buffer is empty both prefix and suffix are empty and we fall
/// back to the empty-state layout.
///
/// `width` is used purely for safety-truncation so we don't paint
/// an absurdly long line that wraps and breaks the pinned input
/// row. Callers usually pass the terminal width minus a small
/// margin. Pass `usize::MAX` to disable truncation.
pub fn paint_editor_line(
    buf: &str,
    cursor: usize,
    queue_depth: usize,
    width: usize,
    color: bool,
) -> String {
    // Hint text. Tone differs whether the buffer is empty (we want
    // to *teach* the user that typing queues messages) or non-empty
    // (we want to *confirm* the Enter binding only).
    let hint = if buf.is_empty() {
        if queue_depth == 0 {
            "type a message · Enter queues for the next turn".to_owned()
        } else {
            format!("type to queue more · queue {queue_depth}")
        }
    } else {
        format!("Enter queues · queue {queue_depth}")
    };

    let prompt = if color {
        format!("{PROMPT_COLOR}▎ ✎{RESET}")
    } else {
        "▎ ✎".to_owned()
    };
    // Synthetic caret. Why so loud?
    //
    // The hardware terminal cursor lives wherever the agent's
    // streaming output last advanced it (somewhere in the scroll
    // region, far above us). The ghost paint that draws this
    // input row deliberately *restores* the hardware cursor after
    // writing so streaming continues uninterrupted — which means
    // the actual blinking caret is **not** where the user is
    // editing. They have to find the cursor by visual scanning.
    //
    // A plain inverse-video space (\x1b[7m \x1b[27m) reads as
    // "background block" and is easy to lose against typed text
    // — users reported "I can't tell where the cursor is" in the
    // mid-turn editor.
    //
    // So we make the synthetic caret unmistakable:
    //
    //   * blink (SGR 5)    — mimics a real terminal caret
    //   * bold (SGR 1)     — thicker, even on low-contrast themes
    //   * yellow (SGR 33)  — stands out against default fg/bg
    //   * inverse (SGR 7)  — fills the whole cell
    //
    // Clide / iTerm2 / Terminal.app / Alacritty / Kitty all
    // honour SGR 5; terminals that don't blink at least show the
    // bright yellow block, which is still unambiguous.
    let cursor_block = if color {
        "\x1b[5;1;33;7m \x1b[0m".to_owned()
    } else {
        "[|]".to_owned()
    };
    let hint_styled = if color {
        format!("{DIM}({hint}){RESET}")
    } else {
        format!("({hint})")
    };

    if buf.is_empty() {
        return format!("{prompt} {cursor_block}  {hint_styled}");
    }

    // Truncate the buffer (only) when its terminal display WIDTH would exceed
    // the budget. We measure in display columns — not char count — so a CJK /
    // emoji buffer (each glyph 2 cells) doesn't overflow the pinned input row
    // and corrupt the layout. `display_width` ANSI-strips the prompt/hint, so
    // the styled and unstyled paths reserve the same number of columns.
    //
    // Truncation keeps the region around the caret: by default the tail (where
    // the caret sits while composing), but it slides the window backwards to
    // keep the caret visible when it's been moved earlier (Home, Left, …).
    let chrome_cols = display_width(&prompt) + display_width(&hint) + 8;
    let max_buf_cols = width.saturating_sub(chrome_cols).max(1);

    // Per-char column offsets, so we can slice the buffer on width boundaries.
    let chars: Vec<char> = buf.chars().collect();
    let widths: Vec<usize> = chars.iter().map(|&c| char_width(c)).collect();
    let total_cols: usize = widths.iter().sum();
    // Caret's column = sum of widths of chars before the byte cursor.
    let cursor_char_idx = buf[..cursor.min(buf.len())].chars().count();
    let caret_col: usize = widths[..cursor_char_idx].iter().sum();

    // Helper: collect chars in [from_char, ..] until adding the next would
    // exceed `col_budget` display columns. Returns (string, chars_consumed).
    let take_cols = |from_char: usize, col_budget: usize| -> (String, usize) {
        let mut s = String::new();
        let mut used = 0usize;
        let mut n = 0usize;
        for i in from_char..chars.len() {
            if used + widths[i] > col_budget {
                break;
            }
            s.push(chars[i]);
            used += widths[i];
            n += 1;
        }
        (s, n)
    };

    let (prefix, suffix) = if total_cols > max_buf_cols {
        // Anchor on the caret: keep it ~two-thirds across the visible window so
        // there's context on both sides. `drop_cols` is how many leading
        // display columns to hide; convert to a char index by walking widths.
        let visible = max_buf_cols;
        let target_caret_col = visible.saturating_sub(visible / 3);
        let want_drop_cols = caret_col.saturating_sub(target_caret_col);
        let max_drop_cols = total_cols.saturating_sub(visible);
        let drop_cols = want_drop_cols.min(max_drop_cols);

        // Walk to the first char whose start column is >= drop_cols (never
        // split a wide char: round to the next char boundary).
        let mut start_char = 0usize;
        let mut acc = 0usize;
        while start_char < chars.len() && acc < drop_cols {
            acc += widths[start_char];
            start_char += 1;
        }
        let start_char = start_char.min(cursor_char_idx);

        // Prefix: from window start up to the caret.
        let prefix_budget: usize = widths[start_char..cursor_char_idx].iter().sum();
        let (prefix, _) = take_cols(start_char, prefix_budget);
        // Suffix: remaining budget after the prefix, from the caret onward.
        let prefix_cols: usize = widths[start_char..cursor_char_idx].iter().sum();
        let suffix_budget = visible.saturating_sub(prefix_cols);
        let (suffix, _) = take_cols(cursor_char_idx, suffix_budget);
        (prefix, suffix)
    } else {
        let prefix: String = chars[..cursor_char_idx].iter().collect();
        let suffix: String = chars[cursor_char_idx..].iter().collect();
        (prefix, suffix)
    };

    format!("{prompt} {prefix}{cursor_block}{suffix}  {hint_styled}")
}

// ─── Termios (Unix) ────────────────────────────────────────────────

/// RAII guard that restores the original termios state when
/// dropped. Constructed by [`enter_raw_mode`] which captures the
/// current attributes before flipping ICANON / ECHO / ISIG off.
///
/// The guard is intentionally `!Send`: it's tied to the calling
/// thread's lifetime and we never move it across threads.
#[cfg(unix)]
pub struct RawModeGuard {
    fd: libc::c_int,
    original: libc::termios,
    active: bool,
}

#[cfg(unix)]
impl RawModeGuard {
    /// Restore the captured termios state explicitly. Idempotent;
    /// subsequent calls are a no-op. Also called by `Drop` so the
    /// happy path doesn't need to do anything special, but the
    /// explicit method matters for the "join thread first, then
    /// restore" ordering we use in the CLI.
    pub fn restore(&mut self) {
        if !self.active {
            return;
        }
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSANOW, &self.original as *const _);
        }
        self.active = false;
    }
}

#[cfg(unix)]
impl Drop for RawModeGuard {
    fn drop(&mut self) {
        self.restore();
    }
}

/// Place stdin into a "raw-ish" mode suitable for the per-key
/// pump: ICANON off (one-byte reads), ECHO off (no kernel-echo of
/// typed chars onto the streaming row), and IEXTEN off. We
/// deliberately leave ISIG / ICRNL / OPOST alone — we still want
/// Ctrl-C to deliver SIGINT to the main process and we still want
/// `\n` translation on output so streaming text is readable.
///
/// Returns `None` if stdin isn't a TTY or termios calls fail; the
/// caller should fall back to the legacy cooked-mode path.
#[cfg(unix)]
pub fn enter_raw_mode() -> Option<RawModeGuard> {
    use std::os::fd::AsRawFd;
    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();
    if unsafe { libc::isatty(fd) } == 0 {
        return None;
    }
    let mut original: libc::termios = unsafe { std::mem::zeroed() };
    if unsafe { libc::tcgetattr(fd, &mut original as *mut _) } != 0 {
        return None;
    }
    let mut raw = original;
    raw.c_lflag &= !(libc::ICANON | libc::ECHO | libc::IEXTEN);
    // VMIN=0, VTIME=0 → polling read: returns immediately with
    // however many bytes are available (possibly zero), never
    // blocks. We do our own blocking via `libc::poll`.
    raw.c_cc[libc::VMIN] = 0;
    raw.c_cc[libc::VTIME] = 0;
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw as *const _) } != 0 {
        return None;
    }
    Some(RawModeGuard {
        fd,
        original,
        active: true,
    })
}

#[cfg(not(unix))]
pub struct RawModeGuard;

#[cfg(not(unix))]
impl RawModeGuard {
    pub fn restore(&mut self) {}
}

#[cfg(not(unix))]
pub fn enter_raw_mode() -> Option<RawModeGuard> {
    None
}

// ─── Pump thread orchestration ─────────────────────────────────────

/// Shared signal between the main thread and the queue editor's
/// background pump. The pump checks `stop` on every tick and exits
/// when it flips true.
pub type EditorStop = Arc<AtomicBool>;

/// Build the args every pump thread needs. Bundled in a struct so
/// the spawning site reads cleanly and we don't pass eight
/// positional `Arc`s around.
pub struct EditorContext {
    pub queue: Arc<Mutex<MessageQueue>>,
    pub stop: EditorStop,
    /// Mutex shared with the spinner / streaming writers so painted
    /// updates to the input row don't interleave mid-byte with a
    /// stream delta.
    pub paint_lock: Arc<Mutex<()>>,
    /// Callback invoked on every visible state change to repaint
    /// the input row. Caller decides exactly how the row is
    /// reached (the anchored status bar owns the row geometry,
    /// not this module).
    pub repaint: Box<dyn Fn(&str) + Send>,
    pub use_color: bool,
    pub line_width: usize,
    /// When set, the pump releases raw mode, blanks its painted
    /// input row, raises `suspended`, and parks itself until the
    /// flag clears. Used by the permission prompt (and any other
    /// caller that needs exclusive cooked-mode stdin) to take
    /// over without racing the pump for bytes.
    ///
    /// Without this, the pump's raw-mode reader gobbles every
    /// byte the user types in response to `[y]es / [n]o / [t] /
    /// [a]ll-yolo`, so the agent's `read_line` never sees the
    /// answer.
    pub suspend: Arc<AtomicBool>,
    /// Set by the pump to `true` once it has actually released
    /// raw mode in response to `suspend`. Callers poll this flag
    /// (with a short timeout) to know when it's safe to read
    /// from stdin in cooked mode. Cleared by the pump once it
    /// re-enters raw mode on resume.
    pub suspended: Arc<AtomicBool>,
    /// Raised by the pump when the user presses Shift+Tab
    /// mid-turn. The CLI consumes this signal either inside the
    /// next permission prompt (as a quick-yolo shortcut) or at
    /// the next turn boundary (cycling to the next mode like
    /// the rustyline binding does between turns).
    ///
    /// Stays raised until consumed — pressing Shift+Tab twice in
    /// a row before consumption is a no-op (the second press
    /// just re-sets a flag that's already true).
    pub mode_cycle: Arc<AtomicBool>,
    /// Cooperative cancellation flag, shared with the agent loop / backend /
    /// tool context. The pump flips it to `true` when the user presses Ctrl-C
    /// on an empty draft during a turn, which aborts the in-flight model stream
    /// and any running tool. The CLI resets it at the start of each turn.
    pub interrupt: Arc<AtomicBool>,
}

#[cfg(unix)]
/// Block (with a timeout) until stdin has at least one byte ready.
/// Returns:
///   * `Ok(true)`  — data ready, caller should `read()`
///   * `Ok(false)` — timed out, caller should re-check `stop`
///   * `Err(_)`    — transient EINTR or similar; caller retries
///
/// Shared with [`crate::prompt_editor`]'s between-turns read loop.
pub(crate) fn poll_stdin(timeout_ms: i32) -> std::io::Result<bool> {
    use std::os::fd::AsRawFd;
    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    let ret = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
    if ret < 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(ret > 0 && (pfd.revents & libc::POLLIN) != 0)
}

#[cfg(unix)]
/// Read up to `cap` bytes from stdin without blocking. Returns
/// `Ok(0)` when no data was ready (caller polls again). Shared with
/// [`crate::prompt_editor`]'s between-turns read loop.
pub(crate) fn read_burst(buf: &mut [u8]) -> std::io::Result<usize> {
    use std::os::fd::AsRawFd;
    let fd = std::io::stdin().as_raw_fd();
    let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
    if n < 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EAGAIN) || err.kind() == std::io::ErrorKind::Interrupted
        {
            return Ok(0);
        }
        return Err(err);
    }
    Ok(n as usize)
}

#[cfg(unix)]
/// Drive a single editor turn until `stop` flips true. Blocks the
/// calling thread. Returns control after `stop` is observed; the
/// final state of the editor (if the user was mid-line when the
/// turn ended) is dropped — those bytes are lost the same way an
/// uncommitted rustyline buffer is at Ctrl-C.
///
/// `guard` is the raw-mode guard the caller acquired via
/// [`enter_raw_mode`]; ownership is transferred so the pump can
/// release / re-acquire raw mode mid-turn in response to the
/// `suspend` signal (e.g. while the agent's permission prompt is
/// reading cooked-mode stdin). The guard is restored on every
/// exit path including suspend transitions and panic via `Drop`.
pub fn run_pump(ctx: EditorContext, mut guard: RawModeGuard) {
    let mut editor = QueueEditor::new();
    let mut buf = [0u8; 64];
    let debug_log = open_debug_log();
    let mut have_raw = true;

    // Initial paint so the affordance shows up immediately, even
    // before the user types a single byte.
    paint(&ctx, &editor);

    while !ctx.stop.load(Ordering::Relaxed) {
        // Handle the suspend handshake first so callers that need
        // exclusive stdin (the agent's permission prompt is the
        // canonical case) can take over within ~20ms of raising
        // `suspend`. We:
        //   1. Blank the input row so the cooked-mode prompt has
        //      a clean line to print on.
        //   2. Drop the raw-mode guard, restoring the terminal's
        //      original termios (ICANON / ECHO / IEXTEN back on).
        //   3. Raise `suspended` to acknowledge release.
        //   4. Park in a 20ms-poll loop until `suspend` clears
        //      (or `stop` fires, in which case we exit cleanly
        //      with raw mode already restored — the explicit
        //      drop path below skips re-restore for an already
        //      inactive guard).
        //   5. Re-acquire raw mode and repaint the editor.
        if ctx.suspend.load(Ordering::Relaxed) {
            // Step 1: blank our painted line so the cooked-mode
            // prompt doesn't have to fight with a stale
            // `▎ ✎ <buf>█` ghost. Best-effort under the paint lock.
            if let Ok(_g) = ctx.paint_lock.lock() {
                (ctx.repaint)("");
            }
            // Step 2: release raw mode.
            if have_raw {
                guard.restore();
                have_raw = false;
            }
            // Step 3: ack release.
            ctx.suspended.store(true, Ordering::Relaxed);

            // Step 4: park until resume (or stop).
            while ctx.suspend.load(Ordering::Relaxed) && !ctx.stop.load(Ordering::Relaxed) {
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            ctx.suspended.store(false, Ordering::Relaxed);
            if ctx.stop.load(Ordering::Relaxed) {
                break;
            }
            // Step 5: re-acquire raw mode. If the second attempt
            // fails (rare — termios on the original fd hasn't gone
            // anywhere), exit gracefully rather than spin-looping
            // forever in cooked mode pretending to be raw.
            match enter_raw_mode() {
                Some(new_guard) => {
                    guard = new_guard;
                    have_raw = true;
                    paint(&ctx, &editor);
                }
                None => break,
            }
            continue;
        }

        match poll_stdin(50) {
            Ok(true) => {}
            Ok(false) => continue,
            Err(_) => continue,
        }
        let n = match read_burst(&mut buf) {
            Ok(0) => continue,
            Ok(n) => n,
            Err(_) => break, // unrecoverable stdin error
        };
        if let Some(file) = debug_log.as_ref() {
            log_burst(file, &buf[..n]);
        }
        let mut dirty = false;
        for &byte in &buf[..n] {
            match editor.consume(byte) {
                KeyOutcome::Repaint => dirty = true,
                KeyOutcome::Submit { pending } => {
                    let depth = match ctx.queue.lock() {
                        Ok(mut q) => {
                            q.push(pending);
                            q.len()
                        }
                        Err(_) => 0,
                    };
                    let _ = depth;
                    dirty = true;
                }
                KeyOutcome::Cancelled => {
                    // Empty-draft Ctrl-C during a turn: request cancellation.
                    // The agent loop's between-step / post-tool checks and the
                    // streaming between-events check observe this flag and
                    // unwind the turn; the tool poll loops kill any running
                    // child. Paint a brief acknowledgement so the press
                    // registers; the turn's terminal "⎿ Interrupted by user"
                    // line follows once the loop returns.
                    ctx.interrupt.store(true, Ordering::Relaxed);
                    (ctx.repaint)("⎿ cancelling… (Ctrl-C)");
                    dirty = false;
                }
                KeyOutcome::ModeCycle => {
                    // Surface the Shift+Tab intent to the CLI via
                    // the shared atomic. The user-visible mode
                    // change happens on the main thread — either
                    // when the next permission prompt fires, or
                    // after `repl.submit()` returns at turn
                    // boundary. We mark the line dirty so the
                    // hint can update if the painter cares about
                    // pending cycles (currently it doesn't, but
                    // the flag is free).
                    ctx.mode_cycle.store(true, Ordering::Relaxed);
                    dirty = true;
                }
                KeyOutcome::Nothing => {}
            }
        }
        if dirty {
            paint(&ctx, &editor);
        }
    }

    // Belt + suspenders: if we exited the loop while suspended
    // (stop fired during the park), make sure the ack flag is
    // false so future callers don't see a phantom "already
    // suspended" signal from a fresh editor instance.
    ctx.suspended.store(false, Ordering::Relaxed);
    // Restoration of the guard is handled by Drop; the
    // `have_raw` bookkeeping above just prevents a double-restore.
    let _ = have_raw;
}

/// Open the byte-stream debug log when the user has set
/// `DEEPTIDE_QUEUE_EDITOR_DEBUG` to a non-empty file path. Used
/// for diagnosing IME / terminal-emulator quirks (e.g. why CJK
/// input on macOS requires N Backspaces): the log records the
/// exact bytes our raw-mode reader received, byte-by-byte, with
/// timestamps and hex+ASCII columns. Returns `None` when the env
/// var is unset / empty / the file can't be opened, so the
/// happy-path zero-overhead behaviour is unchanged.
fn open_debug_log() -> Option<std::fs::File> {
    let path = std::env::var_os("DEEPTIDE_QUEUE_EDITOR_DEBUG")?;
    if path.is_empty() {
        return None;
    }
    std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .ok()
}

/// Append one read burst to the debug log as
/// `[<elapsed_us>] N bytes: hex hex hex … | ascii…`. Errors are
/// swallowed — debug logging must never affect the live UX.
fn log_burst(mut file: &std::fs::File, bytes: &[u8]) {
    use std::io::Write;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or(0);
    let hex = bytes
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ");
    let ascii = bytes
        .iter()
        .map(|&b| {
            if (0x20..0x7f).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect::<String>();
    let _ = writeln!(
        file,
        "[{stamp}] {n} bytes: {hex} | {ascii}",
        n = bytes.len()
    );
}

#[cfg(not(unix))]
pub fn run_pump(_ctx: EditorContext, _guard: RawModeGuard) {
    // No-op on non-Unix. Callers should not invoke this when
    // `enter_raw_mode()` returns None.
}

fn paint(ctx: &EditorContext, editor: &QueueEditor) {
    let depth = ctx.queue.lock().map(|q| q.len()).unwrap_or(0);
    let line = paint_editor_line(
        editor.buffer(),
        editor.cursor(),
        depth,
        ctx.line_width,
        ctx.use_color,
    );
    if let Ok(_guard) = ctx.paint_lock.lock() {
        (ctx.repaint)(&line);
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::time::Duration;

    #[test]
    fn editor_context_carries_suspend_handshake_fields() {
        // Compile-time + structural check: the fields the
        // permission-prompt path depends on are present and have
        // the documented atomic-bool semantics. Constructing the
        // context inline (rather than via a helper) keeps the
        // tuple-arity small enough for clippy::type-complexity.
        let queue = Arc::new(Mutex::new(MessageQueue::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let suspend = Arc::new(AtomicBool::new(false));
        let suspended = Arc::new(AtomicBool::new(false));
        let mode_cycle = Arc::new(AtomicBool::new(false));
        let ctx = EditorContext {
            queue,
            stop,
            paint_lock: Arc::new(Mutex::new(())),
            repaint: Box::new(|_| {}),
            use_color: false,
            line_width: 80,
            suspend: Arc::clone(&suspend),
            suspended: Arc::clone(&suspended),
            mode_cycle: Arc::clone(&mode_cycle),
            interrupt: Arc::new(AtomicBool::new(false)),
        };
        assert!(!suspend.load(Ordering::Relaxed));
        assert!(!suspended.load(Ordering::Relaxed));
        suspend.store(true, Ordering::Relaxed);
        assert!(ctx.suspend.load(Ordering::Relaxed));
        suspend.store(false, Ordering::Relaxed);
        suspended.store(true, Ordering::Relaxed);
        assert!(ctx.suspended.load(Ordering::Relaxed));
        mode_cycle.store(true, Ordering::Relaxed);
        assert!(ctx.mode_cycle.load(Ordering::Relaxed));
        let _ = Duration::from_millis(0); // suppress unused-import warning
    }

    #[test]
    fn shift_tab_csi_z_returns_mode_cycle_outcome() {
        // xterm/iTerm/macOS Terminal/etc. send `ESC [ Z` for
        // Shift+Tab. The first two bytes drive the state
        // machine into Csi; the terminator `Z` should yield
        // KeyOutcome::ModeCycle, not silently swallow.
        let mut ed = QueueEditor::new();
        assert_eq!(ed.consume(0x1b), KeyOutcome::Nothing); // ESC
        assert_eq!(ed.consume(b'['), KeyOutcome::Nothing); // CSI introducer
        assert_eq!(ed.consume(b'Z'), KeyOutcome::ModeCycle);
        // Buffer must remain pristine — Shift+Tab is a control
        // gesture, not text input.
        assert!(ed.buffer().is_empty());
    }

    #[test]
    fn shift_tab_does_not_disturb_existing_buffer_or_state() {
        // Pressing Shift+Tab while typing a draft must not
        // corrupt the in-progress buffer. After the gesture,
        // subsequent ASCII bytes resume appending normally.
        let mut ed = QueueEditor::new();
        ed.seed("hello");
        assert_eq!(ed.consume(0x1b), KeyOutcome::Nothing);
        assert_eq!(ed.consume(b'['), KeyOutcome::Nothing);
        assert_eq!(ed.consume(b'Z'), KeyOutcome::ModeCycle);
        assert_eq!(ed.buffer(), "hello");
        assert_eq!(ed.consume(b'!'), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "hello!");
    }

    #[test]
    fn csi_with_other_terminators_still_swallows_without_mode_cycle() {
        // Sanity check: non-Z CSI sequences (arrow keys,
        // function keys, mouse tracking) must NOT spuriously
        // trigger mode cycle.
        let mut ed = QueueEditor::new();
        // Up arrow: ESC [ A
        assert_eq!(ed.consume(0x1b), KeyOutcome::Nothing);
        assert_eq!(ed.consume(b'['), KeyOutcome::Nothing);
        assert_eq!(ed.consume(b'A'), KeyOutcome::Nothing);
        assert!(ed.buffer().is_empty());
    }

    #[test]
    fn ground_appends_visible_chars() {
        let mut ed = QueueEditor::new();
        assert_eq!(ed.consume(b'h'), KeyOutcome::Repaint);
        assert_eq!(ed.consume(b'i'), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "hi");
    }

    #[test]
    fn enter_submits_trimmed_and_clears() {
        let mut ed = QueueEditor::new();
        ed.seed("  hello world  ");
        match ed.consume(b'\n') {
            KeyOutcome::Submit { pending } => assert_eq!(pending, "hello world"),
            other => panic!("expected Submit, got {other:?}"),
        }
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn empty_enter_clears_buffer_and_yields_repaint() {
        let mut ed = QueueEditor::new();
        ed.seed("   ");
        assert_eq!(ed.consume(b'\r'), KeyOutcome::Repaint);
        assert!(ed.buffer().is_empty());
    }

    #[test]
    fn backspace_pops_one_char_then_no_ops_at_empty() {
        let mut ed = QueueEditor::new();
        ed.seed("ab");
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "a");
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "");
        assert_eq!(ed.consume(DEL), KeyOutcome::Nothing);
    }

    #[test]
    fn ctrl_h_acts_as_backspace_alias() {
        let mut ed = QueueEditor::new();
        ed.seed("abc");
        assert_eq!(ed.consume(BS_LEGACY), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "ab");
    }

    #[test]
    fn ctrl_u_clears_buffer_when_non_empty() {
        let mut ed = QueueEditor::new();
        ed.seed("hello");
        assert_eq!(ed.consume(NAK), KeyOutcome::Repaint);
        assert!(ed.buffer().is_empty());
        // Idempotent on empty buffer.
        assert_eq!(ed.consume(NAK), KeyOutcome::Nothing);
    }

    #[test]
    fn ctrl_c_on_nonempty_draft_clears_without_cancelling() {
        // Ctrl-C with text in the draft clears it (like Ctrl-U) and stays in
        // the editor — it must NOT signal Cancelled, so a stray Ctrl-C while
        // typing ahead never kills the running turn.
        let mut ed = QueueEditor::new();
        ed.seed("oops");
        assert_eq!(ed.consume(ETX), KeyOutcome::Repaint);
        assert!(ed.buffer().is_empty());
    }

    #[test]
    fn ctrl_c_on_empty_draft_signals_cancelled() {
        // Empty-draft Ctrl-C is the "cancel the running turn" gesture.
        let mut ed = QueueEditor::new();
        assert_eq!(ed.consume(ETX), KeyOutcome::Cancelled);
    }

    #[test]
    fn control_codes_other_than_known_keys_are_dropped() {
        let mut ed = QueueEditor::new();
        // 0x05 is Ctrl-E (move to end), we don't support it: must
        // not corrupt the buffer.
        assert_eq!(ed.consume(0x05), KeyOutcome::Nothing);
        assert!(ed.buffer().is_empty());
    }

    #[test]
    fn csi_sequence_is_swallowed_without_polluting_buffer() {
        // ESC [ A is "Arrow Up". The user pressed an arrow key
        // they didn't realise wasn't supported — the bytes should
        // vanish, not appear as `^[[A` in the queue.
        let mut ed = QueueEditor::new();
        assert_eq!(ed.consume(ESC), KeyOutcome::Nothing);
        assert_eq!(ed.consume(b'['), KeyOutcome::Nothing);
        assert_eq!(ed.consume(b'A'), KeyOutcome::Nothing);
        assert!(ed.buffer().is_empty());
        // Editor is back in ground state and can take input.
        assert_eq!(ed.consume(b'x'), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "x");
    }

    #[test]
    fn alt_modified_keys_are_consumed_silently() {
        // ESC followed by a single letter = Alt-letter (Meta).
        // We don't bind these; they should leave the buffer alone.
        let mut ed = QueueEditor::new();
        ed.seed("keep");
        assert_eq!(ed.consume(ESC), KeyOutcome::Nothing);
        assert_eq!(ed.consume(b'b'), KeyOutcome::Nothing);
        assert_eq!(ed.buffer(), "keep");
    }

    #[test]
    fn paint_editor_line_empty_buffer_shows_teach_hint() {
        let painted = paint_editor_line("", 0, 0, 100, false);
        assert!(painted.contains("type a message"), "got: {painted}");
        assert!(painted.contains("queues for the next turn"));
    }

    #[test]
    fn paint_editor_line_empty_buffer_with_pending_swaps_hint() {
        let painted = paint_editor_line("", 0, 3, 100, false);
        assert!(painted.contains("queue 3"), "got: {painted}");
        assert!(painted.contains("type to queue more"));
    }

    #[test]
    fn paint_editor_line_with_buffer_shows_enter_hint_and_depth() {
        let painted = paint_editor_line("hello", 5, 2, 100, false);
        assert!(painted.contains("hello"));
        assert!(painted.contains("Enter queues"));
        assert!(painted.contains("queue 2"));
    }

    #[test]
    fn paint_editor_line_color_emits_sgr() {
        let painted = paint_editor_line("hi", 2, 0, 100, true);
        assert!(
            painted.contains("\x1b[1;36m"),
            "missing prompt color: {painted}"
        );
        assert!(painted.contains("\x1b[2m"), "missing dim hint: {painted}");
        // Synthetic caret uses blink + bold + yellow + inverse
        // (SGR 5;1;33;7). Verify the full combo because we rely on
        // it being unmistakable even when the hardware cursor is
        // elsewhere — a missing SGR here is a regression in the
        // user-visible caret.
        assert!(
            painted.contains("\x1b[5;1;33;7m"),
            "missing blinking yellow caret SGR: {painted:?}"
        );
    }

    #[test]
    fn paint_editor_line_caret_uses_blink_inverse_yellow_in_color_mode() {
        // Pin the synthetic-caret style explicitly so any
        // accidental SGR tweak fails this test rather than
        // silently degrading caret visibility.
        let painted = paint_editor_line("abc", 1, 0, 100, true);
        assert!(
            painted.contains("\x1b[5;1;33;7m \x1b[0m"),
            "caret must be blink+bold+yellow+inverse space: {painted:?}"
        );
    }

    #[test]
    fn paint_editor_line_caret_no_color_uses_visible_marker() {
        // Without color, fall back to a textual marker that still
        // visually breaks up the typed text so the user can spot
        // the caret position. A bare underscore reads ambiguously
        // ("oh, did they type that?"); brackets make the intent
        // obvious.
        let painted = paint_editor_line("abc", 1, 0, 100, false);
        assert!(
            painted.contains("[|]"),
            "no-color caret should use a [|] marker: {painted:?}"
        );
        // And it must NOT leak any SGR opening bytes when color
        // is off.
        assert!(
            !painted.contains("\x1b["),
            "no-color path must be SGR-free: {painted:?}"
        );
    }

    #[test]
    fn paint_editor_line_truncates_long_buffer_keeping_tail() {
        // 80-char-wide budget. The buffer is ~200 chars; we should
        // keep the tail so the user's caret context is preserved.
        let long_buf: String = "abcde".repeat(40);
        let painted = paint_editor_line(&long_buf, long_buf.len(), 0, 80, false);
        // The very first chars of `long_buf` should be gone.
        assert!(
            !painted.contains(&long_buf[..50]),
            "expected head trimmed: {painted}"
        );
        // Last few chars (the simulated caret context) must remain.
        let tail = &long_buf[long_buf.len() - 10..];
        assert!(painted.contains(tail), "tail must be visible: {painted}");
    }

    #[test]
    fn paint_editor_line_mid_buffer_cursor_renders_prefix_block_suffix() {
        // Caret in the middle of "hello": the painter must split
        // around it so the user *sees* their navigation. Without
        // this, Left arrow looks like a no-op to the user even
        // though the internal cursor advanced.
        let painted = paint_editor_line("hello", 2, 0, 200, true);
        let prefix_idx = painted.find("he").expect("prefix present");
        let suffix_idx = painted.find("llo").expect("suffix present");
        // Find the blinking yellow caret. We assert on the FULL
        // SGR open sequence so a future tweak to the caret style
        // forces an explicit test update rather than silently
        // breaking visibility.
        let block_idx = painted
            .find("\x1b[5;1;33;7m")
            .expect("cursor block present");
        assert!(
            prefix_idx < block_idx && block_idx < suffix_idx,
            "expected he | █ | llo ordering: {painted:?}",
        );
    }

    #[test]
    fn paint_editor_line_cursor_at_zero_renders_block_before_buffer() {
        let painted = paint_editor_line("hello", 0, 0, 200, true);
        // No prefix chars before the cursor block; the buffer
        // body should sit AFTER the blinking yellow caret.
        let block_idx = painted.find("\x1b[5;1;33;7m").expect("block present");
        let body_idx = painted.find("hello").expect("body present");
        assert!(
            block_idx < body_idx,
            "Home cursor should render block before body: {painted:?}",
        );
    }

    #[test]
    fn paint_editor_line_long_buffer_with_caret_in_head_keeps_caret_visible() {
        // 80-col budget, ~200-char buffer of unique chars
        // (alphabet repeated), caret near the start. The previous
        // "drop from head" truncation would hide the caret
        // completely; the new anchor-on-caret logic must keep it
        // inside the visible window.
        //
        // Cycling A..Z a..z over 200 chars makes any tail *substring* recur in
        // the head, so we append a genuinely unique sentinel at the very end
        // and assert THAT is trimmed — robust regardless of where the window
        // boundary lands. (The old fixture relied on `prompt.chars().count()`
        // over-counting the ANSI escape, which shrank the window enough to hide
        // the recurring tail; measuring real display width exposed that, so we
        // make the assertion not depend on the exact window size.)
        let mut long_buf: String = (0..180)
            .map(|i| {
                let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
                table[i % table.len()] as char
            })
            .collect();
        long_buf.push_str("UNIQUE_TAIL_SENTINEL_~~~");
        let painted = paint_editor_line(&long_buf, 10, 0, 80, true);
        // The cursor block must be present somewhere in the
        // visible line (we never trim the block itself).
        assert!(
            painted.contains("\x1b[5;1;33;7m"),
            "cursor block must remain visible after truncation: {painted:?}",
        );
        // The unique tail sentinel must NOT appear — caret is near the start,
        // so the painter anchors there and the far tail is trimmed.
        assert!(
            !painted.contains("UNIQUE_TAIL_SENTINEL"),
            "with caret near start, tail should be trimmed: {painted:?}",
        );
        // Conversely, the head context AROUND the caret should
        // be visible (chars [0..10] for caret at offset 10).
        let head_context = &long_buf[..10];
        assert!(
            painted.contains(head_context),
            "head context near caret must remain visible: {painted:?}",
        );
    }

    #[test]
    fn paint_editor_line_truncates_cjk_buffer_on_display_width_not_char_count() {
        use deeptide_core::width::display_width;
        // A buffer of CJK (each glyph 2 cells) longer than fits. With char-count
        // budgeting this would reserve twice the columns it should and overflow
        // the pinned input row; with display-width budgeting the painted line's
        // visible width stays within the terminal budget.
        let cjk: String = "你好世界".repeat(20); // 80 chars, 160 cells
        let width = 40;
        let painted = paint_editor_line(&cjk, cjk.len(), 0, width, false);
        // No-color path: strip the bracket caret marker before measuring.
        let visible = painted.replace("[|]", "");
        assert!(
            display_width(&visible) <= width,
            "CJK input must be truncated to the terminal width ({width}); got {} cols: {painted:?}",
            display_width(&visible)
        );
        // It must NOT have kept all 20 repeats (that would be 160 cells).
        assert!(
            painted.matches('你').count() < 20,
            "long CJK buffer should be truncated, not rendered whole: {painted:?}"
        );
    }

    #[test]
    fn paint_editor_line_cjk_caret_at_zero_shows_head_not_overflowing() {
        use deeptide_core::width::display_width;
        let cjk: String = "中文输入测试".repeat(10); // 60 chars, 120 cells
        let width = 30;
        let painted = paint_editor_line(&cjk, 0, 0, width, false);
        let visible = painted.replace("[|]", "");
        assert!(
            display_width(&visible) <= width,
            "caret-at-zero CJK render must fit width {width}: {painted:?}"
        );
    }

    #[test]
    fn utf8_multibyte_chars_round_trip_then_backspace_removes_full_codepoint() {
        // CJK input: 你 (E4 BD A0) + 好 (E5 A5 BD) are each
        // 3-byte UTF-8 codepoints. The editor should emit
        // `Nothing` for every byte except the codepoint-closing
        // one, which fires a single `Repaint`. Backspace must
        // then remove one full codepoint per press.
        let mut ed = QueueEditor::new();
        let bytes = "你好".as_bytes();
        assert_eq!(bytes.len(), 6, "sanity: 2 CJK codepoints = 6 bytes");
        for (idx, byte) in bytes.iter().enumerate() {
            let expected = if idx == 2 || idx == 5 {
                KeyOutcome::Repaint
            } else {
                KeyOutcome::Nothing
            };
            assert_eq!(ed.consume(*byte), expected, "byte index {idx}");
        }
        assert_eq!(ed.buffer(), "你好");
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "你");
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn backspace_removes_variation_selector_with_base_in_one_press() {
        // macOS IMEs often commit `<base><VS16>` for CJK / emoji
        // input; a naive Backspace pops only the invisible VS16
        // and the user sees no visible change. The grapheme-aware
        // Backspace must treat the pair as one cluster and clear
        // both.
        let mut ed = QueueEditor::new();
        ed.seed("\u{2764}\u{FE0F}"); // ❤️ = U+2764 + VS16
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert_eq!(
            ed.buffer(),
            "",
            "one Backspace must clear base + variation selector"
        );
    }

    #[test]
    fn backspace_removes_zwj_emoji_sequence_in_one_press() {
        // 👨‍👩‍👧 (family) is base + ZWJ + base + ZWJ + base.
        // The visible cluster the user perceives is a single
        // glyph. Backspace should peel off everything up to the
        // previous "visible boundary" — for our heuristic that
        // means: pop the last codepoint, then any ZWJ modifiers,
        // then ONE more codepoint. After one Backspace on this
        // sequence we should be down to the first 👨 + ZWJ + 👩.
        let mut ed = QueueEditor::new();
        ed.seed("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}");
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        // After: 👨 + ZWJ + 👩 + ZWJ remain (popped 👧 + the
        // ZWJ that bound it).
        assert_eq!(ed.buffer(), "\u{1F468}\u{200D}\u{1F469}\u{200D}");
    }

    #[test]
    fn backspace_removes_combining_mark_with_base_in_one_press() {
        // "é" via decomposed form `e` + U+0301 (combining acute).
        let mut ed = QueueEditor::new();
        ed.seed("e\u{0301}");
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn backspace_after_simple_cjk_clears_in_one_press_no_regression() {
        // The plain case (no invisible modifier) must still work
        // with a single Backspace — we don't want the smart
        // grapheme logic to over-eat past the visible char.
        let mut ed = QueueEditor::new();
        ed.seed("你");
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn ascii_after_backspace_still_renders_and_appends() {
        // The user-reported pathology: after one Backspace, an
        // ASCII char "doesn't render". The fix is grapheme-aware
        // Backspace; verify the buffer accepts the follow-up
        // byte cleanly.
        let mut ed = QueueEditor::new();
        ed.seed("\u{2764}\u{FE0F}"); // ❤️
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert!(ed.buffer().is_empty());
        assert_eq!(ed.consume(b'a'), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "a");
    }

    #[test]
    fn backspace_on_lone_modifier_strips_just_the_modifier() {
        // Degenerate sequence: a modifier with no preceding base.
        // `backspace_grapheme` should still remove it (the first
        // pop), and then have nothing to fall back on.
        let mut ed = QueueEditor::new();
        ed.seed("\u{FE0F}");
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn malformed_utf8_sequence_is_dropped_without_corrupting_buffer() {
        let mut ed = QueueEditor::new();
        ed.seed("ok");
        // 0xC3 is a valid 2-byte lead. Following with a
        // non-continuation byte (e.g. 'a' = 0x61) must abort the
        // partial codepoint and NOT push garbage.
        assert_eq!(ed.consume(0xC3), KeyOutcome::Nothing);
        assert_eq!(ed.consume(b'a'), KeyOutcome::Nothing);
        // Buffer must still be "ok" — neither half-byte nor stray
        // 'a' got pushed.
        assert_eq!(ed.buffer(), "ok");
    }

    // ─── Cursor navigation ──────────────────────────────────────

    /// Drive a CSI sequence (`ESC [ <bytes…>`) into the editor and
    /// assert that the FINAL byte produces `expected`. Helper for
    /// the navigation tests — without it every test repeats three
    /// `consume(...)` calls and the actual assertion gets lost in
    /// the noise.
    fn csi(ed: &mut QueueEditor, bytes: &[u8]) -> KeyOutcome {
        let (final_byte, lead) = bytes.split_last().expect("non-empty CSI");
        assert_eq!(ed.consume(ESC), KeyOutcome::Nothing);
        assert_eq!(ed.consume(b'['), KeyOutcome::Nothing);
        for b in lead {
            assert_eq!(ed.consume(*b), KeyOutcome::Nothing, "param byte {b:?}");
        }
        ed.consume(*final_byte)
    }

    #[test]
    fn fresh_editor_has_cursor_at_zero() {
        let ed = QueueEditor::new();
        assert_eq!(ed.cursor(), 0);
    }

    #[test]
    fn typing_advances_cursor_to_end() {
        let mut ed = QueueEditor::new();
        for byte in b"hi" {
            assert_eq!(ed.consume(*byte), KeyOutcome::Repaint);
        }
        assert_eq!(ed.buffer(), "hi");
        assert_eq!(ed.cursor(), 2);
    }

    #[test]
    fn left_arrow_moves_cursor_back_one_grapheme() {
        let mut ed = QueueEditor::new();
        ed.seed("abc");
        assert_eq!(ed.cursor(), 3);
        assert_eq!(csi(&mut ed, b"D"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 2);
        assert_eq!(csi(&mut ed, b"D"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 1);
    }

    #[test]
    fn right_arrow_moves_cursor_forward_one_grapheme() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("abc", 1);
        assert_eq!(csi(&mut ed, b"C"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 2);
        assert_eq!(csi(&mut ed, b"C"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 3);
        // Past the end is a no-op rather than overshoot.
        assert_eq!(csi(&mut ed, b"C"), KeyOutcome::Nothing);
        assert_eq!(ed.cursor(), 3);
    }

    #[test]
    fn left_arrow_at_start_is_noop() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("hi", 0);
        assert_eq!(csi(&mut ed, b"D"), KeyOutcome::Nothing);
        assert_eq!(ed.cursor(), 0);
    }

    #[test]
    fn left_right_step_over_cjk_codepoint_in_one_press() {
        // A 3-byte CJK codepoint must look like ONE cursor step
        // — the user pressed Left once, the visible caret should
        // jump one glyph, not one byte.
        let mut ed = QueueEditor::new();
        ed.seed("a你b");
        // Buffer layout: a (1) | 你 (3) | b (1) = 5 bytes total.
        assert_eq!(ed.buffer().len(), 5);
        assert_eq!(ed.cursor(), 5);
        assert_eq!(csi(&mut ed, b"D"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 4, "step over 'b'");
        assert_eq!(csi(&mut ed, b"D"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 1, "step over '你' (3 bytes) in one press");
        assert_eq!(csi(&mut ed, b"D"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 0, "step over 'a'");
    }

    #[test]
    fn left_arrow_steps_over_zwj_emoji_cluster_in_one_press() {
        // 👨‍👩‍👧 (family) — caret should land before the whole
        // cluster after one Left press, not in the middle of a
        // ZWJ run.
        let mut ed = QueueEditor::new();
        ed.seed("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}");
        let total = ed.buffer().len();
        assert_eq!(ed.cursor(), total);
        assert_eq!(csi(&mut ed, b"D"), KeyOutcome::Repaint);
        // After one Left: skipped the final base (👧) and its
        // attached ZWJ — matching the symmetric Backspace
        // behaviour the older test pins down.
        let expected = "\u{1F468}\u{200D}\u{1F469}\u{200D}".len();
        assert_eq!(ed.cursor(), expected);
    }

    #[test]
    fn home_jumps_to_start_and_end_jumps_to_end() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("hello world", 5);
        // CSI H = Home
        assert_eq!(csi(&mut ed, b"H"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 0);
        // No-op when already at start.
        assert_eq!(csi(&mut ed, b"H"), KeyOutcome::Nothing);
        // CSI F = End
        assert_eq!(csi(&mut ed, b"F"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), ed.buffer().len());
        assert_eq!(csi(&mut ed, b"F"), KeyOutcome::Nothing);
    }

    #[test]
    fn ctrl_a_and_ctrl_e_mirror_home_and_end() {
        // Emacs-style shortcuts. Some terminal users press these
        // by reflex; supporting them is free and matches
        // rustyline's idle-prompt behaviour.
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("abc", 2);
        assert_eq!(ed.consume(0x01), KeyOutcome::Repaint); // Ctrl-A
        assert_eq!(ed.cursor(), 0);
        assert_eq!(ed.consume(0x05), KeyOutcome::Repaint); // Ctrl-E
        assert_eq!(ed.cursor(), 3);
    }

    #[test]
    fn ctrl_b_and_ctrl_f_mirror_left_and_right() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("abc", 1);
        assert_eq!(ed.consume(0x06), KeyOutcome::Repaint); // Ctrl-F (right)
        assert_eq!(ed.cursor(), 2);
        assert_eq!(ed.consume(0x02), KeyOutcome::Repaint); // Ctrl-B (left)
        assert_eq!(ed.cursor(), 1);
    }

    #[test]
    fn home_via_csi_one_tilde_works() {
        // Some terminals (xterm-style "VT220") encode Home as
        // `ESC [ 1 ~` instead of `ESC [ H`. We must accept both.
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("xyz", 3);
        assert_eq!(csi(&mut ed, b"1~"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 0);
    }

    #[test]
    fn end_via_csi_four_tilde_works() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("xyz", 0);
        assert_eq!(csi(&mut ed, b"4~"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 3);
    }

    #[test]
    fn delete_via_csi_three_tilde_removes_grapheme_at_cursor() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("abcd", 1);
        assert_eq!(csi(&mut ed, b"3~"), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "acd", "Delete removed 'b'");
        assert_eq!(ed.cursor(), 1, "caret stayed in place");
    }

    #[test]
    fn delete_at_end_is_noop() {
        let mut ed = QueueEditor::new();
        ed.seed("abc");
        assert_eq!(csi(&mut ed, b"3~"), KeyOutcome::Nothing);
        assert_eq!(ed.buffer(), "abc");
    }

    #[test]
    fn delete_removes_cjk_codepoint_in_one_press() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("a你b", 1);
        assert_eq!(csi(&mut ed, b"3~"), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "ab", "Delete removed '你'");
        assert_eq!(ed.cursor(), 1);
    }

    #[test]
    fn delete_removes_base_plus_variation_selector_in_one_press() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("X\u{2764}\u{FE0F}Y", 1); // X|❤️Y
        assert_eq!(csi(&mut ed, b"3~"), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "XY");
    }

    #[test]
    fn insert_in_middle_splits_existing_buffer() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("ad", 1);
        assert_eq!(ed.consume(b'b'), KeyOutcome::Repaint);
        assert_eq!(ed.consume(b'c'), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "abcd");
        assert_eq!(ed.cursor(), 3);
    }

    #[test]
    fn insert_in_middle_with_cjk_keeps_codepoint_boundaries() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("a好c", 1);
        // Type 你 (3 bytes) into the middle.
        for byte in "你".as_bytes() {
            ed.consume(*byte);
        }
        assert_eq!(ed.buffer(), "a你好c");
        // Cursor is between 你 and 好, i.e. 1 ('a') + 3 ('你') = 4.
        assert_eq!(ed.cursor(), 4);
    }

    #[test]
    fn backspace_in_middle_removes_char_before_cursor() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("abcd", 2);
        assert_eq!(ed.consume(DEL), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "acd", "removed 'b' (char before cursor)");
        assert_eq!(ed.cursor(), 1);
    }

    #[test]
    fn backspace_at_cursor_zero_is_noop() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("abc", 0);
        assert_eq!(ed.consume(DEL), KeyOutcome::Nothing);
        assert_eq!(ed.buffer(), "abc");
        assert_eq!(ed.cursor(), 0);
    }

    #[test]
    fn enter_resets_cursor_to_zero() {
        // Submit clears the buffer; the next draft must start with
        // the caret at the start, not at the stale tail position.
        let mut ed = QueueEditor::new();
        ed.seed("ok");
        match ed.consume(b'\n') {
            KeyOutcome::Submit { pending } => assert_eq!(pending, "ok"),
            other => panic!("expected Submit, got {other:?}"),
        }
        assert_eq!(ed.cursor(), 0);
    }

    #[test]
    fn ctrl_u_resets_cursor_to_zero() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("hello", 3);
        assert_eq!(ed.consume(NAK), KeyOutcome::Repaint);
        assert_eq!(ed.buffer(), "");
        assert_eq!(ed.cursor(), 0);
    }

    #[test]
    fn ctrl_c_resets_cursor_to_zero() {
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("oops", 2);
        // Non-empty draft: Ctrl-C clears it (Repaint), resetting the cursor.
        assert_eq!(ed.consume(ETX), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 0);
    }

    #[test]
    fn csi_with_modifier_param_left_arrow_still_moves_cursor() {
        // Some terminals send `ESC [ 1 ; 2 D` for Shift+Left.
        // Our parser only inspects the first param so this should
        // still dispatch to Left (we don't differentiate Shift).
        let mut ed = QueueEditor::new();
        ed.seed("abc");
        assert_eq!(csi(&mut ed, b"1;2D"), KeyOutcome::Repaint);
        assert_eq!(ed.cursor(), 2);
    }

    #[test]
    fn up_and_down_arrows_are_quiet_noops_for_now() {
        // No history in the queue editor — Up/Down must not
        // pollute the buffer or move the caret.
        let mut ed = QueueEditor::new();
        ed.seed_with_cursor("abc", 1);
        assert_eq!(csi(&mut ed, b"A"), KeyOutcome::Nothing); // Up
        assert_eq!(csi(&mut ed, b"B"), KeyOutcome::Nothing); // Down
        assert_eq!(ed.buffer(), "abc");
        assert_eq!(ed.cursor(), 1);
    }
}
