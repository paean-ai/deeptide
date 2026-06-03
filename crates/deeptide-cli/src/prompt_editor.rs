//! Custom raw-mode multi-line prompt editor (replaces rustyline for the
//! between-turns input).
//!
//! Why this exists
//! ===============
//!
//! The pinned status bar uses a DECSTBM scroll region with the last few
//! rows reserved as a fixed footer. rustyline anchors its redraw to the
//! `start_row` it captured when `readline()` was entered and offers no
//! API to shift that mid-edit, so a wide/CJK line that wraps mid-compose
//! could overwrite the status bar or the bottom of the scroll region —
//! the "characters overlap on the same line" bug.
//!
//! This editor owns the cursor: it lays out the draft into explicit
//! visual rows using [`deeptide_core::width`] and paints each row with an
//! absolute `CUP` move (autowrap disabled), so the terminal's own
//! auto-wrap quirks become irrelevant. The footer height grows with the
//! draft so the input never collides with the status bar.
//!
//! The module is split into pure, unit-testable pieces:
//! * [`layout`] — width-aware visual-row layout (the bug fix lives here).
//! * [`PromptEditor`] — the editing state machine over a `String` buffer.
//! * [`History`] — a persistent, multi-line-safe history store.
//! * `read_prompt` — the raw-mode main-thread read loop (Unix only).

use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

use deeptide_core::width::{char_width, display_width};

use crate::status_bar::AnchoredStatusBar;

use crate::line_edit::{
    Key, KeyDecoder, backspace_grapheme, delete_grapheme_at_cursor, next_grapheme_boundary,
    prev_codepoint_boundary, prev_grapheme_boundary,
};

// ─── Pure layout engine ────────────────────────────────────────────

/// Visual layout of the prompt + draft for a given terminal width.
///
/// `rows` are the literal strings to paint, one per screen row; the
/// first row already includes the (possibly ANSI-styled) prompt prefix.
/// `caret_row` / `caret_col` are 0-based coordinates of the caret within
/// this block (row 0 = the first/topmost visual row, col 0 = the first
/// column). The painter adds the absolute row offset and converts to the
/// terminal's 1-based coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptLayout {
    pub rows: Vec<String>,
    pub caret_row: usize,
    pub caret_col: usize,
}

impl PromptLayout {
    /// Number of visual rows the draft occupies (always `>= 1`).
    pub fn height(&self) -> usize {
        self.rows.len().max(1)
    }
}

/// Lay out `prompt` + `buf` into visual rows for a `cols`-wide terminal,
/// reporting the caret position for byte offset `cursor`. `hint`, when
/// non-empty and the caret is at the end of the buffer, is appended as
/// trailing (dim) text after the draft — it never affects the caret or
/// the row count beyond the row it lands on.
///
/// Wrapping is width-aware (via [`char_width`]): a wide CJK/emoji glyph
/// that wouldn't fit in the remaining columns wraps as a unit to the
/// next row, and explicit `\n` bytes start a fresh row at column 0. The
/// prompt occupies the leading columns of row 0, so the first row's text
/// budget is `cols - prompt_width`.
pub(crate) fn layout(
    prompt: &str,
    buf: &str,
    cursor: usize,
    cols: usize,
    hint: &str,
) -> PromptLayout {
    let cols = cols.max(1);
    let prompt_width = display_width(prompt);

    let mut rows: Vec<String> = vec![prompt.to_owned()];
    let mut col = prompt_width;
    let mut caret_row = 0usize;
    let mut caret_col = prompt_width;
    let mut caret_set = false;

    let mut offset = 0usize;
    for ch in buf.chars() {
        if offset == cursor {
            caret_row = rows.len() - 1;
            caret_col = col;
            caret_set = true;
        }
        if ch == '\n' {
            rows.push(String::new());
            col = 0;
            offset += ch.len_utf8();
            continue;
        }
        let w = char_width(ch);
        // Wrap before placing when the glyph wouldn't fit. `col > 0`
        // guards against an infinite loop on a degenerate `cols` that
        // can't hold even one cell.
        if col > 0 && col + w > cols {
            rows.push(String::new());
            col = 0;
            // The caret rides with the char it precedes: if we just
            // recorded the caret for this exact offset, move it onto the
            // freshly wrapped row too.
            if caret_set && offset == cursor {
                caret_row = rows.len() - 1;
                caret_col = 0;
            }
        }
        rows.last_mut().expect("rows always non-empty").push(ch);
        col += w;
        offset += ch.len_utf8();
    }

    // Caret at the very end of the buffer.
    if !caret_set {
        caret_row = rows.len() - 1;
        caret_col = col;
    }

    // If the caret would sit in the phantom column past the last cell,
    // push it to the start of a fresh row (autowrap is off when we
    // paint, so we place it explicitly).
    if caret_col >= cols {
        caret_row += 1;
        caret_col = 0;
        if rows.len() <= caret_row {
            rows.push(String::new());
        }
    }

    // Append the inline hint after the draft when the caret is at the
    // end. It's purely decorative (dim ghost text) and must not move the
    // caret, so we add it only when the caret already sits at the buffer
    // end (offset == buf.len()).
    if !hint.is_empty() && cursor >= buf.len() {
        rows.last_mut().expect("rows non-empty").push_str(hint);
    }

    PromptLayout {
        rows,
        caret_row,
        caret_col,
    }
}

