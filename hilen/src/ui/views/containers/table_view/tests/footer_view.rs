use std::ops::Deref;

use anyhow::Result;
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    gm::color::{Color, WHITE},
    ui::{
        Button, CellRegistry, Container, Label, Setup, TableData, TableView, View, ViewData, ViewFrame,
        ViewSubviews, ViewTest, view,
    },
    ui_test::{check_colors, inject_touches},
};

static SELECTED: Mutex<String> = Mutex::new(String::new());

const ROW_H: f32 = 100.0;
const TALL_ROW_H: f32 = 150.0;
const SPACING: f32 = 10.0;
const FOOTER_H: f32 = 80.0;

// A footer view with a label and a button under the rows. Every step
// changes the rows, the footer has to sit right under the last one, and
// its button has to take a tap there while the table selects no cell.
#[view]
struct TableFooterView {
    footer: Weak<Container>,
    button: Weak<Button>,
    taps:   usize,
    count:  usize,
    tall:   bool,

    #[init]
    table: TableView,
}

impl Setup for TableFooterView {
    fn setup(mut self: Weak<Self>) {
        self.count = 3;
        self.table.place().tl(0).size(600, 600);
        self.table
            .set_data_source(self)
            .register_cell::<Label>()
            .set_cell_spacing(SPACING)
            .set_cell_margins(SPACING, SPACING)
            .set_footer_height(FOOTER_H);

        self.footer = self.table.add_footer_view::<Container>();
        self.footer.set_color(Color::hex("#264653"));

        let label = self.footer.add_view::<Label>();
        label
            .set_text("footer")
            .set_text_size(24)
            .set_text_color(WHITE)
            .set_color(Color::hex("#264653"));
        label.place().l(10).center_y().size(160, 40);

        self.button = self.footer.add_view::<Button>();
        self.button.set_text("tap").set_color(Color::hex("#e76f51"));
        self.button.place().r(10).center_y().size(120, 40);
        self.button.on_tap(move || {
            self.taps += 1;
            let taps = self.taps;
            self.button.set_text(format!("taps {taps}"));
        });

        self.table.reload_data();
    }
}

impl TableData for TableFooterView {
    fn cell_height(&self, index: usize) -> f32 {
        if self.tall && index.is_multiple_of(4) {
            TALL_ROW_H
        } else {
            ROW_H
        }
    }

    fn number_of_cells(&self) -> usize {
        self.count
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        cell.set_text(format!("cell {index}"));
        cell.set_text_size(20);
        cell.set_text_color(Color::hex("#20262c"));
        cell.set_color(if index.is_multiple_of(2) {
            Color::hex("#f1faee")
        } else {
            Color::hex("#e9c46a")
        });
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        *SELECTED.lock() += &format!("|{index}|");
    }
}

fn footer_top(view: Weak<TableFooterView>) -> f32 {
    from_main(move || {
        assert_eq!(view.footer.width(), 600.0);
        assert_eq!(view.footer.height(), FOOTER_H);
        view.footer.frame().origin.y
    })
}

// Taps the button of the footer and the label next to it, both at `y`
// of the screen. Only the button counts, no cell is selected.
fn tap_footer(view: Weak<TableFooterView>, y: f32, taps: usize) {
    inject_touches(format!(
        "
            540  {y}  b
            540  {y}  e
            80   {y}  b
            80   {y}  e
        "
    ));
    assert_eq!(SELECTED.lock().deref(), "");
    from_main(move || assert_eq!(view.taps, taps));
}

impl ViewTest for TableFooterView {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        SELECTED.lock().clear();

        // 3 rows of 100 with 2 gaps of 10 end at 320, the footer starts
        // there with the rest of the table empty under it.
        assert_eq!(footer_top(view), 320.0);
        tap_footer(view, 360.0, 1);
        check_colors(FEW)?;

        // 20 rows end at 2190. At the bottom the footer fills the last
        // 80 points of the table.
        from_main(move || {
            view.count = 20;
            view.table.reload_data();
            view.table.scroll_to_bottom();
        });
        assert_eq!(footer_top(view), 2190.0);
        from_main(move || assert_eq!(view.table.content_offset(), -1670.0));
        tap_footer(view, 560.0, 2);
        check_colors(MANY)?;

        // 2 columns make 10 rows, which end at 1090.
        from_main(move || {
            view.table.set_columns(2);
            view.table.scroll_to_bottom();
        });
        assert_eq!(footer_top(view), 1090.0);
        tap_footer(view, 560.0, 3);
        check_colors(COLUMNS)?;

