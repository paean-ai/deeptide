#[derive(Debug, Clone, Copy)]
pub struct MarkdownRenderOptions {
    pub color: bool,
}

impl Default for MarkdownRenderOptions {
    fn default() -> Self {
        Self {
            color: std::env::var_os("NO_COLOR").is_none(),
        }
    }
}

pub struct MarkdownRenderer;

/// Incremental markdown renderer for streaming LLM output.
///
/// Buffers tokens until a newline arrives, then renders the just-completed
/// line through the full [`MarkdownRenderer`] pipeline so inline markup
/// (`**bold**`, `__bold__`, `` `code` ``, `[label](url)`, `~~strike~~`),
/// list items (`- item`, `1. item`), headers (`# Title`), and blockquotes
/// (`> quote`) all render correctly even though the model is emitting one
/// token at a time.
///
/// Multi-line constructs that depend on a closing token — fenced code blocks
/// — are passed through unchanged while we're inside them: the renderer
/// detects the opening fence and switches to verbatim mode so partial code
/// remains readable as it streams in. Tables (which need every row to compute
/// column widths) are held in a small buffer until the block ends, then
/// rendered together so columns align — rendering rows one-at-a-time would
/// produce misaligned garbage.
///
/// Trade-off: lines now appear as a unit rather than character-by-character.
/// In practice an LLM emits dozens of characters per token, so the human eye
/// still sees "live" updates roughly every 100–300 ms.
pub struct StreamingMarkdownRenderer {
    buf: String,
    in_fence: bool,
    /// Contiguous markdown table lines (`| … |`) held until the table block
    /// ends, then rendered together. A table needs every row to compute column
    /// widths, so rendering rows one-at-a-time (as the rest of the stream does)
    /// produces misaligned garbage — buffering the block fixes that.
    table_buf: Vec<String>,
    options: MarkdownRenderOptions,
}

impl StreamingMarkdownRenderer {
    pub fn new(options: MarkdownRenderOptions) -> Self {
        Self {
            buf: String::new(),
            in_fence: false,
            table_buf: Vec::new(),
            options,
        }
    }

    /// Render the buffered table block (joined) through the full block-aware
    /// renderer so columns align, ensuring a trailing newline.
    fn render_table_block(&self) -> String {
        let mut rendered =
            MarkdownRenderer::render_with_options(&self.table_buf.join("\n"), self.options);
        if !rendered.ends_with('\n') {
            rendered.push('\n');
        }
        rendered
    }

    /// Feed a streamed chunk of markdown text and return the bytes that
    /// should be written to the terminal now. The returned string consists
    /// of zero or more newline-terminated rendered lines plus, when inside a
    /// fenced code block, any verbatim trailing partial line. Partial lines
    /// outside a fence remain buffered until a newline arrives or
    /// [`Self::flush`] is called.
    pub fn push(&mut self, chunk: &str) -> String {
        self.buf.push_str(chunk);
        let mut out = String::new();

        while let Some(newline_pos) = self.buf.find('\n') {
            let mut line: String = self.buf.drain(..=newline_pos).collect();
            // Drop the trailing '\n' for analysis; we re-append it after.
            line.pop();
            let trimmed = line.trim_start();

            // A table row outside a fence — defer it so the whole block renders
            // together (column alignment needs every row).
            let is_table = !self.in_fence && trimmed.starts_with('|');

            // Any non-table line ends a pending table block: flush it first so
            // it lands above this line in order.
            if !is_table && !self.table_buf.is_empty() {
                out.push_str(&self.render_table_block());
                self.table_buf.clear();
            }

            if trimmed.starts_with("```") {
                // Fence lines themselves stay verbatim — the full block-aware
                // renderer would render an empty box around an unmatched
                // fence, which looks worse than just showing the fence text.
                self.in_fence = !self.in_fence;
                out.push_str(&line);
                out.push('\n');
                continue;
            }

            if self.in_fence {
                out.push_str(&line);
                out.push('\n');
                continue;
            }

            if is_table {
                self.table_buf.push(line);
                continue;
            }

            out.push_str(&MarkdownRenderer::render_with_options(&line, self.options));
            out.push('\n');
        }

        // When inside a fenced block, the user wants to see partial code as
        // it streams. Emit verbatim and drop the buffer so subsequent
        // chunks land fresh.
        if self.in_fence && !self.buf.is_empty() {
            out.push_str(&self.buf);
            self.buf.clear();
        }

        out
    }

