use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Container, ContextMenu, Label, MenuItem, Setup, Theme, ThemeMode, ViewData, ViewFrame, ViewSubviews,
        ViewTest, ViewTouch, view,
    },
    ui_test::{check_colors, inject_right_click, inject_touches, set_record_probe_count},
};

const PUSH_MENU: &str = r"
             592    4 - #597c95
              44   40 - #d9e6ff
              72   40 - #d9e6ff
              96   40 - #d9e6ff
             140   40 - #d9e6ff
             164   40 - #d9e6ff
             196   40 - #d9e6ff
             236   40 - #d9e6ff
             364   40 - #ffe6d9
             468   40 - #ffe6d9
             544   40 - #ffe6d9
             180   44 - #d9e6ff
             216   44 - #d9e6ff
             416   44 - #ffe6d9
             116   52 - #d8e5fe
              40   60 - #d9e6ff
              72   60 - #d1d1d6
              88   60 - #d1d1d6
             104   60 - #d1d1d6
             136   60 - #d1d1d6
             152   60 - #d1d1d6
             172   60 - #d1d1d6
             184   60 - #d1d1d6
             196   60 - #d1d1d6
             236   60 - #d9e6ff
             504   60 - #ffe6d9
              56   64 - #c4cfe6
             216   64 - #ffffff
             220   68 - #a8b3c6
             388   68 - #ffe6d9
             472   68 - #ffe6d9
             124   72 - #ffffff
             220   72 - #a6b0c3
             440   72 - #ffe6d9
             116   76 - #ffffff
             136   76 - #bdbdbd
             200   76 - #ffffff
             220   76 - #a6b0c3
             556   76 - #ffe6d9
              40   80 - #d9e6ff
              60   80 - #d1d1d6
              80   80 - #3f3f3f
             136   80 - #bdbdbd
             220   80 - #a6b0c3
             160   84 - #ffffff
             220   84 - #a6b0c3
             188   88 - #ffffff
             220   88 - #a6b0c3
             496   88 - #ffe6d9
              56   92 - #bac5da
             220   92 - #a6b0c3
             360   92 - #ffe6d9
              96   96 - #ffffff
             220   96 - #a6b0c3
             412   96 - #ffe6d9
             468   96 - #ffe6d9
             528   96 - #ffe6d9
             116  100 - #ffdfdd
             116  104 - #ffffff
             200  104 - #ffffff
              76  108 - #ffffff
             128  108 - #ffa6a1
             144  108 - #ffffff
             164  108 - #ff3b30
              76  112 - #ff6c63
              80  112 - #ff6c63
             184  112 - #ffffff
             220  112 - #445f72
              60  116 - #d1d1d6
             100  116 - #ffffff
             200  124 - #ffffff
             156  128 - #ffffff
             180  128 - #ffffff
             220  128 - #445f72
              60  132 - #d1d1d6
              92  132 - #ffffff
             116  132 - #ffffff
             136  132 - #e2e2e3
             136  136 - #e2e2e3
             208  140 - #ffffff
             176  148 - #ffffff
              80  152 - #3c5465
             108  152 - #3c5465
             164  152 - #3c5465
             220  152 - #49657a
              64  156 - #49657a
              96  156 - #476377
             120  156 - #476377
             136  156 - #476377
             152  156 - #476377
             196  156 - #476377
             208  156 - #476377
             500  244 - #597c95
             264  264 - #597c95
               4  292 - #597c95
             372  320 - #597c95
             180  356 - #597c95
             592  364 - #597c95
             292  412 - #597c95
               4  440 - #597c95
             460  456 - #597c95
             160  480 - #597c95
             592  484 - #597c95
             280  536 - #000000
             292  536 - #000000
             308  536 - #000000
             320  536 - #000000
             276  540 - #000000
             284  540 - #000001
             292  540 - #597c95
             316  540 - #000000
             320  540 - #3f5869
             324  540 - #000000
             276  544 - #000001
             284  544 - #000001
             292  544 - #597c95
             304  544 - #3e5769
             316  544 - #000000
             100  592 - #597c95
             548  592 - #597c95
