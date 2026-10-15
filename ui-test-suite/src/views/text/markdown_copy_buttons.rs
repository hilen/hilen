use anyhow::Result;
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{BLACK, MarkdownView, Setup, TextSelection, Toast, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{check_colors, checkpoint, inject_touches},
};

/// A url in inline code, a bare url, plain inline code and a code block.
const TEXT: &str = r"Open `https://192.168.0.100:47990` or https://example.com/page. Run `make fly` now.

```sh
echo copied
```
";

const SHOWN: &str = r"
             304   64 - #c5d3ea
             504   64 - #c5d3ea
              40   68 - #ffffff
              84   68 - #71a6f9
              92   68 - #2f7df6
             144   68 - #85b3fa
             228   68 - #85b3fa
             304   68 - #94bcfb
             416   68 - #ffffff
             504   68 - #88b4fa
             548   68 - #e4bdae
             552   68 - #c36d4b
             160   72 - #2f7df6
             264   72 - #ffffff
             336   72 - #ffffff
             376   72 - #ffffff
             508   72 - #a6c7fc
             604   72 - #b4491f
             588  108 - #a3c3f6
             584  112 - #2f7df6
             592  112 - #a3c3f6
             584  116 - #2f7df6
             588  116 - #2f7df6
             588  120 - #2f7df6
              84  124 - #98beeb
             136  124 - #eef1f5
             324  204 - #ffffff
             100  220 - #ffffff
               4  232 - #ffffff
             196  232 - #ffffff
             456  232 - #ffffff
             632  232 - #ffffff
";

const LINK_COPIED: &str = r"
             328   36 - #333338
             272   40 - #e8e8e8
             304   40 - #333338
             316   40 - #333338
             292   44 - #333338
             340   48 - #e8e8e8
             308   52 - #333338
             328   56 - #333338
             280   60 - #c5c5c5
             296   60 - #c3c3c3
             320   60 - #c3c3c3
             304   64 - #aebbcf
             312   64 - #e1e1e1
             504   64 - #c5d3ea
              92   68 - #2f7df6
             144   68 - #85b3fa
             156   68 - #ffffff
             228   68 - #85b3fa
             416   68 - #ffffff
             504   68 - #88b4fa
             548   68 - #e4bdae
             552   68 - #c36d4b
             160   72 - #2f7df6
             336   72 - #ffffff
             508   72 - #a6c7fc
             604   72 - #b4491f
             588  108 - #a3c3f6
             588  120 - #2f7df6
              84  124 - #98beeb
               4  232 - #ffffff
             240  232 - #ffffff
             424  232 - #ffffff
";

const CODE_COPIED: &str = r"
             552   28 - #e9e9e9
             604   32 - #333338
             584   36 - #333338
             564   40 - #333338
             612   40 - #d3d3d3
             548   48 - #e1e1e1
             560   52 - #333338
             612   52 - #d3d3d3
             596   56 - #bbbbbb
             580   60 - #dadada
             304   64 - #c5d3ea
             504   64 - #c5d3ea
              40   68 - #ffffff
              84   68 - #71a6f9
              92   68 - #2f7df6
             144   68 - #85b3fa
             228   68 - #85b3fa
             416   68 - #ffffff
             548   68 - #e4bdae
             552   68 - #c36d4b
             160   72 - #2f7df6
             336   72 - #ffffff
             508   72 - #a6c7fc
             604   72 - #b4491f
             584  112 - #2f7df6
             592  112 - #a3c3f6
             588  120 - #2f7df6
              84  124 - #98beeb
             136  124 - #eef1f5
               4  232 - #ffffff
             196  232 - #ffffff
             456  232 - #ffffff
";

