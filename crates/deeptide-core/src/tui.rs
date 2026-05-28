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
pub struct InputBar {
    pub prompt: String,
    pub value: String,
    pub cursor: usize,
    pub hint: Option<String>,
}

impl InputBar {
    pub fn render(&self, width: usize) -> Vec<String> {
        let width = width.max(12);
        let mut content = if self.value.is_empty() {
            self.hint
                .as_ref()
                .map(|hint| format!("{}{}", self.prompt, hint))
                .unwrap_or_else(|| self.prompt.clone())
        } else {
            format!("{}{}", self.prompt, self.value)
        };

        if content.is_empty() {
            content = self.prompt.clone();
        }

        wrap_line(&content, width)
    }
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

fn wrap_line(line: &str, width: usize) -> Vec<String> {
    if display_width(line) <= width {
        return vec![line.to_owned()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = 0;

    for word in line.split_inclusive(' ') {
        let word_width = display_width(word);
        if current_width > 0 && current_width + word_width > width {
            lines.push(current.trim_end().to_owned());
            current.clear();
            current_width = 0;
        }

        if word_width > width {
            for ch in word.chars() {
                let ch_width = char_width(ch);
                if current_width > 0 && current_width + ch_width > width {
                    lines.push(current.trim_end().to_owned());
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
        lines.push(current.trim_end().to_owned());
    }
    lines
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

fn display_width(text: &str) -> usize {
    strip_ansi(text).chars().map(char_width).sum()
}

fn char_width(ch: char) -> usize {
    if is_wide(ch) { 2 } else { 1 }
}

fn is_wide(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1100..=0x115F
            | 0x2329..=0x232A
            | 0x2E80..=0xA4CF
            | 0xAC00..=0xD7A3
            | 0xF900..=0xFAFF
            | 0xFE10..=0xFE19
            | 0xFE30..=0xFE6F
            | 0xFF00..=0xFF60
            | 0xFFE0..=0xFFE6
    )
}

fn strip_ansi(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{
        InputBar, StatusLine, StatusSegment, TranscriptItem, TuiFrame, render_output_panel,
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
            cursor: 0,
            hint: None,
        };

        let rendered = input.render(20);
        assert!(rendered.len() > 1);
        assert!(rendered[0].starts_with("deeptide>"));
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
}
