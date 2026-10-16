use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{Camera, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, Sun, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

const RED: &str = "#ff0000";

/// 2 red boxes under a strong blue sun, the left one lit and the right one
/// unlit, and in front of them the quad of `unlit_quad.glb`, whose glTF
/// material is marked `KHR_materials_unlit`. The unlit box and the quad
/// stay their own red, the lit box turns purple and dark on its shaded
/// sides. Then the left box is made unlit too.
#[scene]
#[derive(Default)]
struct Unlit {
    lit: Option<Weak<Prop>>,
}

impl SceneSetup for Unlit {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 3.0, 6.0),
            target: Vec3::new(0.0, 0.5, 0.0),
            ..Camera::default()
        };
        self.sun = Sun {
            direction: Vec3::new(-0.4, -1.0, -0.3),
            color: Color::hex("#3050ff"),
            intensity: 4.0,
            ..Sun::default()
        };
        self.ambient = Color::hex("#101040");
        self.make_node::<Prop>(Shape3::Plane(20.0), Vec3::ZERO)
            .set_color(Color::hex("#808080"))
            .set_roughness(0.9);
        let mut lit = self.make_node::<Prop>(Shape3::cuboid(1.2, 1.2, 1.2), Vec3::new(-1.5, 0.6, -1.0));
        lit.set_color(Color::hex(RED)).set_roughness(0.8);
        self.lit = Some(lit);
        self.make_node::<Prop>(Shape3::cuboid(1.2, 1.2, 1.2), Vec3::new(1.5, 0.6, -1.0))
            .set_color(Color::hex(RED))
            .set_roughness(0.8)
            .set_unlit(true);
        self.make_node::<Prop>(
            Shape3::Model(Model::get("unlit_quad.glb")),
            Vec3::new(0.0, 0.01, 1.5),
        )
        .set_scale(0.6);
    }
}

impl SceneTest for Unlit {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);

        wait_for_next_frame();
        check_colors(ONE_LIT)?;

        from_main(move || {
            if let Some(mut lit) = scene.lit {
                lit.set_unlit(true);
            }
        });
        wait_for_next_frame();
        check_colors(ALL_UNLIT)
    }
}

/// The left box lit, the right box and the quad unlit.
const ONE_LIT: &str = r"
       4    4 - #597c95
     284    4 - #597c95
     592    4 - #597c95
     144   68 - #597c95
     440   72 - #597c95
     540  100 - #597c95
     300  120 - #597c95
      96  148 - #597c95
       4  164 - #597c95
     592  192 - #2f4beb
     168  216 - #5e0f46
     196  216 - #5e0f46
     228  216 - #5e0f46
     360  220 - #903686
     360  224 - #c12b53
     148  228 - #5e0f45
     452  228 - #f12121
     176  236 - #5e0f45
     212  236 - #5e0f45
     240  236 - #333abe
     240  240 - #372b92
     240  244 - #3b1b65
     164  264 - #370526
     232  264 - #3f0a38
     404  268 - #f12121
     140  276 - #370526
     188  276 - #370526
     240  284 - #3f0b39
     164  296 - #370525
     212  296 - #370526
     364  312 - #f12121
     188  316 - #370525
     148  320 - #370525
     228  320 - #370525
     452  320 - #f12121
     288  372 - #f12121
     352  376 - #f12121
       4  416 - #2e4ae9
     592  420 - #2e4aea
     288  424 - #f12121
     236  432 - #f12121
     336  436 - #f12121
     468  468 - #2e4aea
     112  496 - #2e4ae9
       4  592 - #2e4ae9
     208  592 - #2e4ae9
     416  592 - #2e4ae9
     592  592 - #2e4aea
";

/// Both boxes unlit.
const ALL_UNLIT: &str = r"
       4    4 - #597c95
     268    4 - #597c95
     360    4 - #597c95
     500    4 - #597c95
     592    4 - #597c95
     136   48 - #597c95
     432   64 - #597c95
       4   96 - #597c95
     532   96 - #597c95
     284  124 - #597c95
     372  132 - #597c95
     184  136 - #597c95
      64  168 - #597c95
     584  188 - #597c95
     196  216 - #f12121
     360  220 - #903686
     360  224 - #c12b53
     148  228 - #f12121
     452  228 - #f12121
     240  236 - #6040b8
       4  240 - #2f4aea
     240  240 - #903686
     240  244 - #c12b53
     188  264 - #f12121
     404  268 - #f12121
     568  304 - #2f4aea
     216  308 - #f12121
     364  312 - #f12121
     148  320 - #f12121
     452  320 - #f12121
       4  336 - #2e4aea
     248  372 - #f12121
     316  372 - #f12121
     356  396 - #f12121
     592  420 - #2e4aea
     260  436 - #f12121
     328  436 - #f12121
       4  440 - #2e4ae9
     464  464 - #2e4aea
     104  472 - #2e4ae9
     192  500 - #2e4ae9
     308  548 - #2e4ae9
     504  552 - #2e4aea
     104  576 - #2e4ae9
       4  592 - #2e4ae9
     204  592 - #2e4ae9
     412  592 - #2e4ae9
     592  592 - #2e4aea
";
