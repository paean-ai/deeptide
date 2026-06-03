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
        // A trailing partial row (no closing newline) that belongs to a pending
        // table block gets folded in, so the whole table still renders aligned
        // rather than printing the last row with raw `|` pipes.
        if !self.table_buf.is_empty() && !self.in_fence && self.buf.trim_start().starts_with('|') {
            let line = std::mem::take(&mut self.buf);
            self.table_buf.push(line);
        }
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
    Italic,
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
    // Colour accents come from the active theme (default `dark` reproduces the
    // pre-theming codes, so markdown snapshots are unchanged). Text ATTRIBUTES
    // (bold/italic/dim/strike) are structural, not palette colours, so they
    // stay fixed regardless of theme.
    let md = &crate::theme::active().markdown;
    let code: &str = match style {
        Style::Accent => &md.accent,
        Style::Bold => "\x1b[1m",
        Style::Italic => "\x1b[3m",
        Style::Dim => "\x1b[2m",
        Style::InlineCode => &md.inline_code,
        Style::Strike => "\x1b[9m",
        Style::Link => &md.link,
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
    // Per-token syntax highlighting when colour is on and we recognise the
    // language; otherwise every line keeps the uniform inline-code colour.
    let highlight = color && crate::syntax::is_supported(language);
    for line in lines {
        let body = if highlight {
            crate::syntax::highlight_line(language, line)
                .unwrap_or_else(|| style(line, Style::InlineCode, color))
        } else {
            style(line, Style::InlineCode, color)
        };
        out.push(format!("{} {}", style("│", Style::Dim, color), body));
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
    // Double-delimiter passes run first. Each closure recurses through the
    // remaining emphasis on its inner text so nested spans like `**_x_**`
    // (bold wrapping italic) pick up BOTH styles instead of leaving the inner
    // `_x_` literal.
    let text = replace_delimited(text, "**", "**", |inner| {
        style(render_inline_emphasis(inner, color), Style::Bold, color)
    });
    let text = replace_delimited(&text, "__", "__", |inner| {
        style(render_inline_emphasis(inner, color), Style::Bold, color)
    });
    let text = replace_delimited(&text, "~~", "~~", |inner| {
        style(render_inline_emphasis(inner, color), Style::Strike, color)
    });
    // Italic runs AFTER bold so the double-delimiter passes have already
    // consumed `**`/`__`; only genuine single-delimiter spans remain in the
    // outer text. (Inner spans were already handled by the recursion above,
    // and carry no raw `*`/`_` for this pass to re-match.)
    let text = replace_italic(&text, '*', color);
    let text = replace_italic(&text, '_', color);
    render_links(&text, color)
}

/// Render single-delimiter emphasis (`*italic*` / `_italic_`) using CommonMark
/// flanking rules so we don't mangle prose. An opener must be immediately
/// followed by a non-space, a closer immediately preceded by a non-space, and —
/// for `_` only — neither side may sit between two alphanumerics, so
/// identifiers like `foo_bar_baz` and `a * b` (multiplication) stay literal.
fn replace_italic(text: &str, delim: char, color: bool) -> String {
    let bytes: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != delim {
            out.push(bytes[i]);
            i += 1;
            continue;
        }
        // Candidate opener at i: needs a non-space immediately after and, for
        // `_`, a non-alphanumeric immediately before (word boundary).
        let before = if i == 0 { None } else { Some(bytes[i - 1]) };
        let after = bytes.get(i + 1).copied();
        let opener_ok = matches!(after, Some(c) if !c.is_whitespace() && c != delim)
            && (delim != '_' || before.is_none_or(|c| !c.is_alphanumeric()));
        if !opener_ok {
            out.push(delim);
            i += 1;
            continue;
        }
        // Scan for a valid closer: a delim preceded by non-space and (for `_`)
        // not followed by an alphanumeric.
        let mut j = i + 1;
        let mut found = None;
        while j < bytes.len() {
            if bytes[j] == delim {
                let prev = bytes[j - 1];
                let next = bytes.get(j + 1).copied();
                let closer_ok = !prev.is_whitespace()
                    && (delim != '_' || next.is_none_or(|c| !c.is_alphanumeric()));
                if closer_ok {
                    found = Some(j);
                    break;
                }
            }
            j += 1;
        }
        match found {
            Some(close) => {
                let inner: String = bytes[i + 1..close].iter().collect();
                out.push_str(&style(inner, Style::Italic, color));
                i = close + 1;
            }
            None => {
                out.push(delim);
                i += 1;
            }
        }
    }
    out
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

/// Whether `url` is safe to embed in an OSC-8 hyperlink escape. Rejects any
/// control character (which would prematurely terminate or corrupt the escape
/// framing) and restricts to schemes a terminal can sensibly open, so a crafted
/// link can't smuggle escape sequences into the stream.
fn is_safe_hyperlink(url: &str) -> bool {
    if url.is_empty() || url.chars().any(|c| c.is_control()) {
        return false;
    }
    let lower = url.to_ascii_lowercase();
    ["http://", "https://", "mailto:", "file://"]
        .iter()
        .any(|scheme| lower.starts_with(scheme))
}

fn render_links(text: &str, color: bool) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(label_start) = rest.find('[') {
        // Prose before the markdown link gets bare-URL autolinking.
        out.push_str(&render_autolinks(&rest[..label_start], color));
        let after_label = label_start + 1;
        let Some(label_end_rel) = rest[after_label..].find("](") else {
            out.push_str(&render_autolinks(&rest[label_start..], color));
            return out;
        };
        let label_end = after_label + label_end_rel;
        let url_start = label_end + 2;
        let Some(url_end_rel) = rest[url_start..].find(')') else {
            out.push_str(&render_autolinks(&rest[label_start..], color));
            return out;
        };
        let url_end = url_start + url_end_rel;
        let label = &rest[after_label..label_end];
        let url = &rest[url_start..url_end];
        let styled_label = style(label, Style::Link, color);
        // Wrap the label in an OSC-8 hyperlink so terminals that support it make
        // the label itself clickable (zero-cli parity + then some). The dimmed
        // `(url)` stays as a visible fallback for terminals that ignore OSC-8;
        // the escape framing is zero-width and stripped from width measurement.
        let linked = if color && is_safe_hyperlink(url) {
            format!("\x1b]8;;{url}\x07{styled_label}\x1b]8;;\x07")
        } else {
            styled_label
        };
        out.push_str(&format!("{} ({})", linked, style(url, Style::Dim, color)));
        rest = &rest[url_end + 1..];
    }
    out.push_str(&render_autolinks(rest, color));
    out
}

