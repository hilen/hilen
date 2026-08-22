use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{
        Button, CellRegistry, Container, Label, Setup, TableData, TableView, TouchStack, View, ViewData,
        ViewSubviews, ViewTest, view,
    },
    ui_test::{get_str_state, inject_touches, state::append_state},
};

#[view]
struct ButtonsOnTableView {
    #[init]
    table: TableView,
}

impl Setup for ButtonsOnTableView {
    fn setup(self: Weak<Self>) {
        self.table.place().back();
        self.table.set_data_source(self).register_cell::<Container>();
    }
}

impl TableData for ButtonsOnTableView {
    fn number_of_cells(&self) -> usize {
        1
    }

    fn cell_height(&self, _: usize) -> f32 {
        50.0
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let mut cell = registry.cell::<Container>();
        cell.add_view::<Button>()
            .set_image("plus.png")
            .place()
            .size(40, 40)
            .center_y()
            .r(20);

        cell.add_view::<Label>().place().size(100, 40).center_y().l(20);

        cell.get_subview::<Label>().set_text(format!("{index}"));
        cell.get_subview::<Button>().on_tap(move || {
            append_state(format!("button_pressed: {index}\n"));
        });
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        append_state(format!("cell_selected: {index}\n"));
    }
}

impl ViewTest for ButtonsOnTableView {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        assert_eq!(
            TouchStack::dump(),
            vec![vec![
                "Layer: Root view".to_string(),
                "ButtonsOnTableView.table: TableView".to_string(),
                "Button".to_string(),
            ]],
        );

        inject_touches(
            "
            218  146  b
            218  146  e
            218  144  b
            218  144  e
            218  143  b
            218  143  e
            86   26   b
            86   26   e
            85   26   b
            85   26   e
            84   26   b
            84   26   e
            324  36   b
            324  36   e
            324  36   b
            324  36   e
            324  36   b
            324  36   e
            554  34   b
            554  34   e
            554  34   b
            554  34   e
            554  34   b
            554  34   e
        ",
        );

        assert_eq!(
            get_str_state(),
            r"cell_selected: 0
cell_selected: 0
cell_selected: 0
cell_selected: 0
cell_selected: 0
cell_selected: 0
button_pressed: 0
button_pressed: 0
button_pressed: 0
"
        );

        Ok(())
    }
}
