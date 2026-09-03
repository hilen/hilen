use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLUE, Container, CursorIcon, GRAY, Hover, Setup, ViewData, ViewTest, ViewTouch, view},
    ui_test::inject_touches,
};

#[view]
struct HoverCursor {
    #[init]
    handle: Container,
    plain:  Container,
}

impl Setup for HoverCursor {
    fn setup(self: Weak<Self>) {
        self.handle.set_color(BLUE);
        self.handle.place().tl(20).size(20, 200);
        self.handle.set_hover_cursor(CursorIcon::ColResize);

        self.plain.set_color(GRAY);
        self.plain.place().t(20).l(100).size(100, 200);
        self.plain.enable_hover();
    }
}

impl ViewTest for HoverCursor {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        from_main(Hover::clear);
        from_main(|| assert_eq!(Hover::cursor(), CursorIcon::Default));

        inject_touches("30 100 m");
        from_main(|| assert_eq!(Hover::cursor(), CursorIcon::ColResize));

        // A hovered view without a custom cursor restores the default.
        inject_touches("150 100 m");
        from_main(|| assert_eq!(Hover::cursor(), CursorIcon::Default));

        inject_touches("30 100 m");
        from_main(|| assert_eq!(Hover::cursor(), CursorIcon::ColResize));

        // Empty space restores the default too.
        inject_touches("400 400 m");
        from_main(|| assert_eq!(Hover::cursor(), CursorIcon::Default));

        // So does the cursor leaving the window.
        inject_touches("30 100 m");
        from_main(|| assert_eq!(Hover::cursor(), CursorIcon::ColResize));
        from_main(Hover::clear);
        from_main(|| assert_eq!(Hover::cursor(), CursorIcon::Default));

        Ok(())
    }
}
