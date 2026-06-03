//! Shared line-editing primitives for the raw-mode editors.
//!
//! Both the mid-turn [`queue_editor`](crate::queue_editor) and the
//! between-turns [`prompt_editor`](crate::prompt_editor) operate on an
//! in-memory UTF-8 `String` buffer with a byte cursor that always sits on
//! a codepoint boundary. They share three concerns that have nothing to
//! do with *which* editor is running:
//!
//! 1. **Grapheme / codepoint boundary math.** Backspace, Delete, and
//!    Left/Right arrow must step over a whole user-perceived character —
//!    a base codepoint plus any trailing invisible modifiers (variation
//!    selectors, ZWJ chains, combining marks) — in a single keypress.
//!    These are pure `(&str, usize) -> usize` (or `&mut`) functions with
//!    no editor state.
//! 2. **Incremental UTF-8 assembly.** Raw-mode `read(2)` delivers
//!    multi-byte codepoints (CJK, emoji) one byte at a time; we can't
//!    cast each byte to `char`. [`Utf8Assembler`] accumulates bytes until
//!    a codepoint is complete, rejecting malformed sequences.
//! 3. **Key decoding.** Turning the raw byte stream into high-level
//!    [`Key`] events (printable text, arrows, Enter vs. newline,
//!    bracketed-paste boundaries, Shift+Tab, …) via the [`KeyDecoder`]
//!    CSI/escape state machine.
//!
//! `queue_editor` predates this module and keeps its own (well-tested)
//! consume loop, but now sources the boundary helpers from here so the
//! grapheme logic has a single home. `prompt_editor` is built entirely
//! on top of [`KeyDecoder`] + these helpers.

// ─── Grapheme / codepoint boundary helpers ─────────────────────────

/// Backspace at the caret: remove the grapheme cluster that ends at
/// `*cursor` and shift the caret backwards. A grapheme is what the user
/// perceives as a single visible character — for ASCII / most CJK that's
/// exactly one Rust `char`, but IMEs commonly commit a base codepoint
/// followed by one or more **invisible modifiers** (variation selectors,
/// zero-width joiners, combining marks). Treating each codepoint as a
/// separate Backspace target makes those sequences require 2+ keypresses
/// for one visible character disappearance, which the user perceives as
/// a bug — this function avoids that by:
///
/// 1. Finding the codepoint immediately before the caret.
/// 2. If it's a base, removing just that codepoint.
/// 3. If it's a modifier, walking further back to peel off any chain of
///    modifiers AND the base they decorate, so a single press clears the
///    whole visible cluster.
///
/// Returns `true` if anything was removed (caller should repaint). When
/// the caret is at offset 0 there's nothing to remove → returns `false`.
pub(crate) fn backspace_grapheme(buf: &mut String, cursor: &mut usize) -> bool {
    if *cursor == 0 {
        return false;
    }
    let start = prev_codepoint_boundary(buf, *cursor);
    let first: char = buf[start..*cursor]
        .chars()
        .next()
        .expect("non-empty slice between two valid boundaries");
    let mut new_cursor = start;
    if is_grapheme_modifier(first) {
        while new_cursor > 0 {
            let p = prev_codepoint_boundary(buf, new_cursor);
            let c = buf[p..new_cursor]
                .chars()
                .next()
                .expect("non-empty slice between two valid boundaries");
            if !is_grapheme_modifier(c) {
                new_cursor = p;
                break;
            }
            new_cursor = p;
        }
    }
    buf.drain(new_cursor..*cursor);
    *cursor = new_cursor;
    true
}

/// Forward delete at the caret: remove the grapheme cluster that starts
/// at `cursor`. Mirror of [`backspace_grapheme`] without caret movement.
/// Returns `false` when the caret is already at the end (nothing to
/// delete).
pub(crate) fn delete_grapheme_at_cursor(buf: &mut String, cursor: usize) -> bool {
    if cursor >= buf.len() {
        return false;
    }
    let mut end = next_codepoint_boundary(buf, cursor);
    while end < buf.len() {
        let next = next_codepoint_boundary(buf, end);
        let c = buf[end..next]
            .chars()
            .next()
            .expect("non-empty slice between valid boundaries");
        if !is_grapheme_modifier(c) {
            break;
        }
        end = next;
    }
    buf.drain(cursor..end);
    true
}

