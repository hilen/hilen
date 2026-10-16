//! The find and replace calls of a `CodeEditor` and what it does for the
//! commands of its find bar.

use super::{
    find::{FindError, FindOptions, Finder},
    find_bar::{FindBar, FindCommand},
    text::CodePos,
    view::CodeEditor,
};
use crate::{
    deps::refs::Weak,
    ui::{ViewData, ViewSubviews},
};

impl CodeEditor {
    /// The places of `query` in the text, the start and the end of each,
    /// in the order of the text. A match never goes over a line break, an
    /// empty query finds nothing. The error is a regular expression that
    /// does not parse.
    pub fn find(&self, query: &str, options: FindOptions) -> Result<Vec<(CodePos, CodePos)>, FindError> {
        Ok(Finder::new(query, options)?.map_or_else(Vec::new, |finder| finder.matches(self.text.lines(), 0)))
    }

    /// Opens the find bar over the top of the editor with `query`, marks
    /// every match and gives the keys to the field of the query. The
    /// first match from the cursor is the current one.
    pub fn show_find(&mut self, query: &str) {
        self.open_find(query);
        self.find_bar.focus_query();
    }

    /// Closes the find bar. No match is marked after it.
    pub fn close_find(&mut self) {
        if !self.find.open {
            return;
        }
        self.find.open = false;
        self.find.close();
        self.find_bar.drop_keys();
        self.find_bar.set_hidden(true);
        self.relayout();
    }

    pub fn is_find_open(&self) -> bool {
        self.find.open
    }

    /// The matches the bar marks, in the order of the text. None while
    /// the bar is closed.
    pub fn find_matches(&self) -> &[(CodePos, CodePos)] {
        self.find.matches()
    }

    /// The index of the current match in `find_matches`, the one the bar
    /// counts as "3 of 12" and Replace takes.
    pub fn current_match(&self) -> Option<usize> {
        self.find.current()
    }

    pub fn find_options(&self) -> FindOptions {
        self.find.options
    }

    /// The options of the find bar, its toggles show them. An open bar
    /// searches again.
    pub fn set_find_options(&mut self, options: FindOptions) {
        self.find.options = options;
        self.find_bar.show_options(options);
        if self.find.open {
            self.search();
        }
    }

    /// The next match is current, after the last comes the first. The
    /// editor scrolls to it. While the editor has the keys the match is
    /// also selected, so typing goes on from it.
    pub fn find_next(&mut self) {
        self.step(true);
    }

    pub fn find_previous(&mut self) {
        self.step(false);
    }
}

impl CodeEditor {
    /// Makes the bar after the touch of the editor is on, see `FindBar`.
    pub(super) fn setup_find(mut self: Weak<Self>) {
        self.find_bar = self.add_view::<FindBar>();
        self.find_bar.set_hidden(true);
        self.find_bar.command.val(move |command| self.find_command(command));
    }

    fn find_command(mut self: Weak<Self>, command: FindCommand) {
        match command {
            FindCommand::Query(query) => {
                self.find.query = query;
                self.search();
            }
            FindCommand::Options(options) => self.set_find_options(options),
            FindCommand::Next => self.find_next(),
            FindCommand::Previous => self.find_previous(),
            FindCommand::ShowReplace => self.show_replace(),
            FindCommand::Replace(with) => self.replace_current(&with),
            FindCommand::ReplaceAll(with) => self.replace_all(&with),
            FindCommand::Close => self.leave_find(),
        }
    }

    /// Cmd F, or Cmd Alt F with `replacing`, while the editor has the
    /// keys. The selected text is the query when it is on 1 line, else the
    /// query of the last find stays.
    pub(super) fn find_from_keys(mut self: Weak<Self>, replacing: bool) {
        let query = match self.text.selection() {
            Some((start, end)) if start.line == end.line => {
                // The match of the selection is the current one, drawn as
                // a match and not covered by the selection.
                let query = self.text.selected_text();
                self.text.move_to(start, false);
                query
            }
            _ => self.find.query.clone(),
        };
        self.open_find(&query);
        if replacing {
            self.show_replace();
        } else {
            self.find_bar.focus_query();
        }
    }

    /// Opens the bar with `query` and searches.
    fn open_find(&mut self, query: &str) {
        if !self.find.open {
            self.find.open = true;
            self.find_bar.set_hidden(false);
            self.relayout();
        }
        self.find_bar.set_query(query);
        self.find.query = query.to_string();
        self.search();
    }

    /// Shows the replace row and gives it the keys, or the field of the
    /// query while it is empty.
    fn show_replace(&mut self) {
        if !self.find_bar.is_replacing() {
            self.find_bar.set_replacing(true);
            self.relayout();
        }
        if self.find.query.is_empty() {
            self.find_bar.focus_query();
        } else {
            self.find_bar.focus_replacement();
        }
    }

    /// Searches the whole text again, for a new query, new options or a
    /// new text. The first match from the cursor is current.
    pub(super) fn search(&mut self) {
        self.find.search(self.text.lines());
        if let Some(err) = self.find.error() {
            log::debug!("Find in a code editor: {:?} is no regex: {err}", self.find.query);
        }
        log::debug!(
            "Find in a code editor: {:?}, {} matches",
            self.find.query,
            self.find.matches().len()
        );
        self.find.settle(self.text.cursor());
        self.show_current();
    }

    /// The text changed under an open bar, its matches followed it. The
    /// first match from the cursor is current.
    pub(super) fn matches_followed(&mut self) {
        self.find.settle(self.text.cursor());
        self.find_bar.show_status(&self.find.status());
        self.refresh();
    }

    /// The bar counts the current match, the editor scrolls to it and the
    /// rows draw the matches again.
    fn show_current(&mut self) {
        self.find_bar.show_status(&self.find.status());
        if let Some((from, to)) = self.find.current_range() {
            self.reveal(from, to);
        }
        self.refresh();
    }

    fn step(&mut self, forward: bool) {
        if !self.find.open {
            return;
        }
        if let Some((from, to)) = self.find.step(forward)
            && self.editing
        {
            self.text.select(from, to);
            self.cursor_changed();
        }
        self.show_current();
    }

    /// Replace: the current match gives way to `with`, as 1 step of the
    /// undo history, and the next match is current.
    fn replace_current(&mut self, with: &str) {
        let Some((from, to, text)) = self.find.current_replacement(self.text.lines(), with) else {
            return;
        };
        let changed = self.text.replace_range(from, to, &text);
        self.lines_changed(changed);
        self.show_current();
    }

    /// Replace All: every match gives way to `with`, all as 1 step of the
    /// undo history.
    fn replace_all(&mut self, with: &str) {
        let edits = self.find.replacements(self.text.lines(), with);
        if let Some(changed) = self.text.replace_all(&edits) {
            log::debug!("Replaced {} matches in a code editor", edits.len());
            self.lines_changed(changed);
            self.show_current();
        }
    }

    /// Escape or the close button: the bar goes, the current match is
    /// selected and the editor has the keys again.
    fn leave_find(mut self: Weak<Self>) {
        let current = self.find.current_range();
        self.close_find();
        if let Some((from, to)) = current {
            self.text.select(from, to);
            self.cursor_changed();
        }
        self.focus();
    }
}