const BLOCK_COPIED: &str = r"
             304   64 - #c5d3ea
             504   64 - #c5d3ea
              40   68 - #ffffff
              84   68 - #71a6f9
              92   68 - #2f7df6
             144   68 - #85b3fa
             228   68 - #85b3fa
             416   68 - #ffffff
             548   68 - #e4bdae
             552   68 - #c36d4b
             160   72 - #2f7df6
             336   72 - #ffffff
             508   72 - #a6c7fc
             572   76 - #e1e1e1
             588   80 - #333338
             616   80 - #333338
             560   84 - #d3d3d3
             576   88 - #333338
             596   92 - #a7a7a9
             612   92 - #333338
             556  104 - #e1e4e8
             604  104 - #bbbbbb
             616  104 - #bcbcbc
             572  108 - #ccced2
             588  108 - #8ca7d3
             592  112 - #9bbaeb
             588  120 - #2f7df6
              84  124 - #98beeb
             136  124 - #eef1f5
               4  232 - #ffffff
             196  232 - #ffffff
             448  232 - #ffffff
";

#[view]
struct MarkdownCopyButtons {
    #[init]
    text: MarkdownView,
}

impl Setup for MarkdownCopyButtons {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        self.text.set_text_color(BLACK);
        // A tap must not open a browser on the machine that runs the test.
        self.text.set_opens_links(false);
        self.text.set_copy_buttons(true);
        self.text.set_selectable(true);
        self.text.set_text(TEXT);
        let height = self.text.height_for_width(600.0);
        self.text.set_frame((20.0, 60.0, 600.0, height));
    }
}

impl ViewTest for MarkdownCopyButtons {
    fn canvas() -> (u32, u32) {
        (640, 240)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(SHOWN)?;
        checkpoint("a copy icon after both links and in the code block")?;

        let tapped = from_main(move || {
            let (sender, receiver) = std::sync::mpsc::channel();
            view.text.link_tapped.val(move |url| {
                sender.send(url).expect("the test waits for the link");
            });
            receiver
        });
        let copied = from_main(move || {
            let (sender, receiver) = std::sync::mpsc::channel();
            view.text.copied.val(move |text| {
                sender.send(text).expect("the test waits for the copy");
            });
            receiver
        });

        // The url in inline code is a link, a tap opens it and copies nothing.
        inject_touches("150 68 b\n150 68 e");
        assert_eq!(
            tapped.try_recv().ok().as_deref(),
            Some("https://192.168.0.100:47990")
        );
        assert!(
            from_main(|| Toast::shown().is_null()),
            "a link tap showed a toast"
        );

        assert!(copied.try_recv().is_err(), "a link tap copied");

        inject_touches("306 68 b\n306 68 e");
        assert_eq!(
            copied.try_recv().ok().as_deref(),
            Some("https://192.168.0.100:47990")
        );
        assert!(tapped.try_recv().is_err(), "the copy icon opened the link");
        assert!(from_main(|| Toast::shown().is_ok()), "no toast after a copy");
        check_colors(LINK_COPIED)?;
        checkpoint("Copied over the icon after the first link")?;

        inject_touches("580 68 b\n580 68 e");
        assert_eq!(copied.try_recv().ok().as_deref(), Some("make fly"));
        check_colors(CODE_COPIED)?;
        checkpoint("Copied over the inline code")?;

        inject_touches("591 116 b\n591 116 e");
        assert_eq!(copied.try_recv().ok().as_deref(), Some("echo copied"));
        check_colors(BLOCK_COPIED)?;
        checkpoint("Copied over the icon of the code block")?;

        // Plain text copies nothing.
        inject_touches("50 68 b\n50 68 e");
        assert!(copied.try_recv().is_err(), "a tap on plain text copied");
        assert!(tapped.try_recv().is_err(), "a tap on plain text hit a link");

        // A copy of the selection has no room for the icons in it.
        let selected = from_main(|| {
            TextSelection::select_all();
            TextSelection::text()
        });
        assert!(
            selected.contains("https://example.com/page. Run make fly now."),
            "the selection copy is {selected:?}"
        );
        assert!(!selected.contains('\u{2003}'));
        from_main(|| {
            TextSelection::clear();
            Toast::hide();
        });
        Ok(())
    }
}
