use anyhow::{Result, ensure};

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::color::{Color, WHITE},
    ui::{Label, ScrollView, Setup, UIManager, ViewData, ViewFrame, ViewSubviews, ViewTest, view},
    ui_test::{check_colors, inject_touches, system_input::wait_until},
};

const COLUMNS: usize = 6;
const ROWS: usize = 6;
const TILE: f32 = 150.0;

/// The scroll view on the canvas.
const LEFT: f32 = 50.0;
const TOP: f32 = 80.0;
const WIDTH: f32 = 400.0;
const HEIGHT: f32 = 300.0;
/// How far the content can move: 900 of content in 300 and in 400.
const RANGE_Y: f32 = 600.0;
const RANGE_X: f32 = 500.0;

/// The vertical bar: 296 of track, 98.67 long, so it travels 197.33 for
/// the 600 of the content.
const TRAVEL_Y: f32 = 197.33;
/// The sideways bar keeps 6 free for the vertical one: 390 of track,
/// 173.33 long, it travels 216.67 for the 500 of the content.
const TRAVEL_X: f32 = 216.67;

// A scroll view over a grid of tiles bigger than it in both directions.
// A press on a bar and a drag move the content: the bar follows the
// pointer, and the content moves by as much as the bar stands for. The
// bar is 4 points thin, a press a few points beside it still grabs it.
// A finger on the content still drags the content.
#[view]
struct ScrollBarDrag {
    #[init]
    status: Label,
    scroll: ScrollView,
}

impl Setup for ScrollBarDrag {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.status.set_text_size(18).set_multiline(true);
        self.status.set_text("nothing was dragged, both bars are at their start");
        self.status.place().tl(0).size(600, 70);

        self.scroll.set_color(WHITE);
        self.scroll.set_content_size((900, 900));
        self.scroll.place().t(TOP).l(LEFT).size(WIDTH, HEIGHT);

        for row in 0..ROWS {
            for column in 0..COLUMNS {
                let tile = self.scroll.add_view::<Label>();
                tile.set_text(format!("{row} {column}")).set_text_size(32);
                tile.set_color(if (row + column).is_multiple_of(2) {
                    Color::hex("#dfe8f0")
                } else {
                    Color::hex("#a9bdd0")
                });
                tile.set_frame((column * 150, row * 150, TILE, TILE));
            }
        }
    }
}

impl ScrollBarDrag {
    fn say(self: Weak<Self>, text: &'static str) {
        from_main(move || {
            self.status.set_text(text);
        });
        wait_for_next_frame();
    }

    /// The sideways and the vertical offset.
    fn offsets(self: Weak<Self>) -> (f32, f32) {
        wait_for_next_frame();
        from_main(move || {
            (
                self.scroll.content_offset_x(),
                self.scroll.get_scroll_content_offset(),
            )
        })
    }
}

/// A press at `from`, a move in 4 steps to `to` and a release there.
fn drag(from: (f32, f32), to: (f32, f32)) {
    let mut lines = format!("{} {} b\n", from.0, from.1);
    for step in 1..=4_u8 {
        let part = f32::from(step) / 4.0;
        let x = from.0 + (to.0 - from.0) * part;
        let y = from.1 + (to.1 - from.1) * part;
        lines.push_str(&format!("{x} {y} m\n"));
    }
    lines.push_str(&format!("{} {} e", to.0, to.1));
    inject_touches(lines);
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 1.5
}

impl ViewTest for ScrollBarDrag {
    // With drag scrolling on, the default of a touch screen, a press on
    // a bar has to drag the bar and not the content under it.
    fn before_start() {
        UIManager::set_drag_scrolling(true);
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        wait_for_next_frame();
        check_colors(START)?;

        // The vertical bar, grabbed in its middle and moved down by 100.
        let bar_x = LEFT + WIDTH - 4.0;
        let bar_middle = TOP + 2.0 + 49.0;
        drag((bar_x, bar_middle), (bar_x, bar_middle + 100.0));
        let (x, y) = view.offsets();
        let moved = -100.0 / TRAVEL_Y * RANGE_Y;
        ensure!(
            near(y, moved) && near(x, 0.0),
            "the bar moved down by 100: the offsets are {x} and {y}, {moved} was expected"
        );
        view.say("the vertical bar was dragged down by 100\nthe content moved up by 304");
        check_colors(DOWN)?;

        // Far past the end of its track, the content stops at its end.
        let now = bar_middle + 100.0;
        drag((bar_x, now), (bar_x, now + 500.0));
        let (_, y) = view.offsets();
        ensure!(near(y, -RANGE_Y), "the bar past its end: the offset is {y}");
        view.say("the vertical bar was dragged past the end\nthe content is at its end");
        check_colors(BOTTOM)?;

        // A press beside the thin bar, inside the wider area, grabs it too.
        let beside = LEFT + WIDTH - 11.0;
        let now = TOP + HEIGHT - 2.0 - 49.0;
        drag((beside, now), (beside, now - 500.0));
        let (_, y) = view.offsets();
        ensure!(
            near(y, 0.0),
            "a press beside the bar did not grab it: the offset is {y}"
        );
        view.say("a press 7 points left of the bar grabbed it\nit was dragged back to the top");

        // The sideways bar, moved right by 100.
        let bar_y = TOP + HEIGHT - 4.0;
        let bar_middle = LEFT + 2.0 + 86.0;
        drag((bar_middle, bar_y), (bar_middle + 100.0, bar_y));
        let (x, y) = view.offsets();
        let moved = -100.0 / TRAVEL_X * RANGE_X;
        ensure!(
            near(x, moved) && near(y, 0.0),
            "the sideways bar moved by 100: the offsets are {x} and {y}, {moved} was expected"
        );
        view.say("the sideways bar was dragged right by 100\nthe content moved left by 231");
        check_colors(RIGHT)?;

        // A finger on the content still drags the content, the bars took
        // nothing from it.
        drag((LEFT + 200.0, TOP + 250.0), (LEFT + 200.0, TOP + 150.0));
        // The fling after the release moves it some more, the same way
        // on every machine.
        wait_until("the fling after the drag ends", move || {
            from_main(move || !view.scroll.is_scrolling())
        })?;
        let (x_after, y_after) = view.offsets();
        ensure!(
            near(x_after, x) && y_after <= -99.0,
            "a drag over the content by 100: the offsets are {x_after} and {y_after}"
        );
        view.say("a finger dragged the content up by 100 and let it fly\nthe vertical bar followed");
        check_colors(CONTENT)?;

        Ok(())
    }
}

