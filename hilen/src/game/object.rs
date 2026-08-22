use crate::{
    deps::refs::Weak,
    game::{Point, Rotation, Shape},
    window::image::Image,
};

pub struct Object {
    pub position: Point,
    pub velocity: Point,

    pub shape:    Shape,
    pub rotation: Rotation,

    pub texture: Weak<Image>,
}

impl Object {
    pub(crate) fn update(&mut self) {
        self.position += self.velocity;
    }
}
