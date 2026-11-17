use std::ops::Range;

use crate::{
    filediff::DiffLine,
    ui::{CodeHighlighter, UIColor},
};

pub type LineRuns = Vec<(Range<usize>, UIColor)>;

// Tokenizing a moved or generated megabyte file costs seconds, the
// lines past this stay plain like the original's paged prefix.
pub const MAX_LINES: usize = 5000;

/// The 2 texts of a diff, each colored in its own order, so a removed
/// line and the line that replaced it never share a comment or a string.
pub struct DiffHighlighter {
    old: CodeHighlighter,
    new: CodeHighlighter,
}

impl DiffHighlighter {
    /// Both texts start from `code`, a highlighter for the file.
    pub fn new(code: CodeHighlighter) -> Self {
        Self {
            new: code.clone(),
            old: code,
        }
    }

    /// One run list per line, in the order of the lines. A line that
    /// stayed goes into both texts and takes the colors of the new one.
    pub fn highlight<'a>(mut self, lines: impl IntoIterator<Item = &'a DiffLine>) -> Vec<LineRuns> {
        lines
            .into_iter()
            .take(MAX_LINES)
            .map(|line| match line.kind.as_str() {
                "add" => self.new.color_runs(&line.text),
                "rem" => self.old.color_runs(&line.text),
                _ => {
                    self.old.color_runs(&line.text);
                    self.new.color_runs(&line.text)
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use std::ops::Range;

    use super::DiffHighlighter;
    use crate::{
        filediff::DiffLine,
        ui::{CodeHighlighter, MarkdownStyle, UIColor},
    };

    fn line(kind: &str, text: &str) -> DiffLine {
        DiffLine {
            kind:   kind.to_string(),
            text:   text.to_string(),
            old_no: None,
            new_no: None,
        }
    }

    fn colored(runs: &[(Range<usize>, UIColor)], text: &str, part: &str) -> bool {
        let at = text.find(part).unwrap();
        runs.iter().any(|(range, _)| range.contains(&at))
    }

    #[test]
    fn a_removed_comment_does_not_reach_the_new_text() {
        let lines = [
            line("rem", "/* old"),
            line("add", "let n = 2;"),
            line("ctx", "fn main() {}"),
        ];
        let style = MarkdownStyle::DEFAULT;
        let code = CodeHighlighter::for_file_styled("a.rs", &style).unwrap();
        let runs = DiffHighlighter::new(code).highlight(&lines);

        assert_eq!(runs.len(), 3);
        let comment = style.comment;
        assert_ne!(runs[0], []);
        assert!(runs[0].iter().all(|(_, color)| *color == comment));
        assert!(runs[1].iter().all(|(_, color)| *color != comment));
        assert!(colored(&runs[1], "let n = 2;", "let"));
        assert!(colored(&runs[2], "fn main() {}", "fn"));
    }
}
