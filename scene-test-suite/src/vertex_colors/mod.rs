mod props;

use std::f32::consts::PI;

use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{Camera, MeshData, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sky, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

use self::props::{AXE_EDGE, FIRE};

/// Where the stump stands, the axe is sunk into its top.
const STUMP: Vec3 = Vec3::new(-1.5, 0.0, 0.4);
const STUMP_TOP: f32 = 0.5;
const AXE_SCALE: f32 = 1.3;

/// A camp built from meshes whose every face carries its own color, the
/// way geblings builds its items from parts: faceted grass going bare
/// around a fire ring of stones and leaned logs, a bearded axe sunk into
/// a chopping stump, and two pines. The far pine is the near one's mesh
/// under a yellow green node color, which multiplies every face, so it
/// reads as a younger tree. Every other node is white, which shows the
/// face colors as they are.
#[scene]
#[derive(Default)]
struct VertexColors {}

impl VertexColors {
    fn prop(&mut self, name: &str, mesh: MeshData, position: Vec3) -> Weak<Prop> {
        let mut node = self.make_node::<Prop>(Shape3::Model(Model::from_mesh(name, mesh)), position);
        // A node's material is a random color until set.
        node.set_color(Color::hex("#ffffff")).set_roughness(0.85);
        node
    }
}

impl SceneSetup for VertexColors {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 1.7, 4.6),
            target: Vec3::new(0.0, 0.55, 0.0),
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#5b8fd6"),
            Color::hex("#e3ecf2"),
            Color::hex("#5a4a3a"),
        ));

        self.prop("Vertex colors ground", props::ground(), Vec3::ZERO);
        self.prop("Vertex colors stump", props::stump(), STUMP);
        self.prop("Vertex colors campfire", props::campfire(), FIRE);

        // Tipped past upside down so the edge bites down and back, then
        // swung round so the handle rises to the right, towards the camera.
        let turn = Quat::from_rotation_y(PI - 0.5) * Quat::from_rotation_z(-2.1);
        let bite = STUMP + Vec3::new(0.0, STUMP_TOP - 0.06, 0.0);
        self.prop(
            "Vertex colors axe",
            props::axe(),
            bite - turn * (AXE_EDGE * AXE_SCALE),
        )
        .set_rotation(turn)
        .set_roughness(0.5)
        .set_scale(AXE_SCALE);

        let pine = Model::from_mesh("Vertex colors pine", props::pine());
        self.make_node::<Prop>(Shape3::Model(pine), Vec3::new(2.3, 0.0, -1.0))
            .set_color(Color::hex("#ffffff"))
            .set_roughness(0.9)
            .set_scale(1.3);
        self.make_node::<Prop>(Shape3::Model(pine), Vec3::new(-2.9, 0.0, -1.6))
            .set_color(Color::hex("#d6e07a"))
            .set_roughness(0.9)
            .set_rotation(Quat::from_rotation_y(1.2))
            .set_scale(0.8);
    }
}

impl SceneTest for VertexColors {
    fn perform_test(_scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);
        wait_for_next_frame();
        check_colors(CAMP)
    }
}

const CAMP: &str = r"
       4    4 - #d3dfeb
     280    4 - #d0ddea
     520   32 - #284a3d
     144   56 - #d7e2ec
     500   68 - #284a3d
     480  108 - #284a3d
     572  108 - #1e3226
     324  124 - #dde6ec
      40  156 - #2f5532
     520  160 - #3b764f
     444  164 - #39714c
     588  168 - #dee7ed
     588  172 - #dee7ed
     592  172 - #dee7ed
     592  176 - #dee7ed
     172  204 - #694e36
     160  216 - #3c2c22
     148  220 - #3f322c
     140  232 - #4a382c
     532  232 - #35614e
     136  236 - #5f442c
       8  244 - #619351
     120  248 - #5f442d
     104  252 - #8c6c52
     424  252 - #315c4a
     340  256 - #80b864
     104  260 - #60452d
      76  264 - #525c67
     400  264 - #5f9150
     112  276 - #687384
      84  280 - #525b67
     128  280 - #677283
     100  288 - #99a6b5
      56  300 - #3c3324
     120  300 - #98a5b4
     144  304 - #d8bb92
     496  304 - #4d3b30
      68  308 - #483c29
     164  308 - #d3b78f
      84  316 - #bd9d75
     100  316 - #c7a47a
     132  316 - #ad8861
     152  316 - #d9bb92
     160  316 - #b99973
     172  316 - #d6b991
     288  316 - #80ba64
     364  316 - #b3702d
     592  316 - #5e8d50
     116  320 - #ad8861
     120  320 - #ad8861
      64  324 - #ccb08a
     128  324 - #d2b58e
     144  324 - #b79772
     500  324 - #4d3b30
       4  328 - #5d8c4e
      76  328 - #d5b890
      88  328 - #c3a179
     100  332 - #c7ac88
     112  332 - #d8ba92
     124  332 - #d8ba92
     364  332 - #e88e39
     380  340 - #e35f34
     356  344 - #654d3d
     376  360 - #e35f34
     352  364 - #b3702d
     404  364 - #72675b
     192  368 - #564438
     280  368 - #665d53
     352  368 - #b3702d
     300  372 - #888e96
     352  372 - #b3702d
     388  372 - #de5e36
     420  372 - #999fa9
      48  376 - #5e8e4f
     312  376 - #a8adb4
     336  380 - #413025
     348  380 - #b3702d
     436  380 - #aeb1b3
     576  380 - #76a95d
     292  384 - #686b6e
     348  384 - #b3702d
     376  384 - #e78e39
     388  384 - #de5e36
     348  388 - #b3702d
     404  388 - #47362a
      80  392 - #4d3d34
     284  392 - #8c919a
     304  392 - #979aa1
     348  392 - #b3702d
     392  392 - #de5e36
       8  396 - #7cb361
     144  396 - #5a4639
     324  396 - #72665b
     412  400 - #47362a
     444  400 - #a1a4aa
     280  404 - #75797f
     300  404 - #828587
     348  404 - #e88e38
     388  404 - #de5e36
     464  404 - #888a8d
     312  408 - #716559
     376  408 - #e88e38
     448  412 - #808181
     232  416 - #695f55
     432  416 - #969aa1
     316  420 - #acafb4
     420  420 - #7f8389
     304  424 - #7a7e84
     364  428 - #8a919b
     448  428 - #888a8b
     324  432 - #888889
     416  432 - #555656
     360  440 - #5a5d60
     376  440 - #8b8f92
      52  448 - #7db562
     384  448 - #797a78
     580  448 - #78ad60
     500  452 - #665d53
     336  472 - #665c53
     436  472 - #6b6156
     300  536 - #80ba64
     104  544 - #679e55
       4  592 - #5e9050
     200  592 - #619651
     424  592 - #79b061
     592  592 - #639953
";
