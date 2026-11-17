use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::volume::{Shape3, Vec3, Vertex3D, flow_texel},
    refs::Weak,
    scene::{Camera, MeshData, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::{Color, Image, Point, Size},
    ui_test::check_colors,
};

/// Black and white squares, 2 a side.
const TEXELS: u32 = 64;
/// How many times the texture repeats over the side of a quad.
const REPEATS: f32 = 2.0;
const SIDE: f32 = 2.6;

fn checks() -> Weak<Image> {
    let mut data = vec![];
    for y in 0..TEXELS {
        for x in 0..TEXELS {
            let value = if (x < TEXELS / 2) == (y < TEXELS / 2) {
                255
            } else {
                20
            };
            data.extend([value, value, value, 255]);
        }
    }
    Image::from_raw_data(data, "flow_checks", Size::new(TEXELS, TEXELS), 4)
}

/// A flow map of 1 flow.
fn flow_map(name: &str, flow: Point) -> Weak<Image> {
    let data = flow_texel(flow).repeat(4);
    Image::from_raw_data(data, name, Size::new(2, 2), 4)
}

/// A quad facing the camera.
fn quad() -> MeshData {
    let half = SIDE / 2.0;
    let corner = |x: f32, y: f32, u: f32, v: f32| {
        let map = Point::new(u / REPEATS, v / REPEATS);
        Vertex3D::flowing(Vec3::new(x, y, 0.0), Vec3::Z, Point::new(u, v), map)
    };
    MeshData {
        vertices: vec![
            corner(-half, -half, 0.0, REPEATS),
            corner(half, -half, REPEATS, REPEATS),
            corner(half, half, REPEATS, 0.0),
            corner(-half, half, 0.0, 0.0),
        ],
        indices:  vec![0, 1, 2, 0, 2, 3],
    }
}

/// 2 quads of the same texture from `Model::from_flowing_mesh`, the left one
/// flowing to the right and the right one flowing down, with the clock held
/// at 2 moments. Between the 2 pictures the pattern of each quad moved its
/// own way.
#[scene]
#[derive(Default)]
struct Flow {}

impl Flow {
    fn surface(&mut self, name: &str, flow: Point, x: f32, texture: Weak<Image>) {
        let mut node = self.make_node::<Prop>(
            Shape3::Model(Model::from_flowing_mesh(name, quad())),
            Vec3::new(x, 0.0, 0.0),
        );
        node.set_color(Color::hex("#ffffff")).set_unlit(true);
        node.material.texture = Some(texture);
        node.material.flow_map = Some(flow_map(name, flow));
    }
}

impl SceneSetup for Flow {
    fn setup(&mut self) {
        self.camera = Camera::orthographic(Vec3::new(0.0, 0.0, 8.0), Vec3::ZERO, 6.0);
        self.flow_frozen = true;
        let texture = checks();
        self.surface("Flow right", Point::new(0.25, 0.0), -1.45, texture);
        self.surface("Flow down", Point::new(0.0, 0.25), 1.45, texture);
    }
}

impl SceneTest for Flow {
    fn perform_test(scene: Weak<Self>) -> Result<()> {
        let at = |seconds: f32| {
            from_main(move || {
                let mut scene = scene;
                scene.flow_time = seconds;
            });
            wait_for_next_frame();
            wait_for_next_frame();
        };
        at(0.5);
        check_colors(EARLY)?;
        at(1.5);
        check_colors(LATE)
    }
}

const EARLY: &str = r"
             200    4 - #597c95
             188  172 - #f0f0f0
              28  176 - #a1a1a1
             532  180 - #6e6e6e
             356  188 - #7b7b7b
             112  196 - #6b6b6b
             444  196 - #f0f0f0
             268  208 - #181818
             504  248 - #afafaf
             200  256 - #434343
             144  260 - #797979
              48  264 - #969696
             356  272 - #b7b7b7
             568  272 - #141414
             448  288 - #383838
             272  296 - #c9c9c9
             128  312 - #a8a8a8
             544  320 - #c2c2c2
             388  328 - #f1f1f1
             204  340 - #8b8b8b
              28  348 - #464646
             496  368 - #cccccc
             320  372 - #646464
             256  380 - #c5c5c5
             116  392 - #5c5c5c
             412  404 - #f1f1f1
             572  412 - #141414
             208  424 - #bababa
              40  428 - #2a2a2a
             316  428 - #2a2a2a
             508  428 - #f1f1f1
             328  592 - #597c95
";

const LATE: &str = r"
             312    4 - #597c95
             212  172 - #838383
             564  172 - #2b2b2b
             472  176 - #5c5c5c
             400  180 - #c2c2c2
              28  184 - #e9e9e9
             120  192 - #1f1f1f
             328  192 - #6c6c6c
             280  228 - #d0d0d0
             172  236 - #f1f1f1
             460  244 - #3f3f3f
             564  252 - #acacac
             356  272 - #5c5c5c
              32  292 - #939393
             136  296 - #141414
             228  296 - #cccccc
             124  304 - #bababa
             316  324 - #1f1f1f
             552  324 - #8b8b8b
             180  336 - #8d8d8d
             452  336 - #777777
              76  352 - #3c3c3c
             376  364 - #161616
             260  372 - #888888
             140  388 - #7d7d7d
             496  404 - #f1f1f1
             328  416 - #3c3c3c
              56  420 - #f1f1f1
             208  424 - #595959
             572  424 - #646464
             420  428 - #eeeeee
             388  592 - #597c95
";
