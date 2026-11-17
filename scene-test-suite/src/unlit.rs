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
     352    4 - #597c95
     592    4 - #597c95
     176   48 - #597c95
     468   88 - #597c95
     284  104 - #597c95
     380  120 - #597c95
     104  124 - #597c95
       4  160 - #597c95
     592  160 - #597c95
     160  216 - #5e0f46
     184  216 - #5e0f46
     208  216 - #5e0f46
     236  216 - #5e1046
     364  216 - #f12121
     228  232 - #5e0f46
     188  236 - #5e0f45
     460  236 - #f12121
     136  240 - #370526
     164  244 - #370526
     220  248 - #370526
     240  252 - #3e0b39
     408  252 - #f12121
     152  268 - #370526
     240  276 - #3e0b39
     360  276 - #f12121
     184  280 - #370526
      20  288 - #2f4aea
     212  288 - #370526
     456  296 - #f12121
     144  300 - #370525
     300  300 - #2e4aea
     236  308 - #3e0b38
     168  320 - #370525
     200  320 - #370525
     400  320 - #f12121
     288  376 - #f12121
     592  388 - #2f4aea
       4  412 - #2e4aea
     360  416 - #f12121
     236  436 - #f12121
     300  436 - #f12121
     484  476 - #2e4aea
     112  480 - #2e4ae9
       4  592 - #2e4ae9
     172  592 - #2e4ae9
     408  592 - #2e4aea
     592  592 - #2e4aea
";

/// Both boxes unlit.
const ALL_UNLIT: &str = r"
       4    4 - #597c95
      96    4 - #597c95
     204    4 - #597c95
     296    4 - #597c95
     388    4 - #597c95
     500    4 - #597c95
     592    4 - #597c95
     444   72 - #597c95
     152   76 - #597c95
      48   80 - #597c95
     256  108 - #597c95
     500  148 - #597c95
      96  152 - #597c95
       4  160 - #597c95
     352  160 - #597c95
     592  160 - #597c95
     236  216 - #f12121
     408  216 - #f12121
     460  236 - #f12121
     136  240 - #f12121
     364  244 - #f12121
     188  260 - #f12121
     412  268 - #f12121
      24  276 - #2f4aea
     240  276 - #f12121
     568  276 - #2f4bea
     456  296 - #f12121
     144  300 - #f12121
     300  300 - #2e4aea
     200  320 - #f12121
     400  320 - #f12121
     104  360 - #2e4aea
     288  376 - #f12121
     592  388 - #2f4aea
       4  392 - #2e4aea
     360  416 - #f12121
     236  436 - #f12121
     300  436 - #f12121
     108  476 - #2e4ae9
     488  488 - #2e4aea
       8  492 - #2e4ae9
     588  492 - #2e4aea
     276  548 - #2e4aea
       4  592 - #2e4ae9
     172  592 - #2e4ae9
     380  592 - #2e4aea
     484  592 - #2e4aea
     592  592 - #2e4aea
";
