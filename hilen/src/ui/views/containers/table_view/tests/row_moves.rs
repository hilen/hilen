use std::{fmt::Write, thread::sleep, time::Duration};

use anyhow::{Result, ensure};

use super::super::row_move::HOLD;
use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::{Weak, weak_from_ref},
    },
    gm::{LossyConvert, color::Color},
    ui::{
        CellRegistry, Label, NamedKey, Setup, TableData, TableView, UIManager, View, ViewData, ViewTest, view,
    },
    ui_test::{check_colors, checkpoint, inject_named_key, inject_touches},
};

const ROWS: usize = 20;
const ROW: f32 = 80.0;

/// The rows before any move.
const NATURAL: &str = r"
               4    4 - #73bfc6
             396    4 - #4b7c81
             592    4 - #333340
             232   36 - #73bfc6
             180   40 - #73bfc6
             396   64 - #4b7c81
             164  116 - #000000
             396  120 - #4f7c7e
             208  124 - #000101
             164  196 - #000000
               4  208 - #80bfbf
             236  208 - #436464
             180  280 - #86bfbc
             592  280 - #333340
             236  284 - #010101
             164  356 - #000000
             232  356 - #557370
             232  368 - #557370
             428  396 - #333340
             228  428 - #161d1c
             232  428 - #161d1c
             164  444 - #010101
               4  456 - #93bfb6
             164  516 - #000000
             592  532 - #f2e5b2
             300  560 - #837c60
             232  564 - #54503e
             236  564 - #e2d6a6
             200  568 - #565240
             328  568 - #bab089
              56  592 - #f2e5b2
             464  592 - #f2e5b2
";

/// Row 1 held with its shadow over place 3, rows 2 and 3 moved up.
const HELD: &str = r"
               4    4 - #73bfc6
             364    4 - #73bfc6
             500    4 - #333340
             232   36 - #73bfc6
             164   48 - #000101
             180  120 - #80bfbf
             236  128 - #436464
             396  164 - #577c7a
             236  204 - #010101
             164  208 - #010101
               4  232 - #86bfbc
             592  256 - #333340
             164  300 - #000000
             312  340 - #5e817d
             396  340 - #5e817d
               4  344 - #709893
              88  344 - #709893
             172  356 - #8cbfb9
             228  356 - #000000
             232  360 - #557370
             232  368 - #557370
             228  428 - #161d1c
             232  428 - #161d1c
             452  480 - #333340
               4  532 - #f2e5b2
             592  532 - #f2e5b2
             300  560 - #837c60
             232  564 - #54503e
             236  564 - #e2d6a6
             200  568 - #565240
             328  568 - #bab089
             488  592 - #f2e5b2
";

/// Row 1 dropped at place 3.
const DROPPED: &str = r"
             592    4 - #333340
             232   36 - #73bfc6
             164   48 - #000101
             396   76 - #4b7c81
             180  120 - #80bfbf
             236  128 - #436464
             396  164 - #577c7a
             236  204 - #010101
             164  208 - #010101
               4  232 - #86bfbc
             592  256 - #333340
             176  288 - #000101
             228  356 - #000000
             232  360 - #557370
             232  368 - #557370
             228  428 - #161d1c
             232  428 - #161d1c
             180  444 - #93bfb6
               4  448 - #93bfb6
             592  532 - #f2e5b2
             196  560 - #968e6e
             184  564 - #625c48
             196  564 - #968e6e
             220  564 - #f2e5b2
             272  564 - #dacfa1
             324  564 - #f2e5b2
             340  564 - #f2e5b2
             184  568 - #625c48
             196  568 - #968e6e
             208  568 - #6e6851
             308  568 - #d1c69a
              36  592 - #f2e5b2
";

/// A finger at x 200 from y `from` to `to` in steps of 10 points, still down.
fn held(from: u32, to: u32) -> String {
    let mut lines = format!("200 {from} b\n");
    let mut y = from;
    while y != to {
        y = if to > y {
            (y + 10).min(to)
        } else {
            y.saturating_sub(10).max(to)
        };
        writeln!(lines, "200 {y} m").unwrap();
    }
    lines
}

/// The same drag with a release where it stopped.
fn drag(from: u32, to: u32) -> String {
    format!("{}200 {to} e\n", held(from, to))
}

/// A table whose rows can be dragged to a new place. Each row keeps its
/// name and its color, so a moved row is seen at its new place, and the
/// status under the table names every move and every tap.
#[view]
struct TableRowMoves {
    order:    Vec<usize>,
    moves:    String,
    selected: String,

    #[init]
    table:  TableView,
    status: Label,
}

impl TableRowMoves {
    fn show_status(self: Weak<Self>) {
        let first: Vec<String> = self.order.iter().take(6).map(usize::to_string).collect();
        let text = format!(
            "order {} .. moves {} taps {}",
            first.join(" "),
            self.moves,
            self.selected
        );
        self.status.set_text(text);
    }
}

impl Setup for TableRowMoves {
    fn setup(mut self: Weak<Self>) {
        self.order = (0..ROWS).collect();
        self.set_color(Color::rgb(0.2, 0.2, 0.25));
        self.table.place().tl(0).size(400, 520);
        self.table.set_data_source(self).register_cell::<Label>();
        self.table.set_row_moves(true);
        self.table.reload_data();
        self.status.set_text_size(16).set_color(Color::rgb(0.95, 0.9, 0.7));
        self.status.place().lrb(0).h(70);
        self.show_status();
    }
}

