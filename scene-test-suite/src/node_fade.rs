use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{Camera, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, SceneTestView, scene},
    ui::{Color, Label, ViewData, ViewFrame, ViewSubviews},
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

const LABEL_WIDTH: f32 = 120.0;
const LABEL_HEIGHT: f32 = 26.0;
const LABEL_RISE: f32 = 0.95;
const BLUE: &str = "#3a7fd0";
const RED: &str = "#e04848";
/// The fades of the top row, each with its place and its label.
const ROW: [(f32, f32, &str); 5] = [
    (0.0, -3.2, "0"),
    (0.25, -1.6, "1/4"),
    (0.5, 0.0, "1/2"),
    (0.75, 1.6, "3/4"),
    (1.0, 3.2, "1"),
];
const ROW_Y: f32 = 4.4;
/// The cross fade pair and the cube that fades in over time.
const PAIR: (f32, f32, &str) = (-1.6, 1.6, "cross fade");
const TIMED: (f32, f32, &str) = (1.6, 1.6, "fade in");
/// Seconds the timed cube takes, and scene steps the test waits at most.
const FADE_TIME: f32 = 0.3;
const MAX_WAITS: usize = 600;

/// Blue cubes in front of an orange wall. The top row fades from nothing
/// on the left to whole on the right, each leaving out its pixels in a 4
/// by 4 dither, so the wall shows through the gaps. Below on the left a
/// blue cube at a half and a red flipped cube at a half stand at the same
/// place, a cross fade: every pixel of the cube is blue or red and none
/// shows the wall. Below on the right a cube starts at nothing, then fades
/// in over 0.3 s of scene time and stays whole.
#[scene]
#[derive(Default)]
struct NodeFade {
    timed: Weak<Prop>,
}

impl NodeFade {
    fn cube(&mut self, x: f32, y: f32, color: &str) -> Weak<Prop> {
        let mut cube = self.make_node::<Prop>(Shape3::cube(1.0), Vec3::new(x, y, 0.0));
        cube.set_color(Color::hex(color)).set_roughness(0.7);
        cube
    }
}

impl SceneSetup for NodeFade {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 3.0, 7.4),
            target: Vec3::new(0.0, 3.0, 0.0),
            ..Camera::default()
        };
        self.make_node::<Prop>(Shape3::Plane(16.0), Vec3::ZERO)
            .set_color(Color::hex("#b0b8c0"))
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::cuboid(13.0, 8.0, 0.2), Vec3::new(0.0, 4.0, -2.0))
            .set_color(Color::hex("#d35400"))
            .set_roughness(0.8);

        for (fade, x, _) in ROW {
            self.cube(x, ROW_Y, BLUE).set_fade(fade);
        }

        self.cube(PAIR.0, PAIR.1, BLUE).set_fade(0.5);
        self.cube(PAIR.0, PAIR.1, RED).set_fade(0.5).set_fade_flip(true);

        let mut timed = self.cube(TIMED.0, TIMED.1, BLUE);
        timed.set_fade(0.0);
        self.timed = timed;
    }
}

fn label(view: Weak<SceneTestView>, scene: Weak<NodeFade>, (x, y, text): (f32, f32, &str)) {
    let mut label = view.add_view::<Label>();
    label.set_text(text).set_text_size(16);
    label.set_text_color(Color::hex("#ffffff"));
    label
        .set_color(Color::hex("#303848"))
        .set_frame((0.0, 0.0, LABEL_WIDTH, LABEL_HEIGHT));
    if let Some(point) = scene.view_point(Vec3::new(x, y + LABEL_RISE, 0.0)) {
        label.set_center(point);
    }
}

impl SceneTest for NodeFade {
    fn overlay(scene: Weak<Self>, view: Weak<SceneTestView>) {
        for (_, x, text) in ROW {
            label(view, scene, (x, ROW_Y, text));
        }
        label(view, scene, PAIR);
        label(view, scene, TIMED);
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        check_colors(ROWS)?;

        from_main(move || {
            let mut timed = scene.timed;
            timed.fade_to(1.0, FADE_TIME);
        });
        let mut waits = 0;
        while from_main(move || scene.timed.is_fading()) {
            ensure!(waits < MAX_WAITS, "the fade still runs after {MAX_WAITS} frames");
            wait_for_next_frame();
            waits += 1;
        }
        let fade = from_main(move || scene.timed.material.fade);
        ensure!(
            (fade - 1.0).abs() < f32::EPSILON,
            "the fade ended at {fade}, not 1"
        );
        checkpoint("faded in")?;
        wait_for_next_frame();
        check_colors(FADED_IN)
    }
}

