use std::f32::consts::{FRAC_PI_2, PI};

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, Node, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, SceneTestView, Sky, scene,
    },
    ui::{Color, Label, ViewData, ViewFrame, ViewSubviews},
    ui_test::{check_colors, set_record_probe_count},
};

/// Label sizes in points.
const LABEL_WIDTH: f32 = 90.0;
const LABEL_HEIGHT: f32 = 26.0;
/// The labels float this far over the middle of their node.
const LIFT: f32 = 1.4;
/// Radians per second the ball circles the post while the clock runs.
const CIRCLE_SPEED: f32 = 0.6;
const CIRCLE_RADIUS: f32 = 5.0;

/// A ball circling a post on a field, each with a label over it, the
/// way a game hangs a name or a damage number over a node. The labels
/// are plain views placed every step at `view_point` of a spot above
/// their node, so they follow the ball and stay over the post. The test
/// stops the ball on one side, then the other, then turns the camera
/// away so the post is behind it and its label hides.
#[scene]
#[derive(Default)]
struct ViewPoints {
    ball:       Weak<Prop>,
    post:       Weak<Prop>,
    ball_label: Weak<Label>,
    post_label: Weak<Label>,
    /// Where on its circle the ball is, radians.
    angle:      f32,
    stopped:    bool,
}

impl SceneSetup for ViewPoints {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 6.0, 11.0),
            target: Vec3::new(0.0, 1.0, 0.0),
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.make_node::<Prop>(Shape3::Plane(40.0), Vec3::ZERO)
            .set_color(Color::hex("#6f8f5a"))
            .set_roughness(0.9);

        self.post = self.make_node::<Prop>(Shape3::cuboid(0.6, 2.4, 0.6), Vec3::new(0.0, 1.2, 0.0));
        self.post.set_color(Color::hex("#c0703a")).set_roughness(0.7);
        self.ball = self.make_node::<Prop>(Shape3::Ball(0.7), Vec3::new(CIRCLE_RADIUS, 0.7, 0.0));
        self.ball.set_color(Color::hex("#2f6fb7")).set_roughness(0.4);
    }

    fn update(&mut self, dt: f32) {
        if !self.stopped {
            self.angle += CIRCLE_SPEED * dt;
        }
        self.place();
    }
}

impl ViewPoints {
    /// Moves the ball to its angle and every label over its node, hidden
    /// while the node is behind the camera.
    fn place(&mut self) {
        if self.ball_label.is_null() {
            return;
        }
        let ball = Vec3::new(self.angle.cos(), 0.0, -self.angle.sin()) * CIRCLE_RADIUS + Vec3::Y * 0.7;
        self.ball.set_position(ball);
        for (mut label, node) in [(self.ball_label, ball), (self.post_label, self.post.position())] {
            let point = self.view_point(node + Vec3::Y * LIFT);
            label.set_hidden(point.is_none());
            if let Some(point) = point {
                label.set_center(point);
            }
        }
    }

    fn set_angle(&mut self, angle: f32) {
        self.stopped = true;
        self.angle = angle;
        self.place();
    }
}

fn label(view: Weak<SceneTestView>, text: &str, color: &str) -> Weak<Label> {
    let label = view.add_view::<Label>();
    label.set_text(text).set_text_size(16).set_text_color(Color::hex("#ffffff"));
    label
        .set_color(Color::hex(color))
        .set_corner_radius(6)
        .set_frame((0.0, 0.0, LABEL_WIDTH, LABEL_HEIGHT));
    label
}

impl SceneTest for ViewPoints {
    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        scene.ball_label = label(view, "ball", "#1d3f6e");
        scene.post_label = label(view, "post", "#7a3f18");
        scene.place();
    }

    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);

        from_main(move || scene.set_angle(0.0));
        wait_for_next_frame();
        check_colors(BALL_RIGHT)?;

        from_main(move || scene.set_angle(PI));
        wait_for_next_frame();
        check_colors(BALL_LEFT)?;

        // Looking back over the field with the post behind the camera, so
        // only the ball keeps its label.
        from_main(move || {
            scene.camera = Camera {
                position: Vec3::new(0.0, 2.5, 1.0),
                target: Vec3::new(0.0, 0.7, 12.0),
                ..Camera::default()
            };
            scene.set_angle(-FRAC_PI_2);
        });
        wait_for_next_frame();
        let hidden = from_main(move || (scene.post_label.is_hidden(), scene.ball_label.is_hidden()));
        ensure!(
            hidden == (true, false),
            "post label hidden, ball label shown, got {hidden:?}"
        );
        check_colors(POST_BEHIND)
    }
}

