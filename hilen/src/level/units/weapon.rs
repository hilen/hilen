use std::ops::{Deref, DerefMut};

use rapier2d::dynamics::CoefficientCombineRule;

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::{
        color::Color,
        flat::{Point, Shape},
    },
    level::{Body, LevelManager, Sprite, SpriteData, SpriteTemplates, level::LevelCreation},
    window::image::Image,
};

pub struct Weapon {
    sprite:              SpriteData,
    pub(crate) velocity: Point,
    pub bullet_speed:    f32,
    pub bullet_image:    Weak<Image>,
    pub bullet_shape:    Shape,
}

impl Weapon {
    pub fn shoot_at(self: Weak<Self>, pos: impl Into<Point>) {
        let vector = (pos.into() - self.position()).normalized();
        let pos = self.position() + vector * 4.0;

        let vel = vector * self.bullet_speed + self.velocity;

        let shape = self.bullet_shape.clone();
        let mut bullet = LevelManager::level_weak().make_sprite::<Body>(shape, pos);

        bullet.set_rotation(self.rotation());
        bullet.set_velocity(vel);
        bullet.set_restitution(0.5, CoefficientCombineRule::Average);
        bullet.set_color(Color::random());
        bullet.set_image(self.bullet_image);
    }
}

impl Sprite for Weapon {
    fn make(shape: Shape, position: Point) -> Own<Self>
    where Self: Sized {
        Own::new(Self {
            sprite:       SpriteData::make(shape, position),
            velocity:     Point::default(),
            bullet_speed: 1.0,
            bullet_image: Weak::default(),
            bullet_shape: Shape::Rect((1, 1).into()),
        })
    }

    fn weak_sprite(&self) -> Weak<dyn Sprite> {
        weak_from_ref(self)
    }
}

impl Deref for Weapon {
    type Target = SpriteData;
    fn deref(&self) -> &SpriteData {
        &self.sprite
    }
}

impl DerefMut for Weapon {
    fn deref_mut(&mut self) -> &mut SpriteData {
        &mut self.sprite
    }
}
