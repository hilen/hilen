use anyhow::Result;

use crate::{
    self as hilen,
    deps::refs::Weak,
    gm::{
        color::{BLACK, TURQUOISE},
        test_state::TestState,
    },
    ui::{
        CellCallbacks, CellRegistry, Container, Label, Setup, Spinner, SpinnerLockOnView, TableData,
        TableView, View, ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{inject_scroll, inject_touches},
};

static DATA: TestState = TestState::new();

#[view]
struct LoadingCell {
    spin: SpinnerLockOnView,

    #[init]
    spin_container: Container,
    label:          Label,
}

impl Setup for LoadingCell {
    fn setup(self: Weak<Self>) {
        self.label
            .set_color(TURQUOISE)
            .set_text_size(20)
            .set_corner_radius(20)
            .set_border_width(10)
            .set_border_color(BLACK)
            .place()
            .all_sides(4)
            .l(60);

        self.spin_container.place().center_y().l(5).size(50, 50);
    }
}

impl CellCallbacks for LoadingCell {
    fn cell_added(&mut self) {
        DATA.add(format!("ADD_{}", self.tag()));
        self.spin = Spinner::start_on(self.spin_container);
    }

    fn cell_removed(&mut self) {
        DATA.add(format!("REM_{}", self.tag()));
        self.spin = SpinnerLockOnView::default();
    }
}

#[view]
pub(crate) struct LoadingCellsTest {
    pub(super) test_string: String,

    #[init]
    pub table: TableView,
}

impl Setup for LoadingCellsTest {
    fn setup(mut self: Weak<Self>) {
        self.table.columns = 4;
        self.table
            .set_data_source(self)
            .register_cell::<LoadingCell>()
            .set_size(500, 400);
        self.table.reload_data();
    }
}

impl TableData for LoadingCellsTest {
    fn cell_height(&self, _index: usize) -> f32 {
        80.0
    }

    fn number_of_cells(&self) -> usize {
        10_000
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<LoadingCell>();
        cell.label.set_text(format!("{index} {}", cell.raw().addr()));
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        #[allow(clippy::format_push_string)]
        self.test_string.push_str(&format!("|{index}|"));
    }
}

impl ViewTest for LoadingCellsTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        inject_touches(
            "
               166  70   b
               166  70   e
           ",
        );

        assert_eq!(
            DATA,
            "|ADD_0||ADD_1||ADD_2||ADD_3||ADD_4||ADD_5||ADD_6||ADD_7||ADD_8||ADD_9||ADD_10||ADD_11||ADD_12||ADD_13||ADD_14||ADD_15||ADD_16||ADD_17||ADD_18||ADD_19||ADD_20||ADD_21||ADD_22||ADD_23||ADD_24||ADD_25||ADD_26||ADD_27|"
        );

        for _ in 0..15 {
            inject_scroll(-5);
            assert_eq!(DATA, "");
        }

        inject_scroll(-20);
        assert_eq!(
            DATA,
            "|REM_0||REM_1||REM_2||REM_3||ADD_28||ADD_29||ADD_30||ADD_31|"
        );

        inject_scroll(-100);

        assert_eq!(
            DATA,
            "|REM_4||REM_5||REM_6||REM_7||ADD_32||ADD_33||ADD_34||ADD_35|"
        );

        inject_scroll(-100);

        assert_eq!(
            DATA,
            "|REM_8||REM_9||REM_10||REM_11||ADD_36||ADD_37||ADD_38||ADD_39|"
        );

        // crate::ui_test::record_ui_test();

        Ok(())
    }
}
