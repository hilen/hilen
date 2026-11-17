use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, Sun, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

/// The plates are this big and this far, the size of a small moon.
const PLATE: f32 = 30_000.0;
const DISTANCE: f32 = 300_000.0;
/// How far the red plate stands behind the grey one.
const BEHIND: f32 = 50.0;

/// A camera with no far plane sees a post 1 m from its eye, a floor, and 2
/// plates 30 km wide 300 km away, facing it. The red plate stands 50 m
/// behind the grey one and half behind it. The depth is reversed, so the
/// grey plate covers the red one cleanly where they overlap: in the old
/// depth band a step there was about 140 km, and the 2 mixed in stripes.
#[scene]
#[derive(Default)]
struct NearAndFar {}

impl SceneSetup for NearAndFar {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 1.7, 0.0),
            target: Vec3::new(0.0, 1.7 + PLATE * 0.5 / DISTANCE * 10.0, -10.0),
            far: f32::INFINITY,
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#0b1530"),
            Color::hex("#304060"),
            Color::hex("#202020"),
        ));
        self.sun = Sun {
            direction: Vec3::new(-0.6, -0.3, -0.4),
            ..Sun::default()
        };
        self.make_node::<Prop>(Shape3::Plane(400.0), Vec3::ZERO)
            .set_color(Color::hex("#5a6a4a"))
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::cuboid(0.1, 2.0, 0.1), Vec3::new(-0.3, 1.0, -1.0))
            .set_color(Color::hex("#c0703a"))
            .set_roughness(0.7);
        let height = 1.7 + PLATE * 0.5;
        self.make_node::<Prop>(
            Shape3::cuboid(PLATE, PLATE, 10.0),
            Vec3::new(-PLATE * 0.25, height, -DISTANCE),
        )
        .set_color(Color::hex("#b0b0b0"))
        .set_roughness(1.0);
        self.make_node::<Prop>(
            Shape3::cuboid(PLATE, PLATE, 10.0),
            Vec3::new(PLATE * 0.25, height, -DISTANCE - BEHIND),
        )
        .set_color(Color::hex("#d03030"))
        .set_roughness(1.0);
    }
}

impl SceneTest for NearAndFar {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(MOONS)
    }
}

/// The post, the floor and the grey plate over the red one. The probes at
/// 288 to 312 lie where the 2 plates overlap, 326 300 and 330 290 on the
/// red one where it shows.
const MOONS: &str = r"
       4    4 - #24314e
     320    4 - #212e4a
     592    4 - #23314e
     456    8 - #222f4b
     164   32 - #23304d
       4  128 - #2a3957
     288  136 - #293856
     456  148 - #2a3957
     112  184 - #814d2e
     592  200 - #2d3d5c
     176  216 - #9a5c38
     300  272 - #2f3f5f
     284  280 - #76787d
     264  284 - #76787d
     156  288 - #814d2e
     330  290 - #8c2427
     312  296 - #76787d
     288  300 - #76787d
     326  300 - #8c2427
     268  304 - #76787d
     300  308 - #76787d
     312  320 - #76787d
     264  324 - #76787d
     288  324 - #76787d
     108  328 - #6d4a3a
       4  332 - #3a4336
     436  332 - #3a4436
     528  332 - #3b4436
     164  356 - #9a5c37
     592  380 - #394235
     108  392 - #814d2d
     336  400 - #384234
     204  408 - #384133
     416  420 - #384133
     508  424 - #384133
      12  436 - #384133
     592  460 - #384133
     284  464 - #374132
     172  468 - #9a5c37
     108  488 - #804c2d
     520  508 - #374032
      32  516 - #364032
     424  524 - #364032
     176  552 - #364031
     264  552 - #364031
     592  556 - #364032
       4  592 - #364031
     104  592 - #804c2c
     344  592 - #364031
     504  592 - #364031
";
