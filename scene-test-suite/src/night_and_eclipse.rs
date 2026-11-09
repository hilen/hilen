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
     308  212 - #233221
     248  216 - #060608
     252  216 - #3a120b
     336  220 - #446241
     304  240 - #131f12
     356  240 - #51744d
     336  244 - #405d3d
     376  256 - #60895c
     320  264 - #2e452c
     344  264 - #456542
     216  272 - #010201
     384  276 - #659061
     304  280 - #121e11
     532  280 - #808080
     536  284 - #9d9d9d
      84  288 - #42140c
     532  288 - #787878
     536  288 - #999999
     380  292 - #121e11
     532  292 - #797979
     536  292 - #9a9a9a
      92  296 - #591c0f
     336  296 - #3a5538
     532  296 - #7e7e7e
      64  300 - #060608
      80  300 - #3e130c
     368  304 - #1f301e
     372  304 - #1c2c1b
     388  304 - #689364
      84  312 - #3f130c
     384  312 - #648e60
     268  316 - #010201
     336  316 - #3f5b3c
     384  324 - #658f60
     320  328 - #2e452c
     352  328 - #4c6e49
     220  336 - #010201
     372  340 - #5c8358
     304  356 - #131f12
     360  356 - #547950
     332  360 - #3c5839
     256  376 - #010201
     316  384 - #2e432c
       4  592 - #597c95
     224  592 - #597c95
     592  592 - #597c95
";

/// No occluders: the rock lit, no shadow spot, the moon behind white.
const NOTHING_HIDES: &str = r"
       4    4 - #597c95
     592    4 - #597c95
     248  216 - #060608
     252  216 - #a9a9aa
     328  216 - #3e583b
     364  240 - #587e55
     336  244 - #405d3d
     304  248 - #131f12
     332  272 - #3b5638
     384  276 - #659061
     532  280 - #808080
     536  284 - #9d9d9d
     264  288 - #010201
     360  288 - #52764f
     532  288 - #787878
     536  288 - #999999
      80  292 - #a9a9aa
      88  292 - #e5e5e5
     524  292 - #030304
     536  292 - #9a9a9a
      84  296 - #d3d3d3
      88  296 - #e9e9e9
      92  296 - #f0f0f0
     384  296 - #648e60
     532  296 - #7e7e7e
      64  300 - #060608
      68  300 - #060608
      76  300 - #808081
      80  300 - #b3b3b3
      84  300 - #d5d5d5
      88  300 - #eaeaea
      92  300 - #f1f1f1
      80  304 - #b0b0b0
      84  304 - #d2d2d2
      92  304 - #f0f0f0
     212  304 - #010201
      84  308 - #cacaca
     332  308 - #3b5638
     384  308 - #648e60
     380  316 - #618a5d
     304  328 - #121f11
     352  336 - #4c6e49
     320  356 - #2f462d
     240  364 - #010201
     348  372 - #4d6e4a
     304  384 - #162215
       4  592 - #597c95
     592  592 - #597c95
";
