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
    ui_test::{check_colors, set_record_probe_count},
};

/// The rows `cell_height` was asked for, in the order of the calls.
static ASKED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
/// The rows `setup_cell` was called for, in the order of the calls.
static SET_UP: Mutex<Vec<usize>> = Mutex::new(Vec::new());

const SPACING: f32 = 4.0;
const SHORT: f32 = 40.0;
const TABLE_WIDTH: f32 = 400.0;
const TABLE_HEIGHT: f32 = 600.0;
const ROWS: usize = 12;
const GROWN_ROW: usize = 3;
const TEXT_ROW: usize = 1;
const TAIL_STEPS: usize = 20;
const TAIL_STEP: f32 = 12.0;
const NEW_ROWS: usize = 3;

/// Recorded with `--record-colors`.
const START: &str = r"
             260   16 - #7d899a
             516   16 - #b2b2b2
             192   20 - #010101
             444   20 - #b2b2b2
             452   20 - #ffffff
             512   20 - #ffffff
             528   20 - #ffffff
             436   24 - #e6e6e6
               4   36 - #cfe3ff
             124   64 - #433b30
             192   64 - #010101
             236   64 - #433b30
             272  108 - #cfe3ff
             152  112 - #292d33
              48  124 - #cfe3ff
             184  148 - #2a251e
             396  164 - #ffe2b8
             260  192 - #7d899a
             176  196 - #cfe3ff
             592  224 - #ffffff
             204  236 - #928169
             132  240 - #ffe2b8
             272  240 - #ffe2b8
               4  256 - #ffe2b8
             176  284 - #cfe3ff
             236  284 - #363c43
             396  304 - #3b4252
             204  324 - #928169
             124  328 - #433b30
             272  328 - #ffe2b8
             260  368 - #7d899a
             192  372 - #010101
               4  396 - #ffe2b8
             572  408 - #ffffff
             184  412 - #2a251e
             260  412 - #9a886f
             188  456 - #22252a
             264  456 - #92a0b4
             120  460 - #4b535d
             396  476 - #cfe3ff
             264  500 - #b4a082
             120  504 - #5d5243
             128  504 - #ffe2b8
             204  504 - #000000
             264  504 - #b4a082
               4  592 - #3b4252
             352  592 - #3b4252
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const GREW: &str = r"
             260   16 - #7d899a
             132   20 - #cfe3ff
             192   20 - #010101
             436   24 - #ffffff
             484   24 - #ffffff
             508   36 - #656565
             532   36 - #eaeaea
             560   36 - #ffffff
             452   52 - #6e6e6e
               4   92 - #cfe3ff
             260  104 - #7d899a
             132  108 - #cfe3ff
             200  108 - #000000
             368  128 - #3b4252
             136  184 - #463e33
             204  184 - #928169
             212  184 - #c3ad8d
             220  184 - #cbb492
             576  212 - #ffffff
             396  232 - #3b4252
             184  252 - #22252a
             260  252 - #7d899a
               4  292 - #ffe2b8
             124  300 - #433b30
             176  300 - #ffe2b8
             236  300 - #433b30
             256  344 - #cfe3ff
             380  344 - #cfe3ff
             204  384 - #928169
             124  388 - #433b30
             272  388 - #ffe2b8
             592  388 - #ffffff
             184  428 - #22252a
               4  436 - #cfe3ff
             396  460 - #ffe2b8
             184  472 - #2a251e
             260  472 - #9a886f
             264  516 - #92a0b4
             120  520 - #4b535d
             196  520 - #010101
             264  520 - #92a0b4
             264  560 - #b4a082
             120  564 - #5d5243
             128  564 - #ffe2b8
             264  564 - #b4a082
             380  572 - #ffe2b8
               4  592 - #3b4252
             492  592 - #ffffff
";

/// Recorded with `--record-colors`.
const NEW_TEXT: &str = r"
               4    4 - #cfe3ff
             184   16 - #22252a
             260   16 - #7d899a
             132   20 - #cfe3ff
             524   20 - #040404
             476   24 - #ffffff
             436   36 - #ffffff
             460   36 - #515151
             508   36 - #ffffff
             196   64 - #010101
             260  104 - #7d899a
             124  108 - #363c43
             176  108 - #cfe3ff
             368  128 - #3b4252
              16  132 - #ffe2b8
             136  184 - #463e33
             204  184 - #928169
             212  184 - #c3ad8d
             220  184 - #cbb492
             592  192 - #ffffff
             396  236 - #cfe3ff
             184  252 - #22252a
             260  252 - #7d899a
               4  292 - #ffe2b8
             124  300 - #433b30
             236  300 - #433b30
             176  344 - #cfe3ff
             592  372 - #ffffff
             204  384 - #928169
             260  384 - #9a886f
             132  388 - #ffe2b8
             396  408 - #3b4252
             184  428 - #22252a
             272  432 - #cfe3ff
               4  444 - #cfe3ff
             176  476 - #ffe2b8
             264  516 - #92a0b4
             120  520 - #4b535d
             196  520 - #010101
             264  520 - #92a0b4
             264  560 - #b4a082
             120  564 - #5d5243
             128  564 - #ffe2b8
             204  564 - #000000
             264  564 - #b4a082
               4  580 - #ffe2b8
             396  592 - #3b4252
             532  592 - #ffffff
