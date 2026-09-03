use anyhow::Result;

use super::panel::{
    LED_GREEN, fader, green_lens, grille, hex_bolt, knob, lamp, nameplate, push_button, red_lens, screw,
    seven_segment, vents, vu_meter,
};
use crate::{
    self as hilen,
    deps::{
        hreads::{from_main, wait_for_next_frame},
        refs::Weak,
    },
    gm::{
        color::{BLACK, Color, WHITE},
        flat::{FillRule, Paint, StrokeStyle, VectorPath},
    },
    ui::{DrawingView, Setup, ViewData, ViewTest, view},
    ui_test::{check_colors, set_record_probe_count},
};

/// Pins painter order inside one `DrawingView`: a later path draws over
/// earlier ones at every overlap. The scene is the micbus case, a
/// skeuomorphic hardware panel built from the painters in
/// `panel.rs`, a knob, corner screws, a VU meter, a digit display, a
/// fader, a toggle, a push button, two lamps, a speaker grille, vents,
/// a hex bolt and a nameplate, every piece stacked fills and strokes
/// over the ones below. Every path shares the view's z, so this is the
/// depth compare of the path pipeline under test, not the z ordering
/// between views.
#[view]
struct DrawingLayers {
    #[init]
    drawing: DrawingView,
}

impl Setup for DrawingLayers {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::hex("#26282d"));
        self.drawing.place().back();
        let mut drawing = self.drawing.weak();

        // The panel, one vertical ramp with a vignette over it and a
        // chamfered edge, lit on top, shaded at the bottom.
        drawing.add_fill(
            &VectorPath::polygon([(0.0, 0.0), (600.0, 0.0), (600.0, 600.0), (0.0, 600.0)]),
            Paint::linear((0, 0), (0, 600), Color::hex("#46494f"), Color::hex("#26282d")).grain(0.06),
            FillRule::NonZero,
        );
        drawing.add_fill(
            &VectorPath::polygon([(0.0, 0.0), (600.0, 0.0), (600.0, 600.0), (0.0, 600.0)]),
            Paint::radial((300, 280), 430.0, BLACK.with_alpha(0.0), BLACK.with_alpha(0.25)),
            FillRule::NonZero,
        );
        drawing.add_stroke(
            &VectorPath::polyline([(0.0, 1.0), (600.0, 1.0)]),
            WHITE.with_alpha(0.09),
            StrokeStyle::width(2),
        );
        drawing.add_stroke(
            &VectorPath::polyline([(0.0, 599.0), (600.0, 599.0)]),
            BLACK.with_alpha(0.4),
            StrokeStyle::width(2),
        );

        nameplate(drawing, 80.0, 22.0, 180.0, 26.0);
        knob(drawing, 170.0, 175.0);
        vu_meter(drawing, 300.0, 55.0, 205.0);
        seven_segment(drawing, 300.0, 200.0);

        push_button(drawing, 240.0, 360.0);
        fader(drawing, 70.0, 310.0, 500.0, 420.0);
        lamp(drawing, 150.0, 480.0, LED_GREEN, green_lens(150.0, 480.0), true);
        lamp(
            drawing,
            240.0,
            480.0,
            Color::hex("#e04034"),
            red_lens(240.0, 480.0),
            false,
        );

        grille(drawing, 330.0, 320.0);
        vents(drawing, 320.0, 480.0, 180.0);
        hex_bolt(drawing, 270.0, 565.0);

        screw(drawing, 52.0, 52.0, 40.0);
        screw(drawing, 548.0, 52.0, -25.0);
        screw(drawing, 52.0, 548.0, 10.0);
        screw(drawing, 548.0, 548.0, 75.0);
    }
}

impl ViewTest for DrawingLayers {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            assert_eq!(view.drawing.paths().len(), 242);
        });

        check_colors(COLORS)?;

        Ok(())
    }
}

