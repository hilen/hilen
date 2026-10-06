use anyhow::Result;
#[cfg(desktop)]
use hilen::system::Clipboard;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        CellRegistry, GRAY, Label, MarkdownView, ModifiersState, Setup, TableData, TableView, TextAlignment,
        TextSelection, UIManager, VerticalAlignment, View, ViewData, ViewFrame, ViewSubviews, ViewTest,
        WHITE, view,
    },
    ui_test::{
        check_colors, inject_keys, inject_modifiers, inject_scroll, inject_touches, set_record_probe_count,
        system_input::wait_until,
    },
};

use crate::text_points::{click, drag, label_with, point_after, point_of, shown};

const ROWS: usize = 60;
const LINE_HEIGHT: f32 = 36.0;
const PROSE_HEIGHT: f32 = 72.0;
const NUMBER_WIDTH: f32 = 48.0;

/// Every third row is a markdown text of 2 paragraphs, the others are 1
/// line in a label.
fn is_prose(row: usize) -> bool {
    row % 3 == 2
}

fn line_text(row: usize) -> String {
    format!("Row {row}: a single line of text")
}

fn prose_markdown(row: usize) -> String {
    format!("**Row {row}** is prose.\n\nIt has a second paragraph.")
}

/// What a copy of a whole row gives.
fn row_copy(row: usize) -> String {
    if is_prose(row) {
        format!("Row {row} is prose.\n\nIt has a second paragraph.")
    } else {
        line_text(row)
    }
}

fn rows_copy(rows: impl Iterator<Item = usize>) -> String {
    rows.map(row_copy).collect::<Vec<_>>().join("\n")
}

/// A row with 1 line of text. The number at the left is not selectable.
#[view]
struct LineCell {
    #[init]
    number: Label,
    text:   Label,
}

impl Setup for LineCell {
    fn setup(self: Weak<Self>) {
        number_style(self.number);
        self.text
            .set_text_size(18)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top)
            .set_selectable(true);
        self.text.place().t(8).l(NUMBER_WIDTH).r(8).b(0);
    }
}

/// A row with a markdown text.
#[view]
struct ProseCell {
    #[init]
    number: Label,
    text:   MarkdownView,
}

impl Setup for ProseCell {
    fn setup(self: Weak<Self>) {
        number_style(self.number);
        self.text.set_selectable(true);
        self.text.place().t(8).l(NUMBER_WIDTH).r(8).b(0);
    }
}

fn number_style(number: Weak<Label>) {
    number
        .set_text_size(14)
        .set_text_color(GRAY)
        .set_alignment(TextAlignment::Right)
        .set_vertical_alignment(VerticalAlignment::Top);
    number.place().t(10).l(0).w(NUMBER_WIDTH - 8.0).h(20);
}

