use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, Light, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sun, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

const LAMP: Vec3 = Vec3::new(-3.5, 1.5, 1.0);
const LAMP_MOVED: Vec3 = Vec3::new(3.5, 1.5, 1.0);

/// A dark scene, no sun and little ambient, lit by a red point light on
/// the left and a cyan spot light aimed down at the tall box on the
/// right. The point light falls off with distance and the spot lights
/// nothing outside its cone. Then the point light jumps to the right
/// side, so the frame after it has to be lit from there, the light
/// list is rebuilt every frame.
#[scene]
#[derive(Default)]
struct Lights {}

impl SceneSetup for Lights {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 5.0, 9.0),
            target: Vec3::new(0.0, 0.8, 0.0),
            ..Camera::default()
        };

        self.sun = Sun {
            intensity: 0.0,
            ..Sun::default()
        };
        self.ambient = Color::hex("#303030");

        self.lights
            .push(Light::point(LAMP).color(Color::hex("#ff5030")).intensity(3.0).range(7.0));
        self.lights.push(
            Light::spot(Vec3::new(2.5, 6.0, -1.0), Vec3::new(0.0, -1.0, -0.3), 0.35)
                .color(Color::hex("#40e0ff"))
                .intensity(30.0)
                .range(12.0),
        );

        let matte = Color::hex("#e0e0e0");
        self.make_node::<Prop>(Shape3::Plane(12.0), Vec3::ZERO)
            .set_color(matte)
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::Ball(0.8), Vec3::new(0.0, 0.8, 0.5))
            .set_color(matte)
            .set_roughness(0.4);
        self.make_node::<Prop>(Shape3::cube(1.5), Vec3::new(-2.5, 0.75, -1.0))
            .set_color(matte)
            .set_roughness(0.7);
        self.make_node::<Prop>(Shape3::cuboid(1.2, 3.0, 1.2), Vec3::new(2.5, 1.5, -2.5))
            .set_color(matte)
            .set_roughness(0.7);
    }
}

impl SceneTest for Lights {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        check_colors(LAMP_LEFT)?;

        from_main(move || scene.lights[0].position = LAMP_MOVED);
        wait_for_next_frame();

        capture_screenshot()?;
        check_colors(LAMP_RIGHT)
    }
}

const LAMP_LEFT: &str = r"
       4    4 - #597c95
     232    4 - #597c95
     436    4 - #597c95
     592    4 - #597c95
     120    8 - #597c95
     336   52 - #597c95
     144  120 - #597c95
     268  140 - #597c95
     396  156 - #66ddf9
     404  156 - #67def9
     416  156 - #67def9
     428  156 - #65ddf8
     592  156 - #597c95
       4  160 - #597c95
     392  160 - #71e0fa
     400  160 - #73e0fa
     412  160 - #73e0fa
     424  160 - #73e0fa
     436  160 - #72e0fa
     444  168 - #41b5ce
     396  176 - #41abc2
     428  176 - #3fadc4
     412  180 - #3fa8be
     444  188 - #3b9cb1
     392  192 - #3d98ac
     408  200 - #3a91a4
     428  200 - #3991a4
     396  236 - #357280
     432  236 - #337280
     412  240 - #33707e
     352  252 - #2e4e57
     356  252 - #2f5862
     388  252 - #356774
     428  252 - #316774
     336  256 - #2d484f
     340  256 - #2f5761
     344  256 - #316672
     400  256 - #336572
     444  256 - #2d454c
     412  260 - #31636f
     132  264 - #f76a59
     164  264 - #e6523d
     148  268 - #f35e4a
     204  268 - #9c3e32
     376  268 - #2a2a2a
     452  268 - #3a94a7
     328  272 - #3b94a8
     356  272 - #3b9baf
     376  272 - #2a2a2a
     396  272 - #315c67
     464  272 - #3a96a9
     340  276 - #3c9bb0
     376  276 - #2a2a2a
     408  276 - #305b65
     420  276 - #2f5a65
     440  276 - #3c9eb3
     388  280 - #315863
     152  284 - #ed5641
     340  288 - #40a3b8
     364  288 - #3fa7bd
     396  288 - #30555f
     408  288 - #2f555f
     484  288 - #326c79
     388  292 - #30535c
     428  292 - #2e535c
     448  292 - #3ea9bf
     468  292 - #3ea5bb
     136  296 - #ed5641
     260  296 - #8e3b31
     172  300 - #bd4737
      56  308 - #5d312d
     204  308 - #8c3a31
     364  308 - #3a8090
     368  308 - #3d90a3
     460  308 - #388a9c
     368  312 - #34525b
     372  312 - #35616c
     376  312 - #376f7d
     380  312 - #387c8c
     384  312 - #3a8899
     412  312 - #41b1c9
     448  312 - #368293
     452  312 - #347685
     456  312 - #316875
     460  312 - #2f5964
     464  312 - #2d4a51
     396  316 - #325e69
     424  316 - #336f7d
     440  316 - #2f5a64
     580  320 - #2a2a2a
     116  332 - #bd4636
     236  332 - #6d332e
       4  336 - #6b332e
     192  340 - #9b3e32
     160  344 - #c64938
     100  352 - #ee5741
      48  356 - #be4737
     136  356 - #eb553f
     268  356 - #5d312d
      76  364 - #ec5640
     208  368 - #8c3a31
      96  372 - #f05a45
     116  376 - #ec5540
     160  376 - #bf4737
      36  384 - #c04737
     136  392 - #bd4636
     244  392 - #64322d
      92  396 - #cc4b39
      60  400 - #bf4737
       4  404 - #973d32
     164  408 - #8c3a31
      80  424 - #973d32
     124  424 - #8b3a31
     592  424 - #2a2a2a
      36  428 - #8e3b31
     220  428 - #5e312d
       4  456 - #6f342e
     160  460 - #5e312d
     104  476 - #5e312d
     324  476 - #382b2b
     440  476 - #2c2a2a
      52  484 - #5e312d
     536  508 - #2a2a2a
       4  592 - #3c2c2b
     188  592 - #342b2a
     332  592 - #2d2a2a
     472  592 - #2a2a2a
     592  592 - #2a2a2a
