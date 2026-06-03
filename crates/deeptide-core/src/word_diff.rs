//! Token-level ("word") diff for intra-line change highlighting, dependency
//! free. The line diff in `diff_preview.rs` tells you WHICH lines changed; this
//! tells you which WORDS within a paired old/new line changed, so a preview can
//! dim the common text and brighten only the actual edit — the zero-cli /
//! `git --word-diff` experience, without pulling in a diff crate.
//!
//! Tokenization keeps words (`[A-Za-z0-9_]+`), whitespace runs, and individual
//! punctuation as separate tokens, so `foo_bar(x)` → `foo_bar` `(` `x` `)` and
//! a one-argument change highlights just that argument, not the whole call.

/// One run of the diffed line: text that is unchanged between the two sides
/// (`Common`) or text that exists on only this side (`Changed`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Span {
    Common(String),
    Changed(String),
}

impl Span {
    /// The text this span carries, regardless of kind.
    pub fn text(&self) -> &str {
        match self {
            Span::Common(s) | Span::Changed(s) => s,
        }
    }

    pub fn is_changed(&self) -> bool {
        matches!(self, Span::Changed(_))
    }
}

/// Diff two lines at token granularity. Returns `(old_spans, new_spans)`: the
/// `Common` runs are identical and aligned between the two; the `Changed` runs
/// are the deletions (old side) and insertions (new side). Adjacent spans of
/// the same kind are coalesced so the caller emits one SGR run per change, not
/// one per token.
pub fn word_diff(old: &str, new: &str) -> (Vec<Span>, Vec<Span>) {
    let old_tokens = tokenize(old);
    let new_tokens = tokenize(new);
    let lcs = lcs_indices(&old_tokens, &new_tokens);

    let mut old_spans = SpanBuilder::default();
    let mut new_spans = SpanBuilder::default();

    let mut oi = 0;
    let mut ni = 0;
    for (lo, ln) in &lcs {
        while oi < *lo {
            old_spans.push_changed(old_tokens[oi]);
            oi += 1;
        }
        while ni < *ln {
            new_spans.push_changed(new_tokens[ni]);
            ni += 1;
        }
        // The matched token is common to both sides.
        old_spans.push_common(old_tokens[*lo]);
        new_spans.push_common(new_tokens[*ln]);
        oi = lo + 1;
        ni = ln + 1;
    }
    // Trailing tokens past the last LCS match are pure changes.
    while oi < old_tokens.len() {
        old_spans.push_changed(old_tokens[oi]);
        oi += 1;
    }
    while ni < new_tokens.len() {
        new_spans.push_changed(new_tokens[ni]);
        ni += 1;
    }

    (old_spans.finish(), new_spans.finish())
}

/// Accumulates tokens into coalesced [`Span`]s — consecutive pushes of the same
/// kind grow one span rather than producing many adjacent ones.
#[derive(Default)]
struct SpanBuilder {
    spans: Vec<Span>,
}

impl SpanBuilder {
    fn push_common(&mut self, token: &str) {
        match self.spans.last_mut() {
            Some(Span::Common(s)) => s.push_str(token),
            _ => self.spans.push(Span::Common(token.to_owned())),
        }
    }

    fn push_changed(&mut self, token: &str) {
        match self.spans.last_mut() {
            Some(Span::Changed(s)) => s.push_str(token),
            _ => self.spans.push(Span::Changed(token.to_owned())),
        }
    }

    fn finish(self) -> Vec<Span> {
        self.spans
    }
}

