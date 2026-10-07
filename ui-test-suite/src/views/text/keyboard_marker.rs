use hilen::{
    refs::Weak,
    ui::{Label, Rect, ScreenKeyboard, Setup, ViewData, ViewFrame, ViewSubviews, view},
};

/// Shows where the screen keyboard is, for the `screen_keyboard` tests. On
/// a phone the real keyboard covers it. Everywhere else the keyboard is
/// only a number the engine was told, and nobody could see why a view
/// moved. With no keyboard the marker has no size.
#[view]
pub(super) struct KeyboardMarker {
    #[init]
    label: Label,
}

impl Setup for KeyboardMarker {
    fn setup(self: Weak<Self>) {
        self.set_color((40, 44, 52));
        self.label.set_text("the keyboard").set_text_size(18);
        self.label.set_text_color((255, 255, 255));
        self.label.place().back();
        // The real keyboard is in front of everything, rows of a scroll
        // view included.
        self.bump_z_position(0.001);

        // The keyboard stays where it is on the glass while the screen
        // under it moves up, so the marker goes down inside the screen by
        // the same distance.
        self.place().custom(move |frame: &mut Rect| {
            let above = self.superview().size();
            *frame = match ScreenKeyboard::top() {
                Some(top) => {
                    let y = top + ScreenKeyboard::shift();
                    (0.0, y, above.width, (above.height - top).max(0.0)).into()
                }
                None => (0.0, 0.0, 0.0, 0.0).into(),
            };
        });
    }
}
