//! Unified-diff preview rendering for `Write` / `Edit` / `AppendFile`
//! tool calls, surfaced inside the permission-ask flow so the user
//! sees exactly what the agent is about to write before they say yes.
//!
//! This module deliberately doesn't pull in a third-party diff crate.
//! Tool calls already carry enough structured info that we can compute
//! a focused diff without a generic Myers-style LCS:
//!
//! * `Write` — compare the existing file (if any) against the
//!   proposed `content`, then render a `+/-` line list anchored by
//!   common prefix/suffix.
//! * `Edit` — apply the requested `old_string` → `new_string`
//!   substitution in memory, then run the same line-diff machine
//!   against the file's current contents.
//! * `AppendFile` — there is no "old" region, just emit a `+`-prefixed
//!   preview of the appended chunk.
//!
//! The output is plain text (the CLI ANSI-colours it on the way out)
//! and is bounded by `DiffPreviewOptions::max_lines` so a 10 000-line
//! replacement doesn't spam the terminal.

use std::path::{Path, PathBuf};

/// One rendered preview: a one-line summary plus a multi-line body
/// suitable for printing inside the permission ask. Both pieces are
/// already truncated and line-wrapped where appropriate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffPreview {
    pub summary: String,
    pub body: String,
}

/// Tunables. `max_lines` bounds the total `+/-`/context lines we emit;
/// `context_lines` is the number of common lines we show on either
/// side of a changed hunk.
#[derive(Debug, Clone, Copy)]
pub struct DiffPreviewOptions {
    pub max_lines: usize,
    pub context_lines: usize,
}

impl Default for DiffPreviewOptions {
    fn default() -> Self {
        Self {
            max_lines: 60,
            context_lines: 3,
        }
    }
}

/// Top-level entry point: dispatch on tool name and return a preview
/// when one applies. Unknown tools (Bash, Grep, …) return `None` so
/// the caller falls back to the existing one-line summary.
pub fn render_tool_call_diff(
    tool_name: &str,
    input: &serde_json::Value,
    cwd: &Path,
    options: DiffPreviewOptions,
) -> Option<DiffPreview> {
    match tool_name {
        "Write" => write_preview(input, cwd, options),
        "Edit" => edit_preview(input, cwd, options),
        "AppendFile" => append_preview(input, cwd, options),
        _ => None,
    }
}

fn resolve_path(cwd: &Path, raw: &str) -> PathBuf {
    let p = Path::new(raw);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        cwd.join(p)
    }
}