const CHECK_1: &str = r"
     212   16 - #bfbfbf
     312   16 - #e9e9e9
     116   20 - #fefefe
     152   20 - #ffffff
     360   20 - #ffffff
     396   32 - #bcbcbc
     480   32 - #bcbcbc
     532   32 - #bcbcbc
     584   36 - #a6a6a6
      80   44 - #000000
     584   48 - #a6a6a6
      32   52 - #ffffff
     180   52 - #000000
     216   52 - #7c7c7c
     280   52 - #858585
     584   56 - #a6a6a6
     584   64 - #a6a6a6
     584   72 - #a6a6a6
     584   76 - #a6a6a6
      80   80 - #000000
     152   80 - #b8d2ff
     188   80 - #b8d2ff
     584   80 - #a6a6a6
     120   84 - #151515
      32   88 - #bcbcbc
     120   88 - #151515
     216   88 - #59667c
     256   88 - #161a1f
     164   96 - #b8d2ff
     288   96 - #b8d2ff
      76  116 - #a7bee7
      76  120 - #a7bee7
     116  120 - #b8d2ff
     148  120 - #b8d2ff
     172  120 - #b8d2ff
      12  144 - #bcbcbc
      88  144 - #a3bae2
      88  148 - #a3bae2
     136  148 - #afc8f3
     160  148 - #5b687e
     236  148 - #9bb1d7
     244  148 - #b8d2ff
     112  152 - #b8d2ff
     184  152 - #b8d2ff
     212  152 - #b8d2ff
     592  156 - #ffffff
     400  164 - #ffffff
     100  184 - #b8d2ff
     184  192 - #000000
      76  196 - #b8d2ff
     160  196 - #323232
     224  196 - #999999
     264  196 - #999999
      12  200 - #bcbcbc
     124  200 - #b8d2ff
     104  228 - #000000
     176  232 - #ffffff
     216  232 - #7c7c7c
     280  232 - #858585
      80  264 - #1a1d24
     120  264 - #9b9da0
      32  268 - #ffffff
     456  280 - #ffffff
      88  292 - #e0e1e2
     156  292 - #1a1d24
     200  292 - #787a7e
     236  292 - #d6d6d7
      12  316 - #bcbcbc
      80  332 - #000000
     592  336 - #ffffff
     160  340 - #323232
     216  340 - #7c7c7c
     256  340 - #1f1f1f
      80  368 - #000000
     120  368 - #000000
     160  376 - #323232
     216  376 - #7c7c7c
     280  376 - #858585
      12  388 - #bcbcbc
      80  408 - #1a1d24
     120  408 - #9b9da0
      84  432 - #c9c9c9
     136  432 - #5a5a5a
      40  456 - #ffffff
     104  456 - #bdbdbd
     132  456 - #e2e2e2
     164  456 - #e8e8e8
     208  456 - #c0c0c0
     264  456 - #bcbcbc
     308  456 - #dbdbdb
     388  456 - #ffffff
     444  456 - #e2e2e2
     508  456 - #c7c7c7
     124  592 - #ffffff
     348  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_2: &str = r"
     108   16 - #bcbcbc
     212   16 - #bfbfbf
     312   16 - #e9e9e9
      56   20 - #ffffff
     360   20 - #ffffff
     404   32 - #bcbcbc
     452   32 - #bcbcbc
     508   32 - #bcbcbc
     564   32 - #bcbcbc
      12   36 - #bcbcbc
     180   40 - #010101
     236   40 - #999999
     276   40 - #999999
     120   44 - #4d4d4d
      76   68 - #e6e6e7
      32   76 - #ffffff
     112   76 - #1a1d24
     164   76 - #272a31
      88  100 - #e0e1e2
     136  100 - #f2f3f3
     188  100 - #909195
     220  100 - #909195
      12  112 - #bcbcbc
      80  140 - #000000
     592  140 - #ffffff
     256  144 - #000000
     152  148 - #212121
     188  148 - #ffffff
     220  148 - #bdbdbd
     292  148 - #c1c1c1
      12  152 - #bcbcbc
     120  152 - #4d4d4d
      80  176 - #000000
     152  184 - #212121
     220  184 - #bdbdbd
     276  184 - #999999
     120  188 - #4d4d4d
     584  192 - #a6a6a6
     584  204 - #a6a6a6
     584  212 - #a6a6a6
      24  216 - #ffffff
      80  216 - #1a1d24
     128  216 - #848689
     584  216 - #a6a6a6
     164  220 - #272a31
     428  220 - #ffffff
     584  220 - #a6a6a6
     584  224 - #a6a6a6
     584  228 - #a6a6a6
     584  232 - #a6a6a6
     584  236 - #a6a6a6
      88  244 - #e0e1e2
     120  244 - #909195
     160  244 - #787a7e
     200  244 - #787a7e
     236  244 - #d6d6d7
      12  272 - #bcbcbc
     104  288 - #000000
     256  288 - #000000
     152  292 - #212121
     180  292 - #010101
     216  292 - #797979
     236  292 - #999999
     288  292 - #ffffff
      80  320 - #000000
      32  324 - #bcbcbc
     152  328 - #212121
     236  328 - #999999
     276  328 - #999999
     476  332 - #ffffff
      80  360 - #1a1d24
     128  360 - #848689
     592  372 - #ffffff
     104  388 - #ffffff
     160  388 - #787a7e
     200  388 - #787a7e
     236  388 - #d6d6d7
      12  392 - #bcbcbc
      96  428 - #d1d1d1
      68  432 - #000000
      96  432 - #d1d1d1
     152  432 - #808080
     208  432 - #808080
     300  432 - #5f5f5f
      44  456 - #c0c0c0
     132  456 - #e2e2e2
     180  456 - #ffffff
     264  456 - #bcbcbc
     388  456 - #ffffff
     440  456 - #bcbcbc
     484  456 - #e9e9e9
     480  568 - #ffffff
       4  592 - #ffffff
     180  592 - #ffffff
     372  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_3: &str = r"
     108   16 - #bcbcbc
     228   16 - #cecece
     312   16 - #e9e9e9
      56   20 - #ffffff
     152   20 - #ffffff
     360   20 - #ffffff
      16   32 - #bcbcbc
     396   32 - #bcbcbc
     480   32 - #bcbcbc
     584   36 - #a6a6a6
      80   44 - #000000
     120   44 - #000000
     192   44 - #ffffff
     584   48 - #a6a6a6
     224   52 - #999999
     256   52 - #1f1f1f
     584   56 - #a6a6a6
     584   64 - #a6a6a6
     584   72 - #a6a6a6
     168   76 - #b8d2ff
     236   76 - #b8d2ff
     288   76 - #b8d2ff
     584   76 - #a6a6a6
      80   80 - #000000
     192   80 - #b8d2ff
     584   80 - #a6a6a6
      32   84 - #bcbcbc
     120   84 - #151515
     120   88 - #151515
     216   88 - #59667c
     256   88 - #161a1f
      76  116 - #a7bee7
     152  116 - #b8d2ff
      76  120 - #a7bee7
     108  120 - #b8d2ff
     132  120 - #b8d2ff
     172  120 - #b8d2ff
     184  140 - #b8d2ff
      88  144 - #a3bae2
      88  148 - #a3bae2
     120  148 - #6b7a95
     236  148 - #9bb1d7
     244  148 - #b8d2ff
     152  152 - #b8d2ff
     212  152 - #b8d2ff
     592  156 - #ffffff
      12  164 - #bcbcbc
     100  184 - #b8d2ff
     120  196 - #b8d2ff
     160  196 - #323232
     216  196 - #7c7c7c
     256  196 - #1f1f1f
      80  200 - #b8d2ff
      12  220 - #bcbcbc
     164  232 - #000000
     212  232 - #bcbcbc
     280  232 - #858585
      76  260 - #e6e6e7
     120  264 - #9b9da0
      32  268 - #ffffff
     456  280 - #ffffff
      88  292 - #e0e1e2
     120  292 - #909195
     156  292 - #1a1d24
     200  292 - #787a7e
     236  292 - #d6d6d7
      12  332 - #bcbcbc
     104  336 - #000000
     592  336 - #ffffff
     164  340 - #000000
     192  340 - #999999
     224  340 - #999999
     280  340 - #858585
      80  372 - #ffffff
     160  376 - #323232
     188  376 - #000000
     216  376 - #7c7c7c
     256  376 - #1f1f1f
      12  396 - #bcbcbc
      64  428 - #1f1f1f
     132  432 - #575757
     148  432 - #808080
     168  432 - #808080
     196  432 - #575757
      40  456 - #ffffff
      96  456 - #bcbcbc
     164  456 - #e8e8e8
     264  456 - #bcbcbc
     308  456 - #dbdbdb
     388  456 - #ffffff
     444  456 - #e2e2e2
     508  456 - #c7c7c7
       4  592 - #ffffff
     164  592 - #ffffff
     348  592 - #ffffff
     592  592 - #ffffff