/// One codepoint boundary's visual position. Built by [`visual_map`] and
/// used for vertical (Up/Down) navigation, which needs to translate a
/// `(row, col)` target back into a byte offset.
#[derive(Debug, Clone, Copy)]
struct VisPos {
    offset: usize,
    row: usize,
    col: usize,
}

/// Walk `prompt` + `buf` exactly like [`layout`] and record the visual
/// `(row, col)` of every codepoint boundary (including the end). Used to
/// move the caret vertically between wrapped/explicit rows.
fn visual_map(prompt: &str, buf: &str, cols: usize) -> Vec<VisPos> {
    let cols = cols.max(1);
    let prompt_width = display_width(prompt);
    let mut out = Vec::with_capacity(buf.len() + 1);
    let mut row = 0usize;
    let mut col = prompt_width;
    let mut offset = 0usize;
    for ch in buf.chars() {
        out.push(VisPos { offset, row, col });
        if ch == '\n' {
            row += 1;
            col = 0;
            offset += ch.len_utf8();
            continue;
        }
        let w = char_width(ch);
        if col > 0 && col + w > cols {
            row += 1;
            col = 0;
            // The boundary we just recorded precedes a char that wrapped;
            // fix up its position to the wrapped row so caret lookups land
            // where the glyph actually renders.
            if let Some(last) = out.last_mut() {
                last.row = row;
                last.col = 0;
            }
        }
        col += w;
        offset += ch.len_utf8();
    }
    out.push(VisPos { offset, row, col });
    out
}

// ─── Editor state machine ──────────────────────────────────────────

/// What a consumed byte resolved to. The raw-mode read loop acts on the
/// terminal-affecting variants; `Pending` / `Redraw` only differ in
/// whether a repaint is needed.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum EditOutcome {
    /// Byte consumed mid-escape-sequence or otherwise no visible change.
    Pending,
    /// Buffer / caret changed — repaint.
    Redraw,
    /// Plain Enter on the buffer: submit the (verbatim, possibly
    /// multi-line) draft.
    Submit(String),
    /// Ctrl-C on an empty buffer — caller echoes `^C` and re-prompts.
    Interrupt,
    /// Ctrl-D on an empty buffer — caller finalizes + exits.
    Eof,
    /// Shift+Tab — caller cycles the permission mode.
    ModeCycle,
}

/// Multi-line raw-mode line editor. Owns a UTF-8 `String` buffer and a
/// byte cursor that always rests on a codepoint boundary. Driven one
/// byte at a time via [`PromptEditor::consume`]; the read loop renders
/// the buffer through [`layout`] between bytes.
pub(crate) struct PromptEditor<'a> {
    buf: String,
    cursor: usize,
    decoder: KeyDecoder,
    /// Terminal width, used for width-aware vertical navigation. Updated
    /// by the read loop on resize.
    cols: usize,
    /// Prompt prefix width, also needed for vertical navigation.
    prompt: String,
    /// History snapshot (oldest → newest), taken at read start.
    history: Vec<String>,
    /// `None` = editing the live draft; `Some(i)` = showing `history[i]`.
    hist_idx: Option<usize>,
    /// The live draft stashed when the user first navigates up into
    /// history, restored when they navigate back past the newest entry.
    stash: Option<String>,
    /// Optional completion provider, wired by the read loop. Given
    /// `(line, byte_pos)` it returns a completion result. Borrowed for
    /// the editor's (short) lifetime so the read loop can hand it the
    /// REPL's command tables without cloning them.
    completion: Option<Box<CompletionFn<'a>>>,
    /// Set after a Tab that listed multiple candidates, so a second Tab
    /// can surface them (handled by the read loop via [`take_listing`]).
    listing: Option<Vec<String>>,
}

/// Result of a completion lookup. `start` is the byte offset where the
/// replacement begins; `candidates` are the full replacement strings.
pub(crate) struct CompletionResult {
    pub start: usize,
    pub candidates: Vec<String>,
}

/// Completion provider: maps `(line, byte_pos)` to a [`CompletionResult`].
/// Boxed behind a trait object so the read loop can hand the editor a
/// closure borrowing the REPL's command tables for its (short) lifetime.
type CompletionFn<'a> = dyn Fn(&str, usize) -> CompletionResult + 'a;

impl Default for PromptEditor<'_> {
    fn default() -> Self {
        Self {
            buf: String::new(),
            cursor: 0,
            decoder: KeyDecoder::new(),
            cols: 80,
            prompt: String::new(),
            history: Vec::new(),
            hist_idx: None,
            stash: None,
            completion: None,
            listing: None,
        }
    }
}

impl<'a> PromptEditor<'a> {
    pub(crate) fn new(prompt: impl Into<String>, cols: usize) -> Self {
        Self {
            prompt: prompt.into(),
            cols: cols.max(1),
            ..Self::default()
        }
    }

