use anyhow::{Result, ensure};
use hilen::{
    refs::{Weak, manage::DataManager},
    ui::{
        BLACK, Color, Font, Label, Setup, TextAlignment, U8Color, VerticalAlignment, ViewData, ViewFrame,
        ViewTest, WHITE, view,
    },
    ui_test::{capture_screenshot, check_colors, checkpoint, set_record_probe_count},
    window::Screenshot,
};

const LINES: &str = "line 1 🚀\nline 2 🎉\nline 3 🔥";
const WIDE: &str = "wide 🚀🎉🔥🚀🎉🔥";
const WIDE_RIGHT: &str = "🚀🎉🔥🚀🎉🔥 wide";

const TEXT_SIZE: f32 = 32.0;

const FITS: (f32, f32, f32, f32) = (20.0, 34.0, 560.0, 50.0);
/// 1 line and a half high, the text has 3 lines.
const CUT_BOTTOM: (f32, f32, f32, f32) = (20.0, 190.0, 250.0, 56.0);
const CUT_BOTH: (f32, f32, f32, f32) = (330.0, 190.0, 250.0, 56.0);
/// Narrower than their text, the cut goes through an emoji.
const CUT_RIGHT: (f32, f32, f32, f32) = (20.0, 400.0, 200.0, 50.0);
const CUT_LEFT: (f32, f32, f32, f32) = (380.0, 400.0, 200.0, 50.0);

/// A strip of pixels as x, y, width, height.
type Strip = (u32, u32, u32, u32);

/// The room around a cut frame that has to stay white: the frame it
/// belongs to and the strip.
const WHITE_STRIPS: [(&str, Strip); 5] = [
    ("under the frame cut at the bottom", (20, 247, 250, 90)),
    ("over the frame cut at both edges", (330, 150, 250, 39)),
    ("under the frame cut at both edges", (330, 247, 250, 90)),
    ("right of the frame cut at the right", (221, 400, 78, 50)),
    ("left of the frame cut at the left", (301, 400, 78, 50)),
];

/// Labels with emoji of a color fallback font in frames too small for
/// their text. The tinted box is the frame of the label. The text is cut
/// at its edge, and an emoji has to be cut at the same edge: none may
/// show on the white around a frame.
#[view]
struct ColorGlyphClip {
    #[init]
    fits_caption:       Label,
    fits:               Label,
    cut_bottom_caption: Label,
    cut_bottom:         Label,
    cut_both_caption:   Label,
    cut_both:           Label,
    cut_right_caption:  Label,
    cut_right:          Label,
    cut_left_caption:   Label,
    cut_left:           Label,
}

impl ColorGlyphClip {
    fn caption(label: Weak<Label>, text: &str, x: f32, y: f32, width: f32) {
        label
            .set_multiline(true)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);
        label.set_text(text);
        label.set_text_color(BLACK);
        label.set_frame((x, y, width, 34.0));
    }

    fn framed(label: Weak<Label>, text: &str, frame: (f32, f32, f32, f32)) -> Weak<Label> {
        label.set_text_size(TEXT_SIZE);
        label.set_text(text);
        label.set_text_color(BLACK);
        label.set_color(Color::hex("#e8f0fe"));
        label.set_frame(frame);
        label
    }
}

impl Setup for ColorGlyphClip {
    fn setup(self: Weak<Self>) {
        self.set_color(WHITE);
        Font::set_fallbacks([Font::get("TwemojiColr0.ttf")]);

        Self::caption(self.fits_caption, "fits, nothing is cut", 20.0, 10.0, 560.0);
        Self::framed(self.fits, "fits 🚀 🎉 🔥", FITS).set_alignment(TextAlignment::Left);

        Self::caption(
            self.cut_bottom_caption,
            "cut at the bottom\nline 2 is cut in half, line 3 is not drawn",
            20.0,
            100.0,
            270.0,
        );
        Self::framed(self.cut_bottom, LINES, CUT_BOTTOM)
            .set_multiline(true)
            .set_alignment(TextAlignment::Left)
            .set_vertical_alignment(VerticalAlignment::Top);

        Self::caption(
            self.cut_both_caption,
            "cut at the top and the bottom\nline 2 is whole, lines 1 and 3 are cut",
            330.0,
            100.0,
            270.0,
        );
        Self::framed(self.cut_both, LINES, CUT_BOTH).set_multiline(true);

        Self::caption(
            self.cut_right_caption,
            "cut at the right\nthe cut goes through an emoji",
            20.0,
            350.0,
            270.0,
        );
        Self::framed(self.cut_right, WIDE, CUT_RIGHT).set_alignment(TextAlignment::Left);

        Self::caption(
            self.cut_left_caption,
            "cut at the left\nthe cut goes through an emoji",
            380.0,
            350.0,
            210.0,
        );
        Self::framed(self.cut_left, WIDE_RIGHT, CUT_LEFT).set_alignment(TextAlignment::Right);
    }
}

