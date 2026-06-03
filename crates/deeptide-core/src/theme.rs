//! Selectable colour themes for syntax highlighting and other TUI accents.
//!
//! Design constraints that shape this module:
//!  * **No regressions.** The `dark` theme reproduces the exact SGR codes the
//!    code used before theming existed, so every existing snapshot/assertion
//!    test stays green. Theming is purely additive.
//!  * **No signature churn.** Rather than thread a `Theme` through every render
//!    function (markdown → code block → highlighter, dozens of call sites), the
//!    active theme is a process-global set ONCE at startup (`--theme` / config),
//!    read wherever a palette colour is needed — the same shape as a logger.
//!    Tests don't set it, so they observe the `dark` default.
//!
//! A `SyntaxPalette` is just five SGR introducer strings (`"\x1b[35m"` …). We
//! keep them as owned `String`s so future themes can be loaded from config
//! (24-bit `\x1b[38;2;R;G;Bm`) without changing the type.

use std::sync::OnceLock;

/// The colours a code-fence highlighter applies per token class. Each field is
/// an ANSI SGR introducer; the highlighter appends the text and a reset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxPalette {
    pub keyword: String,
    pub string: String,
    pub comment: String,
    pub number: String,
    pub type_: String,
}

/// The base colours a unified-diff preview applies to inserted / deleted lines.
/// Word-level emphasis (changed tokens) is layered on top with theme-independent
/// reverse-video, so only these two base colours are themed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffPalette {
    pub added: String,
    pub removed: String,
}

/// The *colour* accents a markdown render applies. Only colours live here —
/// text attributes (bold/italic/dim/strikethrough) are structural, theme-
/// independent, and stay hard-coded in the renderer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkdownPalette {
    /// Headings + the blockquote/rule bar (the `Accent` style).
    pub accent: String,
    /// Inline `` `code` `` spans.
    pub inline_code: String,
    /// `[label](url)` link labels.
    pub link: String,
}

/// A named theme. Carries the syntax + diff + markdown palettes; the struct is
/// the growth point for further status-line accents as more consumers opt in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub syntax: SyntaxPalette,
    pub diff: DiffPalette,
    pub markdown: MarkdownPalette,
}

impl Theme {
    /// The default dark theme. These SGR codes are byte-for-byte what the
    /// highlighter hard-coded before themes existed — do not change them
    /// without updating the syntax-highlighter snapshot tests in lockstep.
    pub fn dark() -> Self {
        Self {
            name: "dark",
            syntax: SyntaxPalette {
                keyword: "\x1b[35m".to_owned(), // magenta
                string: "\x1b[32m".to_owned(),  // green
                comment: "\x1b[90m".to_owned(), // bright-black
                number: "\x1b[33m".to_owned(),  // yellow
                type_: "\x1b[36m".to_owned(),   // cyan
            },
            diff: DiffPalette {
                // Byte-for-byte the pre-theming diff colours.
                added: "\x1b[32m".to_owned(),   // green
                removed: "\x1b[31m".to_owned(), // red
            },
            markdown: MarkdownPalette {
                // Byte-for-byte the pre-theming markdown colours.
                accent: "\x1b[36m".to_owned(), // cyan headings/bars
                inline_code: "\x1b[36m".to_owned(), // cyan inline code
                link: "\x1b[34m".to_owned(),   // blue links
            },
        }
    }

    /// Tuned for light terminal backgrounds: swap the bright-black comment
    /// (invisible on white) for a readable mid-grey, and the yellow number for
    /// a darker tone that holds contrast on a pale background.
    pub fn light() -> Self {
        Self {
            name: "light",
            syntax: SyntaxPalette {
                keyword: "\x1b[35m".to_owned(), // magenta still reads on white
                string: "\x1b[32m".to_owned(),  // green
                comment: "\x1b[37m".to_owned(), // grey (bright-black vanishes on white)
                number: "\x1b[34m".to_owned(),  // blue (yellow vanishes on white)
                type_: "\x1b[36m".to_owned(),   // cyan
            },
            diff: DiffPalette {
                // Green/red read fine on white; keep them.
                added: "\x1b[32m".to_owned(),
                removed: "\x1b[31m".to_owned(),
            },
            markdown: MarkdownPalette {
                // Cyan headings/code wash out on white → blue; links stay blue.
                accent: "\x1b[34m".to_owned(),
                inline_code: "\x1b[34m".to_owned(),
                link: "\x1b[34m".to_owned(),
            },
        }
    }

