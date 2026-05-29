//! Single source of truth for terminal display-width measurement.
//!
//! Before this module, three separate modules (`tui`, `markdown`,
//! `status_bar`) each carried their own hand-rolled `is_wide` table
//! covering a small subset of the East-Asian Width ranges. Any divergence
//! between them caused width drift — the input bar's wrap calc might
//! disagree with the markdown renderer's line-break calc, which would
//! disagree with the status bar's truncate calc, producing visibly
//! misaligned output for CJK / emoji / fullwidth punctuation.
//!
//! We now route every display-width measurement through this module,
//! which delegates to the well-maintained `unicode-width` crate. That
//! crate's tables are auto-generated from the Unicode database, so any
//! new East-Asian Wide / Fullwidth blocks (CJK extensions, emoji
//! supplementary planes) are picked up by upgrading the crate, never by
//! us hand-editing a `matches!` range.
//!
//! ANSI escape sequences are stripped before measurement — they're
//! zero-width in the terminal but `unicode-width` would treat their
//! control bytes as width 1 each.

use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// The visible cell width of a single character, treating non-printable
/// and zero-width characters (ZWJ, VS-15/16, control codes) as 0 and
/// East-Asian Wide / Fullwidth characters as 2.
///
/// Used by per-character wrap logic (input bar, status bar truncate).
pub fn char_width(ch: char) -> usize {
    ch.width().unwrap_or(0)
}

/// True iff `ch` occupies more than one terminal cell (East-Asian Wide,
/// Fullwidth, or any emoji that `unicode-width` classifies as width 2).
/// Used to gate "wrap by character" behaviour for CJK runs where breaking
/// on spaces produces unusable layouts.
pub fn is_wide(ch: char) -> bool {
    char_width(ch) >= 2
}

/// The visible cell width of a string, ANSI-stripping first so styling
/// escapes don't inflate the count. This is the right primitive for
/// layout: how many columns will this string occupy when written to a
/// terminal?
pub fn display_width(text: &str) -> usize {
    strip_ansi(text).width()
}

/// Strip CSI / SGR escape sequences (`\x1b[…m`, `\x1b[…K`, etc.) from a
/// string. Public so other modules don't re-roll the same loop. The
/// stripper is intentionally permissive — it accepts the final byte
/// range `@..=~` defined by ECMA-48 for CSI terminators — so newer SGR
/// sequences (e.g. 24-bit colour `\x1b[38;2;R;G;Bm`) are handled
/// correctly.
pub fn strip_ansi(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
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
    use super::{char_width, display_width, is_wide, strip_ansi};

    #[test]
    fn ascii_is_one_cell_wide() {
        for ch in 'a'..='z' {
            assert_eq!(char_width(ch), 1, "{ch:?} expected width 1");
            assert!(!is_wide(ch), "{ch:?} should not be wide");
        }
        assert_eq!(char_width('0'), 1);
        assert_eq!(char_width('$'), 1);
        assert_eq!(char_width(' '), 1);
    }

    #[test]
    fn cjk_ideographs_are_two_cells_wide() {
        // Plain CJK Unified Ideographs — the screenshot used these.
        for ch in ['文', '件', '在', '哪', '个', '目', '录', '中', '国', '日'] {
            assert_eq!(char_width(ch), 2, "{ch:?} expected width 2");
            assert!(is_wide(ch), "{ch:?} should be wide");
        }
    }

    #[test]
    fn fullwidth_punctuation_is_two_cells_wide() {
        for ch in ['，', '。', '：', '；', '？', '！', '（', '）', '【', '】'] {
            assert_eq!(char_width(ch), 2, "{ch:?} expected width 2");
        }
    }

    #[test]
    fn hangul_and_kana_are_two_cells_wide() {
        // Hangul Syllables — Korean.
        for ch in ['가', '한', '글', '안', '녕'] {
            assert_eq!(char_width(ch), 2, "{ch:?} expected width 2");
        }
        // Hiragana / Katakana — Japanese.
        for ch in ['あ', 'い', 'う', 'カ', 'タ', 'カ', 'ナ'] {
            assert_eq!(char_width(ch), 2, "{ch:?} expected width 2");
        }
    }

    #[test]
    fn halfwidth_kana_is_one_cell_wide() {
        // The halfwidth katakana block (U+FF65..=U+FF9F) is EAW=Na →
        // width 1, regardless of how it looks in some fonts.
        assert_eq!(char_width('ｱ'), 1);
        assert_eq!(char_width('ｶ'), 1);
        // Halfwidth voiced sound mark (U+FF9E) is classified as a
        // combining mark — it stacks on the previous base glyph and
        // doesn't take a cell of its own.
        assert_eq!(char_width('\u{FF9E}'), 0);
    }

    #[test]
    fn emoji_are_two_cells_wide() {
        // Single-codepoint emoji classified as EAW=W or as Wide by
        // unicode-width 0.2's emoji table.
        for ch in ['😀', '🚀', '🎉', '📦', '🔥'] {
            assert_eq!(char_width(ch), 2, "{ch:?} expected width 2");
        }
    }

    #[test]
    fn zero_width_joiners_and_combining_marks_are_zero_cells() {
        // ZWJ U+200D — joins emoji into family/profession sequences but
        // doesn't take its own cell.
        assert_eq!(char_width('\u{200D}'), 0);
        // Combining acute accent — doesn't advance the cursor.
        assert_eq!(char_width('\u{0301}'), 0);
        // Variation selector 16 (emoji presentation).
        assert_eq!(char_width('\u{FE0F}'), 0);
    }

    #[test]
    fn display_width_sums_chars_for_mixed_strings() {
        // ASCII + CJK + ASCII — used everywhere in `git diff` / log
        // output. We want a single number that matches what the terminal
        // will render.
        assert_eq!(display_width("game.html在"), 9 + 2);
        assert_eq!(display_width("你好 world"), 4 + 1 + 5);
        assert_eq!(display_width(""), 0);
    }

    #[test]
    fn display_width_strips_ansi_escapes_before_measuring() {
        // SGR colour codes are zero-width on the wire — we must not
        // count them. This is the failure mode that historically caused
        // status-bar truncation to misalign once colour was enabled.
        assert_eq!(display_width("\x1b[31mhello\x1b[0m"), 5);
        // 24-bit colour SGR (RGB foreground) — multi-arg parameter form.
        assert_eq!(display_width("\x1b[38;2;255;0;0mred\x1b[0m"), 3);
        // CSI erase-line — must also be stripped.
        assert_eq!(display_width("\x1b[2Kafter clear"), 11);
    }

    #[test]
    fn display_width_handles_cjk_with_ansi_styling() {
        // The combination that bites in practice: bold + Chinese.
        // \x1b[1m前缀\x1b[0m  →  "前缀" is 2 chars × 2 cells = 4.
        assert_eq!(display_width("\x1b[1m前缀\x1b[0m"), 4);
        assert_eq!(display_width("\x1b[32m✓\x1b[0m \x1b[1mRead\x1b[0m"), 6);
    }

    #[test]
    fn strip_ansi_handles_truncated_escapes_safely() {
        // A stray `\x1b[` at end-of-string mustn't panic or drop chars
        // beyond it — we just bail out of the strip loop. This guards
        // against partial chunks during streaming.
        assert_eq!(strip_ansi("hello\x1b["), "hello");
        assert_eq!(strip_ansi("\x1b[3"), "");
    }
}
