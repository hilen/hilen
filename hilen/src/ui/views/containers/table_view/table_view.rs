use super::{
    layout::LayoutMode,
    rows::{Rows, row_offsets},
};
use crate::{
    self as hilen,
    deps::{
        netrun::Function,
        refs::{Own, Weak},
    },
    gm::{LossyConvert, ToF32, flat::Point},
    ui::{
        CellRegistry, ScrollView, Setup, TableData, UIEvent, View, ViewData, ViewFrame, ViewSubviews,
        ViewTouch, WeakView, struct_name, view,
    },
};

#[view]
pub struct TableView {
    pub(super) data: Weak<dyn TableData>,

    #[educe(Default = 1)]
    pub(super) columns: usize,

    pub(super) cell_spacing: f32,

    /// Rows read their own `cell_height(index)` instead of sharing row 0.
    pub(super) variable_heights: bool,

    /// Row offsets of a variable height table, rebuilt on a full layout.
    /// Empty for a uniform table, which needs no memory per row.
    pub(super) row_offsets: Vec<f32>,

    /// Rows pin to the viewport top per `TableData::is_sticky`.
    pub(super) sticky_enabled: bool,

    /// Cell indices the data marks sticky, rebuilt on a full layout.
    pub(super) sticky_rows: Vec<usize>,

    /// The sticky rows currently on screen: index, y in the table's own
    /// coordinates and height. Taps check them before the row geometry.
    pub(super) pinned: Vec<(usize, f32, f32)>,

    /// Cells drawn in front of the other cells while their row is
    /// pinned, lowered back when it unpins or recycles.
    pub(super) raised: Vec<(usize, WeakView)>,

    pub(super) header_height: f32,
    pub(super) header_views:  Vec<WeakView>,

    pub(super) registry: CellRegistry,

    #[init]
    pub(super) scroll: ScrollView,
}

impl Setup for TableView {
    fn setup(mut self: Weak<Self>) {
        let weak = self;
        self.registry.set_table(weak);
        self.scroll.place().back();

        self.scroll.on_scroll.sub(move || {
            self.layout_cells(LayoutMode::Scroll);
        });

        self.size_changed().sub(move || {
            self.layout_cells(LayoutMode::Resize);
        });

        self.enable_touch_low_priority();
        self.touch().up_inside.val(weak, move |touch| {
            self.select_at(touch.position);
        });
    }
}

impl TableView {
    pub fn set_data_source(mut self: Weak<Self>, data: Weak<dyn TableData>) -> Weak<Self> {
        self.data = data;
        self
    }

    pub fn register_cell<T: View + Default + 'static>(mut self: Weak<Self>) -> Weak<Self> {
        fn constr<T: Default + View + 'static>() -> impl FnMut() -> Own<dyn View> + Send + 'static {
            || T::new()
        }

        let mut func = constr::<T>();

        self.registry
            .constructors
            .insert(struct_name::<T>(), Function::new(move |()| func()));
        self
    }

    pub fn register_cell_id(
        mut self: Weak<Self>,
        id: &'static str,
        mut constructor: impl FnMut() -> Own<dyn View> + Send + 'static,
    ) -> Weak<Self> {
        self.registry.constructors.insert(id, Function::new(move |()| constructor()));
        self
    }

    pub fn reload_data(&mut self) {
        self.layout_cells(LayoutMode::Full);
    }

    pub fn set_columns(&mut self, columns: usize) -> &mut Self {
        self.columns = columns;
        self.layout_cells(LayoutMode::Full);
        self
    }

    pub fn set_cell_spacing(&mut self, spacing: impl ToF32) -> &mut Self {
        self.cell_spacing = spacing.to_f32();
        self.layout_cells(LayoutMode::Full);
        self
    }

    /// Every row gets its own height from `cell_height(index)`, the index
    /// of its first cell. Off by default, where `cell_height(0)` is the
    /// height of every row and the table never walks the data. Turning
    /// it on costs one `cell_height` call per row on every `reload_data`.
    pub fn set_variable_heights(&mut self, variable: bool) -> &mut Self {
        self.variable_heights = variable;
        self.layout_cells(LayoutMode::Full);
        self
    }

    /// Reserves space above the first row for header views. The header
    /// scrolls away with the content.
    pub fn set_header_height(&mut self, height: impl ToF32) -> &mut Self {
        self.header_height = height.to_f32();
        self.layout_cells(LayoutMode::Full);
        self
    }

    /// The view lives in the scroll content above the cells and scrolls
    /// away with them. Lay it out with `place()` rules, they are relative
    /// to the content top.
    pub fn add_header_view<T: View + Default + 'static>(&mut self) -> Weak<T> {
        let view = self.scroll.add_view::<T>();
        self.header_views.push(view.weak_view());
        view
    }

    pub fn bottom_reached(&self) -> &UIEvent {
        &self.scroll.bottom_reached
    }

    /// Scrolls all the way down so the last row is visible, the log tail
    /// case. The offset clamps against the current content size, so call
    /// it after `reload_data` when rows were appended.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll.set_content_offset(f32::MIN);
        self.layout_cells(LayoutMode::Scroll);
    }

    /// Sets the scroll position: 0 is the top, negative values scroll
    /// down. Clamped to the scrollable range on both ends, so call it
    /// after `reload_data` when the row set changed.
    pub fn set_content_offset(&mut self, offset: impl ToF32) -> &mut Self {
        self.scroll.set_content_offset(offset.to_f32().min(0.0));
        self.layout_cells(LayoutMode::Scroll);
        self
    }

    /// The current scroll position: 0 at the top, negative below it.
    pub fn content_offset(&self) -> f32 {
        self.scroll.get_scroll_content_offset()
    }

    /// Rows the data marks with `TableData::is_sticky` pin to the top of
    /// the viewport while their section scrolls by, and the next sticky
    /// row pushes the pinned one away. Off by default, turning it on
    /// walks `is_sticky` over the data on every `reload_data`.
    pub fn set_sticky_rows(&mut self, sticky: bool) -> &mut Self {
        self.sticky_enabled = sticky;
        self.layout_cells(LayoutMode::Full);
        self
    }
}

