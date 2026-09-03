use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Container, ContextMenu, Label, MenuItem, NamedKey, Point, Setup, TouchStack, ViewData, ViewFrame,
        ViewTest, ViewTouch, view,
    },
    ui_test::{
        capture_screenshot, check_colors, inject_long_press, inject_named_key, inject_right_click,
        inject_touches, set_record_probe_count,
    },
};

const OPEN_MENU: &str = r"
            44   40 - #d9e6ff
            84   40 - #d9e6ff
            252   40 - #d9e6ff
            284   40 - #d9e6ff
            328   40 - #d9e6ff
            408   40 - #d9e6ff
            496   40 - #d9e6ff
            556   40 - #d9e6ff
            368   52 - #d9e6ff
            452   56 - #d9e6ff
            64   68 - #d9e6ff
            108   68 - #c0cbe1
            120   68 - #c0cbe1
            132   68 - #c0cbe1
            144   68 - #c0cbe1
            156   68 - #c0cbe1
            168   68 - #c0cbe1
            188   68 - #c0cbe1
            200   68 - #c0cbe1
            212   68 - #c0cbe1
            224   68 - #c0cbe1
            236   68 - #c0cbe1
            248   68 - #c0cbe1
            256   68 - #c4d0e7
            292   72 - #d9e6ff
            512   72 - #d9e6ff
            96   76 - #bfcae0
            256   76 - #ffffff
            344   76 - #d9e6ff
            396   76 - #d9e6ff
            260   80 - #a6b0c3
            112   84 - #ffffff
            160   84 - #8a8a8a
            260   84 - #a6b0c3
            100   88 - #d1d1d6
            132   88 - #ffffff
            136   88 - #ffffff
            148   88 - #696969
            156   88 - #ffffff
            160   88 - #8a8a8a
            176   88 - #252525
            184   88 - #0f0f0f
            260   88 - #a6b0c3
            208   92 - #ffffff
            232   92 - #ffffff
            260   92 - #a6b0c3
            316   92 - #d9e6ff
            40   96 - #d9e6ff
            260   96 - #a6b0c3
            368   96 - #d9e6ff
            424   96 - #d9e6ff
            480   96 - #d9e6ff
            540   96 - #d9e6ff
            100  100 - #d1d1d6
            120  108 - #ffffff
            168  108 - #ffffff
            200  108 - #ffffff
            260  108 - #445f72
            100  112 - #d1d1d6
            140  116 - #bdbdbf
            220  116 - #ffffff
            244  116 - #ffffff
            188  120 - #ffffff
            260  120 - #445f72
            100  124 - #d1d1d6
            164  128 - #ffffff
            260  128 - #445f72
            124  132 - #ffffff
            148  132 - #ffffff
            100  136 - #d1d1d6
            184  136 - #ffffff
            208  136 - #ffffff
            240  136 - #ffffff
            260  136 - #445f72
            100  148 - #d1d1d6
            152  148 - #ffffff
            224  148 - #ffffff
            260  148 - #445f72
            132  152 - #ffffff
            172  152 - #ff3b30
            180  152 - #ff3b30
            192  152 - #ff3b30
            116  156 - #ffffff
            240  156 - #ffffff
            260  156 - #445f72
            280  156 - #000000
            292  156 - #000000
            308  156 - #000000
            320  156 - #000000
            100  160 - #d1d1d6
            212  160 - #ffffff
            276  160 - #000000
            284  160 - #000001
            292  160 - #597c95
            316  160 - #000000
            320  160 - #3f5869
            324  160 - #000000
            260  164 - #445f72
            276  164 - #000001
            284  164 - #000001
            292  164 - #597c95
            304  164 - #3e5769
            316  164 - #000000
            100  168 - #425d6f
            112  172 - #3f5769
            140  172 - #3f5769
            168  172 - #3f5769
            196  172 - #3f5769
            224  172 - #3f5769
            256  172 - #425d6f
            124  176 - #4a677b
            152  176 - #4a677b
            176  176 - #4a677b
            184  176 - #4a677b
            208  176 - #4a677b
            216  176 - #4a677b
            236  176 - #4a677b
            248  176 - #4a677b
            592  192 - #597c95
            440  220 - #597c95
            4  244 - #597c95
            96  276 - #597c95
            268  284 - #597c95
            544  292 - #597c95
            372  328 - #597c95
            168  340 - #597c95
            4  368 - #597c95
            472  368 - #597c95
            288  376 - #597c95
            592  412 - #597c95
            424  460 - #597c95
            104  468 - #597c95
            272  468 - #597c95
            4  488 - #597c95
            552  540 - #ffe6d9
            564  540 - #ffe6d9
            576  540 - #ffe6d9
            592  540 - #ffe6d9
            540  544 - #ffe6d9
            552  552 - #ffe6d9
            564  552 - #ffe6d9
            580  552 - #ffe6d9
            592  552 - #ffe6d9
            540  556 - #ffe6d9
            568  564 - #ffe6d9
            580  564 - #ffe6d9
            544  568 - #ffe6d9
            592  568 - #ffe6d9
            556  572 - #ffe6d9
            272  576 - #597c95
            540  580 - #ffe6d9
            576  580 - #ffe6d9
            560  588 - #ffe6d9
            4  592 - #597c95
            168  592 - #597c95
            376  592 - #597c95
            536  592 - #597c95
            548  592 - #ffe6d9
            572  592 - #ffe6d9
            588  592 - #ffe6d9
