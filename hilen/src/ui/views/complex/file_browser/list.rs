use super::{
    cells::{Columns, GRID_CELL_HEIGHT, GRID_CELL_WIDTH, GridCell, LIST_ROW_HEIGHT, ListCell, Row},
    look::{BACKGROUND, BORDER, DIM_TEXT, ERROR_TEXT, HOVER, RADIUS, SMALL_TEXT_SIZE, TEXT, TEXT_SIZE},
    model::{FileSort, SortKey},
};
use crate::{
    self as hilen,
    deps::{refs::Weak, vents::Event},
    gm::{
        LossyConvert,
        color::CLEAR,
        flat::{Point, Rect},
    },
    ui::{
        Button, CellRegistry, Container, DynamicColor, ImageView, Label, Setup, TableData, TableView,
        TextAlignment, UIImages, View, ViewCallbacks, ViewData, ViewFrame, ViewTouch, view,
    },
};

const HEADER_HEIGHT: f32 = 26.0;
const MARGIN_LEFT: f32 = 6.0;
/// Clear of the scroll bar of the table, which draws over the cells.
const MARGIN_RIGHT: f32 = 10.0;
const GRID_SPACING: f32 = 4.0;
const ARROW: f32 = 12.0;

/// What the list area shows in place of rows.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) enum ListStatus {
    #[default]
    Rows,
    Loading,
    /// A folder with nothing in it, or nothing the filter lets through.
    Empty(String),
    Failed(String),
}

/// The title of one column. A tap sorts by it.
#[view]
pub(super) struct HeaderCell {
    key: SortKey,

    pub tapped: Event<SortKey>,

    #[init]
    title: Label,
    arrow: ImageView,
}

impl HeaderCell {
    fn set_key(mut self: Weak<Self>, key: SortKey, title: &str, right_aligned: bool) {
        self.key = key;
        self.title.set_text(title);
        if right_aligned {
            self.title.set_alignment(TextAlignment::Right);
            self.title.place().clear().tb(0).l(ARROW + 2.0).r(0);
            self.arrow.place().clear().l(0).center_y().size(ARROW, ARROW);
        }
    }

    /// The arrow of the sorted column points the way the rows run.
    fn set_sort(&self, sort: FileSort) {
        let sorted = sort.key == self.key;
        self.arrow.set_hidden(!sorted);
        if sorted {
            self.arrow.set_image(if sort.ascending {
                UIImages::chevron_up()
            } else {
                UIImages::chevron_down()
            });
        }
        let color: DynamicColor = if sorted { TEXT } else { DIM_TEXT };
        self.title.set_text_color(color);
    }
}

impl Setup for HeaderCell {
    fn setup(self: Weak<Self>) {
        self.set_color(CLEAR);

        self.title
            .set_color(CLEAR)
            .set_text_size(SMALL_TEXT_SIZE)
            .set_alignment(TextAlignment::Left)
            .set_text_color(DIM_TEXT);
        self.title.place().tb(0).l(0).r(ARROW + 2.0);

        self.arrow.place().r(0).center_y().size(ARROW, ARROW);
        self.arrow.set_hidden(true);

        self.enable_touch();
        self.touch().up_inside.sub(self, move || self.tapped.trigger(self.key));
    }
}

/// The entries of the open folder, as a list with columns or as a grid
/// of icons. It owns no state of the browser, it draws the rows it is
/// given and reports taps by row index.
#[view]
pub(super) struct FileList {
    rows:   Vec<Row>,
    grid:   bool,
    sort:   FileSort,
    status: ListStatus,

    pub row_tapped:    Event<usize>,
    /// A right click or a long press, on a row or on the empty area.
    pub row_secondary: Event<Option<usize>>,
    pub empty_tapped:  Event,
    pub sort_tapped:   Event<SortKey>,
    pub retry:         Event,

    #[init]
    header:       Container,
    name_header:  HeaderCell,
    size_header:  HeaderCell,
    kind_header:  HeaderCell,
    date_header:  HeaderCell,
    header_line:  Container,
    table:        TableView,
    message:      Label,
    retry_button: Button,
}

