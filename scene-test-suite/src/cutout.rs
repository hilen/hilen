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
            let mut label = view.add_view::<Label>();
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
     120    4 - #597c95
     296    4 - #597c95
     416    4 - #597c95
     592    4 - #597c95
     504    8 - #597c95
       4   16 - #ad4c24
     128   92 - #5d3f3c
     196   92 - #5d403c
     228   92 - #5d403c
     356   92 - #5d403c
     468   92 - #5d403d
     400  104 - #c4c6cb
     404  104 - #9da1a8
     432  104 - #ffffff
     436  104 - #ffffff
     168  108 - #787d88
     432  108 - #ffffff
     112  132 - #293f2c
     204  132 - #3f8848
     224  132 - #2a402d
     184  136 - #2a3f2c
     160  140 - #3e8748
     128  144 - #3e8747
     260  144 - #35683b
     208  148 - #3f8848
     356  148 - #408849
     236  152 - #3f8848
     472  152 - #408849
     112  168 - #293f2b
     176  168 - #293f2c
     252  168 - #2a3f2d
     140  176 - #3e8747
     192  176 - #3e8747
     216  176 - #3f8748
     232  184 - #3f8748
     260  188 - #3f8848
     392  188 - #3f8848
     120  192 - #3e8747
     164  192 - #3e8747
     140  200 - #3e8747
     192  200 - #3e8747
       4  208 - #ad4b23
     232  212 - #3e8747
     120  216 - #3e8747
     260  216 - #293f2c
     204  220 - #3e8747
     472  220 - #3f8848
     160  228 - #3e8747
     396  228 - #3f8848
     136  232 - #3e8747
     240  232 - #3e8747
     592  232 - #ae4d26
     188  236 - #3e8747
     220  236 - #3e8747
     260  244 - #293f2c
     112  248 - #283e2b
     144  252 - #283e2b
     172  256 - #3e8747
     208  260 - #3e8747
     240  260 - #3e8747
     124  264 - #3d8747
     356  264 - #3e8747
     424  264 - #3f8748
     472  264 - #3f8848
     188  280 - #283e2b
     112  284 - #494128
     116  284 - #494129
     120  284 - #494129
     128  284 - #59783e
     148  284 - #494229
     164  284 - #59783e
     168  284 - #5a783e
     176  284 - #494229
     204  284 - #5a783e
     212  284 - #494229
     220  284 - #494229
     232  284 - #494229
     240  284 - #5a783e
     244  284 - #5a783e
     252  284 - #494229
     256  284 - #494229
       4  328 - #ad4b22
     444  328 - #303848
     212  332 - #bdc0c5
     388  332 - #bcbfc4
     408  332 - #a7abb1
     388  336 - #bcbfc4
     392  336 - #ffffff
     408  336 - #a7abb1
     156  340 - #303848
     244  344 - #303848
     360  368 - #2f6736
     388  372 - #2f6736
     396  372 - #2f6736
     192  380 - #2f321f
     132  388 - #408d4a
     224  388 - #2f321f
     448  388 - #ad4c23
     484  388 - #254529
     432  404 - #ad4b23
     360  408 - #2f6736
     396  408 - #2f6736
     224  424 - #2f321f
     168  428 - #408d49
     448  428 - #ad4b23
     432  440 - #ad4b22
     396  444 - #2f6736
       4  448 - #ad4b21
     388  448 - #2f6736
     460  452 - #ad4b22
     592  460 - #ad4b22
     216  472 - #2f321f
     360  472 - #2f6736
     152  480 - #2f321f
     396  484 - #2f6736
     460  484 - #ad4b22
     384  492 - #a8afb7
     388  496 - #a8afb7
     192  500 - #2f321f
     428  512 - #2f6736
      68  516 - #a8afb7
     128  520 - #3f8d49
     220  520 - #2f321f
     476  524 - #2f6736
       4  592 - #a8afb7
     120  592 - #a8afb6
     304  592 - #a8afb6
     592  592 - #a8afb7
