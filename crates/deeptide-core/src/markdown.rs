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
    let text = replace_delimited(text, "`", "`", |inner| {
        style(inner, Style::InlineCode, color)
    });
    let text = replace_delimited(&text, "**", "**", |inner| style(inner, Style::Bold, color));
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

fn display_width(text: &str) -> usize {
    strip_ansi(text)
        .chars()
        .map(|ch| if is_wide(ch) { 2 } else { 1 })
        .sum()
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

#[cfg(test)]
fn strip_ansi(text: &str) -> String {
    strip_ansi_impl(text)
}

#[cfg(not(test))]
fn strip_ansi(text: &str) -> String {
    strip_ansi_impl(text)
}

fn strip_ansi_impl(text: &str) -> String {
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
    use super::{MarkdownRenderOptions, MarkdownRenderer, strip_ansi};

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
}