impl FileList {
    pub(super) fn set_rows(mut self: Weak<Self>, rows: Vec<Row>, status: ListStatus) {
        self.rows = rows;
        self.status = status;
        self.refresh_status();
        self.table.reload_data();
    }

    pub(super) fn message(&self) -> &str {
        self.message.text()
    }

    pub(super) fn set_sort(mut self: Weak<Self>, sort: FileSort) {
        self.sort = sort;
        for header in self.headers() {
            header.set_sort(sort);
        }
    }

    pub(super) fn set_grid(mut self: Weak<Self>, grid: bool) {
        if self.grid == grid {
            return;
        }
        self.grid = grid;
        self.table.set_content_offset(0);
        self.layout();
    }

    pub(super) fn is_grid(&self) -> bool {
        self.grid
    }

    /// A newly opened folder starts at its first row.
    pub(super) fn scroll_to_top(mut self: Weak<Self>) {
        self.table.set_content_offset(0);
    }

    /// How many cells one row of the grid holds, 1 for the list.
    pub(super) fn columns_per_row(&self) -> usize {
        if !self.grid {
            return 1;
        }
        let room = self.width() - MARGIN_LEFT - MARGIN_RIGHT + GRID_SPACING;
        let count: usize = (room / (GRID_CELL_WIDTH + GRID_SPACING)).floor().max(1.0).lossy_convert();
        count
    }

    /// Scrolls just enough to bring the row of `index` fully into view.
    pub(super) fn reveal(mut self: Weak<Self>, index: usize) {
        let row: f32 = (index / self.columns_per_row()).lossy_convert();
        let (height, spacing) = self.cell_metrics();
        let top = row * (height + spacing);
        let bottom = top + height;
        let offset = -self.table.content_offset();
        let visible = self.table.height();

        if top < offset {
            self.table.set_content_offset(-top);
        } else if bottom > offset + visible {
            self.table.set_content_offset(-(bottom - visible));
        }
    }

    /// The middle of the cell of `index` in the points of the window, for
    /// a test that taps a row. `None` while the row is scrolled away.
    pub(super) fn cell_center(&self, index: usize) -> Option<Point> {
        let cells = self.table.visible_cells();
        let (_, cell) = cells.into_iter().find(|(cell_index, _)| *cell_index == index)?;
        Some(cell.absolute_frame().center())
    }

    pub(super) fn header(&self, key: SortKey) -> Weak<HeaderCell> {
        match key {
            SortKey::Name => self.name_header,
            SortKey::Size => self.size_header,
            SortKey::Kind => self.kind_header,
            SortKey::Modified => self.date_header,
        }
    }

    pub(super) fn retry_button(&self) -> Weak<Button> {
        self.retry_button
    }

    /// The frame of the rows in the points of the window, a press inside
    /// it gives the list the keys.
    pub(super) fn rows_frame(&self) -> Rect {
        *self.table.absolute_frame()
    }

    fn headers(&self) -> [Weak<HeaderCell>; 4] {
        [
            self.name_header,
            self.size_header,
            self.kind_header,
            self.date_header,
        ]
    }

    fn cell_metrics(&self) -> (f32, f32) {
        if self.grid {
            (GRID_CELL_HEIGHT, GRID_SPACING)
        } else {
            (LIST_ROW_HEIGHT, 0.0)
        }
    }

    fn list_columns(&self) -> Columns {
        Columns::for_width(self.width() - MARGIN_LEFT - MARGIN_RIGHT)
    }

    fn refresh_status(&self) {
        let (text, failed) = match &self.status {
            ListStatus::Rows => (String::new(), false),
            ListStatus::Loading => ("Loading".to_string(), false),
            ListStatus::Empty(text) => (text.clone(), false),
            ListStatus::Failed(error) => (error.clone(), true),
        };

        self.message.set_hidden(text.is_empty());
        self.message.set_text(text);
        let color: DynamicColor = if failed { ERROR_TEXT } else { DIM_TEXT };
        self.message.set_text_color(color);
        self.retry_button.set_hidden(!failed);
    }

