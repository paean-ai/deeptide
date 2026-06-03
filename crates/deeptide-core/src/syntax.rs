//! Lightweight, dependency-free syntax highlighting for fenced code blocks.
//!
//! This is deliberately a *line-based* tokenizer with no cross-line state — it
//! trades the perfect fidelity of a full grammar (syntect et al.) for zero
//! dependencies, instant build time, and a small binary. Multi-line constructs
//! (block comments, triple-quoted strings) therefore degrade gracefully to
//! per-line coloring rather than being tracked across the fence.
//!
//! Mirrors zero-cli's pure-port philosophy: cover the languages people actually
//! paste (rust/python/js-ts/json/bash/go), color the token classes that carry
//! the most signal (keywords, strings, comments, numbers, types), and fall back
//! to the caller's uniform color for anything unrecognised.

/// A coloured token class. `Plain` carries no escape so ordinary punctuation and
/// whitespace stay the terminal's default foreground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tok {
    Keyword,
    Str,
    Comment,
    Number,
    Type,
    Plain,
}

impl Tok {
    /// The SGR introducer for this class, drawn from the active theme's palette
    /// (default `dark`, whose codes match the pre-theming literals). `Plain`
    /// returns `None` (emitted raw, no styling).
    fn sgr(self, palette: &crate::theme::SyntaxPalette) -> Option<&str> {
        match self {
            Tok::Keyword => Some(palette.keyword.as_str()),
            Tok::Str => Some(palette.string.as_str()),
            Tok::Comment => Some(palette.comment.as_str()),
            Tok::Number => Some(palette.number.as_str()),
            Tok::Type => Some(palette.type_.as_str()),
            Tok::Plain => None,
        }
    }
}

/// Per-language lexing rules. Intentionally tiny — just enough to classify the
/// high-signal tokens.
struct LangSpec {
    /// Line-comment introducers (`//`, `#`, `--`). The rest of the line after
    /// the first match (outside a string) is a comment.
    line_comments: &'static [&'static str],
    /// Characters that open/close a single-line string literal.
    string_delims: &'static [char],
    /// Reserved words coloured as keywords.
    keywords: &'static [&'static str],
    /// Colour `Capitalized` identifiers as types (Rust/Go/TS convention).
    cap_types: bool,
    /// Treat `$name` as a variable/type reference (shell).
    dollar_vars: bool,
}