/// Step the caret one grapheme cluster forward — i.e. past one base
/// codepoint plus any modifier chain that decorates it. `boundary` must
/// already sit on a codepoint boundary; the function clamps to
/// `buf.len()`.
pub(crate) fn next_grapheme_boundary(buf: &str, boundary: usize) -> usize {
    if boundary >= buf.len() {
        return buf.len();
    }
    let mut next = next_codepoint_boundary(buf, boundary);
    while next < buf.len() {
        let after = next_codepoint_boundary(buf, next);
        let c = buf[next..after]
            .chars()
            .next()
            .expect("non-empty slice between valid boundaries");
        if !is_grapheme_modifier(c) {
            break;
        }
        next = after;
    }
    next
}

/// Step the caret one grapheme cluster backward. Symmetric with
/// [`next_grapheme_boundary`]. Returns `0` when `boundary` is already at
/// the start.
pub(crate) fn prev_grapheme_boundary(buf: &str, boundary: usize) -> usize {
    if boundary == 0 {
        return 0;
    }
    let mut cur = boundary;
    loop {
        let prev = prev_codepoint_boundary(buf, cur);
        let c = buf[prev..cur]
            .chars()
            .next()
            .expect("non-empty slice between valid boundaries");
        let is_mod = is_grapheme_modifier(c);
        cur = prev;
        if !is_mod {
            break;
        }
        if cur == 0 {
            break;
        }
    }
    cur
}

/// Byte offset of the codepoint boundary immediately before `boundary`.
/// Required because `String` doesn't expose this directly without
/// `floor_char_boundary` (unstable in our MSRV).
pub(crate) fn prev_codepoint_boundary(buf: &str, boundary: usize) -> usize {
    debug_assert!(buf.is_char_boundary(boundary));
    let mut i = boundary.saturating_sub(1);
    while i > 0 && !buf.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Byte offset of the codepoint boundary immediately after `boundary`.
/// Symmetric with [`prev_codepoint_boundary`].
pub(crate) fn next_codepoint_boundary(buf: &str, boundary: usize) -> usize {
    debug_assert!(buf.is_char_boundary(boundary));
    let len = buf.len();
    let mut i = (boundary + 1).min(len);
    while i < len && !buf.is_char_boundary(i) {
        i += 1;
    }
    i
}

/// `true` for codepoints that the IME / Unicode treats as "non-spacing"
/// or "attaching" — i.e. they decorate a preceding base character
/// without occupying their own visible column.
///
/// We approximate with the ranges that cover the common IME failure
/// modes (combining marks, variation selectors, zero-width formatting).
/// Full UAX #29 grapheme segmentation is out of scope; this classifier
/// catches every IME-emitted modifier observed in the wild on macOS /
/// Windows IMEs for Chinese, Japanese, Korean, and emoji input.
pub(crate) fn is_grapheme_modifier(c: char) -> bool {
    matches!(
        c,
        '\u{0300}'..='\u{036F}'
            | '\u{1AB0}'..='\u{1AFF}'
            | '\u{1DC0}'..='\u{1DFF}'
            | '\u{200B}'..='\u{200F}'
            | '\u{20D0}'..='\u{20FF}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FE20}'..='\u{FE2F}'
            | '\u{E0100}'..='\u{E01EF}'
    )
}

// ─── Incremental UTF-8 assembly ────────────────────────────────────

/// Result of feeding one byte into a [`Utf8Assembler`].
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Utf8Step {
    /// Byte consumed; the codepoint is still incomplete (or the byte was
    /// a malformed lead/continuation and was dropped). Nothing to insert.
    Pending,
    /// A complete codepoint was assembled. The owned `String` holds
    /// exactly one `char`.
    Char(String),
}

/// Accumulates raw bytes into complete UTF-8 codepoints. Raw-mode
/// `read(2)` can split a multi-byte codepoint across reads (and CJK / IME
/// input routinely does), so naively casting each byte to `char` produces
/// Latin-1 garbage. Malformed sequences (continuation byte without a
/// lead, ASCII mid-codepoint, out-of-range continuation) are dropped
/// silently — surfacing U+FFFD would just replace clean rendering with
/// visible garbage on a transient paste glitch.
#[derive(Debug, Default)]
pub(crate) struct Utf8Assembler {
    pending: Vec<u8>,
    expected: usize,
}

impl Utf8Assembler {
    /// Feed one byte. `byte` is assumed `>= 0x80` OR a fresh ASCII byte;
    /// callers that special-case control codes should not route those
    /// here. Returns [`Utf8Step::Char`] when a codepoint completes.
    pub(crate) fn push(&mut self, byte: u8) -> Utf8Step {
        if byte < 0x80 {
            if self.expected > 0 {
                // ASCII byte mid-codepoint: malformed. Drop the partial
                // codepoint and the offending byte.
                self.reset();
                return Utf8Step::Pending;
            }
            return Utf8Step::Char((byte as char).to_string());
        }

        if self.expected == 0 {
            let expected = match byte {
                0xC2..=0xDF => 2,
                0xE0..=0xEF => 3,
                0xF0..=0xF4 => 4,
                _ => return Utf8Step::Pending,
            };
            self.expected = expected;
            self.pending.clear();
            self.pending.push(byte);
            return Utf8Step::Pending;
        }

        if !(0x80..=0xBF).contains(&byte) {
            self.reset();
            return Utf8Step::Pending;
        }
        self.pending.push(byte);
        if self.pending.len() < self.expected {
            return Utf8Step::Pending;
        }
        let step = match std::str::from_utf8(&self.pending) {
            Ok(decoded) => Utf8Step::Char(decoded.to_owned()),
            Err(_) => Utf8Step::Pending,
        };
        self.reset();
        step
    }

    fn reset(&mut self) {
        self.pending.clear();
        self.expected = 0;
    }
}

// ─── Key decoding ──────────────────────────────────────────────────

/// High-level key event produced by [`KeyDecoder`]. The byte-level CSI /
/// escape-sequence detours are fully resolved here so editors match on a
/// flat enum instead of re-implementing a state machine.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Key {
    /// One complete printable codepoint (already UTF-8 assembled).
    Char(String),
    /// Plain Enter / Return (CR). The editor's "submit" gesture.
    Enter,
    /// A newline that should be *inserted* into the buffer rather than
    /// submitting: Ctrl-J (LF), Alt+Enter (`ESC` + CR/LF), or a modified
    /// Enter reported as CSI `13;Nu`.
    Newline,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    /// Ctrl-C.
    Interrupt,
    /// Ctrl-D.
    Eof,
    /// Ctrl-U: kill the whole line.
    KillLine,
    /// Ctrl-W: kill the word before the caret.
    KillWord,
    /// Tab — completion request.
    Tab,
    /// Shift+Tab (CSI `Z`) — mode cycle / reverse.
    BackTab,
    /// Bracketed-paste start marker (`ESC[200~`). The decoder switches to
    /// paste mode; subsequent bytes arrive as `Char`/`Newline` until
    /// [`Key::PasteEnd`].
    PasteStart,
    /// Bracketed-paste end marker (`ESC[201~`).
    PasteEnd,
}

