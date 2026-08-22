use std::ops::{Deref, DerefMut};

use super::LevelInternal;
use crate::{
    deps::refs::{AsAny, Own, Weak},
    gm::{
        color::Color,
        flat::{Point, Shape},
    },
    level::{Body, LevelBase, LevelCreation, LevelManager, Sprite, SpriteTemplates},
};

pub trait Level: AsAny + Deref<Target = LevelBase> + DerefMut + LevelInternal {
    fn add_touch(&mut self, pos: Point) -> bool {
        let pos = LevelManager::convert_touch(pos);

        self.add_box(pos);

        true
    }

    fn add_box(&mut self, pos: Point) {
        let mut bx = self.make_sprite::<Body>(Shape::Rect((2, 2).into()), pos);
        bx.set_color(Color::random());
    }

    fn sprite_at(&self, point: Point) -> Option<Weak<dyn Sprite>> {
        for sprite in &self.sprites {
            if sprite.contains(point) {
                return sprite.weak().into();
            }
        }
        None
    }

    fn sprites(&self) -> &[Own<dyn Sprite>] {
        &self.sprites
    }

    fn sprites_mut(&mut self) -> &mut [Own<dyn Sprite>] {
        &mut self.sprites
    }
}
