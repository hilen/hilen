use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::Color,
    ui::{
        CellRegistry, Label, Setup, TableData, TableView, TextAlignment, View, ViewCallbacks, ViewData,
        ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, inject_scroll, inject_touches, set_record_probe_count},
};

const TOP: f32 = 80.0;
const TALL: f32 = 480.0;
const SHORT: f32 = 300.0;
const ROW: f32 = 40.0;
const GROWN_ROW: f32 = 100.0;
const SCROLLED_UP: f32 = 200.0;

const TABLE: Color = Color::rgba(0.35, 0.38, 0.42, 1.0);
const EVEN: Color = Color::rgba(0.87, 0.91, 0.94, 1.0);
const ODD: Color = Color::rgba(0.76, 0.82, 0.88, 1.0);

/// How much of the rows is under the bottom edge of the table, 0 for a
/// table at its end.
fn below_end(table: &TableView) -> f32 {
    table.content_height() + table.content_offset() - table.height()
}

/// Looks at a table once a frame, after the table and the scroll view
/// inside it got their frames and before the frame is drawn. A table that
/// gets its end back one frame late is caught here, a check after the
/// change would see only the frame after it.
#[view]
struct EndWatch {
    table:   Weak<TableView>,
    /// The table has to be at its end in every frame now.
    expect:  bool,
    /// Frames that drew the table off its end while it had to be there.
    off_end: usize,
}

impl ViewCallbacks for EndWatch {
    fn update(&mut self) {
        if self.expect && self.table.is_ok() && below_end(&self.table).abs() > 0.5 {
            self.off_end += 1;
        }
    }
}

/// 2 tables with the same rows. The left one has `set_keeps_end`, the
/// right one is a plain table, so every picture shows what the rule
/// changes. Both get shorter and taller, get new rows and a row that
/// grows. Then the left one is scrolled up and must keep its offset.
#[view]
struct TableKeepsEnd {
    rows:     usize,
    /// The row that is taller than the others.
    tall_row: Option<usize>,

    #[init]
    status:      Label,
    kept_title:  Label,
    plain_title: Label,
    kept:        TableView,
    plain:       TableView,
    // After the tables, they read the frames of this frame.
    kept_watch:  EndWatch,
    plain_watch: EndWatch,
}

impl TableKeepsEnd {
    fn set_table_height(self: Weak<Self>, height: f32) {
        self.kept.place().clear().t(TOP).l(20).w(270).h(height);
        self.plain.place().clear().t(TOP).l(310).w(270).h(height);
    }

    fn say(self: Weak<Self>, text: &str) {
        self.status.set_text(text);
    }
}

impl Setup for TableKeepsEnd {
    fn setup(mut self: Weak<Self>) {
        self.rows = 30;

        self.status.set_text_size(16);
        self.status.set_alignment(TextAlignment::Left);
        self.status.place().t(10).lr(20).h(24);

        self.kept_title.set_text("set_keeps_end(true)").set_text_size(16);
        self.kept_title.place().t(44).l(20).w(270).h(24);
        self.plain_title.set_text("a plain table").set_text_size(16);
        self.plain_title.place().t(44).l(310).w(270).h(24);

        for table in [self.kept, self.plain] {
            table.set_data_source(self).register_cell::<Label>();
            table.set_color(TABLE);
        }
        self.kept.set_variable_heights(true).set_keeps_end(true);
        self.plain.set_variable_heights(true);
        self.set_table_height(TALL);

        self.kept_watch.table = self.kept;
        self.plain_watch.table = self.plain;
        // From the first frame: a table that is filled with nothing on
        // screen yet starts at its end.
        self.kept_watch.expect = true;
    }
}

impl TableData for TableKeepsEnd {
    fn cell_height(&self, index: usize) -> f32 {
        if self.tall_row == Some(index) {
            GROWN_ROW
        } else {
            ROW
        }
    }

    fn number_of_cells(&self) -> usize {
        self.rows
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        let cell = registry.cell::<Label>();
        let text = if self.tall_row == Some(index) {
            format!("row {index}, grown")
        } else {
            format!("row {index}")
        };
        cell.set_text(text).set_text_size(18);
        cell.set_color(if index.is_multiple_of(2) { EVEN } else { ODD });
        cell
    }

