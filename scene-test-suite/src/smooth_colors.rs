use std::sync::mpsc::channel;

use anyhow::{Result, ensure};
use hilen::{
    dispatch::{from_main, spawn, wait_for_next_frame},
    gm::{
        LossyConvert,
        color::U8Color,
        volume::{Shape3, Vec3},
    },
    refs::Weak,
    scene::{
        Camera, MeshData, Model, NodeTemplates, Prop, SceneCreation, SceneManager, SceneSetup, SceneTest,
        scene,
    },
    ui::Color,
    ui_test::{check_colors, checkpoint, set_record_probe_count},
};

use crate::geometry::vertex;

const RED: U8Color = U8Color::const_rgb(230, 40, 30);
const BLUE: U8Color = U8Color::const_rgb(30, 60, 230);
const GREEN: U8Color = U8Color::const_rgb(40, 200, 60);
const YELLOW: U8Color = U8Color::const_rgb(240, 220, 40);
/// The side of a quad.
const SIDE: f32 = 1.6;
/// The middles of the 3 columns and the 2 rows.
const COLUMNS: [f32; 3] = [-2.0, 0.0, 2.0];
const ROWS: [f32; 2] = [1.2, -1.2];
const CAMERA_FROM: f32 = 8.0;
/// A photo of the whole canvas in meters is this size in pixels.
const PHOTO: (u32, u32) = (400, 400);

fn with_alpha(color: U8Color, a: u8) -> U8Color {
    U8Color { a, ..color }
}

/// A quad facing the camera, its corners from the lower left counter
/// clockwise, 1 color each, 2 triangles over 4 shared vertices.
fn quad(colors: [U8Color; 4]) -> MeshData {
    let half = SIDE / 2.0;
    let corners = [
        Vec3::new(-half, -half, 0.0),
        Vec3::new(half, -half, 0.0),
        Vec3::new(half, half, 0.0),
        Vec3::new(-half, half, 0.0),
    ];
    MeshData {
        vertices: corners
            .iter()
            .zip(colors)
            .map(|(&pos, color)| vertex(pos, Vec3::Z, color))
            .collect(),
        indices:  vec![0, 1, 2, 0, 2, 3],
    }
}

/// The same quad the way a flat mesh has to color it, 1 color a triangle,
/// so the 2 colors meet along its diagonal.
fn flat_quad(lower: U8Color, upper: U8Color) -> MeshData {
    let half = SIDE / 2.0;
    let at = |x: f32, y: f32, color: U8Color| vertex(Vec3::new(x, y, 0.0), Vec3::Z, color);
    MeshData {
        vertices: vec![
            at(-half, -half, lower),
            at(half, -half, lower),
            at(half, half, lower),
            at(-half, -half, upper),
            at(half, half, upper),
            at(-half, half, upper),
        ],
        indices:  vec![0, 1, 2, 3, 4, 5],
    }
}

/// Fades that only a mesh from `Model::from_smooth_mesh` can draw, next to
/// a flat mesh, over a gray wall under a low sun that casts. The top row is
/// unlit, so its colors stand as they are: a flat quad with its 2 colors
/// meeting along a saw line, a smooth quad fading from red on the left to
/// blue on the right, and one with 4 colors on its 4 corners. The bottom
/// row is lit and tries each way a node can draw: the alpha of the colors
/// fades from full on the left to none on the right under a cutout of a
/// half, so only the left half stands and only that half casts a shadow,
/// a node color with alpha under one blends the fade over the wall, and a
/// fade of a half dithers it and its shadow. Then the wireframe of the
/// meshes is drawn over them, and a depth photo sees the wall through the
/// half that is cut away.
#[scene]
#[derive(Default)]
struct SmoothColors {}

