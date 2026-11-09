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
     196    4 - #b0c3e6
     312    4 - #afc2e5
     416    4 - #b1c3e6
     592    4 - #b7c8e6
     100   28 - #b9c9e6
     544   84 - #c2cfe2
     452  100 - #c3cfe1
     156  112 - #c4cfe0
     296  136 - #c4cfdd
       4  168 - #c4cdd7
     384  168 - #c4ced9
     592  168 - #c4cdd7
     500  180 - #c3ccd6
     108  216 - #c0c8d0
     224  264 - #d08a62
     260  264 - #c5a9a0
     272  264 - #c5a9a0
     396  264 - #a96f4e
     320  268 - #bfb5b5
     332  268 - #af958c
     472  268 - #d47e3b
     432  272 - #d47d3b
     272  276 - #c5a9a0
     332  276 - #af958c
     400  276 - #a96f4d
     320  280 - #bfb5b5
     332  280 - #af958c
     260  284 - #c5a9a0
     332  284 - #af958c
     272  288 - #c5a9a0
     332  288 - #af958c
     392  288 - #a96f4d
     320  292 - #bfb6b5
     332  292 - #af958c
     468  292 - #d47d3a
     220  296 - #d08a62
     268  296 - #c5a9a0
     332  296 - #af958c
     300  300 - #b9c0c6
     332  300 - #af958c
     424  300 - #d47d3a
     272  304 - #c5a9a0
     320  304 - #bfb6b5
     332  304 - #af958c
     260  308 - #c5a9a0
     332  308 - #af958c
     400  308 - #a96f4c
     448  308 - #d47d3a
     332  312 - #af958c
     272  316 - #c5a9a0
     332  316 - #af958c
     232  320 - #d08a62
     332  320 - #af958c
     332  324 - #af958c
      60  328 - #adb7c4
     120  328 - #abb5c3
     260  328 - #c4a9a0
     272  328 - #c4a9a0
     332  328 - #af958c
     396  328 - #a96f4c
     528  328 - #acb6c4
       4  332 - #acb6c4
     592  332 - #acb6c4
     332  336 - #af958c
     428  336 - #d47d39
     220  344 - #d08a63
     332  348 - #b6978b
     392  348 - #a96f4c
     468  348 - #d47d39
     400  360 - #a96f4b
     164  364 - #9ba9c1
     252  368 - #c78e71
     432  368 - #d47d38
     392  372 - #a86f4b
     460  376 - #d47d38
      64  384 - #97a7c0
     220  388 - #cf8b63
     420  388 - #d47c37
     248  392 - #cf8a63
     400  392 - #a86e4a
     540  400 - #95a5c0
     452  404 - #d47c37
     392  408 - #a86e49
     424  408 - #d47c37
     136  416 - #91a2bf
     592  416 - #93a4c0
       4  420 - #92a3bf
     392  428 - #a86e48
     468  428 - #d47c36
     412  432 - #d47c36
     432  436 - #d47c36
     248  440 - #8da0bf
     304  440 - #8da0bf
     192  444 - #8da0bf
     392  448 - #a86d48
     468  456 - #d37c36
     392  460 - #a86d47
     404  460 - #d37c36
     440  460 - #d37c36
     532  460 - #8da0bf
      60  464 - #8d9fbe
     392  472 - #a86d46
     592  472 - #8da0bf
     412  480 - #d37c35
     464  484 - #d37c35
     436  488 - #d37c35
     392  492 - #a76c45
     416  500 - #d37c35
     196  508 - #899dbe
     120  512 - #899dbe
     396  512 - #a76c45
       4  516 - #8a9dbe
     276  516 - #899dbe
     432  520 - #d37c34
     464  520 - #d37c35
     404  528 - #d37b34
     532  528 - #899dbe
     336  536 - #899dbe
      64  552 - #899dbe
     592  552 - #899dbe
       4  592 - #899dbe
     120  592 - #899dbe
     208  592 - #899dbe
     292  592 - #899dbe
     368  592 - #899dbe
     448  592 - #899dbe
     524  592 - #899dbe
";

