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
      24    4 - #d3deeb
     284    4 - #d0ddea
     520   32 - #284a3d
     156   56 - #d7e2ec
     500   68 - #284a3d
     480  108 - #284a3d
     572  108 - #1e3226
     340  120 - #dde6ec
      40  156 - #2f5532
     448  160 - #39714c
     520  160 - #3b764f
     588  168 - #dee7ed
     588  172 - #dee7ed
     592  172 - #dee7ed
     592  176 - #dee7ed
     172  196 - #83624c
      28  204 - #305332
     168  204 - #694e36
     160  216 - #3c2c22
     148  220 - #3f322c
     172  224 - #c0d4c6
     200  224 - #bdcec4
     240  224 - #9db99d
     260  224 - #9eba9d
     416  224 - #c0d3c6
     444  224 - #9db99d
     512  228 - #3a7151
     136  236 - #5f442c
       8  244 - #619351
      92  248 - #525d6b
     120  248 - #5f442d
     336  248 - #83bc66
      76  256 - #525b66
     104  260 - #60452d
     400  264 - #5f9150
      84  276 - #525b67
     128  280 - #677283
     100  284 - #727c88
     144  288 - #596577
     116  296 - #96a3b1
      56  300 - #3c3324
     144  304 - #d8bb92
     156  304 - #9b8269
     368  304 - #e88e39
     504  304 - #513f33
      68  308 - #483c29
     560  308 - #35614d
      84  312 - #d4b78f
     152  312 - #cfb38d
     172  312 - #d4b78f
      84  316 - #bd9d75
      96  316 - #d8bb92
     112  316 - #a3978b
     132  316 - #ad8861
     160  316 - #b99973
     288  316 - #80ba64
     364  316 - #b3702d
      76  320 - #bc9c75
     116  320 - #ad8861
     120  320 - #ad8861
     168  320 - #d7ba91
     492  320 - #4d3b30
      64  324 - #ccb08a
     144  324 - #b79772
     148  324 - #b79772
     156  324 - #dabc94
     364  324 - #e88e39
       4  328 - #5d8c4e
      88  328 - #c3a179
     104  332 - #cbb08b
     124  332 - #d8ba92
     380  340 - #e35f34
     356  344 - #654d3d
     396  344 - #73a85b
     376  360 - #e35f34
     352  364 - #b3702d
     404  364 - #72675b
     280  368 - #665d53
     352  372 - #b3702d
     388  372 - #de5e36
      48  376 - #5e8e4f
     308  376 - #92969c
     416  376 - #92969d
     184  380 - #564338
     336  380 - #413025
     348  380 - #b3702d
     428  380 - #b2b6bb
     576  380 - #76a95d
     348  384 - #b3702d
     400  384 - #47362a
     288  388 - #848a95
     348  388 - #b3702d
     380  388 - #e35f34
     328  392 - #72665b
     440  392 - #9ba2ad
       8  396 - #7cb361
     312  396 - #654d3c
     120  400 - #5a4639
     396  400 - #de5e36
     412  400 - #47362a
     460  400 - #989ca2
     280  404 - #75797f
     348  404 - #e88e38
     372  404 - #a9703c
     388  408 - #de5e36
     336  412 - #9fa3a9
     420  412 - #989fa9
     440  412 - #717374
     232  416 - #695f55
     304  416 - #878d97
     324  420 - #a9adb3
     380  424 - #a2a7ae
     304  428 - #7a7e84
     364  428 - #8a919b
     420  428 - #74787c
     432  436 - #585959
     384  444 - #8d8f92
      52  448 - #7db562
     500  452 - #665d53
     584  452 - #74a95d
     328  468 - #665c53
     436  472 - #6b6156
     300  536 - #80ba64
      96  540 - #679e55
     188  588 - #5d8f4f
       4  592 - #5e9050
     432  592 - #79b061
     576  592 - #639953
";