/// The row from nothing to whole, the cross fade pair, and the timed cube
/// at nothing.
const ROWS: &str = r"
     220    4 - #597c95
     312    4 - #597c95
     432    4 - #597c95
     592    4 - #597c95
     472   12 - #597c95
     552   12 - #597c95
     352   16 - #597c95
     272   24 - #ae4e28
      40  124 - #303848
     248  124 - #303848
     352  124 - #303848
     384  124 - #303848
     448  124 - #303848
     584  124 - #6c4339
      76  132 - #303848
     180  132 - #fefefe
     192  132 - #303848
     292  132 - #ffffff
     416  132 - #303848
     524  132 - #a2a5ac
     584  132 - #6c4339
      72  136 - #fefefe
     180  136 - #fefefe
     292  136 - #ffffff
     484  136 - #303848
     524  136 - #a2a5ac
     584  136 - #6c4339
     192  140 - #303848
     300  140 - #303848
     308  140 - #979ba3
     412  140 - #303848
     420  140 - #303848
     584  140 - #6c4339
      16  144 - #303848
     128  144 - #303848
     212  144 - #303848
     584  144 - #6c4339
     456  160 - #3f6dac
     500  160 - #274772
     380  164 - #274772
     524  168 - #3e6dac
     376  172 - #274772
     412  172 - #3e6dac
     576  172 - #3e6dac
     480  176 - #274772
     376  180 - #274772
     380  184 - #274772
     500  184 - #274772
     552  184 - #3e6dac
     376  188 - #274772
     440  188 - #3e6dac
     380  196 - #274772
     524  200 - #3e6dac
     576  200 - #3d6dac
     376  204 - #274772
     412  208 - #3d6dac
     480  208 - #274772
     380  212 - #274772
     376  216 - #274772
     380  220 - #274772
     376  224 - #274772
     500  224 - #274772
     380  228 - #274772
     528  228 - #3d6cab
     576  228 - #3d6cab
     452  232 - #2d5080
     260  236 - #ad4c24
     552  236 - #274772
      92  244 - #ad4c23
       4  304 - #ad4b22
     592  316 - #ad4c24
     128  320 - #303848
     352  320 - #704235
     468  320 - #303848
     352  324 - #704235
     172  328 - #303848
     180  328 - #303848
     352  328 - #704235
     160  332 - #ffffff
     196  332 - #b8bac0
     204  332 - #fdfdfd
     212  332 - #303848
     220  332 - #eaebec
     352  332 - #704235
     400  332 - #ffffff
     408  332 - #535a67
     416  332 - #eaebec
     428  332 - #595f6c
     172  336 - #303848
     352  336 - #704235
     392  336 - #303848
     352  340 - #704235
      68  384 - #ad4b22
     592  408 - #ad4c23
     492  416 - #ad4b23
     380  432 - #ad4b22
     280  456 - #ad4b22
       4  464 - #ad4b21
     152  468 - #a8afb7
     548  500 - #a8afb7
     444  504 - #a8afb7
     348  516 - #a8afb7
     208  548 - #a8afb7
       4  584 - #a8afb7
     492  588 - #a8afb7
     120  592 - #a8afb6
     296  592 - #a8afb6
     396  592 - #a8afb7
     592  592 - #a8afb7
";

/// The timed cube whole.
const FADED_IN: &str = r"
     220    4 - #597c95
     312    4 - #597c95
     432    4 - #597c95
     592    4 - #597c95
     472   12 - #597c95
     552   12 - #597c95
     352   16 - #597c95
     272   24 - #ae4e28
      40  124 - #303848
     248  124 - #303848
     352  124 - #303848
     384  124 - #303848
     448  124 - #303848
     584  124 - #6c4339
      76  132 - #303848
     180  132 - #fefefe
     192  132 - #303848
     292  132 - #ffffff
     416  132 - #303848
     524  132 - #a2a5ac
     584  132 - #6c4339
      72  136 - #fefefe
     180  136 - #fefefe
     292  136 - #ffffff
     524  136 - #a2a5ac
     584  136 - #6c4339
     192  140 - #303848
     300  140 - #303848
     308  140 - #979ba3
     412  140 - #303848
     420  140 - #303848
     484  140 - #303848
     584  140 - #6c4339
      16  144 - #303848
     128  144 - #303848
     212  144 - #303848
     584  144 - #6c4339
     380  164 - #274772
     500  164 - #274772
     552  164 - #3e6dac
     376  172 - #274772
     436  176 - #3e6dac
     376  180 - #274772
     480  180 - #274772
     380  184 - #274772
     376  188 - #274772
     528  192 - #3e6dac
     380  196 - #274772
     376  204 - #274772
     496  208 - #274772
     576  208 - #3d6dab
     380  212 - #274772
     376  216 - #274772
     380  220 - #274772
     436  220 - #3d6dab
     376  224 - #274772
     380  228 - #274772
     516  232 - #2d5080
     564  232 - #2d5080
     260  236 - #ad4c24
      92  244 - #ad4c23
     176  244 - #ad4c24
       4  304 - #ad4b22
     592  316 - #ad4c24
     128  320 - #303848
     352  320 - #704235
     468  320 - #303848
     352  324 - #704235
     172  328 - #303848
     180  328 - #303848
     352  328 - #704235
     160  332 - #ffffff
     196  332 - #b8bac0
     204  332 - #fdfdfd
     212  332 - #303848
     220  332 - #eaebec
     352  332 - #704235
     400  332 - #ffffff
     408  332 - #535a67
     416  332 - #eaebec
     428  332 - #595f6c
     172  336 - #303848
     352  336 - #704235
     392  336 - #303848
     352  340 - #704235
     456  368 - #3b6bab
     420  380 - #3a6bab
      68  384 - #ad4b22
     284  388 - #ad4b22
     456  408 - #3a6bab
     592  408 - #ad4c23
     408  412 - #3a6bab
     384  440 - #3a6baa
     428  440 - #3a6bab
       4  464 - #ad4b21
     248  464 - #ad4b22
     128  488 - #a8afb7
     592  492 - #a8afb7
     512  508 - #a8afb7
     396  520 - #a8afb7
       4  544 - #a8afb7
     320  556 - #a8afb7
     512  588 - #a8afb7
      64  592 - #a8afb6
     204  592 - #a8afb6
     432  592 - #a8afb7
     592  592 - #a8afb7
";
