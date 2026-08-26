use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLUE, Container, GREEN, Setup, Tooltip, ViewData, ViewFrame, ViewTest, ViewTooltip, ViewTouch, view,
    },
    ui_test::{check_colors, inject_long_press, inject_touches, set_record_probe_count},
};

const HELD: &str = r"
            292    4 - #597c95
            592    4 - #597c95
            476    8 - #597c95
            44   20 - #0000e7
            76   20 - #0000e7
            96   20 - #0000e7
            112   20 - #0000e7
            136   20 - #0000e7
            20   24 - #0000e7
            124   24 - #0000e7
            60   28 - #0000e7
            72   32 - #0000e7
            88   32 - #0000e7
            36   36 - #0000e7
            120   36 - #0000e7
            104   40 - #0000e7
            48   44 - #0000e7
            64   44 - #0000e7
            132   44 - #0000e7
            24   48 - #0000e7
            76   48 - #0000e7
            92   52 - #0000e7
            116   52 - #0000e7
            56   56 - #0000e7
            104   56 - #0000e7
            136   56 - #0000e7
            20   60 - #0000e7
            40   60 - #0000e7
            68   60 - #0000e7
            52   68 - #0000e7
            84   68 - #333338
            96   68 - #333338
            104   68 - #333338
            112   68 - #333338
            124   68 - #333338
            132   68 - #333338
            140   68 - #333338
            152   68 - #333338
            164   68 - #333338
            172   68 - #333338
            28   72 - #0000e7
            136   72 - #333338
            148   72 - #333338
            156   72 - #333338
            176   72 - #333338
            40   76 - #0000e7
            72   76 - #333338
            84   76 - #333338
            92   76 - #333338
            100   76 - #333338
            132   76 - #333338
            144   76 - #333338
            160   76 - #4b4b4f
            168   76 - #333338
            180   76 - #4a677b
            84   80 - #55555a
            88   80 - #616165
            112   80 - #aaaaac
            120   80 - #f3f3f3
            128   80 - #333338
            136   80 - #333338
            152   80 - #333338
            160   80 - #4b4b4f
            172   80 - #333338
            180   80 - #4a677b
            76   84 - #333338
            84   84 - #333338
            124   84 - #333338
            140   84 - #333338
            144   84 - #333338
            148   84 - #333338
            152   84 - #333338
            176   84 - #333338
            180   84 - #4a677b
            96   88 - #333338
            116   88 - #333338
            132   88 - #333338
            144   88 - #333338
            160   88 - #333338
            172   88 - #333338
            180   88 - #4a677b
            392   88 - #597c95
            80   92 - #3c5364
            92   92 - #3c5364
            108   92 - #3c5364
            124   92 - #3c5364
            136   92 - #3c5364
            156   92 - #3c5364
            164   92 - #3c5364
            168   92 - #3c5364
            172   92 - #3c5364
            176   92 - #40596b
            180   92 - #4a677c
            72   96 - #4a677b
            88   96 - #476377
            100   96 - #476377
            116   96 - #476377
            128   96 - #476377
            140   96 - #476377
            148   96 - #476377
            156   96 - #476377
            160   96 - #476377
            164   96 - #476377
            168   96 - #476377
            172   96 - #476377
            176   96 - #486579
            540   96 - #597c95
            288  112 - #597c95
            40  120 - #00ff00
            88  120 - #00ff00
            104  120 - #00ff00
            120  120 - #00ff00
            20  124 - #00ff00
            136  124 - #00ff00
            68  128 - #00ff00
            52  132 - #00ff00
            36  136 - #00ff00
            80  136 - #00ff00
            96  136 - #00ff00
            120  136 - #00ff00
            136  140 - #00ff00
            20  144 - #00ff00
            64  144 - #00ff00
            48  148 - #00ff00
            84  152 - #00ff00
            116  152 - #00ff00
            100  156 - #00ff00
            28  160 - #00ff00
            68  164 - #00ff00
            136  164 - #00ff00
            44  172 - #00ff00
            92  172 - #00ff00
            20  176 - #00ff00
            60  176 - #00ff00
            76  176 - #00ff00
            116  176 - #00ff00
            132  176 - #00ff00
            472  180 - #597c95
            592  192 - #597c95
            292  220 - #597c95
            192  260 - #597c95
            4  276 - #597c95
            548  296 - #597c95
            436  300 - #597c95
            96  312 - #597c95
            300  316 - #597c95
            220  368 - #597c95
            488  392 - #597c95
            348  396 - #597c95
            592  404 - #597c95
            4  412 - #597c95
            124  464 - #597c95
            248  492 - #597c95
            500  496 - #597c95
            372  524 - #597c95
            4  564 - #597c95
            152  592 - #597c95
            288  592 - #597c95
            456  592 - #597c95
            592  592 - #597c95
";

#[view]
struct TooltipLongPress {
    secondary: usize,

    #[init]
    hint:   Container,
    action: Container,
}

impl Setup for TooltipLongPress {
    fn setup(self: Weak<Self>) {
        self.hint.set_color(BLUE);
        self.hint.place().tl(20).size(120, 60);
        self.hint.set_tooltip("Held for a while");

        // A view with a secondary action keeps the hold for itself.
        self.action.set_color(GREEN);
        self.action.place().t(120).l(20).size(120, 60);
        self.action.set_tooltip("Never shown by a hold");
        self.action.enable_touch();
        self.action.touch().secondary.sub(self, move || {
            let mut this = self;
            this.secondary += 1;
        });
    }
}

impl ViewTest for TooltipLongPress {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);

        inject_long_press(60, 50);
        from_main(|| {
            let shown = Tooltip::shown();
            assert!(shown.is_ok());
            assert_eq!(shown.frame().origin, (72, 68).into());
        });

        check_colors(HELD)?;

        // The next press anywhere hides it.
        inject_touches("300 300 b\n300 300 e");
        from_main(|| assert!(Tooltip::shown().is_null()));

        inject_long_press(60, 150);
        from_main(move || {
            assert!(Tooltip::shown().is_null());
            assert_eq!(view.secondary, 1);
        });

        Ok(())
    }
}
