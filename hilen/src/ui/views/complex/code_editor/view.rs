use std::ops::Range;

use unicode_width::UnicodeWidthChar;

use super::{
    edit::LinesChanged,
    find::FindState,
    find_bar::FindBar,
    line::{CodeLine, Columns, LineLook, code_label},
    style::CodeEditorStyle,
    text::{CodePos, CodeText},
};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::{LossyConvert, ToF32, flat::Point},
    ui::{
        CellRegistry, Container, Label, Setup, TableData, TableView, ToLabel, UIColor, View, ViewCallbacks,
        ViewData, ViewFrame, ViewTouch, view, view::DoubleTap,
    },
    window::request_frame,
};

/// A tab char of a file draws up to the next stop of this many columns,
/// see `expand_tabs` of the text layout.
const TAB_STOP: usize = 4;
/// The gutter holds the numbers with at least this many digits.
const MIN_DIGITS: usize = 2;
/// Room for the caret after the end of the widest line, in chars.
const END_ROOM: f32 = 2.0;
/// The scroll bars of the table are drawn over the edges of the rows. The
/// rows can scroll this far past them, and the cursor stays clear of them.
const BAR_ROOM: f32 = 12.0;

/// What the app puts on 1 line: its colors and a mark.
#[derive(Debug, Default, Clone)]
pub(super) struct LineDecor {
    pub runs: Vec<(Range<usize>, UIColor)>,
    pub mark: Option<String>,
}

/// A view that edits code. The text is kept as lines, and only the lines
/// on screen are views, so a long file costs no more to show than a short
/// one. It has a gutter with line numbers, a mono font, a caret and a
/// selection, the keys of a text editor, copy and paste, undo and redo,
/// Tab and Shift Tab that indent with spaces, and a find bar with replace.
///
/// The app colors the code with `set_color_runs` per line, marks a line
/// with a message, an error for example, with `set_mark`, and lights a
/// range of lines with `set_highlight`. After every change the user makes
/// it gets `changed` with the first line that changed.
#[view]
pub struct CodeEditor {
    pub(super) text:    CodeText,
    pub(super) decor:   Vec<LineDecor>,
    /// The width of every line in columns, with the lines.
    widths:             Vec<usize>,
    highlight:          Option<Range<usize>>,
    style:              CodeEditorStyle,
    columns:            Columns,
    /// The rows show the text, the cursor and the decor of now.
    pub(super) stale:   bool,
    pub(super) editing: bool,

    pub(super) clicks:      DoubleTap,
    pub(super) click_count: usize,
    /// A press is moving the cursor and the selection follows it.
    pub(super) dragging:    bool,

    /// Whether the find bar is open, the query, the matches and the
    /// current match.
    pub(super) find:     FindState,
    pub(super) find_bar: Weak<FindBar>,

    /// The user changed the text, with the first line that changed. The
    /// colors of the changed lines are gone, the app sets them again
    /// here. A change by `set_text` fires nothing.
    pub changed:      Event<usize>,
    /// The cursor moved, by the user or by a change.
    pub cursor_moved: Event<CodePos>,

    /// The gutter under the rows, so it goes on below the last line.
    #[init]
    gutter:           Container,
    pub(super) table: TableView,
    measure:          Label,
}

impl Setup for CodeEditor {
    fn setup(mut self: Weak<Self>) {
        self.decor = vec![LineDecor::default()];
        self.widths = vec![0];
        self.table.set_data_source(self).register_cell::<CodeLine>();
        self.table.set_footer_height(BAR_ROOM);
        self.measure.set_hidden(true);

        self.setup_touch();
        self.setup_find();
        self.size_changed().sub(move || self.relayout());
        self.use_style();
    }

    fn on_selection_changed(self: Weak<Self>, selected: bool) {
        self.editing_changed(selected);
    }
}

impl ViewCallbacks for CodeEditor {
    fn update(&mut self) {
        if self.stale {
            self.stale = false;
            self.table.reload_data();
        }
    }
}

