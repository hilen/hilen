use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        Button, ContextMenu, MenuAlign, MenuItem, NamedKey, Point, Setup, ViewData, ViewFrame, ViewTest, view,
    },
    ui_test::{check_colors, inject_named_key, inject_touches},
};

const RIGHT_MENU: &str = r"
             444   40 - #ffffff
              68   44 - #303030
             120   44 - #909090
              96   52 - #c0c0c0
             468   52 - #ffffff
             512   52 - #000000
              68   64 - #000000
             120   64 - #909090
             532   64 - #000000
             472   76 - #d1d1d6
             556   84 - #ffffff
             388   88 - #d1d1d6
             428   88 - #ffffff
             440   96 - #bdbdbd
             440  120 - #bdbdbd
             480  124 - #616161
             484  124 - #e9e9e9
             496  124 - #363636
             520  124 - #515151
             420  144 - #ffdfdd
             420  148 - #ffffff
             432  152 - #ffa6a1
             468  152 - #ff3b30
             508  172 - #476377
             556  172 - #4a677b
             248  400 - #597c95
               4  580 - #597c95
             496  580 - #010101
             544  584 - #6c6c6c
             536  588 - #888888
             580  588 - #000000
             536  592 - #888888
";

#[view]
struct ContextMenuBelow {
    #[init]
    left:   Button,
    right:  Button,
    corner: Button,
}

fn items() -> Vec<MenuItem> {
    vec![
        MenuItem::new("Push", || {}),
        MenuItem::new("Push to other remote", || {}),
        MenuItem::new("Force push", || {}).danger(),
    ]
}

impl Setup for ContextMenuBelow {
    fn setup(self: Weak<Self>) {
        self.left.set_text("Push");
        self.left.place().t(40).l(40).size(120, 32);
        self.left.on_tap(move || {
            ContextMenu::show_below(items(), self.left, MenuAlign::Left);
        });

        // A button at the right edge of a toolbar. Its menu grows to the
        // left and never covers what sits left of the button's own edge.
        self.right.set_text("Scope");
        self.right.place().t(40).r(40).size(120, 32);
        self.right.on_tap(move || {
            ContextMenu::show_below(items(), self.right, MenuAlign::Right);
        });

        // No room below, so the menu slides back up inside the screen.
        self.corner.set_text("Corner");
        self.corner.place().br(0).size(120, 32);
        self.corner.on_tap(move || {
            ContextMenu::show_below(items(), self.corner, MenuAlign::Right);
        });
    }
}

impl ViewTest for ContextMenuBelow {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("100 56 b\n100 56 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(
                menu.frame().origin,
                Point::new(view.left.x(), view.left.max_y() + 4.0)
            );
        });
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        inject_touches("500 56 b\n500 56 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(
                Point::new(menu.max_x(), menu.y()),
                Point::new(view.right.max_x(), view.right.max_y() + 4.0)
            );
            assert!(menu.frame().size.width > view.right.frame().size.width);
        });
        check_colors(RIGHT_MENU)?;
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        inject_touches("540 584 b\n540 584 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(Point::new(menu.max_x(), menu.max_y()), Point::new(600.0, 600.0));
        });
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        Ok(())
    }
}
