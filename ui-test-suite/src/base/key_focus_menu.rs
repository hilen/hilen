use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Button, ContextMenu, Focus, Label, MenuAlign, MenuItem, MenuStyle, NamedKey, Setup, View, ViewData,
        ViewFrame, ViewTest, view,
    },
    ui_test::{checkpoint, inject_named_key},
};

/// A menu opened with the remote is walked row by row with the arrows. The
/// ring goes into the menu by itself, never onto the backdrop that closes
/// it, and goes back to the button when a row is picked. An LG TV showed
/// the ring around the whole menu and no row could be reached.
#[view]
struct KeyFocusMenu {
    picked: String,

    #[init]
    open:  Button,
    state: Label,
}

impl KeyFocusMenu {
    fn focused(self: Weak<Self>) -> String {
        from_main(move || {
            let focused = Focus::focused();
            let is = |view: &dyn View| focused.is_ok() && focused.raw() == view.weak_view().raw();
            let menu = ContextMenu::open();
            let row = menu.is_ok().then(|| menu.items().iter().position(|item| is(&**item))).flatten();
            let name = if is(&*self.open) {
                "open".to_string()
            } else if let Some(row) = row {
                format!("row {row}")
            } else if focused.is_ok() {
                "another view".to_string()
            } else {
                "none".to_string()
            };
            self.state.set_text(format!("ring: {name}, picked: {}", self.picked));
            name
        })
    }

    fn press(self: Weak<Self>, key: NamedKey, expected: &str) -> Result<()> {
        inject_named_key(key);
        wait_for_next_frame();
        wait_for_next_frame();
        let focused = self.focused();
        ensure!(
            focused == expected,
            "after {key:?} the ring is on {focused}, expected {expected}"
        );
        wait_for_next_frame();
        Ok(())
    }
}

impl Setup for KeyFocusMenu {
    fn setup(mut self: Weak<Self>) {
        self.picked = "nothing".into();

        self.open.set_text("Open the menu").set_text_size(20);
        self.open.set_color("#cfd8e3").set_corner_radius(10);
        self.open.set_frame((40, 60, 240, 60));
        self.open.on_tap(move || {
            // The gap of the separator leaves the middle of the menu panel
            // nearer below the first row than the second row is.
            let item =
                move |title: &'static str| MenuItem::new(title, move || self.picked = title.to_string());
            let items = vec![
                item("First"),
                MenuItem::separator(),
                item("Second"),
                item("Third"),
            ];
            // The look of the menu over a video, a blur and an arrow.
            let style = MenuStyle {
                blur: 16.0,
                arrow: 8.0,
                ..MenuStyle::default()
            };
            style.show_below(items, self.open, MenuAlign::Left);
        });

        self.state.set_text_size(20);
        self.state.place().lrb(10).h(40);
    }
}

impl ViewTest for KeyFocusMenu {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        view.press(NamedKey::ArrowRight, "open")?;
        checkpoint("the ring is on the open button")?;

        view.press(NamedKey::Enter, "row 0")?;
        checkpoint("the menu is open, the ring is on First")?;

        view.press(NamedKey::ArrowDown, "row 1")?;
        view.press(NamedKey::ArrowDown, "row 2")?;
        checkpoint("the ring walked down to Third")?;

        view.press(NamedKey::ArrowUp, "row 1")?;
        view.press(NamedKey::Enter, "open")?;
        ensure!(
            from_main(move || view.picked.clone()) == "Second",
            "Enter on the second row did not pick it"
        );
        checkpoint("Second is picked, the menu is gone, the ring is back on the button")?;
        Ok(())
    }
}
