use std::f32::consts::PI;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, NodeTemplates, Occluder, Prop, SceneCreation, SceneSetup, SceneTest, Sun, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const PLANET: f32 = 30.0;
const MOON: f32 = 6.0;
const MOON_AT: Vec3 = Vec3::new(65.0, 6.0, 20.0);
const RED_MOON: f32 = 5.0;
const RED_MOON_AT: Vec3 = Vec3::new(-75.0, 0.0, 0.0);

/// A planet in space under a sun from the right, with 3 things the plain
/// shading of a surface cannot show. A rock hangs 2.5 m over the night
/// side, 30 degrees past the line between day and night: its side toward
/// the sun faces it, but the curve of the planet hides the sun, so only
/// the dim red the planet's air lets through reaches it. A moon between
/// the sun and the planet throws a soft round shadow on the day side. A
/// second moon sits in the planet's shadow behind it, red the same way.
/// Then the occluders are cleared: the rock lights up, the shadow on the
/// day side goes, the moon behind turns white.
#[scene]
#[derive(Default)]
struct NightAndEclipse {}

impl SceneSetup for NightAndEclipse {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 25.0, 175.0),
            target: Vec3::new(0.0, 0.0, 0.0),
            far: f32::INFINITY,
            ..Camera::default()
        };
        self.ambient = Color::hex("#060608");
        self.sun = Sun {
            direction: Vec3::NEG_X,
            intensity: 1.0,
            angular_radius: 0.05,
            ..Sun::default()
        };
        self.make_node::<Prop>(Shape3::Ball(PLANET), Vec3::ZERO)
            .set_color(Color::hex("#5f8f5a"))
            .set_roughness(0.9);
        let angle = PI * 2.0 / 3.0;
        let rock = Vec3::new(angle.cos(), angle.sin(), 0.0) * (PLANET + 2.5);
        self.make_node::<Prop>(Shape3::Ball(1.5), rock)
            .set_color(Color::hex("#ffffff"))
            .set_roughness(0.8);
        self.make_node::<Prop>(Shape3::Ball(MOON), MOON_AT)
            .set_color(Color::hex("#b0b0b0"))
            .set_roughness(1.0);
        self.make_node::<Prop>(Shape3::Ball(RED_MOON), RED_MOON_AT)
            .set_color(Color::hex("#ffffff"))
            .set_roughness(1.0);
        self.occluders = vec![
            Occluder::new(Vec3::ZERO, PLANET).transmit(Color::hex("#5a1a08")),
            Occluder::new(MOON_AT, MOON),
            Occluder::new(RED_MOON_AT, RED_MOON),
        ];
    }
}

impl SceneTest for NightAndEclipse {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(HIDDEN)?;

        from_main(move || scene.occluders.clear());
        wait_for_next_frame();
        check_colors(NOTHING_HIDES)
    }
}

/// The rock and the moon behind red, the moon's shadow on the day side.
const HIDDEN: &str = r"
       4    4 - #597c95
     592    4 - #597c95
     304  212 - #192518
     248  216 - #060608
     252  216 - #3a120b
     332  224 - #3e5a3b
     348  240 - #4a6b47
     304  248 - #132012
     328  248 - #385235
     368  248 - #5a8156
     340  268 - #405e3d
     380  268 - #618b5d
     248  276 - #010201
     532  280 - #818181
     304  284 - #121f11
     536  284 - #9d9d9d
      84  288 - #42140c
     532  288 - #787878
     536  288 - #999999
     380  292 - #121e11
     532  292 - #797979
     536  292 - #9a9a9a
      92  296 - #591c0f
     532  296 - #7e7e7e
      64  300 - #060608
      80  300 - #3e130c
     336  300 - #3c583a
      88  304 - #531a0e
     368  304 - #20301e
     372  304 - #1c2c1b
      84  312 - #3f130c
     304  312 - #131f11
     384  320 - #648f60
     216  324 - #010201
     324  328 - #334c31
     352  328 - #4c6e49
     268  336 - #010201
     376  340 - #5f885b
     340  348 - #436240
     364  352 - #577d53
     236  360 - #010201
     316  364 - #2a3f28
     352  368 - #50734c
     324  384 - #3a5338
     288  388 - #2d3f4b
       4  592 - #597c95
     372  592 - #597c95
     592  592 - #597c95
";

/// No occluders: the rock lit, no shadow spot, the moon behind white.
const NOTHING_HIDES: &str = r"
       4    4 - #597c95
     592    4 - #597c95
     248  216 - #060608
     252  216 - #aaaaaa
     328  216 - #3e583b
     304  236 - #142013
     360  240 - #547951
     320  264 - #2e452c
     264  276 - #010201
     364  280 - #557a52
     532  280 - #818181
     524  284 - #030304
     536  284 - #9d9d9d
     304  288 - #131f12
     532  288 - #787878
     536  288 - #999999
      84  292 - #cdcdcd
     332  292 - #3b5638
     384  292 - #648e60
     536  292 - #9a9a9a
      80  296 - #b1b1b2
      84  296 - #d3d3d3
      88  296 - #e9e9e9
      92  296 - #f0f0f0
     532  296 - #7e7e7e
      64  300 - #060608
      68  300 - #060608
      76  300 - #818181
      80  300 - #b3b3b4
      84  300 - #d5d5d5
      88  300 - #eaeaea
      92  300 - #f1f1f1
      80  304 - #b0b0b0
      84  304 - #d2d2d2
      92  304 - #f0f0f0
     212  304 - #010201
      88  308 - #e2e2e2
     304  316 - #131f12
     384  316 - #648f60
     344  320 - #466543
     316  356 - #2a3f28
     364  356 - #577d54
     236  360 - #010201
     340  376 - #476644
     316  384 - #2e432d
     288  388 - #2d3f4b
       4  592 - #597c95
     592  592 - #597c95
";