    /// Maximum-contrast variant using bold + bright colours, for low-vision use
    /// or harsh ambient light.
    pub fn high_contrast() -> Self {
        Self {
            name: "high-contrast",
            syntax: SyntaxPalette {
                keyword: "\x1b[1;95m".to_owned(), // bold bright magenta
                string: "\x1b[1;92m".to_owned(),  // bold bright green
                comment: "\x1b[1;37m".to_owned(), // bold white
                number: "\x1b[1;93m".to_owned(),  // bold bright yellow
                type_: "\x1b[1;96m".to_owned(),   // bold bright cyan
            },
            diff: DiffPalette {
                added: "\x1b[1;92m".to_owned(),   // bold bright green
                removed: "\x1b[1;91m".to_owned(), // bold bright red
            },
            markdown: MarkdownPalette {
                accent: "\x1b[1;96m".to_owned(),      // bold bright cyan
                inline_code: "\x1b[1;96m".to_owned(), // bold bright cyan
                link: "\x1b[1;94m".to_owned(),        // bold bright blue
            },
        }
    }
}

/// Resolve a theme by name (case-insensitive). Accepts the canonical names and
/// a couple of friendly aliases. `None` lets the caller report an error and
/// list [`theme_names`].
pub fn by_name(name: &str) -> Option<Theme> {
    match name.trim().to_ascii_lowercase().as_str() {
        "dark" | "default" => Some(Theme::dark()),
        "light" => Some(Theme::light()),
        "high-contrast" | "high_contrast" | "highcontrast" | "hc" => Some(Theme::high_contrast()),
        _ => None,
    }
}

/// The names a user can pass to `--theme`, for help text and error messages.
pub fn theme_names() -> &'static [&'static str] {
    &["dark", "light", "high-contrast"]
}

static ACTIVE: OnceLock<Theme> = OnceLock::new();

/// Install the process-wide active theme. Call once at startup, before any
/// rendering. Returns `Err` (with the current theme name) if a theme was
/// already set — the active theme is intentionally immutable after init so
/// concurrent renderers never observe a mid-flight palette change.
pub fn set_active(theme: Theme) -> Result<(), &'static str> {
    ACTIVE.set(theme).map_err(|_| active().name)
}

/// The active theme, defaulting to [`Theme::dark`] when none was installed —
/// so library code and tests that never call [`set_active`] see exactly the
/// pre-theming colours.
pub fn active() -> &'static Theme {
    ACTIVE.get_or_init(Theme::dark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_theme_matches_the_legacy_hardcoded_palette() {
        // Guard the no-regression contract: the dark palette MUST equal the
        // SGR codes the highlighter used before theming, or existing snapshot
        // tests would silently drift.
        let d = Theme::dark().syntax;
        assert_eq!(d.keyword, "\x1b[35m");
        assert_eq!(d.string, "\x1b[32m");
        assert_eq!(d.comment, "\x1b[90m");
        assert_eq!(d.number, "\x1b[33m");
        assert_eq!(d.type_, "\x1b[36m");
        // Diff base colours must also equal the pre-theming literals.
        let diff = Theme::dark().diff;
        assert_eq!(diff.added, "\x1b[32m");
        assert_eq!(diff.removed, "\x1b[31m");
        // Markdown accents likewise.
        let md = Theme::dark().markdown;
        assert_eq!(md.accent, "\x1b[36m");
        assert_eq!(md.inline_code, "\x1b[36m");
        assert_eq!(md.link, "\x1b[34m");
    }

    #[test]
    fn high_contrast_bolds_the_diff_colours() {
        let hc = Theme::high_contrast().diff;
        assert!(
            hc.added.contains("1;"),
            "added should be bold: {:?}",
            hc.added
        );
        assert!(
            hc.removed.contains("1;"),
            "removed should be bold: {:?}",
            hc.removed
        );
        assert_ne!(hc.added, Theme::dark().diff.added);
    }

    #[test]
    fn by_name_resolves_canonical_names_and_aliases() {
        assert_eq!(by_name("dark").expect("known theme").name, "dark");
        assert_eq!(by_name("DEFAULT").expect("known theme").name, "dark");
        assert_eq!(by_name("light").expect("known theme").name, "light");
        assert_eq!(
            by_name(" High-Contrast ").expect("known theme").name,
            "high-contrast"
        );
        assert_eq!(by_name("hc").expect("known theme").name, "high-contrast");
        assert!(by_name("solarized").is_none());
    }

    #[test]
    fn light_theme_differs_from_dark_where_dark_is_unreadable_on_white() {
        // The whole point of `light`: comment + number must change (bright-black
        // and yellow disappear on a white background), while green/cyan/magenta
        // that already read on white may stay.
        let dark = Theme::dark().syntax;
        let light = Theme::light().syntax;
        assert_ne!(light.comment, dark.comment);
        assert_ne!(light.number, dark.number);
    }

    #[test]
    fn active_defaults_to_dark_without_set() {
        // In the test process no one calls set_active, so the default holds.
        assert_eq!(active().name, "dark");
        assert_eq!(active().syntax.keyword, "\x1b[35m");
    }
}