";

const SCOPE_MENU: &str = r"
               4    4 - #597c95
              52   40 - #d9e6ff
             136   40 - #d9e6ff
             236   40 - #d9e6ff
             388   40 - #ffe6d9
             420   40 - #ffe6d9
             448   40 - #ffe6d9
             472   40 - #ffe6d9
             504   40 - #ffe6d9
             540   40 - #ffe6d9
             360   44 - #ffe6d9
             404   44 - #ffe6d9
             188   48 - #d9e6ff
             376   52 - #ffe6d9
             488   52 - #fee5d8
              96   56 - #d9e6ff
             424   60 - #d1d1d6
             444   60 - #d1d1d6
             464   60 - #d1d1d6
             508   60 - #d1d1d6
             528   60 - #d1d1d6
             552   60 - #d1d1d6
             360   64 - #ffe6d9
             404   64 - #ffffff
             576   64 - #ffffff
              68   68 - #d9e6ff
             132   68 - #d9e6ff
             212   68 - #d9e6ff
             376   72 - #ffe6d9
             484   72 - #ffffff
             168   76 - #d9e6ff
             580   76 - #d1d1d6
              40   80 - #d9e6ff
             392   80 - #f3dbcf
             420   80 - #ffffff
             440   80 - #ffffff
             480   80 - #d1d1d1
             504   80 - #3c3c3c
             516   80 - #535353
             520   80 - #dfdfdf
             532   80 - #5e5e5e
             536   80 - #040404
             236   84 - #d9e6ff
             556   84 - #ffffff
             108   88 - #d9e6ff
             456   88 - #ffffff
             404   92 - #ffffff
             580   92 - #d1d1d6
              76   96 - #d9e6ff
             140   96 - #d9e6ff
             196   96 - #d9e6ff
             360   96 - #ffe6d9
             384   96 - #ffe6d9
             432   96 - #ffffff
             496   96 - #ffffff
             400  104 - #d1d1d6
             480  104 - #ffffff
             516  108 - #9b9b9b
             552  108 - #e0e0e0
             580  108 - #d1d1d6
             452  112 - #ffffff
             496  116 - #ffffff
             400  120 - #d1d1d6
             424  120 - #ffffff
             580  124 - #d1d1d6
             460  128 - #ffffff
             532  128 - #ffffff
             504  132 - #b2b2b2
             444  136 - #3f3f3f
             480  136 - #ffffff
             492  136 - #ffffff
             504  136 - #b2b2b2
             556  136 - #ffffff
             400  140 - #d1d1d6
             580  140 - #d1d1d6
             400  152 - #476276
             424  152 - #3c5465
             448  152 - #3c5465
             472  152 - #3c5465
             496  152 - #3c5465
             524  152 - #3c5465
             412  156 - #476377
             436  156 - #476377
             460  156 - #476377
             484  156 - #476377
             512  156 - #476377
             536  156 - #476377
             556  156 - #476377
             576  156 - #49657a
             288  200 - #597c95
             104  244 - #597c95
             564  300 - #597c95
             236  316 - #597c95
             424  352 - #597c95
               4  360 - #597c95
             316  412 - #597c95
             592  444 - #597c95
             140  448 - #597c95
             444  476 - #597c95
               4  480 - #597c95
             276  536 - #000000
             288  536 - #000000
             300  536 - #000001
             308  536 - #000000
             320  536 - #000000
             276  540 - #000001
             288  540 - #597c95
             292  540 - #000000
             296  540 - #06090b
             304  540 - #000001
             308  540 - #597c95
             312  540 - #597c95
             320  540 - #3f5869
             276  544 - #000001
             288  544 - #597c95
             296  544 - #06090b
             308  544 - #597c95
             164  588 - #597c95
              44  592 - #597c95
             492  592 - #597c95
