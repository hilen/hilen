use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3, Vertex3D},
    refs::Weak,
    scene::{
        Camera, Line3, MeshData, Model, NodeTemplates, Projection, Prop, SceneCreation, SceneSetup,
        SceneTest, SceneTestView, scene,
    },
    ui::{Color, Label, Point, ViewData, ViewFrame, ViewSubviews},
    ui_test::{check_colors, set_record_probe_count},
};

const PART_AT: Vec3 = Vec3::new(1.8, 0.8, 0.0);
const BALL_AT: Vec3 = Vec3::new(-1.6, 0.9, 0.5);

/// A flat shaded cube of side 1.6 built in code, 4 vertices per face.
fn part_mesh() -> MeshData {
    let mut mesh = MeshData::default();
    for normal in [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z] {
        let u = normal.any_orthonormal_vector();
        let v = normal.cross(u);
        let first = u32::try_from(mesh.vertices.len()).expect("a few vertices");
        for (du, dv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let pos = (normal + u * du + v * dv) * 0.8;
            mesh.vertices.push(Vertex3D::new(pos, normal, Point::default()));
        }
        mesh.indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
    }
    mesh
}

/// The lines of a scene. A floor grid of 1 pixel and the 3 axes of 3,
/// hidden behind the nodes in front of them. A part built in code with
/// its 12 edges from `MeshData::edges` as lines of 2 pixels in its own
/// space, so they turn with it. A yellow line of 6 pixels that passes
/// behind the ball and is cut by it, and a magenta one of 6 that passes
/// through it on top of everything. Then the part turns and its edges
/// follow, a long line runs from in front of the camera to behind it and
/// is cut at the near plane, and an orthographic view keeps every width.
#[scene]
#[derive(Default)]
struct SceneLines {
    part:   Weak<Prop>,
    status: Weak<Label>,
}

impl SceneSetup for SceneLines {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 4.0, 8.0),
            target: Vec3::new(0.0, 0.6, 0.0),
            ..Camera::default()
        };
        self.make_node::<Prop>(Shape3::Plane(10.0), Vec3::ZERO)
            .set_color(Color::hex("#e4e6e8"))
            .set_roughness(0.9);

        self.lines = Line3::grid(8.0, 1.0, Color::hex("#9aa0a6"));
        self.lines.extend(Line3::axes(2.5).into_iter().map(|axis| axis.width(3)));
        self.lines
            .push(Line3::new((-4.0, 0.9, 0.0), (0.5, 0.9, 0.0), Color::hex("#f1c40f")).width(6));
        self.lines.push(
            Line3::new((-4.0, 1.4, 0.5), (0.5, 1.4, 0.5), Color::hex("#d633c0"))
                .width(6)
                .on_top(),
        );

        self.make_node::<Prop>(Shape3::Ball(0.9), BALL_AT)
            .set_color(Color::hex("#2980b9"))
            .set_roughness(0.5);

        let mesh = part_mesh();
        let edges: Vec<Line3> = mesh
            .edges(30.0)
            .into_iter()
            .map(|[a, b]| Line3::new(a, b, Color::hex("#1c2530")).width(2))
            .collect();
        self.part =
            self.make_node::<Prop>(Shape3::Model(Model::from_mesh("scene-lines-part", mesh)), PART_AT);
        self.part.set_color(Color::hex("#e67e22")).set_roughness(0.7);
        self.part.lines = edges;
    }
}

impl SceneTest for SceneLines {
    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        let label = view.add_view::<Label>();
        label.set_text_size(15).set_text_color(Color::hex("#ffffff"));
        label
            .set_color(Color::hex("#1c2530"))
            .set_corner_radius(6)
            .set_frame((10, 10, 580, 30));
        scene.status = label;
    }

    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let say = move |text: &'static str| {
            from_main(move || {
                scene.status.set_text(text);
            });
            wait_for_next_frame();
            wait_for_next_frame();
        };

        let edges = from_main(move || scene.part.lines.len());
        ensure!(edges == 12, "the part got {edges} edge lines");
        say("a grid, the axes, the edges of a part, a line behind the ball and one on top");
        check_colors(LINES)?;

        from_main(move || {
            scene.part.set_rotation(Quat::from_rotation_y(0.6) * Quat::from_rotation_x(0.4));
        });
        say("the part turned and its edges turned with it");
        check_colors(TURNED)?;

        from_main(move || {
            scene
                .lines
                .push(Line3::new((0.6, 0.3, -6.0), (0.6, 0.3, 30.0), Color::hex("#16a085")).width(4));
        });
        say("a green line from the far side to behind the camera, cut at the near plane");
        check_colors(THROUGH)?;

        from_main(move || {
            scene.camera.projection = Projection::Orthographic { height: 7.0 };
        });
        say("orthographic, every line keeps its width in pixels");
        check_colors(ORTHOGRAPHIC)
    }
}

const LINES: &str = r"
      76   24 - #1c2530
     216   24 - #495059
     316   24 - #c8cacd
     416   24 - #1c2530
     532   24 - #ffffff
     116   28 - #f7f7f8
     216   28 - #495059
     248   28 - #3f4650
     260   28 - #ffffff
     284   28 - #e5e6e7
     312   28 - #6d737a
     316   28 - #c8cacd
     324   28 - #5f656d
     372   28 - #313a43
     452   28 - #1c2530
     484   28 - #6f757b
     148   36 - #1c2530
       4  148 - #597c95
     300  188 - #46a758
     448  228 - #1c2530
     300  232 - #46a758
     172  244 - #3076a8
     216  244 - #337fb4
     360  244 - #7e471f
      80  264 - #d633c0
     256  264 - #d633c0
     592  264 - #597c95
     132  272 - #9aa0a6
     300  272 - #46a758
     204  276 - #6d9ecb
     300  276 - #46a758
      96  284 - #f1c40f
     256  284 - #f1c40f
     336  284 - #d8dadb
     424  292 - #bc6a2c
     356  296 - #7e471f
     248  316 - #275a80
     172  324 - #285f88
     136  328 - #d8d9db
      72  332 - #aaaeb3
     304  332 - #e5484d
     336  332 - #e5484d
     464  332 - #bc6a2b
     532  332 - #aaafb3
     300  372 - #3e8ed0
     300  416 - #3e8ed0
       4  592 - #d7d9db
     592  592 - #d8d9db