";

const CORNER_MENU: &str = r"
            44   40 - #d9e6ff
            84   40 - #d9e6ff
            268   40 - #d9e6ff
            304   40 - #d9e6ff
            356   40 - #d9e6ff
            404   40 - #d9e6ff
            464   40 - #d9e6ff
            556   40 - #d9e6ff
            508   56 - #d9e6ff
            64   68 - #d9e6ff
            108   68 - #c0cbe1
            120   68 - #c0cbe1
            132   68 - #c0cbe1
            144   68 - #c0cbe1
            156   68 - #c0cbe1
            164   68 - #c0cbe1
            172   68 - #c0cbe1
            180   68 - #c0cbe1
            192   68 - #c0cbe1
            204   68 - #c0cbe1
            216   68 - #c0cbe1
            228   68 - #c0cbe1
            240   68 - #c0cbe1
            288   68 - #d9e6ff
            412   68 - #d9e6ff
            252   72 - #ffffff
            448   72 - #d9e6ff
            96   76 - #bfcae0
            384   76 - #d9e6ff
            260   80 - #a6b0c3
            112   84 - #ffffff
            160   84 - #8a8a8a
            260   84 - #a6b0c3
            340   84 - #d9e6ff
            100   88 - #d1d1d6
            132   88 - #ffffff
            136   88 - #ffffff
            148   88 - #696969
            156   88 - #ffffff
            160   88 - #8a8a8a
            176   88 - #252525
            184   88 - #0f0f0f
            240   88 - #ffffff
            260   88 - #a6b0c3
            508   88 - #d9e6ff
            260   92 - #a6b0c3
            40   96 - #d9e6ff
            204   96 - #ffffff
            260   96 - #a6b0c3
            300   96 - #d9e6ff
            420   96 - #d9e6ff
            480   96 - #d9e6ff
            540   96 - #d9e6ff
            100  100 - #d1d1d6
            224  100 - #ffffff
            120  108 - #ffffff
            160  108 - #ffffff
            180  108 - #ffffff
            100  112 - #d1d1d6
            260  112 - #445f72
            140  116 - #bdbdbf
            240  116 - #ffffff
            196  120 - #ffffff
            100  124 - #d1d1d6
            168  124 - #ffffff
            260  124 - #445f72
            216  128 - #ffffff
            124  132 - #ffffff
            184  132 - #ffffff
            100  136 - #d1d1d6
            236  136 - #ffffff
            260  136 - #445f72
            152  140 - #ffffff
            204  140 - #ffffff
            260  144 - #445f72
            100  148 - #d1d1d6
            132  152 - #ffffff
            172  152 - #ff3b30
            180  152 - #ff3b30
            192  152 - #ff3b30
            216  152 - #ffffff
            296  152 - #000000
            232  156 - #ffffff
            260  156 - #445f72
            272  156 - #000000
            288  156 - #000000
            296  156 - #000000
            316  156 - #000000
            100  160 - #d1d1d6
            152  160 - #ffffff
            248  160 - #ffffff
            272  160 - #597c95
            288  160 - #3f5869
            296  160 - #000000
            304  160 - #3f5869
            316  160 - #19232b
            260  164 - #445f72
            272  164 - #597c95
            288  164 - #597c95
            296  164 - #000000
            308  164 - #597c95
            100  168 - #425d6f
            112  172 - #3f5769
            136  172 - #3f5769
            164  172 - #3f5769
            188  172 - #3f5769
            216  172 - #3f5769
            240  172 - #3f5769
            260  172 - #4a677c
            124  176 - #4a677b
            148  176 - #4a677b
            156  176 - #4a677b
            176  176 - #4a677b
            200  176 - #4a677b
            208  176 - #4a677b
            228  176 - #4a677b
            252  176 - #4a677b
            592  196 - #597c95
            432  220 - #597c95
            4  244 - #597c95
            96  276 - #597c95
            264  284 - #597c95
            532  292 - #597c95
            364  324 - #597c95
            168  340 - #597c95
            460  360 - #597c95
            4  368 - #597c95
            284  376 - #597c95
            592  408 - #597c95
            188  432 - #597c95
            424  456 - #597c95
            104  468 - #597c95
            272  468 - #597c95
            4  488 - #597c95
            552  540 - #ffe6d9
            564  540 - #ffe6d9
            576  540 - #ffe6d9
            592  540 - #ffe6d9
            540  544 - #ffe6d9
            552  552 - #ffe6d9
            564  552 - #ffe6d9
            580  552 - #ffe6d9
            592  552 - #ffe6d9
            540  556 - #ffe6d9
            568  564 - #ffe6d9
            580  564 - #ffe6d9
            544  568 - #ffe6d9
            592  568 - #ffe6d9
            556  572 - #ffe6d9
            272  576 - #597c95
            540  580 - #ffe6d9
            576  580 - #ffe6d9
            560  588 - #ffe6d9
            4  592 - #597c95
            168  592 - #597c95
            376  592 - #597c95
            536  592 - #597c95
            548  592 - #ffe6d9
            572  592 - #ffe6d9
            588  592 - #ffe6d9
