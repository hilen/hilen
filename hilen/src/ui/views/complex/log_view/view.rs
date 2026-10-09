use log::debug;

use super::{
    ansi::parse,
    cell::{Columns, LogCell, fill},
    data::{LogData, LogLine},
    end_button::{BUTTON_SIZE, LogEndButton},
    style::LogStyle,
};
use crate::{
    self as hilen,
    deps::refs::Weak,
    gm::{LossyConvert, flat::Size},
    ui::{
        CellRegistry, Label, Setup, TableData, TableView, TextAlignment, UIManager, View, ViewCallbacks,
        ViewData, ViewFrame, view,
    },
    window::request_frame,
};

/// The scroll bar of the table is drawn over the right edge of the rows,
/// the text stays clear of it.
const BAR_ROOM: f32 = 8.0;
const BUTTON_MARGIN: f32 = 12.0;
/// How far from the end the view still counts as at the end.
const END_SLOP: f32 = 1.0;
/// 2 prefix columns closer than this are 1 width, a float is not exact.
const WIDTH_SLOP: f32 = 0.5;
/// 2 row heights closer than this are 1 height.
const HEIGHT_SLOP: f32 = 0.01;
/// An exact height of a row on screen can bring another row on screen.
/// This many rounds settle it, the next frame goes on if they did not.
const SETTLE_ROUNDS: usize = 4;
/// The sample that gives the width of 1 char of the font.
const SAMPLE: &str = "MMMMMMMMMM";

/// What the table showed at the last look: the scroll offset, the height
/// of the content and its own size. A change of only the offset is a
/// scroll.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
struct Seen {
    offset:  f32,
    content: f32,
    size:    Size,
}

/// A log: lines of text in a mono font, 1 row per line, the newest at
/// the bottom. A line wraps and its row is as tall as its text. ANSI
/// color codes are drawn as colors. A line can have a prefix, which
/// stands in a column of its own while the text wraps beside it. The
/// text selects and copies over many rows.
///
/// The app holds the lines and gives them through `LogData`. After a
/// change it calls `lines_added`, `lines_removed` or `reload`.
///
/// The view shapes the text of a line only when its row comes on screen.
/// Every other row has a height from the count of its chars and the
/// width of 1 char of the mono font, plain arithmetic. So a new width
/// and new lines cost the same in a log of 1000 lines and of a million.
/// The scroll bar is a little off for a part of the log nobody saw yet,
/// word wrap can take 1 row more than the arithmetic gives.
///
/// While the view is scrolled to the end it follows new lines. It stops
/// when the user scrolls up, and a round button then jumps back to the
/// end.
#[view]
pub struct LogView {
    data:  Weak<dyn LogData>,
    style: LogStyle,

    /// How many chars the text of every line has, its codes left out.
    chars:   Vec<usize>,
    /// The height of the row of every line at the width of `laid`.
    heights: Vec<f32>,
    /// The height of that row came from the shaped text.
    exact:   Vec<bool>,
    /// `chars` holds the lines of the data.
    counted: bool,

    /// The width of the widest prefix, 0 when no line has one.
    prefix_width: f32,
    /// The most chars a measured prefix had.
    prefix_chars: usize,
    /// The height of 1 line of text, the height of a row with no text.
    one_line:     f32,
    /// The width of 1 char of the font.
    advance:      f32,
    /// The empty strip a label keeps beside its text.
    text_margin:  f32,
    /// The size the rows were laid out for.
    laid:         Size,
    seen:         Seen,

    #[educe(Default = true)]
    following: bool,

    #[init]
    table:          TableView,
    measure:        Label,
    measure_prefix: Label,
    to_end:         LogEndButton,
}