    /// Drain any pending table block and trailing partial line, returning their
    /// rendered form. Safe to call repeatedly (e.g. at end-of-turn, or when a
    /// tool call interrupts streamed text so narration doesn't run on).
    pub fn flush(&mut self) -> String {
        let mut out = String::new();
        if !self.table_buf.is_empty() {
            out.push_str(&self.render_table_block());
            self.table_buf.clear();
        }
        if !self.buf.is_empty() {
            let line = std::mem::take(&mut self.buf);
            if self.in_fence {
                out.push_str(&line);
            } else {
                out.push_str(&MarkdownRenderer::render_with_options(&line, self.options));
            }
        }
        out
    }

    /// True if no buffered text and not currently inside a fenced block. The
    /// caller can use this to decide whether a trailing blank-line separator
    /// is needed after streaming completes.
    pub fn is_idle(&self) -> bool {
        self.buf.is_empty() && self.table_buf.is_empty() && !self.in_fence
    }

    /// Discard all buffered state without rendering it.
    ///
    /// Used by the REPL when an SSE stream truncates and the auto-retry
    /// kicks in: the bytes we held would have completed into something
    /// meaningful on the doomed attempt, but the retry will resend its
    /// own deltas from scratch, so keeping the partial buffer would
    /// merge two unrelated outputs and look like model corruption. Pair
    /// with [`Self::flush`] beforehand if you want the user to see how
    /// far the previous attempt got.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.table_buf.clear();
        self.in_fence = false;
    }
}

impl MarkdownRenderer {
    pub fn render(markdown: &str) -> String {
        Self::render_with_options(markdown, MarkdownRenderOptions::default())
    }

    pub fn render_with_options(markdown: &str, options: MarkdownRenderOptions) -> String {
        if markdown.is_empty() {
            return String::new();
        }
        let lines = markdown.lines().collect::<Vec<_>>();
        let mut rendered = Vec::new();
        let mut index = 0;

        while index < lines.len() {
            let line = lines[index];
            let trimmed = line.trim();

            if let Some(language) = trimmed.strip_prefix("```") {
                let mut body = Vec::new();
                index += 1;
                while index < lines.len() {
                    if lines[index].trim().starts_with("```") {
                        index += 1;
                        break;
                    }
                    body.push(lines[index]);
                    index += 1;
                }
                rendered.push(render_code_block(language.trim(), &body, options.color));
                continue;
            }

            if is_horizontal_rule(trimmed) {
                rendered.push(style("─".repeat(48), Style::Dim, options.color));
                index += 1;
                continue;
            }

            if index + 1 < lines.len() && line.contains('|') && is_table_separator(lines[index + 1])
            {
                let (table, end) = parse_table(&lines, index);
                rendered.push(render_table(&table, options.color));
                index = end;
                continue;
            }

            if let Some((level, text)) = parse_header(line) {
                rendered.push(render_header(level, text, options.color));
                index += 1;
                continue;
            }

            if trimmed == ">" {
                rendered.push(style("│", Style::Accent, options.color));
                index += 1;
                continue;
            }
            if let Some(body) = trimmed.strip_prefix("> ") {
                rendered.push(format!(
                    "{} {}",
                    style("│", Style::Accent, options.color),
                    render_inline(body, options.color)
                ));
                index += 1;
                continue;
            }

            if let Some(item) = parse_list_item(line) {
                rendered.push(render_list_item(&item, options.color));
                index += 1;
                continue;
            }

            rendered.push(render_inline(line, options.color));
            index += 1;
        }

        rendered.join("\n")
    }
}

#[derive(Debug, Clone, Copy)]
enum Style {
    Accent,
    Bold,
    Dim,
    InlineCode,
    Strike,
    Link,
}

fn style(text: impl AsRef<str>, style: Style, color: bool) -> String {
    let text = text.as_ref();
    if !color {
        return text.to_owned();
    }
    let code = match style {
        Style::Accent => "\x1b[36m",
        Style::Bold => "\x1b[1m",
        Style::Dim => "\x1b[2m",
        Style::InlineCode => "\x1b[36m",
        Style::Strike => "\x1b[9m",
        Style::Link => "\x1b[34m",
    };
    format!("{code}{text}\x1b[0m")
}

