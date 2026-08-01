use anyhow::Result;
use hreads::{from_main, wait_for_next_frame};
use refs::Weak;

use crate::{
    self as test_engine,
    ui::{Setup, Switch, ViewData, ViewTest, view},
    ui_test::{check_colors, inject_touches},
};

/// Pins the iOS style switch look: rounded gray track with a white round
/// knob on the left when off, green track with the knob on the right
/// when on. Also covers the public `set_on`.
#[view]
struct SwitchLook {
    #[init]
    off: Switch,
    on:  Switch,
}

impl Setup for SwitchLook {
    fn setup(mut self: Weak<Self>) {
        self.off.place().t(20).l(20).size(64, 32);
        self.on.place().t(80).l(20).size(64, 32);
        self.on.set_on(true);
    }
}

impl ViewTest for SwitchLook {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_for_next_frame();
        wait_for_next_frame();

        from_main(move || {
            assert!(!view.off.on());
            assert!(view.on.on());
        });

        check_colors(
            r"
             340    4 - #597c95
              40   20 - #bbbbbc
              64   20 - #d0d0d1
              52   24 - #c8c8c9
              56   28 - #cccccd
              76   28 - #d0d0d1
              24   32 - #ffffff
              52   32 - #b3b3b4
              36   36 - #ffffff
              68   36 - #d0d0d1
              56   40 - #c2c2c3
              80   40 - #d0d0d1
              20   44 - #49667b
              52   48 - #bbbbbc
              68   48 - #d0d0d1
              36   52 - #415b6d
             592   76 - #597c95
              40   80 - #09921a
              72   84 - #ffffff
              24   88 - #09921a
              56   92 - #ffffff
              84  100 - #4a677c
              24  104 - #09921a
              52  104 - #077915
              40  108 - #09921a
              68  108 - #ffffff
              80  108 - #476478
              60  112 - #476376
             348  336 - #597c95
               4  360 - #597c95
             104  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        inject_touches(
            "
            52 36 b
            52 36 e
        ",
        );

        from_main(move || {
            assert!(view.off.on());
        });

        check_colors(
            r"
             592    4 - #597c95
              44   20 - #09921a
              28   24 - #09921a
              72   24 - #ffffff
              56   32 - #ffffff
              24   40 - #09921a
              68   40 - #ffffff
              84   40 - #4a677c
              36   48 - #09921a
              80   48 - #476478
             340   48 - #597c95
              64   52 - #435d70
              76   52 - #486579
              56   80 - #098c19
              24   88 - #09921a
              72   88 - #ffffff
              40   92 - #09921a
              64   92 - #ffffff
              84   96 - #4a677c
              56  100 - #ffffff
              68  100 - #ffffff
              76  104 - #ffffff
              36  108 - #09921a
              80  108 - #476478
              64  112 - #435d70
              76  112 - #486579
             300  300 - #597c95
             592  300 - #597c95
              64  356 - #597c95
               4  592 - #597c95
             300  592 - #597c95
             592  592 - #597c95
            ",
        )?;

        from_main(move || {
            let mut this = view;
            this.off.set_on(false);
        });

        from_main(move || {
            assert!(!view.off.on());
        });

        Ok(())
    }
}
