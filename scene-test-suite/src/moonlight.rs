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
     336    4 - #597c95
     592    4 - #597c95
     212  232 - #aa9bba
     236  240 - #4d65c9
     208  244 - #ad9cb9
     220  248 - #8986c2
     364  252 - #bba6af
     200  256 - #b39fac
     252  256 - #536eda
     156  260 - #eeca97
     156  264 - #eeca96
     220  264 - #7375ba
     452  268 - #bba6b0
     208  272 - #8d83a8
     196  276 - #a69296
     260  276 - #546fdb
     224  284 - #4258af
     176  288 - #c1a379
     208  292 - #676490
     192  296 - #927e6f
     208  296 - #5c5b8a
     180  300 - #aa8f6a
     204  300 - #635d7c
     232  300 - #4258af
     168  304 - #bda076
     188  304 - #8f7959
     204  304 - #575374
     220  308 - #364890
     184  312 - #8d7758
     160  316 - #bb9e75
     196  316 - #554735
     252  316 - #445ab4
     208  320 - #222c57
     180  324 - #806c4f
     224  324 - #303f7f
     176  328 - #826d51
     196  328 - #101010
     240  328 - #374992
     208  332 - #101010
     216  332 - #1f274e
     220  336 - #1d2548
     232  336 - #273366
     196  340 - #101010
     356  340 - #e4c18f
       4  592 - #605459
     284  592 - #5f5359
     592  592 - #5f5359
";

/// The moon's light gone, the right sides dark.
const WITHOUT_MOON: &str = r"
       4    4 - #597c95
     336    4 - #597c95
     592    4 - #597c95
     172   60 - #597c95
       4  176 - #597c95
     208  232 - #ad926c
     220  236 - #867154
     232  236 - #3c3226
     172  244 - #e8c592
     204  248 - #ab916b
     220  248 - #7b674c
     232  248 - #2c251d
     364  252 - #b69a72
     168  256 - #e8c592
     228  256 - #3e3527
     156  260 - #eeca97
     196  260 - #b39870
     216  260 - #79664b
     408  260 - #b69a72
     224  264 - #473c2d
     452  268 - #b69a72
     160  272 - #e9c593
     204  272 - #917a5a
     260  272 - #101010
     220  276 - #403628
     152  284 - #e8c592
     188  284 - #ac916b
     212  284 - #5c4e39
     204  288 - #77644a
     240  292 - #101010
     192  296 - #917a5a
     200  296 - #77644a
     208  296 - #514432
     352  296 - #e4c18f
     204  304 - #4f4231
     184  308 - #947d5c
     168  312 - #b39770
     256  312 - #101010
     188  316 - #78654a
     172  328 - #8c7657
     228  336 - #101010
     192  340 - #101010
     356  340 - #e4c18f
     592  400 - #5d4d38
       4  404 - #5d4e38
       4  592 - #5d4e38
     276  592 - #5c4d37
     592  592 - #5c4d37
";