#[derive(Debug, Default, PartialEq, Eq)]
enum DecodeState {
    #[default]
    Ground,
    Escape,
    Csi,
    /// SS3 (`ESC O`) — some terminals send arrows / Home / End as
    /// `ESC O A`..`ESC O F` in application-keypad mode.
    Ss3,
}

/// Byte-stream → [`Key`] state machine. Stateful across bytes (escape
/// sequences span multiple `read` bytes); feed every byte via
/// [`KeyDecoder::push`] and act on the returned `Option<Key>`.
///
/// Newline policy (chosen for the multi-line prompt editor):
/// * CR (`\r`, the byte Enter sends in raw mode) → [`Key::Enter`] (submit).
/// * LF (`\n`, Ctrl-J) → [`Key::Newline`] (insert) — the widely-used
///   "Ctrl-J for a literal newline" shell convention.
/// * `ESC`+CR/LF (Alt+Enter) and CSI `13;Nu` (kitty / modifyOtherKeys
///   modified Enter) → [`Key::Newline`].
#[derive(Debug, Default)]
pub(crate) struct KeyDecoder {
    state: DecodeState,
    utf8: Utf8Assembler,
    csi_param1: Option<u32>,
    csi_param2: Option<u32>,
    csi_field: u8,
    paste: bool,
}

const ESC: u8 = 0x1b;
const CR: u8 = b'\r';
const LF: u8 = b'\n';

impl KeyDecoder {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// `true` while inside a bracketed-paste block. Callers may use this
    /// to suppress completion / history side effects during a paste.
    pub(crate) fn in_paste(&self) -> bool {
        self.paste
    }

    /// Feed one byte; returns a [`Key`] when a complete event is decoded.
    pub(crate) fn push(&mut self, byte: u8) -> Option<Key> {
        match self.state {
            DecodeState::Ground => self.ground(byte),
            DecodeState::Escape => self.escape(byte),
            DecodeState::Csi => self.csi(byte),
            DecodeState::Ss3 => self.ss3(byte),
        }
    }

