//! What an app puts on the lines of a `CodeEditor` on top of the colors of
//! the code: a mark of a kind, lines under parts of the code and colors
//! behind them. And the switch that keeps the user from changing the text.

use std::ops::Range;

use super::view::CodeEditor;
use crate::{
    deps::refs::weak_from_ref,
    ui::{ToLabel, UIColor},
};

/// How bad the thing is that a mark tells of. Each kind has its 2 colors
/// in `CodeEditorStyle`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MarkKind {
    #[default]
    Error,
    Warning,
    Note,
}

impl CodeEditor {
    /// Marks `line` like `set_mark`, in the colors of `kind`. A warning
    /// looks other than an error.
    pub fn set_mark_kind(&self, line: usize, kind: MarkKind, message: impl ToLabel) {
        let mut this = weak_from_ref(self);
        if let Some(decor) = this.decor.get_mut(line) {
            decor.mark = Some((message.to_label(), kind));
            this.refresh();
        }
    }

    /// The kind of the mark on `line`.
    pub fn mark_kind(&self, line: usize) -> Option<MarkKind> {
        self.decor.get(line)?.mark.as_ref().map(|(_, kind)| *kind)
    }

    /// Draws a line under parts of the code on `line`, byte ranges of that
    /// line with their colors, like the exact place of an error. A range
    /// with no width shows as 1 char. A change of the line by the user
    /// drops them, like its colors.
    pub fn set_underlines(&self, line: usize, runs: impl IntoIterator<Item = (Range<usize>, UIColor)>) {
        let mut this = weak_from_ref(self);
        if let Some(decor) = this.decor.get_mut(line) {
            decor.underlines = runs.into_iter().collect();
            this.refresh();
        }
    }

    pub fn underlines(&self, line: usize) -> &[(Range<usize>, UIColor)] {
        self.decor.get(line).map_or(&[], |decor| decor.underlines.as_slice())
    }

    /// Takes the underlines off every line.
    pub fn clear_underlines(&mut self) {
        for decor in &mut self.decor {
            decor.underlines.clear();
        }
        self.refresh();
    }

    /// Paints a color behind parts of the code on `line`, byte ranges of
    /// that line, like the other uses of the name under the cursor. A
    /// match of the find bar is painted over them. A change of the line by
    /// the user drops them, like its colors.
    pub fn set_background_runs(&self, line: usize, runs: impl IntoIterator<Item = (Range<usize>, UIColor)>) {
        let mut this = weak_from_ref(self);
        if let Some(decor) = this.decor.get_mut(line) {
            decor.backgrounds = runs.into_iter().collect();
            this.refresh();
        }
    }

    pub fn background_runs(&self, line: usize) -> &[(Range<usize>, UIColor)] {
        self.decor.get(line).map_or(&[], |decor| decor.backgrounds.as_slice())
    }

    /// Takes the colors behind the code off every line.
    pub fn clear_background_runs(&mut self) {
        for decor in &mut self.decor {
            decor.backgrounds.clear();
        }
        self.refresh();
    }

    /// With `true` the user can not change the text: typing, Enter, Tab,
    /// the delete keys, cut, paste, undo, redo and the replace of the find
    /// bar do nothing. The cursor, the selection, copy and find work as
    /// before. For a file that is only looked at. The calls of an app,
    /// like `set_text` and `replace`, still change the text.
    pub fn set_read_only(&self, read_only: bool) -> &Self {
        let mut this = weak_from_ref(self);
        this.read_only = read_only;
        self
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only
    }
}
