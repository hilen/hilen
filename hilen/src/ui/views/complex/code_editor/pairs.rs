//! The pairs of a `CodeEditor`, like `(` and `)`. A typed opener puts its
//! closer in after the cursor, a typed closer steps over a closer the
//! editor put in, Backspace right after such an opener takes out the
//! whole pair, an opener typed over a selection wraps it, and Enter
//! between a pair puts the closer on a line of its own. The rules and the
//! lists of pairs are the ones of the Zed editor, its
//! `crates/editor/src/input.rs` and the `config.toml` of each language.

use std::ops::Range;

use super::{
    edit::{LinesChanged, StepKind},
    text::{CodePos, CodeText, Spot, is_word},
};
use crate::ui::CodeScope;

/// 1 pair, like `(` and `)` or `/*` and ` */`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BracketPair {
    pub start:    String,
    pub end:      String,
    /// A typed `start` puts `end` in after the cursor.
    pub close:    bool,
    /// A typed `start` of 1 char wraps a selection in the pair.
    pub surround: bool,
    /// Enter between `start` and `end` puts `end` on a line of its own,
    /// with an indented empty line for the cursor between them.
    pub newline:  bool,
    /// Inside these the pair neither closes nor wraps, like `/*` in a
    /// string. The app says where they are, see `set_scope_runs`.
    pub not_in:   Vec<CodeScope>,
}

impl BracketPair {
    /// A pair that closes and wraps, with no new line between.
    pub fn new(start: &str, end: &str) -> Self {
        Self {
            start:    start.to_string(),
            end:      end.to_string(),
            close:    true,
            surround: true,
            newline:  false,
            not_in:   Vec::new(),
        }
    }

    /// Enter between the pair puts the closer on a line of its own.
    pub fn with_newline(mut self) -> Self {
        self.newline = true;
        self
    }

    /// A typed opener puts in no closer, it still wraps a selection.
    pub fn without_close(mut self) -> Self {
        self.close = false;
        self
    }

    pub fn not_in(mut self, scopes: &[CodeScope]) -> Self {
        self.not_in = scopes.to_vec();
        self
    }

    fn allowed_in(&self, scope: Option<CodeScope>) -> bool {
        scope.is_none_or(|scope| !self.not_in.contains(&scope))
    }
}

/// The pairs of 1 language and the chars a closer may go in front of.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BracketRules {
    pub pairs:        Vec<BracketPair>,
    /// A pair closes only in front of a space, at the end of a line or in
    /// front of one of these chars, so `(` typed in front of a name puts
    /// in only `(`.
    pub close_before: String,
}

impl BracketRules {
    /// No pairs, the editor puts in only what is typed. The default of a
    /// new editor.
    pub fn none() -> Self {
        Self::default()
    }

    /// The rules Zed has for a file by its name or its path: Rust, TOML
    /// and Markdown have their own, every other file has the ones of
    /// plain text.
    pub fn for_file(file_name: &str) -> Self {
        let name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
        let extension = name.rsplit_once('.').map(|(_, extension)| extension);
        match (name, extension) {
            (_, Some("rs")) => Self::rust(),
            ("Cargo.lock" | "Pipfile" | "uv.lock", _) | (_, Some("toml")) => Self::toml(),
            (_, Some("md" | "mdx" | "mdwn" | "mdc" | "markdown" | "MD")) => Self::markdown(),
            _ => Self::plain_text(),
        }
    }

    pub fn rust() -> Self {
        let code = [CodeScope::String, CodeScope::Comment];
        Self {
            pairs:        vec![
                BracketPair::new("{", "}").with_newline(),
                BracketPair::new("r#\"", "\"#").with_newline().not_in(&code),
                BracketPair::new("r##\"", "\"##").with_newline().not_in(&code),
                BracketPair::new("r###\"", "\"###").with_newline().not_in(&code),
                BracketPair::new("[", "]").with_newline(),
                BracketPair::new("(", ")").with_newline(),
                BracketPair::new("<", ">").without_close().with_newline().not_in(&code),
                BracketPair::new("\"", "\"").not_in(&[CodeScope::String]),
                BracketPair::new("`", "`").without_close().not_in(&[CodeScope::String]),
                BracketPair::new("/*", " */").not_in(&code),
            ],
            close_before: ";:.,=}])>".to_string(),
        }
    }

