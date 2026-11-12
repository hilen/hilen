use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{
        Camera, Material, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, SceneTestView,
        scene,
    },
    ui::{Color, Label, ViewData, ViewFrame, ViewSubviews},
    ui_test::{check_colors, set_record_probe_count},
};

use crate::leaves::leaves;

const LABEL_WIDTH: f32 = 120.0;
const LABEL_HEIGHT: f32 = 26.0;
/// A threshold low enough that every leaf grows close to its rim.
const WIDE: f32 = 0.15;
/// Half the side of a quad, and how far over a fixture its label sits.
const HALF: f32 = 1.0;
const LABEL_RISE: f32 = 1.35;
/// Where the middle of each fixture is and what it is, in 2 rows.
const FIXTURES: [(f32, f32, &str); 4] = [
    (-1.5, 4.4, "not cut"),
    (1.5, 4.4, "cut out"),
    (-1.5, 1.4, "crossed"),
    (1.5, 1.4, "glb, MASK"),
];

/// Quads with a texture of round leaves in front of an orange wall. The
/// first is not cut out and draws the whole quad, the dark green between
/// its leaves included. The second is cut out, so the wall shows between
/// its leaves. The third is 2 cut out quads crossed at a right angle, a
/// green one and a brown one, each in front of the other on one half,
/// so each hides the other by depth there and shows through its gaps.
/// The last is a `.glb` whose material is masked. Then the threshold of
/// the cut out quads drops and their leaves grow.
#[scene]
#[derive(Default)]
struct Cutout {
    cut: Vec<Weak<Prop>>,
}

impl Cutout {
    /// An upright quad of leaves facing the camera, turned by `turn`
    /// around the vertical.
    fn quad(&mut self, (x, y, _): (f32, f32, &str), turn: f32, color: &str) -> Weak<Prop> {
        let mut quad = self.make_node::<Prop>(Shape3::Plane(HALF * 2.0), Vec3::new(x, y, 0.0));
        quad.set_material(Material {
            color: Color::hex(color),
            roughness: 0.8,
            texture: Some(leaves()),
            ..Material::default()
        })
        .set_rotation(Quat::from_rotation_y(turn) * Quat::from_rotation_x(FRAC_PI_2));
        quad
    }
}

impl SceneSetup for Cutout {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 3.2, 6.8),
            target: Vec3::new(0.0, 3.2, 0.0),
            ..Camera::default()
        };

        self.make_node::<Prop>(Shape3::Plane(16.0), Vec3::ZERO)
            .set_color(Color::hex("#b0b8c0"))
            .set_roughness(0.9);
        self.make_node::<Prop>(Shape3::cuboid(13.0, 8.0, 0.2), Vec3::new(0.0, 4.0, -2.0))
            .set_color(Color::hex("#d35400"))
            .set_roughness(0.8);

        self.quad(FIXTURES[0], 0.0, "#ffffff");

        let mut cut = self.quad(FIXTURES[1], 0.0, "#ffffff");
        cut.set_cutout(Material::CUTOUT);
        self.cut.push(cut);

        for (turn, color) in [(FRAC_PI_4, "#ffffff"), (-FRAC_PI_4, "#ff7050")] {
            let mut crossed = self.quad(FIXTURES[2], turn, color);
            crossed.set_cutout(Material::CUTOUT);
            self.cut.push(crossed);
        }

        self.make_node::<Prop>(
            Shape3::Model(Model::get("leaf_card.glb")),
            // The model's origin is at its foot.
            Vec3::new(FIXTURES[3].0, FIXTURES[3].1 - HALF, 0.0),
        );
    }
}

impl SceneTest for Cutout {
    fn overlay(scene: Weak<Self>, view: Weak<SceneTestView>) {
        for (x, y, text) in FIXTURES {
            let label = view.add_view::<Label>();
            label.set_text(text).set_text_size(16);
            label.set_text_color(Color::hex("#ffffff"));
            label
                .set_color(Color::hex("#303848"))
                .set_frame((0.0, 0.0, LABEL_WIDTH, LABEL_HEIGHT));
            if let Some(point) = scene.view_point(Vec3::new(x, y + LABEL_RISE, 0.0)) {
                label.set_center(point);
            }
        }
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        check_colors(CUT)?;

        from_main(move || {
            for mut quad in scene.cut.iter().copied() {
                quad.set_cutout(WIDE);
            }
        });
        wait_for_next_frame();
        check_colors(WIDER)
    }
}

