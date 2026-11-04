use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{
        BLACK, Label, MarkdownStyle, MarkdownView, MathView, Setup, TextAlignment, VerticalAlignment,
        ViewData, ViewFrame, ViewSubviews, ViewTest, WHITE, view,
    },
    ui_test::{check_colors, set_record_probe_count},
};

const TEXT: &str = r"$$x = \frac{-b \pm \sqrt{b^{2} - 4ac}}{2a}$$";

/// Recorded with `--record-colors`.
const COLORS: &str = r"
             156    4 - #ffffff
             268    4 - #ffffff
             632    4 - #ffffff
             368   16 - #3b3b3b
             440   16 - #070707
             36   20 - #ffffff
             48   20 - #282828
             84   20 - #363636
             344   20 - #e1e1e1
             364   20 - #030303
             388   20 - #ffffff
             404   20 - #0b0b0b
             440   20 - #000000
             468   20 - #000000
             476   20 - #bababa
             488   20 - #ffffff
             508   20 - #ffffff
             188   60 - #ffffff
             220   64 - #ffffff
             460   64 - #b7b7b7
             504   64 - #ffffff
             548   64 - #363636
             564   64 - #ffffff
             152   68 - #d5d5d5
             176   68 - #ffffff
             456   68 - #afafaf
             464   68 - #afafaf
             508   68 - #1a1a1a
             548   68 - #373737
             460   72 - #b7b7b7
             492   72 - #ffffff
             544   72 - #a7a7a7
             632   72 - #ffffff
             104   76 - #9c9c9c
             108   76 - #9c9c9c
             296   76 - #ffffff
             448   76 - #ffffff
             456   76 - #626262
             476   76 - #ffffff
             488   76 - #ffffff
             564   76 - #ffffff
             8   80 - #ffffff
             400   84 - #e5e5e5
             404   84 - #e5e5e5
             408   84 - #e5e5e5
             184   88 - #ffffff
             68  132 - #ffffff
             4  152 - #ffffff
             136  152 - #ffffff
             220  152 - #ffffff
             292  152 - #ffffff
             364  152 - #ffffff
             456  152 - #ffffff
             544  152 - #ffffff
             632  152 - #ffffff
            ";

const SCALE: f32 = 1.3;

const STYLED: MarkdownStyle = MarkdownStyle {
    math_scale: SCALE,
    math_darkening: 0.5,
    ..MarkdownStyle::DEFAULT
};

const LEFT: f32 = 16.0;
const WIDTH: f32 = 296.0;
const RIGHT: f32 = 328.0;
const TOP: f32 = 50.0;

/// Puts the default style back when it goes away, also on a panic.
struct DefaultStyleBack;

impl Drop for DefaultStyleBack {
    fn drop(&mut self) {
        MarkdownStyle::DEFAULT.apply_globally();
    }
}

/// The same display formula 2 times, left with the default style and
/// right with a bigger formula and more ink.
#[view]
struct MarkdownMathStyle {
    #[init]
    plain_caption:  Label,
    styled_caption: Label,
    plain:          MarkdownView,
    styled:         MarkdownView,
}

impl MarkdownMathStyle {
    fn caption(label: Weak<Label>, text: &str, x: f32) {
        label
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_text(text);
        label.set_text_color(BLACK);
        label.set_frame((x, 12.0, WIDTH, 20.0));
    }

    fn show(text: Weak<MarkdownView>, x: f32) {
        text.set_text_color(BLACK);
        text.set_text(TEXT);
        let height = text.height_for_width(WIDTH);
        text.set_frame((x, TOP, WIDTH, height));
    }

    /// The width of the formula view of a markdown view.
    fn formula_width(text: Weak<MarkdownView>) -> f32 {
        text.subviews()
            .iter()
            .find_map(|child| child.downcast_view::<MathView>())
            .map_or(0.0, |math| math.width())
    }
}

impl Setup for MarkdownMathStyle {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        Self::caption(self.plain_caption, "default style", LEFT);
        Self::caption(self.styled_caption, "math scale 1.3, darkening 0.5", RIGHT);
        Self::show(self.plain, LEFT);
        let back = DefaultStyleBack;
        STYLED.apply_globally();
        Self::show(self.styled, RIGHT);
        drop(back);
    }
}

impl ViewTest for MarkdownMathStyle {
    fn canvas() -> (u32, u32) {
        (640, 160)
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        set_record_probe_count(80);
        check_colors(COLORS)?;

        let (plain, styled) =
            from_main(move || (Self::formula_width(view.plain), Self::formula_width(view.styled)));
        ensure!(plain > 0.0, "the default style shows no formula");
        let ratio = styled / plain;
        ensure!(
            (ratio - SCALE).abs() < 0.02,
            "the styled formula is {ratio} times as wide, not {SCALE}"
        );
        Ok(())
    }
}