    pub(crate) fn buffer(&self) -> &str {
        &self.buf
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn set_cols(&mut self, cols: usize) {
        self.cols = cols.max(1);
    }

    pub(crate) fn set_history(&mut self, history: Vec<String>) {
        self.history = history;
        self.hist_idx = None;
        self.stash = None;
    }

    pub(crate) fn set_completion(&mut self, f: Box<CompletionFn<'a>>) {
        self.completion = Some(f);
    }

    /// Whether the decoder is mid bracketed-paste (used to suppress the
    /// inline hint while a paste streams in).
    pub(crate) fn in_paste(&self) -> bool {
        self.decoder.in_paste()
    }

    /// Pull and clear any pending multi-candidate listing produced by a
    /// previous Tab press.
    pub(crate) fn take_listing(&mut self) -> Option<Vec<String>> {
        self.listing.take()
    }

    /// Current layout for the editor's own width — convenience for the
    /// read loop's repaint.
    pub(crate) fn layout(&self, hint: &str) -> PromptLayout {
        layout(&self.prompt, &self.buf, self.cursor, self.cols, hint)
    }

    /// Feed one byte; returns the resolved [`EditOutcome`].
    pub(crate) fn consume(&mut self, byte: u8) -> EditOutcome {
        let Some(key) = self.decoder.push(byte) else {
            return EditOutcome::Pending;
        };
        self.handle_key(key)
    }

    fn handle_key(&mut self, key: Key) -> EditOutcome {
        match key {
            Key::Char(s) => {
                self.insert_str(&s);
                EditOutcome::Redraw
            }
            Key::Newline => {
                self.insert_str("\n");
                EditOutcome::Redraw
            }
            Key::Enter => EditOutcome::Submit(self.buf.clone()),
            Key::Backspace => {
                if backspace_grapheme(&mut self.buf, &mut self.cursor) {
                    EditOutcome::Redraw
                } else {
                    EditOutcome::Pending
                }
            }
            Key::Delete => {
                if delete_grapheme_at_cursor(&mut self.buf, self.cursor) {
                    EditOutcome::Redraw
                } else {
                    EditOutcome::Pending
                }
            }
            Key::Left => self.move_left(),
            Key::Right => self.move_right(),
            Key::Up => self.move_up(),
            Key::Down => self.move_down(),
            Key::Home => self.move_home(),
            Key::End => self.move_end(),
            Key::Interrupt => {
                if self.buf.is_empty() {
                    EditOutcome::Interrupt
                } else {
                    self.buf.clear();
                    self.cursor = 0;
                    EditOutcome::Redraw
                }
            }
            Key::Eof => {
                if self.buf.is_empty() {
                    EditOutcome::Eof
                } else if delete_grapheme_at_cursor(&mut self.buf, self.cursor) {
                    EditOutcome::Redraw
                } else {
                    EditOutcome::Pending
                }
            }
            Key::KillLine => {
                if self.buf.is_empty() {
                    EditOutcome::Pending
                } else {
                    self.buf.clear();
                    self.cursor = 0;
                    EditOutcome::Redraw
                }
            }
            Key::KillWord => self.kill_word(),
            Key::Tab => self.complete(),
            Key::BackTab => EditOutcome::ModeCycle,
            // Paste boundaries: the decoder flips its own `paste` flag;
            // the body bytes arrive as `Char`. Nothing else to do.
            Key::PasteStart | Key::PasteEnd => EditOutcome::Pending,
        }
    }

    fn insert_str(&mut self, s: &str) {
        debug_assert!(self.buf.is_char_boundary(self.cursor));
        self.buf.insert_str(self.cursor, s);
        self.cursor += s.len();
    }

    fn move_left(&mut self) -> EditOutcome {
        if self.cursor == 0 {
            return EditOutcome::Pending;
        }
        self.cursor = prev_grapheme_boundary(&self.buf, self.cursor);
        EditOutcome::Redraw
    }

    fn move_right(&mut self) -> EditOutcome {
        if self.cursor >= self.buf.len() {
            return EditOutcome::Pending;
        }
        self.cursor = next_grapheme_boundary(&self.buf, self.cursor);
        EditOutcome::Redraw
    }

    /// Start of the current logical line (just after the preceding `\n`).
    fn move_home(&mut self) -> EditOutcome {
        let start = self.buf[..self.cursor]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        if start == self.cursor {
            EditOutcome::Pending
        } else {
            self.cursor = start;
            EditOutcome::Redraw
        }
    }

    /// End of the current logical line (just before the next `\n`).
    fn move_end(&mut self) -> EditOutcome {
        let end = self.buf[self.cursor..]
            .find('\n')
            .map(|i| self.cursor + i)
            .unwrap_or(self.buf.len());
        if end == self.cursor {
            EditOutcome::Pending
        } else {
            self.cursor = end;
            EditOutcome::Redraw
        }
    }

    fn move_up(&mut self) -> EditOutcome {
        let map = visual_map(&self.prompt, &self.buf, self.cols);
        let cur = self.vis_pos(&map);
        if cur.row == 0 {
            return self.history_prev();
        }
        self.cursor = self.offset_on_row(&map, cur.row - 1, cur.col);
        EditOutcome::Redraw
    }

    fn move_down(&mut self) -> EditOutcome {
        let map = visual_map(&self.prompt, &self.buf, self.cols);
        let cur = self.vis_pos(&map);
        let last_row = map.iter().map(|p| p.row).max().unwrap_or(0);
        if cur.row >= last_row {
            return self.history_next();
        }
        self.cursor = self.offset_on_row(&map, cur.row + 1, cur.col);
        EditOutcome::Redraw
    }

    fn vis_pos(&self, map: &[VisPos]) -> VisPos {
        map.iter()
            .find(|p| p.offset == self.cursor)
            .copied()
            .unwrap_or(VisPos {
                offset: self.cursor,
                row: 0,
                col: 0,
            })
    }

    /// Byte offset of the boundary on `row` whose column is nearest
    /// `goal_col`. Falls back to the closest available row when `row`
    /// has no boundaries (shouldn't happen for a valid row).
    fn offset_on_row(&self, map: &[VisPos], row: usize, goal_col: usize) -> usize {
        map.iter()
            .filter(|p| p.row == row)
            .min_by_key(|p| p.col.abs_diff(goal_col))
            .map(|p| p.offset)
            .unwrap_or(self.cursor)
    }

    fn kill_word(&mut self) -> EditOutcome {
        if self.cursor == 0 {
            return EditOutcome::Pending;
        }
        let mut start = self.cursor;
        // Skip trailing whitespace immediately before the caret.
        while start > 0 {
            let prev = prev_codepoint_boundary(&self.buf, start);
            let c = self.buf[prev..start].chars().next();
            if matches!(c, Some(c) if c.is_whitespace()) {
                start = prev;
            } else {
                break;
            }
        }
        // Then delete back to the previous whitespace boundary.
        while start > 0 {
            let prev = prev_codepoint_boundary(&self.buf, start);
            let c = self.buf[prev..start].chars().next();
            if matches!(c, Some(c) if c.is_whitespace()) {
                break;
            }
            start = prev;
        }
        if start == self.cursor {
            return EditOutcome::Pending;
        }
        self.buf.drain(start..self.cursor);
        self.cursor = start;
        EditOutcome::Redraw
    }

    fn complete(&mut self) -> EditOutcome {
        let Some(f) = self.completion.as_ref() else {
            return EditOutcome::Pending;
        };
        let result = f(&self.buf, self.cursor);
        if result.candidates.is_empty() {
            return EditOutcome::Pending;
        }
        if result.candidates.len() == 1 {
            self.apply_completion(result.start, &result.candidates[0]);
            return EditOutcome::Redraw;
        }
        // Multiple candidates: insert the longest common prefix (if it
        // extends what's typed) and stash the list for the read loop to
        // display.
        let lcp = longest_common_prefix(&result.candidates);
        let typed = &self.buf[result.start..self.cursor];
        let changed = lcp.len() > typed.len() && lcp.starts_with(typed);
        if changed {
            self.apply_completion(result.start, &lcp);
        }
        self.listing = Some(result.candidates);
        if changed {
            EditOutcome::Redraw
        } else {
            // Nothing new inserted, but the listing should surface.
            EditOutcome::Redraw
        }
    }

    fn apply_completion(&mut self, start: usize, replacement: &str) {
        if start > self.buf.len() || !self.buf.is_char_boundary(start) {
            return;
        }
        let end = self.cursor.min(self.buf.len());
        self.buf.replace_range(start..end, replacement);
        self.cursor = start + replacement.len();
    }

    // ── History navigation ──────────────────────────────────────────

    fn history_prev(&mut self) -> EditOutcome {
        if self.history.is_empty() {
            return EditOutcome::Pending;
        }
        let idx = match self.hist_idx {
            None => {
                self.stash = Some(self.buf.clone());
                self.history.len() - 1
            }
            Some(0) => return EditOutcome::Pending,
            Some(i) => i - 1,
        };
        self.hist_idx = Some(idx);
        self.buf = self.history[idx].clone();
        self.cursor = self.buf.len();
        EditOutcome::Redraw
    }

    fn history_next(&mut self) -> EditOutcome {
        let Some(idx) = self.hist_idx else {
            return EditOutcome::Pending;
        };
        if idx + 1 < self.history.len() {
            self.hist_idx = Some(idx + 1);
            self.buf = self.history[idx + 1].clone();
            self.cursor = self.buf.len();
        } else {
            // Past the newest entry → restore the stashed live draft.
            self.hist_idx = None;
            self.buf = self.stash.take().unwrap_or_default();
            self.cursor = self.buf.len();
        }
        EditOutcome::Redraw
    }
}

/// Longest common prefix of a candidate set, on byte/char boundaries.
fn longest_common_prefix(items: &[String]) -> String {
    let Some(first) = items.first() else {
        return String::new();
    };
    let mut prefix = first.clone();
    for item in &items[1..] {
        while !item.starts_with(&prefix) {
            // Trim one char off the end to stay on a boundary.
            let new_len = prefix
                .char_indices()
                .next_back()
                .map(|(i, _)| i)
                .unwrap_or(0);
            prefix.truncate(new_len);
            if prefix.is_empty() {
                return prefix;
            }
        }
    }
    prefix
}

// ─── History store ─────────────────────────────────────────────────

/// Maximum number of entries kept on disk. Bounds the file so a
/// long-lived session doesn't grow it without limit; the oldest entries
/// are dropped first (matching rustyline's default behaviour).
const HISTORY_MAX: usize = 1000;

/// Persistent command history, replacing rustyline's `load_history` /
/// `save_history` / `add_history_entry`.
///
/// On-disk format: one entry per line, with `\` escaped as `\\` and
/// embedded newlines escaped as `\n` (literal backslash-n) so multi-line
/// drafts round-trip. Legacy single-line history files written by
/// rustyline load unchanged because a line without escape sequences
/// decodes to itself.
#[derive(Debug, Default)]
pub(crate) struct History {
    entries: Vec<String>,
    ignore_space: bool,
}

impl History {
    /// Empty history with the `history_ignore_space` policy (leading-space
    /// lines are not recorded) matching the old rustyline config.
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::new(),
            ignore_space: true,
        }
    }

    /// Load history from `path`, decoding the escaped multi-line format
    /// and tolerating a missing file (returns empty history).
    pub(crate) fn load(path: &Path) -> Self {
        let mut hist = Self::new();
        if let Ok(contents) = std::fs::read_to_string(path) {
            for line in contents.lines() {
                if line.is_empty() {
                    continue;
                }
                hist.entries.push(decode_entry(line));
            }
        }
        hist
    }

    /// All entries, oldest → newest. Used to seed [`PromptEditor`].
    pub(crate) fn entries(&self) -> &[String] {
        &self.entries
    }

    /// Record `entry`. No-ops for empty/whitespace-only input, for input
    /// that starts with a space (when `ignore_space`), and for an exact
    /// duplicate of the most recent entry (consecutive dedup).
    pub(crate) fn add(&mut self, entry: &str) {
        if entry.trim().is_empty() {
            return;
        }
        if self.ignore_space && entry.starts_with(' ') {
            return;
        }
        if self.entries.last().map(String::as_str) == Some(entry) {
            return;
        }
        self.entries.push(entry.to_owned());
        if self.entries.len() > HISTORY_MAX {
            let overflow = self.entries.len() - HISTORY_MAX;
            self.entries.drain(0..overflow);
        }
    }

    /// Persist to `path`, creating the parent directory if needed.
    /// Best-effort: IO errors are swallowed (history is a convenience,
    /// not correctness-critical).
    pub(crate) fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let mut out = String::new();
        for entry in &self.entries {
            out.push_str(&encode_entry(entry));
            out.push('\n');
        }
        let _ = std::fs::write(path, out);
    }
}

