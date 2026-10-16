//! Find in the text of a `CodeEditor`: a query matched on the lines, the
//! matches that follow the edits and the current match. No view, so every
//! rule has a unit test. The bar is in `find_bar.rs`, the calls of the
//! editor in `finding.rs`.

use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    ops::Range,
};

use regex::{Regex, RegexBuilder};

use super::{
    edit::LinesChanged,
    text::{CodePos, is_word},
};

/// How the query of `CodeEditor::find` matches. All off by default: the
/// case does not count, a match can be a part of a word, and the query
/// is plain text.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FindOptions {
    /// Upper and lower case count, `Size` no longer finds `size`.
    pub case_sensitive: bool,
    /// A match does not start or end inside a word: a letter, digit or
    /// `_` at its edge has none of them next to it outside.
    pub whole_word:     bool,
    /// The query is a regular expression of the `regex` crate, and a
    /// replacement can name its groups, `$1` or `${name}`.
    pub regex:          bool,
}

/// A query that is not a valid regular expression, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FindError(String);

impl Display for FindError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for FindError {}

/// A query made ready to match lines.
#[derive(Debug)]
pub(crate) struct Finder {
    regex:      Regex,
    whole_word: bool,
    /// A replacement fills in the groups of the match.
    expands:    bool,
}

impl Finder {
    /// `None` for an empty query, which matches nothing.
    pub(crate) fn new(query: &str, options: FindOptions) -> Result<Option<Self>, FindError> {
        if query.is_empty() {
            return Ok(None);
        }
        let pattern = if options.regex {
            query.to_string()
        } else {
            regex::escape(query)
        };
        let regex = RegexBuilder::new(&pattern)
            .case_insensitive(!options.case_sensitive)
            .build()
            .map_err(|err| FindError(err.to_string()))?;
        Ok(Some(Self {
            regex,
            whole_word: options.whole_word,
            expands: options.regex,
        }))
    }

    /// The byte ranges of the matches on `line`. An empty match, of a
    /// regular expression like `a*`, is left out.
    pub(crate) fn line_matches(&self, line: &str) -> Vec<Range<usize>> {
        self.regex
            .find_iter(line)
            .map(|found| found.range())
            .filter(|range| !range.is_empty() && (!self.whole_word || stands_alone(line, range)))
            .collect()
    }

    /// The matches of `lines`, the first of them is line `first` of the
    /// text.
    pub(crate) fn matches(&self, lines: &[String], first: usize) -> Vec<(CodePos, CodePos)> {
        lines
            .iter()
            .enumerate()
            .flat_map(|(index, line)| {
                let line_index = first + index;
                self.line_matches(line).into_iter().map(move |range| {
                    (
                        CodePos::new(line_index, range.start),
                        CodePos::new(line_index, range.end),
                    )
                })
            })
            .collect()
    }

    /// What the match at `range` of `line` gives way to: `with` as it is,
    /// or with the groups of a regular expression filled in.
    pub(crate) fn replacement(&self, line: &str, range: Range<usize>, with: &str) -> String {
        if !self.expands {
            return with.to_string();
        }
        match self.regex.captures_at(line, range.start) {
            Some(groups) if groups.get(0).is_some_and(|found| found.range() == range) => {
                let mut replaced = String::new();
                groups.expand(with, &mut replaced);
                replaced
            }
            _ => with.to_string(),
        }
    }
}

/// `range` of `line` does not start or end inside a word. An edge is
/// inside a word when the chars on both sides of it are chars of a word.
fn stands_alone(line: &str, range: &Range<usize>) -> bool {
    let joins = |outside: Option<char>, inside: Option<char>| {
        outside.is_some_and(is_word) && inside.is_some_and(is_word)
    };
    let text = &line[range.clone()];
    !joins(line[..range.start].chars().next_back(), text.chars().next())
        && !joins(line[range.end..].chars().next(), text.chars().next_back())
}

/// The find of an editor: the query and its options, every match in the
/// order of the text, and the current one.
#[derive(Debug, Default)]
pub(crate) struct FindState {
    /// The find bar is open.
    pub(crate) open:    bool,
    pub(crate) query:   String,
    pub(crate) options: FindOptions,
    /// `None` while the bar is closed, for an empty query and for a
    /// query that is no valid regular expression.
    finder:             Option<Finder>,
    error:              Option<FindError>,
    matches:            Vec<(CodePos, CodePos)>,
    current:            Option<usize>,
}

impl FindState {
    /// Searches every line again, for a new query or new options. No
    /// match is current after it.
    pub(crate) fn search(&mut self, lines: &[String]) {
        self.matches.clear();
        self.current = None;
        match Finder::new(&self.query, self.options) {
            Ok(finder) => {
                self.finder = finder;
                self.error = None;
            }
            Err(err) => {
                self.finder = None;
                self.error = Some(err);
            }
        }
        if let Some(finder) = &self.finder {
            self.matches = finder.matches(lines, 0);
        }
    }

