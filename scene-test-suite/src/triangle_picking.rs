use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3, Vertex3D},
    refs::Weak,
    scene::{
        Camera, Line3, MeshData, Model, NodeTemplates, Prop, SceneCreation, SceneManager, SceneSetup,
        SceneTest, SceneTestView, TriangleHit, scene,
    },
    ui::{Color, Label, Point, U8Color, ViewData, ViewFrame, ViewSubviews, WHITE},
    ui_test::{check_colors, inject_touches, set_record_probe_count},
};

const PART_AT: Vec3 = Vec3::new(-0.6, 0.0, 0.0);
const PART_SCALE: f32 = 1.4;

/// The two parts of the L, the way a CSG library keeps the part each
/// triangle came from. The app keeps this table, the engine only gives
/// the triangle.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Part {
    Base,
    Post,
}

/// A box from `min` to `max` with flat faces in one color, appended to
/// `mesh`, every triangle tagged with `part` in `parts`.
fn add_box(mesh: &mut MeshData, parts: &mut Vec<Part>, part: Part, min: Vec3, max: Vec3, color: U8Color) {
    let center = (min + max) / 2.0;
    let half = (max - min) / 2.0;
    for normal in [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z] {
        let u = normal.any_orthonormal_vector();
        let v = normal.cross(u);
        let first = u32::try_from(mesh.vertices.len()).expect("a few vertices");
        for (du, dv) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            let pos = center + (normal + u * du + v * dv) * half;
            let mut vertex = Vertex3D::new(pos, normal, Point::default());
            vertex.color = color;
            mesh.vertices.push(vertex);
        }
        mesh.indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
        parts.extend([part, part]);
    }
}

/// An L shaped part built in code from 2 boxes, a base and a post, as 1
/// mesh, turned and grown on its node. A tap becomes a ray, and
/// `hit_triangle` finds the triangle under it, with the point and the
/// normal, so a marker and a line along the normal show where it landed,
/// and the triangle index finds the part in the app's own table. A tap in
/// the empty corner of the L is inside the box around the model, so the
/// box picking says the node, and no triangle is there.
#[scene]
#[derive(Default)]
struct TrianglePicking {
    parts:  Vec<Part>,
    marker: Weak<Prop>,
    hit:    Option<TriangleHit>,
    status: Weak<Label>,
}

impl SceneSetup for TrianglePicking {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(1.5, 3.0, 7.0),
            target: Vec3::new(0.2, 1.0, 0.0),
            ..Camera::default()
        };
        self.make_node::<Prop>(Shape3::Plane(10.0), Vec3::ZERO)
            .set_color(Color::hex("#e4e6e8"))
            .set_roughness(0.9);

        let mut mesh = MeshData::default();
        let mut parts = vec![];
        add_box(
            &mut mesh,
            &mut parts,
            Part::Base,
            Vec3::new(0.0, 0.0, -0.5),
            Vec3::new(2.0, 0.8, 0.5),
            U8Color::const_rgb(230, 126, 34),
        );
        add_box(
            &mut mesh,
            &mut parts,
            Part::Post,
            Vec3::new(0.0, 0.8, -0.5),
            Vec3::new(0.8, 2.0, 0.5),
            U8Color::const_rgb(22, 160, 133),
        );
        self.parts = parts;
        self.make_node::<Prop>(
            Shape3::Model(Model::from_mesh("triangle-picking-l", mesh)),
            PART_AT,
        )
        .set_rotation(Quat::from_rotation_y(-0.5))
        .set_scale(PART_SCALE)
        .set_color(WHITE)
        .set_roughness(0.7);

        self.marker = self.make_node::<Prop>(Shape3::Ball(0.12), Vec3::new(0.0, -5.0, 0.0));
        self.marker.set_color(Color::hex("#d633c0"));

        self.on_tap.val(|ray| {
            let mut scene = SceneManager::downcast_scene::<TrianglePicking>();
            scene.hit = scene.hit_triangle(ray);
            scene.show_hit();
        });
    }
}

impl TrianglePicking {
    /// The marker on the hit point and a line along the normal, or both
    /// out of sight when the ray met no triangle.
    fn show_hit(&mut self) {
        self.lines.clear();
        let Some(hit) = self.hit else {
            self.marker.set_position(Vec3::new(0.0, -5.0, 0.0));
            return;
        };
        self.marker.set_position(hit.point);
        self.lines.push(
            Line3::new(hit.point, hit.point + hit.normal * 1.2, Color::hex("#d633c0"))
                .width(4)
                .on_top(),
        );
    }

    /// Where a point of the L in its own space is in the world.
    fn world(local: Vec3) -> Vec3 {
        PART_AT + Quat::from_rotation_y(-0.5) * (local * PART_SCALE)
    }
}

