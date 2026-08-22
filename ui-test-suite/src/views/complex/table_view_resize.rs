use anyhow::Result;
use hilen::{
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

    fn setup_cell(&mut self, _index: usize, registry: &mut hilen::ui::CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_color(GREEN);
        cell.set_text("alalalalal");
        cell
    }
}

fn check_initial_cell() -> Result<()> {
    check_colors(
        r"
             116   20 - #00ff00
             216   20 - #00ff00
              20   24 - #00ff00
             104   36 - #000800
             128   36 - #000100
             152   36 - #007000
             180   36 - #007800
              68   40 - #000000
              92   40 - #000000
             140   40 - #000000
             164   40 - #000100
              84   44 - #00ff00
             112   44 - #00ff00
             128   44 - #000100
             160   44 - #00ff00
             180   44 - #007800
              96   48 - #000100
             172   48 - #002800
              64   52 - #00ff00
             104   52 - #000800
             116   52 - #00ff00
             128   52 - #000100
             140   52 - #00ff00
             152   52 - #007000
             180   52 - #007800
              20   68 - #00ff00
             216   68 - #00ff00
             592  180 - #597c95
              16  320 - #597c95
             296  384 - #597c95
               4  592 - #597c95
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
             116   20 - #00ff00
             216   20 - #00ff00
              20   24 - #00ff00
             104   36 - #000800
             128   36 - #000100
             152   36 - #007000
             180   36 - #007800
              68   40 - #000000
              92   40 - #000000
             140   40 - #000000
             164   40 - #000100
              84   44 - #00ff00
             112   44 - #00ff00
             128   44 - #000100
             160   44 - #00ff00
             180   44 - #007800
              96   48 - #000100
             172   48 - #002800
              64   52 - #00ff00
             104   52 - #000800
             116   52 - #00ff00
             128   52 - #000100
             140   52 - #00ff00
             152   52 - #007000
             180   52 - #007800
              20   68 - #00ff00
             216   68 - #00ff00
             592  180 - #597c95
              16  320 - #597c95
             296  384 - #597c95
               4  592 - #597c95
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
             116   20 - #00ff00
             416   20 - #00ff00
              20   24 - #00ff00
             204   36 - #000800
             228   36 - #000100
             252   36 - #007000
             280   36 - #007800
             168   40 - #000000
             192   40 - #000000
             216   40 - #000100
             268   40 - #000100
             184   44 - #00ff00
             236   44 - #00ff00
             260   44 - #00ff00
             280   44 - #007800
             204   48 - #000800
             216   48 - #00ff00
             228   48 - #000100
             272   48 - #002800
             164   52 - #00ff00
             196   52 - #000000
             220   52 - #000000
             240   52 - #00ff00
             252   52 - #007000
             280   52 - #007800
              88   68 - #00ff00
             352   68 - #00ff00
             592  280 - #597c95
               8  324 - #597c95
             300  400 - #597c95
               4  592 - #597c95
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