fn render_code_block(language: &str, lines: &[&str], color: bool) -> String {
    let mut out = Vec::with_capacity(lines.len() + 2);
    let suffix = if language.is_empty() {
        String::new()
    } else {
        format!(" {}", style(language, Style::Dim, color))
    };
    out.push(format!("{}{}", style("┌─", Style::Dim, color), suffix));
    for line in lines {
        out.push(format!(
            "{} {}",
            style("│", Style::Dim, color),
            style(line, Style::InlineCode, color)
        ));
    }
    out.push(style("└─", Style::Dim, color));
    out.join("\n")
}

fn parse_header(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = trimmed.get(level..)?;
    let text = rest.strip_prefix(' ')?;
    Some((level, text))
}

fn render_header(level: usize, text: &str, color: bool) -> String {
    let text = render_inline(text, color);
    match level {
        1 => style(format!("▌ {text}"), Style::Bold, color),
        2 => style(format!("▎ {text}"), Style::Bold, color),
        3 => style(format!("  {text}"), Style::Bold, color),
        _ => style(format!("  {text}"), Style::Dim, color),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ListItem<'a> {
    indent: usize,
    marker: &'a str,
    body: &'a str,
    ordered: bool,
}

fn parse_list_item(line: &str) -> Option<ListItem<'_>> {
    let indent = line
        .chars()
        .take_while(|ch| matches!(ch, ' ' | '\t'))
        .map(|ch| if ch == '\t' { 4 } else { 1 })
        .sum();
    let trimmed = line.trim_start_matches([' ', '\t']);
    let mut chars = trimmed.char_indices();
    let (_, first) = chars.next()?;

    if matches!(first, '-' | '*' | '+') {
        let marker_len = first.len_utf8();
        let body = trimmed.get(marker_len..)?.strip_prefix(' ')?;
        return Some(ListItem {
            indent,
            marker: &trimmed[..marker_len],
            body,
            ordered: false,
        });
    }

    let digits = trimmed
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .map(char::len_utf8)
        .sum::<usize>();
    if digits > 0 && trimmed.get(digits..)?.starts_with(". ") {
        return Some(ListItem {
            indent,
            marker: &trimmed[..digits + 1],
            body: &trimmed[digits + 2..],
            ordered: true,
        });
    }

    None
}

fn render_list_item(item: &ListItem<'_>, color: bool) -> String {
    let indent = " ".repeat(item.indent);
    if !item.ordered {
        if let Some(body) = item.body.strip_prefix("[ ] ") {
            return format!(
                "{indent}{} {}",
                style("☐", Style::Dim, color),
                render_inline(body, color)
            );
        }
        if let Some(body) = item
            .body
            .strip_prefix("[x] ")
            .or_else(|| item.body.strip_prefix("[X] "))
        {
            return format!(
                "{indent}{} {}",
                style("☑", Style::Accent, color),
                style(render_inline(body, color), Style::Strike, color)
            );
        }
    }

    let marker = if item.ordered {
        style(item.marker, Style::Accent, color)
    } else {
        style("•", Style::Accent, color)
    };
    format!("{indent}{marker} {}", render_inline(item.body, color))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Table {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

fn parse_table(lines: &[&str], start: usize) -> (Table, usize) {
    let header = split_table_row(lines[start]);
    let mut rows = Vec::new();
    let mut index = start + 2;
    while index < lines.len() {
        let line = lines[index];
        if !line.contains('|') || line.trim().is_empty() {
            break;
        }
        rows.push(split_table_row(line));
        index += 1;
    }
    (Table { header, rows }, index)
}

fn split_table_row(line: &str) -> Vec<String> {
    let mut trimmed = line.trim();
    if let Some(rest) = trimmed.strip_prefix('|') {
        trimmed = rest;
    }
    if let Some(rest) = trimmed.strip_suffix('|') {
        trimmed = rest;
    }
    trimmed
        .split('|')
        .map(|cell| cell.trim().to_owned())
        .collect()
}

fn render_table(table: &Table, color: bool) -> String {
    let column_count = std::iter::once(table.header.len())
        .chain(table.rows.iter().map(Vec::len))
        .max()
        .unwrap_or(0);
    if column_count == 0 {
        return String::new();
    }

    let mut widths = vec![0usize; column_count];
    for row in std::iter::once(&table.header).chain(table.rows.iter()) {
        for (index, width) in widths.iter_mut().enumerate() {
            let cell = row.get(index).map(String::as_str).unwrap_or("");
            *width = (*width).max(display_width(cell));
        }
    }

    let mut out = Vec::new();
    out.push(render_table_row(&table.header, &widths, true, color));
    let separator = widths
        .iter()
        .map(|width| "─".repeat(*width))
        .collect::<Vec<_>>()
        .join("─┼─");
    out.push(style(separator, Style::Dim, color));
    for row in &table.rows {
        out.push(render_table_row(row, &widths, false, color));
    }
    out.join("\n")
}

fn render_table_row(row: &[String], widths: &[usize], bold: bool, color: bool) -> String {
    widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let cell = row.get(index).map(String::as_str).unwrap_or("");
            let padded = format!(
                "{cell}{}",
                " ".repeat(width.saturating_sub(display_width(cell)))
            );
            let rendered = render_inline(&padded, color);
            if bold {
                style(rendered, Style::Bold, color)
            } else {
                rendered
            }
        })
        .collect::<Vec<_>>()
        .join(&style(" │ ", Style::Dim, color))
}

