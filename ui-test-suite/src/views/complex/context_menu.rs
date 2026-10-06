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
               4    4 - #597c95
             100   40 - #d9e6ff
             164   40 - #d9e6ff
             224   40 - #d9e6ff
             280   40 - #d9e6ff
             328   40 - #d9e6ff
             368   40 - #d9e6ff
             408   40 - #d9e6ff
             496   40 - #d9e6ff
             556   40 - #d9e6ff
              44   44 - #d9e6ff
             252   44 - #d9e6ff
             528   48 - #d9e6ff
             452   56 - #d9e6ff
             132   60 - #d9e6ff
              72   64 - #d9e6ff
             424   64 - #d9e6ff
             480   64 - #d9e6ff
             204   68 - #d9e6ff
             308   68 - #d9e6ff
             548   68 - #d9e6ff
             256   72 - #d9e6ff
             512   72 - #d9e6ff
             384   76 - #d9e6ff
              40   80 - #d9e6ff
             100   80 - #d9e6ff
             176   84 - #d9e6ff
             344   84 - #d9e6ff
             448   88 - #d9e6ff
              68   96 - #d9e6ff
             132   96 - #d9e6ff
             220   96 - #d9e6ff
             288   96 - #d9e6ff
             316   96 - #d9e6ff
             420   96 - #d9e6ff
             480   96 - #d9e6ff
             540   96 - #d9e6ff
             296  152 - #000000
             272  156 - #000000
             288  156 - #000000
             296  156 - #000000
             316  156 - #000000
             272  160 - #597c95
             276  160 - #54758d
             288  160 - #3e5769
             296  160 - #000000
             304  160 - #3e5769
             316  160 - #1c262e
             272  164 - #597c95
             288  164 - #597c95
             296  164 - #000000
             308  164 - #597c95
             388  164 - #597c95
             592  164 - #597c95
               4  168 - #597c95
             180  188 - #597c95
             496  188 - #597c95
              88  212 - #597c95
             412  232 - #597c95
             592  240 - #597c95
             348  268 - #597c95
             276  272 - #597c95
             524  276 - #597c95
             172  292 - #597c95
              12  300 - #597c95
              92  312 - #597c95
             428  312 - #597c95
             352  344 - #597c95
             532  344 - #597c95
             256  368 - #597c95
               4  380 - #597c95
             468  380 - #597c95
             592  380 - #597c95
             128  384 - #597c95
             316  404 - #597c95
             392  420 - #597c95
             528  424 - #597c95
              52  448 - #597c95
             204  452 - #597c95
             324  472 - #597c95
             128  492 - #597c95
             460  500 - #ffffff
             488  500 - #ffffff
             508  500 - #ffffff
             536  500 - #ffffff
             556  500 - #ffffff
             576  500 - #ffffff
             592  504 - #ffffff
             440  508 - #d1d1d6
             440  512 - #d1d1d6
             456  512 - #ffffff
             440  516 - #d1d1d6
             472  516 - #ffffff
             476  516 - #ffffff
             500  516 - #909090
             516  516 - #1a1a1a
             520  516 - #313131
             524  516 - #000000
             564  516 - #ffffff
             440  520 - #d1d1d6
             544  520 - #ffffff
             592  520 - #ffffff
             440  524 - #d1d1d6
              56  528 - #597c95
             440  528 - #d1d1d6
             460  528 - #ffffff
             580  528 - #ffffff
             440  532 - #d1d1d6
             512  532 - #ffffff
             216  536 - #597c95
             440  536 - #d1d1d6
             492  536 - #ffffff
             528  536 - #ffffff
             556  536 - #ffffff
             440  540 - #d1d1d6
             472  540 - #939398
             592  540 - #ffffff
             440  544 - #d1d1d6
             544  544 - #ffffff
             572  544 - #ffffff
             440  548 - #d1d1d6
             508  548 - #ffffff
             356  552 - #597c95
             440  552 - #d1d1d6
             440  556 - #d1d1d6
             460  556 - #ffffff
             536  556 - #ffffff
             592  556 - #ffffff
             440  560 - #d1d1d6
             484  560 - #ffffff
             564  560 - #ffffff
             440  564 - #d1d1d6
             500  564 - #ffffff
             520  564 - #ffffff
             440  568 - #d1d1d6
             548  568 - #ffffff
             440  572 - #d1d1d6
             456  572 - #ffffff
             592  572 - #ffffff
             144  576 - #597c95
             440  576 - #d1d1d6
             576  576 - #ffffff
             440  580 - #d1d1d6
             472  580 - #ffffff
             512  580 - #ff3b30
             440  584 - #d1d1d6
             472  584 - #ffffff
             492  584 - #ffffff
             504  584 - #ffffff
             512  584 - #ff3b30
             520  584 - #ff3b30
             532  584 - #ff3b30
             536  584 - #ff8c85
             552  584 - #ff7a73
             440  588 - #d1d1d6
               4  592 - #597c95
             280  592 - #597c95
             456  592 - #ffffff
             568  592 - #ffffff
             588  592 - #ffffff
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
            assert_eq!(view.picked.len(), 0);
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
