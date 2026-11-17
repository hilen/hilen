use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    gm::color::Color,
    refs::Weak,
    ui::{
        BLACK, Container, Label, MarkdownStyle, MarkdownView, Setup, TextAlignment, VerticalAlignment,
        ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, checkpoint},
};

const TEXT: &str = r"A paragraph of text.

- an item
- another item

```
let code = 1;
```";

const FLUSH: MarkdownStyle = MarkdownStyle {
    text_inset: Some(0.0),
    ..MarkdownStyle::DEFAULT
};

const COLORS: &str = r"
             528   16 - #cbcbcb
              40   20 - #ffffff
              88   20 - #363636
             116   20 - #ffffff
             136   20 - #696969
             160   20 - #313131
             200   20 - #5b5b5b
             376   20 - #e9e9e9
             428   20 - #e9e9e9
             448   20 - #6a6a6a
             496   20 - #373737
              80   68 - #ffffff
             124   68 - #afafaf
             152   68 - #000000
             356   68 - #ffffff
             428   68 - #afafaf
             436   68 - #eaeaea
              64   96 - #909090
              96   96 - #090909
             380   96 - #000000
             404   96 - #424242
             100  116 - #000000
             388  116 - #dddddd
              64  120 - #909090
             132  120 - #c1c1c1
             368  120 - #909090
             428  120 - #141414
             152  160 - #454647
             456  160 - #454647
               4  252 - #ffffff
             280  252 - #ffffff
             632  252 - #ffffff
";

const WIDTH: f32 = 280.0;
const LEFT: f32 = 20.0;
const RIGHT: f32 = 340.0;
const TOP: f32 = 60.0;

/// Puts the default style back when it goes away, also when the code that
/// runs under another style panics. The style is global.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

fn with_flush_style<T>(action: impl FnOnce() -> T) -> T {
    let back = DefaultStyleBack;
    FLUSH.apply_globally();
    let result = action();
    drop(back);
    result
}

/// Where the text of the first label of a view starts, from the left edge of
/// the view, its inset, and the inset of its last label, the code inside its
/// box.
fn starts(text: Weak<MarkdownView>) -> (f32, f32, f32) {
    let labels: Vec<Weak<Label>> = text
        .subviews()
        .iter()
        .filter_map(|view| view.downcast_view::<Label>())
        .collect();
    let first = labels.first().expect("no text label");
    let code = labels.last().expect("no code label");
    (
        first.x() + first.text_inset(),
        first.text_inset(),
        code.text_inset(),
    )
}

/// The same text 2 times, left with the default style and right with a text
/// inset of 0. A line at the left edge of each view shows where the text
/// should start.
#[view]
struct MarkdownTextInset {
    #[init]
    default_caption: Label,
    flush_caption:   Label,
    default_edge:    Container,
    flush_edge:      Container,
    default_text:    MarkdownView,
    flush_text:      MarkdownView,
}

impl MarkdownTextInset {
    fn caption(label: Weak<Label>, text: &str, x: f32) {
        label
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_text(text);
        label.set_text_color(BLACK);
        label.set_frame((x, 12.0, WIDTH, 40.0));
    }

    fn show(text: Weak<MarkdownView>, edge: Weak<Container>, x: f32) {
        text.set_text_color(BLACK);
        text.set_text(TEXT);
        let height = text.height_for_width(WIDTH);
        text.set_frame((x, TOP, WIDTH, height));
        edge.set_color(Color::hex("#e05a2b"));
        edge.set_frame((x - 2.0, TOP, 2.0, height));
    }
}

impl Setup for MarkdownTextInset {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        Self::caption(
            self.default_caption,
            "default style, the text inset of a label",
            LEFT,
        );
        Self::caption(
            self.flush_caption,
            "text inset 0, the text starts at the line",
            RIGHT,
        );
        Self::show(self.default_text, self.default_edge, LEFT);
        with_flush_style(|| Self::show(self.flush_text, self.flush_edge, RIGHT));
    }
}

impl ViewTest for MarkdownTextInset {
    fn canvas() -> (u32, u32) {
        (640, 260)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(COLORS)?;
        checkpoint("left text inset by the default, right text at the line, code padded in its box")?;

        let (default, flush) = from_main(move || (starts(view.default_text), starts(view.flush_text)));
        let default_inset = default.1;
        ensure!(default_inset > 0.0, "the default style gave the text no inset");
        ensure!(
            (default.0 - default_inset).abs() < 0.01,
            "the default text starts at {}, not at the default inset {default_inset}",
            default.0
        );
        ensure!(
            flush.0.abs() < 0.01,
            "the flush text starts at {}, not at 0",
            flush.0
        );
        ensure!(
            (flush.2 - default_inset).abs() < 0.01,
            "the code of the flush text has the inset {}, not the padding {default_inset}",
            flush.2
        );

        // The style is the default again, a new view takes the default inset.
        let again = from_main(move || {
            let mut text = view.add_view::<MarkdownView>();
            text.set_text(TEXT);
            text.height_for_width(WIDTH);
            let start = starts(text).0;
            text.remove_from_superview();
            start
        });
        ensure!(
            (again - default_inset).abs() < 0.01,
            "a new view starts its text at {again}, the default style at {default_inset}"
        );

        Ok(())
    }
}