";

/// Recorded with `--record-colors`.
const TAIL: &str = r"
             356    8 - #3b4252
               4   12 - #cfe3ff
             472   20 - #ffffff
             436   24 - #ffffff
             184   28 - #22252a
             260   28 - #7d899a
             124   32 - #363c43
             528   32 - #cbcbcb
             468   36 - #a1a1a1
             496   36 - #545454
             528   36 - #cbcbcb
             568   36 - #b4b4b4
             572   36 - #cecece
             488   64 - #8e8e8e
             436   68 - #ffffff
             200   76 - #000000
             260  116 - #7d899a
             176  120 - #cfe3ff
             204  160 - #928169
             132  164 - #ffe2b8
             272  164 - #ffe2b8
             396  172 - #a69378
               4  188 - #cfe3ff
             124  208 - #363c43
             176  208 - #cfe3ff
             236  208 - #363c43
             396  224 - #8794a6
             260  248 - #9a886f
             200  252 - #000000
             396  276 - #8794a6
             188  292 - #22252a
             264  292 - #92a0b4
             128  296 - #cfe3ff
             260  316 - #3b4252
             396  316 - #262b35
               4  360 - #ffe2b8
             592  384 - #ffffff
             396  424 - #a69378
             168  456 - #aa977b
             244  456 - #5d5243
             100  460 - #ffe2b8
             168  460 - #aa977b
             176  460 - #776956
             296  460 - #020201
               4  592 - #ffe2b8
             220  592 - #ffe2b8
             396  592 - #a69378
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const ADDED: &str = r"
             504   20 - #696969
             512   20 - #ffffff
             484   24 - #ffffff
             124   32 - #433b30
             200   32 - #000000
             272   32 - #ffe2b8
             468   36 - #cecece
             536   36 - #cecece
             544   36 - #000000
             436   52 - #ffffff
             236   72 - #363c43
             132   76 - #cfe3ff
             192   76 - #010101
               4   92 - #cfe3ff
             184  116 - #2a251e
             260  116 - #9a886f
             124  120 - #433b30
             360  140 - #3b4252
             188  160 - #22252a
             264  160 - #92a0b4
             120  164 - #4b535d
             128  164 - #cfe3ff
             264  164 - #92a0b4
             524  176 - #ffffff
             396  228 - #a69378
               4  260 - #ffe2b8
             592  296 - #ffffff
             168  324 - #aa977b
             244  324 - #5d5243
             300  324 - #c3ad8d
             100  328 - #ffe2b8
             168  328 - #aa977b
             176  328 - #776956
             244  328 - #5d5243
             300  328 - #c3ad8d
             396  364 - #a69378
               4  412 - #ffe2b8
             304  428 - #ffe2b8
             396  468 - #262b35
             168  488 - #808d9e
             224  488 - #758090
             396  508 - #8794a6
             592  520 - #ffffff
             144  536 - #ffe2b8
               4  556 - #3b4252
             396  572 - #8794a6
             220  580 - #cfe3ff
             296  592 - #cfe3ff
";

/// Recorded with `--record-colors`.
const SHRUNK: &str = r"
             504   20 - #b6b6b6
             532   20 - #ffffff
             560   20 - #050505
             260   28 - #9a886f
             124   32 - #433b30
             192   32 - #010101
             460   32 - #a0a0a0
             436   36 - #ffffff
             440   36 - #cbcbcb
             436   52 - #676767
             484   52 - #ffffff
             528   64 - #d4d4d4
             184   72 - #22252a
             260   72 - #7d899a
             132   76 - #cfe3ff
               4   96 - #3b4252
             352   96 - #3b4252
             204  116 - #928169
             236  116 - #433b30
             124  120 - #433b30
             176  120 - #ffe2b8
             264  160 - #92a0b4
             120  164 - #4b535d
             204  164 - #000000
             276  164 - #cfe3ff
             396  228 - #a69378
               4  264 - #ffe2b8
             592  308 - #ffffff
             244  324 - #5d5243
             300  324 - #c3ad8d
             100  328 - #ffe2b8
             168  328 - #aa977b
             244  328 - #5d5243
             300  328 - #c3ad8d
             396  364 - #a69378
               4  416 - #ffe2b8
             396  468 - #262b35
             224  488 - #758090
             176  496 - #2a2e34
              48  504 - #cfe3ff
             396  508 - #8794a6
             592  528 - #ffffff
             144  536 - #ffe2b8
             224  536 - #908068
             168  580 - #808d9e
             220  580 - #cfe3ff
               4  592 - #cfe3ff
             396  592 - #8794a6
