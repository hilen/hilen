use std::mem::take;

use anyhow::{Result, ensure};
use parking_lot::Mutex;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{BLACK, Color, WHITE},
    ui::{
        CellRegistry, Container, Label, Setup, TableData, TableView, TextAlignment, VerticalAlignment, View,
        ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, inject_scroll, inject_touches},
};

/// The rows `cell_height` was asked for, in the order of the calls.
static ASKED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
/// The names of the rows `setup_cell` was called for.
static SET_UP: Mutex<Vec<usize>> = Mutex::new(Vec::new());

const SPACING: f32 = 4.0;
const TABLE_WIDTH: f32 = 400.0;
const TABLE_HEIGHT: f32 = 600.0;
const ROWS: usize = 40;
const DROPPED: usize = 6;

/// The height of the row with this name, some rows are taller.
fn height_of(name: usize) -> f32 {
    if name.is_multiple_of(3) { 70.0 } else { 40.0 }
}

/// A row on screen: its name, where it is on the screen and its height.
type Shown = Vec<(usize, f32, f32)>;

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn calls() -> (Vec<usize>, Vec<usize>) {
    (take(&mut *ASKED.lock()), take(&mut *SET_UP.lock()))
}

/// A table with rows of their own heights loses its first rows through
/// `drop_first_cells`, like a log that drops its oldest lines. Every row
/// has a name that it keeps when its index changes. The rows that are
/// left stay at the same place on the screen, and the table asks no
/// height and sets up no cell that was on screen.
#[view]
struct TableDropFirstCells {
    /// The names of the rows the data holds.
    names: Vec<usize>,

    #[init]
    under: Container,
    table: TableView,
    step:  Label,
}

impl TableDropFirstCells {
    fn set_step(self: Weak<Self>, step: &'static str) {
        from_main(move || {
            self.step.set_text(step);
        });
    }

    fn shown(self: Weak<Self>) -> Shown {
        from_main(move || {
            let mut cells: Shown = self
                .table
                .visible_cells()
                .into_iter()
                .map(|(index, cell)| (self.names[index], cell.absolute_frame().y(), cell.height()))
                .collect();
            cells.sort_by_key(|cell| cell.0);
            cells
        })
    }

    fn drop_rows(mut self: Weak<Self>, count: usize) {
        from_main(move || {
            self.names.drain(..count);
            self.table.drop_first_cells(count);
        });
        wait_for_next_frame();
    }

    /// Every row on screen before and after stands at the same place.
    fn same_place(before: &Shown, after: &Shown, name: &str) -> Result<()> {
        let mut kept = 0;
        for (row, y, height) in after {
            let Some(old) = before.iter().find(|old| old.0 == *row) else {
                continue;
            };
            kept += 1;
            ensure!(
                near(old.1, *y) && near(old.2, *height),
                "{name}: row {row} was at {} with height {}, now at {y} with {height}",
                old.1,
                old.2
            );
        }
        ensure!(kept > 3, "{name}: only {kept} rows stayed on screen");
        Ok(())
    }
}

impl Setup for TableDropFirstCells {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.names = (0..ROWS).collect();

        // A color no cell has, the gaps between the rows show it.
        self.under.set_color(Color::hex("#3b4252"));
        self.under.place().tl(0).size(TABLE_WIDTH, TABLE_HEIGHT);

        self.table.place().tl(0).size(TABLE_WIDTH, TABLE_HEIGHT);
        self.table.set_data_source(self).register_cell::<Label>();
        self.table.set_cell_spacing(SPACING);
        self.table.set_variable_heights(true);
        self.table.reload_data();

        self.step
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        self.step.set_text_color(BLACK);
        self.step.set_text("40 rows as loaded");
        self.step.place().t(12).l(TABLE_WIDTH + 10.0).r(8).h(200);
    }
}

impl TableData for TableDropFirstCells {
    fn cell_height(&self, index: usize) -> f32 {
        ASKED.lock().push(index);
        height_of(self.names[index])
    }

    fn number_of_cells(&self) -> usize {
        self.names.len()
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let name = self.names[index];
        SET_UP.lock().push(name);
        let cell = registry.cell::<Label>();
        cell.set_text_size(14);
        cell.set_text_color(BLACK);
        cell.set_color(if name.is_multiple_of(2) {
            Color::hex("#cfe3ff")
        } else {
            Color::hex("#ffe2b8")
        });
        cell.set_text(format!("row {name}, {} high", height_of(name)));
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}

impl ViewTest for TableDropFirstCells {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        // The table lays its rows out in the first frame it has a size in.
        wait_for_next_frame();
        wait_for_next_frame();

        // Down to the middle, the first rows are out of view. The wheel
        // goes to the view under the pointer, a tap puts it over the table.
        inject_touches("200 300 b\n200 300 e");
        inject_scroll(-700);
        wait_for_next_frame();
        let middle = view.shown();
        ensure!(
            middle.iter().all(|row| row.0 >= DROPPED),
            "a row that will be dropped is on screen: {middle:?}"
        );
        view.set_step("scrolled to the middle\nrows 0 to 5 are above the screen");
        check_colors(MIDDLE)?;
        calls();

