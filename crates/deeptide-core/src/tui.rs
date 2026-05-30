use crate::{MarkdownRenderOptions, MarkdownRenderer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusSegment {
    pub label: String,
    pub value: String,
}

impl StatusSegment {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }

    fn render(&self) -> String {
        if self.label.is_empty() {
            self.value.clone()
        } else {
            format!("{} {}", self.label, self.value)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusLine {
    segments: Vec<StatusSegment>,
}

impl StatusLine {
    pub fn new(segments: impl IntoIterator<Item = StatusSegment>) -> Self {
        Self {
            segments: segments.into_iter().collect(),
        }
    }

    pub fn render(&self, width: usize) -> String {
        let width = width.max(12);
        let mut out = String::from(" ");

        for segment in &self.segments {
            let rendered = segment.render();
            let separator = if out.trim().is_empty() { "" } else { "  |  " };
            let candidate = format!("{out}{separator}{rendered}");
            if display_width(&candidate) > width {
                break;
            }
            out = candidate;
        }

        if display_width(&out) > width {
            truncate_to_width(&out, width)
        } else {
            pad_to_width(out, width)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputState {
    value: String,
    cursor: usize,
}

impl InputState {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self { value, cursor }
    }

    pub fn empty() -> Self {
        Self {
            value: String::new(),
            cursor: 0,
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn is_empty(&self) -> bool {
        self.value.is_empty()
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor = 0;
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.char_len());
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.char_len();
    }

    pub fn insert_char(&mut self, ch: char) {
        self.insert_str(&ch.to_string());
    }

    pub fn insert_newline(&mut self) {
        self.insert_char('\n');
    }

    pub fn insert_str(&mut self, text: &str) {
        let byte_index = self.byte_index(self.cursor);
        self.value.insert_str(byte_index, text);
        self.cursor += text.chars().count();
    }

    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }

        let start = self.byte_index(self.cursor - 1);
        let end = self.byte_index(self.cursor);
        self.value.replace_range(start..end, "");
        self.cursor -= 1;
        true
    }

    pub fn delete(&mut self) -> bool {
        if self.cursor >= self.char_len() {
            return false;
        }

        let start = self.byte_index(self.cursor);
        let end = self.byte_index(self.cursor + 1);
        self.value.replace_range(start..end, "");
        true
    }

    pub fn input_bar(&self, prompt: impl Into<String>, hint: Option<String>) -> InputBar {
        InputBar {
            prompt: prompt.into(),
            value: self.value.clone(),
            cursor: self.cursor,
            hint,
        }
    }

    fn char_len(&self) -> usize {
        self.value.chars().count()
    }

    fn byte_index(&self, char_index: usize) -> usize {
        if char_index == self.char_len() {
            return self.value.len();
        }
        self.value
            .char_indices()
            .nth(char_index)
            .map(|(index, _)| index)
            .unwrap_or(self.value.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputBar {
    pub prompt: String,
    pub value: String,
    /// Cursor position measured in Unicode scalar values within `value`.
    pub cursor: usize,
    pub hint: Option<String>,
}

impl InputBar {
    pub fn render(&self, width: usize) -> Vec<String> {
        self.layout(width).lines
    }

    pub fn layout(&self, width: usize) -> InputLayout {
        let width = width.max(12);
        let cursor = self.cursor.min(self.value.chars().count());
        let mut content = self.prompt.clone();
        let cursor_char = if self.value.is_empty() {
            content.chars().count()
        } else {
            content.push_str(&take_chars(&self.value, cursor));
            let char_index = content.chars().count();
            content.push_str(&skip_chars(&self.value, cursor));
            char_index
        };

        if self.value.is_empty()
            && let Some(hint) = self.hint.as_ref()
        {
            content.push_str(hint);
        }

        if content.is_empty() {
            content = self.prompt.clone();
        }

        let wrapped = wrap_text_with_cursor(&content, width, cursor_char);
        InputLayout {
            lines: wrapped.lines,
            cursor_row: wrapped.cursor_row,
            cursor_col: wrapped.cursor_col,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputLayout {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TranscriptKind {
    Assistant,
    User,
    Tool,
    System,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TranscriptItem {
    pub kind: TranscriptKind,
    pub content: String,
}

impl TranscriptItem {
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            kind: TranscriptKind::Assistant,
            content: content.into(),
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            kind: TranscriptKind::User,
            content: content.into(),
        }
    }

    pub fn tool(content: impl Into<String>) -> Self {
        Self {
            kind: TranscriptKind::Tool,
            content: content.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiFrame {
    pub transcript: Vec<TranscriptItem>,
    pub status: StatusLine,
    pub input: InputBar,
}

impl TuiFrame {
    pub fn render(&self, width: usize, height: usize) -> String {
        let width = width.max(12);
        let height = height.max(3);
        let input_lines = self.input.render(width);
        let transcript_budget = height.saturating_sub(input_lines.len() + 1);
        let mut transcript_lines = render_transcript(&self.transcript, width);

        if transcript_lines.len() > transcript_budget {
            let keep_from = transcript_lines.len() - transcript_budget;
            transcript_lines = transcript_lines.split_off(keep_from);
        }

        let mut lines = transcript_lines;
        while lines.len() < transcript_budget {
            lines.insert(0, String::new());
        }
        lines.push(self.status.render(width));
        lines.extend(input_lines);
        lines.join("\n")
    }
}

pub fn render_output_panel(markdown: &str, width: usize, color: bool) -> String {
    let rendered = MarkdownRenderer::render_with_options(markdown, MarkdownRenderOptions { color });
    wrap_preserving_newlines(&rendered, width.max(12)).join("\n")
}

fn render_transcript(items: &[TranscriptItem], width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for item in items {
        let prefix = match item.kind {
            TranscriptKind::Assistant => "assistant",
            TranscriptKind::User => "you",
            TranscriptKind::Tool => "tool",
            TranscriptKind::System => "system",
        };
        lines.push(format!("{prefix}>"));
        lines.extend(wrap_preserving_newlines(
            &MarkdownRenderer::render_with_options(
                &item.content,
                MarkdownRenderOptions { color: false },
            ),
            width,
        ));
        lines.push(String::new());
    }
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

fn wrap_preserving_newlines(text: &str, width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    text.lines()
        .flat_map(|line| wrap_line(line, width))
        .collect::<Vec<_>>()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WrappedLine {
    lines: Vec<String>,
    cursor_row: usize,
    cursor_col: usize,
}

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    wrap_line_with_cursor(line, width, usize::MAX).lines
}

fn wrap_text_with_cursor(text: &str, width: usize, cursor_char: usize) -> WrappedLine {
    let mut lines = Vec::new();
    let mut cursor_position = None;
    let mut segment = String::new();
    let mut segment_start = 0;
    let cursor_char = cursor_char.min(text.chars().count());

    for (char_index, ch) in text.chars().enumerate() {
        if ch == '\n' {
            append_wrapped_segment(
                &mut lines,
                &mut cursor_position,
                &segment,
                width,
                segment_start,
                cursor_char,
            );
            segment.clear();
            segment_start = char_index + 1;
        } else {
            segment.push(ch);
        }
    }

    append_wrapped_segment(
        &mut lines,
        &mut cursor_position,
        &segment,
        width,
        segment_start,
        cursor_char,
    );

    let last_row = lines.len().saturating_sub(1);
    let fallback_col = lines.last().map(|line| display_width(line)).unwrap_or(0);
    let (cursor_row, cursor_col) = cursor_position.unwrap_or((last_row, fallback_col));
    WrappedLine {
        lines,
        cursor_row,
        cursor_col,
    }
}

fn append_wrapped_segment(
    lines: &mut Vec<String>,
    cursor_position: &mut Option<(usize, usize)>,
    segment: &str,
    width: usize,
    segment_start: usize,
    cursor_char: usize,
) {
    let segment_len = segment.chars().count();
    let cursor_cell = if cursor_char >= segment_start && cursor_char <= segment_start + segment_len
    {
        display_width(&take_chars(segment, cursor_char - segment_start))
    } else {
        usize::MAX
    };

    let wrapped = wrap_line_with_cursor(segment, width, cursor_cell);
    let row_offset = lines.len();
    if cursor_cell != usize::MAX && cursor_position.is_none() {
        *cursor_position = Some((row_offset + wrapped.cursor_row, wrapped.cursor_col));
    }
    lines.extend(wrapped.lines);
}

fn wrap_line_with_cursor(line: &str, width: usize, cursor_cell: usize) -> WrappedLine {
    if display_width(line) <= width {
        let cursor_col = cursor_cell.min(display_width(line));
        let mut lines = vec![line.to_owned()];
        let (cursor_row, cursor_col) =
            normalize_cursor_position(&mut lines, width, cursor_cell, 0, cursor_col);
        return WrappedLine {
            lines,
            cursor_row,
            cursor_col,
        };
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;
    let mut consumed_width = 0;
    let mut cursor_position = None;

    for word in line.split_inclusive(' ') {
        let word_width = display_width(word);
        let flow_by_char = word_width > width
            || current_width > 0
                && current_width + word_width > width
                && is_unspaced_wide_text(word);
        if current_width > 0 && current_width + word_width > width && !flow_by_char {
            if cursor_position.is_none() && cursor_cell <= consumed_width + current_width {
                cursor_position = Some((lines.len(), cursor_cell.saturating_sub(consumed_width)));
            }
            lines.push(current.trim_end().to_owned());
            consumed_width += current_width;
            current.clear();
            current_width = 0;
        }

        if flow_by_char {
            for ch in word.chars() {
                let ch_width = char_width(ch);
                if current_width > 0 && current_width + ch_width > width {
                    if cursor_position.is_none() && cursor_cell <= consumed_width + current_width {
                        cursor_position =
                            Some((lines.len(), cursor_cell.saturating_sub(consumed_width)));
                    }
                    lines.push(current.trim_end().to_owned());
                    consumed_width += current_width;
                    current.clear();
                    current_width = 0;
                }
                current.push(ch);
                current_width += ch_width;
            }
        } else {
            current.push_str(word);
            current_width += word_width;
        }
    }

    if !current.is_empty() {
        if cursor_position.is_none() && cursor_cell <= consumed_width + current_width {
            cursor_position = Some((lines.len(), cursor_cell.saturating_sub(consumed_width)));
        }
        lines.push(current.trim_end().to_owned());
    }

    let last_row = lines.len().saturating_sub(1);
    let fallback_col = lines.last().map(|line| display_width(line)).unwrap_or(0);
    let (cursor_row, cursor_col) = cursor_position.unwrap_or((last_row, fallback_col));
    let (cursor_row, cursor_col) =
        normalize_cursor_position(&mut lines, width, cursor_cell, cursor_row, cursor_col);
    WrappedLine {
        lines,
        cursor_row,
        cursor_col,
    }
}

fn normalize_cursor_position(
    lines: &mut Vec<String>,
    width: usize,
    cursor_cell: usize,
    cursor_row: usize,
    cursor_col: usize,
) -> (usize, usize) {
    if cursor_cell == usize::MAX || cursor_col < width {
        return (cursor_row, cursor_col);
    }

    let next_row = cursor_row + 1;
    if next_row == lines.len() {
        lines.push(String::new());
    }
    (next_row, 0)
}

fn pad_to_width(mut value: String, width: usize) -> String {
    let current = display_width(&value);
    if current < width {
        value.push_str(&" ".repeat(width - current));
    }
    value
}

fn truncate_to_width(value: &str, width: usize) -> String {
    if width <= 1 {
        return "…".to_owned();
    }

    let mut out = String::new();
    let mut used = 0;
    for ch in value.chars() {
        let ch_width = char_width(ch);
        if used + ch_width >= width {
            break;
        }
        out.push(ch);
        used += ch_width;
    }
    out.push('…');
    out
}

use crate::width::{char_width, display_width, is_wide};

fn is_unspaced_wide_text(value: &str) -> bool {
    let trimmed = value.trim_end_matches(' ');
    !trimmed.is_empty() && !trimmed.chars().any(char::is_whitespace) && trimmed.chars().any(is_wide)
}

fn take_chars(value: &str, count: usize) -> String {
    value.chars().take(count).collect()
}

fn skip_chars(value: &str, count: usize) -> String {
    value.chars().skip(count).collect()
}

// ── Activity spinner ────────────────────────────────────────────────────────
//
// The interactive REPL runs each turn synchronously: model "thinking" and tool
// execution happen with no output until the assistant streams text. These pure
// helpers render a single-line braille spinner the CLI animates on a background
// thread so the terminal isn't silent while work is in flight. The CLI owns the
// carriage-return / line-clear control codes; this module only builds the
// visible string so the frame/verb/elapsed logic stays testable.

/// Braille spinner frames, advanced once per animation tick.
const SPINNER_FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Rotating activity verbs — changed every few seconds for a little life
/// without churning so fast it distracts.
const SPINNER_VERBS: [&str; 6] = [
    "Working",
    "Thinking",
    "Pondering",
    "Crunching",
    "Churning",
    "Brewing",
];

/// The spinner glyph for animation tick `tick` (wraps around the frame set).
pub fn spinner_frame(tick: usize) -> char {
    SPINNER_FRAMES[tick % SPINNER_FRAMES.len()]
}

/// The activity verb shown at `elapsed_secs`, rotating every 4 seconds.
pub fn spinner_verb(elapsed_secs: u64) -> &'static str {
    let index = (elapsed_secs / 4) as usize % SPINNER_VERBS.len();
    SPINNER_VERBS[index]
}

/// Render one spinner status line, e.g. `⠹ Thinking… (6s)`. The elapsed suffix
/// is omitted in the first second so a quick turn doesn't flash "(0s)".
pub fn render_spinner_line(tick: usize, elapsed_secs: u64) -> String {
    render_spinner_line_with_phase(tick, elapsed_secs, None)
}

/// Same as [`render_spinner_line`] but with an optional in-flight
/// phase descriptor — typically the running tool (`Bash(npm test)`,
/// `Read(/etc/hosts)`) so users know *what* is taking wall-clock
/// time during a multi-second tool call instead of seeing only a
/// neutral verb. The phase is inserted between the verb and the
/// elapsed suffix:
///
/// ```text
/// ⠴ Working · Bash(npm test) (3s)
/// ⠦ Thinking…                        ← no phase, no elapsed suffix
/// ```
///
/// Empty / whitespace-only phases collapse to the bare verb form
/// so a transient `Some("")` from a race during phase clearing
/// won't render a stray dot.
pub fn render_spinner_line_with_phase(
    tick: usize,
    elapsed_secs: u64,
    phase: Option<&str>,
) -> String {
    let frame = spinner_frame(tick);
    let verb = spinner_verb(elapsed_secs);
    let trimmed_phase = phase.map(str::trim).filter(|s| !s.is_empty());
    match (trimmed_phase, elapsed_secs) {
        (None, 0) => format!("{frame} {verb}…"),
        (None, n) => format!("{frame} {verb}… ({n}s)"),
        (Some(phase), 0) => format!("{frame} {verb} · {phase}"),
        (Some(phase), n) => format!("{frame} {verb} · {phase} ({n}s)"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InputBar, InputState, StatusLine, StatusSegment, TranscriptItem, TuiFrame,
        render_output_panel, render_spinner_line, render_spinner_line_with_phase, spinner_frame,
        spinner_verb,
    };

    #[test]
    fn status_line_fits_requested_width() {
        let line = StatusLine::new([
            StatusSegment::new("model", "deepseek-v4-pro"),
            StatusSegment::new("mode", "accept-edits"),
            StatusSegment::new("ctx", "12%"),
        ]);

        assert_eq!(line.render(24).chars().count(), 24);
        assert!(line.render(80).contains("deepseek-v4-pro"));
    }

    #[test]
    fn input_bar_wraps_long_text() {
        let input = InputBar {
            prompt: "deeptide> ".to_owned(),
            value: "please inspect this very long prompt".to_owned(),
            cursor: 14,
            hint: None,
        };

        let layout = input.layout(20);
        assert!(layout.lines.len() > 1);
        assert!(layout.lines[0].starts_with("deeptide>"));
        assert_eq!(layout.cursor_row, 1);
        assert_eq!(layout.cursor_col, 7);
    }

    #[test]
    fn input_bar_places_cursor_after_wrapped_cjk_text() {
        let input = InputBar {
            prompt: "deeptide> ".to_owned(),
            value: "你好世界，继续推进".to_owned(),
            cursor: 6,
            hint: None,
        };

        let layout = input.layout(18);

        assert_eq!(
            layout.lines,
            vec!["deeptide> 你好世界".to_owned(), "，继续推进".to_owned()]
        );
        assert_eq!(layout.cursor_row, 1);
        assert_eq!(layout.cursor_col, 4);
    }

    #[test]
    fn input_bar_cursor_clamps_to_end_of_text() {
        let input = InputBar {
            prompt: "deeptide> ".to_owned(),
            value: "abc".to_owned(),
            cursor: 99,
            hint: None,
        };

        let layout = input.layout(80);

        assert_eq!(layout.cursor_row, 0);
        assert_eq!(layout.cursor_col, "deeptide> abc".len());
    }

    #[test]
    fn input_bar_hint_does_not_move_empty_value_cursor() {
        let input = InputBar {
            prompt: "deeptide> ".to_owned(),
            value: String::new(),
            cursor: 0,
            hint: Some("输入任务".to_owned()),
        };

        let layout = input.layout(80);

        assert_eq!(layout.lines, vec!["deeptide> 输入任务".to_owned()]);
        assert_eq!(layout.cursor_row, 0);
        assert_eq!(layout.cursor_col, "deeptide> ".len());
    }

    #[test]
    fn input_bar_places_cursor_after_explicit_newline() {
        let input = InputBar {
            prompt: "deeptide> ".to_owned(),
            value: "first\n你好".to_owned(),
            cursor: 7,
            hint: None,
        };

        let layout = input.layout(80);

        assert_eq!(
            layout.lines,
            vec!["deeptide> first".to_owned(), "你好".to_owned()]
        );
        assert_eq!(layout.cursor_row, 1);
        assert_eq!(layout.cursor_col, 2);
    }

    #[test]
    fn input_bar_moves_cursor_to_next_row_at_exact_width_boundary() {
        let input = InputBar {
            prompt: String::new(),
            value: "abcdefghijkl".to_owned(),
            cursor: 12,
            hint: None,
        };

        let layout = input.layout(12);

        assert_eq!(layout.lines, vec!["abcdefghijkl".to_owned(), String::new()]);
        assert_eq!(layout.cursor_row, 1);
        assert_eq!(layout.cursor_col, 0);
    }

    #[test]
    fn input_bar_keeps_boundary_cursor_before_wrapped_suffix() {
        let input = InputBar {
            prompt: String::new(),
            value: "abcdefghijklm".to_owned(),
            cursor: 12,
            hint: None,
        };

        let layout = input.layout(12);

        assert_eq!(
            layout.lines,
            vec!["abcdefghijkl".to_owned(), "m".to_owned()]
        );
        assert_eq!(layout.cursor_row, 1);
        assert_eq!(layout.cursor_col, 0);
    }

    #[test]
    fn input_bar_moves_wide_text_cursor_to_next_row_at_boundary() {
        let input = InputBar {
            prompt: String::new(),
            value: "你好世界测试".to_owned(),
            cursor: 6,
            hint: None,
        };

        let layout = input.layout(12);

        assert_eq!(layout.lines, vec!["你好世界测试".to_owned(), String::new()]);
        assert_eq!(layout.cursor_row, 1);
        assert_eq!(layout.cursor_col, 0);
    }

    #[test]
    fn input_state_inserts_and_moves_across_cjk_text() {
        let mut input = InputState::new("你好");

        input.move_left();
        input.insert_char('，');
        input.insert_str("世界");

        assert_eq!(input.value(), "你，世界好");
        assert_eq!(input.cursor(), 4);
    }

    #[test]
    fn input_state_backspace_and_delete_keep_unicode_boundaries() {
        let mut input = InputState::new("a你b");

        input.move_left();
        assert!(input.backspace());
        assert_eq!(input.value(), "ab");
        assert_eq!(input.cursor(), 1);
        assert!(input.delete());
        assert_eq!(input.value(), "a");
        assert!(!input.delete());
    }

    #[test]
    fn input_state_supports_newlines_and_input_bar_projection() {
        let mut input = InputState::empty();

        input.insert_str("line one");
        input.insert_newline();
        input.insert_str("第二行");

        let bar = input.input_bar("deeptide> ", None);
        let layout = bar.layout(80);

        assert_eq!(input.value(), "line one\n第二行");
        assert_eq!(input.cursor(), 12);
        assert_eq!(
            layout.lines,
            vec!["deeptide> line one".to_owned(), "第二行".to_owned()]
        );
        assert_eq!(layout.cursor_row, 1);
        assert_eq!(layout.cursor_col, 6);
    }

    #[test]
    fn frame_reserves_status_and_input_rows() {
        let frame = TuiFrame {
            transcript: vec![TranscriptItem::assistant(
                "# Hello\n\n| a | b |\n|---|---|\n| 1 | 2 |",
            )],
            status: StatusLine::new([StatusSegment::new("mode", "default")]),
            input: InputBar {
                prompt: "deeptide> ".to_owned(),
                value: String::new(),
                cursor: 0,
                hint: Some("type a task".to_owned()),
            },
        };

        let rendered = frame.render(40, 8);
        let lines = rendered.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 8);
        assert!(rendered.contains("mode default"));
        assert!(rendered.contains("deeptide> type a task"));
    }

    #[test]
    fn output_panel_uses_markdown_renderer_and_wraps() {
        let rendered = render_output_panel("**bold** words in a long line", 12, false);
        assert!(rendered.contains("bold"));
        assert!(rendered.lines().count() > 1);
    }

    #[test]
    fn spinner_frame_cycles_through_all_glyphs() {
        // Tick 0 and tick==len land on the same frame (wrap-around).
        assert_eq!(spinner_frame(0), spinner_frame(10));
        // Consecutive ticks differ, so the spinner visibly animates.
        assert_ne!(spinner_frame(0), spinner_frame(1));
    }

    #[test]
    fn spinner_verb_rotates_every_four_seconds() {
        assert_eq!(spinner_verb(0), spinner_verb(3));
        assert_ne!(spinner_verb(0), spinner_verb(4));
        // Rotation wraps after the full verb set is exhausted.
        assert_eq!(spinner_verb(0), spinner_verb(24));
    }

    #[test]
    fn spinner_line_omits_elapsed_in_first_second_then_shows_it() {
        let immediate = render_spinner_line(0, 0);
        assert!(immediate.ends_with('…'), "got: {immediate}");
        assert!(!immediate.contains("(0s)"));

        let later = render_spinner_line(3, 6);
        assert!(later.contains("(6s)"), "got: {later}");
    }

    #[test]
    fn spinner_phase_inserts_tool_descriptor_between_verb_and_elapsed() {
        let line = render_spinner_line_with_phase(0, 5, Some("Bash(npm test)"));
        assert!(line.contains("Bash(npm test)"), "phase missing: {line}");
        assert!(line.contains("(5s)"), "elapsed missing: {line}");
        // Phase replaces the ellipsis form ("Working…") with the
        // separator ("Working · Bash(…)") so the line reads as a
        // single statement.
        assert!(
            !line.contains("…"),
            "expected no ellipsis with phase: {line}"
        );
        assert!(
            line.contains(" · "),
            "expected middle-dot separator: {line}"
        );
    }

    #[test]
    fn spinner_phase_at_zero_seconds_skips_elapsed_suffix() {
        let line = render_spinner_line_with_phase(0, 0, Some("Read(file.rs)"));
        assert!(line.contains("Read(file.rs)"));
        assert!(!line.contains("(0s)"), "must hide elapsed at t=0: {line}");
    }

    #[test]
    fn spinner_phase_empty_string_collapses_to_bare_verb() {
        // A transient "Some(\"\")" can happen if the CLI clears
        // the phase under a race; we never want a stray middle
        // dot or trailing space showing up.
        let line = render_spinner_line_with_phase(0, 3, Some(""));
        assert!(line.contains("(3s)"), "elapsed still present: {line}");
        assert!(
            line.contains("…"),
            "should fall back to verb-with-ellipsis: {line}"
        );
        assert!(
            !line.contains(" · "),
            "no separator with empty phase: {line}"
        );
    }

    #[test]
    fn spinner_phase_whitespace_only_treated_as_no_phase() {
        let line = render_spinner_line_with_phase(0, 2, Some("   "));
        assert!(
            !line.contains(" · "),
            "whitespace phase should not render: {line}"
        );
    }
}