/// Recorded with `--record-colors`.
const START: &str = r"
             116   36 - #151515
             164   36 - #ffffff
             252   36 - #ffffff
             260   36 - #191919
             308   36 - #ffffff
             372   36 - #545454
             384   36 - #191919
             428   36 - #484848
             440   36 - #c5c5c5
             480   36 - #8a8a8a
             200   40 - #010101
             444  104 - #91979c
             264  144 - #010101
             108  152 - #dfe8f0
             408  152 - #dfe8f0
             140  156 - #dfe8f0
             444  176 - #91979c
             196  228 - #dfe8f0
             352  228 - #dfe8f0
              52  244 - #a9bdd0
             140  300 - #a9bdd0
             264  304 - #888e93
             288  304 - #000000
             444  304 - #a9bdd0
             112  312 - #000000
             412  312 - #000000
              52  372 - #a9bdd0
             116  376 - #6e7b87
             180  376 - #6e7b87
             220  376 - #91979c
             328  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const DOWN: &str = r"
             220   20 - #191919
             372   20 - #000000
             460   20 - #ffffff
             312   24 - #ffffff
             148   28 - #7b7b7b
             184   28 - #3c3c3c
             440   28 - #7b7b7b
             192   40 - #232323
             280   44 - #868686
             340   44 - #2c2c2c
             188   48 - #ffffff
             240   48 - #c8c8c8
             408   48 - #464646
              52   80 - #dfe8f0
             140  148 - #dfe8f0
             444  152 - #dfe8f0
             288  156 - #000000
             440  156 - #dfe8f0
             256  160 - #010101
              56  188 - #dfe8f0
             212  224 - #a9bdd0
             444  228 - #6e7b87
             444  276 - #6e7b87
             116  292 - #000000
             288  300 - #000000
             140  304 - #a9bdd0
             440  308 - #a9bdd0
              52  372 - #a9bdd0
             116  376 - #91979c
             184  376 - #91979c
             332  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const BOTTOM: &str = r"
             140   24 - #ffffff
             184   24 - #8a8a8a
             336   24 - #ffffff
             408   24 - #414141
             460   24 - #ffffff
             220   28 - #acacac
             308   28 - #565656
             368   32 - #6f6f6f
             268   48 - #000000
             304   48 - #656565
             392   48 - #bbbbbb
             440  144 - #000000
             140  152 - #dfe8f0
             264  152 - #22272a
             264  156 - #22272a
             412  156 - #dfe8f0
             108  160 - #56595c
             288  160 - #000000
             348  228 - #a9bdd0
             212  240 - #dfe8f0
             132  300 - #010101
             288  304 - #000000
             412  304 - #a9bdd0
             444  308 - #6e7b87
              52  312 - #a9bdd0
             200  372 - #dfe8f0
             444  372 - #6e7b87
             112  376 - #6e7b87
             208  376 - #91979c
             220  376 - #91979c
               4  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const RIGHT: &str = r"
             408   24 - #525252
             444   24 - #aeaeae
             164   28 - #ffffff
             340   28 - #868686
             188   40 - #3e3e3e
             368   40 - #a2a2a2
             204   48 - #ffffff
             252   48 - #ffffff
             336   48 - #5f5f5f
             352   48 - #6f6f6f
             444  108 - #91979c
             176  148 - #000000
             336  148 - #000000
              56  156 - #778592
             332  156 - #a9bdd0
             212  164 - #000000
             328  164 - #000000
             448  188 - #dfe8f0
             120  228 - #dfe8f0
             248  228 - #dfe8f0
             424  284 - #a9bdd0
             328  296 - #000000
              56  300 - #9da3a8
             180  304 - #8797a6
             120  376 - #a9bdd0
             180  376 - #6e7b87
             240  376 - #6e7b87
             320  376 - #91979c
             448  376 - #a9bdd0
               4  592 - #ffffff
             344  592 - #ffffff
             592  592 - #ffffff
";

/// Recorded with `--record-colors`.
const CONTENT: &str = r"
             400   20 - #ffffff
             492   20 - #e4e4e4
             136   24 - #ffffff
             192   24 - #ffffff
             320   24 - #414141
             328   24 - #cecece
             240   28 - #000000
             284   28 - #3c3c3c
             428   28 - #959595
             492   28 - #e4e4e4
             340   32 - #a2a2a2
             184  144 - #000000
             336  148 - #67737e
             184  152 - #000000
             336  152 - #67737e
              56  156 - #778592
             332  156 - #a9bdd0
             176  160 - #56595c
             180  160 - #56595c
             256  228 - #dfe8f0
             448  228 - #dfe8f0
             156  264 - #a9bdd0
              56  300 - #9da3a8
             212  300 - #010101
             336  304 - #000000
              56  308 - #9da3a8
              56  312 - #9da3a8
             196  372 - #a9bdd0
             444  372 - #6e7b87
             284  376 - #91979c
               4  592 - #ffffff
             592  592 - #ffffff
";
