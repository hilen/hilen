use std::ops::{Deref, DerefMut};

use super::LevelInternal;
use crate::{
    deps::refs::{AsAny, Own, Weak},
    gm::flat::Point,
    level::{LevelBase, LevelManager, Sprite},
};

pub trait Level: AsAny + Deref<Target = LevelBase> + DerefMut + LevelInternal {
    /// A touch that no view took. Fires `on_tap` with the level position.
    fn add_touch(&mut self, pos: Point) -> bool {
        let pos = LevelManager::convert_touch(pos);
        self.on_tap.trigger(pos);
        true
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
