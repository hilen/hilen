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
     284    4 - #d0ddea
     520   32 - #284a3d
     144   52 - #d7e2eb
     500   68 - #284a3d
     480  108 - #284a3d
     572  108 - #1e3226
     348  116 - #dde6ec
      40  156 - #2f5532
     448  160 - #39714c
     520  160 - #3b764f
     588  168 - #dee7ed
     588  172 - #dee7ed
     592  172 - #dee7ed
     592  176 - #dee7ed
     164  204 - #8a6a4f
     172  204 - #694e36
      24  212 - #305331
     152  220 - #45352a
     172  224 - #bed0c4
     200  224 - #bccec3
     240  224 - #a2c1a0
     260  224 - #9eba9d
     272  224 - #c0d3c6
     416  224 - #bfd3c5
     444  224 - #9db89c
     512  228 - #3a7051
     140  232 - #4a382c
      88  248 - #525d6b
     120  248 - #5f442d
     336  248 - #83bc66
     104  260 - #60452d
      76  264 - #525b67
     400  264 - #5f9150
       4  268 - #619451
     128  280 - #677283
     100  284 - #717b88
     144  288 - #596577
     428  292 - #7ab060
      56  300 - #3c3324
     120  300 - #98a5b4
     144  304 - #d8bb92
     156  304 - #9b8269
     504  304 - #513e33
      68  308 - #483c29
     164  308 - #d3b78f
     560  308 - #35614d
     152  312 - #cfb38d
     172  312 - #d4b78f
      84  316 - #bd9d75
      96  316 - #d8bb92
     112  316 - #a3978a
     132  316 - #ad8861
     160  316 - #b99973
     288  316 - #80ba64
     364  316 - #b3702d
      76  320 - #bc9b74
     116  320 - #ad8861
     120  320 - #ad8861
     168  320 - #d7ba91
     492  320 - #4d3b30
      64  324 - #ccb08a
     144  324 - #b79772
     148  324 - #b79772
     156  324 - #dabc93
     376  324 - #e58c3a
       4  328 - #5d8c4e
     104  328 - #caa67c
     140  328 - #c9ae89
      88  332 - #d9bb92
     124  332 - #d8ba92
     364  332 - #e88e39
     380  340 - #e35f34
     356  344 - #654d3d
     352  364 - #b3702d
     376  364 - #e35f34
     408  364 - #72675b
     352  368 - #b3702d
     192  372 - #564338
     276  372 - #665d53
     292  372 - #8a909b
     352  372 - #b3702d
     388  372 - #de5e36
      48  376 - #5e8e4f
     312  376 - #a9adb4
     432  376 - #95999d
     340  380 - #413024
     348  380 - #b3702d
     384  384 - #e35f34
     420  384 - #6e7172
     592  384 - #76a95d
     404  388 - #47362a
     328  392 - #72665b
     348  392 - #b3702d
     440  392 - #9ba2ad
     136  396 - #5a4639
     296  396 - #a7acb1
     312  396 - #654d3c
     392  396 - #de5e36
     460  400 - #979ba2
     280  404 - #75797f
     348  404 - #e88d38
     372  404 - #a8703c
     388  408 - #de5e36
     416  408 - #473629
     332  412 - #a9adb4
     440  412 - #717374
     232  416 - #695e55
     428  420 - #8d8f92
     300  424 - #62666c
     324  424 - #a9adb3
     364  428 - #8a919b
     416  428 - #7c8085
     312  432 - #717479
     388  432 - #9fa3aa
     432  436 - #585959
      52  448 - #7db562
     376  448 - #51514f
     568  448 - #78ad60
     500  452 - #665d53
     340  472 - #665c53
     436  472 - #6b6156
     300  536 - #80ba64
     100  540 - #679e55
     592  584 - #639953
     192  588 - #619651
       4  592 - #5e9050
     432  592 - #79b060
";