impl SceneTest for TrianglePicking {
    fn overlay(mut scene: Weak<Self>, view: Weak<SceneTestView>) {
        let label = view.add_view::<Label>();
        label.set_text_size(15).set_text_color(Color::hex("#ffffff"));
        label
            .set_color(Color::hex("#1c2530"))
            .set_corner_radius(6)
            .set_frame((10, 10, 580, 30));
        scene.status = label;
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        let say = move |text: &'static str| {
            from_main(move || {
                scene.status.set_text(text);
            });
            wait_for_next_frame();
            wait_for_next_frame();
        };
        // The pixel of a point of the L, the place a tap lands.
        let pixel = move |local: Vec3| {
            from_main(move || scene.view_point(TrianglePicking::world(local)))
                .expect("in front of the camera")
        };

        say("an L of 2 parts as 1 mesh, turned and grown");
        check_colors(PART)?;

        // The front face of the base, z of 0.5 in the L's space.
        tap(pixel(Vec3::new(1.5, 0.4, 0.5)));
        let (hit, parts) = from_main(move || (scene.hit, scene.parts.clone()));
        let hit = hit.ok_or_else(|| anyhow::anyhow!("the tap on the base met no triangle"))?;
        ensure!(
            parts[hit.triangle] == Part::Base,
            "triangle {} is not the base",
            hit.triangle
        );
        let front = Quat::from_rotation_y(-0.5) * Vec3::Z;
        ensure!(
            hit.normal.dot(front) > 0.999,
            "the normal {} is not the front",
            hit.normal
        );
        let expected = TrianglePicking::world(Vec3::new(1.5, 0.4, 0.5));
        ensure!(
            hit.point.distance(expected) < 0.02,
            "hit at {}, tapped {expected}",
            hit.point
        );
        say("a tap on the base, its triangle, the point and the normal");
        check_colors(BASE)?;

        // The top of the post, y of 2 in the L's space.
        tap(pixel(Vec3::new(0.4, 2.0, 0.0)));
        let hit =
            from_main(move || scene.hit).ok_or_else(|| anyhow::anyhow!("the tap on the post met nothing"))?;
        ensure!(
            parts[hit.triangle] == Part::Post,
            "triangle {} is not the post",
            hit.triangle
        );
        ensure!(
            hit.normal.y > 0.999,
            "the normal {} of the top does not point up",
            hit.normal
        );
        say("a tap on the top of the post, a triangle of the other part");
        check_colors(POST)?;

        // The empty corner of the L, inside the box around the model.
        let corner = pixel(Vec3::new(1.5, 1.5, 0.0));
        let by_box = from_main(move || scene.node_at(corner).is_some());
        ensure!(by_box, "the corner is outside the box around the model");
        tap(corner);
        let hit = from_main(move || scene.hit.map(|hit| hit.triangle));
        ensure!(hit.is_none(), "the empty corner of the L met triangle {hit:?}");
        say("a tap in the empty corner, the box says the node, no triangle is there");
        check_colors(CORNER)
    }
}

fn tap(point: Point) {
    inject_touches(format!("{} {} b\n{} {} e", point.x, point.y, point.x, point.y));
    wait_for_next_frame();
}

const PART: &str = r"
     124   12 - #1c2530
     556   12 - #1c2530
      12   16 - #1c2530
     188   20 - #1c2530
     316   20 - #e5e6e7
     324   20 - #1c2530
     176   24 - #b0b3b7
     292   24 - #ffffff
     316   24 - #ffffff
     340   24 - #d5d7d9
     344   24 - #ffffff
     348   24 - #595f67
     412   24 - #313a43
     164   28 - #737980
     168   28 - #fefefe
     196   28 - #e8e9ea
     236   28 - #1c2530
     292   28 - #dddfe0
     316   28 - #e5e6e7
     344   28 - #fbfcfc
     348   28 - #595f67
     388   28 - #1c2530
     412   28 - #313a43
     432   28 - #1c2530
     440   28 - #5d636b
      88   36 - #1c2530
     500   36 - #1c2530
     592  124 - #597c95
     108  148 - #597c95
     464  160 - #597c95
     236  172 - #227361
     284  172 - #268671
     332  172 - #268671
       4  188 - #597c95
     204  204 - #227361
     336  216 - #268671
     232  244 - #227361
     280  256 - #268671
     352  296 - #d97a31
     268  312 - #227361
     312  324 - #d97a31
       8  332 - #597c95
     392  344 - #d97a31
     212  368 - #a45c26
     336  368 - #a45c26
     444  376 - #bf6b2b
     268  396 - #a45c26
     400  396 - #bf6b2b
     320  416 - #a45c26
     592  432 - #d8d9db
     372  436 - #a45c26
       4  592 - #d7d9db
     208  592 - #d7d9db
     424  592 - #d7d9db
     592  592 - #d8d9db
";