/// Split a line into tokens: maximal `[A-Za-z0-9_]` words, maximal whitespace
/// runs, and individual other characters (punctuation/symbols). Returns string
/// slices into `line` so no allocation happens during tokenization.
fn tokenize(line: &str) -> Vec<&str> {
    let mut tokens = Vec::new();
    let bytes = line.as_bytes();
    let mut i = 0;
    let len = line.len();
    while i < len {
        let start = i;
        let ch = line[i..].chars().next().unwrap_or('\0');
        if is_word_char(ch) {
            while i < len {
                let c = line[i..].chars().next().unwrap_or('\0');
                if !is_word_char(c) {
                    break;
                }
                i += c.len_utf8();
            }
        } else if ch.is_whitespace() {
            while i < len {
                let c = line[i..].chars().next().unwrap_or('\0');
                if !c.is_whitespace() {
                    break;
                }
                i += c.len_utf8();
            }
        } else {
            i += ch.len_utf8();
        }
        // Guard against a zero-width step on malformed input.
        if i == start {
            i += 1;
        }
        let _ = bytes;
        tokens.push(&line[start..i]);
    }
    tokens
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Classic LCS via dynamic programming, returning the matched index pairs
/// `(old_index, new_index)` in order. Token lines are short (a source line),
/// so the `O(n·m)` table is cheap and not worth a fancier algorithm.
fn lcs_indices(old: &[&str], new: &[&str]) -> Vec<(usize, usize)> {
    let n = old.len();
    let m = new.len();
    if n == 0 || m == 0 {
        return Vec::new();
    }
    // dp[i][j] = LCS length of old[i..] and new[j..].
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if old[i] == new[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    // Backtrack from the top-left to recover the matched pairs.
    let mut pairs = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if old[i] == new[j] {
            pairs.push((i, j));
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(spans: &[Span]) -> String {
        spans.iter().map(Span::text).collect()
    }

    fn changed_text(spans: &[Span]) -> String {
        spans
            .iter()
            .filter(|s| s.is_changed())
            .map(Span::text)
            .collect()
    }

    #[test]
    fn reconstructs_each_side_exactly() {
        // The single most important invariant: concatenating the spans
        // reproduces the original line verbatim (no dropped/added chars).
        let (old, new) = word_diff("let x = foo(a, b);", "let x = foo(a, c);");
        assert_eq!(texts(&old), "let x = foo(a, b);");
        assert_eq!(texts(&new), "let x = foo(a, c);");
    }

    #[test]
    fn highlights_only_the_changed_word() {
        let (old, new) = word_diff("let x = foo(a, b);", "let x = foo(a, c);");
        // Only `b` deleted, only `c` inserted — the rest is common.
        assert_eq!(changed_text(&old), "b");
        assert_eq!(changed_text(&new), "c");
    }

    #[test]
    fn identical_lines_have_no_changed_spans() {
        let (old, new) = word_diff("unchanged line", "unchanged line");
        assert!(old.iter().all(|s| !s.is_changed()));
        assert!(new.iter().all(|s| !s.is_changed()));
    }

    #[test]
    fn fully_different_lines_are_all_changed() {
        let (old, new) = word_diff("aaa", "bbb");
        assert_eq!(changed_text(&old), "aaa");
        assert_eq!(changed_text(&new), "bbb");
        // No common span survived.
        assert!(old.iter().all(|s| s.is_changed()));
    }

    #[test]
    fn pure_insertion_marks_only_new_side() {
        let (old, new) = word_diff("foo bar", "foo new bar");
        assert_eq!(changed_text(&old), "", "nothing deleted from old");
        // The inserted word (and the space that came with it) is the change.
        assert!(changed_text(&new).contains("new"));
        assert_eq!(texts(&new), "foo new bar");
    }

    #[test]
    fn coalesces_adjacent_changed_tokens_into_one_span() {
        // `foo(a, b)` → `foo(x, y)`: `a`/`b` → `x`/`y`. Each changed run is a
        // single Changed span (per side) rather than many token-sized ones,
        // but common punctuation between them stays common.
        let (_old, new) = word_diff("foo(a, b)", "foo(x, y)");
        let changed_runs = new.iter().filter(|s| s.is_changed()).count();
        // `x` and `y` are separated by the common `, ` — so two changed runs.
        assert_eq!(changed_runs, 2, "spans: {new:?}");
    }

    #[test]
    fn handles_unicode_without_splitting_codepoints() {
        let (old, new) = word_diff("价格 = 100", "价格 = 200");
        assert_eq!(texts(&old), "价格 = 100");
        assert_eq!(texts(&new), "价格 = 200");
        assert_eq!(changed_text(&old), "100");
        assert_eq!(changed_text(&new), "200");
    }

    #[test]
    fn empty_lines_are_safe() {
        let (old, new) = word_diff("", "");
        assert!(old.is_empty());
        assert!(new.is_empty());
        let (old, new) = word_diff("", "added");
        assert!(old.is_empty());
        assert_eq!(changed_text(&new), "added");
    }
}
