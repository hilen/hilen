use std::{ops::Range, sync::OnceLock};

use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    parsing::{SyntaxReference, SyntaxSet},
};

use crate::ui::Color;

// The syntax highlighting from highlight.ts, shiki tokens replaced by
// syntect over the bundled Sublime defaults. Runs on a background
// thread, the output is one list of color runs per diff line, applied
// to the content labels through set_color_runs.

pub type LineRuns = Vec<(Range<usize>, Color)>;

// Tokenizing a moved or generated megabyte file costs seconds, the
// lines past this stay plain like the original's paged prefix.
pub const MAX_LINES: usize = 5000;

fn syntaxes() -> &'static SyntaxSet {
    static SET: OnceLock<SyntaxSet> = OnceLock::new();
    SET.get_or_init(SyntaxSet::load_defaults_newlines)
}

fn themes() -> &'static ThemeSet {
    static SET: OnceLock<ThemeSet> = OnceLock::new();
    SET.get_or_init(ThemeSet::load_defaults)
}

fn basename(file: &str) -> &str {
    file.rsplit(['/', '\\']).next().unwrap_or(file)
}

fn syntax_for(file: &str) -> Option<&'static SyntaxReference> {
    let set = syntaxes();
    let name = basename(file);
    if let Some(syntax) = set.find_syntax_by_extension(name) {
        return Some(syntax);
    }
    let ext = name.rsplit_once('.').map(|(_, e)| e)?;
    set.find_syntax_by_extension(ext)
}

// The bundled themes closest to the original's github-dark and
// github-light pair.
fn theme_name(dark: bool) -> &'static str {
    if dark {
        "base16-ocean.dark"
    } else {
        "InspiredGitHub"
    }
}

// One run list per input line, None when the file has no known syntax.
// The lines are highlighted in order as one document, so multi line
// constructs carry across diff lines the way they do in the original.
pub fn highlight(file: &str, lines: &[String], dark: bool) -> Option<Vec<LineRuns>> {
    let syntax = syntax_for(file)?;
    let theme = themes().themes.get(theme_name(dark))?;
    let default_fg = theme.settings.foreground;
    let mut highlighter = HighlightLines::new(syntax, theme);

    let mut result = Vec::with_capacity(lines.len());
    for line in lines.iter().take(MAX_LINES) {
        let with_newline = format!("{line}\n");
        let Ok(spans) = highlighter.highlight_line(&with_newline, syntaxes()) else {
            result.push(Vec::new());
            continue;
        };
        let mut runs: LineRuns = Vec::new();
        let mut offset = 0;
        for (style, text) in spans {
            let start = offset;
            offset += text.len();
            let end = offset.min(line.len());
            if start >= end || Some(style.foreground) == default_fg {
                continue;
            }
            let fg = style.foreground;
            runs.push((
                start..end,
                Color::rgba(
                    f32::from(fg.r) / 255.0,
                    f32::from(fg.g) / 255.0,
                    f32::from(fg.b) / 255.0,
                    f32::from(fg.a) / 255.0,
                ),
            ));
        }
        result.push(runs);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::highlight;

    #[test]
    fn highlights_rust() {
        let lines = vec!["fn main() {".to_string(), "    let x = 1;".to_string()];
        let runs = highlight("src/main.rs", &lines, true).expect("rust is a bundled syntax");
        assert_eq!(runs.len(), 2);
        assert!(!runs[0].is_empty(), "fn keyword should get a color run");
    }

    #[test]
    fn unknown_extension_is_plain() {
        let lines = vec!["whatever".to_string()];
        assert!(highlight("notes.zzz", &lines, true).is_none());
    }

    #[test]
    fn runs_stay_inside_the_line() {
        let lines = vec!["let s = \"x\";".to_string()];
        let runs = highlight("a.rs", &lines, false).expect("rust syntax");
        for (range, _) in &runs[0] {
            assert!(range.end <= lines[0].len());
        }
    }
}