";

const CHECK_4: &str = r"
      56   20 - #ffffff
     360   20 - #ffffff
      88   32 - #8ca7d3
     112   32 - #8ca7d3
     144   32 - #8ca7d3
     212   32 - #8ca7d3
     236   32 - #8ca7d3
     264   32 - #8ca7d3
     288   32 - #8ca7d3
     408   32 - #bcbcbc
     460   32 - #bcbcbc
     532   32 - #bcbcbc
     172   36 - #b8d2ff
      80   64 - #b8d2ff
      12   76 - #bcbcbc
     180   76 - #000000
     216   76 - #59667c
     280   76 - #606e85
     584   76 - #a6a6a6
     136   84 - #b8d2ff
     584   88 - #a6a6a6
     584  100 - #a6a6a6
      76  104 - #a7bee7
      76  108 - #a7bee7
      32  112 - #ffffff
     584  112 - #a6a6a6
     584  120 - #a6a6a6
     112  136 - #b8d2ff
     160  136 - #5b687e
     200  136 - #5b687e
     236  136 - #9bb1d7
     592  164 - #ffffff
      80  176 - #000000
     132  176 - #b8d2ff
      12  180 - #bcbcbc
     188  184 - #000000
     224  184 - #6e7e99
     264  184 - #6e7e99
     416  208 - #ffffff
     300  212 - #b8d2ff
      24  216 - #bdbdbd
     104  216 - #000000
     152  220 - #181b21
     228  220 - #90a5c8
     264  228 - #b8d2ff
      76  248 - #a7bee7
     120  248 - #9cb2d8
      76  252 - #a7bee7
      24  256 - #bdbdbd
     244  276 - #b8d2ff
     120  280 - #6b7a95
     160  280 - #5b687e
     188  280 - #6b7a95
      24  324 - #bdbdbd
      80  324 - #b8d2ff
     180  328 - #010101
     228  328 - #90a5c8
     280  328 - #b8d2ff
     588  328 - #ffffff
     128  332 - #383f4d
     132  332 - #383f4d
     448  332 - #ffffff
     212  360 - #20242c
     152  364 - #181b21
     292  364 - #8b9fc1
     252  368 - #b8d2ff
      12  380 - #bcbcbc
      76  392 - #a7bee7
     120  392 - #1a1d24
      76  396 - #a7bee7
      80  400 - #b8d2ff
     164  400 - #232831
     360  428 - #050505
      32  432 - #ffffff
     136  432 - #5a5a5a
     164  432 - #5f5f5f
     260  432 - #3d3d3d
     320  432 - #808080
     164  456 - #e8e8e8
     208  456 - #c0c0c0
     484  456 - #e9e9e9
     416  468 - #ffffff
     120  472 - #fbfbfb
     288  472 - #d2d2d2
      84  484 - #d4d4d4
      64  488 - #d4d4d4
     248  488 - #d8d8d8
     352  488 - #bcbcbc
     548  488 - #e1e1e1
     148  500 - #dedede
      56  504 - #cdcdcd
      72  504 - #d1d1d1
     140  504 - #d2d2d2
     204  504 - #ffffff
     300  592 - #ffffff
     464  592 - #ffffff
