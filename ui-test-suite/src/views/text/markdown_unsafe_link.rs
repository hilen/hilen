use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLACK, MarkdownView, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, inject_touches},
};

/// A safe link, a `javascript:` link and a bare `javascript://` address. Only
/// the first is a link, a stranger's script must stay plain text.
const TEXT: &str = r"[safe link](https://example.com/safe)

[script link stays text](javascript:alert(document.domain))

javascript://x%0Aalert(1) stays text
";

const DRAWN: &str = r"
             320    4 - #ffffff
             456    4 - #ffffff
             632   12 - #ffffff
              68   24 - #669ff8
              52   28 - #80aff9
              68   28 - #669ff8
              76   28 - #ffffff
              76   32 - #ffffff
              84   32 - #ffffff
              76   52 - #101010
              48   56 - #ffffff
              56   56 - #5e5e5e
              60   56 - #000000
              76   56 - #101010
              80   56 - #a9a9a9
              88   56 - #d1d1d1
             140   56 - #ffffff
             144   56 - #f1f1f1
              60   60 - #131313
              76   80 - #aaaaaa
              84   80 - #dbdbdb
             124   80 - #ffffff
             152   80 - #919191
             176   80 - #ffffff
             180   80 - #858585
              84   84 - #dbdbdb
             212   84 - #ffffff
             508  132 - #ffffff
               4  192 - #ffffff
             264  192 - #ffffff
             384  192 - #ffffff
             632  192 - #ffffff
";

#[view]
struct MarkdownUnsafeLink {
    #[init]
    text: MarkdownView,
}

impl Setup for MarkdownUnsafeLink {
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

impl ViewTest for MarkdownUnsafeLink {
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
        inject_touches("60 28 b\n60 28 e");
        assert_eq!(
            tapped.try_recv().ok().as_deref(),
            Some("https://example.com/safe")
        );
        inject_touches("80 54 b\n80 54 e");
        assert!(
            tapped.try_recv().is_err(),
            "a tap on the javascript: link opened it"
        );
        inject_touches("80 80 b\n80 80 e");
        assert!(
            tapped.try_recv().is_err(),
            "a tap on the bare javascript:// address opened it"
        );
        Ok(())
    }
}
