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
        for (label, node) in [(self.ball_label, ball), (self.post_label, self.post.position())] {
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
     228  108 - #d5dfeb
     592  128 - #d4dfea
      96  152 - #d3dde9
     276  224 - #7a3f18
     308  224 - #7a3f18
     256  228 - #7a3f18
     344  228 - #7a3f18
     288  236 - #7a3f18
     268  244 - #7a3f18
     316  244 - #7a3f18
     336  244 - #7a3f18
     488  244 - #1d3f6e
     548  244 - #1d3f6e
     520  248 - #1d3f6e
     524  248 - #1d3f6e
     512  252 - #ffffff
     512  256 - #ffffff
     516  256 - #1d3f6e
     496  260 - #1d3f6e
     308  264 - #b16f47
     480  264 - #1d3f6e
     540  264 - #1d3f6e
     564  264 - #1d3f6e
     288  268 - #b16f47
     312  284 - #b16f46
     508  284 - #4678d2
     524  288 - #3d7cdd
     292  292 - #b16f46
     500  296 - #316cc6
       4  300 - #6d9370
     488  300 - #3c63ad
     512  300 - #3876d3
     528  304 - #6491e5
     540  304 - #4984de
     484  308 - #3f63a4
     544  308 - #608fdf
     288  312 - #b16f46
     508  312 - #3971c4
     312  316 - #b16f46
     496  316 - #3461a9
     484  320 - #43639b
     504  320 - #3666b0
     512  324 - #3568b3
     536  324 - #3e74c2
     492  328 - #2b538f
     496  328 - #285291
     500  328 - #285393
     288  336 - #b16f46
     500  336 - #2e5084
     520  336 - #29538e
     308  340 - #b16f45
     512  340 - #3d5784
     144  376 - #6d936f
      16  444 - #6d926f
     404  448 - #6c926f
     592  520 - #6d9370
     204  524 - #6c926f
       4  592 - #6c926f
     408  592 - #6c926f
";

const BALL_LEFT: &str = r"
     168    4 - #d3dfec
     592    4 - #d3dfec
     380   16 - #d4dfec
       4   72 - #d5e0ec
     268   92 - #d5e0eb
     120  120 - #d4dfea
     512  164 - #d2dce8
     276  224 - #7a3f18
     308  224 - #7a3f18
     256  228 - #7a3f18
     344  228 - #7a3f18
     288  236 - #7a3f18
      48  244 - #1d3f6e
     104  244 - #1d3f6e
     268  244 - #7a3f18
     316  244 - #7a3f18
     336  244 - #7a3f18
      76  248 - #1d3f6e
     116  248 - #1d3f6e
      80  252 - #1d3f6e
      84  252 - #435f86
      68  256 - #1d3f6e
      80  256 - #677e9e
      84  256 - #435f86
      32  260 - #1d3f6e
     120  260 - #1d3f6e
      52  264 - #1d3f6e
     100  264 - #1d3f6e
     308  264 - #b16f47
     288  268 - #b16f47
      88  284 - #447ddb
     312  284 - #b16f46
      68  292 - #3d78d6
     104  292 - #407cd8
     292  292 - #b16f46
      92  304 - #5f8fe5
     116  304 - #557ec5
      76  308 - #3f7dd8
     104  308 - #3f7bd2
     288  312 - #b16f46
      56  316 - #4d73b8
     116  316 - #486eaf
     312  316 - #b16f46
      60  324 - #3f68aa
      72  324 - #396fbc
      92  324 - #3870bf
     112  324 - #38609f
     592  324 - #6d9370
      80  328 - #346ab5
     108  328 - #305b99
      68  332 - #335a98
      96  332 - #2b5c9f
     100  332 - #2a5898
     104  332 - #2d548d
      76  336 - #2b528b
      84  340 - #3e5886
      88  340 - #405985
     308  340 - #b16f45
     452  388 - #6d936f
     200  444 - #6c926f
     396  528 - #6c926f
       4  532 - #6c926f
     200  592 - #6c926f
     592  592 - #6d9270
";

const POST_BEHIND: &str = r"
       4    4 - #c1d0e8
     328    4 - #bdcde8
     592    4 - #c1d0e8
     168  104 - #cedaea
     460  132 - #d1dceb
      28  144 - #d3deeb
     320  256 - #1d3f6e
     288  264 - #a5b3c6
     292  268 - #1d3f6e
     256  272 - #1d3f6e
     344  272 - #1d3f6e
     312  276 - #1d3f6e
       4  284 - #759877
     592  284 - #709472
     288  356 - #618de4
     304  356 - #5685de
     316  356 - #608be0
     268  360 - #6f98ea
     332  364 - #4b79d1
     256  368 - #628fe4
     276  372 - #aec0f3
     280  372 - #b0c1f3
     284  372 - #9fb5ef
     300  372 - #4675cd
     272  376 - #9fb4ef
     276  376 - #adbff2
     280  376 - #a9bcf1
     356  380 - #5077c5
     328  384 - #2b62bb
     244  388 - #4478cf
     224  400 - #7097e1
     288  400 - #2d62b9
     312  400 - #275db3
     372  400 - #5473b7
     220  408 - #6e94dc
     264  412 - #2f63b7
     380  416 - #5c78b5
     216  424 - #6c8fd2
     244  428 - #3564b1
     540  436 - #6e9471
      56  440 - #709572
     312  440 - #2e59a4
     380  444 - #4567a7
     260  448 - #325ba3
     216  460 - #5f7ab2
     344  460 - #325ba0
     288  472 - #305a9e
     380  472 - #5e77ab
     224  484 - #5b74a5
     372  484 - #45649c
     364  496 - #3f5f96
     244  504 - #385991
     284  508 - #265190
     292  508 - #265190
     300  508 - #265190
     312  508 - #265190
     268  512 - #28518d
     340  512 - #2e538c
     252  516 - #546a96
     332  516 - #2c518b
     292  528 - #385586
     320  528 - #4b618c
       4  592 - #6f9471
     592  592 - #6d9370
";
