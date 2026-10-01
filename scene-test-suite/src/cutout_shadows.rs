use std::f32::consts::FRAC_PI_2;

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
/// Where each fixture stands on x and what it is, left to right.
const FIXTURES: [(f32, &str); 3] = [(-2.6, "not cut"), (0.2, "cut out"), (2.8, "glb, MASK")];

/// Quads of leaves floating over a light floor under a sun that casts.
/// The one not cut out throws the shadow of the whole quad. The cut out
/// one throws the shadow of its leaves alone, with the lit floor between
/// them, and so does the `.glb` whose material is masked, one shadow per
/// card. Then the threshold of the cut out quad drops and the leaves of
/// its shadow grow with the leaves drawn.
#[scene]
#[derive(Default)]
struct CutoutShadows {
    cut: Weak<Prop>,
}

impl CutoutShadows {
    /// An upright quad of leaves facing the camera and the sun.
    fn quad(&mut self, x: f32) -> Weak<Prop> {
        let mut quad = self.make_node::<Prop>(Shape3::Plane(2.0), Vec3::new(x, 1.8, 0.0));
        quad.set_material(Material {
            color: Color::hex("#ffffff"),
            roughness: 0.8,
            texture: Some(leaves()),
            ..Material::default()
        })
        .set_rotation(Quat::from_rotation_x(FRAC_PI_2));
        quad
    }
}

impl SceneSetup for CutoutShadows {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.2, 5.4, 6.6),
            target: Vec3::new(0.2, 1.0, -0.6),
            ..Camera::default()
        };
        self.sun.direction = Vec3::new(-0.35, -1.0, -0.7);
        self.sun.shadows = true;
        // The default is 1024 on a phone and in the browser.
        self.sun.shadow_map_size = 2048;

        self.make_node::<Prop>(Shape3::Plane(40.0), Vec3::ZERO)
            .set_color(Color::hex("#c8ccd0"))
            .set_roughness(0.9);

        self.quad(FIXTURES[0].0);

        self.cut = self.quad(FIXTURES[1].0);
        self.cut.set_cutout(Material::CUTOUT);

        self.make_node::<Prop>(
            Shape3::Model(Model::get("leaf_card.glb")),
            Vec3::new(FIXTURES[2].0, 0.8, 0.0),
        );
    }
}

impl SceneTest for CutoutShadows {
    fn overlay(scene: Weak<Self>, view: Weak<SceneTestView>) {
        for (x, text) in FIXTURES {
            let mut label = view.add_view::<Label>();
            label.set_text(text).set_text_size(16);
            label.set_text_color(Color::hex("#ffffff"));
            label
                .set_color(Color::hex("#303848"))
                .set_frame((0.0, 0.0, LABEL_WIDTH, LABEL_HEIGHT));
            if let Some(point) = scene.view_point(Vec3::new(x, 3.4, 0.0)) {
                label.set_center(point);
            }
        }
    }

    fn perform_test(mut scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        wait_for_next_frame();
        check_colors(SHADOWS)?;

        from_main(move || {
            scene.cut.set_cutout(WIDE);
        });
        wait_for_next_frame();
        check_colors(WIDER)
    }
}

/// The shadow of the whole quad, of the leaves and of the model's cards.
const SHADOWS: &str = r"
       4    4 - #597c95
     200    4 - #597c95
     392    4 - #597c95
     592    4 - #597c95
     492   48 - #597c95
     100   52 - #597c95
     296   56 - #597c95
      24  152 - #303848
     140  152 - #303848
     240  156 - #303848
     476  160 - #e6e7e9
     480  160 - #ffffff
     488  160 - #303848
     516  160 - #303848
     532  160 - #303848
      88  164 - #303848
     100  164 - #9da1a8
     312  164 - #ffffff
     476  164 - #e6e7e9
     480  164 - #ffffff
     496  164 - #d1d3d6
      60  168 - #303848
      64  168 - #f4f5f6
     100  168 - #9da1a8
     292  168 - #303848
     320  168 - #303848
     476  168 - #e6e7e9
     480  168 - #ffffff
     496  168 - #d1d3d6
     264  176 - #303848
     356  176 - #303848
     444  176 - #303848
     560  176 - #303848
      20  212 - #273f2a
      44  212 - #3e8b48
      88  212 - #273f2a
     112  212 - #3f8b48
     164  212 - #273f2b
     516  212 - #418e4a
      64  216 - #3e8b48
     152  220 - #3f8b48
     240  220 - #3f8b48
     120  224 - #3e8b48
     284  224 - #3f8b48
     320  224 - #3f8b48
      84  228 - #3e8b48
     136  228 - #3f8b48
      44  232 - #3e8b48
     356  232 - #3f8b48
     448  232 - #bcc0c3
     484  232 - #418e4a
     156  236 - #3e8b48
      28  240 - #273c2a
      92  240 - #273f2a
     116  240 - #3e8b48
      68  244 - #273d2a
     132  244 - #273e2a
     500  248 - #bcc0c3
     120  252 - #3e8b48
     148  252 - #3e8b48
     552  252 - #336f3a
      44  256 - #3e8b48
     108  256 - #3e8b48
     164  256 - #3e8b48
     248  256 - #3f8b48
     476  256 - #418e4a
     132  260 - #3e8b48
     316  260 - #3f8b48
      84  264 - #3e8b48
     348  264 - #3f8b48
     444  264 - #bcc0c3
     484  268 - #bcc0c3
      68  272 - #263a29
     100  272 - #263a29
     120  272 - #3e8b48
     152  272 - #3e8b48
     172  272 - #263b29
     136  276 - #263a29
      40  280 - #273c29
     524  280 - #336f3a
     280  284 - #3e8b48
     164  288 - #3e8b48
     248  288 - #3e8b48
     436  288 - #bcc0c3
      60  292 - #3e8b48
      84  292 - #3e8b48
     104  292 - #3e8b48
     124  292 - #3e8b48
     144  296 - #28432c
     356  296 - #6e7072
     472  296 - #418e4a
     396  304 - #6e7072
      72  308 - #3e8b48
      52  312 - #3e8b48
     216  312 - #6e7072
     264  312 - #6e7072
     312  312 - #3e8b48
     456  312 - #316b38
      92  316 - #3e8b48
     140  316 - #3e8b48
     160  316 - #3e8b48
     116  320 - #3e8b48
     180  320 - #3e8b48
     420  320 - #bcc0c3
     468  320 - #418e4a
     532  320 - #336f3a
     380  324 - #bcc0c3
      68  328 - #3e8b48
     132  328 - #3e8b48
     460  332 - #418e4a
     492  332 - #336f3a
     232  344 - #6e7072
     288  344 - #6e7072
     324  344 - #6e7072
     428  344 - #6e7072
     100  348 - #6e7072
     128  348 - #6e7072
     156  348 - #6e7072
     384  352 - #6e7072
     408  364 - #6e7072
     468  364 - #6e7072
     592  448 - #bcc0c3
       4  452 - #bcc0c3
     148  496 - #bcc0c3
     444  524 - #bcc0c3
       4  592 - #bcc0c3
     296  592 - #bcc0c3
     592  592 - #bcc0c3