const BALL_RIGHT: &str = r"
       4    4 - #d3dfec
     300    4 - #d3deeb
     592    4 - #d3dfec
     152   12 - #d4dfec
     444   92 - #d5e0eb
     228  108 - #d4dfeb
     592  128 - #d4dfea
      96  152 - #d3dde9
     300  224 - #7a3f18
     256  228 - #7a3f18
     344  228 - #7a3f18
     316  232 - #7a3f18
     288  236 - #7a3f18
     268  244 - #7a3f18
     304  244 - #7a3f18
     332  244 - #7a3f18
     488  244 - #1d3f6e
     548  244 - #1d3f6e
     520  248 - #1d3f6e
     524  248 - #1d3f6e
     512  252 - #ffffff
     512  256 - #ffffff
     516  256 - #1d3f6e
     496  260 - #1d3f6e
     312  264 - #b16f47
     480  264 - #1d3f6e
     540  264 - #1d3f6e
     564  264 - #1d3f6e
     288  268 - #b16f47
     504  284 - #517cd0
     308  288 - #b16f46
     520  288 - #3879db
     288  292 - #b16f46
     492  296 - #3967b8
       4  300 - #6d936f
     512  300 - #3976d3
     532  300 - #4482e0
     312  308 - #b16f46
     484  308 - #3f63a4
     500  308 - #366bbe
     544  308 - #608fdf
     492  312 - #3560a8
     520  312 - #417ad1
     484  320 - #43629b
     508  320 - #386bb7
     288  324 - #b16f46
     488  324 - #315793
     496  324 - #2b5697
     516  324 - #376bb8
     536  324 - #3e74c2
     492  328 - #2b538f
     504  328 - #2a5799
     512  332 - #285697
     500  336 - #2d4f84
     524  336 - #2f5792
     312  340 - #a07850
     516  340 - #3e5886
     140  384 - #6d926f
       8  444 - #6c926f
     408  448 - #6c926f
     592  520 - #6d926f
     208  528 - #6c926f
       4  592 - #6c926f
     412  592 - #6c926f
";

const BALL_LEFT: &str = r"
     168    4 - #d3dfec
     592    4 - #d3dfec
     380   16 - #d4dfec
       4   72 - #d5e0ec
     268   92 - #d5e0eb
     120  120 - #d4dfea
     512  164 - #d2dce8
     300  224 - #7a3f18
     256  228 - #7a3f18
     344  228 - #7a3f18
     316  232 - #7a3f18
     288  236 - #7a3f18
      48  244 - #1d3f6e
     104  244 - #1d3f6e
     268  244 - #7a3f18
     304  244 - #7a3f18
     332  244 - #7a3f18
      76  248 - #1d3f6e
     116  248 - #1d3f6e
      84  252 - #486389
      68  256 - #1d3f6e
      80  256 - #6b81a0
      84  256 - #486389
      32  260 - #1d3f6e
     120  260 - #1d3f6e
      52  264 - #1d3f6e
     100  264 - #1d3f6e
     312  264 - #b16f47
     288  268 - #b16f47
      88  284 - #447ddb
     308  288 - #b16f46
      68  292 - #3d78d6
     104  292 - #407cd8
     288  292 - #b16f46
      92  304 - #5f8fe5
     116  304 - #557ec6
      76  308 - #3f7dd8
     104  308 - #3f7bd2
     312  308 - #b16f46
      56  316 - #4d73b8
     116  316 - #486eaf
      60  324 - #3f68aa
      72  324 - #396fbd
      96  324 - #376fbd
     112  324 - #38609f
     288  324 - #b16f46
     592  324 - #6d9370
      80  328 - #346ab5
     108  328 - #305b99
      68  332 - #325a98
      92  332 - #2c5fa3
     100  332 - #2a5898
     104  332 - #2d548e
      76  336 - #2b518b
      88  336 - #27538f
      84  340 - #3d5786
      88  340 - #3f5885
     312  340 - #a07850
     456  388 - #6d926f
     200  444 - #6c926f
     396  528 - #6c926f
       4  532 - #6c926f
     200  592 - #6c926f
     592  592 - #6c926f
";

const POST_BEHIND: &str = r"
       4    4 - #c1d0e8
     328    4 - #bdcde8
     592    4 - #c1d0e8
     164  100 - #cdd9ea
     492  168 - #d3dfec
     268  256 - #1d3f6e
     324  256 - #1d3f6e
     288  264 - #adbacb
     292  268 - #1d3f6e
     256  272 - #1d3f6e
     276  280 - #a8b9cc
     296  280 - #a8b9cc
     312  280 - #a8b9cc
     324  280 - #a8b9cc
     336  280 - #a8b9cc
       4  284 - #759877
     592  328 - #6f9471
     300  352 - #7298ae
     276  356 - #769dec
     292  356 - #5c8ae2
     316  356 - #618ce1
     268  364 - #5f8be1
     340  368 - #4e7acf
     256  372 - #5584dc
     276  372 - #aec0f3
     280  372 - #afc0f3
     300  372 - #4574cd
     276  376 - #aebff2
     280  376 - #a9bcf1
     324  376 - #2f67c2
     264  380 - #678bda
     356  380 - #5077c6
     236  388 - #5584d8
     300  396 - #2b61b9
     220  408 - #6e94dc
     260  412 - #3064b8
     376  412 - #4d6caf
     336  420 - #2956a5
     216  424 - #6c8ed2
     384  428 - #6881b9
     240  432 - #3664b0
      56  440 - #709572
     300  440 - #2d58a4
     384  444 - #5c77b1
     256  452 - #335ba2
     216  460 - #5e79b1
     380  464 - #4e6ca6
     332  468 - #315a9f
     376  476 - #46659f
     224  484 - #5b74a5
     360  500 - #3b5c94
     248  504 - #305590
     268  508 - #27518f
     288  508 - #265190
     296  508 - #265190
     304  508 - #265190
     348  508 - #32568f
     332  512 - #29518d
     276  520 - #2b5089
     336  520 - #3d5a8b
     316  524 - #2e5187
     296  528 - #365385
       4  592 - #6f9471
     592  592 - #6d9370
";