/// Escape an entry for single-line on-disk storage.
fn encode_entry(entry: &str) -> String {
    let mut out = String::with_capacity(entry.len());
    for ch in entry.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => {} // normalise CR away; CRLF drafts store as LF
            c => out.push(c),
        }
    }
    out
}

/// Inverse of [`encode_entry`]. Unrecognised escapes are passed through
/// verbatim so legacy plain lines survive.
fn decode_entry(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.peek() {
                Some('\\') => {
                    out.push('\\');
                    chars.next();
                }
                Some('n') => {
                    out.push('\n');
                    chars.next();
                }
                _ => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}

// ─── Read loop ─────────────────────────────────────────────────────

/// Everything the read loop needs from the CLI, bundled so the call site
/// stays readable. The completion / hint providers borrow the REPL's
/// command tables (see [`crate::editor_completion`] / [`crate::editor_hint`]).
pub(crate) struct PromptConfig<'a> {
    /// Styled prompt prefix painted on the first visual row.
    pub prompt: &'a str,
    pub use_color: bool,
    /// History snapshot, oldest → newest, for Up/Down recall.
    pub history: &'a [String],
    pub completion: &'a dyn Fn(&str, usize) -> CompletionResult,
    pub hint: &'a dyn Fn(&str, usize) -> Option<String>,
}