/// Resolve a fence info-string (already lowercased by the caller) to a spec.
/// `None` means "unknown language → let the caller use its uniform colour".
fn lang_spec(language: &str) -> Option<LangSpec> {
    let lang = language.trim().to_ascii_lowercase();
    Some(match lang.as_str() {
        "rust" | "rs" => LangSpec {
            line_comments: &["//"],
            // NOTE: `'` is deliberately excluded — Rust lifetimes (`'static`,
            // `'a`) would otherwise be parsed as unterminated string literals
            // and swallow the rest of the line. Char literals lose colour; an
            // acceptable trade for not corrupting ordinary code.
            string_delims: &['"'],
            keywords: &[
                "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
                "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
                "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct",
                "super", "trait", "true", "type", "unsafe", "use", "where", "while",
            ],
            cap_types: true,
            dollar_vars: false,
        },
        "python" | "py" => LangSpec {
            line_comments: &["#"],
            string_delims: &['"', '\''],
            keywords: &[
                "and", "as", "assert", "async", "await", "break", "class", "continue", "def",
                "del", "elif", "else", "except", "False", "finally", "for", "from", "global", "if",
                "import", "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass", "raise",
                "return", "True", "try", "while", "with", "yield",
            ],
            cap_types: false,
            dollar_vars: false,
        },
        "javascript" | "js" | "jsx" | "typescript" | "ts" | "tsx" => LangSpec {
            line_comments: &["//"],
            string_delims: &['"', '\'', '`'],
            keywords: &[
                "abstract",
                "any",
                "as",
                "async",
                "await",
                "boolean",
                "break",
                "case",
                "catch",
                "class",
                "const",
                "continue",
                "debugger",
                "default",
                "delete",
                "do",
                "else",
                "enum",
                "export",
                "extends",
                "false",
                "finally",
                "for",
                "from",
                "function",
                "get",
                "if",
                "implements",
                "import",
                "in",
                "instanceof",
                "interface",
                "let",
                "new",
                "null",
                "number",
                "of",
                "private",
                "protected",
                "public",
                "readonly",
                "return",
                "set",
                "static",
                "string",
                "super",
                "switch",
                "this",
                "throw",
                "true",
                "try",
                "type",
                "typeof",
                "undefined",
                "var",
                "void",
                "while",
                "yield",
            ],
            cap_types: true,
            dollar_vars: false,
        },
        "json" | "jsonc" => LangSpec {
            line_comments: &["//"],
            string_delims: &['"'],
            keywords: &["true", "false", "null"],
            cap_types: false,
            dollar_vars: false,
        },
        "bash" | "sh" | "shell" | "zsh" | "console" => LangSpec {
            line_comments: &["#"],
            string_delims: &['"', '\''],
            keywords: &[
                "if", "then", "else", "elif", "fi", "for", "while", "until", "do", "done", "case",
                "esac", "in", "function", "return", "local", "export", "readonly", "declare",
                "source", "alias", "set", "unset", "echo", "cd", "exit", "trap",
            ],
            cap_types: false,
            dollar_vars: true,
        },
        "go" | "golang" => LangSpec {
            line_comments: &["//"],
            // Backtick raw strings + double-quoted; `'` runes excluded (rare,
            // and avoids the same single-quote ambiguity as Rust).
            string_delims: &['"', '`'],
            keywords: &[
                "break",
                "case",
                "chan",
                "const",
                "continue",
                "default",
                "defer",
                "else",
                "fallthrough",
                "for",
                "func",
                "go",
                "goto",
                "if",
                "import",
                "interface",
                "map",
                "package",
                "range",
                "return",
                "select",
                "struct",
                "switch",
                "type",
                "var",
                "true",
                "false",
                "nil",
            ],
            cap_types: true,
            dollar_vars: false,
        },
        _ => return None,
    })
}

/// Highlight a single line of code in `language`, returning an ANSI-coloured
/// string. Returns `None` when the language is unrecognised so the caller can
/// fall back to its own uniform colouring.
///
/// The caller is responsible for only invoking this when colour is enabled.
pub fn highlight_line(language: &str, line: &str) -> Option<String> {
    let spec = lang_spec(language)?;
    Some(highlight_with(&spec, line, &crate::theme::active().syntax))
}