    fn cell_selected(&mut self, _: usize) {}
}

fn expect_below(view: Weak<TableKeepsEnd>, step: &'static str, kept: f32, plain: f32) -> Result<()> {
    from_main(move || {
        let (kept_now, plain_now) = (below_end(&view.kept), below_end(&view.plain));
        ensure!(
            (kept_now - kept).abs() < 0.5,
            "{step}: the table with the rule has {kept_now} under its edge, not {kept}"
        );
        ensure!(
            (plain_now - plain).abs() < 0.5,
            "{step}: the plain table has {plain_now} under its edge, not {plain}"
        );
        ensure!(
            view.kept_watch.off_end == 0,
            "{step}: {} frames drew the table with the rule off its end",
            view.kept_watch.off_end
        );
        Ok(())
    })
}

fn expect_offset(view: Weak<TableKeepsEnd>, step: &'static str, offset: f32) -> Result<()> {
    from_main(move || {
        let now = view.kept.content_offset();
        ensure!(
            now.to_bits() == offset.to_bits(),
            "{step}: the table scrolled up moved from {offset} to {now}"
        );
        Ok(())
    })
}

impl ViewTest for TableKeepsEnd {
    fn perform_test(mut view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);

        wait_for_next_frame();
        from_main(move || {
            ensure!(view.plain.height() > 0.0, "the tables have no size yet");
            view.plain.scroll_to_bottom();
            view.plain_watch.expect = true;
            view.say("both at their end, row 29 is the last");
            Ok(())
        })?;
        check_colors(AT_END)?;
        expect_below(view, "at the start", 0.0, 0.0)?;

        from_main(move || {
            view.set_table_height(SHORT);
            view.say("shorter: left still ends with row 29, right lost its end");
        });
        check_colors(SHORTER)?;
        expect_below(view, "shorter", 0.0, TALL - SHORT)?;
        // The watch is only worth something when it sees a lost end.
        let seen = from_main(move || view.plain_watch.off_end);
        ensure!(seen > 0, "the watch did not see the plain table lose its end");

        from_main(move || {
            view.plain_watch.expect = false;
            view.set_table_height(TALL);
            view.say("as tall as before: both end with row 29 again");
        });
        check_colors(TALL_AGAIN)?;
        expect_below(view, "taller", 0.0, 0.0)?;

        from_main(move || {
            view.rows += 4;
            view.kept.load_new_cells();
            view.plain.load_new_cells();
            view.say("4 new rows: left ends with row 33, right stays at row 29");
        });
        check_colors(NEW_ROWS)?;
        expect_below(view, "new rows", 0.0, ROW * 4.0)?;

        from_main(move || {
            let last = view.rows - 1;
            view.tall_row = Some(last);
            view.kept.reload_cell(last);
            view.plain.reload_cell(last);
            view.say("row 33 grew: left shows all of it");
        });
        check_colors(GROWN)?;
        expect_below(view, "grown row", 0.0, ROW * 3.0 + GROWN_ROW)?;

        from_main(move || view.kept_watch.expect = false);
        inject_touches("150 300 m");
        inject_scroll(SCROLLED_UP);
        let scrolled = from_main(move || view.kept.content_offset());
        from_main(move || {
            ensure!(
                (below_end(&view.kept) - SCROLLED_UP).abs() < 0.5,
                "the wheel left {} under the edge",
                below_end(&view.kept)
            );
            view.set_table_height(SHORT);
            view.say("left scrolled up by 200, then shorter: its top rows stay");
            Ok(())
        })?;
        check_colors(SCROLLED_SHORTER)?;
        expect_offset(view, "scrolled up and shorter", scrolled)?;

        from_main(move || {
            view.rows += 2;
            view.kept.load_new_cells();
            view.plain.load_new_cells();
            view.set_table_height(TALL);
            view.say("2 new rows and taller: the top rows of left still stay");
        });
        check_colors(SCROLLED_TALLER)?;
        expect_offset(view, "scrolled up, new rows and taller", scrolled)?;

