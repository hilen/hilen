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
              64   40 - #d9e6ff
              84   40 - #d9e6ff
             128   40 - #d9e6ff
             152   40 - #d9e6ff
             172   40 - #d9e6ff
             204   40 - #d9e6ff
             236   40 - #d9e6ff
             364   40 - #ffe6d9
             448   40 - #ffe6d9
             540   40 - #ffe6d9
             108   44 - #d9e6ff
             188   48 - #d9e6ff
             220   48 - #d9e6ff
             492   52 - #ffe6d9
             404   56 - #ffe6d9
              72   60 - #d1d1d6
              84   60 - #d1d1d6
              96   60 - #d1d1d6
             112   60 - #d1d1d6
             128   60 - #d1d1d6
             140   60 - #d1d1d6
             156   60 - #d1d1d6
             172   60 - #d1d1d6
             204   60 - #d1d1d6
             236   60 - #d9e6ff
              40   64 - #d9e6ff
              56   64 - #c4d0e6
             184   64 - #ffffff
             220   68 - #a9b3c6
             440   68 - #ffe6d9
             524   68 - #ffe6d9
             124   72 - #ffffff
             220   72 - #a6b0c3
             116   76 - #ffffff
             136   76 - #c3c3c3
             220   76 - #a6b0c3
             360   76 - #ffe6d9
              56   80 - #bac5da
              76   80 - #ffffff
              96   80 - #ffffff
             136   80 - #c3c3c3
             220   80 - #a6b0c3
             168   84 - #ffffff
             220   84 - #a6b0c3
             480   84 - #ffe6d9
             556   84 - #ffe6d9
             220   88 - #a6b0c3
             420   88 - #ffe6d9
             152   92 - #ffffff
             192   92 - #ffffff
             220   92 - #a6b0c3
              40   96 - #d9e6ff
              60   96 - #d1d1d6
              84   96 - #ffffff
             220   96 - #a6b0c3
             388   96 - #ffe6d9
             452   96 - #ffe6d9
             512   96 - #ffe6d9
             116  100 - #ffe1df
             116  104 - #ffffff
             128  108 - #ffaaa5
             144  108 - #ffffff
             164  108 - #ff3b30
             184  108 - #ffffff
             100  112 - #ffffff
             220  112 - #445f72
              60  116 - #d1d1d6
             196  124 - #ffffff
              88  128 - #ffffff
             156  128 - #ffffff
             220  128 - #445f72
              60  132 - #d1d1d6
             116  132 - #ffffff
             136  132 - #e4e4e6
             176  132 - #ffffff
             136  136 - #e4e4e6
             220  140 - #445f72
              64  148 - #ffffff
              80  152 - #3c5364
             108  152 - #3c5364
             164  152 - #3c5364
             188  152 - #3c5364
             220  152 - #49657a
              96  156 - #476377
             120  156 - #476377
             136  156 - #476377
             152  156 - #476377
             176  156 - #476377
             200  156 - #476377
             332  204 - #597c95
             496  244 - #597c95
               4  284 - #597c95
             368  320 - #597c95
             176  348 - #597c95
             592  356 - #597c95
             284  412 - #597c95
               4  436 - #597c95
             464  456 - #597c95
             160  476 - #597c95
             588  476 - #597c95
             280  536 - #000000
             292  536 - #000001
             308  536 - #000000
             320  536 - #000000
             276  540 - #000000
             284  540 - #000000
             292  540 - #597c95
             316  540 - #000000
             320  540 - #3e5769
             324  540 - #000001
             276  544 - #000000
             284  544 - #000000
             292  544 - #597c95
             304  544 - #3d5566
             316  544 - #000001
             436  584 - #597c95
             100  592 - #597c95
             556  592 - #597c95
";

const SCOPE_MENU: &str = r"
               4    4 - #597c95
              60   40 - #d9e6ff
             132   40 - #d9e6ff
             216   40 - #d9e6ff
             364   40 - #ffe6d9
             392   40 - #ffe6d9
             412   40 - #ffe6d9
             432   40 - #ffe6d9
             452   40 - #ffe6d9
             480   40 - #ffe6d9
             512   40 - #ffe6d9
             556   40 - #ffe6d9
             176   52 - #d9e6ff
             492   52 - #fee5d8
             532   52 - #fee5d8
              96   56 - #d9e6ff
             360   56 - #ffe6d9
             408   60 - #d1d1d6
             428   60 - #d1d1d6
             456   60 - #d1d1d6
             472   60 - #d1d1d6
             508   60 - #d1d1d6
             568   60 - #d1d1d6
              40   64 - #d9e6ff
             236   64 - #d9e6ff
             552   64 - #ffffff
             156   68 - #d9e6ff
             376   68 - #ffe6d9
             396   72 - #dac5ba
             484   72 - #ffffff
             580   72 - #d1d1d6
              68   76 - #d9e6ff
             204   76 - #d9e6ff
             360   76 - #ffe6d9
             132   80 - #d9e6ff
             460   80 - #ffffff
             480   80 - #cecece
             504   80 - #444444
             516   80 - #5d5d5d
             520   80 - #e8e8e8
             532   80 - #5f5f5f
             536   80 - #000000
             440   84 - #ffffff
             400   88 - #d1d1d6
             380   92 - #ffe6d9
             420   92 - #ffffff
             556   92 - #ffffff
              40   96 - #d9e6ff
              96   96 - #d9e6ff
             168   96 - #d9e6ff
             236   96 - #d9e6ff
             360   96 - #ffe6d9
             456  100 - #ffffff
             580  100 - #d1d1d6
             436  104 - #ffffff
             480  104 - #ffffff
             400  108 - #d1d1d6
             508  108 - #fcfcfc
             516  108 - #9e9e9e
             536  108 - #ffffff
             420  116 - #ffffff
             472  120 - #ffffff
             552  120 - #ffffff
             400  124 - #d1d1d6
             452  124 - #ffffff
             580  128 - #d1d1d6
             504  132 - #aaaaaa
             536  132 - #ffffff
             400  136 - #d1d1d6
             428  136 - #ffffff
             480  136 - #ffffff
             492  136 - #ffffff
             504  136 - #aaaaaa
             516  136 - #ffffff
             404  148 - #ffffff
             428  152 - #3c5364
             452  152 - #3c5364
             476  152 - #3c5364
             500  152 - #3c5364
             528  152 - #3c5364
             548  152 - #3c5364
             580  152 - #466276
             408  156 - #476377
             420  156 - #476377
             440  156 - #476377
             464  156 - #476377
             488  156 - #476377
             516  156 - #476377
             540  156 - #476377
             560  156 - #476377
             296  204 - #597c95
             112  244 - #597c95
             592  296 - #597c95
             240  312 - #597c95
               4  344 - #597c95
             428  344 - #597c95
             320  408 - #597c95
             592  440 - #597c95
             128  456 - #597c95
               4  468 - #597c95
             436  476 - #597c95
             276  536 - #000000
             288  536 - #000001
             300  536 - #000000
             308  536 - #000001
             320  536 - #000000
             276  540 - #000000
             288  540 - #597c95
             296  540 - #050708
             304  540 - #000000
             308  540 - #597c95
             312  540 - #597c95
             320  540 - #3e5769
             276  544 - #000000
             288  544 - #597c95
             296  544 - #050708
             308  544 - #597c95
              28  592 - #597c95
             156  592 - #597c95
             496  592 - #597c95