";

const CHECK_5: &str = r"
     108   16 - #bcbcbc
     312   20 - #e9e9e9
     136   32 - #8ca7d3
     168   32 - #8ca7d3
     212   32 - #8ca7d3
     232   32 - #8ca7d3
     252   32 - #8ca7d3
     284   32 - #8ca7d3
     396   32 - #bcbcbc
     480   32 - #bcbcbc
     540   32 - #bcbcbc
      32   40 - #ffffff
      76   44 - #b8d2ff
     120   68 - #000000
     160   76 - #242932
     216   76 - #59667c
     280   76 - #606e85
     584   76 - #a6a6a6
     584   88 - #a6a6a6
     584  100 - #a6a6a6
      76  104 - #a7bee7
      76  108 - #a7bee7
     120  108 - #7383a0
     156  112 - #232831
     584  112 - #a6a6a6
      12  116 - #bcbcbc
     584  120 - #a6a6a6
      88  132 - #a3bae2
     168  136 - #1a1d24
     200  136 - #5b687e
     220  136 - #6b7a95
     416  156 - #ffffff
     592  160 - #ffffff
      12  184 - #bcbcbc
     164  184 - #010101
     192  184 - #6e7e99
     224  184 - #6e7e99
     264  184 - #6e7e99
     104  188 - #b8d2ff
      76  212 - #b8d2ff
     120  220 - #0f1115
     180  220 - #010101
     216  220 - #576479
     292  220 - #8b9fc1
     252  224 - #b8d2ff
     144  244 - #b8d2ff
      76  248 - #a7bee7
      76  252 - #a7bee7
      80  252 - #1a1d24
     120  252 - #9cb2d8
      24  256 - #bdbdbd
     200  272 - #b8d2ff
     160  280 - #5b687e
     236  280 - #9bb1d7
     112  284 - #b8d2ff
     460  288 - #ffffff
      12  320 - #bcbcbc
      76  320 - #b8d2ff
     172  320 - #b8d2ff
     216  328 - #576479
     280  328 - #b8d2ff
     128  332 - #383f4d
     132  332 - #383f4d
     592  332 - #ffffff
     104  360 - #000000
     152  364 - #181b21
     216  364 - #576479
     292  364 - #8b9fc1
     244  368 - #b8d2ff
      76  392 - #a7bee7
      76  396 - #a7bee7
     120  396 - #1b1e25
     180  396 - #b8d2ff
      24  400 - #bdbdbd
      76  432 - #000000
     132  432 - #ffffff
     172  456 - #dbdbdb
     236  456 - #ffffff
     344  456 - #bcbcbc
     528  456 - #ffffff
     292  468 - #dbdbdb
      60  472 - #d8d8d8
     100  472 - #e5e5e5
     140  472 - #d1d1d1
     160  472 - #d1d1d1
     480  472 - #d2d2d2
     580  472 - #ffffff
     396  488 - #d2d2d2
     552  488 - #e1e1e1
      52  504 - #c6c6c6
     140  504 - #ffffff
     188  504 - #d2d2d2
     208  504 - #d1d1d1
     228  504 - #bcbcbc
     320  592 - #ffffff
     528  592 - #ffffff