const BASE: &str = r"
      60   12 - #1c2530
     588   16 - #1c2530
     184   20 - #8a8f95
     204   20 - #737980
     256   20 - #1c2530
     300   20 - #6d737a
     324   20 - #ffffff
     148   24 - #1c2530
     272   24 - #dedfe1
     284   24 - #ffffff
     360   24 - #565d65
      12   28 - #1c2530
     172   28 - #858a90
     244   28 - #1c2530
     268   28 - #5d636b
     300   28 - #6d737a
     344   28 - #ffffff
     360   28 - #565d65
     364   28 - #caccce
     392   28 - #1c2530
     420   28 - #f7f7f8
     440   28 - #e0e1e3
     452   28 - #ced0d2
     520   36 - #1c2530
       4   80 - #597c95
     460  160 - #597c95
     592  164 - #597c95
     204  172 - #227361
     316  172 - #268671
     260  184 - #227361
     328  228 - #268671
     212  232 - #227361
     272  240 - #268671
     288  320 - #a45c26
       4  332 - #597c95
     444  336 - #bf6b2b
     396  348 - #bf6b2b
     264  364 - #a45c26
     216  372 - #a45c26
     340  380 - #c73ab3
     336  384 - #ab319a
     340  384 - #ac319a
     332  388 - #752569
     336  388 - #752569
     312  392 - #d633c0
     440  396 - #bf6b2b
     296  400 - #d633c0
     284  408 - #d633c0
     268  416 - #d633c0
     384  440 - #a45c26
       4  592 - #d7d9db
     196  592 - #d7d9db
     432  592 - #d7d9db
     592  592 - #d8d9db
";

const POST: &str = r"
      44   12 - #1c2530
     588   16 - #1c2530
     184   20 - #e5e6e7
     252   20 - #b8bbbe
     396   20 - #ffffff
     184   24 - #ffffff
     216   24 - #434b54
     252   24 - #ffffff
     272   24 - #ffffff
     320   24 - #ffffff
     352   24 - #1c2530
     376   24 - #1c2530
     440   24 - #1c2530
     172   28 - #e0e1e3
     184   28 - #e5e6e7
     216   28 - #434b54
     256   28 - #d1d3d5
     328   28 - #1c2530
     400   28 - #767b82
     472   28 - #303842
      12   36 - #1c2530
     104   36 - #1c2530
     532   36 - #1c2530
     272   72 - #d633c0
     272   96 - #d633c0
       4  132 - #597c95
     272  132 - #d633c0
     272  148 - #d633c0
     112  152 - #597c95
     460  160 - #597c95
     268  164 - #c73ab3
     276  164 - #d644c1
     592  164 - #597c95
     276  168 - #c53eb1
     204  212 - #30756e
     336  212 - #268671
     268  240 - #227361
     256  284 - #227361
     296  304 - #268671
     408  328 - #d97a31
       4  332 - #597c95
     356  332 - #d97a31
     444  348 - #bf6b2b
     272  352 - #a45c26
     392  372 - #bf6b2b
     220  376 - #a45c26
     348  392 - #a45c26
     440  392 - #bf6b2b
     288  404 - #a45c26
     592  432 - #d8d9db
     384  440 - #a45c26
     168  536 - #d7d9db
       4  560 - #d7d9db
     324  592 - #d7d9db
     592  592 - #d8d9db
";

const CORNER: &str = r"
      60   12 - #1c2530
     284   12 - #1c2530
     588   16 - #1c2530
     384   20 - #474e57
     460   20 - #dedfe1
     116   24 - #333b45
     168   24 - #ffffff
     216   24 - #414952
     312   24 - #1c2530
     432   24 - #ffffff
     444   24 - #ffffff
     476   24 - #d2d4d6
     496   24 - #ffffff
      88   28 - #1c2530
     136   28 - #a1a4a9
     164   28 - #898d93
     220   28 - #1c2530
     228   28 - #414952
     256   28 - #1c2530
     344   28 - #1c2530
     384   28 - #474e57
     440   28 - #ffffff
     444   28 - #b8bbbe
     448   28 - #d1d3d5
     460   28 - #dedfe1
     512   28 - #e5e6e7
      12   36 - #1c2530
     552   36 - #1c2530
     100  144 - #597c95
     592  156 - #597c95
     460  160 - #597c95
     236  172 - #227361
     284  172 - #268671
     332  172 - #268671
     204  204 - #227361
       4  212 - #597c95
     336  216 - #268671
     232  244 - #227361
     280  256 - #268671
     352  296 - #d97a31
      48  308 - #597c95
     268  312 - #227361
     392  344 - #d97a31
     212  368 - #a45c26
     336  368 - #a45c26
     444  376 - #bf6b2b
     268  396 - #a45c26
     400  396 - #bf6b2b
       8  404 - #d8d9db
     320  416 - #a45c26
     372  436 - #a45c26
       4  592 - #d7d9db
     208  592 - #d7d9db
     424  592 - #d7d9db
     592  592 - #d8d9db
";