    fn ground(&mut self, byte: u8) -> Option<Key> {
        // Inside a paste block the only control bytes we honour are
        // CR/LF (→ literal newline) and Tab (→ literal tab); the ESC
        // that begins the end-marker is handled normally. Everything
        // else is literal text.
        if self.paste {
            match byte {
                ESC => {
                    self.state = DecodeState::Escape;
                    return None;
                }
                CR | LF => return Some(Key::Char("\n".to_owned())),
                b'\t' => return Some(Key::Char("\t".to_owned())),
                b if b < 0x20 => return None,
                b => return self.feed_text(b),
            }
        }
        match byte {
            ESC => {
                self.state = DecodeState::Escape;
                None
            }
            CR => Some(Key::Enter),
            LF => Some(Key::Newline),
            0x09 => Some(Key::Tab),
            0x7f | 0x08 => Some(Key::Backspace),
            0x03 => Some(Key::Interrupt),
            0x04 => Some(Key::Eof),
            0x15 => Some(Key::KillLine),
            0x17 => Some(Key::KillWord),
            0x01 => Some(Key::Home),
            0x05 => Some(Key::End),
            0x02 => Some(Key::Left),
            0x06 => Some(Key::Right),
            b if b < 0x20 => None,
            b => self.feed_text(b),
        }
    }

    fn feed_text(&mut self, byte: u8) -> Option<Key> {
        match self.utf8.push(byte) {
            Utf8Step::Char(s) => Some(Key::Char(s)),
            Utf8Step::Pending => None,
        }
    }

    fn escape(&mut self, byte: u8) -> Option<Key> {
        match byte {
            b'[' => {
                self.state = DecodeState::Csi;
                self.csi_param1 = None;
                self.csi_param2 = None;
                self.csi_field = 0;
                None
            }
            b'O' => {
                self.state = DecodeState::Ss3;
                None
            }
            // Alt+Enter: ESC followed by CR/LF → insert a newline.
            CR | LF => {
                self.state = DecodeState::Ground;
                Some(Key::Newline)
            }
            // ESC ESC: treat the second ESC as a fresh prefix.
            ESC => {
                self.state = DecodeState::Escape;
                None
            }
            // ESC <other>: Alt-modified key we don't bind. Back to ground.
            _ => {
                self.state = DecodeState::Ground;
                None
            }
        }
    }

    fn ss3(&mut self, byte: u8) -> Option<Key> {
        self.state = DecodeState::Ground;
        match byte {
            b'A' => Some(Key::Up),
            b'B' => Some(Key::Down),
            b'C' => Some(Key::Right),
            b'D' => Some(Key::Left),
            b'H' => Some(Key::Home),
            b'F' => Some(Key::End),
            _ => None,
        }
    }