";

const CHECK_6: &str = r"
     108   16 - #bcbcbc
     212   16 - #bfbfbf
     312   16 - #e9e9e9
      56   20 - #ffffff
     152   20 - #ffffff
     236   20 - #ffffff
     380   32 - #bcbcbc
     448   32 - #bcbcbc
     516   32 - #bcbcbc
     584   32 - #bcbcbc
      12   36 - #bcbcbc
     188   40 - #000000
     216   40 - #7c7c7c
     280   40 - #858585
      80   68 - #000000
     120   68 - #000000
      12   76 - #bcbcbc
     164   76 - #010101
     216   76 - #7c7c7c
     280   76 - #858585
     584   76 - #a6a6a6
     584   88 - #a6a6a6
     584  100 - #a6a6a6
      76  104 - #e6e6e7
     584  104 - #a6a6a6
     120  108 - #9b9da0
     584  108 - #a6a6a6
      32  112 - #ffffff
     584  112 - #a6a6a6
     584  116 - #a6a6a6
     584  120 - #a6a6a6
      88  132 - #e0e1e2
     136  136 - #f2f3f3
     160  136 - #787a7e
     188  136 - #909195
     220  136 - #909195
      12  164 - #bcbcbc
      80  180 - #ffffff
     120  180 - #ffffff
     160  184 - #323232
     216  184 - #7c7c7c
     256  184 - #1f1f1f
      24  216 - #bdbdbd
      80  216 - #ffffff
     120  216 - #151515
     452  216 - #ffffff
     120  220 - #151515
     152  220 - #212121
     216  220 - #797979
     288  220 - #ffffff
     120  248 - #d7d7d8
      80  252 - #1a1d24
      32  256 - #bcbcbc
     164  256 - #272a31
      88  280 - #e0e1e2
     120  280 - #909195
     160  280 - #787a7e
     200  280 - #787a7e
     220  280 - #909195
      12  292 - #bcbcbc
      80  320 - #000000
     120  324 - #151515
     212  324 - #2c2c2c
      24  328 - #bdbdbd
     120  328 - #151515
     180  328 - #010101
     236  328 - #999999
     288  328 - #ffffff
     128  332 - #4d4d4d
     132  332 - #4d4d4d
      80  356 - #000000
     532  356 - #ffffff
      24  360 - #bdbdbd
     120  360 - #151515
     212  360 - #2c2c2c
     120  364 - #151515
     152  364 - #212121
     256  364 - #010101
     292  364 - #c1c1c1
      76  392 - #e6e6e7
      24  396 - #bdbdbd
     128  396 - #848689
     164  400 - #272a31
      80  432 - #000000
     104  432 - #808080
     136  432 - #515151
     176  432 - #4d4d4d
     400  440 - #ffffff
      44  456 - #c0c0c0
      88  456 - #e2e2e2
     116  456 - #e9e9e9
     520  484 - #ffffff
       4  592 - #ffffff
     196  592 - #ffffff
     352  592 - #ffffff
     592  592 - #ffffff
