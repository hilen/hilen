use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::{
        LossyConvert,
        volume::{Quat, Shape3, Vec3},
    },
    refs::Weak,
    scene::{Camera, Light, Material, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::{Color, Image, Size},
    ui_test::{check_colors, set_record_probe_count},
};

const SIDE: u32 = 64;
const SQUARES: u32 = 4;

/// A checker of orange and blue squares, `SQUARES` a side.
fn checker() -> Weak<Image> {
    let mut data = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    let cell = SIDE / SQUARES;
    for y in 0..SIDE {
        for x in 0..SIDE {
            let odd = (x / cell + y / cell) % 2 == 1;
            data.extend_from_slice(if odd {
                &[230, 140, 40, 255]
            } else {
                &[40, 90, 200, 255]
            });
        }
    }
    Image::from_raw_data(data, "triplanar-test-checker", Size::new(SIDE, SIDE), 4)
}

/// Ridges along y, their slope in red, the glTF convention with green up.
fn ridges() -> Weak<Image> {
    let mut data = Vec::with_capacity((SIDE * SIDE * 4) as usize);
    for _ in 0..SIDE {
        for x in 0..SIDE {
            let share: f32 = x.lossy_convert() / SIDE.lossy_convert();
            let slope = (share * std::f32::consts::TAU * 2.0).cos() * 0.7;
            let red: u8 = ((slope * 0.5 + 0.5) * 255.0).round().lossy_convert();
            let up: u8 = (((1.0 - slope * slope).sqrt() * 0.5 + 0.5) * 255.0).round().lossy_convert();
            data.extend_from_slice(&[red, 128, up, 255]);
        }
    }
    Image::from_raw_data(data, "triplanar-test-ridges", Size::new(SIDE, SIDE), 4)
}

/// A box turned on 2 axes and a ball, both with no use of their uv: the
/// checker and the ridges are read from 3 sides of each node's own space.
/// Every face of the box shows whole squares that follow its edges, so the
/// texture turns with the node, and the ball shows the same squares on
/// its front, top and side, blended where 2 sides meet. With a uv read the
/// box shows 1 square stretched over each face and the ball long strips.
#[scene]
#[derive(Default)]
struct Triplanar {}

impl SceneSetup for Triplanar {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 1.2, 5.0),
            target: Vec3::ZERO,
            ..Camera::default()
        };
        self.sun.direction = Vec3::new(-0.4, -1.0, -0.6).normalize();
        self.ambient = Color::rgb(0.35, 0.35, 0.4);
        self.lights
            .push(Light::point(Vec3::new(1.5, 1.5, 2.0)).intensity(1.5).range(8.0));
        let material = Material {
            color: Color::rgb(1.0, 1.0, 1.0),
            roughness: 0.6,
            texture: Some(checker()),
            normal_map: Some(ridges()),
            triplanar: 1.0,
            ..Material::default()
        };
        self.make_node::<Prop>(Shape3::cuboid(1.0, 1.0, 1.0), Vec3::new(-1.1, 0.0, 0.0))
            .set_material(material)
            .set_rotation(Quat::from_rotation_y(0.6) * Quat::from_rotation_x(0.4));
        self.make_node::<Prop>(Shape3::Ball(0.6), Vec3::new(1.1, 0.0, 0.0))
            .set_material(material);
    }
}

impl SceneTest for Triplanar {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(SHAPES)
    }
}

// Whole squares on every face of the box and on the front of the ball.
const SHAPES: &str = r"
     592    4 - #597c95
     168  232 - #d88531
     188  232 - #325bc4
     212  232 - #dd8934
     428  244 - #3561cf
     124  248 - #14214e
     232  248 - #2f58bf
     396  248 - #b6702b
     448  256 - #496ac7
     420  260 - #7d81aa
     392  264 - #625e78
     140  268 - #523116
     412  272 - #b1805c
     424  272 - #857784
     436  276 - #7184c8
     448  276 - #586db4
     460  276 - #7b738a
     264  280 - #84501f
     356  280 - #503017
     376  280 - #6d6477
     184  284 - #c6792d
     396  284 - #5b79dd
     368  288 - #41528b
     440  288 - #ee994e
     472  288 - #2b499b
     388  292 - #d89967
     380  304 - #8d6850
     132  308 - #623b19
     372  308 - #4b4657
     380  308 - #88634a
     364  312 - #7b512d
     428  312 - #9f6124
     360  316 - #1b3273
     388  316 - #46568e
     464  316 - #7d6156
     380  320 - #3d4264
     464  320 - #705a56
     252  324 - #824f1e
     388  328 - #5b4a4a
     188  332 - #1e377e
     136  336 - #2f2830
     368  340 - #523116
     448  344 - #62442f
     240  352 - #1d377f
     400  352 - #2e2831
     428  356 - #20243f
     152  368 - #122051
     592  592 - #597c95
";
