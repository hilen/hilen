use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, Label, MarkdownView, Setup, TextAlignment, VerticalAlignment, ViewData,
        ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

/// A list whose second item is only bold text over a nested list, then a
/// paragraph that is bold as a whole. Such a text has no char in the
/// regular font, every one is drawn by the bold font of a run.
const TEXT: &str = r"- **Done.** an item that starts with bold text
- **Docs.**
  - a nested item
  - a second nested item

**A paragraph that is bold as a whole.**
";

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             384    4 - #ffffff
             148   16 - #7f7f7f
             288   16 - #4b4b4b
             296   16 - #191919
              64   20 - #0b0b0b
              68   20 - #aeaeae
              84   20 - #343434
             104   20 - #ffffff
             128   20 - #ffffff
             144   20 - #b7b7b7
             184   20 - #9c9c9c
             204   20 - #989898
             220   20 - #000000
             244   20 - #ffffff
             288   20 - #4b4b4b
             296   20 - #000000
             348   20 - #ffffff
             168   24 - #ffffff
              32   32 - #e3e3e3
              52   32 - #000000
              92   32 - #fefefe
             232   32 - #464646
              32   36 - #e3e3e3
              56   36 - #1e1e1e
             112   36 - #cecece
             124   36 - #050505
             148   36 - #cecece
             156   36 - #000000
             168   36 - #ffffff
             196   36 - #000000
             208   36 - #818181
             220   36 - #cecece
             224   36 - #7f7f7f
             232   36 - #464646
             252   36 - #ffffff
              52   64 - #cfd6e2
             148   64 - #404246
             240   64 - #54575c
             256   64 - #3e4044
             264   64 - #35373a
              52   68 - #cfd6e2
             100   68 - #83888f
             116   68 - #1c1d1f
             144   68 - #62666c
             152   68 - #030303
             164   68 - #797d84
             176   68 - #e8f0fe
             196   68 - #000000
             256   68 - #3e4044
             264   68 - #000000
             272   68 - #4b4d52
             288   68 - #e8f0fe
               4   76 - #ffffff
             332   80 - #e8f0fe
              52   88 - #cfd6e2
              52   92 - #cfd6e2
              72   92 - #e8f0fe
             392   92 - #ffffff
             124  108 - #8c9199
             108  112 - #bfc6d1
             140  112 - #060707
             244  112 - #e8f0fe
             288  120 - #e8f0fe
               4  124 - #ffffff
             156  132 - #000000
             108  136 - #e8f0fe
             124  136 - #e8f0fe
             144  136 - #dbe3f0
             156  136 - #000000
             164  136 - #dbe3f0
             172  136 - #000000
             180  136 - #b1b8c2
             200  136 - #d9e0ed
             204  136 - #575a60
             332  152 - #e8f0fe
             392  152 - #ffffff
             172  160 - #000000
             180  160 - #000000
              48  164 - #e8f0fe
              76  164 - #e8f0fe
              96  164 - #e8f0fe
             116  164 - #e8f0fe
             156  164 - #000000
             164  164 - #010101
             172  164 - #000000
             208  164 - #000000
             236  164 - #e8f0fe
             248  164 - #000000
             348  196 - #ffffff
             184  208 - #ffffff
               4  212 - #ffffff
              64  212 - #ffffff
             136  212 - #ffffff
             228  212 - #ffffff
             300  212 - #ffffff
             392  212 - #ffffff
            ";

const WIDTH: f32 = 360.0;
const LEFT: f32 = 16.0;
const TOP: f32 = 60.0;

/// The views of `TEXT` in the order the view makes them: the 2 markers of
/// the list, the text of the first item, the text of the second one, the 2
/// markers and the 2 texts of the nested list, then the paragraph.
const MIXED_ITEM: usize = 2;
const BOLD_ITEM: usize = 3;
const NESTED_ITEM: usize = 6;
const BOLD_PARAGRAPH: usize = 8;
const VIEWS: usize = 9;

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

/// A text that is bold from its first char to its last one keeps its
/// room. It was measured as nothing: the item `Docs.` drew no text and its
/// nested list moved up into its line. The tinted box is as high as
/// `height_for_width` said.
#[view]
struct MarkdownBoldItem {
    #[init]
    caption:    Label,
    background: Container,
    text:       MarkdownView,
}

impl Setup for MarkdownBoldItem {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        self.caption
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        self.caption
            .set_text("the item Docs. and the last paragraph are bold as a whole\nboth show, the nested list is under Docs.");
        self.caption.set_text_color(BLACK);
        self.caption.set_frame((LEFT, 12.0, WIDTH, 40.0));

        self.text.set_text_color(BLACK);
        self.text.set_text(TEXT);
        let height = self.text.height_for_width(WIDTH);
        self.text.set_frame((LEFT, TOP, WIDTH, height));
        self.background.set_color(Color::hex("#e8f0fe"));
        self.background.set_frame((LEFT, TOP, WIDTH, height));
    }
}

impl ViewTest for MarkdownBoldItem {
    fn canvas() -> (u32, u32) {
        (400, 220)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        // 3 times the default, the picture is mostly small text.
        set_record_probe_count(96);
        check_colors(COLORS)?;

        // The top and the bottom of every view the text made.
        let frames: Vec<(f32, f32)> =
            from_main(move || view.text.subviews().iter().map(|view| (view.y(), view.max_y())).collect());
        ensure!(frames.len() == VIEWS, "{} views, not {VIEWS}", frames.len());
        let height = |index: usize| frames[index].1 - frames[index].0;

        let line = height(MIXED_ITEM);
        ensure!(line > 1.0, "the first item is {line} high");
        for (name, index) in [
            ("the bold item", BOLD_ITEM),
            ("the bold paragraph", BOLD_PARAGRAPH),
        ] {
            ensure!(
                near(height(index), line),
                "{name} is {} high, a line of text is {line}",
                height(index)
            );
        }
        ensure!(
            frames[NESTED_ITEM].0 >= frames[BOLD_ITEM].1,
            "the nested list starts at {}, the text of its item ends at {}",
            frames[NESTED_ITEM].0,
            frames[BOLD_ITEM].1
        );

        Ok(())
    }
}