impl Setup for LogView {
    fn setup(mut self: Weak<Self>) {
        self.table.set_data_source(self).register_cell::<LogCell>();
        self.table.set_variable_heights(true).set_text_selectable(true);

        self.measure.set_multiline(true);
        self.measure.set_hidden(true);
        self.measure_prefix.set_hidden(true);
        for label in [self.measure, self.measure_prefix] {
            label.set_alignment(TextAlignment::Left);
        }

        // Over the rows of the table, which are deeper in the tree than
        // a view added after it.
        self.to_end.bump_z_position(UIManager::subview_z_offset() * 10.0);
        self.to_end.set_hidden(true);
        self.to_end.tapped.sub(move || self.scroll_to_end());

        self.use_style();
    }
}

impl LogView {
    pub fn set_data_source(mut self: Weak<Self>, data: Weak<dyn LogData>) -> Weak<Self> {
        self.data = data;
        self.reload();
        self
    }

    pub fn style(&self) -> LogStyle {
        self.style
    }

    pub fn set_style(&mut self, style: LogStyle) -> &mut Self {
        self.style = style;
        self.use_style();
        self.reload();
        self
    }

    /// Lines came to the end of the data. Only they are asked, once
    /// each, and the ones that are on screen once more. A new line with
    /// a wider prefix than every line before it makes the prefix column
    /// wider.
    pub fn lines_added(&mut self) {
        if !self.is_laid() {
            return;
        }
        self.note_scroll();

        let before = self.chars.len();
        let count = self.line_count();
        if count < before || !self.counted {
            self.reload();
            return;
        }
        if count == before {
            return;
        }

        let mut wider = false;
        for index in before..count {
            let line = self.data.line(index);
            wider |= self.note_prefix(&line);
            self.chars.push(parse(&line.text).text.chars().count());
        }

        if wider {
            // The text column got narrower, every row has a new height.
            self.estimate_all();
            self.table.reload_data();
        } else {
            let per_row = self.chars_per_row();
            for index in before..count {
                self.heights.push(self.estimate(self.chars[index], per_row));
                self.exact.push(false);
            }
            self.table.load_new_cells();
        }
        self.keep_end();
    }

    /// The first `count` lines left the data. No line is asked. The
    /// lines on screen stay where they are, and a selection stays on its
    /// text.
    pub fn lines_removed(&mut self, count: usize) {
        if !self.is_laid() || count == 0 {
            return;
        }
        self.note_scroll();

        if !self.counted || count > self.chars.len() || self.chars.len() - count != self.line_count() {
            self.reload();
            return;
        }
        self.chars.drain(..count);
        self.heights.drain(..count);
        self.exact.drain(..count);
        self.table.drop_first_cells(count);
        self.keep_end();
    }

    /// The lines changed in some other way. Every line is asked again.
    pub fn reload(&mut self) {
        if !self.is_laid() {
            return;
        }
        self.note_scroll();
        self.count_all();
        self.table.reload_data();
        self.keep_end();
    }

    /// The view shows the end of the log and stays there when lines are
    /// added.
    pub fn is_following(&self) -> bool {
        self.following
    }

    /// Jumps to the end of the log and follows it again.
    pub fn scroll_to_end(&mut self) {
        self.following = true;
        self.keep_end();
        request_frame();
    }
}

impl LogView {
    fn use_style(&mut self) {
        let style = self.style;
        self.set_color(style.background);
        self.table.set_header_height(style.padding).set_footer_height(style.padding);
        self.to_end.set_look(style.button, style.button_icon);

        // 2 samples, the second twice as long: their difference is the
        // width of the chars alone, the rest of the first is the margin.
        fill(self.measure, &parse(SAMPLE), style.text, &style);
        let once = self.measure.size_for_width(f32::MAX);
        fill(self.measure, &parse(&SAMPLE.repeat(2)), style.text, &style);
        let twice = self.measure.size_for_width(f32::MAX);

        let sample_chars: f32 = SAMPLE.len().lossy_convert();
        self.one_line = once.height;
        self.advance = ((twice.width - once.width) / sample_chars).max(1.0);
        self.text_margin = (once.width - self.advance * sample_chars).max(0.0);
    }

    /// The table has a size, so a line can be measured.
    fn is_laid(&self) -> bool {
        self.data.is_ok() && self.laid.width > 0.0 && self.laid.height > 0.0
    }

