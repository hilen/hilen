use anyhow::Result;
use hilen::{
    dispatch::from_main,
    gm::color::Color,
    refs::Weak,
    ui::{
        Button, Container, ContextMenu, MenuAlign, MenuItem, NamedKey, Point, Setup, ViewData, ViewFrame,
        ViewTest, view,
    },
    ui_test::{check_colors, inject_named_key, inject_touches},
};

/// A bar at the bottom of the window with a button at each end, like the
/// controls of a player. A menu under such a button does not fit and
/// slides up over the button, a menu over it leaves the bar free.
#[view]
struct ContextMenuAbove {
    #[init]
    bar:   Container,
    left:  Button,
    right: Button,
    top:   Button,
}

fn items() -> Vec<MenuItem> {
    vec![
        MenuItem::new("Off", || {}).checked(false),
        MenuItem::new("English", || {}).checked(true),
        MenuItem::new("Lithuanian subtitles", || {}).checked(false),
    ]
}

impl Setup for ContextMenuAbove {
    fn setup(self: Weak<Self>) {
        self.bar.set_color(Color::rgb(0.2, 0.2, 0.22));
        self.bar.place().lrb(0).h(48);

        self.left.set_text("Sound");
        self.left.place().b(8).l(40).size(160, 32);
        self.left.on_tap(move || {
            ContextMenu::show_above(items(), self.left, MenuAlign::Left);
        });

        // A button at the right end of the bar. Its menu grows up and to
        // the left, the bar and the button stay free.
        self.right.set_text("Subtitles");
        self.right.place().b(8).r(40).size(160, 32);
        self.right.on_tap(move || {
            ContextMenu::show_above(items(), self.right, MenuAlign::Right);
        });

        // No room above, so the menu slides back down inside the screen.
        self.top.set_text("Top");
        self.top.place().t(0).l(0).size(160, 32);
        self.top.on_tap(move || {
            ContextMenu::show_above(items(), self.top, MenuAlign::Left);
        });
    }
}

impl ViewTest for ContextMenuAbove {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("100 576 b\n100 576 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(
                Point::new(menu.x(), menu.max_y()),
                Point::new(view.left.x(), view.left.y() - 4.0)
            );
        });
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        inject_touches("500 576 b\n500 576 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(
                Point::new(menu.max_x(), menu.max_y()),
                Point::new(view.right.max_x(), view.right.y() - 4.0)
            );
            assert!(menu.frame().size.width > view.right.frame().size.width);
        });
        check_colors(RIGHT_MENU)?;
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        inject_touches("60 16 b\n60 16 e");
        from_main(move || {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(menu.frame().origin, Point::new(0.0, 0.0));
        });
        inject_named_key(NamedKey::Escape);
        from_main(|| assert!(ContextMenu::open().is_null()));

        Ok(())
    }
}

const RIGHT_MENU: &str = r"
   4    4 - #ffffff
  56    4 - #272727
 156    4 - #ffffff
 592    4 - #597c95
  92   20 - #000000
 348  184 - #597c95
 396  464 - #d1d1d6
 512  464 - #d1d1d6
 436  476 - #ffffff
 560  488 - #445f72
 372  496 - #476377
 464  508 - #909090
 436  512 - #c5c5c5
 428  536 - #868686
 532  536 - #c3c3c3
 484  540 - #444444
  84  568 - #ffffff
 160  568 - #292929
 124  572 - #727272
 440  572 - #000000
 456  572 - #7d7d7d
 124  576 - #727272
 516  576 - #e6e6e6
 104  580 - #ffffff
 420  580 - #4f4f4f
 488  580 - #dbdbdb
 144  584 - #010101
 160  584 - #000000
 432  584 - #010101
 456  584 - #7d7d7d
   4  592 - #333338
 300  592 - #333338
";
