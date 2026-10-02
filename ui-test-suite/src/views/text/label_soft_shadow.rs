use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{BLACK, Color, Container, Label, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::check_colors,
};

const CHECK_1: &str = r"
     200   48 - #404040
     320   52 - #fefefe
     436   52 - #6e6e6e
     140   60 - #000000
     236   68 - #929292
     352  156 - #dcdcdc
     448  156 - #000000
     192  164 - #414141
     256  164 - #262626
     472  256 - #252525
     160  264 - #c5c5c5
     248  264 - #3a3a3a
     380  264 - #5f5f5f
     592  264 - #000000
     304  272 - #ffffff
     296  352 - #c6c6c6
     424  356 - #585858
     200  364 - #959595
     252  364 - #b2b2b2
     140  368 - #cecece
     256  368 - #afafaf
       4  372 - #ffffff
     368  448 - #adbbc9
     432  452 - #103050
     212  456 - #c8e3ff
     296  460 - #b3d9ff
     396  460 - #123558
     364  464 - #133a61
     168  468 - #c6e3ff
     252  476 - #b1d8ff
     432  476 - #081727
       4  592 - #597c95
";

/// White text over a white band and over a black band, 5 rows, each named
/// by its own text. Proves an outline wider than 2 points still closes into
/// a ring around every glyph with no gaps, a shadow with a blur is soft and
/// one without stays hard, and a blurred shadow with no offset is a glow on
/// all sides. The glow is light blue so it shows on the black band too.
#[view]
struct LabelSoftShadow {
    #[init]
    light:        Container,
    dark:         Container,
    thin_outline: Label,
    wide_outline: Label,
    hard_shadow:  Label,
    soft_shadow:  Label,
    glow:         Label,
}

impl Setup for LabelSoftShadow {
    fn setup(self: Weak<Self>) {
        self.light.set_frame((0, 0, 300, 520));
        self.light.set_color(WHITE);
        self.dark.set_frame((300, 0, 300, 520));
        self.dark.set_color(BLACK);

        let rows = [
            (self.thin_outline, "outline of 2 points", 30),
            (self.wide_outline, "outline of 6 points", 130),
            (self.hard_shadow, "hard shadow, no blur", 230),
            (self.soft_shadow, "soft shadow, blur 4", 330),
            (self.glow, "blue glow, blur 6", 430),
        ];
        for (label, text, top) in rows {
            label.set_frame((0, top, 600, 60));
            label.set_text(text).set_text_size(40).set_text_color(WHITE);
        }
        self.thin_outline.set_text_outline(BLACK, 2);
        self.wide_outline.set_text_outline(BLACK, 6);
        self.hard_shadow.set_text_shadow(BLACK, (3, 3));
        self.soft_shadow.set_text_shadow(BLACK, (3, 3)).set_text_shadow_blur(4);
        self.glow
            .set_text_shadow(Color::rgb(0.2, 0.6, 1.0), (0, 0))
            .set_text_shadow_blur(6);
    }
}

impl ViewTest for LabelSoftShadow {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(CHECK_1)?;
        Ok(())
    }
}