        inject_scroll(-5000);
        from_main(move || {
            view.kept_watch.expect = true;
            view.set_table_height(SHORT);
            view.say("left scrolled back to its end, then shorter: ends with row 35");
        });
        check_colors(BACK_AT_END)?;
        expect_below(view, "back at the end", 0.0, ROW * 5.0 + GROWN_ROW + TALL - SHORT)
    }
}

const AT_END: &str = r"
             592    4 - #597c95
              60   20 - #24323c
              96   20 - #24323c
             212   20 - #49667a
              40   24 - #597c95
              76   24 - #597c95
             136   24 - #0e1317
             168   24 - #597c95
             240   24 - #161f25
             468   52 - #000000
             124   56 - #2d3e4b
             192   56 - #597c95
             424   56 - #40596c
             484   56 - #2d3e4b
             324   76 - #597c95
             168   96 - #848a8f
             168  100 - #848a8f
             168  104 - #848a8f
             456  104 - #aeb6bc
              52  116 - #dee8f0
             168  144 - #747d86
             420  144 - #41464c
             576  144 - #c2d1e0
             288  176 - #dee8f0
             140  180 - #dee8f0
             180  180 - #010101
               4  204 - #597c95
             468  216 - #60676f
             468  220 - #60676f
             420  224 - #41464c
             468  224 - #60676f
             176  256 - #dee8f0
             428  260 - #dee8f0
             468  264 - #c0c9d0
             576  272 - #dee8f0
             316  276 - #dee8f0
             140  300 - #c2d1e0
              36  304 - #c2d1e0
             420  304 - #41464c
             428  340 - #dee8f0
             168  344 - #c0c9d0
             460  380 - #c2d1e0
             168  384 - #a8b5c2
             420  384 - #41464c
             572  396 - #c2d1e0
              20  404 - #dee8f0
             284  412 - #90979c
             140  420 - #dee8f0
             464  420 - #000000
             576  436 - #90979c
             176  456 - #c2d1e0
             428  460 - #c2d1e0
              60  476 - #c2d1e0
             576  480 - #90979c
             464  496 - #000000
             140  500 - #dee8f0
             284  504 - #90979c
             420  504 - #4b4e51
             576  516 - #90979c
             460  540 - #c2d1e0
             168  544 - #a8b5c2
              20  556 - #c2d1e0
             316  592 - #597c95
             532  592 - #597c95
";
const SHORTER: &str = r"
              96   20 - #141b21
             144   20 - #374d5c
             280   20 - #24323c
             316   20 - #49667a
             392   20 - #25343f
              64   24 - #172026
              80   24 - #597c95
              96   24 - #141b21
             136   24 - #49667a
             172   24 - #597c95
             204   24 - #263540
             212   24 - #090d0f
             260   24 - #597c95
             280   24 - #24323c
             304   24 - #000000
             348   24 - #1d2931
             372   24 - #597c95
             424   52 - #40596c
             468   52 - #000000
             140   56 - #597c95
             184   56 - #597c95
             200   56 - #000000
             484   56 - #2d3e4b
             416   60 - #000000
              20   92 - #dee8f0
             464   96 - #000000
             420  104 - #4b4e51
             328  112 - #dee8f0
             140  120 - #c2d1e0
             236  140 - #dee8f0
             456  140 - #98a4b0
             456  144 - #98a4b0
             576  144 - #c2d1e0
             140  160 - #dee8f0
              20  176 - #dee8f0
             428  180 - #dee8f0
             140  200 - #c2d1e0
             312  200 - #c2d1e0
             468  216 - #60676f
             420  224 - #41464c
             468  224 - #60676f
             140  240 - #dee8f0
             420  264 - #4b4e51
             576  264 - #90979c
              44  276 - #c2d1e0
             176  276 - #c2d1e0
             576  276 - #90979c
             464  300 - #c2d1e0
             420  304 - #41464c
             284  308 - #90979c
             140  320 - #dee8f0
             284  324 - #90979c
             576  328 - #90979c
             288  336 - #dee8f0
             428  340 - #dee8f0
             168  364 - #a8b5c2
              20  376 - #c2d1e0
             456  376 - #c2d1e0
             504  376 - #c2d1e0
             148  496 - #597c95
             424  500 - #597c95
               4  592 - #597c95
             292  592 - #597c95
             560  592 - #597c95