/// How a single [`read_prompt`] call resolved.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PromptOutcome {
    /// User submitted a line (Enter). Verbatim, possibly multi-line.
    Line(String),
    /// Ctrl-D on an empty buffer — caller finalizes + exits.
    Eof,
    /// Ctrl-C on an empty buffer — caller echoes `^C` and re-prompts.
    Interrupted,
    /// Shift+Tab — caller cycles the permission mode and re-prompts.
    ModeCycle,
}

/// Read one (possibly multi-line) line from the user, owning the cursor
/// and the dynamic footer for the duration. On Unix with a working TTY
/// this runs the custom raw-mode editor; otherwise it falls back to a
/// minimal cooked `read_line`.
pub(crate) fn read_prompt(
    bar: &mut AnchoredStatusBar,
    lock: &Mutex<()>,
    cfg: &PromptConfig,
) -> PromptOutcome {
    #[cfg(unix)]
    {
        if let Some(guard) = crate::queue_editor::enter_raw_mode() {
            return read_prompt_raw(bar, lock, cfg, guard);
        }
    }
    read_prompt_cooked(cfg)
}

/// Cooked-mode fallback: print the prompt and read a single line. No
/// raw-mode editing, history, or Shift+Tab — used when stdin isn't a TTY
/// or termios is unavailable (the caller gates richer behaviour on color
/// / TTY anyway).
pub(crate) fn read_prompt_cooked(cfg: &PromptConfig) -> PromptOutcome {
    let mut out = std::io::stdout();
    let _ = out.write_all(cfg.prompt.as_bytes());
    let _ = out.flush();
    let mut line = String::new();
    match std::io::stdin().read_line(&mut line) {
        Ok(0) => PromptOutcome::Eof,
        Ok(_) => {
            // Trim the trailing newline the kernel line discipline adds.
            while line.ends_with('\n') || line.ends_with('\r') {
                line.pop();
            }
            PromptOutcome::Line(line)
        }
        Err(_) => PromptOutcome::Eof,
    }
}