/// Is this language known to the highlighter? Lets callers decide layout
/// (e.g. whether to bother per-line) without building a string.
pub fn is_supported(language: &str) -> bool {
    lang_spec(language).is_some()
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn highlight_with(spec: &LangSpec, line: &str, palette: &crate::theme::SyntaxPalette) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len() + 16);
    let mut plain = String::new();
    let mut i = 0;

    // Flush any buffered plain run verbatim (no escape codes).
    macro_rules! flush_plain {
        ($out:expr, $plain:expr) => {
            if !$plain.is_empty() {
                $out.push_str(&$plain);
                $plain.clear();
            }
        };
    }

    while i < chars.len() {
        // 1. Line comment: matches an introducer at this position → rest of
        //    line is a comment. (We are outside any string here.)
        if let Some(prefix) = spec
            .line_comments
            .iter()
            .find(|p| starts_with_at(&chars, i, p))
        {
            flush_plain!(out, plain);
            let rest: String = chars[i..].iter().collect();
            emit(&mut out, Tok::Comment, &rest, palette);
            let _ = prefix;
            break;
        }

        let c = chars[i];

        // 2. String literal opened by a known delimiter. Consumes through the
        //    matching unescaped delimiter, or to end-of-line if unterminated.
        if spec.string_delims.contains(&c) {
            flush_plain!(out, plain);
            let (lit, next) = scan_string(&chars, i, c);
            emit(&mut out, Tok::Str, &lit, palette);
            i = next;
            continue;
        }

        // 3. Shell `$name` / `${name}` variable reference.
        if spec.dollar_vars && c == '$' {
            flush_plain!(out, plain);
            let (var, next) = scan_dollar(&chars, i);
            emit(&mut out, Tok::Type, &var, palette);
            i = next;
            continue;
        }

        // 4. Number literal — only when not glued to the tail of an identifier
        //    (so `utf8` stays one identifier, but `0x1F` / `3.14` colour).
        let prev_ident = i > 0 && is_ident_continue(chars[i - 1]);
        if c.is_ascii_digit() && !prev_ident {
            flush_plain!(out, plain);
            let (num, next) = scan_number(&chars, i);
            emit(&mut out, Tok::Number, &num, palette);
            i = next;
            continue;
        }

        // 5. Identifier / keyword / type.
        if is_ident_start(c) {
            let start = i;
            i += 1;
            while i < chars.len() && is_ident_continue(chars[i]) {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let tok = classify_word(spec, &word, &chars, i);
            if tok == Tok::Plain {
                plain.push_str(&word);
            } else {
                flush_plain!(out, plain);
                emit(&mut out, tok, &word, palette);
            }
            continue;
        }

        // 6. Anything else: punctuation / whitespace → plain.
        plain.push(c);
        i += 1;
    }
    flush_plain!(out, plain);
    out
}

/// Decide the class of a completed identifier `word`. `after` is the index just
/// past the word, used to spot a following `(` (function call) — currently we
/// only special-case keywords and capitalised types to stay conservative.
fn classify_word(spec: &LangSpec, word: &str, _chars: &[char], _after: usize) -> Tok {
    if spec.keywords.contains(&word) {
        return Tok::Keyword;
    }
    if spec.cap_types
        && word
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_uppercase())
        // Pure SCREAMING_CASE constants read better as plain than as a "type".
        && word.chars().any(|c| c.is_ascii_lowercase())
    {
        return Tok::Type;
    }
    Tok::Plain
}

/// Scan a string literal starting at `open` (whose char is `delim`). Returns the
/// literal text (including both delimiters) and the index just past it. Handles
/// backslash escapes; an unterminated literal runs to end-of-line.
fn scan_string(chars: &[char], open: usize, delim: char) -> (String, usize) {
    let mut lit = String::new();
    lit.push(chars[open]);
    let mut i = open + 1;
    // Backtick (template/raw) strings don't honour backslash escapes.
    let escapes = delim != '`';
    while i < chars.len() {
        let c = chars[i];
        lit.push(c);
        i += 1;
        if escapes && c == '\\' {
            // Consume the escaped char verbatim if present.
            if i < chars.len() {
                lit.push(chars[i]);
                i += 1;
            }
            continue;
        }
        if c == delim {
            break;
        }
    }
    (lit, i)
}

/// Scan a `$VAR`, `${VAR}`, or `$1` reference starting at the `$`.
fn scan_dollar(chars: &[char], open: usize) -> (String, usize) {
    let mut var = String::from("$");
    let mut i = open + 1;
    if i < chars.len() && chars[i] == '{' {
        var.push('{');
        i += 1;
        while i < chars.len() {
            var.push(chars[i]);
            let close = chars[i] == '}';
            i += 1;
            if close {
                break;
            }
        }
        return (var, i);
    }
    while i < chars.len() && (is_ident_continue(chars[i])) {
        var.push(chars[i]);
        i += 1;
    }
    // A bare `$` (e.g. end of line, or `$(`) — just the sigil.
    (var, i)
}