    fn layout(mut self: Weak<Self>) {
        let width = self.width();
        let height = self.height();
        if width <= 0.0 || height <= 0.0 {
            return;
        }

        let header_height = if self.grid { 0.0 } else { HEADER_HEIGHT };
        self.header.set_hidden(self.grid);
        self.header_line.set_hidden(self.grid);
        self.header.set_frame((0.0, 0.0, width, header_height));
        self.header_line.set_frame((0.0, header_height - 1.0, width, 1.0));
        self.table.set_frame((0.0, header_height, width, height - header_height));

        let columns = self.list_columns();
        let grid = self.grid;
        let place = |header: Weak<HeaderCell>, column: Option<(f32, f32)>| {
            header.set_hidden(grid || column.is_none());
            if let Some((x, column_width)) = column {
                header.set_frame((MARGIN_LEFT + x, 0.0, column_width, HEADER_HEIGHT));
            }
        };
        place(self.name_header, Some(columns.name));
        place(self.size_header, columns.size);
        place(self.kind_header, columns.kind);
        place(self.date_header, columns.date);

        self.message
            .set_frame((16.0, header_height + 28.0, (width - 32.0).max(0.0), 40.0));
        self.retry_button
            .set_frame(((width - 96.0) / 2.0, header_height + 76.0, 96.0, 30.0));

        let per_row = self.columns_per_row();
        let (_, spacing) = self.cell_metrics();
        self.table.set_cell_spacing(spacing);
        self.table.set_columns(per_row);
    }

    fn secondary_at(self: Weak<Self>, position: Point) {
        self.row_secondary.trigger(self.table.index_at(position));
    }
}

impl TableData for FileList {
    fn cell_height(&self, _: usize) -> f32 {
        self.cell_metrics().0
    }

    fn number_of_cells(&self) -> usize {
        self.rows.len()
    }

    fn cell_selected(&mut self, index: usize) {
        self.row_tapped.trigger(index);
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let row = &self.rows[index];
        if self.grid {
            let cell = registry.cell::<GridCell>();
            cell.fill(row);
            cell
        } else {
            let cell = registry.cell::<ListCell>();
            cell.fill(row, self.list_columns());
            cell
        }
    }
}

impl ViewCallbacks for FileList {
    /// The recycled cells hold icons tinted for the old theme.
    fn theme_changed(&mut self) {
        self.table.reload_data();
    }
}

impl Setup for FileList {
    fn setup(mut self: Weak<Self>) {
        self.set_color(BACKGROUND);

        self.header.set_color(BACKGROUND);
        self.header_line.set_color(BORDER);

        self.name_header.set_key(SortKey::Name, "Name", false);
        self.size_header.set_key(SortKey::Size, "Size", true);
        self.kind_header.set_key(SortKey::Kind, "Kind", false);
        self.date_header.set_key(SortKey::Modified, "Date Modified", false);
        for header in self.headers() {
            header.tapped.val(move |key| self.sort_tapped.trigger(key));
        }
        let sort = self.sort;
        self.set_sort(sort);

        self.table.set_color(CLEAR);
        self.table
            .set_data_source(self)
            .register_cell::<ListCell>()
            .register_cell::<GridCell>();
        self.table.set_cell_margins(MARGIN_LEFT, MARGIN_RIGHT);
        self.table.set_footer_height(8);

        // The table reports a tap on a row itself. A tap under the last
        // row is no row, it clears the selection.
        self.table.touch().up_inside.val(self, move |touch| {
            if self.table.index_at(touch.position).is_none() {
                self.empty_tapped.trigger(());
            }
        });
        self.table
            .touch()
            .secondary
            .val(self, move |touch| self.secondary_at(touch.position));

        self.message
            .set_color(CLEAR)
            .set_text_size(TEXT_SIZE)
            .set_alignment(TextAlignment::Center)
            .set_multiline(true);
        self.message.set_hidden(true);

        self.retry_button
            .set_text("Try again")
            .set_text_size(TEXT_SIZE)
            .set_text_color(TEXT);
        self.retry_button.set_color(HOVER).set_corner_radius(RADIUS);
        self.retry_button.set_hidden(true);
        self.retry_button.on_tap(move || self.retry.trigger(()));

        self.size_changed().sub(move || self.layout());
    }
}