#[cfg(unix)]
fn read_prompt_raw(
    bar: &mut AnchoredStatusBar,
    lock: &Mutex<()>,
    cfg: &PromptConfig,
    mut guard: crate::queue_editor::RawModeGuard,
) -> PromptOutcome {
    let orig_footer = bar.footer_rows();

    // Enable bracketed paste so multi-line pastes arrive as one literal
    // block instead of a flurry of Enter keypresses.
    {
        let mut out = std::io::stdout();
        let _ = out.write_all(b"\x1b[?2004h");
        let _ = out.flush();
    }

    let mut ed = PromptEditor::new(cfg.prompt, bar.cols());
    ed.set_history(cfg.history.to_vec());
    let completion = cfg.completion;
    ed.set_completion(Box::new(move |line: &str, pos: usize| {
        completion(line, pos)
    }));

    // `prev_top` starts at the status row so the first frame's clear band
    // is just the input area (nothing stale above it yet).
    let mut prev_top = bar.rows();
    repaint(&ed, bar, lock, cfg, &mut prev_top);

    let mut bytes = [0u8; 256];
    let outcome = loop {
        match crate::queue_editor::poll_stdin(50) {
            Ok(true) => {}
            Ok(false) => {
                if bar.sync_size() {
                    ed.set_cols(bar.cols());
                    prev_top = bar.rows();
                    repaint(&ed, bar, lock, cfg, &mut prev_top);
                }
                continue;
            }
            Err(_) => continue,
        }
        let n = match crate::queue_editor::read_burst(&mut bytes) {
            Ok(0) => continue,
            Ok(n) => n,
            Err(_) => break PromptOutcome::Eof,
        };
        let mut dirty = false;
        let mut terminal: Option<PromptOutcome> = None;
        for &b in &bytes[..n] {
            match ed.consume(b) {
                EditOutcome::Pending => {}
                EditOutcome::Redraw => dirty = true,
                EditOutcome::Submit(s) => {
                    terminal = Some(PromptOutcome::Line(s));
                    break;
                }
                EditOutcome::Interrupt => {
                    terminal = Some(PromptOutcome::Interrupted);
                    break;
                }
                EditOutcome::Eof => {
                    terminal = Some(PromptOutcome::Eof);
                    break;
                }
                EditOutcome::ModeCycle => {
                    terminal = Some(PromptOutcome::ModeCycle);
                    break;
                }
            }
        }
        // A multi-candidate Tab already inserted the longest common
        // prefix and surfaced the options via the inline hint palette;
        // drain the stashed listing so it doesn't accumulate.
        let _ = ed.take_listing();
        if let Some(out) = terminal {
            break out;
        }
        if dirty {
            repaint(&ed, bar, lock, cfg, &mut prev_top);
        }
    };

    // Restore the terminal: disable bracketed paste, drop raw mode, and
    // hand the static footer + scroll region back to the streaming-phase
    // machinery with the cursor parked at the scroll-region bottom.
    {
        let mut out = std::io::stdout();
        let _ = out.write_all(b"\x1b[?2004l");
        let _ = out.flush();
    }
    guard.restore();
    bar.reset_region_to_footer(orig_footer, lock);
    outcome
}

/// Repaint the editor's current state into the dynamic footer.
#[cfg(unix)]
fn repaint(
    ed: &PromptEditor,
    bar: &mut AnchoredStatusBar,
    lock: &Mutex<()>,
    cfg: &PromptConfig,
    prev_top: &mut u16,
) {
    // Inline hint (dim ghost text). Suppressed mid-paste so a streaming
    // paste doesn't flicker palette suggestions.
    let hint = if ed.in_paste() {
        String::new()
    } else {
        match (cfg.hint)(ed.buffer(), ed.cursor()) {
            Some(h) if cfg.use_color => format!("\x1b[2;90m{h}\x1b[0m"),
            Some(h) => h,
            None => String::new(),
        }
    };
    let l = ed.layout(&hint);
    let region = bar.set_input_region(l.height() as u16, lock);

    // When the draft is taller than the (capped) input area, show a
    // window of rows ending at the caret so the caret line stays visible
    // — the editor "scrolls" within its own region.
    let max = region.input_rows as usize;
    let (rows, caret_row) = if l.rows.len() > max {
        let start = l
            .caret_row
            .saturating_sub(max.saturating_sub(1))
            .min(l.rows.len() - max);
        (l.rows[start..start + max].to_vec(), l.caret_row - start)
    } else {
        (l.rows, l.caret_row)
    };

    let clear_from = (*prev_top).min(region.top_row);
    bar.paint_input_region(region, &rows, caret_row, l.caret_col, clear_from, lock);
    *prev_top = region.top_row;
}

#[cfg(test)]
mod history_tests {
    use super::*;

    #[test]
    fn add_dedups_consecutive_and_ignores_space_and_empty() {
        let mut h = History::new();
        h.add("first");
        h.add("first"); // consecutive dup → skipped
        h.add("   "); // whitespace only → skipped
        h.add(" secret"); // leading space → skipped (ignore_space)
        h.add("second");
        h.add("first"); // non-consecutive dup → kept
        assert_eq!(h.entries(), &["first", "second", "first"]);
    }

    #[test]
    fn multiline_entry_round_trips_through_encode_decode() {
        let entry = "line1\nline2 with a \\ backslash";
        let encoded = encode_entry(entry);
        assert!(
            !encoded.contains('\n'),
            "newline must be escaped: {encoded:?}"
        );
        assert_eq!(decode_entry(&encoded), entry);
    }

    #[test]
    fn legacy_plain_line_decodes_to_itself() {
        // A pre-existing rustyline entry with no escape sequences.
        assert_eq!(decode_entry("plain command --flag"), "plain command --flag");
    }

    #[test]
    fn save_then_load_preserves_entries_including_multiline() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("history");
        let mut h = History::new();
        h.add("simple");
        h.add("multi\nline");
        h.save(&path);

