use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLUE, Container, GREEN, Hover, Label, ORANGE, Setup, Tooltip, ViewData, ViewFrame, ViewTest,
        ViewTooltip, WHITE, view,
    },
    ui_test::{check_colors, inject_touches, set_record_probe_count, wait_for_tooltip},
};

const TEXT_TOOLTIP: &str = r"
            484    4 - #597c95
            44   20 - #0000e7
            76   20 - #0000e7
            104   20 - #0000e7
            124   20 - #0000e7
            180   20 - #00ff00
            208   20 - #00ff00
            252   20 - #00ff00
            276   20 - #00ff00
            20   24 - #0000e7
            160   24 - #00ff00
            228   24 - #00ff00
            132   36 - #0000e7
            200   36 - #00ff00
            264   36 - #00ff00
            60   40 - #0000e7
            184   40 - #00ff00
            240   40 - #00ff00
            88   44 - #0000e7
            112   44 - #0000e7
            32   48 - #0000e7
            160   48 - #00ff00
            212   48 - #00ff00
            228   48 - #00ff00
            276   52 - #00ff00
            68   56 - #0000e7
            256   56 - #00ff00
            236   60 - #00ff00
            592   60 - #597c95
            204   64 - #00ff00
            48   68 - #0000e7
            84   68 - #333338
            92   68 - #333338
            104   68 - #333338
            116   68 - #333338
            128   68 - #333338
            148   68 - #333338
            156   68 - #333338
            164   68 - #333338
            176   68 - #333338
            188   68 - #333338
            264   68 - #00ff00
            76   72 - #333338
            140   72 - #333338
            184   72 - #333338
            220   72 - #00ff00
            392   72 - #597c95
            20   76 - #0000e7
            84   76 - #5d5d61
            92   76 - #333338
            96   76 - #c2c2c4
            104   76 - #333338
            108   76 - #333338
            112   76 - #e1e1e2
            120   76 - #333338
            124   76 - #333338
            156   76 - #333338
            164   76 - #333338
            168   76 - #555559
            176   76 - #333338
            192   76 - #333338
            236   76 - #00ff00
            252   76 - #00ff00
            276   76 - #00ff00
            84   80 - #7d7d80
            88   80 - #cfcfd0
            96   80 - #c2c2c4
            116   80 - #ffffff
            160   80 - #333338
            176   80 - #333338
            180   80 - #333338
            188   80 - #333338
            72   84 - #333338
            84   84 - #333338
            132   84 - #8d8d90
            148   84 - #333338
            152   84 - #333338
            156   84 - #333338
            164   84 - #333338
            180   84 - #333338
            184   84 - #333338
            196   84 - #476377
            92   88 - #333338
            100   88 - #333338
            120   88 - #333338
            140   88 - #333338
            172   88 - #333338
            192   88 - #333338
            80   92 - #3c5364
            112   92 - #3c5364
            128   92 - #3c5364
            152   92 - #3c5364
            160   92 - #3c5364
            168   92 - #3c5364
            180   92 - #3c5364
            188   92 - #3c5364
            196   92 - #476478
            72   96 - #4a677b
            88   96 - #476377
            96   96 - #476377
            104   96 - #476377
            120   96 - #476377
            140   96 - #476377
            148   96 - #476377
            156   96 - #476377
            164   96 - #476377
            172   96 - #476377
            184   96 - #476377
            192   96 - #476478
            496  116 - #597c95
            40  120 - #ffcb00
            88  120 - #ffcb00
            116  120 - #ffcb00
            136  120 - #ffcb00
            20  124 - #ffcb00
            68  128 - #ffcb00
            96  136 - #ffcb00
            120  136 - #ffcb00
            136  136 - #ffcb00
            20  144 - #ffcb00
            48  144 - #ffcb00
            76  144 - #ffcb00
            116  152 - #ffcb00
            92  156 - #ffcb00
            36  160 - #ffcb00
            68  160 - #ffcb00
            136  164 - #ffcb00
            20  176 - #ffcb00
            48  176 - #ffcb00
            72  176 - #ffcb00
            88  176 - #ffcb00
            112  176 - #ffcb00
            408  184 - #597c95
            592  184 - #597c95
            268  232 - #597c95
            4  284 - #597c95
            444  292 - #597c95
            112  312 - #597c95
            240  336 - #597c95
            348  360 - #597c95
            536  360 - #597c95
            4  396 - #597c95
            180  428 - #597c95
            292  460 - #597c95
            420  476 - #597c95
            4  524 - #597c95
            208  540 - #597c95
            564  540 - #0000e7
            592  540 - #0000e7
            540  544 - #0000e7
            580  552 - #0000e7
            568  564 - #0000e7
            544  568 - #0000e7
            592  568 - #0000e7
            320  572 - #597c95
            428  584 - #597c95
            560  588 - #0000e7
            108  592 - #597c95
            536  592 - #597c95
            588  592 - #0000e7
