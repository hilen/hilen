use netrun::Function;
use refs::{Own, Weak};

use super::layout::LayoutMode;
use crate::{
    self as test_engine,
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

        let columns: f32 = self.columns.lossy_convert();
        let cell_height = self.data.cell_height(0);
        let spacing = self.cell_spacing;
        let cell_width = (self.width() - spacing * (columns - 1.0)) / columns;

        let col = ((pos.x - cell_width / 2.0) / (cell_width + spacing))
            .round()
            .clamp(0.0, columns - 1.0);

        let y = pos.y - self.scroll.get_scroll_content_offset() - self.header_height;
        if y < 0.0 {
            return;
        }
        let row = ((y - cell_height / 2.0) / (cell_height + spacing)).round().max(0.0);

        let index: usize = (row * columns + col).lossy_convert();

        if index >= number_of_cells {
            return;
        }

        self.data.cell_selected(index);
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

        self.layout_fixed_cells(number_of_cells, self.columns, mode);
    }
}

mod test {
    use std::ops::Deref;

    use anyhow::Result;
    use hreads::from_main;
    use parking_lot::Mutex;
    use refs::Weak;

    use crate::{
        self as test_engine,
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