impl SmoothColors {
    fn smooth(&mut self, name: &str, mesh: MeshData, at: (f32, f32)) -> Weak<Prop> {
        let mut node = self.make_node::<Prop>(
            Shape3::Model(Model::from_smooth_mesh(name, mesh)),
            Vec3::new(at.0, at.1, 0.0),
        );
        node.set_color(Color::hex("#ffffff")).set_roughness(0.8);
        node
    }
}

impl SceneSetup for SmoothColors {
    fn setup(&mut self) {
        self.camera = Camera::orthographic(Vec3::new(0.0, 0.0, CAMERA_FROM), Vec3::ZERO, 6.0);
        self.sun.direction = Vec3::new(-0.4, -0.5, -1.0);
        self.sun.shadows = true;
        // The default is 1024 on a phone and in the browser.
        self.sun.shadow_map_size = 2048;

        let wall = flat_quad(
            U8Color::const_rgb(255, 255, 255),
            U8Color::const_rgb(255, 255, 255),
        );
        let mut wall = self.make_node::<Prop>(
            Shape3::Model(Model::from_mesh("Smooth colors wall", wall)),
            Vec3::new(0.0, 0.0, -1.5),
        );
        wall.set_color(Color::hex("#9aa3aa")).set_roughness(0.9).set_scale(10.0);

        let mut flat = self.make_node::<Prop>(
            Shape3::Model(Model::from_mesh("Smooth colors flat", flat_quad(RED, BLUE))),
            Vec3::new(COLUMNS[0], ROWS[0], 0.0),
        );
        flat.set_color(Color::hex("#ffffff")).set_unlit(true);

        let fade = quad([RED, BLUE, BLUE, RED]);
        self.smooth("Smooth colors fade", fade.clone(), (COLUMNS[1], ROWS[0]))
            .set_unlit(true);
        self.smooth(
            "Smooth colors corners",
            quad([RED, BLUE, GREEN, YELLOW]),
            (COLUMNS[2], ROWS[0]),
        )
        .set_unlit(true);

        let clear = |color: U8Color| with_alpha(color, 0);
        self.smooth(
            "Smooth colors cutout",
            quad([RED, clear(BLUE), clear(BLUE), RED]),
            (COLUMNS[0], ROWS[1]),
        )
        .set_cutout(0.5);
        self.smooth("Smooth colors blended", fade.clone(), (COLUMNS[1], ROWS[1]))
            .set_color(Color::hex("#ffffff").with_alpha(0.5));
        self.smooth("Smooth colors dithered", fade, (COLUMNS[2], ROWS[1])).set_fade(0.5);
    }
}

impl SceneTest for SmoothColors {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        set_record_probe_count(64);
        wait_for_next_frame();
        check_colors(FADES)?;

        from_main(move || {
            let mut scene = scene;
            scene.show_wireframe = true;
        });
        wait_for_next_frame();
        check_colors(WIREFRAME)?;
        from_main(move || {
            let mut scene = scene;
            scene.show_wireframe = false;
        });

        // The height of 6 meters fills the photo, so a meter is a sixth of
        // its side, and the lower left quad stands left of its middle.
        let camera = from_main(move || scene.camera);
        let (sender, receiver) = channel();
        from_main(move || {
            let depth = SceneManager::depth_photo(camera, PHOTO);
            spawn(async move {
                sender.send(depth.await).expect("the test waits for the depth photo");
            });
        });
        let depth = receiver.recv()??;
        let pixel = |meters: f32, size: u32| -> u32 {
            let size: f32 = size.lossy_convert();
            (size * (0.5 + meters / 6.0)).lossy_convert()
        };
        let y = PHOTO.1 - pixel(ROWS[1], PHOTO.1);
        let stands = depth.at(pixel(COLUMNS[0] - 0.4, PHOTO.0), y).unwrap_or(f32::NAN);
        let cut = depth.at(pixel(COLUMNS[0] + 0.4, PHOTO.0), y).unwrap_or(f32::NAN);
        ensure!(
            (stands - CAMERA_FROM).abs() < 0.01,
            "the half of the cutout that stands is {stands} m away"
        );
        ensure!(
            (cut - (CAMERA_FROM + 1.5)).abs() < 0.01,
            "through the half that is cut the wall is {cut} m away"
        );
        checkpoint("a depth photo stops on the left half of the cutout and sees the wall through the right")
    }
}