const FAR_FOG: &str = r"
       4    4 - #b7c8e6
     272    4 - #afc2e5
     376    4 - #b0c3e5
     488    4 - #b3c5e6
     592    4 - #b7c8e6
      72    8 - #b5c7e6
     140   20 - #b6c7e6
     324   60 - #bccce5
       4   72 - #c1cfe2
      72   80 - #c1cee2
     540   84 - #c2cfe2
     148   88 - #c1cee2
     432   88 - #c1cee2
     216   92 - #c1cfe2
     308  136 - #c4cfdd
       4  140 - #c5cedb
     136  156 - #c5ceda
     592  164 - #c4cdd7
      68  168 - #c4cdd7
     492  168 - #c4cdd8
     392  172 - #c4ced8
     232  176 - #c4cdd7
     328  200 - #c2cbd3
      24  224 - #bfc6ce
     168  224 - #bfc7ce
     532  228 - #bec6cd
     100  256 - #bac1c8
     232  264 - #d47d3a
     352  264 - #d47d3b
     400  264 - #a96f4e
     468  264 - #d47e3b
     260  268 - #d47d3a
     288  268 - #cc977f
     300  268 - #91aed5
     312  268 - #ce9070
     288  272 - #cc977f
     312  272 - #ce9070
     332  272 - #a96f4d
     392  272 - #a96f4d
     432  272 - #d47d3b
     288  276 - #cc977f
     312  276 - #ce9070
     288  280 - #cc977f
     292  280 - #8b9aae
     296  280 - #8998ac
     300  280 - #8999ad
     304  280 - #899ab0
     312  280 - #ce9070
     584  280 - #b9c0c6
     288  284 - #cc977f
     296  284 - #8c99a7
     300  284 - #8a97a5
     312  284 - #ce9070
     392  284 - #a96f4d
     288  288 - #cc977f
     312  288 - #ce9070
     332  292 - #a96f4d
     472  292 - #d47d3a
     248  296 - #d47d39
     400  296 - #a96f4d
       4  304 - #8ea1bf
     152  304 - #8a9ebe
     444  304 - #d47d3a
     396  308 - #a96f4c
      68  312 - #8a9ebe
     332  312 - #a86f4c
     220  316 - #d47d39
     392  320 - #a96f4c
     468  320 - #d47d39
     264  328 - #d47d38
     332  328 - #a86e4b
     428  328 - #d47d39
     400  332 - #a96f4c
     392  340 - #a96f4c
     540  340 - #8a9ebe
     332  344 - #a86e4b
     468  348 - #d47d39
     392  352 - #a96f4b
     224  356 - #d47c38
     112  360 - #8a9ebe
     252  360 - #ca7f4f
     400  360 - #a96f4b
     432  360 - #d47d38
      36  372 - #8a9ebe
     396  372 - #a86f4b
     220  384 - #d47c37
     392  384 - #a86e4a
     248  388 - #d47c37
     400  392 - #a86e4a
     452  396 - #d47c37
     592  396 - #8a9ebe
     392  400 - #a86e4a
     160  404 - #8a9dbe
     524  408 - #8a9ebe
     392  412 - #a86e49
     316  416 - #8a9dbe
     400  424 - #a86e49
     432  428 - #d47c36
     396  436 - #a86d48
     108  440 - #8a9dbe
     468  444 - #d37c36
      32  448 - #8a9dbe
     392  448 - #a86d48
     428  456 - #d37c36
     400  460 - #a86d47
     180  472 - #899dbe
     392  480 - #a86c46
     440  484 - #d37c35
     288  488 - #899dbe
     400  492 - #a76c46
     560  492 - #8a9dbe
     464  500 - #d37c35
     392  508 - #a76c45
     436  512 - #d37c35
       4  516 - #899dbe
      88  516 - #899dbe
     412  528 - #d37c34
     464  528 - #d37c34
     324  540 - #899dbe
     208  552 - #899dbe
      68  580 - #899dbe
       4  592 - #899dbe
     132  592 - #899dbe
     280  592 - #899dbe
     360  592 - #899dbe
     436  592 - #899dbe
     512  592 - #899dbe
     592  592 - #899dbe
";

