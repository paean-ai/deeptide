//! `@file` references: expand `@path/to/file.rs` tokens in user input
//! into inline file content the model sees as part of the prompt.
//!
//! This mirrors the headline UX of Cursor / Claude Code / Codex: the
//! user writes `@src/main.rs explain this` and the agent receives the
//! original sentence *plus* a structured attachments block with the
//! file's contents. Without this, the user has to either (a) paste
//! file contents inline by hand, or (b) wait for the model to issue a
//! `Read` tool call, which costs an extra turn and burns latency.
//!
//! Design notes:
//!
//!   * **Parse → resolve → expand → render** is the order. We never
//!     mutate the user's text in the parse step; that pipeline is
//!     re-runnable and unit-testable.
//!   * Email-like tokens are rejected at the parser level (the `@` must
//!     be at a word boundary), so `someone@example.com` is left alone.
//!   * Binary files, non-existent paths, directories, and oversized
//!     files are reported as a brief notice instead of inlined. The
//!     reference token stays in the message body so the model still
//!     sees the user's intent.
//!   * Per-attachment + per-message byte caps prevent runaway prompts;
//!     a single `@big-file.json` doesn't accidentally OOM the context
//!     window. Defaults are conservative (256KB per file, 1MB total).
//!
//! What this is **not**: a tool-replacement. The model still has
//! `Read` / `Grep` / `Glob` for everything else. `@file` is purely a
//! UX accelerator for the "I want to talk about *this* file" case.

use std::path::{Path, PathBuf};

/// A single `@path` reference detected in user input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AtReference {
    /// Byte offset of the `@` character in the original input.
    pub start: usize,
    /// Byte offset one past the last char of the token (exclusive).
    pub end: usize,
    /// The path portion: everything after `@`, with trailing
    /// punctuation already stripped (so `@file.rs.` becomes `file.rs`).
    pub path: String,
}

/// Result of expanding a single reference. Tracks whether content was
/// inlined and, if not, why — so the renderer can produce helpful
/// "skipped: too large", "skipped: binary", etc. notices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpansionStatus {
    /// File was read and inlined into the attachments block.
    Inlined {
        bytes: usize,
        canonical_path: PathBuf,
    },
    /// Path didn't resolve to anything on disk.
    NotFound,
    /// Path resolved to a directory; we don't inline directory listings
    /// (cheap to do later if there's demand).
    Directory,
    /// File exceeded `max_per_file_bytes`.
    TooLarge { bytes: usize, limit: usize },
    /// File appears to be binary (contains NUL bytes in the first
    /// sampled window).
    Binary,
    /// File was unreadable (permission denied, I/O error, etc.).
    Unreadable { reason: String },
    /// Inlining was skipped because the per-message budget was
    /// already exhausted by earlier references.
    BudgetExhausted,
}

/// One resolved attachment. Returned by [`expand_at_references`] so the
/// caller (REPL / CLI) can show a one-line summary above the agent's
/// reply ("attached 2 files, 14 KB").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachedFile {
    /// The reference text from the original message (`@src/main.rs`).
    pub reference: String,
    /// The displayed relative path (relative to cwd when possible,
    /// absolute otherwise). This is what we tag the inline block with.
    pub display_path: String,
    pub status: ExpansionStatus,
}

impl AttachedFile {
    /// Was the file actually included in the expanded prompt?
    pub fn is_inlined(&self) -> bool {
        matches!(self.status, ExpansionStatus::Inlined { .. })
    }
}

/// Result of expanding every `@`-reference in a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpansionResult {
    /// The original input with an attachments block appended (only when
    /// at least one reference was successfully inlined). The original
    /// `@token`s are left in place so the model can see what the user
    /// pointed at.
    pub expanded: String,
    /// Every reference we tried to resolve, with its outcome. Even
    /// `NotFound` references are listed — they tell the caller to print
    /// a "couldn't find @foo.rs" hint.
    pub attachments: Vec<AttachedFile>,
}

impl ExpansionResult {
    pub fn inlined_count(&self) -> usize {
        self.attachments.iter().filter(|a| a.is_inlined()).count()
    }

    pub fn skipped_count(&self) -> usize {
        self.attachments.len() - self.inlined_count()
    }

    pub fn total_inlined_bytes(&self) -> usize {
        self.attachments
            .iter()
            .filter_map(|a| match a.status {
                ExpansionStatus::Inlined { bytes, .. } => Some(bytes),
                _ => None,
            })
            .sum()
    }
}

