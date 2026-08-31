use anyhow::Result;

use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{
        color::{BLACK, Color, WHITE},
        flat::{FillRule, LineJoin, Paint, StrokeStyle, VectorPath},
    },
    ui::{DrawingView, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, set_record_probe_count},
};

/// Pins every `Paint` a path can carry. A horizontal and a diagonal
/// linear ramp, a radial ramp, a conic ramp with four repeats, a
/// gradient on a stroke, a radial fading to a transparent stop, the
/// soft shadow case, a flat path through the same `Paint` route, a
/// four stop glossy bar, and a grained brushed metal disc.
#[view]
struct DrawingGradients {
    #[init]
    drawing: DrawingView,
}

const RED: Color = Color::hex("#e03131");
const YELLOW: Color = Color::hex("#ffd43b");
const BLUE: Color = Color::hex("#1971c2");
const GREEN: Color = Color::hex("#2f9e44");

fn rect(x: f32, y: f32, width: f32, height: f32) -> VectorPath {
    VectorPath::polygon([(x, y), (x + width, y), (x + width, y + height), (x, y + height)])
}

impl Setup for DrawingGradients {
    fn setup(mut self: Weak<Self>) {
        self.set_color(WHITE);
        self.drawing.place().back();

        // A horizontal linear ramp, and a diagonal one whose ends sit
        // inside the shape so both clamps show.
        self.drawing.add_fill(
            &rect(20.0, 20.0, 260.0, 70.0),
            Paint::linear((20, 0), (280, 0), RED, YELLOW),
            FillRule::NonZero,
        );
        self.drawing.add_fill(
            &rect(320.0, 20.0, 260.0, 70.0),
            Paint::linear((370, 40), (530, 70), BLUE, GREEN),
            FillRule::NonZero,
        );

        // A radial ramp centered off middle, like a lit dome.
        self.drawing.add_fill(
            &VectorPath::circle((150, 300), 100),
            Paint::radial((130, 275), 120.0, YELLOW, RED),
            FillRule::NonZero,
        );

        // A conic ramp, four light to dark cycles per turn, the turned
        // metal sheen.
        self.drawing.add_fill(
            &VectorPath::circle((450, 300), 100),
            Paint::conic((450, 300), 4.0, Color::hex("#e9ecef"), Color::hex("#868e96")),
            FillRule::NonZero,
        );

        // A gradient on a stroke.
        self.drawing.add_stroke(
            &VectorPath::polyline([(30.0, 430.0), (570.0, 430.0)]),
            Paint::linear((30, 0), (570, 0), GREEN, BLUE),
            StrokeStyle::width(24).join(LineJoin::Round),
        );

        // A radial ramp into a transparent stop over a flat path drawn
        // through the same `Paint` route, the soft shadow case. The
        // probes on its rim must read the flat color mixed with the
        // background, not black.
        self.drawing.add_fill(
            &rect(180.0, 480.0, 240.0, 100.0),
            Paint::flat(GREEN),
            FillRule::NonZero,
        );
        self.drawing.add_fill(
            &VectorPath::circle((300, 530), 70),
            Paint::radial((300, 530), 70.0, BLACK.with_alpha(0.6), BLACK.with_alpha(0.0)),
            FillRule::NonZero,
        );

        // A four stop glossy bar, the iOS button ramp, bright top,
        // hard mid step, reflected light at the bottom.
        self.drawing.add_fill(
            &rect(20.0, 115.0, 260.0, 50.0),
            Paint::linear((0, 115), (0, 165), Color::hex("#9be2ff"), Color::hex("#7ec8f4"))
                .stop(Color::hex("#3b9de4"), 0.5)
                .stop(Color::hex("#1d7fd0"), 0.52),
            FillRule::NonZero,
        );

        // The brushed metal disc, a conic sheen under angle following
        // grain that streaks along the radius.
        self.drawing.add_fill(
            &VectorPath::circle((510, 530), 50),
            Paint::conic((510, 530), 3.0, Color::hex("#dde1e5"), Color::hex("#9099a1")).grain(0.25),
            FillRule::NonZero,
        );
    }
}

impl ViewTest for DrawingGradients {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(96);

        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            assert_eq!(view.drawing.paths().len(), 9);
        });

        check_colors(COLORS)?;

        Ok(())
    }
}

const COLORS: &str = r"
     104   20 - #ea6634
     236   20 - #fab939
     264   20 - #fdca3a
     388   20 - #1b75b7
     460   20 - #258980
     532   20 - #2e9c49
     276   40 - #ffd23b
      20   44 - #e03131
     180   56 - #f39637
     276   64 - #ffd23b
     220   68 - #f8af39
     260   80 - #fdc83a
     404   80 - #1f7ca2
     344   88 - #1971c2
     468   88 - #278e70
     572   88 - #2f9e44
      68  116 - #95defd
     148  116 - #95defd
     212  116 - #95defd
      20  132 - #58b2ec
     276  132 - #58b2ec
     100  136 - #48a7e8
      40  140 - #2c8eda
     168  140 - #2c8eda
     240  140 - #2c8eda
     132  144 - #2b8ad5
     200  148 - #3b96db
      60  152 - #4ba2e1
      84  152 - #4ba2e1
      32  160 - #6cbaed
     260  160 - #6cbaed
     228  164 - #7cc6f3
     592  164 - #ffffff
     144  200 - #f09268
     156  200 - #f08f67
     444  200 - #aab0b5
     496  212 - #c4c8cd
       4  224 - #ffffff
      84  228 - #ee7c36
     220  232 - #e54c33
     528  244 - #d4d8dc
     472  248 - #babfc5
     168  252 - #f39737
     416  252 - #d3d8dc
     360  260 - #bac0c5
     124  280 - #fdc93a
     248  284 - #e86665
     548  284 - #b3b9be
     496  292 - #9aa1a8
      52  300 - #ea6534
     164  300 - #f49a37
     444  300 - #9199a0
     392  304 - #90979f
     236  340 - #e03131
     540  340 - #bbc0c6
     484  344 - #d9dde1
     140  352 - #eb6a34
     368  352 - #ced2d7
     428  352 - #b7bdc2
      84  372 - #e34232
     192  376 - #e03231
     512  376 - #dce0e4
     456  396 - #8e969e
     256  420 - #268b79
      88  428 - #2d9952
      32  432 - #2f9e45
     200  432 - #28906c
     504  436 - #1c76b3
     568  436 - #1971c2
     144  440 - #2a945f
     320  440 - #238688
     396  440 - #207f9a
     228  480 - #2f9e44
     296  488 - #247833
      84  496 - #ffffff
     492  500 - #9099a1
     264  504 - #247a35
     340  512 - #257b35
     300  516 - #185123
     472  516 - #d6dce2
     552  516 - #a4abb2
     284  536 - #1a5625
     316  536 - #1a5726
     464  536 - #eaeff4
     300  544 - #195324
     480  548 - #c7cfd5
     468  552 - #cbd2d9
     540  556 - #aeb2b7
     300  564 - #216e2f
     500  564 - #9aa1a8
     396  568 - #2f9e44
     180  576 - #2f9e44
      92  584 - #ffffff
     276  584 - #e8e8e8
     324  584 - #e9e9e9
       4  592 - #ffffff
";