fn is_table_separator(line: &str) -> bool {
    let cells = split_table_row(line);
    cells.len() >= 2
        && cells.iter().all(|cell| {
            let trimmed = cell.trim();
            trimmed.len() >= 3
                && trimmed.chars().all(|ch| matches!(ch, '-' | ':' | ' '))
                && trimmed.chars().any(|ch| ch == '-')
        })
}

fn is_horizontal_rule(trimmed: &str) -> bool {
    let mut chars = trimmed.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    matches!(first, '-' | '*' | '_')
        && trimmed.len() >= 3
        && chars.all(|ch| ch == first || ch.is_whitespace())
}

fn render_inline(text: &str, color: bool) -> String {
    // Inline code is processed first and its content is protected from the
    // remaining passes, so markdown-significant characters inside backticks
    // (`**kwargs`, `a_b_c`, `x~y`) render literally instead of being styled.
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        out.push_str(&render_inline_emphasis(&rest[..start], color));
        let after_open = start + 1;
        match rest[after_open..].find('`') {
            Some(end_rel) => {
                let inner = &rest[after_open..after_open + end_rel];
                if inner.is_empty() {
                    // An empty span (``) isn't code; keep one backtick literal
                    // and continue scanning from just after it.
                    out.push('`');
                    rest = &rest[after_open..];
                } else {
                    out.push_str(&style(inner, Style::InlineCode, color));
                    rest = &rest[after_open + end_rel + 1..];
                }
            }
            None => {
                // Unterminated backtick: treat it as ordinary text and let the
                // emphasis pass handle the remainder.
                out.push_str(&render_inline_emphasis(&rest[start..], color));
                return out;
            }
        }
    }
    out.push_str(&render_inline_emphasis(rest, color));
    out
}

/// Apply the non-code inline passes (bold, strike, links) to a span that is
/// already known to contain no inline code.
fn render_inline_emphasis(text: &str, color: bool) -> String {
    if text.is_empty() {
        return String::new();
    }
    let text = replace_delimited(text, "**", "**", |inner| style(inner, Style::Bold, color));
    let text = replace_delimited(&text, "__", "__", |inner| style(inner, Style::Bold, color));
    let text = replace_delimited(&text, "~~", "~~", |inner| {
        style(inner, Style::Strike, color)
    });
    render_links(&text, color)
}

fn replace_delimited<F>(text: &str, opener: &str, closer: &str, apply: F) -> String
where
    F: Fn(&str) -> String,
{
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(opener) {
        out.push_str(&rest[..start]);
        let after_start = start + opener.len();
        if let Some(end) = rest[after_start..].find(closer) {
            let inner_start = after_start;
            let inner_end = inner_start + end;
            let inner = &rest[inner_start..inner_end];
            if inner.is_empty() {
                out.push_str(opener);
                rest = &rest[after_start..];
            } else {
                out.push_str(&apply(inner));
                rest = &rest[inner_end + closer.len()..];
            }
        } else {
            out.push_str(&rest[start..]);
            return out;
        }
    }
    out.push_str(rest);
    out
}

