mod shapes;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3, Vertex3D},
    refs::{Weak, manage::DataManager},
    scene::{Camera, MeshData, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, scene},
    ui::{Color, Point},
    ui_test::{check_colors, set_record_probe_count},
};

/// Grid points along each side of the terrain. 281 squared is over the
/// 65535 vertices of one 16 bit mesh, so the terrain draws as several
/// parts and a seam between them would show as a gap.
const TERRAIN_POINTS: u16 = 281;
/// Units between two grid points.
const TERRAIN_STEP: f32 = 0.1;
/// How high the hills rise above and sink below zero.
const HILL: f32 = 1.2;
const ROCK: &str = "Code meshes rock";

/// Meshes built in code rather than loaded from a `.glb`, standing on a
/// rolling terrain big enough to split into parts: a trefoil knot tube
/// floating in the middle, a vase turned on a lathe, a twisted star
/// column and a flat shaded rock, each built a different way, see
/// `shapes`. A code mesh has no material and takes its node's.
#[scene]
#[derive(Default)]
struct CodeMeshes {}

impl SceneSetup for CodeMeshes {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 6.5, 12.0),
            target: Vec3::new(0.0, 1.2, 0.0),
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));

        self.make_node::<Prop>(
            Shape3::Model(Model::from_mesh("Code meshes terrain", terrain())),
            Vec3::ZERO,
        )
        .set_color(Color::hex("#5f9e4a"))
        .set_roughness(0.9);

        self.make_node::<Prop>(
            Shape3::Model(Model::from_mesh("Code meshes knot", shapes::knot())),
            Vec3::new(0.0, 3.2, -2.5),
        )
        .set_color(Color::hex("#e8b923"))
        .set_metallic(1.0)
        .set_roughness(0.3);

        // The last number lifts a shape off the ground, the rock's origin
        // is its middle. The rest sink a little so the base meets the
        // slope on every side.
        let grounded = [
            (
                "Code meshes vase",
                shapes::vase(),
                -4.5,
                0.5,
                "#2e86de",
                0.25,
                -0.15,
            ),
            (
                "Code meshes star",
                shapes::twisted_star(),
                4.5,
                0.5,
                "#e67e22",
                0.5,
                -0.15,
            ),
            (ROCK, shapes::rock(), 0.0, 3.0, "#8a8580", 0.95, 0.35),
        ];
        for (name, mesh, x, z, color, roughness, lift) in grounded {
            let position = Vec3::new(x, height(x, z) + lift, z);
            self.make_node::<Prop>(Shape3::Model(Model::from_mesh(name, mesh)), position)
                .set_color(Color::hex(color))
                .set_roughness(roughness);
        }
    }
}

impl SceneTest for CodeMeshes {
    fn perform_test(_scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        let stored = from_main(|| Model::get(ROCK) == Model::from_mesh(ROCK, shapes::rock()));
        ensure!(stored, "a code mesh is stored under its name like a loaded file");

        wait_for_next_frame();
        check_colors(SCENE)
    }
}

fn height(x: f32, z: f32) -> f32 {
    HILL * (x * 0.35).sin() * (z * 0.3).cos()
}

/// A grid in the xz plane lifted by `height`, centered on the origin,
/// with the normals of the height function so the hills shade smooth.
fn terrain() -> MeshData {
    let side = u32::from(TERRAIN_POINTS);
    let half = f32::from(TERRAIN_POINTS - 1) * TERRAIN_STEP / 2.0;

    let vertices = (0..TERRAIN_POINTS)
        .flat_map(|row| (0..TERRAIN_POINTS).map(move |column| (row, column)))
        .map(|(row, column)| {
            let x = f32::from(column) * TERRAIN_STEP - half;
            let z = f32::from(row) * TERRAIN_STEP - half;
            let dx = HILL * 0.35 * (x * 0.35).cos() * (z * 0.3).cos();
            let dz = -HILL * 0.3 * (x * 0.35).sin() * (z * 0.3).sin();
            Vertex3D::new(
                Vec3::new(x, height(x, z), z),
                Vec3::new(-dx, 1.0, -dz).normalize(),
                Point::default(),
            )
        })
        .collect();

    // Counter clockwise seen from above, a row is one step of +z.
    let indices = (0..side - 1)
        .flat_map(|row| (0..side - 1).map(move |column| row * side + column))
        .flat_map(|i| [i, i + side, i + 1, i + 1, i + side, i + side + 1])
        .collect();

    MeshData { vertices, indices }
}