    pub fn toml() -> Self {
        let code = [CodeScope::Comment, CodeScope::String];
        Self {
            pairs:        vec![
                BracketPair::new("{", "}").with_newline(),
                BracketPair::new("[", "]").with_newline(),
                BracketPair::new("\"", "\"").not_in(&code),
                BracketPair::new("'", "'").not_in(&code),
            ],
            close_before: ",]}".to_string(),
        }
    }

    pub fn markdown() -> Self {
        Self {
            pairs:        vec![
                BracketPair::new("{", "}").with_newline(),
                BracketPair::new("[", "]").with_newline(),
                BracketPair::new("(", ")").with_newline(),
                BracketPair::new("<", ">").with_newline(),
                BracketPair::new("\"", "\"").without_close(),
                BracketPair::new("'", "'").without_close(),
                BracketPair::new("`", "`").without_close(),
                BracketPair::new("*", "*").without_close(),
                BracketPair::new("~", "~").without_close(),
            ],
            close_before: ";:.,=}])>".to_string(),
        }
    }

    pub fn plain_text() -> Self {
        Self {
            pairs:        ["()", "[]", "{}", "\"\"", "''"]
                .iter()
                .map(|pair| BracketPair::new(&pair[..1], &pair[1..]))
                .collect(),
            close_before: ")]}".to_string(),
        }
    }

    /// A closer may go in front of `next`, `None` at the end of a line.
    fn closes_before(&self, next: Option<char>) -> bool {
        next.is_none_or(|ch| ch.is_whitespace() || self.close_before.contains(ch))
    }
}

/// What lies between an opener and the closer the editor put in after
/// it. The start stays right after the opener, the end moves with what is
/// typed at it and stays right in front of the closer. It lives while
/// the cursor is in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PairRegion {
    start: CodePos,
    end:   CodePos,
    pair:  BracketPair,
}

/// What a typed char did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Typed {
    Changed(LinesChanged),
    /// It was the closer the editor put in, the cursor stepped over it.
    Moved,
}

/// The scope of a place of a line: inside a run, not at its start.
/// `scopes` are the runs of `set_scope_runs`, a run over the line break
/// ends 1 byte past the line.
fn scope_at(scopes: &[(Range<usize>, CodeScope)], byte: usize) -> Option<CodeScope> {
    scopes
        .iter()
        .find(|(range, _)| range.start < byte && byte < range.end)
        .map(|(_, scope)| *scope)
}

/// Where a place goes when the text from `from` to `to` gives way to a
/// text that ends at `end`. A place in the replaced text, or at its start
/// when nothing was taken out, goes to `from` with `right` off and to
/// `end` with it on.
pub(super) fn moved(pos: CodePos, from: CodePos, to: CodePos, end: CodePos, right: bool) -> CodePos {
    if pos < from || (pos == from && !right) {
        pos
    } else if pos < to || (pos == to && from == to) {
        if right { end } else { from }
    } else if pos.line == to.line {
        CodePos::new(end.line, end.byte + pos.byte - to.byte)
    } else {
        CodePos::new(pos.line - to.line + end.line, pos.byte)
    }
}

impl CodeText {
    /// The regions follow a change from `from` to `to` that put in a text
    /// ending at `end`.
    pub(super) fn follow_regions(&mut self, from: CodePos, to: CodePos, end: CodePos) {
        for region in &mut self.regions {
            region.start = moved(region.start, from, to, end, false);
            region.end = moved(region.end, from, to, end, true);
        }
    }

