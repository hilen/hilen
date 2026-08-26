use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Color, Container, ContextMenu, MenuItem, Setup, ViewData, ViewFrame, ViewTest, ViewTouch, view},
    ui_test::inject_right_click,
};

#[view]
struct ContextMenuBadges {
    #[init]
    row: Container,
}

impl Setup for ContextMenuBadges {
    fn setup(self: Weak<Self>) {
        self.row.set_color("#d9e6ff");
        self.row.place().t(40).lr(40).h(60);
        self.row.enable_touch();
        self.row.touch().secondary.sub(self, move || {
            ContextMenu::show_at_cursor(vec![
                MenuItem::new("thing", || {})
                    .badge("\u{2193}2", Color::hex("#ef4444"))
                    .badge("\u{2191}1", Color::hex("#10b981")),
                MenuItem::new("clean repo", || {}).badge("\u{2713}", Color::hex("#10b981")),
                MenuItem::new("plain", || {}),
            ]);
        });
    }
}

impl ViewTest for ContextMenuBadges {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        inject_right_click(100, 70);

        from_main(|| {
            let menu = ContextMenu::open();
            assert!(menu.is_ok());
            assert_eq!(menu.items().len(), 3);

            let first = menu.items()[0];
            assert_eq!(first.badges().len(), 2);
            assert_eq!(first.badges()[0].text(), "\u{2193}2");
            assert_eq!(first.badges()[1].text(), "\u{2191}1");

            // Badges sit in order at the right edge of the row, after
            // the title.
            let down = first.badges()[0].frame();
            let up = first.badges()[1].frame();
            assert!(down.max_x() <= up.x());
            assert!(up.max_x() < first.frame().size.width);

            let second = menu.items()[1];
            assert_eq!(second.badges().len(), 1);
            assert_eq!(second.badges()[0].text(), "\u{2713}");

            assert!(menu.items()[2].badges().is_empty());

            ContextMenu::dismiss_open();
        });

        Ok(())
    }
}