impl CodeEditor {
    /// Puts a new text in, with the cursor at the start, no colors, no
    /// marks and no undo history.
    pub fn set_text(&mut self, text: &str) -> &mut Self {
        self.text.set_text(text);
        let count = self.text.line_count();
        self.decor = vec![LineDecor::default(); count];
        self.widths = self.text.lines().iter().map(|line| columns_of(line)).collect();
        self.relayout();
        self.table.set_content_offset(0);
        self.table.set_content_offset_x(0);
        if self.find.open {
            self.search();
        }
        self
    }

    /// The whole text, lines joined by a line break.
    pub fn text(&self) -> String {
        self.text.text()
    }

    pub fn lines(&self) -> &[String] {
        self.text.lines()
    }

    pub fn line_count(&self) -> usize {
        self.text.line_count()
    }

    /// The colors of the code on `line`, byte ranges of that line. A
    /// change of the line by the user drops them.
    pub fn set_color_runs(&mut self, line: usize, runs: impl IntoIterator<Item = (Range<usize>, UIColor)>) {
        if let Some(decor) = self.decor.get_mut(line) {
            decor.runs = runs.into_iter().collect();
            self.refresh();
        }
    }

    /// The colors of the code on `line`, empty after a change of the line
    /// until the app sets them again.
    pub fn color_runs(&self, line: usize) -> &[(Range<usize>, UIColor)] {
        self.decor.get(line).map_or(&[], |decor| decor.runs.as_slice())
    }

    /// Marks `line` with a message, drawn after its code: the number and
    /// the message in the mark color and the line on its own background.
    /// A mark moves with its line when lines are added or taken out above
    /// it.
    pub fn set_mark(&mut self, line: usize, message: impl ToLabel) {
        if let Some(decor) = self.decor.get_mut(line) {
            decor.mark = Some(message.to_label());
            self.refresh();
        }
    }

    pub fn clear_mark(&mut self, line: usize) {
        if let Some(decor) = self.decor.get_mut(line) {
            decor.mark = None;
            self.refresh();
        }
    }

    pub fn clear_marks(&mut self) {
        for decor in &mut self.decor {
            decor.mark = None;
        }
        self.refresh();
    }

    /// The message of the mark on `line`.
    pub fn mark(&self, line: usize) -> Option<&str> {
        self.decor.get(line)?.mark.as_deref()
    }

    /// Lights the lines of `lines` with the highlight color, like the
    /// lines of a part picked in a 3D view. `None` lights none.
    pub fn set_highlight(&mut self, lines: Option<Range<usize>>) {
        self.highlight = lines;
        self.refresh();
    }

    pub fn highlight(&self) -> Option<Range<usize>> {
        self.highlight.clone()
    }

    pub fn style(&self) -> CodeEditorStyle {
        self.style
    }

    pub fn set_style(&mut self, style: CodeEditorStyle) -> &mut Self {
        self.style = style;
        self.use_style();
        self
    }

    /// How many spaces Tab puts in and Shift Tab takes out, 4 by default.
    pub fn set_tab_width(&mut self, spaces: usize) -> &mut Self {
        self.text.set_tab_width(spaces);
        self
    }

    pub fn tab_width(&self) -> usize {
        self.text.tab_width()
    }

    pub fn cursor(&self) -> CodePos {
        self.text.cursor()
    }

    /// Puts the cursor at `pos` and scrolls to it. The selection goes.
    pub fn set_cursor(&mut self, pos: CodePos) {
        self.text.move_to(pos, false);
        self.cursor_changed();
    }

    /// Selects from `from` to `to`, the cursor at `to`.
    pub fn select(&mut self, from: CodePos, to: CodePos) {
        self.text.select(from, to);
        self.cursor_changed();
    }

    pub fn select_all(&mut self) {
        self.text.select_all();
        self.cursor_changed();
    }

    /// The selection, start before end, `None` when nothing is selected.
    pub fn selection(&self) -> Option<(CodePos, CodePos)> {
        self.text.selection()
    }

    pub fn selected_text(&self) -> String {
        self.text.selected_text()
    }

