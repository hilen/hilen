use anyhow::Result;
use hilen::{
    gm::Shape,
    level::{Banner, LevelCreation, LevelSetup, LevelTest, SpriteTemplates, level},
    refs::Weak,
    ui_test::{capture_screenshot, check_colors},
};

/// A 34 by 56 pixel cutout drawn above and below its size, plus a
/// rotated copy, so magnified, minified and slanted edges are on screen
/// at once.
#[level]
#[derive(Default)]
struct SpriteCutout {}

impl LevelSetup for SpriteCutout {
    fn setup(&mut self) {
        self.make_sprite::<Banner>(Shape::Rect((6, 10).into()), (-4, 0))
            .set_image("game/frisk.png");
        self.make_sprite::<Banner>(Shape::Rect((0.6, 1).into()), (4, 0))
            .set_image("game/frisk.png");
        self.make_sprite::<Banner>(Shape::Rect((6, 10).into()), (4, -6))
            .set_image("game/frisk.png")
            .set_rotation(0.4);
    }
}

/// The sprite edges after the alpha sharpening in `sprite_textured.wgsl`.
/// Bilinear sampling alone smears a magnified cutout edge over as many
/// pixels as the magnification, these probes sit where that smear used
/// to be.
impl LevelTest for SpriteCutout {
    fn perform_test(_level: Weak<Self>) -> Result<()> {
        capture_screenshot()?;
        check_colors(
            r"
               4    4 - #597c95
             592    4 - #597c95
             272  272 - #8e5f0e
             252  280 - #ffc90e
             232  288 - #3d120e
             284  292 - #b28d0a
             288  292 - #3d120e
             268  300 - #673a0e
             272  300 - #673a0e
             276  300 - #673a0e
             280  300 - #673a0e
             260  316 - #67a4e0
             352  316 - #3d120e
             268  320 - #e607f8
             308  320 - #3d120e
             264  324 - #673a0e
             268  328 - #c28f0e
             252  332 - #67a4e0
             264  332 - #77490e
             336  332 - #ffc90e
             256  336 - #67a4e0
             352  336 - #ffc90e
             328  344 - #ffc90e
             276  348 - #3d120e
             340  352 - #ffc90e
             352  372 - #67a4e0
             336  380 - #3d120e
             356  384 - #ffc90e
             376  388 - #3d120e
             348  392 - #67a4e0
             360  392 - #67a4e0
               4  592 - #597c95
        ",
        )
    }
}
