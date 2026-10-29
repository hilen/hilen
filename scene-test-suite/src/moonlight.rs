use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, Light, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sun, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

/// A ball and a crate on a floor, a warm sun from the left and a blue
/// directional light from the right, the light of a moon, with a lamp far
/// off that reaches nothing. The right sides take the blue light, the
/// left ones the sun. Then the moon's light is taken away and the right
/// sides go dark, only the faint ambient left on them.
#[scene]
#[derive(Default)]
struct Moonlight {}

impl SceneSetup for Moonlight {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 3.0, 7.0),
            target: Vec3::new(0.0, 0.6, 0.0),
            ..Camera::default()
        };
        self.ambient = Color::hex("#101010");
        self.sun = Sun {
            direction: Vec3::new(1.0, -0.6, -0.2),
            color: Color::hex("#ffd8a0"),
            intensity: 0.9,
            ..Sun::default()
        };
        self.lights.push(Light::point(Vec3::new(500.0, 2.0, 0.0)).range(5.0));
        self.lights.push(
            Light::directional(Vec3::new(-1.0, -0.4, -0.1))
                .color(Color::hex("#6080ff"))
                .intensity(0.7),
        );
        self.make_node::<Prop>(Shape3::Plane(20.0), Vec3::ZERO)
            .set_color(Color::hex("#808080"))
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::Ball(0.8), Vec3::new(-1.3, 0.8, 0.0))
            .set_color(Color::hex("#ffffff"))
            .set_roughness(0.7);
        self.make_node::<Prop>(Shape3::cube(1.2), Vec3::new(1.4, 0.6, 0.0))
            .set_color(Color::hex("#ffffff"))
            .set_roughness(0.7);
    }
}

impl SceneTest for Moonlight {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(WITH_MOON)?;

        from_main(move || {
            scene.lights.pop();
        });
        wait_for_next_frame();
        check_colors(WITHOUT_MOON)
    }
}

/// The right sides blue from the moon, the left ones warm from the sun.
const WITH_MOON: &str = r"
       4    4 - #597c95
     332    4 - #597c95
     592    4 - #597c95
     208  232 - #b3a1b7
     232  240 - #5f6ec8
     400  252 - #bca6af
     160  256 - #edc996
     200  260 - #af9caa
     220  260 - #797abd
     256  260 - #546fdb
     352  268 - #817068
     452  268 - #bca6af
     152  272 - #edc996
     352  272 - #817068
     352  276 - #a28b75
     200  280 - #978797
     236  280 - #4b63c5
     352  280 - #a28b75
     180  284 - #bea177
     352  284 - #c3a683
     264  288 - #596097
     192  292 - #998478
     200  296 - #7c6f7e
     208  296 - #5d5c8a
     212  296 - #464f8f
     152  300 - #dbba8a
     172  300 - #bb9e75
     204  304 - #575375
     240  304 - #455cb7
     188  308 - #887355
     180  312 - #98805f
     212  312 - #2d3b77
     224  312 - #384a94
     196  316 - #554735
     360  316 - #c8aa85
     204  320 - #1c2343
     168  324 - #9f8663
     180  324 - #806c50
     212  324 - #242e5d
     360  324 - #ac937a
     360  328 - #8f7b6e
     236  332 - #314182
     220  336 - #1e264a
     192  340 - #101010
     208  340 - #101010
       4  592 - #605459
     308  592 - #5f5459
     592  592 - #5f5459
";

/// The moon's light gone, the right sides dark.
const WITHOUT_MOON: &str = r"
       4    4 - #597c95
     336    4 - #597c95
     592    4 - #597c95
       4  180 - #597c95
     216  232 - #967e5d
     184  236 - #dab989
     172  248 - #e7c491
     200  248 - #b49871
     220  248 - #7a674c
     232  248 - #2d261d
     400  252 - #b69a72
     160  256 - #edc996
     228  256 - #3e3527
     216  260 - #79664b
     256  260 - #101010
     224  264 - #483c2d
     192  268 - #b49971
     352  268 - #806b4e
     424  268 - #b69a72
     452  268 - #b69a72
     160  272 - #e9c593
     204  272 - #917a5a
     220  272 - #4d4130
     352  272 - #806b4e
     352  276 - #a18864
     352  280 - #a18864
     152  284 - #e8c592
     212  284 - #5c4e39
     352  284 - #c2a47a
     188  288 - #a78d68
     204  288 - #77644a
     264  288 - #372f24
     200  296 - #77644a
     236  300 - #101010
     204  304 - #4f4331
     184  308 - #947d5c
     360  316 - #c7a97e
     184  320 - #7c694d
     360  324 - #ab906b
     172  328 - #8c7757
     240  328 - #101010
     360  328 - #8e7858
     192  340 - #101010
     592  400 - #5d4d38
       4  404 - #5e4e38
       4  592 - #5d4e38
     300  592 - #5c4d37
     592  592 - #5c4d37
";