impl TableView {
    // The whole table maps a tap to a cell index, so a tap in a
    // spacing gap selects the nearest cell instead of dying on a
    // pixel gap between touch areas. Taps past the last row are
    // ignored.
    fn select_at(mut self: Weak<Self>, pos: Point) {
        if self.data.is_null() {
            return;
        }

        let number_of_cells = self.data.number_of_cells();

        if number_of_cells == 0 {
            return;
        }

        // A pinned sticky row covers whatever the row geometry has at its
        // position, so it takes the tap first.
        for (index, y, height) in self.pinned.clone() {
            if pos.y >= y && pos.y < y + height {
                self.data.cell_selected(index);
                return;
            }
        }

        let columns: f32 = self.columns.lossy_convert();
        let spacing = self.cell_spacing;
        let cell_width = (self.width() - spacing * (columns - 1.0)) / columns;

        let col = ((pos.x - cell_width / 2.0) / (cell_width + spacing))
            .round()
            .clamp(0.0, columns - 1.0);

        let y = pos.y - self.scroll.get_scroll_content_offset() - self.header_height;
        if y < 0.0 {
            return;
        }

        let rows = self.rows(number_of_cells);

        if y > rows.total() + spacing / 2.0 {
            return;
        }

        let row: f32 = rows.row_for_tap(y).lossy_convert();
        let index: usize = (row * columns + col).lossy_convert();

        if index >= number_of_cells {
            return;
        }

        self.data.cell_selected(index);
    }

    pub(super) fn row_count(&self, number_of_cells: usize) -> usize {
        number_of_cells.div_ceil(self.columns)
    }

    /// The row geometry for the current data. A variable table answers
    /// from the cached offsets, see `rebuild_row_offsets`.
    pub(super) fn rows(&self, number_of_cells: usize) -> Rows<'_> {
        if self.variable_heights {
            Rows::Variable {
                offsets: &self.row_offsets,
                spacing: self.cell_spacing,
            }
        } else {
            Rows::uniform(
                self.row_count(number_of_cells),
                self.data.cell_height(0),
                self.cell_spacing,
            )
        }
    }

    pub(super) fn rebuild_row_offsets(&mut self, number_of_cells: usize) {
        if !self.variable_heights {
            self.row_offsets.clear();
            return;
        }

        let columns = self.columns;
        let heights = (0..self.row_count(number_of_cells)).map(|row| self.data.cell_height(row * columns));

        self.row_offsets = row_offsets(heights, self.cell_spacing);
    }

    fn layout_cells(&mut self, mode: LayoutMode) {
        if self.height() <= 0.0 {
            return;
        }

        assert!(
            self.data.is_ok(),
            "TableView data source is not set. Use TableView::set_data_source method."
        );

        let number_of_cells = self.data.number_of_cells();

        if number_of_cells == 0 {
            return;
        }

        if matches!(mode, LayoutMode::Full) || (self.variable_heights && self.row_offsets.is_empty()) {
            self.rebuild_row_offsets(number_of_cells);
            self.sticky_rows = if self.sticky_enabled {
                (0..number_of_cells).filter(|i| self.data.is_sticky(*i)).collect()
            } else {
                Vec::new()
            };
        }

        self.layout_fixed_cells(number_of_cells, self.columns, mode);
    }
}