";

#[view]
struct ContextMenuTest {
    picked: Vec<&'static str>,

    #[init]
    row:    Container,
    corner: Container,
    status: Label,
}

impl ContextMenuTest {
    fn items(self: Weak<Self>) -> Vec<MenuItem> {
        let pick = move |name: &'static str| {
            move || {
                let mut this = self;
                this.picked.push(name);
                this.status.set_text(name);
            }
        };

        vec![
            MenuItem::new("Checkout", pick("checkout")),
            MenuItem::new("Rename", pick("rename")).disabled(),
            MenuItem::separator(),
            MenuItem::new("Delete branch", pick("delete")).danger(),
        ]
    }
}

impl Setup for ContextMenuTest {
    fn setup(self: Weak<Self>) {
        self.row.set_color("#d9e6ff");
        self.row.place().t(40).lr(40).h(60);
        self.row.enable_touch();
        self.row.touch().secondary.sub(self, move || {
            ContextMenu::show_at_cursor(self.items());
        });

        // Near the bottom right edge, so the menu has to slide back in.
        self.corner.set_color("#ffe6d9");
        self.corner.place().br(0).size(60, 60);
        self.corner.enable_touch();
        self.corner.touch().secondary.sub(self, move || {
            ContextMenu::show_at_cursor(self.items());
        });

        self.status.set_text("none").set_text_size(24);
        self.status.place().t(140).lr(40).h(40);
    }
}

impl ViewTest for ContextMenuTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);

        inject_right_click(100, 70);

        from_main(|| {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(menu.items().len(), 3);
            assert_eq!(menu.items()[0].title(), "Checkout");
            assert!(!menu.items()[1].is_enabled());
            assert_eq!(menu.items()[2].title(), "Delete branch");
            assert_eq!(menu.frame().origin, Point::new(100.0, 70.0));
        });

        capture_screenshot()?;
        check_colors(OPEN_MENU)?;

        // A disabled item does nothing and keeps the menu open.
        inject_touches("150 116 b\n150 116 e");
        from_main(move || {
            assert!(ContextMenu::open().is_ok());
            assert!(view.picked.is_empty());
        });

        // A tap outside closes it and does not reach the view below.
        inject_touches("300 300 b\n300 300 e");
        from_main(move || {
            assert!(ContextMenu::open().is_null());
            assert_eq!(TouchStack::root_name(), "Root view");
        });

        // Escape closes it too.
        inject_right_click(100, 70);
        from_main(|| assert!(ContextMenu::open().is_ok()));
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        // Picking an item closes the menu and runs the action.
        inject_right_click(100, 70);
        inject_touches("150 88 b\n150 88 e");
        from_main(move || {
            assert!(ContextMenu::open().is_null());
            assert_eq!(view.picked, vec!["checkout"]);
            assert_eq!(view.status.text(), "checkout");
        });

        // The danger item, after a separator.
        inject_right_click(100, 70);
        inject_touches("150 153 b\n150 153 e");
        from_main(move || {
            assert_eq!(view.picked, vec!["checkout", "delete"]);
        });

        // A long press opens the same menu, the touch screen way.
        inject_long_press(100, 70);
        from_main(|| assert!(ContextMenu::open().is_ok()));

        // Opening another menu closes the first.
        inject_right_click(570, 570);
        from_main(|| {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(TouchStack::root_name(), "Context menu backdrop");
            assert!(menu.max_x() <= 600.0);
            assert!(menu.max_y() <= 600.0);
        });

        check_colors(CORNER_MENU)?;

        inject_named_key(NamedKey::Escape);
        from_main(|| assert_eq!(TouchStack::root_name(), "Root view"));

        Ok(())
    }
}
