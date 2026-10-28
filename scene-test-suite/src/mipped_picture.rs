use std::{
    f32::consts::{FRAC_PI_2, TAU},
    sync::mpsc::channel,
};

use anyhow::Result;
use hilen::{
    dispatch::{from_main, spawn, wait_for_next_frame},
    gm::volume::{Quat, Shape3, Vec3},
    refs::Weak,
    scene::{
        Camera, Material, NodeTemplates, Prop, SceneCreation, SceneManager, SceneSetup, SceneTest,
        SceneTestView, scene,
    },
    ui::{Color, Label, ViewData, ViewFrame, ViewSubviews},
    ui_test::{check_colors, set_record_probe_count},
};

const PICTURE: u32 = 256;
const PLAIN: &str = "mipped-picture-test-plain";
const MIPPED: &str = "mipped-picture-test-mipped";
const LABEL_WIDTH: f32 = 90.0;
const LABEL_HEIGHT: f32 = 26.0;
/// Each quad's middle along x and its side, full, a quarter and a 16th.
const QUADS: [(f32, f32); 3] = [(-2.0, 2.0), (0.6, 0.5), (2.2, 0.125)];
/// The rows, the plain picture on top and the mipped one below.
const PLAIN_Y: f32 = 3.7;
const MIPPED_Y: f32 = 1.3;
const LABEL_X: f32 = 3.5;
const BALLS: usize = 18;

/// A tree of balls of leaves in 2 greens on a trunk over nothing, seen
/// from the side by an orthographic camera, what a far tree atlas is
/// baked from. The balls lie at places of a fixed random walk.
#[scene]
#[derive(Default)]
struct TreeShot {}

impl SceneSetup for TreeShot {
    fn setup(&mut self) {
        self.camera = Camera::orthographic(Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO, 2.0);
        self.make_node::<Prop>(Shape3::cuboid(0.12, 0.7, 0.12), Vec3::new(0.0, -0.6, 0.0))
            .set_color(Color::hex("#6b4a2b"))
            .set_unlit(true);
        let mut seed: u32 = 7;
        let mut next = move || {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            f32::from(u16::try_from(seed >> 16).expect("16 bits fit")) / f32::from(u16::MAX)
        };
        for ball in 0..BALLS {
            let (turn, reach, radius) = (next() * TAU, next().sqrt() * 0.55, 0.1 + next() * 0.2);
            let at = Vec3::new(turn.cos() * reach, 0.2 + turn.sin() * reach * 0.8, 0.0);
            let color = if ball % 3 == 0 { "#2f6b37" } else { "#3f8d49" };
            self.make_node::<Prop>(Shape3::Ball(radius), at)
                .set_color(Color::hex(color))
                .set_unlit(true);
        }
    }
}

/// The same tree picture on cut out quads at full size, a quarter and a
/// 16th, over an orange wall. The top row has the plain picture of
/// `SceneManager::picture`, which has 1 level, so its small quads pick
/// scattered texels and the crown falls apart. The bottom row has
/// `SceneManager::mipped_picture` with the cutout threshold, so its
/// small quads keep the outline of the crown.
#[scene]
#[derive(Default)]
struct MippedPicture {
    plain:  Vec<Weak<Prop>>,
    mipped: Vec<Weak<Prop>>,
}

impl MippedPicture {
    /// A quad facing the camera, with no picture on it yet.
    fn quad(&mut self, (x, side): (f32, f32), y: f32) -> Weak<Prop> {
        let mut quad = self.make_node::<Prop>(Shape3::Plane(2.0), Vec3::new(x, y, 0.0));
        quad.set_material(Material {
            color: Color::hex("#ffffff"),
            unlit: true,
            cutout: Some(Material::CUTOUT),
            fade: 0.0,
            ..Material::default()
        })
        .set_rotation(Quat::from_rotation_x(FRAC_PI_2))
        .set_scale(side / 2.0);
        quad
    }
}

impl SceneSetup for MippedPicture {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 2.5, 7.4),
            target: Vec3::new(0.0, 2.5, 0.0),
            ..Camera::default()
        };
        self.make_node::<Prop>(Shape3::cuboid(13.0, 8.0, 0.2), Vec3::new(0.0, 3.0, -2.0))
            .set_color(Color::hex("#d35400"))
            .set_unlit(true);
        for quad in QUADS {
            let plain = self.quad(quad, PLAIN_Y);
            self.plain.push(plain);
            let mipped = self.quad(quad, MIPPED_Y);
            self.mipped.push(mipped);
        }
    }
}