impl TableData for TableRowMoves {
    fn cell_height(&self, _: usize) -> f32 {
        ROW
    }

    fn number_of_cells(&self) -> usize {
        self.order.len()
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let name = self.order[index];
        let cell = registry.cell::<Label>();
        cell.set_text(format!("row {name}"));
        let (name_f, rows): (f32, f32) = (name.lossy_convert(), ROWS.lossy_convert());
        let shade = 0.45 + 0.5 * (name_f / rows);
        cell.set_color(Color::rgb(shade, 0.75, 1.0 - shade * 0.5));
        cell
    }

    fn cell_selected(&mut self, index: usize) {
        let mut this = weak_from_ref(self);
        write!(this.selected, "{index};").unwrap();
        this.show_status();
    }

    fn move_row(&mut self, from: usize, to: usize) {
        let mut this = weak_from_ref(self);
        let name = this.order.remove(from);
        this.order.insert(to, name);
        write!(this.moves, "{from}>{to};").unwrap();
        this.show_status();
    }
}

fn state(view: Weak<TableRowMoves>) -> (Vec<usize>, String, String) {
    from_main(move || (view.order.clone(), view.moves.clone(), view.selected.clone()))
}

impl ViewTest for TableRowMoves {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        UIManager::set_drag_scrolling(false);
        check_colors(NATURAL)?;

        // A mouse lifts row 1 by its middle and puts it 2 rows lower.
        inject_touches(held(120, 300));
        checkpoint("row 1 held over row 3, rows 2 and 3 moved up")?;
        check_colors(HELD)?;
        inject_touches("200 300 e");
        let (order, moves, _) = state(view);
        ensure!(moves == "1>3;", "the drag down moved {moves}");
        ensure!(
            order[..5] == [0, 2, 3, 1, 4],
            "the order after the drag down is {order:?}"
        );
        check_colors(DROPPED)?;

        // Row 4 goes all the way up.
        inject_touches(drag(360, 40));
        let (order, moves, _) = state(view);
        ensure!(moves == "1>3;4>0;", "the drag up moved {moves}");
        ensure!(
            order[..5] == [4, 0, 2, 3, 1],
            "the order after the drag up is {order:?}"
        );

        // Escape puts a held row back, its release moves nothing.
        inject_touches(held(200, 420));
        inject_named_key(NamedKey::Escape);
        checkpoint("Escape put the held row back")?;
        inject_touches("200 420 e");
        let (order, moves, selected) = state(view);
        ensure!(moves == "1>3;4>0;", "a cancelled drag moved {moves}");
        ensure!(
            order[..5] == [4, 0, 2, 3, 1],
            "a cancelled drag changed the order to {order:?}"
        );
        ensure!(selected.is_empty(), "a cancelled drag tapped {selected}");

        // A tap still selects.
        inject_touches("200 40 b\n200 40 e");
        let (.., selected) = state(view);
        ensure!(selected == "0;", "a tap selected {selected}");

        finger(view)?;
        edge(view)
    }
}

/// With a finger a quick drag scrolls, a hold lifts the row.
fn finger(view: Weak<TableRowMoves>) -> Result<()> {
    UIManager::set_drag_scrolling(true);
    inject_touches(drag(300, 100));
    let offset = from_main(move || view.table.content_offset());
    ensure!(
        offset < -100.0,
        "a quick finger drag did not scroll, offset {offset}"
    );
    let (.., moves, _) = state(view);
    ensure!(moves == "1>3;4>0;", "a quick finger drag moved {moves}");
    // The fling of the quick drag runs on for a moment.
    sleep(Duration::from_secs_f32(1.5));
    wait_for_next_frame();
    from_main(move || {
        let mut table = view.table;
        table.set_content_offset(0);
    });
    wait_for_next_frame();
    let offset = from_main(move || view.table.content_offset());
    ensure!(
        offset.abs() < 0.5,
        "the table is still moving after its fling, offset {offset}"
    );

    inject_touches("200 120 b");
    sleep(Duration::from_secs_f32(HOLD + 0.2));
    wait_for_next_frame();
    inject_touches(drag(120, 270).replacen("200 120 b\n", "", 1));
    let (order, moves, _) = state(view);
    ensure!(moves == "1>3;4>0;1>2;", "the held finger moved {moves}");
    ensure!(
        order[..4] == [4, 2, 0, 3],
        "the order after the held finger is {order:?}"
    );
    let offset = from_main(move || view.table.content_offset());
    ensure!(
        offset.abs() < 0.5,
        "the held finger scrolled the table to {offset}"
    );
    Ok(())
}

/// A row held at the bottom edge scrolls the table and goes down with it.
fn edge(view: Weak<TableRowMoves>) -> Result<()> {
    UIManager::set_drag_scrolling(false);
    inject_touches(held(40, 510));
    sleep(Duration::from_secs_f32(1.0));
    wait_for_next_frame();
    let offset = from_main(move || view.table.content_offset());
    ensure!(
        offset < -200.0,
        "a row at the edge did not scroll the table, offset {offset}"
    );
    checkpoint("the table scrolled under the held row")?;
    inject_touches("200 510 e");
    let (order, moves, _) = state(view);
    let last = moves.split(';').rfind(|m| !m.is_empty()).unwrap_or_default().to_owned();
    ensure!(last.starts_with("0>"), "the edge drag moved {moves}");
    let to: usize = last[2..].parse()?;
    ensure!(
        to > 7 && order[to] == 4,
        "the edge drag put row 4 at {to}, order {order:?}"
    );
    Ok(())
}
