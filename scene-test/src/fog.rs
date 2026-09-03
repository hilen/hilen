use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, Fog, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, scene},
    ui::Color,
    ui_test::{capture_screenshot, check_colors, set_record_probe_count},
};

/// The fog's color, a pale grey a floor and a post both fade into.
const FOG: &str = "#b9c0c6";

/// A row of posts marching away over a long floor under a gradient sky
/// and distance fog. The near posts keep their orange, the far ones
/// fade into the fog's grey and the floor with them, so the far plane
/// never shows, and the sky is fog at the horizon clearing to blue
/// above. The fog is then pushed back, and the posts it hid come out of
/// it, then taken away, and the sky clears down to the horizon.
#[scene]
#[derive(Default)]
struct FogTest {}

impl SceneSetup for FogTest {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 6.0, 12.0),
            target: Vec3::new(0.0, 1.0, -60.0),
            ..Camera::default()
        };
        self.fog = Some(Fog::new(Color::hex(FOG), 15.0, 70.0));
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));

        self.make_node::<Prop>(Shape3::Plane(400.0), Vec3::ZERO)
            .set_color(Color::hex("#8f9aa3"))
            .set_roughness(0.9);

        for step in 0..10u8 {
            let z = -f32::from(step) * 12.0;
            let x = if step % 2 == 0 { 3.0 } else { -3.0 };
            self.make_node::<Prop>(Shape3::cuboid(1.5, 6.0, 1.5), Vec3::new(x, 3.0, z))
                .set_color(Color::hex("#e67e22"))
                .set_roughness(0.6);
        }

        self.make_node::<Prop>(Shape3::Ball(3.0), Vec3::new(0.0, 3.0, -130.0))
            .set_color(Color::hex("#2980b9"))
            .set_roughness(0.4);
    }
}

impl SceneTest for FogTest {
    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        check_colors(NEAR_FOG)?;

        from_main(move || scene.fog = Some(Fog::new(Color::hex(FOG), 80.0, 200.0)));
        wait_for_next_frame();
        capture_screenshot()?;
        check_colors(FAR_FOG)?;

        from_main(move || scene.fog = None);
        wait_for_next_frame();
        check_colors(NO_FOG)
    }
}

const NEAR_FOG: &str = r"
       4    4 - #b7c8e6
     124    4 - #b2c5e6
     320    4 - #afc2e5
     424    4 - #b1c4e6
     592    4 - #b7c8e6
     224   20 - #b4c6e6
     552   88 - #c2cfe1
      64   96 - #c3cfe1
     456  100 - #c3cfe1
     176  108 - #c3cfe0
     308  136 - #c4cfdd
     396  172 - #c4cdd8
     592  172 - #c3ccd6
      24  200 - #c1cad2
     132  220 - #bfc8cf
     232  264 - #d08a62
     272  264 - #c5a9a0
     408  264 - #d47d3b
     468  264 - #d47e3b
     252  268 - #c49b87
     332  268 - #af958c
     440  268 - #d47d3b
     392  272 - #a96f4d
     568  272 - #b9c0c6
     332  276 - #af958c
     268  280 - #c5a9a0
     320  280 - #bfb5b5
     332  280 - #af958c
     252  284 - #c49b87
     332  284 - #af958c
     332  288 - #af958c
     332  292 - #af958c
     392  292 - #a96f4d
     464  292 - #d47d3a
     332  296 - #af958c
     436  296 - #d47d3a
     216  300 - #c5a594
     272  300 - #c5a9a0
     332  300 - #af958c
     332  304 - #af958c
     332  308 - #af958c
     400  308 - #a96f4c
     260  312 - #c4a9a0
     320  312 - #beb6b6
     332  312 - #af958c
     216  316 - #c1a294
     332  316 - #af958c
     472  320 - #b9aaa1
     180  324 - #acb6c4
     248  324 - #d08a62
     276  324 - #b0b3bc
     332  324 - #af958c
     396  324 - #a96f4c
      76  328 - #acb6c4
     428  328 - #d47d39
       8  332 - #acb6c4
     332  332 - #af958c
     128  336 - #a6b2c3
     216  340 - #aea6aa
     400  340 - #a96f4c
     332  344 - #af958c
     468  348 - #d47d39
     244  356 - #d08a62
     392  356 - #a96f4b
     444  360 - #d47d38
     416  364 - #d47d38
     524  364 - #9cabc1
     220  368 - #d08a63
     592  368 - #9dabc1
      52  376 - #99a8c0
     392  376 - #a86e4b
     468  376 - #d47d38
     252  388 - #c78e71
     404  388 - #d47c37
     136  392 - #95a5c0
     220  396 - #a29ea8
     236  396 - #a29ea8
     436  396 - #d47c37
       4  400 - #95a6c0
     392  404 - #a86e49
     468  404 - #d47c37
     416  408 - #d47c37
     448  416 - #d47c37
     524  420 - #91a2bf
     392  424 - #a86e49
      72  428 - #90a2bf
     316  428 - #8ea1bf
     428  432 - #d37c36
     176  436 - #8ea0bf
     460  440 - #d37c36
     400  444 - #a86d48
       4  452 - #8ea1bf
     236  456 - #8c9fbe
     416  460 - #d37c36
     564  460 - #8da0bf
     392  464 - #a86d47
     436  464 - #d37c36
     132  472 - #8b9ebe
     468  472 - #c18557
     468  476 - #c18557
     400  480 - #a86c46
     448  480 - #d37c35
     468  484 - #af8d7a
     468  488 - #af8d7a
     424  492 - #d37c35
      32  496 - #8a9ebe
     284  496 - #899dbe
     392  496 - #a76c45
     468  496 - #9c959c
     468  500 - #9c959c
     444  504 - #d37c35
     204  512 - #899dbe
     396  512 - #a76c45
     420  516 - #d37c34
      92  520 - #899dbe
     464  524 - #d37c34
     400  528 - #bd743c
     440  528 - #d37c34
     336  532 - #899dbe
     548  532 - #899dbe
     272  556 - #899dbe
     112  580 - #899dbe
       4  592 - #899dbe
     220  592 - #899dbe
     320  592 - #899dbe
     420  592 - #899dbe
     504  592 - #899dbe
     592  592 - #899dbe