/// The whole quad, the cut out one, the crossed pair and the model.
const CUT: &str = r"
      60    4 - #597c95
     120    4 - #597c95
     284    4 - #597c95
     480    4 - #597c95
     592    4 - #597c95
     380    8 - #597c95
     160   12 - #597c95
     204   12 - #597c95
     332   12 - #597c95
     432   12 - #597c95
     536   12 - #597c95
       4   16 - #ad4c24
      64   92 - #ad4c24
     528   92 - #ae4e29
     128   96 - #303848
     204   96 - #303848
     244   96 - #303848
     356   96 - #303848
     400  104 - #c4c6cb
     404  104 - #9da1a8
     432  104 - #ffffff
     436  104 - #ffffff
     168  108 - #787d88
     432  108 - #ffffff
     220  136 - #2a402d
     260  136 - #2b402d
     148  140 - #293f2c
     592  144 - #ae4e27
     396  148 - #408849
     356  152 - #408849
     432  152 - #408849
     472  152 - #408849
       4  156 - #ad4c23
     112  164 - #293f2c
     180  172 - #293f2c
     248  176 - #3f8848
     356  188 - #3f8848
     424  188 - #3f8848
     112  204 - #283e2b
     220  208 - #293f2c
     148  212 - #283f2b
     480  224 - #3f8848
     564  224 - #ad4d26
     256  228 - #3e8747
     356  228 - #3f8848
     404  228 - #3f8848
      32  232 - #ad4b23
     192  236 - #3e8747
     112  244 - #283e2b
     432  264 - #3f8748
     472  264 - #3f8848
     392  268 - #3e8748
     352  272 - #3e8747
     148  280 - #283e2b
     212  280 - #3e8747
     260  280 - #283f2b
     592  304 - #ad4c24
       4  312 - #ad4b22
     468  324 - #303848
     212  332 - #bdc0c5
     388  332 - #bcbfc4
     408  332 - #a7abb1
     168  336 - #303848
     388  336 - #bcbfc4
     392  336 - #ffffff
     408  336 - #a7abb1
     128  344 - #303848
     396  364 - #2f6736
     396  368 - #2f6736
     428  368 - #2f6736
     228  372 - #408d49
     360  372 - #2f6736
     388  372 - #2f6736
     396  372 - #2f6736
     176  376 - #2f321f
     480  384 - #2f6736
     448  388 - #ad4b23
     392  392 - #ad4b23
     128  396 - #408d4a
     440  396 - #ad4b23
     576  396 - #ad4c23
      48  400 - #ad4b21
     380  404 - #ad4b23
     396  404 - #2f6736
     432  404 - #ad4b23
     396  408 - #2f6736
     232  412 - #3f8d49
     360  416 - #2f6635
     192  420 - #2f321f
     480  424 - #2f6736
     448  428 - #ad4b22
     392  432 - #ad4b22
     432  440 - #ad4b22
     396  444 - #2f6736
     388  448 - #2f6736
     396  448 - #2f6736
     156  452 - #2f321f
     460  452 - #ad4b22
     200  464 - #2f321f
     420  464 - #2f6736
     448  464 - #ad4b22
     364  468 - #2f6736
     392  468 - #ad4b22
     480  468 - #2f6736
     396  484 - #2f6736
     440  484 - #408d49
     460  484 - #ad4b22
       4  488 - #ad4b21
     384  492 - #a8b0b7
     580  492 - #a8b0b7
     388  496 - #a8b0b7
     168  500 - #3f8d49
     304  500 - #a8b0b7
     224  508 - #2f321f
     428  512 - #2f6736
     456  512 - #2f6736
     128  520 - #3f8d49
     480  520 - #2f6736
      60  528 - #a8afb7
     268  556 - #a8afb7
       4  568 - #a8afb7
     144  584 - #a8afb7
      80  592 - #a8afb7
     208  592 - #a8afb7
     324  592 - #a8afb7
     420  592 - #a8afb7
     512  592 - #a8afb7
     592  592 - #a8b0b7