const SCENE: &str = r"
       4    4 - #d4deeb
     184    4 - #d3deeb
     412    4 - #d3deeb
     592    4 - #d4deeb
     300   44 - #d5e0ec
      96   96 - #d5e0ec
     276  160 - #fbd36c
     324  160 - #6d6d36
     312  164 - #515e22
     280  168 - #523806
     304  168 - #6f6c21
     264  172 - #af9429
     320  172 - #765d10
     328  172 - #674e0d
     332  172 - #6c530f
     312  176 - #543a06
     256  180 - #fee0a6
     288  180 - #ad9228
     304  180 - #513706
     340  180 - #604717
     324  184 - #d0dae5
     328  184 - #d0dae5
     292  188 - #513706
     312  188 - #7f7620
     324  188 - #d0dae5
     344  188 - #533907
     472  188 - #d9b7a8
     476  188 - #d9b8a8
     480  188 - #d9b8a8
     336  192 - #a3881e
     284  196 - #583e08
     296  196 - #ced8e3
     300  196 - #ced8e3
     332  196 - #978420
     340  196 - #69500d
     516  196 - #e2956a
     520  196 - #e2956a
     528  196 - #c0743e
     532  196 - #c0743f
     256  200 - #af9320
     312  200 - #af9326
     496  200 - #ea8b44
     284  204 - #583e0a
     328  204 - #b29621
     272  208 - #ab9322
     280  208 - #7e7627
     288  208 - #847b3a
     292  208 - #847b3c
     296  208 - #7a7436
     304  208 - #636727
     464  208 - #cf7c3b
     488  216 - #ab7456
      64  220 - #7db181
     280  220 - #523809
     288  220 - #5a400a
     296  220 - #5f460d
     316  220 - #ad9224
     412  220 - #96bda0
     512  220 - #a96125
     472  224 - #da823f
     276  228 - #8f7623
     304  232 - #63a661
     532  232 - #b16628
     292  236 - #546022
     324  236 - #896f1b
     492  236 - #a16030
     496  240 - #a1602f
     468  244 - #e58a46
     500  244 - #a05f2d
     296  248 - #886f1e
     304  252 - #644d20
     524  252 - #bd6f32
     504  256 - #9f5d27
     456  264 - #c8783a
     480  264 - #e98b44
     508  276 - #9f5c23
      96  280 - #438bf3
     100  280 - #438cf3
     104  280 - #478ef4
     120  280 - #3e89f1
     488  284 - #ea8c44
     456  288 - #da823e
     520  292 - #ac6325
     112  296 - #2561aa
     112  300 - #2d76ce
     468  300 - #e18641
     484  304 - #a1602f
     492  308 - #a1602f
     512  308 - #ba6c2c
     460  312 - #e68a44
     128  316 - #468df2
     492  316 - #9f5d27
     144  332 - #4f8fee
     108  336 - #428bf0
     128  352 - #468ef1
     108  356 - #438bee
     280  356 - #797e8e
     324  360 - #828693
     300  364 - #878b99
      92  368 - #4c7dce
     152  368 - #517dc5
      96  372 - #3d77cc
     128  376 - #307dd9
     140  376 - #2e74ca
     268  380 - #6c7280
     108  384 - #2e67b3
     288  384 - #848896
     140  388 - #3b67aa
     316  392 - #91949e
     324  392 - #91949e
     120  396 - #2c6bbc
     320  396 - #91949e
     332  396 - #92949e
     340  396 - #92949d
     328  400 - #92949c
     260  404 - #61646b
     264  416 - #606268
     280  416 - #616369
     292  420 - #636468
     340  424 - #767679
     316  428 - #6c6c6f
     296  432 - #606064
     592  436 - #64a863
     148  544 - #5c9c5d
     440  548 - #63a762
       4  592 - #5b9b5c
     288  592 - #60a460
     592  592 - #63a862
";