    fn csi(&mut self, byte: u8) -> Option<Key> {
        if byte.is_ascii_digit() {
            let digit = (byte - b'0') as u32;
            let slot = if self.csi_field == 0 {
                &mut self.csi_param1
            } else {
                &mut self.csi_param2
            };
            *slot = Some(slot.unwrap_or(0).saturating_mul(10).saturating_add(digit));
            return None;
        }
        if byte == b';' {
            self.csi_field = self.csi_field.saturating_add(1);
            return None;
        }
        // Intermediate bytes — legal but unused by the keys we handle.
        if (0x20..=0x2F).contains(&byte) {
            return None;
        }
        if (0x40..=0x7e).contains(&byte) {
            let p1 = self.csi_param1.unwrap_or(0);
            self.state = DecodeState::Ground;
            self.csi_field = 0;
            return match byte {
                b'A' => Some(Key::Up),
                b'B' => Some(Key::Down),
                b'C' => Some(Key::Right),
                b'D' => Some(Key::Left),
                b'H' => Some(Key::Home),
                b'F' => Some(Key::End),
                b'Z' => Some(Key::BackTab),
                // CSI u (kitty / fixterms): codepoint;modifier u. A
                // modified Enter (codepoint 13) means "insert newline".
                b'u' => {
                    if p1 == 13 {
                        Some(Key::Newline)
                    } else {
                        None
                    }
                }
                b'~' => match p1 {
                    1 | 7 => Some(Key::Home),
                    3 => Some(Key::Delete),
                    4 | 8 => Some(Key::End),
                    200 => {
                        self.paste = true;
                        Some(Key::PasteStart)
                    }
                    201 => {
                        self.paste = false;
                        Some(Key::PasteEnd)
                    }
                    _ => None,
                },
                _ => None,
            };
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_all(bytes: &[u8]) -> Vec<Key> {
        let mut dec = KeyDecoder::new();
        let mut out = Vec::new();
        for &b in bytes {
            if let Some(k) = dec.push(b) {
                out.push(k);
            }
        }
        out
    }

    #[test]
    fn ascii_decodes_to_char_keys() {
        assert_eq!(
            decode_all(b"hi"),
            vec![Key::Char("h".into()), Key::Char("i".into())]
        );
    }

    #[test]
    fn cr_is_enter_lf_is_newline() {
        assert_eq!(decode_all(b"\r"), vec![Key::Enter]);
        assert_eq!(decode_all(b"\n"), vec![Key::Newline]);
    }

    #[test]
    fn alt_enter_inserts_newline() {
        assert_eq!(decode_all(&[ESC, b'\r']), vec![Key::Newline]);
        assert_eq!(decode_all(&[ESC, b'\n']), vec![Key::Newline]);
    }

    #[test]
    fn csi_u_modified_enter_is_newline() {
        // ESC [ 1 3 ; 2 u  (Shift+Enter in kitty/fixterms).
        assert_eq!(decode_all(b"\x1b[13;2u"), vec![Key::Newline]);
        // A non-Enter CSI u codepoint is ignored.
        assert_eq!(decode_all(b"\x1b[97;2u"), vec![]);
    }

    #[test]
    fn arrows_decode_from_csi_and_ss3() {
        assert_eq!(
            decode_all(b"\x1b[A\x1b[B\x1b[C\x1b[D"),
            vec![Key::Up, Key::Down, Key::Right, Key::Left]
        );
        assert_eq!(
            decode_all(b"\x1bOA\x1bOB\x1bOC\x1bOD"),
            vec![Key::Up, Key::Down, Key::Right, Key::Left]
        );
    }

    #[test]
    fn home_end_delete_via_tilde_family() {
        assert_eq!(decode_all(b"\x1b[1~"), vec![Key::Home]);
        assert_eq!(decode_all(b"\x1b[4~"), vec![Key::End]);
        assert_eq!(decode_all(b"\x1b[3~"), vec![Key::Delete]);
        assert_eq!(decode_all(b"\x1b[H\x1b[F"), vec![Key::Home, Key::End]);
    }

    #[test]
    fn shift_tab_is_back_tab() {
        assert_eq!(decode_all(b"\x1b[Z"), vec![Key::BackTab]);
    }

    #[test]
    fn control_keys_decode() {
        assert_eq!(decode_all(&[0x03]), vec![Key::Interrupt]);
        assert_eq!(decode_all(&[0x04]), vec![Key::Eof]);
        assert_eq!(decode_all(&[0x15]), vec![Key::KillLine]);
        assert_eq!(decode_all(&[0x17]), vec![Key::KillWord]);
        assert_eq!(decode_all(&[0x09]), vec![Key::Tab]);
        assert_eq!(decode_all(&[0x7f]), vec![Key::Backspace]);
    }

    #[test]
    fn cjk_codepoint_assembles_then_emits_one_char() {
        // 你 = E4 BD A0
        let keys = decode_all("你".as_bytes());
        assert_eq!(keys, vec![Key::Char("你".into())]);
    }

    #[test]
    fn bracketed_paste_emits_literal_text_including_newlines() {
        // ESC[200~ a \n b ESC[201~
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x1b[200~");
        bytes.extend_from_slice(b"a\nb");
        bytes.extend_from_slice(b"\x1b[201~");
        let keys = decode_all(&bytes);
        assert_eq!(
            keys,
            vec![
                Key::PasteStart,
                Key::Char("a".into()),
                Key::Char("\n".into()),
                Key::Char("b".into()),
                Key::PasteEnd,
            ]
        );
    }

    #[test]
    fn utf8_assembler_rejects_malformed_lead() {
        let mut a = Utf8Assembler::default();
        // 0xFF is never a valid lead byte.
        assert_eq!(a.push(0xFF), Utf8Step::Pending);
        // A subsequent ASCII byte still decodes cleanly.
        assert_eq!(a.push(b'x'), Utf8Step::Char("x".into()));
    }

    #[test]
    fn grapheme_backspace_removes_base_plus_variation_selector() {
        let mut buf = String::from("\u{2764}\u{FE0F}");
        let mut cursor = buf.len();
        assert!(backspace_grapheme(&mut buf, &mut cursor));
        assert_eq!(buf, "");
        assert_eq!(cursor, 0);
    }

    #[test]
    fn grapheme_boundaries_step_over_cjk() {
        let buf = "a你b";
        let after_a = next_grapheme_boundary(buf, 0);
        assert_eq!(after_a, 1);
        let after_cjk = next_grapheme_boundary(buf, 1);
        assert_eq!(after_cjk, 4);
        assert_eq!(prev_grapheme_boundary(buf, 4), 1);
    }
}
