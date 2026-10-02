use anyhow::{Result, ensure};
use hilen::{
    refs::Weak,
    ui::{Color, Label, RED, Setup, U8Color, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{capture_screenshot, check_colors, checkpoint},
    window::Screenshot,
};

/// Points of the outline and of the blur, both far past the 12 pixels the
/// effect shader used to cut a spread at.
const OUTLINE: u16 = 20;
const BLUR: u16 = 30;

/// The rows the 2 labels are scanned on, through the stems of their text.
const OUTLINE_ROW: f32 = 150.0;
const BLUR_ROW: f32 = 420.0;

/// One label with an outline of 20 points and one with a shadow blurred by
/// 30. Proves a spread is not cut at 12 pixels any more: the outline is 20
/// pixels wide in front of the first letter, and the blur reaches further
/// than 12.
#[view]
struct LabelWideEffect {
    #[init]
    outlined: Label,
    glowing:  Label,
}

impl Setup for LabelWideEffect {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::hex("#2060c0"));

        self.outlined.set_frame((0, 60, 600, 180));
        self.outlined
            .set_text("HI wide")
            .set_text_size(110)
            .set_text_color(WHITE)
            .set_text_outline(RED, f32::from(OUTLINE));

        self.glowing.set_frame((0, 330, 600, 180));
        self.glowing
            .set_text("HI soft")
            .set_text_size(110)
            .set_text_color(WHITE)
            .set_text_shadow(Color::rgb(0.0, 0.0, 0.0), (0, 0))
            .set_text_shadow_blur(f32::from(BLUR));
    }
}

fn far(a: U8Color, b: U8Color) -> bool {
    [(a.r, b.r), (a.g, b.g), (a.b, b.b)].into_iter().any(|(a, b)| a.abs_diff(b) > 2)
}

/// On row `y`, the first pixel that is not the background and the first
/// pixel of the white text.
fn edges(shot: &Screenshot, y: f32) -> Result<(u16, u16)> {
    let back = shot.get_pixel((2.0, 2.0));
    let white = U8Color::rgba(255, 255, 255, 255);
    let mut effect = None;
    for x in 0..600_u16 {
        let pixel = shot.get_pixel((f32::from(x), y));
        if effect.is_none() && far(pixel, back) {
            effect = Some(x);
        }
        if !far(pixel, white) {
            let effect = effect.unwrap_or(x);
            return Ok((effect, x));
        }
    }
    anyhow::bail!("no text on row {y}")
}

impl ViewTest for LabelWideEffect {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        checkpoint("an outline of 20 points and a shadow blurred by 30, none cut at 12")?;

        let shot = capture_screenshot()?;

        let (ring, text) = edges(&shot, OUTLINE_ROW)?;
        let width = text - ring;
        ensure!(
            (OUTLINE - 1..=OUTLINE + 1).contains(&width),
            "the outline is {width} pixels wide in front of the text, asked for {OUTLINE}"
        );
        // Red all the way, not a ring with a hole.
        let red = U8Color::rgba(255, 0, 0, 255);
        for x in ring + 2..text - 2 {
            let pixel = shot.get_pixel((f32::from(x), OUTLINE_ROW));
            ensure!(!far(pixel, red), "the outline has a gap at {x}, {pixel:?}");
        }

        let (glow, text) = edges(&shot, BLUR_ROW)?;
        let reach = text - glow;
        ensure!(
            (16..=BLUR + 1).contains(&reach),
            "the blur shows {reach} pixels in front of the text, a blur of {BLUR} reaches past 12"
        );

        check_colors(CHECK)
    }
}

/// The recorded look of both labels.
const CHECK: &str = r"
     436  108 - #a5274e
     156  112 - #d91020
     344  112 - #ffffff
     156  116 - #d91020
     156  120 - #d91020
     252  124 - #ff0000
     176  140 - #ffd4d4
     416  144 - #ff9797
     368  160 - #ffffff
     476  160 - #ffffff
     156  172 - #d91020
     156  180 - #d91020
     176  184 - #ffd4d4
     416  184 - #ff9797
     300  204 - #ff0000
     592  264 - #2060c0
     144  384 - #a8bcda
     360  396 - #194c97
     412  396 - #e6eaf0
     416  396 - #133972
     420  396 - #133972
     460  404 - #a4b8d5
     288  408 - #fefeff
     204  416 - #8498b6
     300  416 - #143d79
     316  416 - #9aacc9
     144  420 - #a6b7cf
     204  452 - #87a1c8
     460  452 - #d9e2f0
     148  456 - #ffffff
     376  456 - #1a4d9a
       4  592 - #2060c0
";
