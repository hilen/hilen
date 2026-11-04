use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::color::Color,
    refs::Weak,
    ui::{Container, DynamicColor, Label, Setup, Theme, ThemeMode, ToolButton, ViewData, ViewTest, view},
    ui_test::{check_colors, checkpoint},
    window::image::Image,
};

const BAR: DynamicColor = DynamicColor::new(Color::hex("#F4F4F6"), Color::hex("#27272A"));
const STATUS: DynamicColor = DynamicColor::new(Color::hex("#1C1C1E"), Color::hex("#F2F2F4"));

const SIDE: u32 = 32;

/// A picture red on the left half and blue on the right, so a probe tells a
/// tinted picture from its own colors.
fn avatar() -> Weak<Image> {
    let mut pixels = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for _ in 0..SIDE {
        for x in 0..SIDE {
            let rgba = if x < SIDE / 2 {
                [0xE5, 0x39, 0x35, 0xFF]
            } else {
                [0x1E, 0x88, 0xE5, 0xFF]
            };
            pixels.extend_from_slice(&rgba);
        }
    }
    Image::from_raw_data(pixels, "tool_button_picture_avatar", (SIDE, SIDE).into(), 4)
}

/// An account button with the avatar of the user in place of its icon. The
/// picture stays round and red and blue at rest and while the button is on,
/// and the icon comes back when the picture goes.
#[view]
struct ToolButtonPictureTest {
    #[init]
    bar:     Container,
    account: ToolButton,
    status:  Label,
}

impl Setup for ToolButtonPictureTest {
    fn setup(self: Weak<Self>) {
        self.bar.set_color(BAR);
        self.bar.place().back();

        self.account.set_icon("tool_tag.svg").set_text("account button");
        self.account.set_picture(Some(avatar()));
        self.account.place().t(12).l(12);

        self.status.set_text_size(16).set_text_color(STATUS);
        self.status.place().t(72).lr(12).h(30);
        self.status.set_text("round red and blue picture, not tinted");
    }
}

impl ViewTest for ToolButtonPictureTest {
    fn canvas() -> (u32, u32) {
        (400, 120)
    }

    fn before_start() {
        from_main(|| Theme::set_mode(ThemeMode::Light));
    }

    fn perform_test(view: Weak<Self>) -> Result<()> {
        check_colors(REST)?;

        from_main(move || {
            view.account.set_on(true);
            view.status.set_text("button on, the picture keeps its colors");
        });
        wait_for_next_frame();
        check_colors(ON)?;

        from_main(move || {
            view.account.set_on(false);
            view.account.set_picture(None);
            view.status.set_text("picture gone, the tag icon is back");
        });
        wait_for_next_frame();
        check_colors(ICON_BACK)?;
        checkpoint("the tag icon is back")?;

        from_main(|| Theme::set_mode(ThemeMode::System));

        Ok(())
    }
}

/// At rest: the round red and blue picture in place of the icon.
const REST: &str = r"
             208    4 - #f4f4f6
             292    4 - #f4f4f6
             392    4 - #f4f4f6
              28   20 - #e53935
              36   20 - #1e88e5
              20   24 - #e53935
              24   28 - #e53935
              32   28 - #1e88e5
              36   28 - #1e88e5
              40   28 - #1e88e5
              60   28 - #f4f4f6
              80   28 - #9e9ea0
              96   28 - #a3a3a5
             124   28 - #777779
              20   32 - #e53935
              28   32 - #e53935
              24   36 - #e53935
              32   36 - #1e88e5
              36   36 - #1e88e5
             136   84 - #a2a2a4
             328   84 - #1c1c1e
              80   88 - #f4f4f6
             100   88 - #525254
             116   88 - #1d1d1f
             156   88 - #88888a
             176   88 - #f4f4f6
             216   88 - #777779
             232   88 - #f4f4f6
             252   88 - #616163
             268   88 - #b3b3b5
             300   88 - #1d1d1f
             316   88 - #616163
";

/// On: the blue fill of the on look around the picture, which keeps its colors.
const ON: &str = r"
             236    4 - #f4f4f6
             392    4 - #f4f4f6
             312   12 - #f4f4f6
              24   20 - #e53935
              32   20 - #1e88e5
              20   24 - #e53935
              28   24 - #e53935
              24   28 - #e53935
              32   28 - #1e88e5
              36   28 - #1e88e5
              40   28 - #1e88e5
              80   28 - #8bb9ff
              96   28 - #90bcff
             116   28 - #94beff
             124   28 - #65a2ff
             140   28 - #0a6cff
              20   32 - #e53935
              36   32 - #1e88e5
              28   36 - #e53935
              36   36 - #1e88e5
              68   84 - #1c1c1e
             148   84 - #5d5d5f
              96   88 - #1c1c1e
             132   88 - #5c5c5e
             172   88 - #f4f4f6
             192   88 - #505052
             204   88 - #dfdfe1
             228   88 - #f4f4f6
             248   88 - #1c1c1e
             268   88 - #cdcdcf
             292   88 - #f4f4f6
             320   88 - #9e9ea0
";

/// The picture is gone, the tag icon is back.
const ICON_BACK: &str = r"
             240    4 - #f4f4f6
             392    4 - #f4f4f6
             316   12 - #f4f4f6
              28   28 - #f4f4f6
              56   28 - #f4f4f6
              76   28 - #9d9da0
              92   28 - #a3a3a4
             112   28 - #a7a7a9
             120   28 - #777779
             136   28 - #1d1d1f
             360   60 - #f4f4f6
             284   84 - #1c1c1e
             100   88 - #f4f4f6
             112   88 - #b0b0b2
             120   88 - #1d1d1f
             128   88 - #616163
             144   88 - #cbcbce
             160   88 - #b2b2b5
             168   88 - #616163
             180   88 - #8c8c8d
             208   88 - #919193
             224   88 - #f4f4f6
             240   88 - #f4f4f6
             248   88 - #f4f4f6
             256   88 - #d5d5d6
             260   88 - #1c1c1e
             268   88 - #1c1c1e
             284   88 - #1c1c1e
             304   88 - #f4f4f6
              84   92 - #727273
               4  112 - #f4f4f6
             392  112 - #f4f4f6
";