/// Auto-link bare `http(s)://` URLs sitting in prose (not already part of a
/// `[label](url)` span, which `render_links` handles). Each detected URL is
/// wrapped in an OSC-8 hyperlink so it's clickable while the visible text stays
/// the URL itself. No-op when colour is off (a plain URL is already its own
/// "label", and we don't emit escapes into piped output) — which also keeps the
/// no-colour snapshot tests byte-identical.
fn render_autolinks(text: &str, color: bool) -> String {
    // Skip when colour is off, the segment is empty, or it ALREADY carries an
    // OSC-8 escape. The last guard matters because `render_links` recurses
    // (bold/italic closures), so a segment can arrive already containing a
    // hyperlink whose visible label is itself a URL — re-linking it would nest
    // OSC-8 and corrupt the sequence. Conservative but always correct.
    if !color || text.is_empty() || text.contains("\x1b]8") {
        return text.to_owned();
    }
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = find_scheme(rest) {
        out.push_str(&rest[..start]);
        let run = &rest[start..];
        let len = url_run_len(run);
        let url = &run[..len];
        if is_safe_hyperlink(url) {
            out.push_str(&format!("\x1b]8;;{url}\x07{url}\x1b]8;;\x07"));
        } else {
            out.push_str(url);
        }
        rest = &run[len..];
    }
    out.push_str(rest);
    out
}

/// Byte offset of the earliest `http://` / `https://` in `text`, if any.
fn find_scheme(text: &str) -> Option<usize> {
    match (text.find("https://"), text.find("http://")) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, b) => b,
    }
}

