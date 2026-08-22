use anyhow::Result;

use crate::{
    self as hilen,
    deps::{hreads::from_main, refs::Weak},
    ui::{Alert, ScrollView, Setup, ViewData, ViewSubviews, ViewTest, view},
    ui_test::{inject_scroll, inject_touches},
};

#[view]
struct WheelScrollTest {
    inner: Weak<ScrollView>,

    #[init]
    under: ScrollView,
    outer: ScrollView,
}

impl Setup for WheelScrollTest {
    fn setup(mut self: Weak<Self>) {
        self.under.set_content_size((300, 1200));
        self.under.place().tr(0).size(300, 600);

        self.outer.set_content_size((600, 1200));
        self.outer.place().back();

        self.inner = self.outer.add_view();
        self.inner.set_content_size((200, 800));
        self.inner.place().tl(50).size(200, 200);
    }
}

impl ViewTest for WheelScrollTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let offsets = move || {
            from_main(move || {
                (
                    view.under.get_scroll_content_offset(),
                    view.outer.get_scroll_content_offset(),
                    view.inner.get_scroll_content_offset(),
                )
            })
        };

        // Over the nested scroll the deepest one wins.
        inject_touches("150 150 m");
        inject_scroll(-100);
        assert_eq!(offsets(), (0.0, 0.0, -100.0));

        // Outside the nested scroll the fullscreen one wins.
        // `under` is below it and must not scroll.
        inject_touches("450 400 m");
        inject_scroll(-100);
        assert_eq!(offsets(), (0.0, -100.0, -100.0));

        // A modal layer blocks wheel scrolling under it.
        from_main(|| Alert::show("wheel"));
        inject_scroll(-100);
        assert_eq!(offsets(), (0.0, -100.0, -100.0));

        // After the modal is dismissed wheel scrolling works again.
        inject_touches(
            "
            320 383 b
            320 383 e
        ",
        );
        inject_scroll(-100);
        assert_eq!(offsets(), (0.0, -200.0, -100.0));

        // A hidden scroll view doesn't take the wheel:
        // the one below it scrolls instead.
        from_main(move || {
            view.outer.set_hidden(true);
        });
        inject_touches("450 400 m");
        inject_scroll(-100);
        assert_eq!(offsets(), (-100.0, -200.0, -100.0));

        // A scroll view inside a hidden one doesn't scroll either.
        inject_touches("150 150 m");
        inject_scroll(-100);
        assert_eq!(offsets(), (-100.0, -200.0, -100.0));

        // A hidden scroll view doesn't capture drag scrolling
        // and doesn't shadow the visible one below.
        from_main(move || {
            view.outer.set_hidden(false);
            view.under.set_hidden(true);
        });
        inject_touches(
            "
            450 400 b
            450 300 m
        ",
        );
        assert_eq!(offsets(), (-100.0, -300.0, -100.0));
        inject_touches("450 300 e");

        Ok(())
    }
}