    fn line_count(&self) -> usize {
        if self.data.is_ok() {
            self.data.number_of_lines()
        } else {
            0
        }
    }

    fn columns(&self) -> Columns {
        let padding = self.style.padding;
        let text_x = if self.prefix_width > 0.0 {
            padding + self.prefix_width + self.style.prefix_gap
        } else {
            padding
        };
        Columns {
            prefix_x: padding,
            prefix_width: self.prefix_width,
            text_x,
            text_width: (self.laid.width - text_x - padding - BAR_ROOM).max(1.0),
        }
    }

    /// How many chars of the font fit into 1 row of the text column.
    fn chars_per_row(&self) -> f32 {
        ((self.columns().text_width - self.text_margin) / self.advance).floor().max(1.0)
    }

    /// The height of a row from the count of its chars. Right for a
    /// line that fits 1 row. A line that wraps at words can take a row
    /// more, its row gets the exact height when it comes on screen.
    fn estimate(&self, chars: usize, per_row: f32) -> f32 {
        let chars: f32 = chars.lossy_convert();
        (chars / per_row).ceil().max(1.0) * self.one_line
    }

    fn estimate_all(&mut self) {
        let per_row = self.chars_per_row();
        self.heights = self.chars.iter().map(|chars| self.estimate(*chars, per_row)).collect();
        self.exact = vec![false; self.chars.len()];
    }

    /// The height of a row from its shaped text.
    fn height_of(&self, line: &LogLine, text_width: f32) -> f32 {
        fill(self.measure, &parse(&line.text), self.style.text, &self.style);
        self.measure.size_for_width(text_width).height.max(self.one_line)
    }

    /// Takes the prefix of a line into the width of the prefix column,
    /// true when the column got wider. A prefix is shaped only when it
    /// has more chars than every prefix before it, in a mono font the
    /// others cannot be wider. A prefix with chars the font may not
    /// have is always shaped, a glyph of another font has its own width.
    fn note_prefix(&mut self, line: &LogLine) -> bool {
        let prefix = parse(&line.prefix);
        if prefix.text.is_empty() {
            return false;
        }
        let chars = prefix.text.chars().count();
        if chars <= self.prefix_chars && prefix.text.is_ascii() {
            return false;
        }
        self.prefix_chars = self.prefix_chars.max(chars);

        fill(self.measure_prefix, &prefix, self.style.prefix, &self.style);
        let width = self.measure_prefix.content_size().width;
        if width <= self.prefix_width + WIDTH_SLOP {
            return false;
        }
        self.prefix_width = width;
        true
    }

    /// Asks the data for every line and counts its chars. No text is
    /// shaped but the prefixes that are longer than every one before.
    fn count_all(&mut self) {
        let count = self.line_count();
        self.prefix_width = 0.0;
        self.prefix_chars = 0;
        self.chars.clear();
        self.chars.reserve(count);

        for index in 0..count {
            let line = self.data.line(index);
            self.note_prefix(&line);
            self.chars.push(parse(&line.text).text.chars().count());
        }
        self.counted = true;
        self.estimate_all();
        debug!(
            "Counted {count} log lines, {} chars in a row, prefix column {}",
            self.chars_per_row(),
            self.prefix_width
        );
    }

    fn look(&self) -> Seen {
        Seen {
            offset:  self.table.content_offset(),
            content: self.table.content_height(),
            size:    self.table.size(),
        }
    }

    fn at_end(&self) -> bool {
        self.table.content_height() + self.table.content_offset() <= self.table.height() + END_SLOP
    }

    /// A scroll since the last look decides whether the view follows the
    /// end. It is read before every change of the rows, the change moves
    /// the offset too.
    fn note_scroll(&mut self) {
        let now = self.look();
        let before = self.seen;
        let scrolled = now.offset.to_bits() != before.offset.to_bits()
            && now.content.to_bits() == before.content.to_bits()
            && now.size == before.size;
        if scrolled {
            self.following = self.at_end();
        }
        self.seen = now;
    }