/// Scan a numeric literal: leading digit already confirmed. Accepts hex/bin/oct
/// prefixes, digit separators, decimals, and exponents — loosely, since we only
/// need to colour, not validate.
fn scan_number(chars: &[char], start: usize) -> (String, usize) {
    let mut num = String::new();
    let mut i = start;
    while i < chars.len() {
        let c = chars[i];
        let ok = match c {
            // A `+`/`-` only continues the literal as an exponent sign, i.e.
            // right after an `e`/`E` (`1e-9`); otherwise it's an operator.
            '+' | '-' => matches!(chars[i - 1], 'e' | 'E'),
            // Hex digits cover 0-9a-fA-F; the rest are prefix/separator/decimal
            // characters we accept loosely (we colour, we don't validate).
            _ => {
                c.is_ascii_hexdigit() || matches!(c, '.' | '_' | 'x' | 'X' | 'o' | 'O' | 'b' | 'B')
            }
        };
        if !ok {
            break;
        }
        num.push(c);
        i += 1;
    }
    (num, i)
}

/// Emit a token wrapped in its themed SGR (or raw for `Plain`), always
/// resetting after.
fn emit(out: &mut String, tok: Tok, text: &str, palette: &crate::theme::SyntaxPalette) {
    match tok.sgr(palette) {
        Some(code) => {
            out.push_str(code);
            out.push_str(text);
            out.push_str("\x1b[0m");
        }
        None => out.push_str(text),
    }
}