";

/// The leaves of the cut out quads grown to the lower threshold.
const WIDER: &str = r"
       4    4 - #597c95
     120    4 - #597c95
     284    4 - #597c95
     480    4 - #597c95
     380    8 - #597c95
      64   12 - #597c95
     160   12 - #597c95
     204   12 - #597c95
     332   12 - #597c95
     432   12 - #597c95
     536   12 - #597c95
     592   16 - #ae4f2b
     528   88 - #ae4e29
      40   92 - #ad4c24
     128   96 - #303848
     244   96 - #303848
     356   96 - #303848
     400  104 - #c4c6cb
     404  104 - #9da1a8
     432  104 - #ffffff
     436  104 - #ffffff
     168  108 - #787d88
     432  108 - #ffffff
     112  136 - #293f2c
     148  136 - #293f2c
     260  136 - #2b402d
     224  140 - #2a402d
     344  148 - #408849
     484  148 - #408849
     392  156 - #408849
     592  156 - #ae4d27
     184  160 - #293f2c
       4  168 - #ad4b23
     140  172 - #293f2c
     256  172 - #2a3f2d
     432  184 - #3f8848
     476  192 - #3f8848
     112  212 - #283e2b
     256  212 - #293f2c
     192  216 - #3e8747
     364  216 - #3f8848
     564  236 - #ad4d25
     224  240 - #293f2c
     480  240 - #3f8848
     116  248 - #283e2b
     408  264 - #3f8748
     184  276 - #283e2b
     348  276 - #3e8747
       4  280 - #ad4b22
     148  280 - #283e2b
     220  280 - #283e2b
     260  280 - #283f2b
     592  312 - #ad4c24
     168  324 - #303848
     472  324 - #303848
     212  332 - #bdc0c5
     388  332 - #bcbfc4
     408  332 - #a7abb1
     388  336 - #bcbfc4
     392  336 - #ffffff
     408  336 - #a7abb1
     128  340 - #303848
      48  344 - #ad4b22
     172  364 - #2f321f
     396  364 - #2f6736
     444  364 - #408d49
     360  368 - #2f6736
     396  368 - #2f6736
     420  368 - #408d49
     204  372 - #408d49
     388  372 - #2f6736
     396  372 - #2f6736
     284  380 - #ad4b22
     436  384 - #2f6736
     460  384 - #2f6736
     484  388 - #254529
     180  396 - #ad4b22
     576  400 - #ad4b23
     144  404 - #ad4b22
     200  404 - #408d49
     360  404 - #2f6736
     396  404 - #2f6736
     428  404 - #408d49
       4  408 - #ad4b21
     212  408 - #408d49
     396  408 - #2f6736
     448  408 - #408d49
     152  424 - #ad4b22
     208  428 - #ad4b22
     360  436 - #2f6736
     472  436 - #2f6736
     432  440 - #ad4b22
     164  444 - #ad4b21
     204  444 - #3f8d49
     396  444 - #2f6736
     420  444 - #408d49
     448  444 - #408d49
     216  448 - #ad4b22
     388  448 - #2f6736
     396  448 - #2f6736
     140  452 - #ad4b21
     476  464 - #2f6736
     208  468 - #ad4b21
     180  472 - #ad4b21
     360  472 - #2f6736
     428  480 - #408d49
     140  484 - #ad4b21
     396  484 - #2f6736
     460  484 - #ad4b22
     212  488 - #3f8d49
     216  492 - #a8b0b7
     384  492 - #a8b0b7
     576  492 - #a8b0b7
     388  496 - #a8b0b7
       4  508 - #a8b0b7
     456  508 - #2f6736
     292  512 - #a8b0b7
     428  512 - #2f6736
      76  520 - #a8afb7
     476  524 - #2f6736
     124  528 - #3f8d49
     232  560 - #a8afb7
      52  592 - #a8afb7
     168  592 - #a8afb7
     296  592 - #a8afb7
     388  592 - #a8afb7
     512  592 - #a8afb7
     592  592 - #a8b0b7
";