        // Rows above the screen go. Nothing on screen moves and nothing
        // is asked.
        view.drop_rows(DROPPED);
        let (asked, set_up) = calls();
        let dropped = view.shown();
        ensure!(asked.is_empty(), "rows above the screen went: asked {asked:?}");
        ensure!(
            set_up.iter().all(|name| !middle.iter().any(|row| row.0 == *name)),
            "rows above the screen went: a row on screen was set up again, {set_up:?}"
        );
        Self::same_place(&middle, &dropped, "rows above the screen went")?;
        // Only the scroll bar is new: the content is shorter now.
        view.set_step("rows 0 to 5 left the data\nno row on screen moved\nno height was asked");
        check_colors(DROPPED_ABOVE)?;

        // A full reload of the same data gives the same rows.
        from_main(move || {
            view.table.reload_data();
        });
        wait_for_next_frame();
        let (asked, _) = calls();
        ensure!(
            asked.len() == ROWS - DROPPED,
            "a full reload asked {} rows",
            asked.len()
        );
        let reloaded = view.shown();
        Self::same_place(&dropped, &reloaded, "a full reload")?;
        ensure!(
            reloaded.len() == dropped.len(),
            "a full reload shows {} rows, before {}",
            reloaded.len(),
            dropped.len()
        );

        // Rows that are on screen go. The row that was first under them
        // is now the first row of the table.
        inject_scroll(100_000);
        wait_for_next_frame();
        calls();
        view.drop_rows(4);
        let (asked, _) = calls();
        let top = view.shown();
        ensure!(asked.is_empty(), "rows on screen went: asked {asked:?}");
        ensure!(
            top.first().is_some_and(|row| row.0 == DROPPED + 4 && near(row.1, 0.0)),
            "rows on screen went: the first row is {:?}",
            top.first()
        );
        view.set_step("at the top, rows 6 to 9 left the data\nrow 10 is the first row now");
        check_colors(DROPPED_TOP)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const MIDDLE: &str = r"
             532   16 - #cbcbcb
             476   20 - #515151
             492   20 - #cfcfcf
             428   36 - #eeeeee
             556   36 - #ffffff
             212   40 - #ffe2b8
               4   84 - #cfe3ff
             184   88 - #808d9e
             324  108 - #3b4252
             204  140 - #1e1b16
             184  144 - #9e8c72
             184  200 - #808d9e
              44  216 - #cfe3ff
             396  224 - #262b35
             396  256 - #a69378
             396  288 - #8794a6
             240  304 - #616a77
             160  308 - #cfe3ff
             396  320 - #8794a6
               4  348 - #ffe2b8
             396  352 - #a69378
             212  364 - #ffe2b8
             592  392 - #ffffff
             204  464 - #1e1b16
             192  468 - #d1b997
             184  472 - #c3ad8d
             160  528 - #cfe3ff
             240  568 - #776956
             184  576 - #342e26
               4  588 - #ffe2b8
             376  592 - #3b4252
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const DROPPED_ABOVE: &str = r"
             488   16 - #ffffff
               4   20 - #3b4252
             524   36 - #cdcdcd
             564   36 - #ffffff
             212   40 - #ffe2b8
             432   52 - #b4b4b4
             184   88 - #808d9e
             396  136 - #a69378
             204  140 - #1e1b16
             184  144 - #9e8c72
              28  184 - #cfe3ff
             396  184 - #8794a6
             184  204 - #808d9e
             396  224 - #262b35
             184  248 - #9e8c72
             396  264 - #a69378
             240  304 - #616a77
             160  308 - #cfe3ff
             396  312 - #8794a6
               4  336 - #cfe3ff
             212  364 - #ffe2b8
             592  404 - #ffffff
             336  432 - #3b4252
             204  464 - #1e1b16
             192  468 - #d1b997
             184  472 - #c3ad8d
             160  528 - #cfe3ff
             240  568 - #776956
             184  576 - #342e26
               4  588 - #ffe2b8
             396  588 - #ffe2b8
             584  592 - #ffffff
";

/// Recorded with `--record-colors`.
const DROPPED_TOP: &str = r"
             160   20 - #cfe3ff
             440   20 - #ffffff
             472   36 - #aeaeae
              40   40 - #3b4252
             396   52 - #a69378
             492   52 - #cdcdcd
             512   52 - #888888
             564   52 - #ffffff
             192   60 - #d1b997
             192   64 - #d1b997
             396  108 - #8794a6
             184  124 - #808d9e
               4  156 - #cfe3ff
             396  160 - #262b35
             212  180 - #ffe2b8
             204  184 - #83745f
             396  204 - #262b35
             192  224 - #15171a
             192  280 - #000000
             240  340 - #616a77
             160  344 - #cfe3ff
             592  348 - #ffffff
               4  388 - #ffe2b8
             184  388 - #9e8c72
             236  388 - #ffe2b8
             396  404 - #ffe2b8
             240  444 - #616a77
             184  508 - #9e8c72
             204  508 - #83745f
              12  568 - #cfe3ff
             392  572 - #3b4252
             592  592 - #ffffff
";