";

const FAR_FOG: &str = r"
       4    4 - #b7c8e6
     164    4 - #b1c4e6
     284    4 - #afc2e5
     384    4 - #b0c3e6
     488    4 - #b3c5e6
     592    4 - #b7c8e6
      64   76 - #c1cee2
     356   76 - #bfcde4
     228   80 - #c0cde3
     436   92 - #c2cee2
     536   92 - #c3cfe1
     144  120 - #c4cfdf
     308  136 - #c4cfdd
       4  148 - #c4ceda
      80  164 - #c4ced8
     220  172 - #c4ced8
     392  172 - #c4ced8
     488  176 - #c4cdd7
     592  176 - #c3ccd6
     112  236 - #bdc5cc
     528  248 - #bbc3c9
     220  264 - #d47d3a
     396  264 - #a96f4e
     448  264 - #d47e3b
     252  268 - #c6906e
     288  268 - #cc977f
     312  268 - #ce9070
     332  268 - #a96f4d
     348  272 - #d47d3a
     252  276 - #c6906e
     292  280 - #8b9aae
     296  280 - #8998ac
     300  280 - #8999ae
     312  280 - #ce9070
     332  280 - #a96f4d
     472  280 - #d47d3b
     252  284 - #c28d6e
     296  284 - #8c99a7
     300  284 - #8a97a5
     400  284 - #a96f4d
       4  288 - #a6b2c3
     288  288 - #cc977f
     312  288 - #ce9070
     428  288 - #d47d3a
     148  292 - #99a8c0
     332  292 - #a96f4d
     216  296 - #c4875b
     252  296 - #bc896d
     592  300 - #93a4c0
     332  304 - #a86f4c
     392  304 - #a96f4d
     216  308 - #af8e7c
     472  308 - #af8e7c
     216  316 - #af8e7c
     276  316 - #9a97a3
     324  316 - #af8e7c
     328  316 - #af8e7c
     432  316 - #d47d39
     472  316 - #af8e7c
     276  320 - #9a97a3
     216  324 - #9d969d
     276  324 - #9a97a3
     396  324 - #a96f4c
     472  324 - #9d969d
     216  328 - #9d969d
     248  328 - #d47d38
     332  328 - #a86e4b
     216  332 - #9d969d
     216  336 - #9d969d
     216  340 - #9d969d
     108  344 - #8a9ebe
     332  344 - #a86e4b
     400  344 - #a96f4c
     468  352 - #d47d39
     252  356 - #ca7f4f
     440  356 - #d47d38
     392  360 - #a86f4b
     224  368 - #d47c37
     548  368 - #8a9ebe
       4  376 - #8a9dbe
     392  380 - #a86e4a
     428  384 - #d47c38
     468  388 - #d47c38
     220  396 - #9c959c
     228  396 - #9c959c
     236  396 - #9c959c
     240  396 - #9c959c
     244  396 - #9c959c
     248  396 - #9c959c
     400  400 - #a86e4a
     436  416 - #d47c37
     144  420 - #8a9dbe
     396  420 - #a86e49
     468  424 - #d47c37
     316  432 - #899dbe
     392  436 - #a86d48
     592  436 - #8a9dbe
      64  440 - #899dbe
     428  444 - #d37c36
     400  448 - #a86d48
     528  452 - #8a9dbe
     456  456 - #d37c36
     392  460 - #a86d47
     392  476 - #a86d46
     428  484 - #d37c35
     184  488 - #899dbe
     400  488 - #a86c46
     468  488 - #ae8d7a
     468  500 - #9c959c
       4  504 - #899dbe
     396  504 - #a76c45
     280  508 - #899dbe
     444  508 - #d37c35
     552  516 - #899dbe
     464  528 - #d37c34
     404  532 - #ae8c79
     412  532 - #ae8c79
     420  532 - #ae8c79
     428  532 - #ae8c79
     436  532 - #ae8d79
     444  532 - #ae8d79
     452  532 - #ae8d79
     104  544 - #899dbe
       4  592 - #899dbe
     204  592 - #899dbe
     328  592 - #899dbe
     512  592 - #899dbe
     592  592 - #899dbe
