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
     392  160 - #71e0fa
     436  160 - #72e0fa
     400  164 - #60cfe9
     412  164 - #60d0ea
     424  164 - #61d1ea
     444  168 - #42b6ce
       4  172 - #597c95
     428  180 - #3ea8be
     412  184 - #3ea3b9
     444  188 - #3c9cb1
     392  192 - #3d98ac
     380  200 - #425360
     408  200 - #3a91a4
     428  200 - #3991a4
     388  212 - #365860
     396  236 - #357281
     432  236 - #337281
     412  244 - #336d7b
     352  252 - #2e4e57
     360  252 - #30616d
     388  252 - #356774
     428  252 - #316875
     336  256 - #2d484f
     340  256 - #2f5761
     344  256 - #316672
     444  256 - #2d454c
     376  260 - #325d67
     416  260 - #316370
     136  264 - #f76756
     184  264 - #be4737
     364  264 - #3a94a7
     396  264 - #32616d
     156  268 - #ef5843
     340  268 - #3a94a8
     376  268 - #2a2a2a
     352  272 - #3b9aae
     376  272 - #2a2a2a
     432  272 - #305c68
     448  272 - #3b99ad
     328  276 - #3d98ac
     376  276 - #2a2a2a
     460  276 - #3b9baf
     384  280 - #325863
     132  284 - #ca5445
     392  284 - #315761
     404  284 - #2f5761
     424  284 - #2f5761
     152  288 - #eb553f
     484  288 - #326c79
     176  292 - #c04737
     208  292 - #913b31
     340  292 - #41a4ba
     360  292 - #40a8bf
     384  292 - #31535c
     396  292 - #2f535d
     416  292 - #2e535d
     432  292 - #2e535c
     452  292 - #3ea8bf
     468  292 - #3ea5bb
     260  296 - #8e3b31
     400  300 - #40afc6
      84  304 - #61312d
     368  308 - #3d90a3
     460  308 - #388a9c
     156  312 - #c04737
     368  312 - #34525b
     376  312 - #376f7d
     380  312 - #387c8c
     384  312 - #3a8899
     412  312 - #41b1c9
     448  312 - #368293
     452  312 - #347685
     460  312 - #2f5964
      32  316 - #5e312d
     392  316 - #325660
     428  316 - #326c79
     116  320 - #953c32
     580  320 - #2a2a2a
     192  324 - #8b3a31
     232  332 - #70342e
     164  344 - #c14837
     104  352 - #ee5742
     124  352 - #ee5741
      48  356 - #be4737
      84  356 - #eb553f
     268  356 - #5e312d
     132  368 - #eb553f
     208  368 - #8d3a31
     108  372 - #f05944
     160  376 - #bf4737
       4  380 - #9b3e32
      84  380 - #eb553f
      52  388 - #cd4b39
     136  388 - #c54938
     188  392 - #8d3a31
      72  400 - #c34837
     108  400 - #be4737
     148  416 - #8b3a31
     592  420 - #2a2a2a
      16  424 - #8d3a31
     224  424 - #5f312d
      64  432 - #8c3a31
     184  448 - #5f312d
     140  468 - #5e312d
     444  468 - #2c2a2a
      44  472 - #66322d
     328  476 - #372b2b
      88  480 - #5e312d
       4  484 - #5e312d
     108  580 - #3b2c2b
     468  588 - #2a2a2a
       4  592 - #3c2c2b
     208  592 - #342b2a
     348  592 - #2d2a2a
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
     416  156 - #67def9
     432  156 - #56d8f4
     424  160 - #73e0fa
     396  164 - #78d0e8
     412  164 - #7cd1ea
     436  164 - #7fd1ea
     448  168 - #85b8ce
     176  176 - #597c95
     396  180 - #7aa8bd
     420  184 - #84a6ba
     440  188 - #8aa1b3
     404  196 - #8198aa
     380  200 - #425360
     428  200 - #8b95a6
     444  200 - #8e94a4
     392  204 - #7d8f9f
     416  208 - #888e9d
     440  212 - #908a98
     388  216 - #686e79
     420  220 - #8c8491
     408  232 - #877b86
     424  232 - #8e7b86
     392  236 - #7f7682
     440  236 - #927882
     404  248 - #84707a
     356  252 - #305862
     360  252 - #31616d
     364  252 - #336976
     368  252 - #34707e
     388  252 - #7b6c76
     420  252 - #8a6e77
     436  252 - #8f6e77
     336  256 - #2e484f
     340  256 - #305761
     348  256 - #357482
     444  256 - #31454c
     376  260 - #345d67
     376  268 - #2a2a2a
     420  268 - #85656d
     444  268 - #4596a9
     456  268 - #4493a6
     340  272 - #4298ac
     356  272 - #449baf
     376  272 - #2a2a2a
     400  272 - #7c626a
     328  276 - #4298ac
     376  276 - #2a2a2a
     432  276 - #856167
     468  276 - #4b99ad
     388  280 - #745d64
     448  280 - #51a1b6
     488  280 - #433e42
     396  284 - #765c63
     484  284 - #4c6672
     488  284 - #494b52
     344  288 - #4da4b9
     352  288 - #4fa5bb
     364  288 - #52a8be
     376  288 - #54a9bf
     416  288 - #7b5b61
     488  288 - #4f535a
     340  292 - #4fa5ba
     356  292 - #53a8be
     392  292 - #71585e
     404  292 - #75595f
     432  292 - #7d595f
     464  292 - #61a7bc
     484  292 - #586c79
     348  296 - #54a8be
     448  296 - #68acc2
     488  296 - #5b4b50
      44  300 - #2a2a2a
     364  308 - #5e8191
     368  308 - #6192a4
     404  308 - #73b2c9
     432  308 - #7cb2c8
     460  308 - #7c8d9d
     468  308 - #796b74
     368  312 - #60555c
     372  312 - #63636d
     376  312 - #66717e
     384  312 - #6c8a9a
     448  312 - #848694
     456  312 - #836d77
     464  312 - #825254
     392  316 - #725b62
     420  316 - #827580
     568  316 - #5e312d
     500  336 - #c34837
     528  344 - #c14837
     476  348 - #ea553f
     428  352 - #be4737
     552  356 - #bd4737
     516  360 - #ee5741
     588  360 - #943c31
     332  364 - #5e312d
     460  364 - #e8533e
     496  364 - #f25c47
     400  372 - #933c31
     476  372 - #ed5640
     492  380 - #ea543f
     516  380 - #ea553f
     432  384 - #ab4234
     544  384 - #d44d3a
     472  392 - #c44938
     500  400 - #c14837
     592  400 - #9c3e32
     448  412 - #8f3b31
     184  416 - #302b2a
     368  420 - #5d312d
     564  424 - #933c31
     524  428 - #923b31
     408  440 - #61312d
       8  444 - #2a2a2a
     448  464 - #5e312d
     520  480 - #5f312d
     592  484 - #5e312d
     116  528 - #2a2a2a
       4  592 - #2a2a2a
     228  592 - #2c2a2a
     376  592 - #332b2a
";
