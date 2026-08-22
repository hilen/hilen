use crate::{
    deps::refs::{Own, Weak},
    game::object::Object,
    window::image::Image,
};

#[derive(Default)]
pub struct Game {
    pub objects: Vec<Own<Object>>,
    pub skybox:  Weak<Image>,
}

impl Game {
    pub(crate) fn update(&mut self) {
        for obj in &mut self.objects {
            obj.update();
        }
    }
}