/// Tunable limits + behaviour for the expander. Conservative defaults
/// keep prompts bounded.
#[derive(Debug, Clone, Copy)]
pub struct AtExpansionOptions {
    pub max_per_file_bytes: usize,
    pub max_total_bytes: usize,
    pub max_references: usize,
    /// Sample size used for the binary-detection probe. Files containing
    /// a NUL byte within the first `binary_probe_bytes` are treated as
    /// binary and skipped.
    pub binary_probe_bytes: usize,
}

impl Default for AtExpansionOptions {
    fn default() -> Self {
        Self {
            max_per_file_bytes: 256 * 1024, // 256 KB
            max_total_bytes: 1024 * 1024,   // 1 MB
            max_references: 32,
            binary_probe_bytes: 4096,
        }
    }
}

/// Walk `input` and return every `@<path>` reference we can plausibly
/// resolve. The parser deliberately stays simple: an `@` qualifies
/// when it sits at a word boundary (start of string, after whitespace,
/// or after one of a few brackets/quotes). This avoids capturing email
/// addresses, decorator-style annotations like `@deprecated`, and other
/// false positives.
pub fn parse_at_references(input: &str) -> Vec<AtReference> {
    let bytes = input.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] != b'@' {
            i += 1;
            continue;
        }

        // Word-boundary check: the byte before `@` (if any) must be
        // whitespace or one of a few common "opening" delimiters. This
        // is what stops `someone@example.com` from being captured.
        if i > 0 {
            let prev = bytes[i - 1];
            let is_boundary = matches!(
                prev,
                b' ' | b'\t' | b'\n' | b'\r' | b'(' | b'[' | b'{' | b'\'' | b'"' | b'`' | b','
            );
            if !is_boundary {
                i += 1;
                continue;
            }
        }

        // Scan forward to the end of the path token. We stop at
        // whitespace, end of input, or a closing delimiter. Trailing
        // punctuation (`.`, `,`, `;`, `:`, `!`, `?`, `)`, `]`, `}`,
        // quote chars) is stripped from the captured path — `@file.rs.`
        // at end of sentence should resolve to `file.rs`.
        let start = i;
        let path_start = i + 1; // skip the `@`
        let mut j = path_start;
        while j < bytes.len() {
            let c = bytes[j];
            if matches!(c, b' ' | b'\t' | b'\n' | b'\r' | b'<' | b'>' | b'|' | b'@') {
                break;
            }
            j += 1;
        }
        let mut end_excl = j;

        // Strip the trailing punctuation/quote run, but keep at least
        // one character of path after the `@`.
        while end_excl > path_start {
            let c = bytes[end_excl - 1];
            if matches!(
                c,
                b'.' | b',' | b';' | b':' | b'!' | b'?' | b')' | b']' | b'}' | b'\'' | b'"' | b'`'
            ) {
                end_excl -= 1;
            } else {
                break;
            }
        }

        // Empty path (`@` followed by punctuation only) → skip.
        if end_excl <= path_start {
            i = j;
            continue;
        }

        // We slice on bytes here, which is safe because the boundary
        // chars we accepted as terminators are all single-byte ASCII;
        // any multi-byte UTF-8 sequence within the path is preserved.
        let path = match std::str::from_utf8(&bytes[path_start..end_excl]) {
            Ok(s) => s.to_owned(),
            Err(_) => {
                i = j;
                continue;
            }
        };

        out.push(AtReference {
            start,
            end: end_excl,
            path,
        });

        i = j;
    }

    out
}

