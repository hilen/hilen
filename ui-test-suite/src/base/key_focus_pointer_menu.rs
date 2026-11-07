use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        Button, Container, ContextMenu, Focus, Label, MenuAlign, MenuItem, MenuStyle, NamedKey, Setup, View,
        ViewData, ViewFrame, ViewSubviews, ViewTest, view,
    },
    ui_test::{checkpoint, inject_named_key, inject_touches},
};

/// The pointer of an LG Magic Remote shows at any move of the hand, and OK
/// is then a click at the pointer. So there the pointer puts the ring on
/// the row under it and the row with the ring is the one OK picks. Before,
/// the move hid the ring and the click landed off the menu and closed it.
#[view]
struct KeyFocusPointerMenu {
    picked:  String,
    /// A moved pointer draws nothing, the dot shows where it is.
    pointer: Weak<Container>,

    #[init]
    open:  Button,
    state: Label,
}

impl KeyFocusPointerMenu {
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

    fn expect(self: Weak<Self>, after: &str, expected: &str) -> Result<()> {
        wait_for_next_frame();
        wait_for_next_frame();
        let focused = self.focused();
        ensure!(
            focused == expected,
            "after {after} the ring is on {focused}, expected {expected}"
        );
        wait_for_next_frame();
        Ok(())
    }

    fn press(self: Weak<Self>, key: NamedKey, expected: &str) -> Result<()> {
        inject_named_key(key);
        self.expect(&format!("{key:?}"), expected)
    }

    fn move_pointer(self: Weak<Self>, x: f32, y: f32) {
        from_main(move || {
            let mut pointer = self.pointer;
            pointer.set_hidden(false);
            pointer.set_center((x, y));
        });
        inject_touches(format!("{x} {y} m"));
    }

    /// Moves the pointer to the middle of a menu row, shifted by `shift`
    /// points to the right.
    fn point_at_row(self: Weak<Self>, row: usize, shift: f32, expected: &str) -> Result<()> {
        let center = from_main(move || {
            let menu = ContextMenu::open();
            menu.items().get(row).map(|item| item.absolute_frame().center())
        });
        let Some(center) = center else {
            anyhow::bail!("the menu has no row {row}");
        };
        self.move_pointer(center.x + shift, center.y);
        self.expect(&format!("a move to row {row}"), expected)
    }

    fn picked(self: Weak<Self>) -> String {
        from_main(move || self.picked.clone())
    }
}

impl Setup for KeyFocusPointerMenu {
    fn setup(mut self: Weak<Self>) {
        self.picked = "nothing".into();
        Focus::set_pointer_moves_ring(true);

        self.open.set_text("Open the menu").set_text_size(20);
        self.open.set_color("#cfd8e3").set_corner_radius(10);
        self.open.set_frame((40, 60, 240, 60));
        self.open.on_tap(move || {
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

        // The z of the engine's own touch marks, set before it is added.
        let mut pointer = Container::new();
        pointer.set_z_position(0.1);
        pointer.set_color("#e0457b").set_corner_radius(7).set_size(14, 14);
        pointer.set_hidden(true);
        self.pointer = self.add_subview(pointer);
    }
}

impl ViewTest for KeyFocusPointerMenu {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        view.press(NamedKey::ArrowRight, "open")?;
        view.press(NamedKey::Enter, "row 0")?;
        checkpoint("the menu is open, the ring is on First")?;

        view.point_at_row(2, 0.0, "row 2")?;
        checkpoint("the pointer put the ring on Third")?;

        view.point_at_row(2, 2.0, "row 2")?;
        checkpoint("a small move of the hand keeps the ring on Third")?;

        view.press(NamedKey::Enter, "open")?;
        ensure!(
            view.picked() == "Third",
            "Enter after the pointer did not pick Third"
        );
        checkpoint("Third is picked, the ring is back on the button")?;

        view.press(NamedKey::Enter, "row 0")?;
        view.move_pointer(400.0, 400.0);
        view.expect("a move off the menu", "none")?;
        checkpoint("the pointer off the menu hides the ring")?;

        view.press(NamedKey::ArrowDown, "row 1")?;
        checkpoint("the arrow went on from First to Second")?;

        view.point_at_row(0, 0.0, "row 0")?;
        let row = from_main(move || {
            let menu = ContextMenu::open();
            menu.items().first().map(|item| item.absolute_frame().center())
        });
        let Some(row) = row else {
            anyhow::bail!("the menu has no first row");
        };
        inject_touches(format!("{x} {y} b\n{x} {y} e", x = row.x, y = row.y));
        view.expect("a click on First", "open")?;
        ensure!(view.picked() == "First", "the click on First did not pick it");
        checkpoint("a click picks the row with the ring")?;

        Ok(())
    }
}
