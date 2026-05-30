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
//! - Mid-line cursor editing (left/right, word jumps). The buffer
//!   only supports append + delete-from-end. Most queued prompts
//!   are short follow-ups; cursor editing can come later if real
//!   usage demands it.
//! - History recall during the turn. rustyline owns history and is
//!   idle; resurrecting it would require shared state.
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

    /// Test helper: insert raw text as if it had come from the
    /// terminal in ground state. Useful for unit tests that want
    /// to set up a non-empty buffer without driving every byte
    /// through `consume`.
    #[cfg(test)]
    fn seed(&mut self, text: &str) {
        self.buf.push_str(text);
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
        match byte {
            ESC => {
                self.state = PumpState::Escape;
                KeyOutcome::Nothing
            }
            CR | LF => {
                let trimmed = self.buf.trim().to_owned();
                self.buf.clear();
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
                // Ctrl-C clears the draft. We deliberately don't
                // propagate SIGINT — the main thread owns signal
                // handling and the user can still Ctrl-C at the
                // rustyline prompt to actually exit.
                self.buf.clear();
                KeyOutcome::Cancelled
            }
            NAK => {
                if self.buf.is_empty() {
                    KeyOutcome::Nothing
                } else {
                    self.buf.clear();
                    KeyOutcome::Repaint
                }
            }
            DEL | BS_LEGACY => {
                if self.buf.pop().is_some() {
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
            self.buf.push(byte as char);
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

        // Codepoint assembled. Decode and append.
        let outcome = match std::str::from_utf8(&self.utf8_pending) {
            Ok(decoded) => {
                self.buf.push_str(decoded);
                KeyOutcome::Repaint
            }
            Err(_) => KeyOutcome::Nothing,
        };
        self.utf8_pending.clear();
        self.utf8_expected = 0;
        outcome
    }

    fn consume_escape(&mut self, byte: u8) -> KeyOutcome {
        match byte {
            b'[' => {
                self.state = PumpState::Csi;
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
        // CSI terminator: any byte in the `@…~` range closes the
        // sequence. We check for the bracketed-paste start marker
        // ("200~") and end marker ("201~") at the moment of close
        // by looking at the param fragment we accumulated — but
        // since we don't actually accumulate the params (we'd need
        // a separate buffer for that), we approximate with a
        // direct match on the closing byte plus the last few
        // bytes seen. For the v1 implementation we settle for
        // *consuming* the sequence cleanly and treat any future
        // bracketed-paste bytes as ground-mode input — the
        // start/end markers will hit `consume_paste` via a future
        // refinement once we wire param accumulation.
        if (0x40..=0x7e).contains(&byte) {
            self.state = PumpState::Ground;
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
/// Layout (color on, with non-empty buffer + 2 pending):
///
/// ```text
/// ▎ ✎ hello world█  (Enter to queue · queue 2)
/// ```
///
/// The trailing space after the buffer represents the simulated
/// cursor — we paint it with a reverse-video block (`\x1b[7m \x1b[27m`)
/// so the user has a clear "you are here" indicator without
/// actually moving the real terminal cursor. The text after the
/// hint is right-padded conceptually but we just append, letting
/// the terminal truncate if the line exceeds the width.
///
/// `width` is used purely for safety-truncation so we don't paint
/// an absurdly long line that wraps and breaks the pinned input
/// row. Callers usually pass the terminal width minus a small
/// margin. Pass `usize::MAX` to disable truncation.
pub fn paint_editor_line(buf: &str, queue_depth: usize, width: usize, color: bool) -> String {
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
    let cursor_block = if color {
        "\x1b[7m \x1b[27m".to_owned()
    } else {
        "_".to_owned()
    };
    let hint_styled = if color {
        format!("{DIM}({hint}){RESET}")
    } else {
        format!("({hint})")
    };

    if buf.is_empty() {
        format!("{prompt} {cursor_block}  {hint_styled}")
    } else {
        // Truncate the buffer (only) when the visible-character
        // count would exceed the width budget. We approximate
        // visible width with `.chars().count()` — correct for ASCII
        // and CJK in fixed-width fonts (which is what terminals
        // assume), wrong for double-width emoji, but a reasonable
        // first pass.
        let max_buf_chars = width.saturating_sub(prompt.chars().count() + hint.chars().count() + 8);
        let trimmed = if buf.chars().count() > max_buf_chars {
            // Take last `max_buf_chars` characters: while typing a
            // long line the *tail* (where the cursor is) is what
            // matters to the user, not the head.
            let drop = buf.chars().count() - max_buf_chars;
            buf.chars().skip(drop).collect::<String>()
        } else {
            buf.to_owned()
        };
        format!("{prompt} {trimmed}{cursor_block}  {hint_styled}")
    }
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
/// the spawning site reads cleanly and we don't pass six positional
/// `Arc`s around.
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
}

#[cfg(unix)]
/// Block (with a timeout) until stdin has at least one byte ready.
/// Returns:
///   * `Ok(true)`  — data ready, caller should `read()`
///   * `Ok(false)` — timed out, caller should re-check `stop`
///   * `Err(_)`    — transient EINTR or similar; caller retries
fn poll_stdin(timeout_ms: i32) -> std::io::Result<bool> {
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
/// `Ok(0)` when no data was ready (caller polls again).
fn read_burst(buf: &mut [u8]) -> std::io::Result<usize> {
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
pub fn run_pump(ctx: EditorContext) {
    let mut editor = QueueEditor::new();
    let mut buf = [0u8; 64];

    // Initial paint so the affordance shows up immediately, even
    // before the user types a single byte.
    paint(&ctx, &editor);

    while !ctx.stop.load(Ordering::Relaxed) {
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
                KeyOutcome::Cancelled => dirty = true,
                KeyOutcome::Nothing => {}
            }
        }
        if dirty {
            paint(&ctx, &editor);
        }
    }
}

#[cfg(not(unix))]
pub fn run_pump(_ctx: EditorContext) {
    // No-op on non-Unix. Callers should not invoke this when
    // `enter_raw_mode()` returns None.
}

fn paint(ctx: &EditorContext, editor: &QueueEditor) {
    let depth = ctx.queue.lock().map(|q| q.len()).unwrap_or(0);
    let line = paint_editor_line(editor.buffer(), depth, ctx.line_width, ctx.use_color);
    if let Ok(_guard) = ctx.paint_lock.lock() {
        (ctx.repaint)(&line);
    }
}

// ─── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
    fn ctrl_c_cancels_and_clears() {
        let mut ed = QueueEditor::new();
        ed.seed("oops");
        assert_eq!(ed.consume(ETX), KeyOutcome::Cancelled);
        assert!(ed.buffer().is_empty());
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
        let painted = paint_editor_line("", 0, 100, false);
        assert!(painted.contains("type a message"), "got: {painted}");
        assert!(painted.contains("queues for the next turn"));
    }

    #[test]
    fn paint_editor_line_empty_buffer_with_pending_swaps_hint() {
        let painted = paint_editor_line("", 3, 100, false);
        assert!(painted.contains("queue 3"), "got: {painted}");
        assert!(painted.contains("type to queue more"));
    }

    #[test]
    fn paint_editor_line_with_buffer_shows_enter_hint_and_depth() {
        let painted = paint_editor_line("hello", 2, 100, false);
        assert!(painted.contains("hello"));
        assert!(painted.contains("Enter queues"));
        assert!(painted.contains("queue 2"));
    }

    #[test]
    fn paint_editor_line_color_emits_sgr() {
        let painted = paint_editor_line("hi", 0, 100, true);
        assert!(
            painted.contains("\x1b[1;36m"),
            "missing prompt color: {painted}"
        );
        assert!(painted.contains("\x1b[2m"), "missing dim hint: {painted}");
        assert!(
            painted.contains("\x1b[7m"),
            "missing reverse cursor block: {painted}"
        );
    }

    #[test]
    fn paint_editor_line_truncates_long_buffer_keeping_tail() {
        // 80-char-wide budget. The buffer is ~200 chars; we should
        // keep the tail so the user's caret context is preserved.
        let long_buf: String = "abcde".repeat(40);
        let painted = paint_editor_line(&long_buf, 0, 80, false);
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
}