";

/// The leaves of the cut out quads grown to the lower threshold.
const WIDER: &str = r"
     120    4 - #597c95
     252    4 - #597c95
     384    4 - #597c95
     480    4 - #597c95
     592    4 - #597c95
       4   16 - #ad4c24
     128   92 - #5d3f3c
     204   92 - #5d403c
     240   92 - #5d403c
     356   92 - #5d403c
     460   92 - #5d403d
     400  104 - #c4c6cb
     404  104 - #9da1a8
     428  104 - #303848
     432  104 - #ffffff
     436  104 - #ffffff
     168  108 - #787d88
     432  108 - #ffffff
     472  116 - #303848
     560  128 - #ae4e28
       4  132 - #ad4c23
     112  132 - #293f2c
     204  132 - #3f8848
     224  132 - #2a402d
     148  136 - #293f2c
     128  144 - #3e8747
     168  144 - #3e8748
     260  144 - #35683b
     236  156 - #3f8848
     484  156 - #408849
     200  160 - #3f8748
     112  168 - #293f2b
     256  172 - #2a3f2d
     140  176 - #3e8747
     192  176 - #3e8747
     216  176 - #3f8748
     172  180 - #3e8747
     260  188 - #3f8848
     240  192 - #3f8748
     128  196 - #3e8747
     204  196 - #3e8747
     156  200 - #3e8747
     356  204 - #3f8848
     224  208 - #293f2c
     112  212 - #283e2b
     176  216 - #3e8747
     200  216 - #3e8747
     260  216 - #293f2c
     240  220 - #3e8747
     144  224 - #3e8747
     124  228 - #3e8747
     220  228 - #3e8747
     464  228 - #3f8848
     164  232 - #3e8747
     196  236 - #3e8747
     256  244 - #293f2c
     116  248 - #283e2b
     140  248 - #283e2b
     592  248 - #ae4d25
     164  252 - #3e8747
     228  256 - #3e8747
     400  260 - #3f8748
     248  264 - #3e8747
     200  268 - #3e8747
     112  284 - #494128
     116  284 - #494129
     120  284 - #494129
     128  284 - #59783e
     152  284 - #494229
     164  284 - #59783e
     168  284 - #5a783e
     172  284 - #494229
     188  284 - #494229
     204  284 - #5a783e
     216  284 - #494229
     228  284 - #494229
     240  284 - #5a783e
     244  284 - #5a783e
     252  284 - #494229
     256  284 - #494229
     460  324 - #303848
     128  328 - #303848
     212  332 - #bdc0c5
     388  332 - #bcbfc4
     408  332 - #a7abb1
     176  336 - #303848
     388  336 - #bcbfc4
     392  336 - #ffffff
     408  336 - #a7abb1
     436  336 - #303848
     156  364 - #2f321f
     200  364 - #408d4a
     396  364 - #2f6736
     444  364 - #408d49
     420  368 - #408d49
     388  372 - #2f6736
       4  388 - #ad4b21
     484  388 - #254529
     208  392 - #ad4b22
     180  396 - #ad4b22
     368  400 - #2f6736
     144  404 - #ad4b22
     428  404 - #408d49
     396  408 - #2f6736
     480  424 - #2f6736
     176  440 - #2f321f
     212  440 - #ad4b22
     360  444 - #2f6736
     396  444 - #2f6736
     448  444 - #408d49
     388  448 - #2f6736
     140  452 - #ad4b21
     484  472 - #254528
     176  476 - #2f321f
     144  484 - #ad4b21
     396  484 - #2f6736
     440  484 - #408d49
      20  492 - #a8afb7
     216  492 - #a8afb7
     384  492 - #a8afb7
     388  496 - #a8afb7
     428  512 - #2f6736
     480  520 - #2f6736
     120  524 - #3f8d49
       4  592 - #a8afb7
     196  592 - #a8afb6
     312  592 - #a8afb6
     592  592 - #a8afb7
";
