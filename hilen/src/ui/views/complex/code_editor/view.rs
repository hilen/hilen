use std::{mem::take, ops::Range};

use unicode_width::UnicodeWidthChar;

use super::{
    comments::CommentRules,
    completion::CompletionList,
    decor::MarkKind,
    edit::LinesChanged,
    find::FindState,
    find_bar::FindBar,
    inlay::{Inlays, Shown},
    line::{CodeLine, Columns, LineLook, code_label},
    pairs::BracketRules,
    pointer::{CodePress, Rest},
    style::CodeEditorStyle,
    text::{CodePos, CodeText},
};
use crate::{
    self as hilen,
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{LossyConvert, ToF32, flat::Point},
    ui::{
        CellRegistry, CodeScope, Container, Label, Setup, TableData, TableView, ToLabel, UIColor, View,
        ViewCallbacks, ViewData, ViewFrame, ViewTouch, WeakView, view, view::DoubleTap,
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

/// What the app puts on 1 line: its colors, a mark, the lines under and
/// the colors behind parts of it, the texts inside it that are no part of
/// the file, and where its strings and comments are.
#[derive(Debug, Default, Clone)]
pub(super) struct LineDecor {
    pub runs:        Vec<(Range<usize>, UIColor)>,
    pub mark:        Option<(String, MarkKind)>,
    pub underlines:  Vec<(Range<usize>, UIColor)>,
    pub backgrounds: Vec<(Range<usize>, UIColor)>,
    pub inlays:      Inlays,
    pub scopes:      Vec<(Range<usize>, CodeScope)>,
}

/// A view that edits code. The text is kept as lines, and only the lines
/// on screen are views, so a long file costs no more to show than a short
/// one. It has a gutter with line numbers, a mono font, a caret and a
/// selection, the keys of a text editor, copy and paste, undo and redo,
/// Tab and Shift Tab that indent with spaces, and a find bar with replace.
///
/// The app colors the code with `set_color_runs` per line, marks a line
/// with a message, an error for example, with `set_mark`, lights a range
/// of lines with `set_highlight`, and shows a text inside a line that is
/// no part of the file with `set_inlays`. After every change the user makes
/// it gets `changed` with the first line that changed.
/// What the rows of the editor do on the next frame.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Redraw {
    #[default]
    Nothing,
    Rows,
    /// A line got wider or narrower with no change of the text, so the
    /// columns are worked out again before the rows draw.
    Columns,
}

#[view]
pub struct CodeEditor {
    pub(super) text:        CodeText,
    pub(super) decor:       Vec<LineDecor>,
    /// The width of every line in columns, with the lines.
    widths:                 Vec<usize>,
    highlight:              Option<Range<usize>>,
    style:                  CodeEditorStyle,
    columns:                Columns,
    /// What the rows do on the next frame to show the text, the cursor and
    /// the decor of now.
    redraw:                 Redraw,
    pub(super) editing:     bool,
    /// The user can not change the text, see `set_read_only`.
    pub(super) read_only:   bool,
    /// The pairs a typed opener closes, see `set_brackets`.
    pub(super) brackets:    BracketRules,
    /// The comments Cmd / toggles, see `set_comments`.
    pub(super) comments:    CommentRules,
    /// Where the pointer is over the code and whether it rests there.
    pub(super) rest:        Rest,
    /// The view that floats at a place of the code, see `show_panel`, and
    /// that place.
    pub(super) panel:       WeakView,
    pub(super) panel_at:    CodePos,
    /// The view that floats over a line of the code, see `show_note`.
    pub(super) note:        WeakView,
    /// The list of completions while it shows, the content of the panel.
    pub(super) completions: Weak<CompletionList>,

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
    pub changed:           Event<usize>,
    /// The cursor moved, by the user or by a change.
    pub cursor_moved:      Event<CodePos>,
    /// A press with a mouse put the cursor at a place, with whether Cmd
    /// was held, Ctrl elsewhere. An app opens a definition from it.
    pub pressed:           Event<CodePress>,
    /// The pointer rests on a char of the code, `None` once it moved off
    /// that place again. An app shows what it knows of the name there.
    pub pointer_rested:    Event<Option<CodePos>>,
    /// The user picked a row of the list of `show_completions`, with its
    /// index. The list is closed already.
    pub completion_picked: Event<usize>,
    /// The mark of the list of `show_completions` moved to a row, with
    /// its index, also to the first row when a list shows. An app asks
    /// here for more of that row, like the `use` line that a language
    /// server gives only on a resolve.
    pub completion_marked: Event<usize>,
    /// The keys left the editor and its find bar, by a press on another
    /// view, a field that took them or an unselect. Keys that go into its
    /// own find bar and back fire nothing. An app saves the file here.
    pub editing_ended:     Event,

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
        match self.redraw {
            Redraw::Nothing => (),
            Redraw::Rows => {
                self.redraw = Redraw::Nothing;
                self.table.reload_data();
            }
            Redraw::Columns => self.relayout(),
        }
    }
}