const COLORS: &str = r"
     316    4 - #393c41
     224   24 - #b4bac1
     100   36 - #9ea3aa
     140   36 - #a0a6ad
     184   40 - #959ba2
     252   44 - #91979d
     528   44 - #73787c
      72   48 - #5b6065
     564   52 - #9fa4aa
      36   56 - #adb2b9
      68   68 - #1e2125
     540   68 - #71767c
     492   72 - #bcb8ad
     428   84 - #4d4b43
     436   84 - #c83f34
     444   88 - #c73c31
     144   92 - #ced0d3
     188   96 - #eff1f3
     376   96 - #e5dfce
     452   96 - #46443b
     148  104 - #e9ebee
     324  104 - #e2dbca
     432  104 - #2d2b26
     224  112 - #dfe2e5
     112  116 - #e3e7ea
     420  128 - #26241e
       4  136 - #34363b
     492  140 - #d7d0bc
     144  144 - #535962
      96  156 - #c7cbd1
     336  168 - #cfc7b3
     404  168 - #2f3237
     180  172 - #4a5059
     256  172 - #9ba0a4
      80  176 - #54575b
     136  184 - #474e57
     252  200 - #898d94
     556  200 - #101214
     456  208 - #4aff82
     104  224 - #9ea4ab
     232  224 - #9ea4aa
     304  236 - #202326
     216  240 - #bfc3c8
     396  240 - #3bff77
     164  248 - #393c40
     176  248 - #3b3d41
     428  264 - #3bff77
       4  268 - #2e3035
     472  272 - #3bff77
     592  308 - #2d2f33
     328  316 - #090b0d
     240  340 - #b35953
     232  344 - #f07167
     244  344 - #ea6359
     248  344 - #dc5d54
     216  348 - #d0d5d9
     236  348 - #f67368
     248  348 - #e24f43
     252  348 - #cd4a40
     408  348 - #121417
      72  352 - #1c1e22
     224  352 - #ea4c40
     228  352 - #f25e52
     232  352 - #fa6f64
     236  352 - #fb7468
     244  352 - #eb5144
     256  352 - #b32e24
     232  356 - #f55b4d
     252  356 - #c6271c
     224  360 - #e12b1d
     236  360 - #ed4537
     248  360 - #d32518
     256  360 - #a41a10
     232  364 - #e43123
     248  364 - #c42115
     252  364 - #af1c12
     224  368 - #b21f14
     240  368 - #c02217
     232  372 - #9a1b13
     248  372 - #851710
     240  376 - #741510
     556  404 - #121417
      60  408 - #e9ecef
      84  424 - #747b82
      72  428 - #7b8289
     456  428 - #090b0d
      56  432 - #555a5f
     136  456 - #e8ebee
     164  456 - #e9ecef
     148  460 - #149138
     220  464 - #d2d7dc
     260  464 - #d6dbe0
     136  468 - #15a13d
     148  468 - #1ed251
     156  468 - #19c549
     148  472 - #44e972
     152  472 - #41e76f
     164  472 - #16ab41
     232  472 - #410f0d
     244  472 - #47100e
     348  476 - #060809
     120  480 - #525559
     140  480 - #31dd61
     144  480 - #67f18d
     160  480 - #28d85a
     240  480 - #5a1512
     248  480 - #48100e
     168  484 - #14963a
     248  484 - #440f0d
     140  488 - #1ac94b
     152  488 - #38e167
     160  488 - #19c249
     236  488 - #46100e
     244  488 - #440f0d
     156  492 - #18bd47
     148  500 - #0f762e
     152  500 - #0f732c
     448  508 - #1b1d22
      60  536 - #b7bdc4
      32  540 - #767a7f
     528  540 - #767a7f
     568  544 - #5a5e63
     380  556 - #15171b
      60  564 - #6e7379
     260  564 - #b0b8bf
     280  568 - #7b8187
     168  592 - #1f2125
     460  592 - #1e2024
";
