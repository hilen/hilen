use std::ops::DerefMut;

use hilen::{
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
             228    4 - #52b155
             592    4 - #52b155
             216  136 - #41a35e
             108  200 - #ffffff
             252  200 - #ebc8ce
             348  200 - #d8aaac
              28  216 - #000000
              56  220 - #010101
              28  224 - #000000
              68  224 - #ffffff
              40  228 - #010101
              76  228 - #ffffff
              28  232 - #010101
              52  232 - #ffffff
             280  244 - #b9735f
             108  248 - #ffffff
             560  256 - #3da15d
             340  260 - #a26858
             348  272 - #cb9998
             308  276 - #c3a489
             172  300 - #51afab
             460  300 - #5cb2ac
             292  308 - #c0907c
             348  308 - #c89496
             328  340 - #ae8e79
             252  344 - #e0a8a9
             348  364 - #ab8d75
             268  396 - #e4bfb7
             332  396 - #a5836a
               4  592 - #65bae3
             208  592 - #65bae3
             592  592 - #65bae3
            ",
        )?;

        // hilen::ui_test::record_ui_test();

        Ok(())
    }
}