fn render_links(text: &str, color: bool) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(label_start) = rest.find('[') {
        out.push_str(&rest[..label_start]);
        let after_label = label_start + 1;
        let Some(label_end_rel) = rest[after_label..].find("](") else {
            out.push_str(&rest[label_start..]);
            return out;
        };
        let label_end = after_label + label_end_rel;
        let url_start = label_end + 2;
        let Some(url_end_rel) = rest[url_start..].find(')') else {
            out.push_str(&rest[label_start..]);
            return out;
        };
        let url_end = url_start + url_end_rel;
        let label = &rest[after_label..label_end];
        let url = &rest[url_start..url_end];
        out.push_str(&format!(
            "{} ({})",
            style(label, Style::Link, color),
            style(url, Style::Dim, color)
        ));
        rest = &rest[url_end + 1..];
    }
    out.push_str(rest);
    out
}

use crate::width::display_width;

#[cfg(test)]
mod tests {
    use super::{MarkdownRenderOptions, MarkdownRenderer};
    use crate::width::strip_ansi;

    fn render_plain(markdown: &str) -> String {
        strip_ansi(&MarkdownRenderer::render_with_options(
            markdown,
            MarkdownRenderOptions { color: true },
        ))
    }

    #[test]
    fn renders_core_markdown_blocks() {
        let rendered =
            render_plain("# Title\n\n- [ ] todo\n- [x] done\n> quoted\n\n```rust\nlet x = 1;\n```");

        assert!(rendered.contains("▌ Title"));
        assert!(rendered.contains("☐ todo"));
        assert!(rendered.contains("☑ done"));
        assert!(rendered.contains("│ quoted"));
        assert!(rendered.contains("┌─ rust"));
        assert!(rendered.contains("│ let x = 1;"));
        assert!(rendered.contains("└─"));
    }

    #[test]
    fn renders_tables_without_treating_prose_pipes_as_tables() {
        let rendered = render_plain(
            "run `cat | grep foo`\n\n| col a | col b |\n|-------|-------|\n| 1 | one |\n| 2 | two |\nafter",
        );

        assert!(rendered.contains("cat | grep foo"));
        assert!(rendered.contains("col a"));
        assert!(rendered.contains("col b"));
        assert!(rendered.contains("one"));
        assert!(rendered.contains("two"));
        assert!(rendered.contains("│"));
        assert!(rendered.contains("after"));
    }

    #[test]
    fn no_color_keeps_plain_markdown_structure_but_formats_tables() {
        let rendered = MarkdownRenderer::render_with_options(
            "| a | b |\n|---|---|\n| 1 | 2 |",
            MarkdownRenderOptions { color: false },
        );

        assert_eq!(rendered, "a │ b\n──┼──\n1 │ 2");
    }

    fn render_colored(markdown: &str) -> String {
        MarkdownRenderer::render_with_options(markdown, MarkdownRenderOptions { color: true })
    }

    #[test]
    fn empty_input_renders_empty() {
        assert_eq!(MarkdownRenderer::render(""), "");
    }

    #[test]
    fn inline_bold_and_strike_styles_apply() {
        // ** and __ both bold; ~~ strikes. Plain text survives ANSI stripping.
        let plain = render_plain("a **b** c __d__ e ~~f~~");
        assert_eq!(plain, "a b c d e f");

        // The corresponding ANSI styles are actually emitted with color on.
        let colored = render_colored("**b** __d__ ~~f~~");
        assert!(
            colored.contains("\x1b[1m"),
            "bold escape missing: {colored:?}"
        );
        assert!(
            colored.contains("\x1b[9m"),
            "strike escape missing: {colored:?}"
        );
    }

    #[test]
    fn single_tilde_is_not_strikethrough() {
        // The model uses ~ for "approximately"; only ~~ should strike, so the
        // single tildes must be preserved verbatim.
        let plain = render_plain("about ~5ms, not ~10ms");
        assert_eq!(plain, "about ~5ms, not ~10ms");
    }

    #[test]
    fn inline_code_protects_markdown_characters() {
        // Markdown-significant characters inside a code span must NOT be
        // re-interpreted (Python `**kwargs`, snake_case `a_b_c`, etc.).
        assert_eq!(render_plain("call `a**b**c` now"), "call a**b**c now");
        assert_eq!(render_plain("`def f(**kwargs)`"), "def f(**kwargs)");
        assert_eq!(render_plain("`a__b__c`"), "a__b__c");

        // The code content is still styled with color on.
        assert!(render_colored("`code`").contains("\x1b[36m"));
    }