/// Length of the URL run starting at the scheme. Stops at whitespace, an ANSI
/// escape, or URL-hostile delimiters (`<>"'` `` ` `` `)`), then trims trailing
/// sentence punctuation so `…example.com.` / `(…example.com)` link cleanly.
fn url_run_len(run: &str) -> usize {
    let mut end = run.len();
    for (i, ch) in run.char_indices() {
        if ch.is_whitespace() || ch == '\x1b' || matches!(ch, '<' | '>' | '"' | '\'' | '`' | ')') {
            end = i;
            break;
        }
    }
    // Don't swallow trailing sentence punctuation into the link target.
    let trimmed = run[..end].trim_end_matches(['.', ',', ';', ':', '!', '?']);
    trimmed.len()
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
    fn inline_italic_styles_apply_and_strip_clean() {
        // Both `*` and `_` mark italics; the text survives ANSI stripping.
        let plain = render_plain("a *b* c _d_ e");
        assert_eq!(plain, "a b c d e");

        // The italic SGR (ESC[3m) is actually emitted with color on, and is
        // distinct from bold (ESC[1m) so `*x*` and `**x**` don't collide.
        let colored = render_colored("*one* and **two**");
        assert!(
            colored.contains("\x1b[3m"),
            "italic escape missing: {colored:?}"
        );
        assert!(
            colored.contains("\x1b[1m"),
            "bold escape missing: {colored:?}"
        );
    }

    #[test]
    fn intraword_underscores_and_bare_stars_stay_literal() {
        // CommonMark flanking: `_` inside identifiers must NOT italicize, and a
        // multiplication `*` with spaces around it isn't an emphasis opener.
        assert_eq!(render_plain("call foo_bar_baz now"), "call foo_bar_baz now");
        assert_eq!(render_plain("compute a * b * c"), "compute a * b * c");
        // An unmatched single delimiter is left verbatim.
        assert_eq!(render_plain("a *lonely star"), "a *lonely star");
    }

    #[test]
    fn bold_wrapping_italic_keeps_both_styles() {
        // `**_x_**` → bold pass consumes the `**`, italic pass the inner `_`.
        let colored = render_colored("**_emph_**");
        assert!(colored.contains("\x1b[1m"), "bold missing: {colored:?}");
        assert!(colored.contains("\x1b[3m"), "italic missing: {colored:?}");
        assert_eq!(render_plain("**_emph_**"), "emph");
    }

    #[test]
    fn fenced_code_gets_syntax_highlighting_for_known_languages() {
        // A recognised language colours keywords inside the fence (per-token),
        // while the fence chrome stays dim. Stripping ANSI must still yield the
        // exact source line — highlighting is lossless.
        let colored = render_colored("```rust\nlet x = 1;\n```");
        assert!(
            colored.contains("\x1b[35mlet\x1b[0m"),
            "rust `let` keyword should be magenta inside the fence: {colored:?}"
        );
        let plain = render_plain("```rust\nlet x = 1;\n```");
        assert!(
            plain.contains("│ let x = 1;"),
            "source preserved: {plain:?}"
        );

        // An UNKNOWN language falls back to the uniform inline-code colour with
        // no per-token keyword escape.
        let unknown = render_colored("```\nlet x = 1;\n```");
        assert!(
            !unknown.contains("\x1b[35mlet\x1b[0m"),
            "no language → no per-token highlight: {unknown:?}"
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
    fn http_links_emit_osc8_hyperlink_with_visible_fallback() {
        // With colour on, an http(s) link wraps the label in an OSC-8 escape so
        // supporting terminals make it clickable, while the dimmed `(url)` stays
        // visible for terminals that ignore the escape.
        let colored = render_colored("[Anthropic](https://example.com)");
        assert!(
            colored.contains("\x1b]8;;https://example.com\x07"),
            "OSC-8 open missing: {colored:?}"
        );
        assert!(
            colored.contains("\x1b]8;;\x07"),
            "OSC-8 close missing: {colored:?}"
        );
        // Stripping ALL escapes (CSI + OSC) leaves the human-readable fallback.
        assert_eq!(strip_ansi(&colored), "Anthropic (https://example.com)");
    }

    #[test]
    fn bare_urls_in_prose_are_autolinked_with_osc8() {
        // A bare URL in running text becomes a clickable OSC-8 link whose
        // visible text is the URL itself; stripping escapes restores the prose.
        let colored = render_colored("see https://example.com here");
        assert!(
            colored.contains("\x1b]8;;https://example.com\x07https://example.com\x1b]8;;\x07"),
            "bare url should be OSC-8 wrapped: {colored:?}"
        );
        assert_eq!(strip_ansi(&colored), "see https://example.com here");

        // Trailing sentence punctuation is NOT swallowed into the link target.
        let dotted = render_colored("visit https://example.com.");
        assert!(
            dotted.contains("\x1b]8;;https://example.com\x07"),
            "url without trailing dot: {dotted:?}"
        );
        assert!(
            !dotted.contains("https://example.com.\x07"),
            "the period must stay outside the link: {dotted:?}"
        );
        assert_eq!(strip_ansi(&dotted), "visit https://example.com.");
    }

    #[test]
    fn autolinking_is_off_without_color_and_leaves_prose_unchanged() {
        // No-colour output must be byte-identical to the input prose (no escapes
        // leak into piped/`--no-color` consumers).
        let plain = MarkdownRenderer::render_with_options(
            "see https://example.com here",
            MarkdownRenderOptions { color: false },
        );
        assert_eq!(plain, "see https://example.com here");
    }

    #[test]
    fn non_http_schemes_are_not_hyperlinked() {
        // A `javascript:`/relative/control-char URL must NOT become an OSC-8
        // escape — only known-safe schemes are clickable, the rest render plain.
        let colored = render_colored("[x](javascript:alert(1))");
        assert!(
            !colored.contains("\x1b]8;;"),
            "unsafe scheme must not emit OSC-8: {colored:?}"
        );
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

    #[test]
    fn flush_folds_trailing_partial_table_row() {
        // The final row arrives with no closing newline (the stream/turn ends
        // mid-row), so it sits in the partial-line buffer. flush() must fold it
        // into the pending table block and render the whole table aligned — not
        // print the last row as raw `|` pipes. We assert the partial-tail render
        // matches the newline-terminated render (the aligned baseline).
        let partial = "| A | BB |\n|---|----|\n| 1 | 2 |"; // no trailing newline
        let full = "| A | BB |\n|---|----|\n| 1 | 2 |\n";

        let mut r1 = streaming_plain();
        let mut from_partial = String::new();
        from_partial.push_str(&r1.push(partial));
        from_partial.push_str(&r1.flush());

        let mut r2 = streaming_plain();
        let mut from_full = String::new();
        from_full.push_str(&r2.push(full));
        from_full.push_str(&r2.flush());

        assert_eq!(from_partial, from_full, "partial tail must render aligned");
        // The raw separator row is consumed by table rendering, so it must not
        // survive verbatim — proving the tail was folded into the table block.
        assert!(
            !from_partial.contains("|---|----|"),
            "table must render, not pass through raw: {from_partial:?}"
        );
    }
}