    /// The rows on screen, in order.
    fn rows_on_screen(&self) -> Vec<usize> {
        let mut rows: Vec<usize> = self.table.visible_cells().into_iter().map(|(row, _)| row).collect();
        rows.sort_unstable();
        rows
    }

    /// Where a row is on the screen, in the points of the table.
    fn screen_top(&self, row: usize) -> Option<f32> {
        Some(self.table.row_top(row)? + self.table.content_offset())
    }

    /// Gives every row on screen its exact height. A row whose height
    /// was exact before stays at its place on the screen, so the text
    /// the user reads does not jump when a row above or under it gets
    /// its real height.
    fn settle(&mut self) {
        if !self.is_laid() {
            return;
        }
        let text_width = self.columns().text_width;

        for _ in 0..SETTLE_ROUNDS {
            let rows = self.rows_on_screen();
            let anchor = rows
                .iter()
                .find(|row| self.exact.get(**row).copied().unwrap_or(false))
                .and_then(|row| Some((*row, self.screen_top(*row)?)));

            let mut changed = false;
            for row in rows {
                if row >= self.heights.len() || self.exact[row] {
                    continue;
                }
                let height = self.height_of(&self.data.line(row), text_width);
                changed |= (height - self.heights[row]).abs() > HEIGHT_SLOP;
                self.heights[row] = height;
                self.exact[row] = true;
            }
            if !changed {
                break;
            }

            self.table.reload_data();
            if self.following {
                self.table.scroll_to_bottom();
            } else if let Some((row, top)) = anchor
                && let Some(now) = self.table.row_top(row)
            {
                self.table.set_content_offset(top - now);
            }
        }
    }

    fn keep_end(&mut self) {
        if self.following {
            self.table.scroll_to_bottom();
        }
        self.settle();
        self.seen = self.look();
        self.to_end.set_hidden(self.following);
    }

    fn resized(&mut self, size: Size) {
        let rewrap = size.width.to_bits() != self.laid.width.to_bits();
        // The first row on screen keeps its place over a new width.
        let anchor = self
            .rows_on_screen()
            .first()
            .and_then(|row| Some((*row, self.screen_top(*row)?)));
        self.laid = size;

        // The table gets its frame here and not from a layout rule: the
        // rows are laid out right below, and a rule would give the frame
        // only later in this frame.
        self.table.set_frame((0.0, 0.0, size.width, size.height));
        let corner = BUTTON_SIZE + BUTTON_MARGIN;
        self.to_end.set_frame((
            size.width - corner,
            size.height - corner,
            BUTTON_SIZE,
            BUTTON_SIZE,
        ));

        if !self.is_laid() {
            return;
        }
        if !self.counted {
            self.count_all();
            self.table.reload_data();
        } else if rewrap {
            self.estimate_all();
            self.table.reload_data();
            if !self.following
                && let Some((row, top)) = anchor
                && let Some(now) = self.table.row_top(row)
            {
                self.table.set_content_offset(top - now);
            }
        }
        self.keep_end();
        // The scroll view inside the table gets its new frame after this
        // update, the next frame puts the end in place against it.
        request_frame();
    }
}

impl ViewCallbacks for LogView {
    fn update(&mut self) {
        self.note_scroll();

        let size = self.size();
        if size != self.laid {
            self.resized(size);
        }

        // A scroll brought rows on screen that have no exact height yet.
        self.settle();
        if self.following && !self.at_end() {
            self.keep_end();
            request_frame();
        }
        self.seen = self.look();
        self.to_end.set_hidden(self.following);
    }
}

impl TableData for LogView {
    fn cell_height(&self, index: usize) -> f32 {
        self.heights.get(index).copied().unwrap_or(self.one_line)
    }

    /// The lines the view was told about. A line the app added and did
    /// not tell about is not shown.
    fn number_of_cells(&self) -> usize {
        self.heights.len()
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<LogCell>();
        let line = if index < self.line_count() {
            self.data.line(index)
        } else {
            LogLine::default()
        };
        cell.show(&line, &self.columns(), self.cell_height(index), &self.style);
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}