#[cfg(feature = "ui-tests")]
mod test {
    use std::ops::Deref;

    use anyhow::Result;
    use parking_lot::Mutex;

    use crate::{
        self as hilen,
        deps::{hreads::from_main, refs::Weak},
        gm::color::Color,
        ui::{CellRegistry, Label, Setup, TableData, TableView, View, ViewData, ViewTest, view},
        ui_test::{inject_scroll, inject_touches},
    };

    static TEST_DATA: Mutex<String> = Mutex::new(String::new());

    #[view]
    struct TableView2Test {
        #[init]
        table: TableView,
    }

    impl Setup for TableView2Test {
        fn setup(mut self: Weak<Self>) {
            self.table.place().back();
            self.table.set_data_source(self);
            self.table.register_cell::<Label>();
            self.table.reload_data();
        }
    }

    impl TableData for TableView2Test {
        fn cell_height(&self, _: usize) -> f32 {
            100.0
        }

        fn number_of_cells(&self) -> usize {
            100_000
        }

        fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
            let cell = registry.cell::<Label>();
            cell.set_text(index);
            cell.set_border_width(index % 20);
            cell.set_color(Color::ALL[index % Color::ALL.len()]);
            cell.set_border_color(Color::ALL[(index + 1) % Color::ALL.len()]);
            cell.set_corner_radius(index % 40);

            cell
        }

        #[allow(clippy::format_push_string)]
        fn cell_selected(&mut self, index: usize) {
            *TEST_DATA.lock() += &format!("|{index}|");
        }
    }

    impl ViewTest for TableView2Test {
        #[allow(clippy::too_many_lines)]
        fn perform_test(mut view: Weak<Self>) -> Result<()> {
            inject_touches(
                "
                    395  35   b
                    394  35   e
                    357  160  b
                    357  159  e
                    349  258  b
                    349  258  e
                    351  366  b
                    351  366  e
                    353  455  b
                    353  455  e
                    350  528  b
                    350  528  e
                ",
            );

            assert_eq!(TEST_DATA.lock().deref(), "|0||1||2||3||4||5|");

            TEST_DATA.lock().clear();

            for _ in 0..200 {
                inject_scroll(-20);
            }

            inject_scroll(-1000);

            inject_touches(
                "
                359  58   b
                359  58   e
                334  159  b
                334  159  e
                349  239  b
                349  239  e
                354  346  b
                353  345  e
                354  436  b
                353  435  e
                353  536  b
                353  536  e

            ",
            );

            assert_eq!(TEST_DATA.lock().deref(), "|50||51||52||53||54||55|");
            TEST_DATA.lock().clear();

            from_main(move || {
                view.table.set_columns(2);
            });

            for _ in 0..100 {
                inject_scroll(-20);
            }

            inject_scroll(-1000);

            inject_touches(
                "
                239  57   b
                239  57   e
                219  174  b
                219  174  e
                220  248  b
                220  248  e
                213  358  b
                213  358  e
                201  453  b
                200  453  e
                206  537  b
                206  537  e
                468  531  b
                468  531  e
                494  420  b
                494  420  e
                489  350  b
                489  350  e
                485  244  b
                485  244  e
                485  138  b
                485  138  e
                479  48   b
                479  48   e
            ",
            );

            assert_eq!(
                TEST_DATA.lock().deref(),
                "|160||162||164||166||168||170||171||169||167||165||163||161|"
            );
            TEST_DATA.lock().clear();

            inject_scroll(-100_000_000);
            inject_scroll(-100_000_000);
            inject_scroll(-100_000_000);
            inject_scroll(-100_000_000);

            inject_touches(
                "
                212  565  b
                212  565  e
                211  455  b
                210  455  e
                215  365  b
                215  365  e
                219  262  b
                219  262  e
                211  139  b
                211  139  e
                205  62   b
                205  62   e
                390  56   b
                390  56   e
                380  144  b
                380  144  e
                382  264  b
                382  264  e
                370  351  b
                370  351  e
                372  432  b
                371  432  e
                396  569  b
                396  569  e

            ",
            );

            assert_eq!(
                TEST_DATA.lock().deref(),
                "|99998||99996||99994||99992||99990||99988||99989||99991||99993||99995||99997||99999|"
            );
            TEST_DATA.lock().clear();

            // crate::ui_test::record_ui_test();
            Ok(())
        }
    }
}