impl SceneTest for MippedPicture {
    fn overlay(scene: Weak<Self>, view: Weak<SceneTestView>) {
        for (y, text) in [(PLAIN_Y, "plain"), (MIPPED_Y, "mipped")] {
            let mut label = view.add_view::<Label>();
            label.set_text(text).set_text_size(16);
            label.set_text_color(Color::hex("#ffffff"));
            label
                .set_color(Color::hex("#303848"))
                .set_frame((0.0, 0.0, LABEL_WIDTH, LABEL_HEIGHT));
            if let Some(point) = scene.view_point(Vec3::new(LABEL_X, y, 0.0)) {
                label.set_center(point);
            }
        }
    }

    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(128);

        let (sender, receiver) = channel();
        from_main(move || {
            let clear = Color::rgba(0.0, 0.0, 0.0, 0.0);
            let plain = SceneManager::picture(TreeShot::default(), PLAIN, (PICTURE, PICTURE), clear);
            let mipped = SceneManager::mipped_picture(
                TreeShot::default(),
                MIPPED,
                (PICTURE, PICTURE),
                clear,
                Some(Material::CUTOUT),
            );
            spawn(async move {
                sender.send((plain, mipped.await)).expect("the test waits for the pictures");
            });
        });
        let (plain, mipped) = receiver.recv()?;
        let mipped = mipped?;
        from_main(move || {
            for (quads, image) in [(&scene.plain, plain), (&scene.mipped, mipped)] {
                for mut quad in quads.iter().copied() {
                    quad.material.texture = Some(image);
                    quad.set_fade(1.0);
                }
            }
        });

        wait_for_next_frame();
        check_colors(LEAVES)
    }
}

/// Both rows at the 3 sizes.
const LEAVES: &str = r"
       4   52 - #d35400
     276   52 - #d35400
     548  124 - #d35400
     272  140 - #d35400
     428  148 - #d35400
      16  160 - #d35400
     156  160 - #2f6b37
     132  168 - #3f8d49
     188  168 - #2f6b37
     168  172 - #2f6b37
     148  188 - #3f8d49
     120  192 - #2f6b37
     196  192 - #2f6b37
     116  196 - #2f6b37
     172  196 - #3f8d49
     340  204 - #2f6b37
     348  204 - #2f6b37
     512  204 - #303848
     568  204 - #303848
     588  204 - #303848
     336  208 - #3f8d49
     340  208 - #3f8d49
     188  212 - #2f6b37
     208  212 - #2f6b37
     336  212 - #3f8d49
     340  212 - #3f8d49
     344  212 - #3f8d49
     352  212 - #2f6b37
     536  212 - #303848
     540  212 - #686e7a
     156  216 - #3f8d49
     344  216 - #3f8d49
     532  216 - #303848
     540  216 - #686e7a
     548  216 - #fefefe
     552  216 - #e4e5e7
     128  220 - #2f6b37
     132  224 - #2f6b37
     504  224 - #303848
     576  224 - #303848
     172  236 - #3f8d49
     156  240 - #6b4a2b
     160  248 - #6b4a2b
     156  256 - #6b4a2b
     160  268 - #6b4a2b
       4  272 - #d35400
     160  280 - #6b4a2b
     276  292 - #d35400
     420  296 - #d35400
     168  332 - #2f6b37
     184  332 - #2f6b37
     148  336 - #2f6b37
     120  344 - #3f8d49
     192  344 - #2f6b37
     136  352 - #3f8d49
      24  360 - #d35400
     152  360 - #3f8d49
     176  360 - #3f8d49
     120  364 - #2f6b37
     192  364 - #2f6b37
     116  368 - #2f6b37
     208  368 - #2f6b37
     136  372 - #3f8d49
     340  372 - #2f6b37
     344  372 - #2f6b37
     348  372 - #2f6b37
     504  372 - #303848
     520  372 - #303848
     552  372 - #303848
     588  372 - #303848
     340  376 - #3f8d49
     180  380 - #3f8d49
     340  380 - #3f8d49
     352  380 - #2f6b37
     156  384 - #3f8d49
     200  384 - #2f6b37
     340  384 - #3f8d49
     344  384 - #3f8d49
     508  384 - #303848
     540  384 - #303848
     568  384 - #303848
     128  388 - #2f6b37
     524  388 - #303848
     528  388 - #303848
     540  392 - #303848
     544  392 - #303848
     504  396 - #303848
     560  396 - #303848
     588  396 - #303848
     176  400 - #3f8d49
     156  412 - #6b4a2b
     160  428 - #6b4a2b
     160  448 - #6b4a2b
       4  452 - #d35400
     348  464 - #d35400
     428  472 - #d35400
     272  476 - #d35400
      84  496 - #597c95
     516  500 - #597c95
      20  524 - #597c95
     204  524 - #597c95
     456  532 - #597c95
     344  544 - #597c95
     592  552 - #597c95
     132  572 - #597c95
      68  580 - #597c95
     516  588 - #597c95
       4  592 - #597c95
     256  592 - #597c95
     432  592 - #597c95
";