    /// Drops the regions the cursor and the selection left.
    pub(crate) fn drop_far_regions(&mut self) {
        let (from, to) = self.selection().unwrap_or((self.cursor, self.cursor));
        self.regions.retain(|region| region.start <= to && from <= region.end);
    }

    /// The innermost region the cursor is in.
    fn region_at(&self, pos: CodePos) -> Option<&PairRegion> {
        self.regions
            .iter()
            .rev()
            .find(|region| region.start <= pos && pos <= region.end)
    }

    /// A typed char with the pairs of `rules`. `scopes` are the strings and
    /// comments of the line of the cursor.
    pub(crate) fn type_paired(
        &mut self,
        ch: char,
        rules: &BracketRules,
        scopes: &[(Range<usize>, CodeScope)],
    ) -> Typed {
        let text = ch.to_string();
        let cursor = self.cursor;
        let selection = self.selection();
        let (from, to) = selection.unwrap_or((cursor, cursor));
        let scope = scope_at(scopes, cursor.byte);
        let before = &self.lines[from.line][..from.byte];

        let mut found: Option<(&BracketPair, bool)> = None;
        let mut ends_with = None;
        for pair in &rules.pairs {
            if !pair.close && !pair.surround {
                continue;
            }
            if pair.allowed_in(scope) && pair.start.ends_with(&text) {
                let prefix = &pair.start[..pair.start.len() - text.len()];
                if before.ends_with(prefix) {
                    found = Some((pair, true));
                    break;
                }
            }
            if pair.end == text && ends_with.is_none() {
                ends_with = Some(pair);
            }
        }
        let Some((pair, is_start)) = found.or(ends_with.map(|pair| (pair, false))) else {
            return Typed::Changed(self.type_char(ch));
        };

        if selection.is_none() {
            if is_start && pair.close && self.closes_here(pair, rules, scopes) {
                let pair = pair.clone();
                let inside = CodePos::new(cursor.line, cursor.byte + text.len());
                let changed = self.replace_to(
                    cursor,
                    cursor,
                    &format!("{text}{}", pair.end),
                    StepKind::Other,
                    Some(inside),
                );
                let at = self.regions.partition_point(|region| region.start <= inside);
                self.regions.insert(
                    at,
                    PairRegion {
                        start: inside,
                        end: inside,
                        pair,
                    },
                );
                return Typed::Changed(changed);
            }
            let steps_over = self.region_at(cursor).is_some_and(|region| {
                cursor == region.end
                    && text == region.pair.end
                    && self.lines[cursor.line][cursor.byte..].starts_with(&text)
            });
            if steps_over {
                self.move_to(CodePos::new(cursor.line, cursor.byte + text.len()), false);
                return Typed::Moved;
            }
        } else if pair.surround && is_start && pair.start.chars().count() == 1 {
            let pair = pair.clone();
            return Typed::Changed(self.wrap(from, to, &pair));
        }
        Typed::Changed(self.type_char(ch))
    }

    /// The opener of `pair` just typed at the cursor gets its closer: the
    /// next char lets a closer in, a quote does not follow a letter, and a
    /// quote is no closing quote of the line.
    fn closes_here(
        &self,
        pair: &BracketPair,
        rules: &BracketRules,
        scopes: &[(Range<usize>, CodeScope)],
    ) -> bool {
        let cursor = self.cursor;
        let line = &self.lines[cursor.line];
        let before = &line[..cursor.byte];
        if !rules.closes_before(line[cursor.byte..].chars().next()) {
            return false;
        }
        let same = pair.start == pair.end;
        if same && before.chars().next_back().is_some_and(is_word) {
            return false;
        }
        let mut quote = pair.start.chars();
        let (Some(quote), None) = (quote.next(), quote.next()) else {
            return true;
        };
        if !same {
            return true;
        }
        // An odd count of quotes before the cursor makes this one the
        // closing quote. A quote that opens a string, where the pair is
        // not allowed right after it, does not count.
        let count = before
            .char_indices()
            .filter(|(byte, ch)| {
                *ch == quote
                    && pair.allowed_in(scope_at(scopes, *byte))
                    && pair.allowed_in(scope_at(scopes, byte + ch.len_utf8()))
            })
            .count();
        count % 2 == 0
    }

