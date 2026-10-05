use anyhow::{Result, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{Color, Image, ImageView, Label, Setup, ViewData, ViewFrame, ViewTest, WHITE, view},
    ui_test::{capture_screenshot, check_colors, checkpoint, step_frames},
};

/// More than the 120 frames an effect image is kept after its last draw.
const GONE_FRAMES: u32 = 130;

/// The rows of the screen the 3 texts and their effects draw in.
const TEXT_ROWS: (u32, u32) = (20, 380);

fn striped(index: u8) -> Weak<Image> {
    let side = 24 + u32::from(index) * 20;
    let mut pixels = Vec::new();
    for y in 0..side {
        for x in 0..side {
            let band = u8::from((x + y) / 6 % 2 == 0) * 255;
            pixels.extend_from_slice(&[band, 255 - band, index * 40, 255]);
        }
    }
    Image::from_raw_data(
        pixels,
        format!("effect made again picture {index}"),
        (side, side).into(),
        4,
    )
}

const CHECK_1: &str = r"
      84   56 - #e64d00
     276   60 - #e64d00
     376   64 - #ffffff
     176   72 - #fad9c8
     400   72 - #9d3a05
     516   72 - #261a0d
     128   88 - #261a0d
     328   96 - #e64d00
     412   96 - #f4ad8a
     264  108 - #ee8550
     208  168 - #6d5419
     304  188 - #957521
     420  192 - #523e15
     240  220 - #6a5219
     304  292 - #fefefe
     420  312 - #cfdeeb
     308  316 - #75abd5
     248  320 - #6dacdd
     252  320 - #215176
     324  320 - #dbe8f3
     380  320 - #7c7c79
     176  324 - #97928d
     224  324 - #1c83d5
     280  328 - #403c37
     288  328 - #1d76bc
     424  332 - #1d79c2
     328  336 - #205e8f
     196  344 - #224661
     392  344 - #233a4b
     248  536 - #ffffff
     308  540 - #4b4237
       4  592 - #261a0d
";

const CHECK_2: &str = r"
     340    4 - #261a0d
      84   56 - #e64d00
     232   56 - #fffefe
     516   72 - #261a0d
     404  100 - #e64d00
     208  168 - #6d5419
     304  188 - #957521
     420  204 - #584316
     240  220 - #6a5219
     304  292 - #fefefe
     420  312 - #cfdeeb
     248  320 - #6dacdd
     252  320 - #215176
     224  324 - #1c83d5
     424  332 - #1d79c2
     348  340 - #1f6499
     196  344 - #224661
     388  344 - #1e73b7
     100  400 - #00ff00
     388  400 - #00ff78
     316  404 - #ff0078
      24  408 - #ff0000
     192  444 - #ff0028
     264  460 - #ff0050
     100  476 - #00ff00
     384  476 - #00ff78
     484  480 - #0ef1a0
     156  540 - #776f67
     280  540 - #4b4237
     464  540 - #e4e2e1
     148  544 - #c9c6c3
     224  544 - #a6a19b
";

/// 3 texts on a dark screen, one with an outline and 2 with a glow, and a
/// row of 6 empty picture boxes under them. The texts are hidden until
/// their effect images are freed, the boxes get pictures the process has
/// never made, then the texts are shown again. Proves the effects then
/// look pixel for pixel like before. A text once came back with a piece of
/// another picture or a dark copy of itself in place of its effect.
#[view]
struct LabelEffectMadeAgain {
    #[init]
    outlined: Label,
    glowing:  Label,
    shadowed: Label,
    first:    ImageView,
    second:   ImageView,
    third:    ImageView,
    fourth:   ImageView,
    fifth:    ImageView,
    sixth:    ImageView,
    note:     Label,
}

impl LabelEffectMadeAgain {
    fn texts(&self) -> [Weak<Label>; 3] {
        [self.outlined, self.glowing, self.shadowed]
    }

    fn pictures(&self) -> [Weak<ImageView>; 6] {
        [
            self.first,
            self.second,
            self.third,
            self.fourth,
            self.fifth,
            self.sixth,
        ]
    }
}

impl Setup for LabelEffectMadeAgain {
    fn setup(self: Weak<Self>) {
        self.set_color(Color::rgb(0.15, 0.1, 0.05));

        self.outlined.set_frame((20, 20, 560, 110));
        self.outlined.set_text("Reset progress").set_text_size(70).set_text_color(WHITE);
        self.outlined.set_text_outline(Color::rgb(0.9, 0.3, 0.0), 4);

        self.glowing.set_frame((20, 140, 560, 110));
        self.glowing.set_text("Custom").set_text_size(70).set_text_color(WHITE);
        self.glowing
            .set_text_shadow(Color::rgb(1.0, 0.8, 0.2), (0, 0))
            .set_text_shadow_blur(10);

        self.shadowed.set_frame((20, 260, 560, 110));
        self.shadowed.set_text("Settings").set_text_size(70).set_text_color(WHITE);
        self.shadowed
            .set_text_shadow(Color::rgb(0.1, 0.6, 1.0), (4, 4))
            .set_text_shadow_blur(6);

        for (index, picture) in self.pictures().iter().enumerate() {
            picture.set_frame((20 + index * 95, 400, 85, 85));
        }

        self.note.set_frame((20, 510, 560, 60));
        self.note.set_text("the first look").set_text_size(22).set_text_color(WHITE);
    }
}

impl ViewTest for LabelEffectMadeAgain {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(CHECK_1)?;
        let first = capture_screenshot()?;

        from_main(move || {
            for text in view.texts() {
                text.set_hidden(true);
            }
            view.note.set_text("texts hidden, their effect images are freed");
        });
        step_frames(GONE_FRAMES);

        from_main(move || {
            for (index, picture) in (0_u8..).zip(view.pictures()) {
                picture.set_image(striped(index));
            }
        });
        step_frames(2);
        checkpoint("the texts are gone, the boxes have new pictures")?;

        from_main(move || {
            for text in view.texts() {
                text.set_hidden(false);
            }
        });
        step_frames(2);
        let again = capture_screenshot()?;

        let mut wrong = 0;
        let mut first_wrong = None;
        for y in TEXT_ROWS.0..TEXT_ROWS.1 {
            for x in 0..600 {
                if first.get_pixel((x, y)) != again.get_pixel((x, y)) {
                    wrong += 1;
                    first_wrong.get_or_insert((x, y));
                }
            }
        }
        ensure!(
            wrong == 0,
            "{wrong} pixels of the texts differ after their effects were made again, the first at \
             {first_wrong:?}"
        );

        from_main(move || {
            view.note.set_text("the texts must look like at the start");
        });
        check_colors(CHECK_2)?;
        Ok(())
    }
}