    /// Puts `text` in place of the selection, or at the cursor, as 1 step
    /// of the undo history. Fires `changed` like a paste.
    pub fn insert(&mut self, text: &str) {
        let changed = self.text.insert(text);
        self.lines_changed(changed);
    }

    pub fn undo(&mut self) {
        if let Some(changed) = self.text.undo() {
            self.lines_changed(changed);
        }
    }

    pub fn redo(&mut self) {
        if let Some(changed) = self.text.redo() {
            self.lines_changed(changed);
        }
    }

    pub fn can_undo(&self) -> bool {
        self.text.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.text.can_redo()
    }

    /// The editor has the keys.
    pub fn is_editing(&self) -> bool {
        self.editing
    }

    /// Gives the editor the keys, the way a press on it does.
    pub fn focus(&self) {
        self.select();
    }

    /// Scrolls `line` to the top, or as near as the end allows.
    pub fn scroll_to_line(&mut self, line: usize) {
        self.table.scroll_to_row(line);
    }

    /// The height of 1 line in points.
    pub fn line_height(&self) -> f32 {
        self.columns.row_height
    }
}

impl CodeEditor {
    fn use_style(&mut self) {
        let style = self.style;
        self.set_color(style.background);
        self.gutter.set_color(style.gutter);
        code_label(self.measure, &style);
        self.find_bar.set_look(&style);
        self.relayout();
    }

    /// The rows draw again on the next frame. For the decor, which an app
    /// may set for many lines in a row.
    pub(super) fn refresh(&mut self) {
        self.stale = true;
        request_frame();
    }

    /// Works out the columns from the font, the count of lines and the
    /// widest line, puts the rows under the find bar, and draws every row
    /// again.
    pub(super) fn relayout(&mut self) {
        let (width, height) = (self.width(), self.height());
        let top = if self.find.open {
            self.find_bar.bar_height()
        } else {
            0.0
        };
        self.find_bar.set_frame((0.0, 0.0, width, top));
        self.table.set_frame((0.0, top, width, (height - top).max(0.0)));

        let cell = self.measure.cell_size();
        let digits = self.text.line_count().to_string().len().max(MIN_DIGITS);
        let digits: f32 = digits.lossy_convert();
        let widest: f32 = self.widths.iter().copied().max().unwrap_or(0).lossy_convert();

        let gutter_width = (digits + 2.0) * cell.width;
        let text_x = gutter_width + self.style.padding;
        self.columns = Columns {
            row_height: cell.height + self.style.line_spacing,
            gutter_width,
            text_x,
            char_width: cell.width,
            text_width: (widest + END_ROOM) * cell.width,
        };
        self.gutter.set_frame((0.0, 0.0, gutter_width, self.height()));
        self.table
            .set_content_width(text_x + self.columns.text_width + self.style.padding);
        self.table.reload_data();
        self.stale = false;
    }

    /// The user changed the lines: their decor and widths follow, then
    /// the rows, the scroll and the events.
    pub(super) fn lines_changed(&mut self, changed: LinesChanged) {
        let LinesChanged {
            start,
            removed,
            inserted,
        } = changed;
        let mark = self
            .decor
            .get(start)
            .and_then(|decor| decor.mark.clone())
            .filter(|_| removed > 0);
        self.decor.splice(start..start + removed, vec![LineDecor::default(); inserted]);
        if inserted > 0 {
            self.decor[start].mark = mark;
        }
        let widths: Vec<usize> = self.text.lines()[start..start + inserted]
            .iter()
            .map(|line| columns_of(line))
            .collect();
        self.widths.splice(start..start + removed, widths);
        if self.find.open {
            self.find.lines_changed(self.text.lines(), changed);
            self.matches_followed();
        }

        self.relayout();
        self.scroll_to_cursor();
        self.changed.trigger(start);
        self.cursor_moved.trigger(self.text.cursor());
    }

    /// The cursor or the selection moved with no change of the text.
    pub(super) fn cursor_changed(&mut self) {
        self.table.reload_data();
        self.stale = false;
        self.scroll_to_cursor();
        self.cursor_moved.trigger(self.text.cursor());
    }

