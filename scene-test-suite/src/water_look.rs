use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Shape3, Vec3, Vertex3D, water_texel},
    refs::Weak,
    scene::{Camera, MeshData, Model, NodeTemplates, Prop, SceneCreation, SceneSetup, SceneTest, scene},
    ui::{Color, Image, Point, Size},
    ui_test::{check_colors, set_record_probe_count},
};

const TEXELS: u32 = 32;
const SIDE: f32 = 5.0;

/// A map of slopes that changes from cell to cell, the ripples the foam opens
/// on.
fn ripples() -> Weak<Image> {
    let mut data = vec![];
    for y in 0..TEXELS {
        for x in 0..TEXELS {
            let high = (x / 4 + y / 4) % 2 == 0;
            let slope = if high { 255 } else { 128 };
            data.extend([slope, 128, 255, 255]);
        }
    }
    Image::from_raw_data(data, "water look ripples", Size::new(TEXELS, TEXELS), 4)
}

/// Depth grows from the left to the right, foam from the bottom to the top. The
/// lowest row has none.
fn look_map() -> Weak<Image> {
    let last = 7_u8;
    let mut data = vec![];
    for b in 0..=last {
        for a in 0..=last {
            let depth = f32::from(a) / f32::from(last);
            let foam = f32::from(last - b) / f32::from(last);
            data.extend(water_texel(Point::new(0.0, 0.0), foam, depth));
        }
    }
    let side = u32::from(last) + 1;
    Image::from_raw_data(data, "water look map", Size::new(side, side), 4)
}

fn quad() -> MeshData {
    let half = SIDE / 2.0;
    // Inside the middles of the outer texels, the map repeats past them.
    let place = |at: f32| (at * 7.0 + 0.5) / 8.0;
    let corner = |x: f32, y: f32, u: f32, v: f32| {
        let map = Point::new(place(u), place(v));
        Vertex3D::flowing(Vec3::new(x, y, 0.0), Vec3::Z, Point::new(u, v), map)
    };
    MeshData {
        vertices: vec![
            corner(-half, -half, 0.0, 1.0),
            corner(half, -half, 1.0, 1.0),
            corner(half, half, 1.0, 0.0),
            corner(-half, half, 0.0, 0.0),
        ],
        indices:  vec![0, 1, 2, 0, 2, 3],
    }
}

/// A flowing mesh with its look from `water_texel`: pale shallow water on the
/// left, dark deep water on the right, plain along the bottom and more and more
/// foam toward the top, broken up by its ripples.
#[scene]
#[derive(Default)]
struct WaterLook {}

impl SceneSetup for WaterLook {
    fn setup(&mut self) {
        self.camera = Camera::orthographic(Vec3::new(0.0, 0.0, 8.0), Vec3::ZERO, 6.0);
        self.flow_frozen = true;
        let mut node = self.make_node::<Prop>(
            Shape3::Model(Model::from_flowing_mesh("Water look", quad())),
            Vec3::ZERO,
        );
        node.set_color(Color::hex("#103060")).set_unlit(true);
        node.material.shallow_color = Some(Color::hex("#40c0b0"));
        node.material.normal_map = Some(ripples());
        node.material.flow_map = Some(look_map());
    }
}

impl SceneTest for WaterLook {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(32);
        wait_for_next_frame();
        check_colors(LOOK)
    }
}

const LOOK: &str = r"
             452   52 - #206475
             372   56 - #b7c5c7
              52   64 - #6dc7ba
             300   64 - #2f908f
             292   72 - #2f9290
             200   88 - #cededc
             452  120 - #3f6f7e
             544  120 - #113561
             460  124 - #8a9ba4
             456  128 - #577a87
             520  128 - #89929e
             544  128 - #113561
             116  132 - #8dc8c0
             348  132 - #b2c4c6
             272  136 - #9fbfbe
             332  140 - #e4e8e8
             476  144 - #ebecec
              60  148 - #74c7bb
             212  152 - #75b3ae
             420  160 - #90a5ab
             472  164 - #607b8a
             152  196 - #ccdfdc
             412  196 - #9db0b4
             268  224 - #acc7c6
             520  228 - #475d76
             172  360 - #38aba1
               8  368 - #597c95
             408  384 - #25737d
             548  404 - #103160
             260  496 - #329995
             532  548 - #143d64
               4  592 - #597c95
";