";

const CARD_TOOLTIP: &str = r"
            460    4 - #597c95
            592    4 - #597c95
            24   20 - #0000e7
            44   20 - #0000e7
            76   20 - #0000e7
            96   20 - #0000e7
            168   20 - #00ff00
            188   20 - #00ff00
            212   20 - #00ff00
            248   20 - #00ff00
            276   20 - #00ff00
            128   28 - #0000e7
            60   32 - #0000e7
            112   32 - #0000e7
            200   32 - #00ff00
            232   32 - #00ff00
            160   36 - #00ff00
            216   36 - #00ff00
            256   36 - #00ff00
            272   36 - #00ff00
            84   44 - #0000e7
            132   44 - #0000e7
            180   44 - #00ff00
            228   44 - #00ff00
            32   48 - #0000e7
            68   48 - #0000e7
            108   48 - #0000e7
            204   48 - #00ff00
            240   48 - #00ff00
            260   48 - #00ff00
            276   48 - #00ff00
            52   52 - #0000e7
            160   56 - #00ff00
            224   56 - #00ff00
            136   60 - #0000e7
            188   60 - #00ff00
            248   60 - #00ff00
            264   60 - #00ff00
            88   64 - #0000e7
            208   64 - #00ff00
            68   68 - #0000e7
            176   68 - #00ff00
            44   72 - #0000e7
            196   72 - #00ff00
            20   76 - #0000e7
            104   76 - #0000e7
            124   76 - #0000e7
            160   76 - #00ff00
            216   76 - #00ff00
            236   76 - #00ff00
            256   76 - #00ff00
            276   76 - #00ff00
            384   80 - #597c95
            500   96 - #597c95
            24  120 - #ffcb00
            44  120 - #ffcb00
            68  120 - #ffcb00
            104  120 - #ffcb00
            120  120 - #ffcb00
            80  124 - #ffcb00
            136  124 - #ffcb00
            92  128 - #ffcb00
            36  132 - #ffcb00
            56  132 - #ffcb00
            20  136 - #ffcb00
            124  136 - #ffcb00
            592  136 - #597c95
            80  140 - #ffcb00
            112  140 - #ffcb00
            136  140 - #ffcb00
            64  144 - #ffcb00
            96  144 - #ffcb00
            40  148 - #ffcb00
            124  152 - #ffcb00
            84  156 - #ffcb00
            36  160 - #ffcb00
            56  160 - #ffcb00
            108  160 - #ffcb00
            348  160 - #597c95
            20  164 - #ffcb00
            72  164 - #ffcb00
            96  164 - #ffcb00
            132  164 - #ffcb00
            200  168 - #ffffff
            220  168 - #ffffff
            240  168 - #ffffff
            264  168 - #ffffff
            44  172 - #ffcb00
            28  176 - #ffcb00
            68  176 - #ffcb00
            92  176 - #ffffff
            116  176 - #ffffff
            432  176 - #597c95
            140  180 - #ffffff
            160  180 - #a5a5a5
            176  180 - #3a3a3a
            252  180 - #ffffff
            160  184 - #a5a5a5
            176  184 - #000000
            196  184 - #ffffff
            212  188 - #ffffff
            232  188 - #ffffff
            268  188 - #ffffff
            72  192 - #ffffff
            88  196 - #ffffff
            116  196 - #ffffff
            140  200 - #ffffff
            192  200 - #ffffff
            208  204 - #ffffff
            224  204 - #ffffff
            244  204 - #ffffff
            72  208 - #ffffff
            264  208 - #ffffff
            92  212 - #ffffff
            164  212 - #0f0f0f
            168  212 - #2a2a2a
            172  212 - #414141
            108  220 - #ffffff
            220  220 - #ffffff
            80  224 - #ffffff
            132  224 - #ffffff
            152  224 - #ffffff
            196  224 - #ffffff
            240  224 - #ffffff
            268  224 - #ffffff
            500  244 - #597c95
            368  264 - #597c95
            592  272 - #597c95
            104  320 - #597c95
            4  328 - #597c95
            472  336 - #597c95
            200  352 - #597c95
            324  364 - #597c95
            588  404 - #597c95
            96  420 - #597c95
            256  420 - #597c95
            336  448 - #597c95
            512  452 - #597c95
            420  460 - #597c95
            4  480 - #597c95
            172  488 - #597c95
            272  508 - #597c95
            84  528 - #597c95
            456  536 - #597c95
            592  536 - #597c95
            544  540 - #0000e7
            568  540 - #0000e7
            556  552 - #0000e7
            592  560 - #0000e7
            540  564 - #0000e7
            568  564 - #0000e7
            552  576 - #0000e7
            580  576 - #0000e7
            564  588 - #0000e7
            592  588 - #0000e7
            4  592 - #597c95
            152  592 - #597c95
            300  592 - #597c95
            392  592 - #597c95
            540  592 - #0000e7