";

const LAMP_RIGHT: &str = r"
       4    4 - #597c95
     232    4 - #597c95
     436    4 - #597c95
     592    4 - #597c95
     592  152 - #597c95
       4  156 - #597c95
     400  156 - #67ddf9
     392  160 - #71e0fa
     404  160 - #73e0fa
     412  160 - #73e0fa
     424  160 - #73e0fa
     436  160 - #72e0fa
     448  168 - #85b7cd
     176  176 - #597c95
     432  176 - #85afc4
     396  180 - #7aa8bd
     420  184 - #84a6ba
     440  188 - #8aa0b2
     404  196 - #8198a9
     428  200 - #8b95a5
     392  204 - #7d8f9f
     416  208 - #888d9d
     440  212 - #908997
     428  220 - #8e8390
     404  232 - #857a86
     420  232 - #8c7b86
     440  232 - #927a85
     400  244 - #83727c
     348  252 - #2d444a
     352  252 - #2f4e57
     356  252 - #305862
     360  252 - #32616d
     364  252 - #336976
     368  252 - #34707e
     436  252 - #8f6e76
     336  256 - #2e484f
     340  256 - #305761
     344  256 - #336672
     348  256 - #357482
     388  256 - #7b6a73
     416  256 - #886b74
     444  256 - #31454c
     376  268 - #2a2a2a
     400  268 - #7d646c
     440  268 - #4596aa
     340  272 - #4298ac
     356  272 - #449baf
     376  272 - #2a2a2a
     428  272 - #866369
     464  272 - #4796a9
     328  276 - #4399ad
     376  276 - #2a2a2a
     388  280 - #745d64
     488  280 - #433e42
     372  284 - #50a7bc
     444  284 - #57a5bb
     484  284 - #4c6672
     344  288 - #4ea4b9
     352  288 - #4fa6bb
     364  288 - #52a8be
     384  288 - #6f5960
     416  288 - #7b5b61
     484  288 - #526d7a
     488  288 - #4f535a
     340  292 - #4fa5ba
     356  292 - #53a8be
     392  292 - #71585e
     404  292 - #75595e
     432  292 - #7d595e
     460  292 - #61a8bd
     484  292 - #586c79
     488  292 - #55525a
     492  292 - #533b3d
     348  296 - #54a8be
     484  296 - #5e636e
     488  296 - #5c4b50
      44  300 - #2a2a2a
     364  308 - #5e8191
     368  308 - #6292a4
     404  308 - #74b2c9
     432  308 - #7cb2c8
     460  308 - #7c8d9d
     464  308 - #7b7c89
     540  308 - #5f312d
     364  312 - #5d4649
     368  312 - #60555c
     372  312 - #63636d
     376  312 - #66717e
     380  312 - #697e8d
     384  312 - #6c8a9a
     448  312 - #848694
     452  312 - #847a86
     456  312 - #836d77
     460  312 - #836066
     464  312 - #825254
     420  316 - #827580
     324  332 - #5f312d
     512  340 - #c74938
     432  344 - #be4737
     488  348 - #eb553f
     392  352 - #903b31
     460  356 - #e9543e
     504  360 - #f15a45
     556  360 - #be4737
     588  360 - #943c31
     524  364 - #eb553f
     336  376 - #5f312d
     424  376 - #ac4234
     484  376 - #ec5640
     508  380 - #eb553f
     460  388 - #c24837
     548  396 - #c14837
     496  400 - #c04737
     524  400 - #c34837
     196  408 - #312b2a
     592  416 - #903b31
     472  420 - #903b31
     524  428 - #913b31
     560  428 - #8f3b31
     384  432 - #5d312d
      52  448 - #2a2a2a
     444  460 - #5f312d
     508  480 - #5d312d
     592  484 - #5e312d
     116  552 - #2a2a2a
       4  592 - #2a2a2a
     228  592 - #2c2a2a
     392  592 - #332b2a
";