/// The first pixel of a strip that is not white.
fn stain(shot: &Screenshot, (x, y, width, height): Strip) -> Option<(u32, u32, U8Color)> {
    (y..y + height)
        .flat_map(|row| (x..x + width).map(move |column| (column, row)))
        .map(|(column, row)| (column, row, shot.get_pixel((column, row))))
        .find(|(_, _, color)| color.r != 255 || color.g != 255 || color.b != 255)
}

impl ViewTest for ColorGlyphClip {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        set_record_probe_count(120);
        check_colors(PROBES)?;
        checkpoint("every emoji ends at the edge of its tinted frame, none shows on the white")?;

        let shot = capture_screenshot()?;
        for (name, strip) in WHITE_STRIPS {
            let stain = stain(&shot, strip);
            ensure!(
                stain.is_none(),
                "{name} the white has a drawn pixel, x, y and color: {stain:?}"
            );
        }

        Ok(())
    }
}

const PROBES: &str = r"
     592    4 - #ffffff
      60   20 - #cbcbcb
      80   20 - #222222
     112   52 - #55acee
     180   52 - #f4900c
      48   56 - #494b50
     100   56 - #55acee
     140   56 - #ea596e
     156   56 - #aa8dd8
      48   60 - #494b50
      56   60 - #d9e1ee
     144   60 - #dd2e44
     184   60 - #fcbd3d
     196   60 - #f4900c
      48   64 - #494b50
      56   64 - #d9e1ee
     104   64 - #55acee
     132   68 - #ea596e
     140   68 - #dd2e44
     176   68 - #ffcc4d
     188   68 - #ffcc4d
     456  104 - #3b3b3b
      40  108 - #ffffff
      52  108 - #838383
     108  108 - #ffffff
     384  108 - #838383
     404  108 - #d0d0d0
     492  108 - #373737
     148  120 - #d4d4d4
     468  120 - #939393
      40  124 - #313131
      48  124 - #676767
      72  124 - #a1a1a1
      96  124 - #9f9f9f
     124  124 - #676767
     204  124 - #ffffff
     248  124 - #b7b7b7
     348  124 - #d4d4d4
     408  124 - #1e1e1e
     424  124 - #cecece
     440  124 - #989898
     468  124 - #939393
     480  124 - #959595
     536  124 - #ffffff
     484  192 - #ffac33
     148  196 - #55acee
      40  200 - #000000
     128  204 - #a0041e
     400  208 - #000000
      56  212 - #484a4f
      76  212 - #dae2ef
      80  212 - #dae2ef
      56  216 - #484a4f
      64  216 - #2d2f32
     104  216 - #010101
     136  216 - #a0041e
     488  216 - #dd2e44
     432  220 - #010101
     496  220 - #dd2e44
     460  228 - #18181a
     468  228 - #18181a
     484  228 - #ea596e
     144  232 - #ffcc4d
     132  236 - #a0041e
     136  244 - #a0041e
     400  244 - #000000
     500  244 - #f4900c
     272  272 - #ffffff
      48  360 - #000000
      80  360 - #ffffff
     440  360 - #ffffff
     112  372 - #030303
     184  372 - #000000
     476  372 - #000000
     548  372 - #000000
     380  412 - #f4900c
     420  412 - #55acee
     476  412 - #f4900c
      80  416 - #9a9fa8
     132  416 - #55acee
     188  416 - #f4900c
     412  416 - #55acee
     436  416 - #a0041e
     544  416 - #37383c
     124  420 - #55acee
     400  420 - #a0041e
     524  420 - #010101
     544  420 - #37383c
     120  424 - #55acee
     152  424 - #ea596e
     176  424 - #f4900c
     196  424 - #f4900c
     216  424 - #55acee
     436  424 - #ea596e
     460  424 - #f4900c
     544  424 - #37383c
      96  428 - #acb2bc
     128  428 - #55acee
     160  428 - #dd2e44
     204  428 - #f4900c
     384  428 - #f4900c
     412  428 - #55acee
     440  428 - #dd2e44
     484  428 - #f4900c
     560  428 - #acb2bc
      64  432 - #000000
     148  432 - #ea596e
     472  432 - #ffcc4d
     544  432 - #37383c
     116  436 - #ffac33
     184  436 - #ffcc4d
     196  436 - #ffcc4d
     212  436 - #ffac33
     400  436 - #ffac33
     432  436 - #dd2e44
     480  436 - #ffcc4d
     148  580 - #ffffff
       4  592 - #ffffff
     292  592 - #ffffff
     524  592 - #ffffff
";