    #[test]
    fn links_render_label_and_url() {
        assert_eq!(
            render_plain("see [Anthropic](https://example.com) here"),
            "see Anthropic (https://example.com) here"
        );
        // Malformed links are left untouched rather than dropped.
        assert_eq!(render_plain("[label](unclosed"), "[label](unclosed");
        assert_eq!(render_plain("[bare label] text"), "[bare label] text");
    }

    #[test]
    fn headers_render_each_level_and_reject_non_headers() {
        assert_eq!(render_plain("# One"), "▌ One");
        assert_eq!(render_plain("## Two"), "▎ Two");
        assert_eq!(render_plain("### Three"), "  Three");
        assert_eq!(render_plain("###### Six"), "  Six");
        // Seven hashes exceed the ATX range, and a missing space is not a header.
        assert_eq!(render_plain("####### Seven"), "####### Seven");
        assert_eq!(render_plain("#NoSpace"), "#NoSpace");
    }

    #[test]
    fn ordered_and_nested_lists_render() {
        let plain = render_plain("1. first\n2. second");
        assert!(plain.contains("1. first"), "got: {plain:?}");
        assert!(plain.contains("2. second"), "got: {plain:?}");

        // A nested bullet keeps its indentation and uses the • glyph.
        let nested = render_plain("- top\n  - child");
        let lines: Vec<&str> = nested.lines().collect();
        assert_eq!(lines[0], "• top");
        assert_eq!(lines[1], "  • child");
    }

    #[test]
    fn horizontal_rules_and_near_misses() {
        for rule in ["---", "***", "___"] {
            let rendered = render_plain(rule);
            assert_eq!(
                rendered.chars().filter(|c| *c == '─').count(),
                48,
                "{rule:?} should render a 48-wide rule"
            );
        }
        // Two characters is below the threshold, so it stays literal text.
        assert_eq!(render_plain("--"), "--");
    }

    #[test]
    fn code_block_without_language_has_no_label() {
        let plain = render_plain("```\nplain code\n```");
        assert!(plain.contains("┌─"), "got: {plain:?}");
        assert!(plain.contains("│ plain code"), "got: {plain:?}");
        assert!(plain.contains("└─"), "got: {plain:?}");
        // No language token trails the top border.
        let first = plain.lines().next().unwrap_or_default();
        assert_eq!(first.trim(), "┌─");
    }

    use super::StreamingMarkdownRenderer;

    fn streaming_plain() -> StreamingMarkdownRenderer {
        StreamingMarkdownRenderer::new(MarkdownRenderOptions { color: false })
    }

    #[test]
    fn streaming_reset_discards_buffer_and_clears_fence_state() {
        // Setup: feed a partial fenced block so both the buffer AND the
        // in_fence flag are non-default. reset() must wipe both, after
        // which a fresh push parses from scratch with no leftover
        // contamination from the abandoned stream.
        let mut r = streaming_plain();
        let _ = r.push("```rust\nlet x = ");
        assert!(
            !r.is_idle(),
            "precondition: renderer must be holding fenced state"
        );
        r.reset();
        assert!(
            r.is_idle(),
            "reset() must drop buffered bytes AND the fence flag"
        );
        // After reset, a brand-new heading renders cleanly with no
        // leftover code-block markers.
        let out = r.push("# fresh\n");
        assert!(!out.contains("let x ="), "stale buffer leaked: {out:?}");
        assert!(
            out.contains("fresh"),
            "post-reset push must render: {out:?}"
        );
    }

    #[test]
    fn streaming_holds_partial_line_until_newline() {
        let mut r = streaming_plain();
        assert_eq!(r.push("**hi"), "");
        // Still buffered: closing `**` and newline haven't arrived.
        assert_eq!(r.push(" there"), "");
        assert_eq!(r.push("**"), "");
        assert_eq!(r.push("\n"), "hi there\n");
        assert!(r.is_idle());
    }

    #[test]
    fn streaming_buffers_table_block_so_columns_align() {
        // Streaming a table one character at a time must produce the SAME
        // aligned output as block-rendering it whole — i.e. rows are held and
        // rendered together, not one-at-a-time (which can't compute column
        // widths and produces misaligned garbage).
        let opts = MarkdownRenderOptions { color: false };
        let table = "| Area | Rating |\n|---|---|\n| Design | Good |\n| Tests | Strong feature |\n";
        let mut r = StreamingMarkdownRenderer::new(opts);
        let mut streamed = String::new();
        for ch in table.chars() {
            streamed.push_str(&r.push(&ch.to_string()));
        }
        streamed.push_str(&r.flush());
        let block = MarkdownRenderer::render_with_options(table, opts);
        assert_eq!(
            streamed.trim_end(),
            block.trim_end(),
            "streamed table must match the block-rendered (aligned) table"
        );
        assert!(r.is_idle(), "renderer must be idle after the table flushes");
    }