        let loaded = History::load(&path);
        assert_eq!(loaded.entries(), &["simple", "multi\nline"]);
    }

    #[test]
    fn load_missing_file_yields_empty_history() {
        let h = History::load(Path::new("/nonexistent/deeptide/history"));
        assert!(h.entries().is_empty());
    }
}

#[cfg(test)]
mod editor_tests {
    use super::*;

    fn feed(ed: &mut PromptEditor, bytes: &[u8]) -> EditOutcome {
        let mut last = EditOutcome::Pending;
        for &b in bytes {
            last = ed.consume(b);
        }
        last
    }

    #[test]
    fn typing_inserts_and_advances() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"hello");
        assert_eq!(ed.buffer(), "hello");
        assert_eq!(ed.cursor(), 5);
    }

    #[test]
    fn enter_submits_buffer_verbatim() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"hi");
        match ed.consume(b'\r') {
            EditOutcome::Submit(s) => assert_eq!(s, "hi"),
            other => panic!("expected Submit, got {other:?}"),
        }
    }

    #[test]
    fn ctrl_j_inserts_newline_enter_submits_multiline() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"a");
        ed.consume(b'\n'); // Ctrl-J → newline insert
        feed(&mut ed, b"b");
        assert_eq!(ed.buffer(), "a\nb");
        match ed.consume(b'\r') {
            EditOutcome::Submit(s) => assert_eq!(s, "a\nb"),
            other => panic!("expected Submit, got {other:?}"),
        }
    }

    #[test]
    fn alt_enter_inserts_newline() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"a");
        feed(&mut ed, &[0x1b, b'\r']); // Alt+Enter
        feed(&mut ed, b"b");
        assert_eq!(ed.buffer(), "a\nb");
    }

    #[test]
    fn backspace_and_delete_are_grapheme_aware() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, "❤️x".as_bytes()); // heart+VS16, then x
        // Cursor at end; Backspace removes 'x'.
        ed.consume(0x7f);
        assert_eq!(ed.buffer(), "❤️");
        // Another Backspace removes heart + variation selector in one go.
        ed.consume(0x7f);
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn ctrl_c_clears_then_signals_interrupt_when_empty() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"draft");
        assert_eq!(ed.consume(0x03), EditOutcome::Redraw);
        assert_eq!(ed.buffer(), "");
        assert_eq!(ed.consume(0x03), EditOutcome::Interrupt);
    }

    #[test]
    fn ctrl_d_deletes_when_nonempty_eofs_when_empty() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"ab");
        // Move to start, Ctrl-D deletes 'a'.
        ed.consume(0x01); // Home
        assert_eq!(ed.consume(0x04), EditOutcome::Redraw);
        assert_eq!(ed.buffer(), "b");
        ed.consume(0x04); // delete 'b'
        assert_eq!(ed.buffer(), "");
        assert_eq!(ed.consume(0x04), EditOutcome::Eof);
    }

    #[test]
    fn shift_tab_signals_mode_cycle() {
        let mut ed = PromptEditor::new("> ", 80);
        assert_eq!(feed(&mut ed, b"\x1b[Z"), EditOutcome::ModeCycle);
    }

    #[test]
    fn ctrl_u_clears_buffer() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"hello world");
        assert_eq!(ed.consume(0x15), EditOutcome::Redraw);
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn ctrl_w_kills_previous_word() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"foo bar baz");
        assert_eq!(ed.consume(0x17), EditOutcome::Redraw);
        assert_eq!(ed.buffer(), "foo bar ");
        ed.consume(0x17);
        assert_eq!(ed.buffer(), "foo ");
    }

    #[test]
    fn home_end_within_logical_line() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"abc");
        ed.consume(0x01); // Home
        assert_eq!(ed.cursor(), 0);
        ed.consume(0x05); // End
        assert_eq!(ed.cursor(), 3);

        // Multi-line: Home/End operate on the current logical line.
        let mut ed2 = PromptEditor::new("> ", 80);
        feed(&mut ed2, b"ab");
        ed2.consume(b'\n');
        feed(&mut ed2, b"cd"); // buffer "ab\ncd", cursor at end (5)
        ed2.consume(0x01); // Home → start of "cd" = offset 3
        assert_eq!(ed2.cursor(), 3);
        ed2.consume(0x05); // End → 5
        assert_eq!(ed2.cursor(), 5);
    }

    #[test]
    fn up_down_navigate_visual_rows_of_multiline_draft() {
        let mut ed = PromptEditor::new("> ", 80);
        feed(&mut ed, b"line1");
        ed.consume(b'\n');
        feed(&mut ed, b"line2"); // cursor at end of line2
        // Up → moves onto row 0 (line1), near the same column.
        assert_eq!(ed.consume(b'\x1b'), EditOutcome::Pending);
        ed.consume(b'[');
        assert_eq!(ed.consume(b'A'), EditOutcome::Redraw);
        // Caret should now be somewhere on line1 (offset <= 5).
        assert!(ed.cursor() <= 5, "cursor={}", ed.cursor());
    }

    #[test]
    fn up_recalls_history_when_on_first_row() {
        let mut ed = PromptEditor::new("> ", 80);
        ed.set_history(vec!["older".into(), "newer".into()]);
        // Up from empty first row → newest history entry.
        feed(&mut ed, b"\x1b[A");
        assert_eq!(ed.buffer(), "newer");
        feed(&mut ed, b"\x1b[A");
        assert_eq!(ed.buffer(), "older");
        // Down returns toward newer, then back to the (empty) stashed draft.
        feed(&mut ed, b"\x1b[B");
        assert_eq!(ed.buffer(), "newer");
        feed(&mut ed, b"\x1b[B");
        assert_eq!(ed.buffer(), "");
    }

    #[test]
    fn completion_single_candidate_applies() {
        let mut ed = PromptEditor::new("> ", 80);
        ed.set_completion(Box::new(|line: &str, pos: usize| {
            let _ = (line, pos);
            CompletionResult {
                start: 0,
                candidates: vec!["/exit".to_owned()],
            }
        }));
        feed(&mut ed, b"/exi");
        assert_eq!(ed.consume(b'\t'), EditOutcome::Redraw);
        assert_eq!(ed.buffer(), "/exit");
    }

    #[test]
    fn completion_multi_candidate_inserts_lcp_and_lists() {
        let mut ed = PromptEditor::new("> ", 80);
        ed.set_completion(Box::new(|_line, _pos| CompletionResult {
            start: 0,
            candidates: vec!["/model".to_owned(), "/mode".to_owned()],
        }));
        feed(&mut ed, b"/m");
        ed.consume(b'\t');
        // LCP of "/model" and "/mode" is "/mode" (the shorter is a prefix
        // of the longer), so the editor fills in up to there.
        assert_eq!(ed.buffer(), "/mode");
        let listing = ed.take_listing().expect("listing surfaced");
        assert_eq!(listing.len(), 2);
    }

    #[test]
    fn bracketed_paste_inserts_literal_multiline() {
        let mut ed = PromptEditor::new("> ", 80);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"\x1b[200~");
        bytes.extend_from_slice(b"a\nb");
        bytes.extend_from_slice(b"\x1b[201~");
        feed(&mut ed, &bytes);
        assert_eq!(ed.buffer(), "a\nb");
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;

    #[test]
    fn single_short_line_is_one_row_with_prompt() {
        let l = layout("> ", "hello", 5, 80, "");
        assert_eq!(l.rows, vec!["> hello".to_owned()]);
        assert_eq!(l.height(), 1);
        // Caret at end: after "> hello" = 2 + 5 = 7.
        assert_eq!((l.caret_row, l.caret_col), (0, 7));
    }

    #[test]
    fn caret_in_middle_reports_prompt_offset_column() {
        // cursor=2 → between "he" and "llo".
        let l = layout("> ", "hello", 2, 80, "");
        assert_eq!((l.caret_row, l.caret_col), (0, 4)); // 2 (prompt) + 2
    }

    #[test]
    fn ascii_wraps_at_cols() {
        // cols=10, prompt width 2 → 8 cols for text on row 0.
        // "abcdefghij" (10 chars): 8 fit on row 0, 2 wrap to row 1.
        let l = layout("> ", "abcdefghij", 10, 10, "");
        assert_eq!(l.rows[0], "> abcdefgh");
        assert_eq!(l.rows[1], "ij");
        assert_eq!(l.height(), 2);
        // Caret at end → row 1, col 2.
        assert_eq!((l.caret_row, l.caret_col), (1, 2));
    }

    #[test]
    fn explicit_newline_starts_new_row_without_prompt() {
        let l = layout("> ", "a\nb", 3, 80, "");
        assert_eq!(l.rows, vec!["> a".to_owned(), "b".to_owned()]);
        assert_eq!((l.caret_row, l.caret_col), (1, 1));
    }

    #[test]
    fn wide_cjk_char_wraps_as_a_unit() {
        // cols=5, prompt "> " width 2 → 3 text cols on row 0.
        // "你好" each width 2: 你 fits (col 2→4), 好 would be 4→6 > 5 so
        // wraps to row 1.
        let l = layout("> ", "你好", "你好".len(), 5, "");
        assert_eq!(l.rows[0], "> 你");
        assert_eq!(l.rows[1], "好");
        assert_eq!(l.height(), 2);
    }

    #[test]
    fn trailing_wide_char_at_boundary_wraps() {
        // prompt width 2, cols=4 → 2 text cols. A width-2 char fills
        // exactly cols (col 2→4). Next char wraps.
        let l = layout("> ", "你你", "你你".len(), 4, "");
        assert_eq!(l.rows[0], "> 你");
        assert_eq!(l.rows[1], "你");
    }

    #[test]
    fn hint_appended_only_at_end_of_buffer() {
        let l = layout("> ", "ex", 2, 80, "it");
        assert_eq!(l.rows[0], "> exit");
        // Caret stays before the hint.
        assert_eq!((l.caret_row, l.caret_col), (0, 4));

        // With caret NOT at end, no hint is appended.
        let l2 = layout("> ", "exit", 1, 80, "HINT");
        assert!(!l2.rows[0].contains("HINT"));
    }

    #[test]
    fn empty_buffer_is_one_row_with_caret_after_prompt() {
        let l = layout("deeptide> ", "", 0, 80, "");
        assert_eq!(l.height(), 1);
        assert_eq!(l.caret_row, 0);
        assert_eq!(l.caret_col, display_width("deeptide> "));
    }

    #[test]
    fn caret_at_exact_line_end_wraps_to_next_row() {
        // prompt width 2, cols=5 → 3 text cols. "abc" fills cols exactly
        // (col 2→5). Caret at end should wrap to row 1 col 0.
        let l = layout("> ", "abc", 3, 5, "");
        assert_eq!(l.rows[0], "> abc");
        assert_eq!((l.caret_row, l.caret_col), (1, 0));
    }
}
