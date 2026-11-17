use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Color, Container, Label, MarkdownLinkHosts, MarkdownStyle, MarkdownView, Setup, TextAlignment,
        VerticalAlignment, ViewData, ViewFrame, ViewTest, WHITE, view,
    },
    ui_test::check_colors,
};

/// A link whose text hides where it goes, a link whose text is its own
/// address, and a mail link.
const TEXT: &str = r"Read [my guide](https://evil.example/x) first.

Or open [https://good.example/a](https://good.example/a).

Then [write me](mailto:ann@good.example).
";

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             632    4 - #ffffff
             344   16 - #e5e5e5
              36   20 - #ffffff
             396   20 - #ffffff
              76   24 - #ffffff
             460   32 - #000000
              56   36 - #b2b2b2
             120   36 - #b4b4b4
             376   36 - #cecece
             416   36 - #010101
             424   36 - #707070
             440   36 - #cecece
             456   36 - #414141
              68   68 - #2f7df6
             148   68 - #4a4c51
             356   68 - #92979f
             380   68 - #2f7df6
             496   68 - #6b7280
             528   68 - #e0e8f5
             208   92 - #609cf8
             192   96 - #9fc3fa
             348   96 - #e8f0fe
             472   96 - #9dc1fa
             520   96 - #609cf8
             136  100 - #2f7df6
             436  100 - #2f7df6
              36  120 - #27282b
              84  120 - #aac9fb
             348  120 - #27282b
             112  124 - #d1e2fd
             392  124 - #e8f0fe
             632  192 - #ffffff
            ";

const WIDTH: f32 = 296.0;
const LEFT: f32 = 16.0;
const RIGHT: f32 = 328.0;
const TOP: f32 = 60.0;

/// Puts the default style back when it goes away, also when the code that
/// runs under another style panics. The style is global, a test that left
/// its own behind would draw every markdown view after it with it.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

/// The same text 2 times. Left with the default style, a link shows only
/// its text. Right with the hosts shown: `evil.example` stands in gray
/// after `my guide`, the 2 other links get nothing. The tinted box behind
/// each text is as high as `height_for_width` said.
#[view]
struct MarkdownLinkHostsTest {
    #[init]
    hidden_caption: Label,
    shown_caption:  Label,
    hidden_box:     Container,
    shown_box:      Container,
    hidden_text:    MarkdownView,
    shown_text:     MarkdownView,
}

impl MarkdownLinkHostsTest {
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

    /// Lays the text out with the style that is global right now.
    fn show(text: Weak<MarkdownView>, background: Weak<Container>, x: f32) -> f32 {
        text.set_text_color(BLACK);
        text.set_text(TEXT);
        let height = text.height_for_width(WIDTH);
        text.set_frame((x, TOP, WIDTH, height));
        background.set_color(Color::hex("#e8f0fe"));
        background.set_frame((x, TOP, WIDTH, height));
        height
    }
}

impl Setup for MarkdownLinkHostsTest {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);

        Self::caption(
            self.hidden_caption,
            "default style\na link shows only its text",
            LEFT,
        );
        Self::caption(
            self.shown_caption,
            "hosts shown\ngray evil.example after my guide",
            RIGHT,
        );

        Self::show(self.hidden_text, self.hidden_box, LEFT);

        let back = DefaultStyleBack;
        MarkdownStyle {
            link_hosts: MarkdownLinkHosts::Outside,
            ..MarkdownStyle::DEFAULT
        }
        .apply_globally();
        Self::show(self.shown_text, self.shown_box, RIGHT);
        drop(back);
    }
}

impl ViewTest for MarkdownLinkHostsTest {
    fn canvas() -> (u32, u32) {
        (640, 200)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(COLORS)?;

        // The host adds text to 1 line and no line, both texts fit the width.
        let (hidden, shown) = from_main(move || (view.hidden_text.height(), view.shown_text.height()));
        ensure!(
            (hidden - shown).abs() < 0.5,
            "the text with hosts is {shown} high, the one without {hidden}"
        );

        Ok(())
    }
}
