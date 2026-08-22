use hilen::{
    gm::{Animation, Shape},
    level::{LevelCreation, LevelSetup, MovingWall, Player, SpriteTemplates, Wall, level},
    refs::{Weak, manage::DataManager},
    ui::{Alert, Color, Image, UIManager},
};

use crate::levels::make_test_terrain;

#[level]
#[derive(Default)]
pub struct BenchmarkLevel {
    low_frames: usize,

    top_wall: Weak<Wall>,

    left_wall:     Weak<MovingWall>,
    right_wall:    Weak<MovingWall>,
    floor:         Weak<MovingWall>,
    bottom_moving: Weak<MovingWall>,

    left_animation:   Animation,
    right_animation:  Animation,
    floor_animation:  Animation,
    bottom_animation: Animation,

    finish: bool,

    pub player:        Weak<Player>,
    pub bullets_count: u64,
}

impl BenchmarkLevel {
    fn make_walls(&mut self) {
        let square = Image::get("square.png");

        self.top_wall = self.make_sprite(Shape::Rect((100, 5).into()), (0, 110));
        self.top_wall.set_color(Color::random());

        self.floor = self.make_sprite(Shape::Rect((100, 6).into()), (0, 0));
        self.floor.set_image(square);

        self.left_wall = self.make_sprite(Shape::Rect((6, 50).into()), (-40, 0));
        self.left_wall.set_image(square);

        self.right_wall = self.make_sprite(Shape::Rect((6, 50).into()), (40, 0));
        self.right_wall.set_image(square);

        self.bottom_moving = self.make_sprite(Shape::rect(6, 14), (0, -75));
        self.bottom_moving.set_image(square);

        self.left_animation = Animation::new(-80.0, -20.0, 2.0);
        self.right_animation = Animation::new(80.0, 20.0, 2.0);
        self.floor_animation = Animation::new(-25.0, 0.0, 0.5);
        self.bottom_animation = Animation::new(-100.0, 100.0, 4.0);

        // Cage around whole scene
        self.make_sprite::<Wall>(Shape::rect(400, 2), (0, -85)).set_image(square);
        self.make_sprite::<Wall>(Shape::rect(2, 200), (120, 0)).set_image(square);
        self.make_sprite::<Wall>(Shape::rect(2, 200), (-120, 0)).set_image(square);

        for island in make_test_terrain() {
            self.make_sprite::<Wall>(Shape::Polygon(island), (-20, 0));
        }
    }
}

impl LevelSetup for BenchmarkLevel {
    fn setup(&mut self) {
        self.background = Image::get("sky.png");

        self.player = self.make_sprite(Shape::Rect((2, 2).into()), (0, 5));
        self.player.set_color(Color::random());

        self.player.set_image("game/frisk.png");

        self.player.weapon.set_image("ak.png");
        self.player.weapon.bullet_image = Image::get("game/bullet.png");
        self.player.weapon.bullet_speed = 100.0;
        self.player.weapon.bullet_shape = Shape::Rect((1, 0.28).into());

        self.make_walls();
    }

    fn update(&mut self) {
        self.left_wall.move_x(self.left_animation.value());
        self.right_wall.move_x(self.right_animation.value());
        self.floor.move_y(self.floor_animation.value());
        self.bottom_moving.move_x(self.bottom_animation.value());

        if self.finish {
            return;
        }

        if UIManager::fps() < 40.0 {
            self.low_frames += 1;
        }

        if self.low_frames >= 120 {
            self.finish = true;
            Alert::show(format!("{} sprites", self.bullets_count));
        }

        self.player.weapon.weak().shoot_at((0, 15));
        self.player.weapon.weak().shoot_at((10, 15));
        self.player.weapon.weak().shoot_at((15, 10));
        self.player.weapon.weak().shoot_at((-10, 15));
        self.player.weapon.weak().shoot_at((-15, 10));
        self.bullets_count += 5;
    }

    fn needs_physics(&self) -> bool {
        true
    }
}