";

/// One selection goes over the cells of a recycling table, cells of 2
/// kinds. It is kept as positions in the data, so it stays when its rows
/// scroll out and come back, a drag at the edge scrolls the table, and
/// Cmd or Ctrl with A takes the text of every row.
#[view]
struct TableTextSelection {
    taps: Vec<usize>,

    #[init]
    role:     Label,
    table:    TableView,
    step:     Label,
    selected: Label,
}

impl Setup for TableTextSelection {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);

        self.role
            .set_text("table with selectable text, a label row or a markdown row")
            .set_text_size(14)
            .set_text_color(GRAY)
            .set_alignment(TextAlignment::Left);
        self.role.place().t(8).lr(12).h(20);

        self.table
            .set_data_source(self)
            .register_cell::<LineCell>()
            .register_cell::<ProseCell>();
        self.table.set_variable_heights(true);
        self.table.set_text_selectable(true);
        self.table.set_border_color(GRAY).set_border_width(1);
        self.table.place().t(32).lr(12).h(380);

        self.step
            .set_text("nothing done yet")
            .set_text_size(16)
            .set_alignment(TextAlignment::Left);
        self.step.place().t(420).lr(12).h(24);

        self.selected
            .set_text("selected: nothing")
            .set_multiline(true)
            .set_text_size(14)
            .set_text_color(GRAY)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        self.selected.place().t(446).lr(12).h(150);
    }
}

impl TableData for TableTextSelection {
    fn number_of_cells(&self) -> usize {
        ROWS
    }

    fn cell_height(&self, index: usize) -> f32 {
        if is_prose(index) {
            PROSE_HEIGHT
        } else {
            LINE_HEIGHT
        }
    }

    fn setup_cell(&mut self, index: usize, registry: &mut CellRegistry) -> Weak<dyn View> {
        if is_prose(index) {
            let cell = registry.cell::<ProseCell>();
            cell.number.set_text(index);
            cell.text.set_text(&prose_markdown(index));
            cell
        } else {
            let cell = registry.cell::<LineCell>();
            cell.number.set_text(index);
            cell.text.set_text(line_text(index));
            cell
        }
    }

    fn cell_selected(&mut self, index: usize) {
        self.taps.push(index);
    }
}

impl TableTextSelection {
    fn show(self: Weak<Self>, step: &'static str) -> String {
        from_main(move || {
            let text = TextSelection::text();
            self.step.set_text(step);
            self.selected.set_text(if text.is_empty() {
                "selected: nothing".to_string()
            } else {
                format!("selected: {}", shown(&text))
            });
            text
        })
    }

    /// The label on screen in the cell of `row` that shows `part`.
    fn label(self: Weak<Self>, row: usize, part: &str) -> Weak<Label> {
        let (_, cell) = self
            .table
            .visible_cells()
            .into_iter()
            .find(|(index, _)| *index == row)
            .unwrap_or_else(|| panic!("row {row} is not on screen"));
        label_with(cell, part)
    }
}

impl ViewTest for TableTextSelection {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        from_main(|| UIManager::set_drag_scrolling(false));
        wait_until("the markdown rows are laid out", move || prose_is_laid_out(view))?;