";

const SCOPE_MENU_DARK: &str = r"
               4    4 - #597c95
              52   40 - #d9e6ff
             136   40 - #d9e6ff
             236   40 - #d9e6ff
             388   40 - #ffe6d9
             416   40 - #ffe6d9
             448   40 - #ffe6d9
             508   40 - #ffe6d9
             548   40 - #ffe6d9
             360   44 - #ffe6d9
             432   44 - #ffe6d9
             476   44 - #ffe6d9
             492   44 - #ffe6d9
             188   48 - #d9e6ff
             460   52 - #fee5d8
              96   56 - #d9e6ff
             376   56 - #ffe6d9
             528   56 - #eed7cb
             432   60 - #545459
             500   60 - #545459
             556   60 - #545459
             396   64 - #e6cfc4
             404   64 - #2c2c2e
              68   68 - #d9e6ff
             132   68 - #d9e6ff
             212   68 - #d9e6ff
             396   68 - #dcc6bb
             456   68 - #2c2c2e
             580   68 - #545459
             368   72 - #ffe6d9
             396   72 - #dac5ba
             484   72 - #2c2c2e
             168   76 - #d9e6ff
             396   76 - #dac5ba
              40   80 - #d9e6ff
             396   80 - #dac5ba
             420   80 - #2c2c2e
             480   80 - #525254
             504   80 - #cdcdce
             516   80 - #bababb
             520   80 - #464648
             532   80 - #b1b1b2
             536   80 - #fcfcfc
             556   80 - #2c2c2e
             236   84 - #d9e6ff
             396   84 - #dac5ba
             108   88 - #d9e6ff
             376   88 - #ffe6d9
             396   88 - #dac5ba
             440   88 - #2c2c2e
             460   88 - #2c2c2e
             396   92 - #dac5ba
             572   92 - #2c2c2e
              76   96 - #d9e6ff
             140   96 - #d9e6ff
             196   96 - #d9e6ff
             360   96 - #ffe6d9
             396   96 - #dac5ba
             416  100 - #2c2c2e
             480  104 - #2c2c2e
             456  108 - #2c2c2e
             508  108 - #2f2f31
             516  108 - #7f7f80
             552  108 - #464647
             400  112 - #545459
             432  112 - #2c2c2e
             580  120 - #545459
             412  124 - #2c2c2e
             536  124 - #2c2c2e
             444  132 - #cbcbcb
             504  132 - #6c6c6d
             400  136 - #545459
             444  136 - #cbcbcb
             480  136 - #2c2c2e
             492  136 - #2c2c2e
             504  136 - #6c6c6d
             516  136 - #2c2c2e
             560  136 - #2c2c2e
             420  140 - #2c2c2e
             540  148 - #2c2c2e
             524  152 - #3c5465
             580  152 - #476276
             404  156 - #49657a
             428  156 - #476377
             460  156 - #476377
             484  156 - #476377
             508  156 - #476377
             556  156 - #476377
             576  156 - #49657a
             284  200 - #597c95
             104  244 - #597c95
             560  300 - #597c95
             236  316 - #597c95
             424  352 - #597c95
               4  360 - #597c95
             316  412 - #597c95
             592  444 - #597c95
             140  448 - #597c95
             444  476 - #597c95
               4  480 - #597c95
             276  536 - #000000
             288  536 - #000000
             300  536 - #000001
             308  536 - #000000
             320  536 - #000000
             276  540 - #000001
             288  540 - #597c95
             292  540 - #000000
             296  540 - #06090b
             304  540 - #000001
             308  540 - #597c95
             312  540 - #597c95
             320  540 - #3f5869
             276  544 - #000001
             288  544 - #597c95
             296  544 - #06090b
             308  544 - #597c95
             164  588 - #597c95
              44  592 - #597c95
             492  592 - #597c95
";

