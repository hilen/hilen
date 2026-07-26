use std::ops::DerefMut;

use test_engine::{
    RenderPass,
    game::{Game, GameDrawer, Object, Shape},
    refs::{Own, Weak, manage::DataManager},
    ui::{Image, Point, Setup, ViewCallbacks, ViewData, ViewTest, view},
    ui_test::check_colors,
};

use crate::interface::HAS_BACK_BUTTON;

#[view]
pub struct GameView {
    game: Own<Game>,
}

impl Setup for GameView {
    fn setup(mut self: Weak<Self>) {
        self.apply_style(HAS_BACK_BUTTON);

        self.game.skybox = Image::get("sky.png");

        self.game.objects.push(Own::new(Object {
            position: Point::default(),
            rotation: 0.0,
            texture:  Image::get("cat.png"),
            velocity: (0.1, 0.1).into(),
            shape:    Shape::Rect((5, 10).into()),
        }));
    }
}

impl ViewCallbacks for GameView {
    fn before_render(&self, pass: &mut RenderPass) {
        GameDrawer::draw(pass, self.game.weak().deref_mut());
    }
}

impl ViewTest for GameView {
    fn perform_test(_view: Weak<Self>) -> anyhow::Result<()> {
        check_colors(
            r"
             136    4 - #52b155
             380    4 - #52b155
             592    4 - #52b155
             284   84 - #379f5c
             380  116 - #369f5c
             480  124 - #67b75b
             256  200 - #eac5cc
             108  204 - #ffffff
             348  212 - #d8aaac
              36  216 - #ffffff
              76  224 - #ffffff
              32  228 - #ffffff
              52  228 - #ffffff
             300  236 - #e3b5b5
              12  248 - #ffffff
             108  248 - #ffffff
             256  248 - #eac2c3
             560  260 - #389f5c
             340  276 - #cb9998
             260  292 - #e5b7b9
             400  292 - #369f5c
             348  308 - #c89496
             284  332 - #d6b39f
             340  340 - #a58570
             340  364 - #9f8169
             348  380 - #b49882
             260  396 - #dda1a1
             328  396 - #9f7f66
             196  580 - #65bae3
               4  592 - #65bae3
             388  592 - #65bae3
             592  592 - #65bae3
            ",
        )?;

        // test_engine::ui_test::record_ui_test();

        Ok(())
    }
}
