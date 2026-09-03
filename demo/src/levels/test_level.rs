use hilen::{
    audio::Sound,
    generate::noise::{TerrainParams, generate_terrain},
    gm::{LossyConvert, Shape},
    level::{
        Body, Level, LevelCreation, LevelManager, LevelSetup, Player, Sprite, SpriteTemplates, Wall, level,
    },
    refs::{Weak, manage::DataManager},
    ui::{BLUE, Color, Image, Point, Size},
};

#[level]
#[derive(Default)]
pub struct TestLevel {
    pub player:      Weak<Player>,
    selected_sprite: Option<Weak<dyn Sprite>>,
    collision_sound: Weak<Sound>,
}

impl TestLevel {
    pub fn add_random_box(&mut self, pos: impl Into<Point>) {
        let w: u32 = fastrand::u32(2..20);
        let h: u32 = fastrand::u32(2..20);

        let mut bx = self.make_sprite::<Body>(
            Shape::Rect(Size::<f32>::new(
                w.lossy_convert() / 10.0,
                h.lossy_convert() / 10.0,
            )),
            pos,
        );

        if fastrand::bool() && fastrand::bool() {
            bx.set_color(Color::random());
        } else {
            bx.set_image("crate_box.png");
        }
    }

    fn on_touch(&mut self, pos: Point) {
        if let Some(mut sprite) = self.sprite_at(pos) {
            sprite.set_selected(true);
            self.on_sprite_selected.trigger(sprite);
            if let Some(mut old) = self.selected_sprite {
                old.set_selected(false);
            }
            self.selected_sprite = sprite.into();
            return;
        }

        if let Some(mut sprite) = self.selected_sprite {
            sprite.set_selected(false);
            self.selected_sprite = None;
            self.on_sprite_selected.trigger(Weak::default());
        }
    }

    fn add_player(&mut self) {
        let mut player: Weak<Player> = self.make_sprite(Shape::Rect((1.2, 2).into()), (0, 0));
        self.player = player;
        player.set_image("game/frisk.png").unit.enable_collision_detection();
        player.weapon.set_image("ak.png");

        player.on_collision.sub(move || {
            LevelManager::level_weak()
                .as_any_mut()
                .downcast_mut::<Self>()
                .unwrap()
                .collision_sound
                .play();
        });

        self.make_sprite::<Wall>(Shape::Rect((10, 1).into()), (-50, 55));

        self.collision_sound = Sound::get("pek.wav");
    }
}

impl LevelSetup for TestLevel {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.background = Image::get("sky.png");

        self.make_sprite::<Wall>(Shape::Rect((10, 10).into()), (15, 3))
            .set_image("game/board.png");
        self.make_sprite::<Wall>(Shape::Rect((257.0 * 0.04, 216.0 * 0.04).into()), (-15, 2))
            .set_image("shop.png");

        self.make_sprite::<Wall>(Shape::Rect((349.0 * 0.1, 32.0 * 0.1).into()), (0, -3))
            .set_image("game/stone_floor.png");

        self.make_sprite::<Wall>(Shape::Rect((349.0 * 0.1, 32.0 * 0.1).into()), (-3, -3))
            .set_image("game/stone_floor.png");

        self.make_sprite::<Wall>(Shape::Rect((349.0 * 0.1, 32.0 * 0.1).into()), (3, -3))
            .set_image("game/stone_floor.png");

        self.make_sprite::<Body>(Shape::triangle((-2, -2), (2, -2), (-2, 2)), (0, 50))
            .set_image("game/triangle.png");

        self.make_sprite::<Body>(Shape::triangle((-2, -2), (2, -2), (-2, 2)), (-20, 80))
            .set_color(BLUE);

        let boxes = 40;

        for i in 0..boxes {
            self.add_random_box((i.lossy_convert() * 0.2f32, i + 10));
        }

        self.add_player();

        self.on_tap.val(move |pos| {
            LevelManager::level_weak()
                .as_any_mut()
                .downcast_mut::<Self>()
                .unwrap()
                .on_touch(pos);
        });
    }

    fn update(&mut self, _: f32) {
        let pos = self.player.position();
        *LevelManager::camera_pos() = pos;
    }
}

pub fn make_test_terrain() -> Vec<Vec<Point>> {
    generate_terrain(TerrainParams::default()).islands
}