";
const TALL_AGAIN: &str = r"
             592    4 - #597c95
              72   20 - #0b0f12
             136   20 - #486478
             300   20 - #597c95
             136   24 - #486478
             200   24 - #597c95
             252   24 - #1f2c35
             468   52 - #000000
             140   56 - #597c95
             192   56 - #597c95
             408   56 - #000000
             168   96 - #848a8f
             140  100 - #dee8f0
             168  100 - #848a8f
             168  104 - #848a8f
             456  104 - #aeb6bc
              20  116 - #dee8f0
             284  132 - #c2d1e0
             592  136 - #597c95
             168  144 - #747d86
             420  144 - #41464c
             456  144 - #98a4b0
             140  180 - #dee8f0
             180  180 - #010101
             468  216 - #60676f
             176  220 - #abb8c6
             468  220 - #60676f
             420  224 - #41464c
             468  224 - #60676f
              20  236 - #c2d1e0
             312  240 - #dee8f0
             584  244 - #597c95
             140  260 - #dee8f0
             420  264 - #4b4e51
             468  264 - #c0c9d0
             464  300 - #c2d1e0
             168  304 - #a8b5c2
             420  304 - #41464c
              56  316 - #c2d1e0
             260  320 - #dee8f0
             140  340 - #dee8f0
             576  352 - #dee8f0
             460  380 - #c2d1e0
             168  384 - #a8b5c2
             576  400 - #90979c
               4  404 - #597c95
             284  412 - #90979c
             140  420 - #dee8f0
             464  420 - #000000
             420  424 - #4b4e51
             576  436 - #90979c
             176  456 - #c2d1e0
             464  460 - #c2d1e0
             576  480 - #90979c
             176  496 - #dee8f0
             464  496 - #000000
             284  504 - #90979c
             420  504 - #4b4e51
             572  516 - #dee8f0
              20  524 - #c2d1e0
             140  540 - #c2d1e0
             460  540 - #c2d1e0
             356  556 - #c2d1e0
             232  592 - #597c95
";
const NEW_ROWS: &str = r"
             592    4 - #597c95
             208   20 - #000000
             264   20 - #597c95
             420   20 - #597c95
              56   24 - #161f25
              84   24 - #0e1317
             156   24 - #597c95
             300   24 - #172026
             304   24 - #2c3e4a
             424   52 - #40596c
             124   56 - #2d3e4b
             132   56 - #2d3e4b
             140   56 - #597c95
             180   56 - #000000
             192   56 - #597c95
             472   56 - #597c95
             484   56 - #2d3e4b
             464   96 - #000000
             428  100 - #dee8f0
             568  100 - #dee8f0
             168  104 - #c0c9d0
              20  116 - #dee8f0
             344  120 - #c2d1e0
             168  144 - #a8b5c2
             456  144 - #98a4b0
             140  180 - #dee8f0
             428  180 - #dee8f0
             592  192 - #597c95
               4  216 - #597c95
             468  216 - #60676f
             312  220 - #c2d1e0
             168  224 - #a8b5c2
             420  224 - #41464c
             140  260 - #dee8f0
             468  264 - #c0c9d0
             244  276 - #dee8f0
             420  304 - #41464c
             324  316 - #c2d1e0
             176  336 - #dee8f0
             576  336 - #90979c
              32  356 - #dee8f0
             140  380 - #c2d1e0
             460  380 - #c2d1e0
             420  384 - #41464c
             284  404 - #90979c
             576  404 - #90979c
             180  420 - #010101
              40  456 - #c2d1e0
             464  456 - #c2d1e0
             140  460 - #c2d1e0
             176  464 - #abb8c6
             360  480 - #dee8f0
             576  480 - #90979c
             284  484 - #90979c
             464  496 - #000000
             428  500 - #dee8f0
             260  516 - #dee8f0
             576  524 - #c2d1e0
             176  536 - #c2d1e0
             460  540 - #c2d1e0
             420  544 - #41464c
              20  556 - #c2d1e0
             148  592 - #597c95
             344  592 - #597c95