";

const EDGE_TOOLTIP: &str = r"
            4    4 - #597c95
            500    4 - #597c95
            92   20 - #0000e7
            116   20 - #0000e7
            176   20 - #00ff00
            204   20 - #00ff00
            244   20 - #00ff00
            260   20 - #00ff00
            276   20 - #00ff00
            20   24 - #0000e7
            68   24 - #0000e7
            160   24 - #00ff00
            44   28 - #0000e7
            228   28 - #00ff00
            100   32 - #0000e7
            136   32 - #0000e7
            192   32 - #00ff00
            252   32 - #00ff00
            268   36 - #00ff00
            28   40 - #0000e7
            88   40 - #0000e7
            124   40 - #0000e7
            180   40 - #00ff00
            216   40 - #00ff00
            108   44 - #0000e7
            160   44 - #00ff00
            200   44 - #00ff00
            240   44 - #00ff00
            68   48 - #0000e7
            260   48 - #00ff00
            276   48 - #00ff00
            44   52 - #0000e7
            96   52 - #0000e7
            224   52 - #00ff00
            20   56 - #0000e7
            128   56 - #0000e7
            192   56 - #00ff00
            248   56 - #00ff00
            80   60 - #0000e7
            112   60 - #0000e7
            160   60 - #00ff00
            208   60 - #00ff00
            60   64 - #0000e7
            176   64 - #00ff00
            92   68 - #0000e7
            228   68 - #00ff00
            260   68 - #00ff00
            196   72 - #00ff00
            400   72 - #597c95
            24   76 - #0000e7
            48   76 - #0000e7
            72   76 - #0000e7
            112   76 - #0000e7
            136   76 - #0000e7
            160   76 - #00ff00
            216   76 - #00ff00
            240   76 - #00ff00
            276   76 - #00ff00
            592   84 - #597c95
            48  120 - #ffcb00
            76  120 - #ffcb00
            96  120 - #ffcb00
            20  124 - #ffcb00
            116  124 - #ffcb00
            60  128 - #ffcb00
            136  128 - #ffcb00
            496  128 - #597c95
            36  132 - #ffcb00
            72  136 - #ffcb00
            92  136 - #ffcb00
            52  140 - #ffcb00
            120  140 - #ffcb00
            40  148 - #ffcb00
            104  148 - #ffcb00
            24  152 - #ffcb00
            64  152 - #ffcb00
            132  152 - #ffcb00
            84  156 - #ffcb00
            48  160 - #ffcb00
            100  160 - #ffcb00
            116  160 - #ffcb00
            324  160 - #597c95
            32  164 - #ffcb00
            72  164 - #ffcb00
            136  168 - #ffcb00
            104  172 - #ffcb00
            20  176 - #ffcb00
            40  176 - #ffcb00
            60  176 - #ffcb00
            80  176 - #ffcb00
            124  176 - #ffcb00
            412  192 - #597c95
            592  208 - #597c95
            500  228 - #597c95
            264  232 - #597c95
            176  248 - #597c95
            348  268 - #597c95
            4  284 - #597c95
            436  300 - #597c95
            592  304 - #597c95
            112  312 - #597c95
            236  336 - #597c95
            344  360 - #597c95
            520  368 - #597c95
            428  388 - #597c95
            4  396 - #597c95
            180  428 - #597c95
            592  440 - #597c95
            84  460 - #597c95
            288  460 - #597c95
            420  476 - #597c95
            4  524 - #597c95
            208  540 - #597c95
            568  540 - #0000e7
            540  544 - #0000e7
            592  548 - #0000e7
            556  556 - #0000e7
            576  556 - #0000e7
            540  560 - #0000e7
            316  568 - #597c95
            532  576 - #333338
            536  576 - #333338
            540  576 - #333338
            544  576 - #333338
            548  576 - #333338
            560  576 - #333338
            568  576 - #333338
            580  576 - #333338
            592  576 - #333338
            528  580 - #333338
            532  580 - #333338
            536  580 - #333338
            540  580 - #333338
            544  580 - #333338
            552  580 - #333338
            564  580 - #333338
            572  580 - #333338
            584  580 - #333338
            524  584 - #333338
            528  584 - #333338
            532  584 - #333338
            536  584 - #5d5d61
            540  584 - #333338
            548  584 - #333338
            560  584 - #333338
            580  584 - #333338
            592  584 - #333338
            420  588 - #597c95
            524  588 - #333338
            528  588 - #333338
            536  588 - #55555a
            552  588 - #ffffff
            568  588 - #333338
            576  588 - #ffffff
            588  588 - #b1b1b3
            108  592 - #597c95
            524  592 - #333338
            536  592 - #8d8d90
            564  592 - #333338
            592  592 - #333338