const NO_FOG: &str = r"
       4    4 - #b7c8e7
     268    4 - #afc2e5
     372    4 - #b0c3e5
     488    4 - #b3c5e6
     592    4 - #b7c8e6
      68   16 - #b7c8e6
     136   16 - #b5c7e6
     204   16 - #b3c5e6
     284   68 - #bdcde8
     352   76 - #bfcfe8
      64   84 - #c3d2e9
     140   84 - #c2d1e8
     208   84 - #c1d0e8
     540   84 - #c3d2e9
     428   88 - #c2d1e9
     300  132 - #c9d6ea
     136  152 - #ced9ea
       4  164 - #d0dbeb
     492  164 - #cfdaeb
     592  164 - #d0dbeb
      72  172 - #d0dceb
     384  172 - #cfdbeb
     224  176 - #d0dceb
     320  200 - #d2ddeb
     536  224 - #d4dfec
      40  232 - #d4dfec
     176  232 - #d4dfec
     112  264 - #d5e0ec
     240  264 - #d47d3a
     300  264 - #4191e4
     400  264 - #a96f4e
     468  264 - #d47e3b
     296  268 - #2f86d4
     332  268 - #a96f4d
     292  272 - #3477b9
     296  272 - #3581c6
     300  272 - #3f8ad0
     304  272 - #3f8dd4
     308  272 - #3d8bd2
     392  272 - #a96f4d
     292  276 - #32689f
     296  276 - #3373ae
     300  276 - #357ab7
     304  276 - #377ebc
     308  276 - #3a7ebc
     432  276 - #d47d3b
     292  280 - #30618f
     296  280 - #245b8c
     300  280 - #235e90
     304  280 - #276396
     584  280 - #8b9ebf
     220  284 - #d47d39
     300  284 - #255278
     392  284 - #a96f4d
     332  288 - #a96f4d
     472  292 - #d47d3a
     260  296 - #d47d39
     400  296 - #a96f4d
       4  300 - #8a9ebe
     332  304 - #a96f4c
     156  308 - #8a9ebe
     392  308 - #a96f4c
     444  308 - #d47d3a
     220  312 - #d47d39
     400  320 - #a96f4c
      68  324 - #8a9ebe
     244  324 - #d47d38
     332  324 - #a86f4c
     468  324 - #d47d39
     272  328 - #d47d38
     392  332 - #a96f4c
     432  340 - #d47d39
     540  340 - #8a9ebe
     332  344 - #a86e4b
     392  344 - #a96f4c
     236  352 - #d47d38
     400  352 - #a96f4c
     468  352 - #d47d38
     392  360 - #a96f4b
     124  364 - #8a9ebe
     392  372 - #a86f4b
     424  372 - #d47d38
     468  380 - #d47d38
     252  384 - #ca7f4e
     400  384 - #a86e4a
       4  392 - #8a9ebe
     220  392 - #d47c37
     396  396 - #a86e4a
     448  396 - #d47c37
     592  396 - #8a9ebe
     524  404 - #8a9ebe
     400  408 - #a86e49
     296  412 - #8a9dbe
     468  412 - #d47c37
     392  420 - #a86e49
     132  428 - #8a9dbe
     432  428 - #d47c36
     392  432 - #a86e48
     400  440 - #a86d48
     468  440 - #d47c36
      72  448 - #8a9dbe
     392  460 - #a86d47
     424  460 - #d37c36
     468  468 - #d37c35
     400  472 - #a86d46
     172  476 - #899dbe
     392  484 - #a86c46
     444  484 - #d37c35
     280  488 - #899dbe
     560  492 - #8a9dbe
     112  496 - #899dbe
     400  500 - #a76c45
     464  500 - #d37c35
       4  504 - #899dbe
     396  516 - #a76c45
     432  516 - #d37c34
     216  528 - #899dbe
     464  528 - #d37c34
      56  548 - #899dbe
     300  552 - #899dbe
     124  560 - #899dbe
       4  592 - #899dbe
     176  592 - #899dbe
     240  592 - #899dbe
     356  592 - #899dbe
     428  592 - #899dbe
     512  592 - #899dbe
     592  592 - #899dbe
";