";

const TURNED: &str = r"
      60   12 - #1c2530
     536   12 - #1c2530
     256   20 - #5d636b
     184   24 - #434b54
     220   24 - #a3a7ab
     292   24 - #4a515a
     360   24 - #ffffff
     168   28 - #1c2530
     184   28 - #434b54
     228   28 - #6f757b
     276   28 - #9ca0a5
     284   28 - #ffffff
     324   28 - #ffffff
     436   28 - #1c2530
     112   36 - #1c2530
       4  104 - #597c95
     124  148 - #597c95
     592  176 - #597c95
     300  188 - #46a758
     412  220 - #e68337
     360  228 - #1c2530
     300  232 - #46a758
     216  244 - #337fb4
     456  244 - #e68337
     392  252 - #e68337
     168  264 - #d633c0
     256  264 - #d633c0
     132  272 - #9aa0a6
     300  272 - #46a758
     204  276 - #6d9ecb
     300  276 - #46a758
     336  280 - #d8dadb
      96  284 - #f1c40f
     256  284 - #f1c40f
     404  288 - #7f4720
     484  288 - #9e5926
     444  292 - #9e5926
     164  312 - #2b6691
     200  316 - #2e6f9f
     248  316 - #275a80
     468  328 - #d8d9db
      68  332 - #aaaeb3
     104  332 - #aaaeb3
     308  332 - #e5484d
     352  332 - #e5484d
     532  332 - #aaafb3
     164  340 - #214865
     228  340 - #214865
     416  352 - #d8d9db
     300  372 - #3e8ed0
     300  416 - #3e8ed0
     140  504 - #d7d9db
       4  592 - #d7d9db
     592  592 - #d8d9db
";

const THROUGH: &str = r"
     588   16 - #1c2530
     248   20 - #1c2530
     100   24 - #ecedee
     216   24 - #caccce
     384   24 - #b3b6ba
     412   24 - #ffffff
     504   24 - #ffffff
     120   28 - #61676f
     144   28 - #82878d
     184   28 - #5d636b
     308   28 - #414952
     332   28 - #d1d3d5
     364   28 - #9fa3a7
     476   28 - #9ca0a5
     528   28 - #767b82
      12   36 - #1c2530
     520  160 - #597c95
     300  188 - #46a758
     300  220 - #46a758
     404  224 - #e68337
     360  228 - #1c2530
     324  236 - #16a085
     180  240 - #3077a9
     452  244 - #e68337
     144  264 - #d633c0
     236  264 - #d633c0
     300  272 - #46a758
     592  272 - #597c95
     204  276 - #6d9ecb
     300  276 - #46a758
     104  284 - #f1c40f
     128  284 - #f1c40f
     336  288 - #1c2530
     400  288 - #7f4720
     468  292 - #9e5926
     140  304 - #234c6c
     240  320 - #29628b
     468  328 - #d8d9db
      76  332 - #aaaeb3
     304  332 - #e5484d
     324  332 - #e5484d
     344  332 - #e5484d
     348  332 - #e5484d
     532  332 - #aaafb3
     164  336 - #214865
     416  352 - #d8d9db
     300  368 - #3e8ed0
     344  376 - #16a085
     348  404 - #16a085
     300  408 - #3e8ed0
     368  532 - #16a085
     592  548 - #d8d9db
       4  592 - #d7d9db
     376  592 - #16a085
";

const ORTHOGRAPHIC: &str = r"
      12   16 - #1c2530
     384   20 - #9da1a6
     392   20 - #595f67
     448   20 - #ffffff
     200   24 - #ffffff
     216   24 - #404750
     344   24 - #83888e
     392   24 - #595f67
     404   24 - #404750
     152   28 - #e5e6e7
     168   28 - #414952
     184   28 - #ffffff
     208   28 - #868b91
     268   28 - #1c2530
     324   28 - #888c92
     392   28 - #595f67
     408   28 - #b3b6ba
     588   32 - #1c2530
      80   36 - #1c2530
      40  132 - #597c95
     300  152 - #46a758
     440  192 - #e68338
     508  204 - #e68338
     352  208 - #16a085
     128  216 - #9aa0a6
     128  220 - #9aa0a6
     208  224 - #d8d9db
       4  252 - #d633c0
      52  252 - #d633c0
     556  252 - #c9cbce
     412  256 - #7f4720
     180  264 - #6499c8
     300  264 - #46a758
     300  268 - #46a758
      28  276 - #f1c40f
     176  276 - #6698c6
     240  276 - #f1c40f
     352  284 - #1c2530
     472  288 - #1c2530
      92  316 - #214865
     144  316 - #2c6995
     300  320 - #46a758
     228  328 - #27587d
     536  328 - #9e5925
     188  344 - #275b82
     468  360 - #9e5925
     136  368 - #d8d9db
     300  372 - #3e8ed0
     300  424 - #3e8ed0
     592  460 - #d8d9db
     128  480 - #9aa0a6
     432  484 - #d8d9db
       4  512 - #d8d9db
     164  592 - #597c95
     352  592 - #16a085
     504  592 - #597c95
";
