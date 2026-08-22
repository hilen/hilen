use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{
        BLACK, BLUE, Container, CornerRadii, GREEN, LIGHTER_GRAY, RED, Setup, Shadow, ViewData, ViewTest,
        WHITE, YELLOW, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

#[view]
struct ShadowTest {
    #[init]
    under_ghost: Container,
    plain:       Container,
    offset_card: Container,
    round:       Container,
    red_glow:    Container,
    ghost:       Container,
}

impl Setup for ShadowTest {
    fn setup(self: Weak<Self>) {
        self.plain.set_color(GREEN);
        self.plain.place().tl(60).size(120, 80);
        self.plain.set_shadow(Shadow {
            offset: (0, 0).into(),
            radius: 30.0,
            color:  BLACK,
        });

        self.offset_card.set_color(BLUE);
        self.offset_card.place().t(60).l(300).size(120, 80);
        self.offset_card.set_shadow(Shadow {
            offset: (25, 25).into(),
            radius: 20.0,
            color:  BLACK,
        });

        self.round.set_color(YELLOW);
        self.round.place().t(250).l(60).size(100, 100);
        self.round.set_corner_radii(CornerRadii::all(40));
        self.round.set_shadow(Shadow {
            offset: (0, 0).into(),
            radius: 25.0,
            color:  BLACK.with_alpha(0.8),
        });

        self.red_glow.set_color(WHITE);
        self.red_glow.place().t(250).l(300).size(100, 100);
        self.red_glow.set_shadow(Shadow {
            offset: (0, 0).into(),
            radius: 30.0,
            color:  RED,
        });

        // A hidden view must not cast a shadow. The backdrop catches
        // probes where the shadow would fall, pinning its absence.
        self.under_ghost.set_color(LIGHTER_GRAY);
        self.under_ghost.place().t(430).l(40).size(140, 140);

        self.ghost.set_color(GREEN);
        self.ghost.place().t(450).l(60).size(100, 100);
        self.ghost.set_shadow(Shadow {
            offset: (0, 0).into(),
            radius: 30.0,
            color:  BLACK,
        });
        self.ghost.set_hidden(true);
    }
}

impl ViewTest for ShadowTest {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);

        check_colors(
            r"
             592    4 - #597c95
              52   48 - #49667a
             100   48 - #456073
             140   56 - #344957
             172   56 - #344957
             304   60 - #0000e7
             344   60 - #0000e7
             416   60 - #0000e7
             380   64 - #0000e7
              88   84 - #00ff00
             184   84 - #364c5b
             440   88 - #212e37
             360   96 - #0000e7
              60  108 - #00ff00
             160  108 - #00ff00
             424  108 - #000000
             128  112 - #00ff00
             300  116 - #0000e7
             420  128 - #000000
             440  128 - #1e2932
              80  136 - #00ff00
             176  136 - #00ff00
             348  140 - #000000
             372  140 - #000000
             404  140 - #000000
             140  144 - #364c5b
             328  144 - #212e37
             432  156 - #12191e
             356  160 - #1e2932
             380  160 - #1e2932
             328  168 - #384e5e
             364  240 - #865a6d
              80  244 - #49657a
             328  244 - #954f5f
             120  252 - #ffff00
             400  252 - #aa404c
             160  260 - #4a677c
             592  260 - #597c95
             296  264 - #9e4957
              84  276 - #ffff00
             408  276 - #8a5869
              52  280 - #476376
             112  292 - #ffff00
             160  300 - #364c5b
             280  300 - #667289
             408  304 - #8a5869
             296  320 - #9e4957
              64  324 - #ffff00
             108  324 - #ffff00
             144  336 - #ffff00
             288  344 - #7f6073
             368  352 - #a24654
             100  356 - #435d70
             408  356 - #815e71
             328  360 - #825d70
             560  428 - #597c95
             112  432 - #f3f3f3
              40  444 - #f3f3f3
             176  460 - #f3f3f3
             100  504 - #f3f3f3
              40  568 - #f3f3f3
             152  568 - #f3f3f3
             364  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        Ok(())
    }
}
