use std::ops::{Deref, DerefMut};

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::flat::{Point, Shape},
    level::{Sprite, SpriteData},
};

pub struct Banner {
    sprite: SpriteData,
}

impl Deref for Banner {
    type Target = SpriteData;

    fn deref(&self) -> &Self::Target {
        &self.sprite
    }
}

impl DerefMut for Banner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.sprite
    }
}

impl Sprite for Banner {
    fn make(shape: Shape, position: Point) -> Own<Self>
    where Self: Sized {
        Own::new(Self {
            sprite: SpriteData::make(shape, position),
        })
    }

    fn weak_sprite(&self) -> Weak<dyn Sprite> {
        weak_from_ref(self)
    }
}