/// Does `chars[at..]` begin with the string `pat`?
fn starts_with_at(chars: &[char], at: usize, pat: &str) -> bool {
    let pat: Vec<char> = pat.chars().collect();
    if at + pat.len() > chars.len() {
        return false;
    }
    chars[at..at + pat.len()] == pat[..]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strip ANSI for content assertions (reuse the shared helper).
    fn plain(s: &str) -> String {
        crate::width::strip_ansi(s)
    }

    #[test]
    fn unknown_language_returns_none() {
        assert!(highlight_line("brainfuck", "+++.").is_none());
        assert!(highlight_line("", "anything").is_none());
        assert!(!is_supported("cobol"));
        assert!(is_supported("rust"));
        assert!(is_supported("PY")); // case-insensitive
    }

    #[test]
    fn highlighting_preserves_exact_text_after_stripping() {
        // The colouring must be lossless: strip the ANSI and you get the input
        // back verbatim, byte for byte. This is the single most important
        // invariant — a highlighter that drops or reorders characters is worse
        // than none.
        let cases = [
            ("rust", "let x: u32 = 0x1F; // hi"),
            ("python", "def f(a, b):  # comment\n"),
            ("json", "{\"key\": [1, 2.5, true, null]}"),
            ("bash", "echo \"$HOME/bin\" # path"),
            ("go", "func main() { return `raw` }"),
            ("ts", "const x: Foo = `tmpl ${y}`;"),
        ];
        for (lang, src) in cases {
            let out = highlight_line(lang, src).expect("supported");
            assert_eq!(plain(&out), src, "lossy highlight for {lang}: {out:?}");
        }
    }

    #[test]
    fn keywords_strings_comments_numbers_get_distinct_colors() {
        let out = highlight_line("rust", "let s = \"hi\"; // n=42").expect("supported language");
        assert!(out.contains("\x1b[35mlet\x1b[0m"), "keyword: {out:?}");
        assert!(out.contains("\x1b[32m\"hi\"\x1b[0m"), "string: {out:?}");
        // The comment swallows the rest of the line, including the `42` inside.
        assert!(out.contains("\x1b[90m// n=42\x1b[0m"), "comment: {out:?}");
        // A number OUTSIDE a comment/string is yellow.
        let n = highlight_line("rust", "x = 42").expect("supported language");
        assert!(n.contains("\x1b[33m42\x1b[0m"), "number: {n:?}");
    }

    #[test]
    fn a_non_default_theme_repaints_token_classes() {
        // Drive the internal palette-taking path directly (the process-global
        // `active()` can't be re-set mid-test without poisoning sibling tests),
        // proving that swapping the theme actually changes the emitted SGR: the
        // `light` theme recolours comments and numbers away from dark's codes.
        let spec = lang_spec("rust").expect("rust supported");
        let light = crate::theme::Theme::light().syntax;
        let out = highlight_with(&spec, "let n = 42 // c", &light);
        // Keyword stays magenta (light keeps it), but the number is now blue
        // (light) not yellow (dark), and the comment grey not bright-black.
        assert!(out.contains("\x1b[35mlet\x1b[0m"), "keyword: {out:?}");
        assert!(
            out.contains("\x1b[34m42\x1b[0m"),
            "light number is blue: {out:?}"
        );
        assert!(!out.contains("\x1b[33m42"), "not dark yellow: {out:?}");
        assert!(
            out.contains("\x1b[37m// c\x1b[0m"),
            "light comment is grey: {out:?}"
        );
    }

    #[test]
    fn rust_lifetime_is_not_swallowed_as_a_string() {
        // The classic failure mode: `'static` opening an unterminated char/str
        // literal and eating the rest of the line. `'` is excluded for Rust, so
        // the text must survive intact and the keyword still colours.
        let out = highlight_line("rust", "fn f<'a>(x: &'a str) -> &'static str")
            .expect("supported language");
        assert_eq!(plain(&out), "fn f<'a>(x: &'a str) -> &'static str");
        assert!(out.contains("\x1b[35mfn\x1b[0m"));
    }

    #[test]
    fn capitalized_identifiers_are_types_but_screaming_case_is_not() {
        let out = highlight_line("rust", "let v: Vec<Foo> = MAX").expect("supported language");
        assert!(out.contains("\x1b[36mVec\x1b[0m"), "type Vec: {out:?}");
        assert!(out.contains("\x1b[36mFoo\x1b[0m"), "type Foo: {out:?}");
        // SCREAMING_CASE stays plain (no lowercase letter).
        assert!(!out.contains("\x1b[36mMAX"), "MAX should be plain: {out:?}");
    }

    #[test]
    fn shell_variables_are_highlighted() {
        let out = highlight_line("bash", "echo ${HOME} and $PATH").expect("supported language");
        assert_eq!(plain(&out), "echo ${HOME} and $PATH");
        assert!(
            out.contains("\x1b[36m${HOME}\x1b[0m"),
            "braced var: {out:?}"
        );
        assert!(out.contains("\x1b[36m$PATH\x1b[0m"), "bare var: {out:?}");
        assert!(out.contains("\x1b[35mecho\x1b[0m"), "echo keyword: {out:?}");
    }

    #[test]
    fn escaped_quote_does_not_end_the_string_early() {
        let out = highlight_line("python", "s = \"a\\\"b\" + c").expect("supported language");
        // The whole "a\"b" is one green string; `+ c` is outside it.
        assert!(
            out.contains("\x1b[32m\"a\\\"b\"\x1b[0m"),
            "escaped: {out:?}"
        );
        assert!(plain(&out).ends_with("+ c"));
    }

    #[test]
    fn identifiers_with_trailing_digits_stay_one_token() {
        // `utf8` must not split into ident + number.
        let out = highlight_line("rust", "let utf8 = 1").expect("supported language");
        assert_eq!(plain(&out), "let utf8 = 1");
        assert!(
            !out.contains("\x1b[33m8"),
            "8 must not be a number: {out:?}"
        );
        assert!(
            out.contains("\x1b[33m1\x1b[0m"),
            "the lone 1 is a number: {out:?}"
        );
    }

    #[test]
    fn empty_line_is_empty() {
        assert_eq!(highlight_line("rust", "").expect("supported language"), "");
    }
}