/// Resolve and inline every `@`-reference detected in `input`. See the
/// module-level doc for the policy; this is the single public entry
/// point the REPL calls before forwarding text to the agent.
pub fn expand_at_references(
    input: &str,
    cwd: &Path,
    options: AtExpansionOptions,
) -> ExpansionResult {
    let refs = parse_at_references(input);
    if refs.is_empty() {
        return ExpansionResult {
            expanded: input.to_owned(),
            attachments: Vec::new(),
        };
    }

    // Cap the number of references we *attempt* to resolve. Anything
    // past the cap is just left in the prose. We deliberately don't
    // record them as "attachments" — they were ignored entirely.
    let to_process = refs.iter().take(options.max_references);

    let mut attachments: Vec<AttachedFile> = Vec::new();
    let mut budget_remaining = options.max_total_bytes;
    let mut inlined_blocks: Vec<String> = Vec::new();

    for r in to_process {
        let reference_text = format!("@{}", r.path);
        let resolved = resolve_candidate_path(cwd, &r.path);

        let status = match resolved {
            None => ExpansionStatus::NotFound,
            Some(canonical) => classify_and_read(&canonical, options, budget_remaining),
        };

        // For successful reads, append a rendered file block and
        // decrement the budget.
        let display_path = match &status {
            ExpansionStatus::Inlined { canonical_path, .. } => {
                let display = display_relative_to(canonical_path, cwd);
                if let ExpansionStatus::Inlined { bytes, .. } = &status {
                    budget_remaining = budget_remaining.saturating_sub(*bytes);
                }
                // Re-read the file content for the inlined block. This
                // is the second read of the same file; the first was
                // inside `classify_and_read` for size+binary checks.
                // We keep the two phases separate because the first one
                // also enforces the binary-probe and TooLarge gates,
                // which we don't want to repeat-with-different-limits
                // here.
                if let Ok(body) = std::fs::read_to_string(canonical_path) {
                    inlined_blocks.push(render_inline_block(&display, &body));
                }
                display
            }
            _ => display_relative_to_or_raw(&r.path, cwd),
        };

        attachments.push(AttachedFile {
            reference: reference_text,
            display_path,
            status,
        });
    }

    if inlined_blocks.is_empty() {
        // Even with parse hits, nothing successfully inlined. Leave the
        // input untouched so we don't confuse the model with an empty
        // "attachments:" block.
        return ExpansionResult {
            expanded: input.to_owned(),
            attachments,
        };
    }

    let mut expanded = String::with_capacity(
        input.len() + inlined_blocks.iter().map(String::len).sum::<usize>() + 64,
    );
    expanded.push_str(input);
    expanded.push_str("\n\n<deeptide-attachments>\n");
    for block in &inlined_blocks {
        expanded.push_str(block);
        expanded.push('\n');
    }
    expanded.push_str("</deeptide-attachments>\n");

    ExpansionResult {
        expanded,
        attachments,
    }
}

/// Render a single inlined file as a fenced block. Format chosen to
/// mirror what the model already sees in tool-result blocks for `Read`
/// — same `<file path="...">` tag pattern — so the model doesn't have
/// to learn a new schema.
fn render_inline_block(display_path: &str, content: &str) -> String {
    format!(
        "<file path=\"{}\">\n{}\n</file>",
        display_path,
        content.trim_end_matches('\n')
    )
}

/// Try to map a user-typed `@path` token to an existing filesystem
/// path. Order of attempts:
///   1. Absolute path (`@/etc/hosts`) — used verbatim.
///   2. `cwd / path` — the most common case.
///   3. `~/path` — `~` expanded to `$HOME`.
fn resolve_candidate_path(cwd: &Path, token: &str) -> Option<PathBuf> {
    let raw = Path::new(token);

    if raw.is_absolute() {
        return raw.exists().then(|| raw.to_path_buf());
    }

    if let Some(stripped) = token.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        let combined = Path::new(&home).join(stripped);
        if combined.exists() {
            return Some(combined);
        }
    }

    let joined = cwd.join(raw);
    joined.exists().then_some(joined)
}

/// Classify the resolved path (dir? binary? too big?) and, if eligible,
/// declare it as inlined (the actual read happens later in the
/// expander). Splitting "classify" from "render" lets us track the
/// per-message byte budget cleanly.
fn classify_and_read(
    path: &Path,
    options: AtExpansionOptions,
    budget_remaining: usize,
) -> ExpansionStatus {
    let metadata = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(error) => {
            return ExpansionStatus::Unreadable {
                reason: error.to_string(),
            };
        }
    };

    if metadata.is_dir() {
        return ExpansionStatus::Directory;
    }

    let size = metadata.len() as usize;
    if size > options.max_per_file_bytes {
        return ExpansionStatus::TooLarge {
            bytes: size,
            limit: options.max_per_file_bytes,
        };
    }
    if size > budget_remaining {
        return ExpansionStatus::BudgetExhausted;
    }

    // Binary detection: read up to `binary_probe_bytes` and look for a
    // NUL byte. Cheap, deterministic, and matches `git diff`'s heuristic.
    if let Ok(file) = std::fs::File::open(path) {
        use std::io::Read;
        let mut probe = vec![0u8; options.binary_probe_bytes.min(size)];
        let mut handle = file.take(probe.len() as u64);
        if handle.read(&mut probe).is_ok() && probe.contains(&0) {
            return ExpansionStatus::Binary;
        }
    }

    ExpansionStatus::Inlined {
        bytes: size,
        canonical_path: path.to_path_buf(),
    }
}