";

#[view]
struct AuthorCard {
    #[init]
    name: Label,
    mail: Label,
}

impl Setup for AuthorCard {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE).set_corner_radius(6);
        self.name.set_text("Vladas").set_text_size(14).place().t(4).lr(8).h(24);
        self.mail.set_text("v@example.com").set_text_size(12).place().t(30).lr(8).h(24);
    }
}

#[view]
struct TooltipHover {
    #[init]
    first:  Container,
    plain:  Container,
    rich:   Container,
    corner: Container,
}

impl Setup for TooltipHover {
    fn setup(self: Weak<Self>) {
        self.first.set_color(BLUE);
        self.first.place().tl(20).size(120, 60);
        self.first.set_tooltip("Full sha 2894b7b7");

        self.plain.set_color(GREEN);
        self.plain.place().t(20).l(160).size(120, 60);

        self.rich.set_color(ORANGE);
        self.rich.place().t(120).l(20).size(120, 60);
        self.rich.set_tooltip_view(|| {
            let card = AuthorCard::new();
            card.set_size(200, 60);
            card
        });

        // At the bottom right edge, so the tooltip has to slide back in.
        self.corner.set_color(BLUE);
        self.corner.place().br(0).size(60, 60);
        self.corner.set_tooltip("Edge case");
    }
}

impl ViewTest for TooltipHover {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        from_main(Hover::clear);

        assert_eq!(
            from_main(move || view.first.tooltip_text().to_string()),
            "Full sha 2894b7b7"
        );

        // Resting on the view shows the text after the delay, not before.
        inject_touches("60 50 m");
        from_main(|| assert!(Tooltip::shown().is_null()));
        wait_for_tooltip();
        from_main(|| {
            let shown = Tooltip::shown();
            assert!(shown.is_ok());
            assert_eq!(shown.frame().origin, (72, 68).into());
        });

        check_colors(TEXT_TOOLTIP)?;

        // Leaving the view hides it, a view without a tooltip shows none.
        inject_touches("220 50 m");
        from_main(|| assert!(Tooltip::shown().is_null()));
        wait_for_tooltip();
        from_main(|| assert!(Tooltip::shown().is_null()));

        // A press hides it too.
        inject_touches("60 50 m");
        wait_for_tooltip();
        from_main(|| assert!(Tooltip::shown().is_ok()));
        inject_touches("60 50 b\n60 50 e");
        from_main(|| assert!(Tooltip::shown().is_null()));

        // Moving away before the delay never shows it.
        inject_touches("60 50 m");
        inject_touches("220 50 m");
        wait_for_tooltip();
        from_main(|| assert!(Tooltip::shown().is_null()));

        // A view tooltip is built on every show and keeps its own size.
        inject_touches("60 150 m");
        wait_for_tooltip();
        from_main(|| {
            let shown = Tooltip::shown();
            assert!(shown.is_ok());
            assert_eq!(shown.size(), (200, 60).into());
        });

        check_colors(CARD_TOOLTIP)?;

        // The corner tooltip stays inside the screen.
        inject_touches("580 580 m");
        wait_for_tooltip();
        from_main(|| {
            let shown = Tooltip::shown();
            assert!(shown.is_ok());
            assert!(shown.max_x() <= 600.0);
            assert!(shown.max_y() <= 600.0);
        });

        check_colors(EDGE_TOOLTIP)?;

        inject_touches("400 400 m");
        from_main(|| assert!(Tooltip::shown().is_null()));

        Ok(())
    }
}