    /// Scrolls so the line and the column of the cursor are on screen.
    fn scroll_to_cursor(&mut self) {
        let cursor = self.text.cursor();
        self.reveal(cursor, cursor);
    }

    /// Scrolls so the line of `from` and the text from `from` to `to` on
    /// it are on screen, `from` first when both do not fit.
    pub(super) fn reveal(&mut self, from: CodePos, to: CodePos) {
        let height = self.table.height() - BAR_ROOM;
        let width = self.table.width() - BAR_ROOM;
        if height <= 0.0 || width <= 0.0 {
            return;
        }
        let row = self.columns.row_height;
        let top = from.line.to_f32() * row;
        let shown_top = -self.table.content_offset();
        if top < shown_top {
            self.table.set_content_offset(-top);
        } else if top + row > shown_top + height {
            self.table.set_content_offset(height - top - row);
        }

        let x = self.x_of(from);
        let end = if to.line == from.line { self.x_of(to) } else { x };
        let room = (width - self.columns.text_x - self.columns.char_width).max(0.0);
        let shown_left = -self.table.content_offset_x();
        let left = if x < shown_left {
            x
        } else if end > shown_left + room {
            (end - room).min(x)
        } else {
            return;
        };
        self.table.set_content_offset_x(-left);
    }

    /// Where the caret place of `pos` is, in the points of the editor, in
    /// the middle of its line. `None` for a line that is not laid out.
    /// For a test that points at a byte of the code.
    pub fn point_of(&self, pos: CodePos) -> Option<Point> {
        if pos.line >= self.text.line_count() {
            return None;
        }
        let top = pos.line.to_f32() * self.columns.row_height + self.table.content_offset() + self.table.y();
        let x = self.columns.text_x + self.x_of(pos) + self.table.content_offset_x();
        Some(Point::new(x, top + self.columns.row_height / 2.0))
    }

    /// Where `pos` is on its line, from the start of the code, in points.
    fn x_of(&self, pos: CodePos) -> f32 {
        let line = self.text.line(pos.line);
        self.measure.text_layout_for(line).x_on_line(0, pos.byte)
    }

    /// The place in the text under `point`, in the points of the editor.
    /// A point above or below the lines lands on the first or the last.
    pub(super) fn pos_at(&self, point: Point) -> CodePos {
        let y = point.y - self.table.y() - self.table.content_offset();
        let last = self.text.line_count() - 1;
        let row: usize = (y / self.columns.row_height).floor().max(0.0).lossy_convert();
        let line = row.min(last);
        let x = point.x - self.table.content_offset_x() - self.columns.text_x;
        let byte = self.measure.text_layout_for(self.text.line(line)).nearest_on_line(0, x);
        CodePos::new(line, byte)
    }
}

/// How many columns of a mono font `line` takes.
fn columns_of(line: &str) -> usize {
    line.chars().fold(0, |column, ch| {
        if ch == '\t' {
            (column / TAB_STOP + 1) * TAB_STOP
        } else {
            column + ch.width().unwrap_or(0)
        }
    })
}

impl TableData for CodeEditor {
    fn cell_height(&self, _: usize) -> f32 {
        self.columns.row_height
    }

    fn number_of_cells(&self) -> usize {
        self.text.line_count()
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<CodeLine>();
        let decor = self.decor.get(index);
        let cursor = self.text.cursor();
        let look = LineLook {
            number:      index + 1,
            text:        self.text.line(index),
            runs:        decor.map_or(&[], |decor| decor.runs.as_slice()),
            selected:    self.text.selected_on(index),
            caret:       (self.editing && cursor.line == index).then_some(cursor.byte),
            mark:        decor.and_then(|decor| decor.mark.as_deref()),
            highlighted: self.highlight.as_ref().is_some_and(|lines| lines.contains(&index)),
            matches:     self.find.on_line(index),
        };
        cell.show(&look, &self.columns, &self.style);
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}
