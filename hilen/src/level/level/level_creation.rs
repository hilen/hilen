use crate::{
    deps::refs::{Own, Weak},
    gm::flat::{Point, Rect, Shape},
    level::{Banner, Level, LevelManager, Sprite},
};

pub trait LevelCreation {
    fn add_sprite<S: 'static + Sprite>(&mut self, sprite: Own<S>) -> Weak<S>;
    fn make_sprite<S: 'static + Sprite>(&mut self, _: Shape, _: impl Into<Point>) -> Weak<S>;
    fn make_rect(&mut self, rect: impl Into<Rect>) -> Weak<Banner>;
}

impl<T: ?Sized + Level> LevelCreation for T {
    fn add_sprite<S: 'static + Sprite>(&mut self, mut sprite: Own<S>) -> Weak<S> {
        let weak = sprite.weak();

        self.last_z_pos -= LevelManager::z_position_offset();
        sprite.z_position = self.last_z_pos;
        self.sprites.push(sprite);
        weak
    }

    fn make_sprite<S: 'static + Sprite>(&mut self, shape: Shape, position: impl Into<Point>) -> Weak<S> {
        self.add_sprite(S::make(shape, position.into()))
    }

    fn make_rect(&mut self, rect: impl Into<Rect>) -> Weak<Banner> {
        let rect = rect.into();
        self.make_sprite::<Banner>(Shape::Rect(rect.size), rect.origin)
    }
}