    /// The bar closed: no match is marked until the next search. The
    /// query and the options stay for the next open.
    pub(crate) fn close(&mut self) {
        self.finder = None;
        self.error = None;
        self.matches.clear();
        self.current = None;
    }

    /// The lines of `changed` gave way to new ones. The matches of the
    /// old lines go, the new lines are searched, and the matches after
    /// them move along by the count of lines that came or went. Call
    /// `settle` after it, the current match is then unknown.
    pub(crate) fn lines_changed(&mut self, lines: &[String], changed: LinesChanged) {
        let Some(finder) = &self.finder else {
            return;
        };
        let LinesChanged {
            start,
            removed,
            inserted,
        } = changed;
        let first = self.matches.partition_point(|(from, _)| from.line < start);
        let after = self.matches.partition_point(|(from, _)| from.line < start + removed);
        let moved = |pos: CodePos| CodePos::new(pos.line - removed + inserted, pos.byte);
        let tail: Vec<_> = self.matches.drain(after..).map(|(from, to)| (moved(from), moved(to))).collect();
        self.matches.truncate(first);
        self.matches.extend(finder.matches(&lines[start..start + inserted], start));
        self.matches.extend(tail);
        self.current = None;
    }

    /// The first match at or after `pos` is current, the first of the
    /// text when none is after it.
    pub(crate) fn settle(&mut self, pos: CodePos) {
        if self.matches.is_empty() {
            self.current = None;
            return;
        }
        let index = self.matches.partition_point(|(from, _)| *from < pos);
        self.current = Some(if index < self.matches.len() { index } else { 0 });
    }

    /// Makes the next match current, or the one before, and gives it.
    /// After the last match comes the first one.
    pub(crate) fn step(&mut self, forward: bool) -> Option<(CodePos, CodePos)> {
        let count = self.matches.len();
        if count == 0 {
            self.current = None;
            return None;
        }
        let index = match (self.current, forward) {
            (Some(index), true) => (index + 1) % count,
            (Some(index), false) => (index + count - 1) % count,
            (None, true) => 0,
            (None, false) => count - 1,
        };
        self.current = Some(index);
        Some(self.matches[index])
    }

    pub(crate) fn matches(&self) -> &[(CodePos, CodePos)] {
        &self.matches
    }

    pub(crate) fn current(&self) -> Option<usize> {
        self.current
    }

    pub(crate) fn current_range(&self) -> Option<(CodePos, CodePos)> {
        self.matches.get(self.current?).copied()
    }

    /// The matches on `line` as byte ranges, each with whether it is the
    /// current one.
    pub(crate) fn on_line(&self, line: usize) -> Vec<(Range<usize>, bool)> {
        let first = self.matches.partition_point(|(from, _)| from.line < line);
        self.matches[first..]
            .iter()
            .take_while(|(from, _)| from.line == line)
            .enumerate()
            .map(|(offset, (from, to))| (from.byte..to.byte, self.current == Some(first + offset)))
            .collect()
    }

    /// What the bar says: "3 of 12", "No results", the reason of a bad
    /// regular expression, or nothing for an empty query.
    pub(crate) fn status(&self) -> String {
        if self.error.is_some() {
            return "Bad regex".to_string();
        }
        if self.query.is_empty() {
            return String::new();
        }
        let count = self.matches.len();
        match self.current {
            _ if count == 0 => "No results".to_string(),
            Some(index) => format!("{} of {count}", index + 1),
            None => format!("{count} found"),
        }
    }

    pub(crate) fn error(&self) -> Option<&FindError> {
        self.error.as_ref()
    }

    /// The current match and the text it gives way to.
    pub(crate) fn current_replacement(
        &self,
        lines: &[String],
        with: &str,
    ) -> Option<(CodePos, CodePos, String)> {
        let (from, to) = self.current_range()?;
        let finder = self.finder.as_ref()?;
        let text = finder.replacement(&lines[from.line], from.byte..to.byte, with);
        Some((from, to, text))
    }

    /// Every match and the text it gives way to, in the order of the text.
    pub(crate) fn replacements(&self, lines: &[String], with: &str) -> Vec<(CodePos, CodePos, String)> {
        let Some(finder) = &self.finder else {
            return Vec::new();
        };
        self.matches
            .iter()
            .map(|&(from, to)| {
                (
                    from,
                    to,
                    finder.replacement(&lines[from.line], from.byte..to.byte, with),
                )
            })
            .collect()
    }
}