";

/// What the table shows: every cell on screen as row, top and height in
/// the content, the height of the content and the scroll offset.
#[derive(PartialEq, Debug)]
struct Shown {
    cells:   Vec<(usize, f32, f32)>,
    content: f32,
    offset:  f32,
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// The calls since the last time this was read.
fn calls() -> (Vec<usize>, Vec<usize>) {
    (take(&mut *ASKED.lock()), take(&mut *SET_UP.lock()))
}

/// A table with rows of their own heights, like a chat. A row changes
/// through `reload_cell` and rows come to the end through
/// `load_new_cells`. The test counts which rows the table asks the
/// height of and which cells it sets up: only the changed ones. Every
/// row says its number, its height and what happened to it.
#[view]
struct TableReloadCell {
    heights: Vec<f32>,
    notes:   Vec<&'static str>,

    #[init]
    under: Container,
    table: TableView,
    step:  Label,
}

impl TableReloadCell {
    fn set_step(self: Weak<Self>, step: &'static str) {
        from_main(move || {
            self.step.set_text(step);
        });
    }

    fn shown(self: Weak<Self>) -> Shown {
        from_main(move || {
            let mut cells: Vec<_> = self
                .table
                .visible_cells()
                .into_iter()
                .map(|(row, cell)| (row, cell.y(), cell.height()))
                .collect();
            cells.sort_by_key(|cell| cell.0);
            Shown {
                cells,
                content: self.table.content_height(),
                offset: self.table.content_offset(),
            }
        })
    }

    /// Every cell on screen stands where the heights of the rows above
    /// put it, and the content is as high as all rows.
    fn check(self: Weak<Self>, name: &str) -> Result<Shown> {
        let shown = self.shown();
        let heights = from_main(move || self.heights.clone());
        let top = |row: usize| -> f32 { heights[..row].iter().map(|height| height + SPACING).sum() };

        ensure!(!shown.cells.is_empty(), "{name}: no cell is on screen");
        for (row, y, height) in &shown.cells {
            ensure!(
                near(*y, top(*row)),
                "{name}: row {row} starts at {y}, the rows above end at {}",
                top(*row)
            );
            ensure!(
                near(*height, heights[*row]),
                "{name}: row {row} is {height} high, its data says {}",
                heights[*row]
            );
        }
        let total = top(heights.len()) - SPACING;
        ensure!(
            near(shown.content, total),
            "{name}: the content is {} high, the rows are {total}",
            shown.content
        );
        Ok(shown)
    }
}

impl Setup for TableReloadCell {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.heights = vec![SHORT; ROWS];
        self.notes = vec!["as loaded"; ROWS];

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
        self.step.set_text("12 rows as loaded");
        self.step.place().t(12).l(TABLE_WIDTH + 10.0).r(8).h(200);
    }
}

impl TableData for TableReloadCell {
    fn cell_height(&self, index: usize) -> f32 {
        ASKED.lock().push(index);
        self.heights[index]
    }

    fn number_of_cells(&self) -> usize {
        self.heights.len()
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        SET_UP.lock().push(index);
        let cell = registry.cell::<Label>();
        cell.set_text_size(14);
        cell.set_text_color(BLACK);
        cell.set_color(if index.is_multiple_of(2) {
            Color::hex("#cfe3ff")
        } else {
            Color::hex("#ffe2b8")
        });
        cell.set_text(format!(
            "row {index}, {} high, {}",
            self.heights[index], self.notes[index]
        ));
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}

impl TableReloadCell {
    fn grow_tail(mut view: Weak<Self>) -> Result<()> {
        // The last row grows step by step, like a reply that is written.
        // The table follows it to the end.
        let last = ROWS - 1;
        for _ in 0..TAIL_STEPS {
            from_main(move || {
                view.heights[last] += TAIL_STEP;
                view.notes[last] = "grows like a reply";
                view.table.reload_cell(last);
                view.table.scroll_to_bottom();
            });
        }
        let (asked, set_up) = calls();
        ensure!(
            asked == [last; TAIL_STEPS] && set_up == [last; TAIL_STEPS],
            "the last row grew {TAIL_STEPS} times: asked {asked:?}, set up {set_up:?}"
        );
        let tail = view.check("the last row grew")?;
        let (_, y, height) = tail.cells[tail.cells.len() - 1];
        ensure!(
            near(y + height + tail.offset, TABLE_HEIGHT),
            "the last row ends at {}, the table at {TABLE_HEIGHT}",
            y + height + tail.offset
        );
        view.set_step(
            "row 11 grew 20 times to 280\nthe table followed it to the end\nonly row 11 was asked and set up",
        );
        check_colors(TAIL)?;
        Ok(())
    }
}

impl ViewTest for TableReloadCell {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);

