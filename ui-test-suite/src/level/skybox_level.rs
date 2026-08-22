use hilen::{
    level::{LevelSetup, level},
    refs::manage::DataManager,
    ui::Image,
};

#[level]
#[derive(Default)]
pub struct SkyboxLevel {}

impl LevelSetup for SkyboxLevel {
    fn setup(&mut self) {
        self.background = Image::get("sky.png");
    }
}