        // 4 new cells at the end make 12 rows, the footer moves to 1310.
        from_main(move || {
            view.count = 24;
            view.table.load_new_cells();
        });
        assert_eq!(footer_top(view), 1310.0);

        // Every other row is 150 tall now, 6 of them, so the rows end at
        // 6 * 150 + 6 * 100 + 11 * 10 = 1610.
        from_main(move || {
            view.tall = true;
            view.table.set_variable_heights(true);
        });
        assert_eq!(footer_top(view), 1610.0);

        // No rows, the footer is the only content, at the top.
        from_main(move || {
            view.count = 0;
            view.table.reload_data();
        });
        assert_eq!(footer_top(view), 0.0);
        tap_footer(view, 40.0, 4);
        check_colors(EMPTY)?;

        Ok(())
    }
}

const FEW: &str = r"
              12    4 - #f1faee
             592    4 - #597c95
             304   44 - #20262c
             280   52 - #f1faee
             132   96 - #f1faee
             280  160 - #e9c46a
             292  160 - #8f7d4e
             292  164 - #e9c46a
             304  164 - #20262c
             488  172 - #e9c46a
              12  204 - #e9c46a
             300  272 - #2d3338
             316  276 - #4b5154
             320  276 - #4b5154
             472  340 - #e76f51
             588  340 - #e76f51
              60  356 - #ffffff
             516  356 - #010000
             564  356 - #a24e39
              60  360 - #8e9fa5
             108  360 - #677d86
             564  360 - #a24e39
              60  364 - #8e9fa5
             524  364 - #e76f51
             540  364 - #e76f51
             500  368 - #010000
             508  368 - #000000
             564  368 - #a24e39
             516  372 - #000000
             240  396 - #264653
             364  396 - #264653
             392  592 - #597c95
";
const MANY: &str = r"
               4    4 - #597c95
             504    4 - #e9c46a
             324   24 - #b79c5a
             276   28 - #e9c46a
             300  140 - #d4ddd3
             588  164 - #f1faee
              12  228 - #e9c46a
             312  252 - #21272c
             520  348 - #f1faee
             300  356 - #d4ddd3
             324  356 - #f1faee
             300  364 - #d4ddd3
               4  396 - #597c95
             312  464 - #20262c
             276  472 - #e9c46a
             308  520 - #264653
             472  540 - #e76f51
             564  548 - #000000
              60  556 - #ffffff
             516  556 - #010000
              60  560 - #8e9fa5
              84  560 - #264653
             108  560 - #677d86
              60  564 - #8e9fa5
             112  564 - #264653
             524  564 - #e76f51
             540  564 - #e76f51
             500  568 - #010000
             564  568 - #793a2b
             568  568 - #793a2b
             472  576 - #e76f51
             220  592 - #264653
";
const COLUMNS: &str = r"
              12    4 - #f1faee
             152   24 - #6b7272
             460   24 - #21272c
             472   24 - #81724a
             472   32 - #81724a
             268   80 - #597c95
             128  140 - #f1faee
             420  140 - #21272c
             588  140 - #e9c46a
             472  244 - #b79c5a
             164  252 - #777e7d
             304  280 - #597c95
              12  292 - #f1faee
             472  352 - #958250
             424  360 - #e9c46a
             152  364 - #6b7272
             424  472 - #e9c46a
             308  516 - #e9c46a
             472  540 - #e76f51
             588  540 - #e76f51
             516  556 - #010000
              60  560 - #8e9fa5
              68  560 - #fefefe
              88  560 - #264653
             568  560 - #000000
              60  564 - #8e9fa5
             112  564 - #264653
             524  564 - #e76f51
             540  564 - #e76f51
             500  568 - #010000
             224  592 - #264653
             392  592 - #264653
";
const EMPTY: &str = r"
             308    4 - #264653
             472   20 - #e76f51
             588   20 - #e76f51
             568   28 - #000000
              96   36 - #ffffff
             516   36 - #010000
              60   40 - #8e9fa5
              72   40 - #264653
              84   40 - #264653
             100   40 - #264653
             108   40 - #677d86
             116   40 - #ffffff
             500   40 - #e76f51
             528   40 - #000000
             564   40 - #e76f51
              60   44 - #8e9fa5
              96   44 - #ffffff
             112   44 - #264653
             116   44 - #fefefe
             520   44 - #e76f51
             528   44 - #000000
             540   44 - #e76f51
             500   48 - #010000
             508   48 - #000000
             516   52 - #000000
             472   56 - #e76f51
             224   76 - #264653
             384   76 - #264653
             304  328 - #597c95
             592  392 - #597c95
               4  396 - #597c95
             304  592 - #597c95
";