    #[test]
    fn streaming_flushes_table_before_following_prose() {
        // A non-table line ends the table block and lands after it, in order.
        let mut r = streaming_plain();
        let out = r.push("| a | b |\n|---|---|\n| 1 | 2 |\nAfter the table.\n");
        assert!(
            out.contains("After the table."),
            "prose after table must render: {out:?}"
        );
        // The table content precedes the prose line.
        let table_at = out.find('1').expect("table cell present");
        let prose_at = out.find("After").expect("prose present");
        assert!(
            table_at < prose_at,
            "table must render before the following prose"
        );
        assert!(r.is_idle());
    }

    #[test]
    fn streaming_renders_bold_on_completed_line() {
        let mut r = StreamingMarkdownRenderer::new(MarkdownRenderOptions { color: true });
        let out = r.push("Hello **world**\n");
        // Bold escape must appear somewhere in the rendered output.
        assert!(
            out.contains("\x1b[1m"),
            "expected bold escape, got: {out:?}"
        );
        // And the inner text is preserved literally inside the escapes.
        assert!(out.contains("world"));
    }

    #[test]
    fn streaming_renders_bullet_lists() {
        let mut r = streaming_plain();
        let out = r.push("- first\n- second\n");
        assert!(out.contains("• first"), "got: {out:?}");
        assert!(out.contains("• second"), "got: {out:?}");
    }

    #[test]
    fn streaming_renders_headers() {
        let mut r = streaming_plain();
        let out = r.push("# Title\n");
        assert!(out.contains("▌ Title"), "got: {out:?}");
    }

    #[test]
    fn streaming_passes_fenced_code_through_verbatim() {
        let mut r = streaming_plain();
        // Open fence: line is emitted as-is, not as an empty box.
        let out_open = r.push("```rust\n");
        assert_eq!(out_open, "```rust\n");
        // Inside fence: the `- not a list` line must NOT become `• not a list`.
        let out_inside = r.push("- not a list\n");
        assert_eq!(out_inside, "- not a list\n");
        // Partial line inside fence is emitted verbatim immediately so the
        // user sees code as it streams.
        let out_partial = r.push("let x = 1");
        assert_eq!(out_partial, "let x = 1");
        let out_eol = r.push(";\n");
        assert_eq!(out_eol, ";\n");
        let out_close = r.push("```\n");
        assert_eq!(out_close, "```\n");
        // Outside the fence again, list syntax renders normally.
        let out_after = r.push("- back to lists\n");
        assert!(out_after.contains("• back to lists"));
    }

    #[test]
    fn streaming_flush_drains_trailing_partial_line() {
        let mut r = streaming_plain();
        r.push("**partial");
        // Buffered, nothing emitted yet.
        let flushed = r.flush();
        // The partial line is rendered as best-effort: unbalanced `**` is
        // preserved literally (the existing emphasis pass leaves unmatched
        // delimiters as-is).
        assert!(flushed.contains("partial"), "got: {flushed:?}");
        // Flushing again is a no-op.
        assert_eq!(r.flush(), "");
        assert!(r.is_idle());
    }

    #[test]
    fn streaming_handles_chunks_split_mid_codepoint_is_a_caller_concern() {
        // We don't claim to split UTF-8 bytes; callers must pass us valid
        // &str slices. But we DO handle multi-codepoint Chinese/Japanese
        // text within a single chunk without panicking.
        let mut r = streaming_plain();
        let out = r.push("- 你好世界\n- 再见\n");
        assert!(out.contains("• 你好世界"), "got: {out:?}");
        assert!(out.contains("• 再见"), "got: {out:?}");
    }

    #[test]
    fn streaming_emits_each_line_independently() {
        // Three small chunks that together form two lines should produce
        // exactly two newline-terminated rendered lines, in order.
        let mut r = streaming_plain();
        let mut out = String::new();
        out.push_str(&r.push("first "));
        out.push_str(&r.push("line\nsecond"));
        out.push_str(&r.push(" line\n"));
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines, vec!["first line", "second line"]);
    }
}
