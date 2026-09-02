use hilen::{
    gm::{
        LossyConvert,
        volume::{Shape3, Vec3},
    },
    scene::{Body, NodeTemplates, SceneCreation, SceneSetup, Wall, scene},
    ui::Color,
};

const PYRAMID_ROWS: usize = 4;

/// The 3D playground: a floor, a pyramid of boxes to knock over, and
/// the balls the page drops onto it.
#[scene]
#[derive(Default)]
pub struct DemoScene {}

impl DemoScene {
    pub fn drop_ball(&mut self) {
        let x = fastrand::f32() * 4.0 - 2.0;
        let z = fastrand::f32() * 4.0 - 2.0;
        self.make_node::<Body>(Shape3::Ball(0.5), Vec3::new(x, 9.0, z))
            .set_color(Color::random());
    }
}

impl SceneSetup for DemoScene {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.make_node::<Wall>(Shape3::Plane(30.0), Vec3::ZERO)
            .set_color(Color::hex("#95a5a6"));

        for row in 0..PYRAMID_ROWS {
            let count = PYRAMID_ROWS - row;
            for i in 0..count {
                let x = (i.lossy_convert() - (count - 1).lossy_convert() / 2.0) * 1.05;
                let y = 0.5 + row.lossy_convert() * 1.0;
                self.make_node::<Body>(Shape3::cube(1.0), Vec3::new(x, y, 0.0))
                    .set_color(Color::hex("#e67e22"));
            }
        }
    }
}
