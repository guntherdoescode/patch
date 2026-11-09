//! Language-aware syntax highlighting backed by Syntect.

use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::util::LinesWithEndings;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub text: String,
    pub foreground: String,
}

fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn themes() -> &'static ThemeSet {
    static THEMES: OnceLock<ThemeSet> = OnceLock::new();
    THEMES.get_or_init(ThemeSet::load_defaults)
}

fn find_syntax<'a>(set: &'a SyntaxSet, language: &str) -> Option<&'a SyntaxReference> {
    let language = language.trim();
    if language.is_empty() {
        return None;
    }
    set.find_syntax_by_token(language)
        .or_else(|| set.find_syntax_by_extension(language))
}

/// Highlight a fenced code block. Unknown or absent languages return None so
/// callers can retain their normal monospaced fallback.
pub fn highlight_code(language: &str, source: &str) -> Option<Vec<StyledSpan>> {
    let syntax_set = syntaxes();
    let syntax = find_syntax(syntax_set, language)?;
    let theme_set = themes();
    let theme = theme_set
        .themes
        .get("base16-ocean.dark")
        .or_else(|| theme_set.themes.values().next())?;

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut spans = Vec::new();

    for line in LinesWithEndings::from(source) {
        let ranges = highlighter.highlight_line(line, syntax_set).ok()?;
        for (style, text) in ranges {
            if text.is_empty() {
                continue;
            }
            let color = style.foreground;
            spans.push(StyledSpan {
                text: text.to_owned(),
                foreground: format!("#{:02x}{:02x}{:02x}", color.r, color.g, color.b),
            });
        }
    }

    Some(spans)
}

#[cfg(test)]
mod tests {
    use super::highlight_code;

    #[test]
    fn highlights_rust_source() {
        let spans = highlight_code("rust", "fn main() { let answer = 42; }\n")
            .expect("Rust grammar should be available");
        assert!(!spans.is_empty());
        assert!(spans.iter().any(|span| span.text.contains("fn")));
    }

    #[test]
    fn unknown_language_uses_fallback() {
        assert!(highlight_code("not-a-real-language", "some text").is_none());
    }
}
