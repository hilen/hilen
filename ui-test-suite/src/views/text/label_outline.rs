use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{BLACK, Color, Container, Label, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::check_colors,
};

const CHECK_1: &str = r"
    4    4 - #ffffff
    176    4 - #ffffff
    324   56 - #737373
    392   60 - #dadada
    436   60 - #000000
    368   64 - #262626
    592  128 - #000000
    376  148 - #d9d9d9
    144  156 - #9f9f9f
    344  160 - #dadada
    464  160 - #000000
    244  164 - #fefefe
    384  260 - #c1c1c1
    492  260 - #a7a7a7
    252  264 - #525252
    492  264 - #a7a7a7
    128  268 - #555555
    304  272 - #fefefe
    268  344 - #8d8d8d
    196  348 - #c2c2c2
    228  360 - #b6b6b6
    352  360 - #454545
    408  360 - #7f7f7f
    132  364 - #848484
    172  364 - #1f1f1f
    228  368 - #b6b6b6
    292  368 - #2b2b2b
    328  368 - #808080
    460  372 - #808080
    592  416 - #000000
    4  592 - #597c95
    380  592 - #597c95
";

/// White subtitle text over a white band and over a black band, 3 times:
/// plain, with a black outline, and with a black shadow. Proves the outline
/// rings every glyph and the shadow sits behind and beside it, so the text
/// reads over the white band where the plain one is lost, and that a faded
/// label fades its outline with it.
#[view]
struct LabelOutline {
    #[init]
    light:    Container,
    dark:     Container,
    plain:    Label,
    outlined: Label,
    shadowed: Label,
    faded:    Label,
}

impl Setup for LabelOutline {
    fn setup(self: Weak<Self>) {
        self.light.set_frame((0, 0, 300, 420));
        self.light.set_color(WHITE);
        self.dark.set_frame((300, 0, 300, 420));
        self.dark.set_color(BLACK);

        let rows = [
            (self.plain, "plain subtitle text", 30),
            (self.outlined, "outlined subtitle text", 130),
            (self.shadowed, "shadowed subtitle text", 230),
            (self.faded, "outlined, half faded", 330),
        ];
        for (label, text, top) in rows {
            label.set_frame((0, top, 600, 60));
            label.set_text(text).set_text_size(40).set_text_color(WHITE);
        }
        self.outlined.set_text_outline(BLACK, 2);
        self.shadowed.set_text_shadow(Color::rgb(0.0, 0.0, 0.0), (3, 3));
        self.faded.set_text_outline(BLACK, 2).set_opacity(0.5);
    }
}

impl ViewTest for LabelOutline {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(CHECK_1)?;
        Ok(())
    }
}