";
const GROWN: &str = r"
             592    4 - #597c95
              76   20 - #597c95
             132   20 - #0b0f12
             248   20 - #49667a
              96   24 - #486579
             108   24 - #597c95
             172   24 - #354a59
             212   24 - #597c95
             424   52 - #40596c
             468   52 - #000000
             140   56 - #597c95
             192   56 - #597c95
             408   56 - #000000
              20   84 - #c2d1e0
             168   84 - #a8b5c2
             316   92 - #dee8f0
             456  104 - #aeb6bc
             140  120 - #dee8f0
             420  144 - #41464c
             456  144 - #98a4b0
             228  148 - #c2d1e0
             576  148 - #c2d1e0
             140  160 - #c2d1e0
              20  172 - #c2d1e0
             420  184 - #4b4e51
             304  192 - #597c95
             140  200 - #dee8f0
             468  216 - #60676f
             468  220 - #60676f
             420  224 - #41464c
             468  224 - #60676f
             168  244 - #a8b5c2
             252  260 - #dee8f0
             468  264 - #c0c9d0
             140  280 - #dee8f0
             576  288 - #c2d1e0
              48  296 - #dee8f0
             428  300 - #c2d1e0
             176  316 - #c2d1e0
             312  324 - #dee8f0
             576  344 - #90979c
             140  360 - #dee8f0
             180  360 - #010101
             428  380 - #c2d1e0
             576  400 - #90979c
             176  404 - #abb8c6
             352  404 - #dee8f0
             464  420 - #000000
              20  424 - #dee8f0
             576  436 - #90979c
             140  440 - #dee8f0
             284  444 - #90979c
             428  460 - #c2d1e0
             576  480 - #90979c
             460  500 - #dee8f0
             420  504 - #4b4e51
             100  508 - #1b1e20
             172  512 - #7d8690
             460  540 - #c2d1e0
             420  544 - #41464c
             320  556 - #c2d1e0
               4  592 - #597c95
             228  592 - #597c95
             592  592 - #597c95
";
const SCROLLED_SHORTER: &str = r"
             124   20 - #2a3a46
             244   20 - #24323c
              44   24 - #597c95
              76   24 - #597c95
             100   24 - #151d22
             176   24 - #000000
             228   24 - #354a59
             296   24 - #151d22
             344   24 - #000000
             372   24 - #597c95
             404   24 - #597c95
             468   52 - #000000
             140   56 - #597c95
             196   56 - #304351
             424   56 - #40596c
             484   56 - #2d3e4b
             168   84 - #848a8f
             464   96 - #000000
             420  104 - #4b4e51
             168  124 - #747d86
             576  136 - #c2d1e0
             288  140 - #dee8f0
             456  140 - #98a4b0
              32  144 - #dee8f0
             420  144 - #41464c
             168  164 - #c0c9d0
             428  180 - #dee8f0
             468  180 - #dee8f0
             420  184 - #4b4e51
             348  196 - #dee8f0
             176  200 - #abb8c6
             428  220 - #c2d1e0
             468  224 - #60676f
             176  236 - #dee8f0
             140  240 - #dee8f0
             284  240 - #90979c
             576  240 - #90979c
             272  244 - #dee8f0
             284  256 - #90979c
             576  260 - #90979c
              20  264 - #c2d1e0
             420  264 - #4b4e51
             468  264 - #c0c9d0
             176  276 - #c2d1e0
             576  276 - #90979c
             140  280 - #c2d1e0
             428  300 - #c2d1e0
             464  300 - #c2d1e0
             140  320 - #dee8f0
             168  324 - #c0c9d0
             244  328 - #dee8f0
             428  340 - #dee8f0
             420  344 - #4b4e51
              52  348 - #c2d1e0
             168  364 - #a8b5c2
             312  376 - #c2d1e0
             456  376 - #c2d1e0
             560  376 - #c2d1e0
              12  428 - #597c95
             156  528 - #597c95
             448  532 - #597c95
               4  592 - #597c95
             304  592 - #597c95
             592  592 - #597c95
