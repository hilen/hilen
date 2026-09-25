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
            let mut label = view.add_view::<Label>();
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
     488   76 - #19213c
     100   80 - #1a213c
     292   80 - #19213c
     592  124 - #1b223e
     124  192 - #303848
     144  192 - #303848
     280  200 - #e9eaec
     312  200 - #303848
     476  200 - #6a6f7b
     508  200 - #707681
      92  204 - #303848
      96  204 - #6a707c
     104  204 - #303848
     280  204 - #e9eaec
     284  204 - #fcfcfc
     288  204 - #303848
     312  204 - #303848
     324  204 - #303848
     328  204 - #fefefe
     468  204 - #303848
     476  204 - #6a6f7b
     508  204 - #707681
      80  212 - #303848
     124  212 - #303848
     144  212 - #303848
     300  248 - #bd6a17
     492  248 - #f18b30
     276  252 - #443636
     320  264 - #bd6a17
     456  268 - #513e3c
     276  276 - #443636
     456  276 - #875838
     456  280 - #875838
     488  280 - #f18b30
     456  284 - #bc7134
     516  284 - #bc7134
     456  288 - #bc7134
     516  288 - #875838
     308  292 - #bd6a17
     276  300 - #332015
     276  304 - #332015
     484  308 - #f18b30
     320  320 - #bd6a17
       4  324 - #050714
     292  324 - #bd6a17
     456  332 - #f18b30
     512  332 - #f18b30
     484  340 - #f18b30
     308  344 - #bd6a17
     280  348 - #bd6a17
     512  364 - #f18b30
     320  368 - #bd6a17
     480  368 - #f18b30
     104  416 - #050714
     592  452 - #050714
     416  536 - #050714
     124  544 - #050714
       4  592 - #050714
     244  592 - #050714
     592  592 - #050714
";

/// The left flame turned to full glow too.
const ALL_GLOWING: &str = r"
       4    4 - #181f3a
     372    4 - #171e38
     592    4 - #181f3a
     188   12 - #181f39
     480   68 - #19213b
       4  124 - #1b233e
     144  192 - #303848
     456  192 - #303848
     280  200 - #e9eaec
     312  200 - #303848
     476  200 - #6a6f7b
     508  200 - #707681
      96  204 - #6a707c
     104  204 - #303848
     280  204 - #e9eaec
     284  204 - #fcfcfc
     288  204 - #303848
     312  204 - #303848
     324  204 - #303848
     328  204 - #fefefe
     468  204 - #303848
     476  204 - #6a6f7b
     508  204 - #707681
     124  212 - #303848
     144  212 - #303848
     128  248 - #f18b30
     276  252 - #443636
      84  260 - #f18b30
     320  264 - #bd6a17
     456  268 - #513e3c
     276  276 - #443636
     456  276 - #875838
     456  280 - #875838
     456  284 - #bc7134
     516  284 - #bc7134
     456  288 - #bc7134
     516  288 - #875838
     308  292 - #bd6a17
     144  300 - #40281b
     276  300 - #332015
     144  304 - #40281b
     276  304 - #332015
      84  308 - #b66a29
     144  308 - #7b4922
      84  312 - #7b4922
     144  312 - #7b4922
     144  316 - #b66a29
      84  320 - #40281b
     144  320 - #b66a29
     320  320 - #bd6a17
     476  324 - #f18b30
     308  344 - #bd6a17
     280  348 - #bd6a17
     512  348 - #f18b30
     456  360 - #f18b30
     108  368 - #f18b30
     320  368 - #bd6a17
       4  456 - #050714
     476  480 - #050714
     332  496 - #050714
     592  520 - #050714
     212  548 - #050714
       4  592 - #050714
     416  592 - #050714
";