    /// Puts `pair` around the text from `from` to `to`, which stays
    /// selected, with the cursor at the end it had.
    fn wrap(&mut self, from: CodePos, to: CodePos, pair: &BracketPair) -> LinesChanged {
        let inner = self.selected_text();
        let cursor_at_end = self.cursor == to;
        let changed = self.replace_to(
            from,
            to,
            &format!("{}{inner}{}", pair.start, pair.end),
            StepKind::Other,
            None,
        );
        let start = CodePos::new(from.line, from.byte + pair.start.len());
        let end = if to.line == from.line {
            CodePos::new(to.line, to.byte + pair.start.len())
        } else {
            to
        };
        let (cursor, anchor) = if cursor_at_end { (end, start) } else { (start, end) };
        self.set_spot(Spot {
            cursor,
            anchor: Some(anchor),
        });
        changed
    }

    /// Backspace right after the opener of a pair the editor closed takes
    /// out the opener, what is between and the closer. `None` anywhere
    /// else.
    pub(crate) fn delete_pair(&mut self) -> Option<LinesChanged> {
        if self.selection().is_some() {
            return None;
        }
        let cursor = self.cursor;
        let region = self.region_at(cursor)?;
        let (start, end) = (region.pair.start.len(), region.pair.end.len());
        if cursor != region.start || region.start.byte < start {
            return None;
        }
        let open = CodePos::new(region.start.line, region.start.byte - start);
        let close = region.end;
        let has_open = self.lines[open.line][open.byte..].starts_with(&region.pair.start);
        let has_close = self.lines[close.line][close.byte..].starts_with(&region.pair.end);
        if !has_open || !has_close {
            return None;
        }
        let close_end = CodePos::new(close.line, close.byte + end);
        Some(self.replace_to(open, close_end, "", StepKind::Deleting, None))
    }

    /// Enter. Between the opener and the closer of a pair with `newline`,
    /// spaces around the cursor left out, the closer goes to a line of its
    /// own with the indent of the line, and the cursor to an empty line
    /// between, 1 level deeper. Anywhere else a plain `new_line`.
    pub(crate) fn new_line_paired(
        &mut self,
        rules: &BracketRules,
        scopes: &[(Range<usize>, CodeScope)],
    ) -> LinesChanged {
        let (from, to) = self.selection().unwrap_or((self.cursor, self.cursor));
        let first = &self.lines[from.line];
        let before = first[..from.byte].trim_end_matches([' ', '\t']);
        let rest = &self.lines[to.line][to.byte..];
        let after = rest.trim_start_matches([' ', '\t']);
        let scope = scope_at(scopes, from.byte);
        let between = rules.pairs.iter().any(|pair| {
            pair.newline
                && pair.allowed_in(scope)
                && before.ends_with(pair.start.trim_end())
                && after.starts_with(pair.end.trim_start())
        });
        if !between {
            return self.new_line();
        }
        let indent = first.len() - first.trim_start().len();
        let outer = " ".repeat(indent.min(before.len()));
        let inner = " ".repeat(indent.min(before.len()) + self.tab_width);
        let start = CodePos::new(from.line, before.len());
        let end = CodePos::new(to.line, to.byte + rest.len() - after.len());
        let cursor = CodePos::new(from.line + 1, inner.len());
        self.replace_to(
            start,
            end,
            &format!("\n{inner}\n{outer}"),
            StepKind::Other,
            Some(cursor),
        )
    }
}