";

/// The shadow of the cut out quad with its leaves grown.
const WIDER: &str = r"
      60    4 - #597c95
     212    4 - #597c95
     392    4 - #597c95
     592    4 - #597c95
     492   48 - #597c95
     136   60 - #597c95
     300   60 - #597c95
      24  152 - #303848
     140  152 - #303848
     244  152 - #303848
     356  152 - #303848
     476  160 - #e6e7e9
     480  160 - #ffffff
     488  160 - #303848
     500  160 - #303848
     516  160 - #303848
     532  160 - #303848
      88  164 - #303848
     100  164 - #9da1a8
     312  164 - #ffffff
     476  164 - #e6e7e9
     480  164 - #ffffff
     496  164 - #d1d3d6
      64  168 - #f4f5f6
     100  168 - #9da1a8
     292  168 - #303848
     320  168 - #303848
     476  168 - #e6e7e9
     480  168 - #ffffff
     496  168 - #d1d3d6
     264  176 - #303848
     444  176 - #303848
     560  176 - #303848
     520  200 - #418e4a
      20  212 - #273f2a
      40  212 - #3e8b48
      60  212 - #273f2a
     112  212 - #3f8b48
     164  212 - #273f2b
      92  216 - #273f2a
     324  216 - #3f8b48
     504  216 - #418e4a
     152  220 - #3f8b48
     484  220 - #418e4a
     140  224 - #3f8b48
      48  228 - #3e8b48
      76  228 - #3e8b48
     232  228 - #3f8b48
     116  232 - #3e8b48
     160  232 - #3f8b48
     448  232 - #bcc0c3
     480  236 - #418e4a
     556  236 - #336f3a
      28  240 - #273c2a
      84  244 - #3e8b48
     100  244 - #273e2a
      64  248 - #273d2a
     128  248 - #3e8b48
     152  248 - #3e8b48
     360  248 - #3f8b48
     508  248 - #418e4a
     288  252 - #3f8b48
      44  256 - #3e8b48
     112  256 - #3e8b48
     164  256 - #3e8b48
     480  256 - #418e4a
      80  260 - #3e8b48
     124  260 - #3e8b48
     144  264 - #3e8b48
     100  272 - #263a29
     120  272 - #3e8b48
     172  272 - #263b29
      68  276 - #263929
      40  280 - #273c29
     452  280 - #408e4a
     496  280 - #418e4a
     152  284 - #3e8b48
      56  292 - #3e8b48
      92  292 - #3e8b48
     124  292 - #3e8b48
     176  292 - #3e8b48
     140  296 - #3e8b48
     160  296 - #3e8b48
     472  296 - #418e4a
      76  304 - #253628
     108  304 - #253628
     396  304 - #6e7072
     164  308 - #3e8b48
     420  308 - #6e7072
      52  312 - #3e8b48
     208  312 - #6e7072
     236  312 - #6e7072
     264  312 - #6e7072
     300  312 - #6e7072
     364  312 - #bcc0c3
     456  312 - #316b38
     540  312 - #336f3a
      92  316 - #3e8b48
     140  316 - #3e8b48
      72  320 - #3e8b48
     116  320 - #3e8b48
     180  320 - #3e8b48
     468  320 - #418e4a
     132  328 - #3e8b48
     160  328 - #3e8b48
     440  328 - #bcc0c3
     220  332 - #6e7072
     460  332 - #418e4a
     392  336 - #bcc0c3
     244  340 - #bcc0c3
     300  340 - #bcc0c3
     328  344 - #6e7072
     372  344 - #6e7072
     440  344 - #408e4a
      64  348 - #6e7072
     100  348 - #6e7072
     156  348 - #6e7072
     268  348 - #6e7072
     516  348 - #336f3a
     408  352 - #bcc0c3
     468  364 - #6e7072
     588  448 - #bcc0c3
       8  464 - #bcc0c3
     164  504 - #bcc0c3
     460  516 - #bcc0c3
       4  592 - #bcc0c3
     328  592 - #bcc0c3
     592  592 - #bcc0c3
";
