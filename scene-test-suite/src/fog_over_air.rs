use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{
        Atmosphere, Camera, Fog, NodeTemplates, Occluder, Prop, SceneCreation, SceneSetup, SceneTest,
        SpaceSky, Sun, scene,
    },
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const PLANET: f32 = 20_000.0;
const AIR: f32 = 5000.0;
const CENTER: Vec3 = Vec3::new(0.0, -PLANET, 0.0);
const WATER: &str = "#123a5a";

/// A camera on the ground of a planet with air, in front of a near post and a
/// far post. First the sky and the haze alone. A `Fog::around` then lies over
/// them, the near post still shows, the far post and the whole sky are its
/// color, up as well. Taking it away brings all of it back.
#[scene]
#[derive(Default)]
struct FogOverAir {}

impl SceneSetup for FogOverAir {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 2.0, 0.0),
            target: Vec3::new(100.0, 30.0, 0.0),
            far: f32::INFINITY,
            ..Camera::default()
        };
        self.space_sky = Some(SpaceSky {
            atmosphere: Some(Atmosphere::scaled(CENTER, PLANET, AIR)),
            ..SpaceSky::default()
        });
        self.occluders = vec![Occluder::new(CENTER, PLANET)];
        self.sun = Sun {
            direction: -Vec3::new(-0.3, 1.0, 0.4).normalize(),
            intensity: 1.0,
            ..Sun::default()
        };
        self.make_node::<Prop>(Shape3::Plane(6000.0), Vec3::ZERO)
            .set_color(Color::hex("#6f8f5a"))
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::cuboid(1.5, 8.0, 1.5), Vec3::new(10.0, 4.0, 3.0))
            .set_color(Color::hex("#c0703a"))
            .set_roughness(0.8);
        self.make_node::<Prop>(Shape3::cuboid(12.0, 60.0, 12.0), Vec3::new(120.0, 30.0, -30.0))
            .set_color(Color::hex("#c0703a"))
            .set_roughness(0.8);
    }
}

impl SceneTest for FogOverAir {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(32);

        wait_for_next_frame();
        check_colors(AIR_ALONE)?;

        from_main(move || scene.fog = Some(Fog::around(Color::hex(WATER), 2.0, 40.0)));
        wait_for_next_frame();
        check_colors(IN_THE_FOG)?;

        from_main(move || scene.fog = None);
        wait_for_next_frame();
        check_colors(AIR_ALONE)
    }
}

const AIR_ALONE: &str = r"
             136    4 - #283d5e
             592    4 - #2c4264
             292   12 - #4d576e
             444   16 - #2a4061
               4   84 - #2b4263
             280  148 - #2d4365
             416  164 - #824f33
             152  196 - #815140
             592  256 - #385276
             404  264 - #503021
               4  268 - #395377
             500  296 - #814d30
              92  300 - #3c567a
             312  316 - #3c577b
             208  332 - #3f5b7f
             412  364 - #503121
               4  380 - #4c698b
             548  388 - #4c688a
             416  416 - #503121
             208  420 - #547190
             296  436 - #597591
              48  444 - #5d7993
             124  444 - #5d7893
             424  468 - #503122
             520  480 - #804c2e
             432  520 - #513122
             592  540 - #6e885a
             220  552 - #6e8759
             440  572 - #804c2e
               4  592 - #6e8759
             116  592 - #6e8759
             328  592 - #6e8759
";

const IN_THE_FOG: &str = r"
               4    4 - #123a5a
             232    4 - #123a5a
             592    4 - #123a5a
             408  148 - #463336
             168  176 - #123a5a
             488  204 - #734a3e
             400  212 - #453337
               4  252 - #123a5a
             420  272 - #744a3c
             496  280 - #734a3d
             300  300 - #123a5a
             144  336 - #123a5a
             508  356 - #74493b
             412  368 - #473335
             428  416 - #493333
             508  448 - #74493b
             416  464 - #473335
              84  484 - #38545a
             316  484 - #3e595a
             592  488 - #3c575a
               4  504 - #4e685a
             216  508 - #556e5a
             528  508 - #536c5a
             144  520 - #59725a
             372  524 - #5b745a
             436  548 - #493333
              68  556 - #607959
             528  572 - #73493b
               4  592 - #637d59
             132  592 - #647e59
             264  592 - #657e59
             360  592 - #657e59
";
