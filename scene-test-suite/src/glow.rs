use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, SceneTestView, Sky, Sun, scene,
    },
    ui::{Color, Label, ViewData, ViewFrame, ViewSubviews},
    ui_test::{check_colors, set_record_probe_count},
};

const FLAME: &str = "#ff9020";
const LABEL_WIDTH: f32 = 70.0;
const LABEL_HEIGHT: f32 = 26.0;
/// Where each flame stands on x and how much it glows, left to right.
const FLAMES: [(f32, f32); 3] = [(-2.5, 0.0), (0.0, 0.5), (2.5, 1.0)];

/// 3 orange flames at night, no sun and a dim ambient, glowing by 0,
/// 0.5 and 1 of their own color, each labeled with its glow. The glow
/// shows the color whatever the light, 1 the full color.
/// Then the left flame is turned to full glow.
#[scene]
#[derive(Default)]
struct Glow {
    flames: Vec<Weak<Prop>>,
}

impl SceneSetup for Glow {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 1.6, 7.0),
            target: Vec3::new(0.0, 0.9, 0.0),
            ..Camera::default()
        };
        self.sun = Sun {
            intensity: 0.0,
            ..Sun::default()
        };
        self.ambient = Color::hex("#101418");
        self.sky = Some(Sky::gradient(
            Color::hex("#0b1026"),
            Color::hex("#1c2440"),
            Color::hex("#0a0a0a"),
        ));
        self.make_node::<Prop>(Shape3::Plane(20.0), Vec3::ZERO)
            .set_color(Color::hex("#808080"))
            .set_roughness(0.9);
        for (x, glow) in FLAMES {
            let mut flame = self.make_node::<Prop>(Shape3::cuboid(0.6, 1.6, 0.6), Vec3::new(x, 0.8, 0.0));
            flame.set_color(Color::hex(FLAME)).set_roughness(0.8).set_emissive(glow);
            self.flames.push(flame);
        }
    }
}

impl SceneTest for Glow {
    fn overlay(scene: Weak<Self>, view: Weak<SceneTestView>) {
        for (x, glow) in FLAMES {
            let label = view.add_view::<Label>();
            label.set_text(format!("glow {glow}")).set_text_size(16);
            label.set_text_color(Color::hex("#ffffff"));
            label
                .set_color(Color::hex("#303848"))
                .set_frame((0.0, 0.0, LABEL_WIDTH, LABEL_HEIGHT));
            if let Some(point) = scene.view_point(Vec3::new(x, 2.2, 0.0)) {
                label.set_center(point);
            }
        }
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);

        wait_for_next_frame();
        check_colors(STEPS)?;

        from_main(move || {
            let mut flame = scene.flames[0];
            flame.set_emissive(1.0);
        });
        wait_for_next_frame();
        check_colors(ALL_GLOWING)
    }
}

/// The flames at glow 0, 0.5 and 1.
const STEPS: &str = r"
       4    4 - #181f3a
     384    4 - #171e38
     592    4 - #181f3a
     196    8 - #171f39
     100   76 - #19213c
     488   76 - #19213c
     292   80 - #19213c
       4  120 - #1b223e
     392  120 - #1a223d
     592  124 - #1b223e
      80  192 - #303848
     124  192 - #303848
     144  192 - #303848
     456  192 - #303848
     492  192 - #303848
     280  200 - #e6e7e9
     312  200 - #303848
     476  200 - #686e7a
     508  200 - #6d737e
      92  204 - #303848
      96  204 - #6c727d
     104  204 - #303848
     280  204 - #e6e7e9
     284  204 - #fdfdfe
     288  204 - #303848
     312  204 - #303848
     324  204 - #303848
     328  204 - #fefefe
     468  204 - #303848
     476  204 - #686e7a
     508  204 - #6d737e
      80  212 - #303848
     124  212 - #303848
     144  212 - #303848
     492  212 - #303848
     280  248 - #bd6a17
     320  248 - #bd6a17
     500  248 - #f18b30
     460  256 - #f18b30
     308  284 - #bd6a17
     480  284 - #f18b30
     512  288 - #f18b30
     280  292 - #bd6a17
     456  308 - #f18b30
     304  312 - #bd6a17
     492  312 - #f18b30
       4  320 - #050714
     280  324 - #bd6a17
     320  332 - #bd6a17
     476  336 - #f18b30
     512  340 - #f18b30
     280  352 - #bd6a17
     300  368 - #bd6a17
     468  368 - #f18b30
     500  368 - #f18b30
      96  412 - #050714
     592  452 - #050714
     216  464 - #050714
       4  476 - #050714
     120  544 - #050714
     412  544 - #050714
       4  592 - #050714
     236  592 - #050714
     592  592 - #050714
";

/// The left flame turned to full glow too.
const ALL_GLOWING: &str = r"
       4    4 - #181f3a
     384    4 - #171e38
     592    4 - #181f3a
     196   12 - #171f39
      96   76 - #19213c
     488   76 - #19213c
     292   80 - #19213c
     392  112 - #1a223d
       4  124 - #1b233e
     592  124 - #1b223e
      80  192 - #303848
     112  192 - #303848
     136  192 - #303848
     456  192 - #303848
     492  192 - #303848
     280  200 - #e6e7e9
     312  200 - #303848
     476  200 - #686e7a
     508  200 - #6d737e
      92  204 - #303848
      96  204 - #6c727d
     104  204 - #303848
     280  204 - #e6e7e9
     284  204 - #fdfdfe
     288  204 - #303848
     312  204 - #303848
     324  204 - #303848
     328  204 - #fefefe
     468  204 - #303848
     476  204 - #686e7a
     508  204 - #6d737e
     124  212 - #303848
     144  212 - #303848
     300  212 - #303848
     492  212 - #303848
     280  248 - #bd6a17
     468  252 - #f18b30
     320  256 - #bd6a17
      84  264 - #f18b30
     516  272 - #f18b30
     280  288 - #bd6a17
     316  288 - #bd6a17
     140  292 - #f18b30
     456  296 - #f18b30
     292  312 - #bd6a17
     320  320 - #bd6a17
     500  320 - #f18b30
     280  336 - #bd6a17
     144  348 - #f18b30
      88  356 - #f18b30
     456  356 - #f18b30
     280  364 - #bd6a17
     308  368 - #bd6a17
     512  368 - #f18b30
     388  432 - #050714
       4  456 - #050714
     592  464 - #050714
     240  468 - #050714
     124  500 - #050714
     416  528 - #050714
       4  592 - #050714
     240  592 - #050714
     340  592 - #050714
     592  592 - #050714
";