        // From a label row over a markdown row into the next label row.
        let (single, third) = from_main(move || {
            (
                point_of(view.label(1, "single"), "single"),
                point_after(view.label(3, "Row 3"), "Row 3"),
            )
        });
        let over_3_rows = format!("single line of text\n{}\nRow 3", row_copy(2));
        inject_touches(drag(single, third));
        assert_eq!(view.show("drag from row 1 into row 3"), over_3_rows);
        check_colors(CHECK_1)?;

        // The rows of the selection scroll out and their cells go to other
        // rows. The selection is not of the cells, it stays.
        inject_scroll(-1200);
        from_main(move || {
            assert!(
                view.table.visible_cells().iter().all(|(row, _)| *row > 3),
                "the selected rows did not scroll out"
            );
        });
        assert_eq!(view.show("scrolled down, the selected rows are out"), over_3_rows);
        check_colors(CHECK_2)?;

        inject_scroll(1200);
        assert_eq!(
            view.show("scrolled back, the selection is where it was"),
            over_3_rows
        );
        check_colors(CHECK_3)?;

        // A drag that rests under the bottom edge scrolls the table and
        // the selection grows with it.
        let edge = from_main(move || {
            let frame = *view.table.absolute_frame();
            (single.0, frame.max_y() + 30.0)
        });
        inject_touches(format!("{} {} b\n{} {} m", single.0, single.1, edge.0, edge.1));
        wait_until("the drag at the edge scrolls the table", move || {
            view.table.content_offset() < -400.0
        })?;
        inject_touches(format!("{} {} e", edge.0, edge.1));
        // How far the drag got depends on the time it had. The check looks
        // at rows it passed for sure, at a place that is always the same.
        from_main(move || {
            let mut view = view;
            view.table.set_content_offset(-300);
        });
        let grown = view.show("drag from row 1 to under the bottom edge, held there");
        let all_from_row_1 = format!("single line of text\n{}", rows_copy(2..ROWS));
        assert!(
            all_from_row_1.starts_with(&grown),
            "not the text from row 1 on: {grown}"
        );
        assert!(
            grown.starts_with(&format!("single line of text\n{}\n", rows_copy(2..12))),
            "the selection did not grow over the rows that scrolled in: {grown}"
        );
        check_colors(CHECK_4)?;

        // All the text, also of the rows that are not on screen.
        inject_modifiers(ModifiersState::SUPER);
        inject_keys("a");
        assert_eq!(view.show("Cmd or Ctrl with A"), rows_copy(0..ROWS));
        check_colors(CHECK_5)?;

        #[cfg(desktop)]
        from_main(|| Clipboard::set_in_process(true));
        inject_keys("c");
        inject_modifiers(ModifiersState::empty());
        #[cfg(desktop)]
        from_main(|| {
            assert_eq!(
                Clipboard::get_text().expect("the clipboard has the copy"),
                rows_copy(0..ROWS)
            );
        });

        // A click selects nothing and is still a tap on its row.
        let row = from_main(move || view.table.visible_cells().iter().map(|(row, _)| *row).min());
        let row = row.expect("the table shows rows");
        let row = if is_prose(row + 1) { row + 2 } else { row + 1 };
        let text = from_main(move || point_of(view.label(row, "single"), "single"));
        from_main(move || {
            let mut view = view;
            view.taps.clear();
        });
        inject_touches(click(text));
        assert_eq!(view.show("click on the text of a row"), "");
        from_main(move || assert_eq!(view.taps, vec![row]));
        check_colors(CHECK_6)?;

        Ok(())
    }
}

/// The markdown of row 2 has its labels, they are made a turn after the
/// text is set.
fn prose_is_laid_out(view: Weak<TableTextSelection>) -> bool {
    view.table
        .visible_cells()
        .into_iter()
        .filter(|(row, _)| *row == 2)
        .any(|(_, cell)| {
            cell.downcast_view::<ProseCell>()
                .is_some_and(|cell| !cell.text.subviews().is_empty())
        })
}