impl CodeEditor {
    /// Puts a new text in, with the cursor at the start, no colors, no
    /// marks and no undo history.
    pub fn set_text(&self, text: &str) -> &Self {
        let mut this = weak_from_ref(self);
        this.text.set_text(text);
        let count = this.text.line_count();
        this.decor = vec![LineDecor::default(); count];
        this.widths = this.text.lines().iter().map(|line| columns_of(line)).collect();
        this.relayout();
        this.table.set_content_offset(0);
        this.table.set_content_offset_x(0);
        if this.find.open {
            this.search();
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
    pub fn set_color_runs(&self, line: usize, runs: impl IntoIterator<Item = (Range<usize>, UIColor)>) {
        let mut this = weak_from_ref(self);
        if let Some(decor) = this.decor.get_mut(line) {
            decor.runs = runs.into_iter().collect();
            this.refresh();
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
    pub fn set_mark(&self, line: usize, message: impl ToLabel) {
        let mut this = weak_from_ref(self);
        if let Some(decor) = this.decor.get_mut(line) {
            decor.mark = Some((message.to_label(), MarkKind::Error));
            this.refresh();
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
        self.decor.get(line)?.mark.as_ref().map(|(message, _)| message.as_str())
    }

    /// Lights the lines of `lines` with the highlight color, like the
    /// lines of a part picked in a 3D view. `None` lights none.
    pub fn set_highlight(&self, lines: Option<Range<usize>>) {
        let mut this = weak_from_ref(self);
        this.highlight = lines;
        this.refresh();
    }

    pub fn highlight(&self) -> Option<Range<usize>> {
        self.highlight.clone()
    }

    pub fn style(&self) -> CodeEditorStyle {
        self.style
    }

    pub fn set_style(&self, style: CodeEditorStyle) -> &Self {
        let mut this = weak_from_ref(self);
        this.style = style;
        this.use_style();
        self
    }

    /// How many spaces Tab puts in and Shift Tab takes out, 4 by default.
    pub fn set_tab_width(&self, spaces: usize) -> &Self {
        let mut this = weak_from_ref(self);
        this.text.set_tab_width(spaces);
        self
    }

    pub fn tab_width(&self) -> usize {
        self.text.tab_width()
    }

    pub fn cursor(&self) -> CodePos {
        self.text.cursor()
    }

    /// Puts the cursor at `pos` and scrolls to it. The selection goes.
    pub fn set_cursor(&self, pos: CodePos) {
        let mut this = weak_from_ref(self);
        this.text.move_to(pos, false);
        this.cursor_changed();
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

    /// Puts a snippet in place of the text from `from` to `to` as 1 step
    /// of the undo history, like a completion of a language server. The
    /// syntax is the one of LSP and `TextMate`: `$1`, `${1:default}`,
    /// `${1|one,two|}` and `$0` for the end. The marks go, each line after
    /// the first gets the indent of the line it goes into, and the cursor
    /// goes to the first place with its default text selected. Tab and
    /// Shift Tab go to the next place and the one before, `$0` or the end
    /// of the text ends it, so does Escape or a cursor that left the place.
    /// Fires `changed` like `insert`.
    pub fn insert_snippet(&mut self, from: CodePos, to: CodePos, snippet: &str) {
        let changed = self.text.insert_snippet(from, to, snippet);
        self.lines_changed(changed);
    }

    /// `insert_snippet`, but when the text is still at `version` the
    /// snippet joins the newest step of the undo history, like
    /// `replace_joined`. A late part of 1 change that has places to fill
    /// in, like the items of a trait a language server writes after its
    /// name went in.
    pub fn insert_snippet_joined(&mut self, version: u64, from: CodePos, to: CodePos, snippet: &str) {
        let changed = self.text.insert_snippet_joined(version, from, to, snippet);
        self.lines_changed(changed);
    }

    /// The user is filling in the places of a snippet, Tab goes to the
    /// next one.
    pub fn snippet_active(&self) -> bool {
        self.text.snippet_active()
    }

    /// Puts `text` in place of the text from `from` to `to` as 1 step of
    /// the undo history, the way an app changes the text, like a format on
    /// save. The cursor and the selection keep their place in the text
    /// around the change, and the lines on screen stay where they are.
    /// Fires `changed` like `insert`.
    pub fn replace(&mut self, from: CodePos, to: CodePos, text: &str) {
        let changed = self.text.replace_kept(from, to, text);
        self.replaced(changed);
    }

    /// A number that grows with every change of the text, an undo and a
    /// redo too. An app that waits for something, like a language server,
    /// tells by it that the text did not change meanwhile.
    pub fn version(&self) -> u64 {
        self.text.version
    }

    /// `replace`, but when the text is still at `version` the change joins
    /// the newest step of the undo history, so 1 undo takes back both and
    /// 1 redo puts both back. A late part of 1 change, like the `use` line
    /// a language server sends after the name of a completion went in.
    /// A text that changed since `version` gets a step of its own, like
    /// with `replace`.
    pub fn replace_joined(&mut self, version: u64, from: CodePos, to: CodePos, text: &str) {
        let changed = self.text.replace_joined(version, from, to, text);
        self.replaced(changed);
    }

    fn replaced(&mut self, changed: LinesChanged) {
        let shown_top = -self.table.content_offset();
        let row = self.columns.row_height;
        self.follow_lines(changed);
        // Lines added or taken out above the screen move the text on it,
        // so the scroll moves by as much.
        if (changed.start + changed.removed).to_f32() * row <= shown_top {
            let moved = (changed.inserted.to_f32() - changed.removed.to_f32()) * row;
            self.table.set_content_offset(-(shown_top + moved));
        }
        let first = self.follow_mirrored(changed.start);
        self.changed.trigger(first);
        self.cursor_moved.trigger(self.text.cursor());
    }

    /// The lines the copies of a snippet place were written on follow,
    /// each on its own, so the lines between keep their marks and hints.
    /// The first changed line of them and of `first`.
    fn follow_mirrored(&mut self, first: usize) -> usize {
        let mut first = first;
        for line in self.text.take_mirrored() {
            self.follow_lines(LinesChanged {
                start:    line,
                removed:  1,
                inserted: 1,
            });
            first = first.min(line);
        }
        first
    }

    /// Takes back the newest step of the undo history, with the steps
    /// joined to it by `replace_joined` and `insert_snippet_joined`.
    pub fn undo(&mut self) {
        for changed in self.text.undo() {
            self.lines_changed(changed);
        }
    }

    pub fn redo(&mut self) {
        for changed in self.text.redo() {
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
        if self.redraw == Redraw::Nothing {
            self.redraw = Redraw::Rows;
        }
        request_frame();
    }

    /// `line` as it is drawn, with its inlays.
    fn shown(&self, line: usize) -> Shown<'_> {
        let inlays = self.decor.get(line).map_or(&[][..], |decor| decor.inlays.items());
        Shown::of(self.text.line(line), inlays)
    }

    /// The inlays of `line` changed, and so the width it is drawn with.
    pub(super) fn width_changed(&mut self, line: usize) {
        let width = columns_of(&self.shown(line).text);
        if let Some(kept) = self.widths.get_mut(line) {
            *kept = width;
        }
        self.redraw = Redraw::Columns;
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
        self.redraw = Redraw::Nothing;
    }

    /// The user changed the lines: their decor and widths follow, then
    /// the rows, the scroll and the events.
    pub(super) fn lines_changed(&mut self, changed: LinesChanged) {
        self.text.drop_far_regions();
        self.text.drop_far_snippet();
        self.follow_lines(changed);
        let first = self.follow_mirrored(changed.start);
        self.scroll_to_cursor();
        self.changed.trigger(first);
        self.cursor_moved.trigger(self.text.cursor());
    }

    /// The decor, the widths, the matches and the rows follow a change of
    /// the lines.
    fn follow_lines(&mut self, changed: LinesChanged) {
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
        // The inlays of a line that changed inside itself stay on it.
        let inlays = (removed == 1 && inserted == 1).then(|| take(&mut self.decor[start].inlays));
        self.decor.splice(start..start + removed, vec![LineDecor::default(); inserted]);
        if inserted > 0 {
            self.decor[start].mark = mark;
        }
        if let Some(mut inlays) = inlays {
            inlays.follow(self.text.line(start));
            self.decor[start].inlays = inlays;
        }
        let widths: Vec<usize> = (start..start + inserted)
            .map(|line| columns_of(&self.shown(line).text))
            .collect();
        self.widths.splice(start..start + removed, widths);
        if self.find.open {
            self.find.lines_changed(self.text.lines(), changed);
            self.matches_followed();
        }
        self.relayout();
    }

    /// The cursor or the selection moved with no change of the text.
    pub(super) fn cursor_changed(&mut self) {
        self.text.drop_far_regions();
        self.text.drop_far_snippet();
        self.table.reload_data();
        if self.redraw == Redraw::Rows {
            self.redraw = Redraw::Nothing;
        }
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
        let shown = self.shown(pos.line);
        self.measure.text_layout_for(&shown.text).x_on_line(0, shown.caret(pos.byte))
    }

    /// The place in the text under `point`, in the points of the editor,
    /// only when a char of the code is there: `None` beside the code, past
    /// the end of a line, under the last line, on the find bar and on an
    /// inlay.
    pub fn text_at(&self, point: Point) -> Option<CodePos> {
        let y = point.y - self.table.y() - self.table.content_offset();
        let x = point.x - self.table.content_offset_x() - self.columns.text_x;
        let inside =
            point.y >= self.table.y() && point.x >= self.columns.gutter_width && y >= 0.0 && x >= 0.0;
        if !inside {
            return None;
        }
        let row: usize = (y / self.columns.row_height).floor().lossy_convert();
        if row >= self.text.line_count() {
            return None;
        }
        let shown = self.shown(row);
        let layout = self.measure.text_layout_for(&shown.text);
        if x >= layout.x_on_line(0, shown.text.len()) {
            return None;
        }
        let (byte, on_inlay) = shown.byte_of(layout.nearest_on_line(0, x));
        (!on_inlay).then(|| CodePos::new(row, byte))
    }

    /// The place in the text nearest to `point`, in the points of the
    /// editor. A point above or below the lines lands on the first or the
    /// last, the way a press puts the cursor. A point on an inlay lands on
    /// the byte the inlay stands at.
    pub fn pos_at(&self, point: Point) -> CodePos {
        let y = point.y - self.table.y() - self.table.content_offset();
        let last = self.text.line_count() - 1;
        let row: usize = (y / self.columns.row_height).floor().max(0.0).lossy_convert();
        let line = row.min(last);
        let x = point.x - self.table.content_offset_x() - self.columns.text_x;
        let shown = self.shown(line);
        let nearest = self.measure.text_layout_for(&shown.text).nearest_on_line(0, x);
        CodePos::new(line, shown.byte_of(nearest).0)
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
            mark:        decor
                .and_then(|decor| decor.mark.as_ref())
                .map(|(message, kind)| (message.as_str(), *kind)),
            underlines:  decor.map_or(&[], |decor| decor.underlines.as_slice()),
            backgrounds: decor.map_or(&[], |decor| decor.backgrounds.as_slice()),
            inlays:      decor.map_or(&[][..], |decor| decor.inlays.items()),
            highlighted: self.highlight.as_ref().is_some_and(|lines| lines.contains(&index)),
            matches:     self.find.on_line(index),
        };
        cell.show(&look, &self.columns, &self.style);
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}
