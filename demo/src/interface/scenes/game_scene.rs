use anyhow::Result;
use hilen::{
    dispatch::from_back,
    level::{Control, LevelManager},
    refs::{Weak, manage::DataManager},
    ui::{AlertErr, Button, DPadView, Image, Setup, Spinner, StickView, ViewData, ViewSubviews, view},
};

use crate::{
    interface::{
        palette::{BORDER, SURFACE, TEXT},
        scenes::add_back_button,
    },
    levels::TestLevel,
};

/// The classic physics playground: a gravity level with a player, an
/// on-screen d-pad and analog stick to drive it, and two dev controls
/// that need a live level: spawn a box and swap in a downloaded image.
#[view]
pub struct GameScene {
    level: Weak<TestLevel>,

    #[init]
    dpad:  DPadView,
    stick: StickView,
}

impl Setup for GameScene {
    fn setup(mut self: Weak<Self>) {
        self.level = LevelManager::set_level(TestLevel::default());

        self.dpad.place().bl(20).size(120, 90);
        self.dpad.on_press.val(move |direction| {
            self.level.player.unit.body.move_by_direction(direction);
        });

        self.stick.place().br(20).size(150, 150);
        self.stick.on_change.val(move |direction| {
            self.level.player.unit.body.add_impulse(direction.invert_y() / 500.0);
        });

        self.add_control("Add box", 20, move || {
            self.level.add_random_box((0, 40));
        });

        self.add_control("Load bg", 72, move || {
            from_back(load_assets_test, move |result| {
                let Some(image) = result.alert_err() else {
                    return;
                };
                self.level.background = image;
            });
        });

        add_back_button(self);
    }
}

impl GameScene {
    fn add_control(self: Weak<Self>, title: &str, top: i32, action: impl FnMut() + Send + 'static) {
        let button = self.add_view::<Button>();
        button
            .set_text(title)
            .set_color(SURFACE)
            .set_text_color(TEXT)
            .set_corner_radius(10)
            .set_border_width(1)
            .set_border_color(BORDER);
        button.on_tap(action);
        button.place().t(top).r(20).size(120, 40);
    }
}

async fn load_assets_test() -> Result<Weak<Image>> {
    let spin = Spinner::lock();

    let result = Image::download(
        "downloaded.jpg",
        "https://fastly.picsum.photos/id/299/1000/1000.jpg?hmac=DRpkgVaALpt0f0Y4kSTUOtLJ66_ULgUDZn2n6pbuafA",
    )
    .await;

    drop(spin);
    result
}