";
const SCROLLED_TALLER: &str = r"
             592    4 - #597c95
             308   20 - #263540
             372   20 - #000000
              56   24 - #161f25
              84   24 - #0e1317
             136   24 - #000000
             180   24 - #597c95
             240   24 - #597c95
             292   24 - #597c95
             308   24 - #263540
             336   24 - #597c95
             424   52 - #40596c
             124   56 - #2d3e4b
             132   56 - #2d3e4b
             192   56 - #597c95
             484   56 - #2d3e4b
             168   84 - #848a8f
               4   96 - #597c95
             576   96 - #dee8f0
             456  100 - #aeb6bc
             356  116 - #dee8f0
             168  124 - #747d86
             456  144 - #98a4b0
             140  160 - #dee8f0
             180  160 - #010101
             288  180 - #c2d1e0
             428  180 - #dee8f0
              20  184 - #c2d1e0
             576  196 - #dee8f0
             468  216 - #60676f
             428  220 - #c2d1e0
             176  236 - #dee8f0
             348  236 - #c2d1e0
             428  260 - #dee8f0
             468  264 - #c0c9d0
              44  276 - #c2d1e0
             140  280 - #c2d1e0
             420  304 - #41464c
             168  324 - #c0c9d0
             284  336 - #90979c
             420  344 - #4b4e51
             576  356 - #90979c
               4  360 - #597c95
             140  360 - #c2d1e0
             460  380 - #c2d1e0
             140  400 - #dee8f0
             284  400 - #90979c
             576  408 - #90979c
             464  420 - #000000
             420  424 - #4b4e51
             168  444 - #a8b5c2
             560  456 - #c2d1e0
             284  464 - #90979c
              28  476 - #dee8f0
             168  484 - #c0c9d0
             464  496 - #000000
             360  516 - #dee8f0
             140  520 - #c2d1e0
             460  540 - #c2d1e0
             420  544 - #41464c
             176  556 - #dee8f0
             576  556 - #c2d1e0
               4  592 - #597c95
             252  592 - #597c95
";
const BACK_AT_END: &str = r"
              96   20 - #0c1013
             316   20 - #3e5769
              56   24 - #030506
             136   24 - #597c95
             188   24 - #597c95
             216   24 - #000000
             264   24 - #0e1317
             344   24 - #000000
             380   24 - #40596c
             444   24 - #597c95
             140   56 - #597c95
             184   56 - #597c95
             424   56 - #40596c
             472   56 - #597c95
             176   96 - #dee8f0
              92  100 - #dee8f0
             140  100 - #dee8f0
             456  100 - #aeb6bc
             420  104 - #4b4e51
             252  108 - #dee8f0
             576  116 - #dee8f0
             140  140 - #c2d1e0
             176  140 - #abb8c6
             456  140 - #98a4b0
              20  144 - #c2d1e0
             176  144 - #abb8c6
             420  144 - #41464c
             456  144 - #98a4b0
             312  156 - #c2d1e0
             140  180 - #dee8f0
             428  180 - #dee8f0
             468  180 - #dee8f0
             468  216 - #60676f
             428  220 - #c2d1e0
             468  220 - #60676f
             468  224 - #60676f
              20  228 - #c2d1e0
             340  236 - #c2d1e0
             576  240 - #90979c
             100  252 - #1b1e20
             148  252 - #c2d1e0
             208  252 - #383c40
             576  260 - #90979c
             420  264 - #4b4e51
             468  264 - #c0c9d0
             576  276 - #90979c
             460  300 - #c2d1e0
             420  304 - #41464c
             140  320 - #dee8f0
             576  320 - #dee8f0
             284  324 - #90979c
              20  336 - #dee8f0
             284  336 - #90979c
             420  344 - #4b4e51
             140  360 - #c2d1e0
             236  376 - #c2d1e0
             348  376 - #c2d1e0
             456  376 - #c2d1e0
             592  460 - #597c95
             424  512 - #597c95
             160  524 - #597c95
               4  592 - #597c95
             312  592 - #597c95
             540  592 - #597c95
";
