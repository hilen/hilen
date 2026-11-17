use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Container, ContextMenu, Label, MenuItem, Point, Setup, TouchStack, ViewData, ViewFrame, ViewTest,
        ViewTouch, view,
    },
    ui_test::{check_colors, inject_right_click, inject_touches},
};

const SECOND_MENU: &str = r"
               4    4 - #597c95
             144   40 - #d9e6ff
             468   40 - #ffe6d9
             320   44 - #ffe6d9
             424   68 - #e1cbc0
             396   76 - #e0cabf
             556   76 - #ffffff
             504   84 - #676767
             448   88 - #ffffff
             484   88 - #ffffff
             532  104 - #ffffff
             444  112 - #ffbeba
              76  116 - #d9e6ff
             208  116 - #d9e6ff
             328  116 - #ffe6d9
             444  116 - #ffbeba
             556  128 - #ffffff
             400  132 - #445e71
             464  136 - #415b6d
             496  136 - #415b6d
             528  136 - #415b6d
             336  412 - #000000
             268  416 - #000000
             304  416 - #000000
             320  416 - #000000
             280  420 - #3e5769
             280  424 - #597c95
             292  424 - #597c95
             332  424 - #597c95
             336  424 - #000000
               4  592 - #597c95
             592  592 - #597c95
";

/// A right click outside an open menu closes it and reaches the view
/// under the cursor, so that view opens its own menu with the same click.
#[view]
struct ContextMenuReopen {
    taps: usize,

    #[init]
    first:  Container,
    second: Container,
    status: Label,
}

impl Setup for ContextMenuReopen {
    fn setup(mut self: Weak<Self>) {
        self.first.set_color("#d9e6ff");
        self.first.place().t(40).l(40).size(200, 80);
        self.first.enable_touch();
        self.first.touch().secondary.sub(self, move || {
            self.status.set_text("first");
            ContextMenu::show_at_cursor(vec![MenuItem::new("First card", || {})]);
        });

        self.second.set_color("#ffe6d9");
        self.second.place().t(40).l(320).size(200, 80);
        self.second.enable_touch();
        self.second.touch().secondary.sub(self, move || {
            self.status.set_text("second");
            ContextMenu::show_at_cursor(vec![
                MenuItem::new("Second card", || {}),
                MenuItem::new("Kill", || {}).danger(),
            ]);
        });
        // A left tap outside a menu still only closes it.
        self.second.touch().up_inside.sub(self, move || self.taps += 1);

        self.status.set_text("none").set_text_size(24);
        self.status.place().t(400).lr(40).h(40);
    }
}

impl ViewTest for ContextMenuReopen {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_right_click(100, 70);
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(menu.items()[0].title(), "First card");
        });

        // The second view gets the click while the first menu is open.
        inject_right_click(400, 70);
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(menu.items().len(), 2);
            assert_eq!(menu.items()[0].title(), "Second card");
            assert_eq!(menu.frame().origin, Point::new(400.0, 70.0));
            assert_eq!(view.status.text(), "second");
            // 1 menu, so 1 layer over the root.
            assert_eq!(TouchStack::root_name(), "Context menu backdrop");
        });
        check_colors(SECOND_MENU)?;

        // And back, any number of times.
        inject_right_click(100, 70);
        from_main(move || {
            assert_eq!(ContextMenu::open().items()[0].title(), "First card");
            assert_eq!(view.status.text(), "first");
        });

        // A right click on nothing only closes the menu.
        inject_right_click(300, 300);
        from_main(move || {
            assert!(ContextMenu::open().is_null());
            assert_eq!(TouchStack::root_name(), "Root view");
            assert_eq!(view.status.text(), "first");
        });

        // A left tap outside closes the menu and does not reach the view.
        inject_right_click(100, 70);
        inject_touches("400 70 b\n400 70 e");
        from_main(move || {
            assert!(ContextMenu::open().is_null());
            assert_eq!(view.taps, 0);
        });

        Ok(())
    }
}
