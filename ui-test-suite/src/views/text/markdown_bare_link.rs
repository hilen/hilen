use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLACK, MarkdownView, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, inject_touches},
};

/// A bare url in a line with a dot after it, and one alone in its paragraph.
const TEXT: &str = r"The site is https://example.com/page. Click it.

https://example.com/alone
";

const DRAWN: &str = r"
             528    4 - #ffffff
             104   24 - #5192f7
             192   24 - #2f7df6
             300   24 - #aaaaaa
              40   28 - #2b2b2b
              72   28 - #767676
              76   28 - #b8b8b8
             124   28 - #ffffff
             152   28 - #b2cffc
             176   28 - #2f7df6
             208   28 - #ffffff
             260   28 - #2f7df6
             280   28 - #ffffff
             296   28 - #ffffff
             300   28 - #aaaaaa
             312   28 - #5d5d5d
             316   28 - #9f9f9f
             228   32 - #ffffff
             124   52 - #a5c7fb
             180   52 - #3d86f7
              56   56 - #ffffff
             124   56 - #a5c7fb
             164   56 - #7daef9
              40   60 - #2f7df6
              88   60 - #2f7df6
             124   60 - #2f7df6
             144   60 - #2f7df6
             200   60 - #2f7df6
               4  192 - #ffffff
             276  192 - #ffffff
             432  192 - #ffffff
             632  192 - #ffffff
";

#[view]
struct MarkdownBareLink {
    #[init]
    text: MarkdownView,
}

impl Setup for MarkdownBareLink {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.text.set_text_color(BLACK);
        // A tap must not open a browser on the machine that runs the test.
        self.text.set_opens_links(false);
        self.text.set_text(TEXT);
        let height = self.text.height_for_width(600.0);
        self.text.set_frame((20.0, 20.0, 600.0, height));
    }
}

impl ViewTest for MarkdownBareLink {
    fn canvas() -> (u32, u32) {
        (640, 200)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(DRAWN)?;

        let tapped = from_main(move || {
            let (sender, receiver) = std::sync::mpsc::channel();
            view.text.link_tapped.val(move |url| {
                sender.send(url).expect("the test waits for the link");
            });
            receiver
        });
        inject_touches("180 28 b\n180 28 e");
        assert_eq!(
            tapped.try_recv().ok().as_deref(),
            Some("https://example.com/page")
        );
        inject_touches("120 55 b\n120 55 e");
        assert_eq!(
            tapped.try_recv().ok().as_deref(),
            Some("https://example.com/alone")
        );
        inject_touches("300 28 b\n300 28 e");
        assert!(tapped.try_recv().is_err(), "a tap on plain text hit a link");
        Ok(())
    }
}