#[view]
struct ContextMenuIcons {
    picked: Vec<&'static str>,

    #[init]
    push:   Container,
    scope:  Container,
    status: Label,
}

impl ContextMenuIcons {
    fn pick(self: Weak<Self>, name: &'static str) -> impl FnMut() + Send + 'static {
        move || {
            let mut this = self;
            this.picked.push(name);
            this.status.set_text(name);
        }
    }
}

impl Setup for ContextMenuIcons {
    fn setup(self: Weak<Self>) {
        self.push.set_color("#d9e6ff");
        self.push.place().t(40).l(40).size(200, 60);
        self.push.enable_touch();
        self.push.touch().secondary.sub(self, move || {
            ContextMenu::show_at_cursor(vec![
                MenuItem::new("Push", self.pick("push")).icon("arrow_up.svg"),
                MenuItem::new("Force push", self.pick("force"))
                    .icon("triangle_alert.svg")
                    .danger(),
                MenuItem::new("Push tags", self.pick("tags")).disabled(),
            ]);
        });

        self.scope.set_color("#ffe6d9");
        self.scope.place().t(40).r(40).size(200, 60);
        self.scope.enable_touch();
        self.scope.touch().secondary.sub(self, move || {
            ContextMenu::show_at_cursor(vec![
                MenuItem::new("All branches", self.pick("all")).checked(false),
                MenuItem::new("Current branch", self.pick("current")).checked(true),
                MenuItem::new("Stashes", self.pick("stashes"))
                    .checked(false)
                    .icon("arrow_up.svg"),
            ]);
        });

        self.status.set_text("none").set_text_size(24);
        self.status.place().b(40).lr(40).h(40);
    }
}

impl ViewTest for ContextMenuIcons {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(120);

        inject_right_click(60, 60);

        from_main(|| {
            let menu = ContextMenu::open();
            let items = menu.items();
            assert_eq!(items.len(), 3);

            assert!(items[0].icon().is_ok());
            assert!(items[1].icon().is_ok());
            assert!(items[2].icon().is_null());
            assert!(items.iter().all(|item| item.check().is_null()));

            // A row with no icon still starts its title after the icon
            // place, so all three titles line up.
            let title = items[0].subviews()[0].frame().origin;
            assert!(title.x > items[0].icon().max_x());
            for item in items {
                assert_eq!(item.subviews()[0].frame().origin, title);
            }
        });

        check_colors(PUSH_MENU)?;

        // Picking an item with an icon still runs its action.
        inject_touches("100 102 b\n100 102 e");
        from_main(move || {
            assert!(ContextMenu::open().is_null());
            assert_eq!(view.picked, vec!["force"]);
        });

        inject_right_click(400, 60);

        from_main(|| {
            let menu = ContextMenu::open();
            let items = menu.items();
            assert_eq!(items.len(), 3);

            // Only the checked choice draws a check.
            assert!(items[0].check().is_null());
            assert!(items[1].check().is_ok());
            assert!(items[2].check().is_null());

            // With a check place and an icon place, the icon sits after
            // the check place and every title after both.
            let check = items[1].check();
            let icon = items[2].icon();
            assert!(icon.x() > check.max_x());
            let title = items[0].subviews()[0].frame().origin;
            assert!(title.x > icon.max_x());
            for item in items {
                assert_eq!(item.subviews()[0].frame().origin, title);
            }
        });

        check_colors(SCOPE_MENU)?;

        // The check and the icon follow the theme like the titles do.
        from_main(|| Theme::set_mode(ThemeMode::Dark));
        wait_for_next_frame();
        check_colors(SCOPE_MENU_DARK)?;
        from_main(|| Theme::set_mode(ThemeMode::System));

        inject_touches("420 132 b\n420 132 e");
        from_main(move || {
            assert!(ContextMenu::open().is_null());
            assert_eq!(view.picked, vec!["force", "stashes"]);
        });

        Ok(())
    }
}