/// Pretty-print a path relative to `cwd` when it's inside `cwd`,
/// otherwise return the absolute form.
fn display_relative_to(path: &Path, cwd: &Path) -> String {
    path.strip_prefix(cwd)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

/// When the path couldn't be resolved, fall back to whatever the user
/// typed (relative form) for the display tag.
fn display_relative_to_or_raw(token: &str, _cwd: &Path) -> String {
    token.to_owned()
}

#[cfg(test)]
mod tests {
    // Tests panic on bad fixtures by design; the workspace
    // `unwrap_used = "deny"` lint is overly aggressive for setup
    // helpers like `tempdir()` / `fs::write` where a failure is a
    // test infra bug, not a runtime concern. We mirror what other
    // test modules do via explicit `.expect(...)` calls — but for
    // brevity in fixture-heavy tests we opt out here.
    #![allow(clippy::unwrap_used)]

    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parser_extracts_simple_at_reference() {
        let refs = parse_at_references("explain @src/main.rs please");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].path, "src/main.rs");
    }

    #[test]
    fn parser_extracts_multiple_at_references() {
        let refs = parse_at_references("@a.rs and @b.rs together");
        assert_eq!(
            refs.iter().map(|r| r.path.as_str()).collect::<Vec<_>>(),
            vec!["a.rs", "b.rs"]
        );
    }

    #[test]
    fn parser_rejects_email_addresses() {
        let refs = parse_at_references("ping ryan@example.com about it");
        assert!(refs.is_empty(), "email must not be parsed as ref: {refs:?}");
    }

    #[test]
    fn parser_strips_trailing_punctuation() {
        let refs = parse_at_references("see @file.rs. and @other.toml, ok?");
        let paths: Vec<&str> = refs.iter().map(|r| r.path.as_str()).collect();
        assert_eq!(paths, vec!["file.rs", "other.toml"]);
    }

    #[test]
    fn parser_handles_at_at_start_of_input() {
        let refs = parse_at_references("@README.md");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].path, "README.md");
    }

    #[test]
    fn parser_handles_at_inside_quotes() {
        let refs = parse_at_references("`@code.rs` is broken");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].path, "code.rs");
    }

    #[test]
    fn parser_skips_empty_at_tokens() {
        let refs = parse_at_references("look at @. or @, nothing");
        assert!(refs.is_empty(), "empty refs must be dropped: {refs:?}");
    }

    #[test]
    fn parser_supports_subdirectory_paths_with_dots_and_dashes() {
        let refs = parse_at_references("check @path/to/my-file.test.rs done");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].path, "path/to/my-file.test.rs");
    }

    #[test]
    fn parser_rejects_at_glued_to_identifier() {
        // `func()@deprecated` should NOT be parsed as a file reference
        // — close-paren isn't on the boundary-opener allow-list, so
        // the `@` is treated as in-word punctuation. Without this
        // guard, code-review prompts mentioning Java/Python/TS
        // decorators would get peppered with NotFound notices.
        let refs = parse_at_references("call func()@deprecated info");
        assert!(refs.is_empty(), "expected zero refs, got: {refs:?}");
    }

    #[test]
    fn parser_accepts_at_after_open_paren() {
        // Inline asides like `(see @notes.md)` are common in
        // English/Markdown prose; open-paren counts as a boundary
        // opener so the ref still resolves.
        let refs = parse_at_references("(see @notes.md)");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].path, "notes.md");
    }

    #[test]
    fn parser_caps_at_reference_count_via_options() {
        // Parser itself doesn't cap; the *expander* does. Verify parse
        // returns everything so the cap stays observable.
        let many = (0..40)
            .map(|i| format!("@f{i}.rs"))
            .collect::<Vec<_>>()
            .join(" ");
        let refs = parse_at_references(&many);
        assert_eq!(refs.len(), 40);
    }

    #[test]
    fn expander_inlines_existing_text_file() {
        let dir = tempdir().expect("tempdir");
        fs::write(dir.path().join("hello.txt"), "world\n").expect("write");

        let result = expand_at_references(
            "look at @hello.txt now",
            dir.path(),
            AtExpansionOptions::default(),
        );
        assert!(result.expanded.contains("<file path=\"hello.txt\">"));
        assert!(result.expanded.contains("world"));
        assert_eq!(result.inlined_count(), 1);
        assert_eq!(result.skipped_count(), 0);
    }

    #[test]
    fn expander_keeps_original_text_intact_above_attachments() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("x.md"), "# Hi").unwrap();
        let result = expand_at_references(
            "summarise @x.md please",
            dir.path(),
            AtExpansionOptions::default(),
        );
        // Original line must still be the first line of the expansion.
        let first_line = result.expanded.lines().next().unwrap();
        assert_eq!(first_line, "summarise @x.md please");
        assert!(result.expanded.contains("<deeptide-attachments>"));
    }

    #[test]
    fn expander_reports_not_found_for_missing_path() {
        let dir = tempdir().unwrap();
        let result =
            expand_at_references("see @nope.rs", dir.path(), AtExpansionOptions::default());
        assert_eq!(result.attachments.len(), 1);
        assert!(matches!(
            result.attachments[0].status,
            ExpansionStatus::NotFound
        ));
        // No attachments block is emitted when nothing was inlined.
        assert!(!result.expanded.contains("<deeptide-attachments>"));
        // Original text is untouched.
        assert_eq!(result.expanded, "see @nope.rs");
    }

    #[test]
    fn expander_skips_directories() {
        let dir = tempdir().unwrap();
        let sub = dir.path().join("subdir");
        fs::create_dir(&sub).unwrap();
        let result = expand_at_references(
            "show @subdir please",
            dir.path(),
            AtExpansionOptions::default(),
        );
        assert!(matches!(
            result.attachments[0].status,
            ExpansionStatus::Directory
        ));
        assert!(!result.expanded.contains("<deeptide-attachments>"));
    }

    #[test]
    fn expander_skips_binary_files() {
        let dir = tempdir().unwrap();
        // A leading NUL is enough to mark the file binary.
        fs::write(dir.path().join("bin.dat"), b"\x00\x01\x02\x03data").unwrap();
        let result = expand_at_references(
            "look at @bin.dat",
            dir.path(),
            AtExpansionOptions::default(),
        );
        assert!(matches!(
            result.attachments[0].status,
            ExpansionStatus::Binary
        ));
    }

    #[test]
    fn expander_skips_too_large_files() {
        let dir = tempdir().unwrap();
        // 1 KB file but limit set to 10 bytes for the test.
        fs::write(dir.path().join("big.txt"), "x".repeat(1024)).unwrap();
        let opts = AtExpansionOptions {
            max_per_file_bytes: 10,
            ..AtExpansionOptions::default()
        };
        let result = expand_at_references("see @big.txt", dir.path(), opts);
        assert!(matches!(
            result.attachments[0].status,
            ExpansionStatus::TooLarge { .. }
        ));
    }

    #[test]
    fn expander_honours_total_message_budget() {
        let dir = tempdir().unwrap();
        // Two 600-byte files, total 1200 bytes; cap total at 1000.
        fs::write(dir.path().join("a.txt"), "a".repeat(600)).unwrap();
        fs::write(dir.path().join("b.txt"), "b".repeat(600)).unwrap();

        let opts = AtExpansionOptions {
            max_per_file_bytes: 4096,
            max_total_bytes: 1000,
            ..AtExpansionOptions::default()
        };
        let result = expand_at_references("@a.txt then @b.txt", dir.path(), opts);
        // First fits; second runs into BudgetExhausted.
        assert!(matches!(
            result.attachments[0].status,
            ExpansionStatus::Inlined { .. }
        ));
        assert!(matches!(
            result.attachments[1].status,
            ExpansionStatus::BudgetExhausted
        ));
        // Only the inlined file shows up in the expansion text.
        assert_eq!(result.expanded.matches("<file path=").count(), 1);
    }

    #[test]
    fn expander_caps_at_max_references() {
        let dir = tempdir().unwrap();
        for n in 0..5 {
            fs::write(dir.path().join(format!("f{n}.txt")), "x").unwrap();
        }
        let opts = AtExpansionOptions {
            max_references: 2,
            ..AtExpansionOptions::default()
        };
        let result =
            expand_at_references("@f0.txt @f1.txt @f2.txt @f3.txt @f4.txt", dir.path(), opts);
        // Only the first 2 are processed at all.
        assert_eq!(result.attachments.len(), 2);
    }

    #[test]
    fn expander_supports_subdirectory_paths() {
        let dir = tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").unwrap();
        let result = expand_at_references(
            "summary of @src/main.rs",
            dir.path(),
            AtExpansionOptions::default(),
        );
        assert!(result.expanded.contains("<file path=\"src/main.rs\">"));
        assert!(result.expanded.contains("fn main() {}"));
    }

    #[test]
    fn expander_returns_zero_inlined_when_no_refs() {
        let dir = tempdir().unwrap();
        let r = expand_at_references(
            "just a plain message",
            dir.path(),
            AtExpansionOptions::default(),
        );
        assert_eq!(r.expanded, "just a plain message");
        assert!(r.attachments.is_empty());
    }
}