const FADES: &str = r"
               4    4 - #969ea5
             248  100 - #c22c42
             340  100 - #4f37b5
             496  100 - #90d232
             552  100 - #4acb39
             420  104 - #ebd32b
             576  104 - #2cc43c
             460  112 - #bdc92c
              20  128 - #1f3ce6
             576  128 - #2baf56
             512  132 - #7aac32
             144  140 - #e6281f
             480  140 - #a2a72d
             544  140 - #52a03b
             340  148 - #4f37b5
             420  148 - #eaa427
             560  148 - #3d9b57
             452  152 - #c49d29
             508  156 - #7e9130
             576  156 - #299674
             540  160 - #568e4b
             472  164 - #aa8d2b
             524  168 - #69853f
             556  172 - #41866c
             572  172 - #2d8880
             492  176 - #917d2c
             444  180 - #cc7e26
             512  180 - #77793d
             420  184 - #e87c24
             316  188 - #6d3497
             548  188 - #4a7773
             576  188 - #277a96
             468  192 - #ae6e28
             504  196 - #806a44
             352  204 - #4039c4
             444  204 - #cb6325
             476  208 - #a35c2e
             552  208 - #44668d
             420  212 - #e75d22
             496  212 - #895b4b
             576  212 - #2565af
             444  220 - #ca5124
             464  220 - #b1502c
             572  224 - #2a5ab7
             460  228 - #b5492f
             484  228 - #974c4d
             512  228 - #744f70
             540  228 - #515393
             576  236 - #2450c9
             352  244 - #4039c4
             180  248 - #555a5d
              28  256 - #e6281f
             320  300 - #91999f
             224  340 - #b76c6e
              32  344 - #ce3b3e
              96  356 - #85407d
             376  376 - #6773bf
             288  408 - #926f8d
              72  420 - #a03e64
             376  476 - #5d68b3
              88  496 - #8e4075
             220  496 - #b96c6d
               4  572 - #555a5d
             512  572 - #7b8287
";

const WIREFRAME: &str = r"
               4    4 - #969ea5
              92  104 - #1f3ce6
             340  104 - #4f37b5
             424  104 - #e8d52a
             512  104 - #7ccb33
             576  116 - #2bb949
             176  120 - #e6281f
             448  132 - #cab42a
             488  132 - #98af2f
             536  136 - #5ca434
             556  144 - #439e4e
             336  148 - #5437b0
             468  148 - #b09f2b
             292  152 - #8b3179
             576  152 - #299a6f
             424  156 - #e69b26
             504  168 - #82842e
             560  168 - #3c8a6c
             472  176 - #aa7f2a
             440  184 - #d17a25
             352  188 - #4039c4
             548  188 - #4a7773
             516  192 - #726f4f
             424  208 - #e46122
             552  208 - #44668d
             108  216 - #e6281f
             284  216 - #95306f
             444  216 - #ca5624
             496  216 - #89584f
             464  220 - #b1502c
             576  220 - #255eb8
             512  228 - #744f70
             576  236 - #2450c9
             544  256 - #4b3bb6
             320  272 - #969ea5
              20  288 - #555a5d
             468  332 - #555a5d
              96  344 - #85407d
             336  344 - #7871a8
              32  364 - #ce3b3e
             376  400 - #6773bf
             276  424 - #996e86
              60  428 - #ae3d57
             376  472 - #5b66b2
              96  488 - #85407d
             276  496 - #996e86
              36  568 - #555a5d
              32  572 - #555a5d
             392  572 - #7b8287
             512  572 - #7b8287
             176  592 - #969ea5
";