";

const SCOPE_MENU_DARK: &str = r"
              76   40 - #d9e6ff
             144   40 - #d9e6ff
             204   40 - #d9e6ff
             364   40 - #ffe6d9
             404   40 - #ffe6d9
             432   40 - #ffe6d9
             468   40 - #ffe6d9
             512   40 - #ffe6d9
             556   40 - #ffe6d9
             176   44 - #d9e6ff
             384   44 - #ffe6d9
             536   44 - #ffe6d9
             448   52 - #fee5d8
             492   52 - #fee5d8
              40   56 - #d9e6ff
             112   56 - #d9e6ff
             360   56 - #ffe6d9
             508   56 - #eed7cb
             236   60 - #d9e6ff
             380   60 - #ffe6d9
             420   60 - #545459
             468   60 - #545459
             524   60 - #545459
             548   60 - #545459
             564   60 - #545459
             396   64 - #e6d0c4
              68   68 - #d9e6ff
             160   68 - #d9e6ff
             208   68 - #d9e6ff
             368   68 - #ffe6d9
             396   68 - #dcc7bb
             580   68 - #545459
             132   72 - #d9e6ff
             396   72 - #dac5ba
             444   72 - #2c2c2e
             484   72 - #2c2c2e
             396   76 - #dac5ba
             560   76 - #2c2c2e
             396   80 - #dac5ba
             480   80 - #555556
             504   80 - #c7c7c7
             516   80 - #b2b2b3
             532   80 - #b0b0b1
             536   80 - #ffffff
             376   84 - #ffe6d9
             396   84 - #dac5ba
             580   84 - #545459
              92   88 - #d9e6ff
             188   88 - #d9e6ff
             396   88 - #dac5ba
             420   88 - #2c2c2e
             360   92 - #ffe6d9
             396   92 - #dac5ba
              44   96 - #d9e6ff
             140   96 - #d9e6ff
             236   96 - #d9e6ff
             396   96 - #dac5ba
             456   96 - #2c2c2e
             556   96 - #2c2c2e
             580  100 - #545459
             480  104 - #2c2c2e
             436  108 - #2c2c2e
             508  108 - #2e2e30
             516  108 - #7c7c7e
             536  112 - #2c2c2e
             492  116 - #2c2c2e
             408  120 - #2c2c2e
             460  120 - #2c2c2e
             552  124 - #2c2c2e
             580  128 - #545459
             432  132 - #2c2c2e
             504  132 - #727274
             480  136 - #2c2c2e
             492  136 - #2c2c2e
             504  136 - #727274
             516  136 - #2c2c2e
             536  136 - #2c2c2e
               4  148 - #597c95
             404  148 - #2c2c2e
             456  148 - #2c2c2e
             556  148 - #2c2c2e
             520  152 - #3c5364
             580  152 - #466276
             404  156 - #49657a
             432  156 - #476377
             476  156 - #476377
             500  156 - #476377
             536  156 - #476377
             576  156 - #49657a
             292  208 - #597c95
             156  216 - #597c95
              68  296 - #597c95
             556  300 - #597c95
             228  312 - #597c95
             420  352 - #597c95
             308  408 - #597c95
               4  440 - #597c95
             156  444 - #597c95
             592  444 - #597c95
             440  476 - #597c95
             276  536 - #000000
             288  536 - #000001
             300  536 - #000000
             308  536 - #000001
             320  536 - #000000
             276  540 - #000000
             288  540 - #597c95
             296  540 - #050708
             304  540 - #000000
             308  540 - #597c95
             312  540 - #597c95
             320  540 - #3e5769
             276  544 - #000000
             288  544 - #597c95
             296  544 - #050708
             308  544 - #597c95
               4  548 - #597c95
             592  552 - #597c95
             100  592 - #597c95
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
