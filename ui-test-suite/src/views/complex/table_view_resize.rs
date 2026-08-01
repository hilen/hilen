use anyhow::Result;
use test_engine::{
    dispatch::from_main,
    refs::Weak,
    ui::{GREEN, Label, Setup, TableData, TableView, View, ViewData, ViewFrame, ViewTest, view},
    ui_test::{check_colors, inject_scroll},
};

#[view]
struct TableViewResize {
    #[init]
    table: TableView,
}

impl Setup for TableViewResize {
    fn setup(self: Weak<Self>) {
        self.table.set_frame((20, 20, 200, 200));
        self.table.set_data_source(self).register_cell::<Label>();
    }
}

impl TableData for TableViewResize {
    fn cell_height(&self, _: usize) -> f32 {
        50.0
    }

    fn number_of_cells(&self) -> usize {
        1
    }

    fn setup_cell(&mut self, _index: usize, registry: &mut test_engine::ui::CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_color(GREEN);
        cell.set_text("alalalalal");
        cell
    }
}

fn check_initial_cell() -> Result<()> {
    check_colors(
        r"
             448    4 - #597c95
              24   24 - #00ff00
              52   24 - #00ff00
              80   24 - #00ff00
             152   24 - #00ff00
             184   24 - #00ff00
             216   24 - #00ff00
             592   36 - #597c95
             116   40 - #00ff00
             200   44 - #00ff00
              64   48 - #00ff00
              92   48 - #00ff00
             116   48 - #00ff00
             140   48 - #00ff00
             164   48 - #00ff00
              24   68 - #00ff00
              48   68 - #00ff00
             128   68 - #00ff00
             152   68 - #00ff00
             180   68 - #00ff00
             208   68 - #00ff00
             592  184 - #597c95
             372  204 - #597c95
             200  248 - #597c95
              44  332 - #597c95
             516  388 - #597c95
             300  404 - #597c95
             148  448 - #597c95
             404  564 - #597c95
               4  592 - #597c95
             212  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn check_cell_after_scroll() -> Result<()> {
    for i in 0..5 {
        inject_scroll(i);
    }

    check_colors(
        r"
             448    4 - #597c95
              24   24 - #00ff00
              52   24 - #00ff00
              80   24 - #00ff00
             152   24 - #00ff00
             184   24 - #00ff00
             216   24 - #00ff00
             592   36 - #597c95
             116   40 - #00ff00
             200   44 - #00ff00
              64   48 - #00ff00
              92   48 - #00ff00
             116   48 - #00ff00
             140   48 - #00ff00
             164   48 - #00ff00
              24   68 - #00ff00
              48   68 - #00ff00
             128   68 - #00ff00
             152   68 - #00ff00
             180   68 - #00ff00
             208   68 - #00ff00
             592  184 - #597c95
             372  204 - #597c95
             200  248 - #597c95
              44  332 - #597c95
             516  388 - #597c95
             300  404 - #597c95
             148  448 - #597c95
             404  564 - #597c95
               4  592 - #597c95
             212  592 - #597c95
             592  592 - #597c95
        ",
    )
}

fn check_resized_table(view: Weak<TableViewResize>) -> Result<()> {
    from_main(move || {
        view.table.set_size(400, 100);
    });

    check_colors(
        r"
              24   24 - #00ff00
             116   24 - #00ff00
             312   24 - #00ff00
             364   24 - #00ff00
             416   24 - #00ff00
              68   28 - #00ff00
             216   40 - #00ff00
             164   48 - #00ff00
             192   48 - #00ff00
             216   48 - #00ff00
             240   48 - #00ff00
             264   48 - #00ff00
             372   52 - #00ff00
              40   68 - #00ff00
              88   68 - #00ff00
             128   68 - #00ff00
             300   68 - #00ff00
             344   68 - #00ff00
             396   68 - #00ff00
             592   92 - #597c95
              56  200 - #597c95
             184  240 - #597c95
             412  252 - #597c95
             592  280 - #597c95
               8  324 - #597c95
             300  396 - #597c95
             524  436 - #597c95
             132  444 - #597c95
             404  556 - #597c95
               4  592 - #597c95
             216  592 - #597c95
             592  592 - #597c95
        ",
    )
}

impl ViewTest for TableViewResize {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_initial_cell()?;
        check_cell_after_scroll()?;
        check_resized_table(view)?;

        for i in 0..5 {
            inject_scroll(-i);
        }

        Ok(())
    }
}