        // The table lays its rows out in the first frame it has a size in.
        wait_for_next_frame();
        wait_for_next_frame();
        let start = view.check("as loaded")?;
        ensure!(
            start.cells.len() == ROWS,
            "{} of the {ROWS} rows are on screen",
            start.cells.len()
        );
        check_colors(START)?;
        calls();

        // A row in the middle grows. Only that row is asked and set up,
        // the rows under it move down.
        from_main(move || {
            view.heights[GROWN_ROW] = 100.0;
            view.notes[GROWN_ROW] = "grew";
            view.table.reload_cell(GROWN_ROW);
        });
        let (asked, set_up) = calls();
        ensure!(
            asked == [GROWN_ROW] && set_up == [GROWN_ROW],
            "a row grew: asked {asked:?}, set up {set_up:?}"
        );
        let grew = view.check("a row grew")?;
        view.set_step("row 3 grew to 100\nthe rows under it moved down\nno other row was asked or set up");
        check_colors(GREW)?;

        // Only the text of a row changes, no frame moves.
        from_main(move || {
            view.notes[TEXT_ROW] = "new text";
            view.table.reload_cell(TEXT_ROW);
        });
        let (asked, set_up) = calls();
        ensure!(
            asked == [TEXT_ROW] && set_up == [TEXT_ROW],
            "a text changed: asked {asked:?}, set up {set_up:?}"
        );
        let texted = view.check("a text changed")?;
        ensure!(
            texted == grew,
            "a new text moved a row: {texted:?}, before {grew:?}"
        );
        view.set_step("row 1 has a new text\nnothing moved");
        check_colors(NEW_TEXT)?;

        Self::grow_tail(view)?;

        // Rows come to the end. Only they are asked and set up.
        from_main(move || {
            view.heights.extend([SHORT; NEW_ROWS]);
            view.notes.extend(["new"; NEW_ROWS]);
            view.table.load_new_cells();
            view.table.scroll_to_bottom();
        });
        let new_rows: Vec<usize> = (ROWS..ROWS + NEW_ROWS).collect();
        let (asked, set_up) = calls();
        ensure!(
            asked == new_rows && set_up == new_rows,
            "{NEW_ROWS} rows were added: asked {asked:?}, set up {set_up:?}"
        );
        let added = view.check("rows were added")?;
        ensure!(
            added.cells.iter().any(|cell| cell.0 == ROWS + NEW_ROWS - 1),
            "the last new row is not on screen: {added:?}"
        );
        view.set_step("3 new rows at the end\nonly they were asked and set up");
        check_colors(ADDED)?;

        // A row above the screen shrinks. Its cell is not on screen, so
        // it is not set up. The rows on screen move up in the content.
        ensure!(
            added.cells.iter().all(|cell| cell.0 != GROWN_ROW),
            "row {GROWN_ROW} has to be off screen for this step: {added:?}"
        );
        from_main(move || {
            view.heights[GROWN_ROW] = SHORT;
            view.notes[GROWN_ROW] = "shrank";
            view.table.reload_cell(GROWN_ROW);
            view.table.scroll_to_bottom();
        });
        let (asked, set_up) = calls();
        ensure!(
            asked == [GROWN_ROW] && set_up.iter().all(|row| *row != GROWN_ROW && *row < ROWS),
            "a row off screen shrank: asked {asked:?}, set up {set_up:?}"
        );
        let shrunk = view.check("a row off screen shrank")?;
        view.set_step(
            "row 3, above the screen, shrank to 40\nthe content is 60 shorter\nthe end stays in view",
        );
        check_colors(SHRUNK)?;

        // A full reload of the same data gives the same picture.
        from_main(move || {
            view.table.reload_data();
        });
        let (asked, _) = calls();
        ensure!(
            asked.len() == ROWS + NEW_ROWS,
            "a full reload asked {} rows",
            asked.len()
        );
        let reloaded = view.check("a full reload")?;
        ensure!(
            reloaded == shrunk,
            "a full reload moved the rows: {reloaded:?}, before {shrunk:?}"
        );
        check_colors(SHRUNK)?;

        Ok(())
    }
}