fn read_existing(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

fn display_relative(path: &Path, cwd: &Path) -> String {
    path.strip_prefix(cwd)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

/// Diff for `Write { file_path, content }`. Two cases:
///   * file doesn't exist yet → "creating" preview (every line is `+`)
///   * file exists           → line-diff between current and proposed
fn write_preview(
    input: &serde_json::Value,
    cwd: &Path,
    options: DiffPreviewOptions,
) -> Option<DiffPreview> {
    let file_path = input.get("file_path")?.as_str()?;
    let new_content = input.get("content")?.as_str()?;
    let abs = resolve_path(cwd, file_path);
    let display = display_relative(&abs, cwd);

    let existing = read_existing(&abs);
    match existing {
        None => Some(creation_preview(&display, new_content, options)),
        Some(old) => Some(line_diff_preview(
            &display,
            &old,
            new_content,
            options,
            DiffKind::Replace,
        )),
    }
}

/// Diff for `Edit { file_path, old_string, new_string, replace_all? }`.
/// Applies the substitution in memory, then runs the same line-diff
/// against the file's current contents.
fn edit_preview(
    input: &serde_json::Value,
    cwd: &Path,
    options: DiffPreviewOptions,
) -> Option<DiffPreview> {
    let file_path = input.get("file_path")?.as_str()?;
    let old_string = input.get("old_string")?.as_str()?;
    let new_string = input.get("new_string")?.as_str()?;
    let replace_all = input
        .get("replace_all")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let abs = resolve_path(cwd, file_path);
    let display = display_relative(&abs, cwd);
    let current = read_existing(&abs)?;

    let proposed = if replace_all {
        current.replace(old_string, new_string)
    } else {
        // Single replacement of the first occurrence. Matches Edit's
        // behaviour (the tool errors out when old_string isn't unique
        // and `replace_all = false`, but for the preview we treat the
        // first match as "what would happen if it's unique").
        match current.find(old_string) {
            Some(idx) => {
                let mut out = String::with_capacity(current.len() + new_string.len());
                out.push_str(&current[..idx]);
                out.push_str(new_string);
                out.push_str(&current[idx + old_string.len()..]);
                out
            }
            None => {
                // Show a clear "no match" preview so the user knows
                // approving will just produce an error.
                return Some(DiffPreview {
                    summary: format!("edit no-op: `old_string` not found in {display}"),
                    body: format!(
                        "  (the substring the model wants to replace is not currently in {display};\n   approving would yield an error from the Edit tool — consider denying.)"
                    ),
                });
            }
        }
    };

    Some(line_diff_preview(
        &display,
        &current,
        &proposed,
        options,
        DiffKind::Replace,
    ))
}

/// Diff for `AppendFile { file_path, content, ensure_trailing_newline? }`.
/// The current contents (if any) are unchanged; only the appended
/// chunk is shown with `+` prefixes.
fn append_preview(
    input: &serde_json::Value,
    cwd: &Path,
    options: DiffPreviewOptions,
) -> Option<DiffPreview> {
    let file_path = input.get("file_path")?.as_str()?;
    let chunk = input.get("content")?.as_str()?;
    let abs = resolve_path(cwd, file_path);
    let display = display_relative(&abs, cwd);
    let exists = abs.exists();

    let lines: Vec<&str> = chunk.lines().collect();
    let total_lines = lines.len();
    let new_bytes = chunk.len();

    let summary = if exists {
        format!(
            "append to {display}  (+{total_lines} line(s), +{}B)",
            new_bytes
        )
    } else {
        format!(
            "create {display}  (+{total_lines} line(s), +{}B)",
            new_bytes
        )
    };

    // Show at most `max_lines` of the appended chunk so we don't spam
    // the terminal for a multi-KB append. A trailing `… (N more lines
    // suppressed)` summary keeps the user oriented.
    let take = options.max_lines.min(total_lines);
    let mut body_lines: Vec<String> = Vec::with_capacity(take + 2);
    body_lines.push(format!("--- {display}  (current)"));
    body_lines.push(format!("+++ {display}  (after append)"));
    for line in lines.iter().take(take) {
        body_lines.push(format!("+{line}"));
    }
    if total_lines > take {
        body_lines.push(format!(
            "… ({} more line(s) suppressed)",
            total_lines - take
        ));
    }
    Some(DiffPreview {
        summary,
        body: body_lines.join("\n"),
    })
}

#[derive(Debug, Clone, Copy)]
enum DiffKind {
    Replace,
}

/// Render the "create a new file" case as a pure `+` block — there is
/// no left side, so a real diff isn't useful. Truncate at `max_lines`.
fn creation_preview(display: &str, new_content: &str, options: DiffPreviewOptions) -> DiffPreview {
    let lines: Vec<&str> = new_content.lines().collect();
    let take = options.max_lines.min(lines.len());
    let mut body = Vec::with_capacity(take + 2);
    body.push(String::from("--- /dev/null"));
    body.push(format!("+++ {display}  (new file)"));
    for line in lines.iter().take(take) {
        body.push(format!("+{line}"));
    }
    if lines.len() > take {
        body.push(format!(
            "… ({} more line(s) suppressed)",
            lines.len() - take
        ));
    }
    DiffPreview {
        summary: format!(
            "create {display}  ({} line(s), {} bytes)",
            lines.len(),
            new_content.len()
        ),
        body: body.join("\n"),
    }
}

/// Line-by-line diff between `old` and `new`, anchored by the common
/// prefix and common suffix. This is intentionally NOT a full LCS
/// diff: most file edits are localized (one function rewrite, one
/// added block), and the prefix/suffix trick produces a tight,
/// readable preview without dragging in a heavy crate.
fn line_diff_preview(
    display: &str,
    old: &str,
    new: &str,
    options: DiffPreviewOptions,
    _kind: DiffKind,
) -> DiffPreview {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    if old_lines == new_lines {
        return DiffPreview {
            summary: format!("{display}: no-op (content unchanged)"),
            body: format!(
                "  (approving will write {} bytes, but the resulting file is byte-for-byte identical to the current one — approving is safe but does nothing.)",
                new.len()
            ),
        };
    }

    let common_prefix = old_lines
        .iter()
        .zip(new_lines.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let common_suffix = old_lines
        .iter()
        .rev()
        .zip(new_lines.iter().rev())
        .take_while(|(a, b)| a == b)
        // Suffix mustn't overlap the prefix region.
        .take(
            old_lines
                .len()
                .min(new_lines.len())
                .saturating_sub(common_prefix),
        )
        .count();

    let old_changed = &old_lines[common_prefix..old_lines.len() - common_suffix];
    let new_changed = &new_lines[common_prefix..new_lines.len() - common_suffix];

    let mut body = Vec::with_capacity(options.max_lines + 4);
    body.push(format!("--- {display}  ({} line(s))", old_lines.len()));
    body.push(format!("+++ {display}  ({} line(s))", new_lines.len()));

    let context_before = common_prefix.saturating_sub(options.context_lines);
    let context_after_end =
        (old_lines.len() - common_suffix + options.context_lines).min(old_lines.len());
    let hunk_old_start = context_before;
    let _hunk_old_count = context_after_end - context_before;
    body.push(format!(
        "@@ -{},{} +{},{} @@",
        hunk_old_start + 1,
        (old_lines.len() - common_suffix - context_before)
            + (context_after_end - (old_lines.len() - common_suffix)),
        hunk_old_start + 1,
        (new_lines.len() - common_suffix - context_before)
            + (context_after_end - (old_lines.len() - common_suffix))
    ));

    // Context BEFORE the change.
    let pre_start = common_prefix.saturating_sub(options.context_lines);
    for line in &old_lines[pre_start..common_prefix] {
        body.push(format!(" {line}"));
    }

    // Deletions.
    let mut emitted = body.len();
    const SUPPRESSED_NOTICE: &str =
        "… (more diff lines suppressed — set a larger context or open the file directly)";
    for line in old_changed {
        if emitted >= options.max_lines + 4 {
            body.push(String::from(SUPPRESSED_NOTICE));
            return DiffPreview {
                summary: summarize_change(display, old_lines.len(), new_lines.len()),
                body: body.join("\n"),
            };
        }
        body.push(format!("-{line}"));
        emitted += 1;
    }

    // Insertions.
    for line in new_changed {
        if emitted >= options.max_lines + 4 {
            body.push(String::from(SUPPRESSED_NOTICE));
            return DiffPreview {
                summary: summarize_change(display, old_lines.len(), new_lines.len()),
                body: body.join("\n"),
            };
        }
        body.push(format!("+{line}"));
        emitted += 1;
    }

    // Context AFTER the change.
    let post_start = old_lines.len() - common_suffix;
    let post_end = (post_start + options.context_lines).min(old_lines.len());
    for line in &old_lines[post_start..post_end] {
        body.push(format!(" {line}"));
    }

    DiffPreview {
        summary: summarize_change(display, old_lines.len(), new_lines.len()),
        body: body.join("\n"),
    }
}

fn summarize_change(display: &str, old_lines: usize, new_lines: usize) -> String {
    let delta = new_lines as isize - old_lines as isize;
    if delta == 0 {
        format!("modify {display}  ({old_lines} line(s), 0 net change)")
    } else if delta > 0 {
        format!("modify {display}  ({old_lines} → {new_lines} line(s), +{delta})")
    } else {
        format!("modify {display}  ({old_lines} → {new_lines} line(s), {delta})")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use serde_json::json;
    use std::fs;
    use tempfile::tempdir;

    fn opts() -> DiffPreviewOptions {
        DiffPreviewOptions::default()
    }

    #[test]
    fn unknown_tool_returns_none() {
        let dir = tempdir().unwrap();
        let preview = render_tool_call_diff("Bash", &json!({"command": "ls"}), dir.path(), opts());
        assert!(preview.is_none());
    }

    #[test]
    fn write_new_file_shows_creation_preview() {
        let dir = tempdir().unwrap();
        let preview = render_tool_call_diff(
            "Write",
            &json!({
                "file_path": "hello.txt",
                "content": "line one\nline two\n",
            }),
            dir.path(),
            opts(),
        )
        .expect("preview");
        assert!(preview.summary.contains("create hello.txt"));
        assert!(preview.body.contains("--- /dev/null"));
        assert!(preview.body.contains("+line one"));
        assert!(preview.body.contains("+line two"));
    }

    #[test]
    fn write_replace_existing_shows_minimal_diff() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("notes.md");
        fs::write(&path, "alpha\nbeta\ngamma\ndelta\n").unwrap();
        let preview = render_tool_call_diff(
            "Write",
            &json!({
                "file_path": "notes.md",
                "content": "alpha\nBETA\ngamma\ndelta\n",
            }),
            dir.path(),
            opts(),
        )
        .expect("preview");

        assert!(preview.summary.contains("modify notes.md"));
        // Common prefix `alpha` + suffix `gamma\ndelta` → only the
        // middle line shows up as `-`/`+`.
        assert!(preview.body.contains("-beta"), "got: {}", preview.body);
        assert!(preview.body.contains("+BETA"), "got: {}", preview.body);
        // Context shows `alpha` and `gamma` (one of each, no need for
        // multiple at our default context).
        assert!(preview.body.contains(" alpha"));
    }

    #[test]
    fn write_no_change_shows_noop_preview() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("same.txt");
        fs::write(&path, "x\ny\nz\n").unwrap();
        let preview = render_tool_call_diff(
            "Write",
            &json!({
                "file_path": "same.txt",
                "content": "x\ny\nz\n",
            }),
            dir.path(),
            opts(),
        )
        .expect("preview");
        assert!(preview.summary.contains("no-op"));
        assert!(preview.body.contains("identical"));
    }

    #[test]
    fn edit_with_match_renders_diff() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("code.rs");
        fs::write(&path, "fn main() {\n    println!(\"hi\");\n}\n").unwrap();
        let preview = render_tool_call_diff(
            "Edit",
            &json!({
                "file_path": "code.rs",
                "old_string": "    println!(\"hi\");",
                "new_string": "    println!(\"hello, world\");",
            }),
            dir.path(),
            opts(),
        )
        .expect("preview");
        assert!(preview.body.contains("-    println!(\"hi\");"));
        assert!(preview.body.contains("+    println!(\"hello, world\");"));
    }

    #[test]
    fn edit_with_no_match_warns_user() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("code.rs");
        fs::write(&path, "fn main() {}\n").unwrap();
        let preview = render_tool_call_diff(
            "Edit",
            &json!({
                "file_path": "code.rs",
                "old_string": "nonexistent",
                "new_string": "irrelevant",
            }),
            dir.path(),
            opts(),
        )
        .expect("preview");
        assert!(preview.summary.contains("no-op"));
        assert!(preview.body.contains("not currently in"));
    }

    #[test]
    fn edit_replace_all_substitutes_every_occurrence() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("conf.toml");
        fs::write(&path, "k = \"v\"\nk = \"v\"\nk = \"v\"\n").unwrap();
        let preview = render_tool_call_diff(
            "Edit",
            &json!({
                "file_path": "conf.toml",
                "old_string": "\"v\"",
                "new_string": "\"VV\"",
                "replace_all": true,
            }),
            dir.path(),
            opts(),
        )
        .expect("preview");
        // All three lines change; total deletions == 3 lines.
        let minuses = preview.body.matches("\n-").count();
        assert!(
            minuses >= 3,
            "preview should show all 3 deletions: {}",
            preview.body
        );
    }

    #[test]
    fn append_to_existing_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("log.txt");
        fs::write(&path, "[old log]\n").unwrap();
        let preview = render_tool_call_diff(
            "AppendFile",
            &json!({
                "file_path": "log.txt",
                "content": "[new entry]\n[next entry]",
            }),
            dir.path(),
            opts(),
        )
        .expect("preview");
        assert!(preview.summary.contains("append to log.txt"));
        assert!(preview.summary.contains("+2 line(s)"));
        assert!(preview.body.contains("+[new entry]"));
        assert!(preview.body.contains("+[next entry]"));
    }

    #[test]
    fn append_creating_file() {
        let dir = tempdir().unwrap();
        let preview = render_tool_call_diff(
            "AppendFile",
            &json!({
                "file_path": "new.log",
                "content": "first line",
            }),
            dir.path(),
            opts(),
        )
        .expect("preview");
        assert!(preview.summary.contains("create new.log"));
    }

    #[test]
    fn long_diff_truncates_at_max_lines() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("big.txt");
        // 200-line file, fully replaced with a different 200-line file.
        let old: String = (0..200).map(|i| format!("old line {i}\n")).collect();
        let new: String = (0..200).map(|i| format!("new line {i}\n")).collect();
        fs::write(&path, &old).unwrap();
        let preview = render_tool_call_diff(
            "Write",
            &json!({
                "file_path": "big.txt",
                "content": new,
            }),
            dir.path(),
            DiffPreviewOptions {
                max_lines: 10,
                context_lines: 1,
            },
        )
        .expect("preview");
        assert!(
            preview.body.contains("suppressed"),
            "should truncate: {}",
            preview.body
        );
    }

    #[test]
    fn missing_file_path_returns_none() {
        let dir = tempdir().unwrap();
        let preview = render_tool_call_diff("Write", &json!({"content": "x"}), dir.path(), opts());
        assert!(preview.is_none());
    }

    #[test]
    fn edit_outside_cwd_uses_absolute_path() {
        let dir = tempdir().unwrap();
        let outside = dir.path().join("outside.txt");
        fs::write(&outside, "old\n").unwrap();
        let preview = render_tool_call_diff(
            "Edit",
            &json!({
                "file_path": outside.to_str().unwrap(),
                "old_string": "old",
                "new_string": "new",
            }),
            std::env::temp_dir().as_path(),
            opts(),
        )
        .expect("preview");
        assert!(preview.body.contains("-old"));
        assert!(preview.body.contains("+new"));
    }
}