";

const NO_FOG: &str = r"
       4    4 - #b7c8e7
     268    4 - #afc2e5
     372    4 - #b0c3e5
     488    4 - #b3c5e6
     592    4 - #b7c8e6
     136    8 - #b3c5e6
      68   76 - #c2d1e8
     352   76 - #bfcfe8
     216   88 - #c2d1e8
     432   88 - #c3d1e9
     536   92 - #c5d3e9
     304  132 - #c9d6ea
     136  140 - #cbd8ea
       4  144 - #cdd9eb
     388  172 - #d0dbeb
     480  172 - #d0dbeb
     224  176 - #d0dceb
     592  176 - #d1dceb
      96  232 - #d4dfec
     540  248 - #d5e0ec
     220  264 - #d47d3a
     300  264 - #4191e4
     396  264 - #a96f4e
     448  264 - #d47e3b
     252  268 - #cd9878
     296  268 - #3086d4
     304  268 - #4093e1
     332  268 - #a96f4d
     352  268 - #d47d3a
     252  272 - #cd9878
     292  272 - #3478ba
     296  272 - #3581c6
     300  272 - #408ad0
     304  272 - #3f8dd4
     308  272 - #3d8bd2
     252  276 - #cd9878
     292  276 - #32689f
     296  276 - #3373af
     300  276 - #357ab8
     304  276 - #377ebd
     308  276 - #3b7fbc
       4  280 - #8a9ebe
     252  280 - #ba886c
     292  280 - #30618f
     296  280 - #245b8c
     300  280 - #235e90
     304  280 - #276396
     472  280 - #d47d3b
     252  284 - #ba886c
     300  284 - #255278
     400  284 - #a96f4d
     428  284 - #d47d3a
     252  288 - #ba886c
     332  288 - #a96f4d
     252  292 - #ba886c
     252  296 - #ba886c
     216  300 - #af8e7c
     332  304 - #a86f4c
     392  304 - #a96f4d
     472  308 - #af8e7c
     216  312 - #af8e7c
     432  312 - #d47d39
     272  316 - #d47d39
     324  316 - #af8e7c
     328  316 - #af8e7c
     472  316 - #af8e7c
     244  320 - #d47d39
     592  320 - #8a9ebe
     396  324 - #a96f4c
     332  328 - #a86e4b
     104  332 - #8a9ebe
     392  336 - #a96f4c
     252  344 - #ca7f4f
     332  344 - #a86e4b
     404  344 - #d47d39
     448  352 - #d47d38
     220  360 - #d47c38
     392  364 - #a86f4b
       4  368 - #8a9ebe
     428  376 - #d47d38
     544  380 - #8a9ebe
     396  384 - #a86e4a
     468  392 - #d47c37
     220  396 - #9c959c
     224  396 - #9c959c
     228  396 - #9c959c
     232  396 - #9c959c
     236  396 - #9c959c
     240  396 - #9c959c
     244  396 - #9c959c
     248  396 - #9c959c
     400  404 - #a86e4a
     428  408 - #d47c37
     392  424 - #a86e49
     468  424 - #d47c37
      60  436 - #899dbe
     392  444 - #a86d48
     444  444 - #d37c36
     592  444 - #8a9dbe
     148  456 - #899dbe
     468  456 - #d37c36
     308  460 - #899dbe
     400  460 - #a86d47
     396  480 - #a86c46
     468  484 - #ae8d7a
     428  488 - #d37c35
     468  496 - #9c959c
     228  500 - #899dbe
     392  500 - #a76c45
     468  500 - #9c959c
       4  504 - #899dbe
      80  508 - #899dbe
     396  516 - #a76c45
     548  520 - #899dbe
     464  528 - #d37c34
     312  532 - #899dbe
     404  532 - #ae8c79
     412  532 - #ae8c79
     420  532 - #ae8c79
     432  532 - #ae8c79
     444  532 - #ae8d79
     456  532 - #ae8d79
     140  552 - #899dbe
       4  592 - #899dbe
     272  592 - #899dbe
     352  592 - #899dbe
     512  592 - #899dbe
     592  592 - #899dbe
";
