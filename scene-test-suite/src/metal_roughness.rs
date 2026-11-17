use anyhow::Result;
use hilen::{
    dispatch::wait_for_next_frame,
    gm::volume::{Shape3, Vec3},
    refs::{Weak, manage::DataManager},
    scene::{Camera, Light, Model, Prop, SceneCreation, SceneSetup, SceneTest, Sky, scene},
    ui::Color,
    ui_test::{check_colors, set_record_probe_count},
};

/// `metal_rough.glb`, an upright quad whose material has a metallic and a
/// roughness factor of 1 and the real values in 1 texture: blue is the
/// metallic, green the roughness and red the occlusion, as glTF packs
/// them. The left half is a smooth metal that mirrors the sky and the
/// lamp in front of it, the right half a rough orange dielectric, and the
/// lower half is occluded, so the sky lights it less. Without the texture
/// the whole quad is 1 rough metal, without the occlusion the lower half
/// looks like the upper one.
#[scene]
#[derive(Default)]
struct MetalRoughness {}

impl SceneSetup for MetalRoughness {
    fn setup(&mut self) {
        self.camera = Camera {
            position: Vec3::new(0.0, 0.0, 5.0),
            target: Vec3::ZERO,
            ..Camera::default()
        };
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));
        self.lights
            .push(Light::point(Vec3::new(-1.0, 0.5, 1.5)).intensity(3.0).range(8.0));
        self.make_node::<Prop>(Shape3::Model(Model::get("metal_rough.glb")), Vec3::ZERO);
    }
}

impl SceneTest for MetalRoughness {
    fn perform_test(_: Weak<Self>) -> Result<()> {
        set_record_probe_count(48);
        wait_for_next_frame();
        check_colors(QUAD)
    }
}

// The lamp's highlight sits on the smooth metal around 220 260, the
// rough half around 432 300 has none, and the occluded lower half is
// darker on both sides, 160 320 and 496 376.
const QUAD: &str = r"
       4    4 - #aec1e5
     372    4 - #a5bbe3
     188   12 - #a8bde4
     592  180 - #cdd9eb
     476  192 - #cedaea
     292  220 - #d1904f
     412  232 - #f0a661
     220  240 - #fcc095
     204  244 - #fbb885
     216  248 - #fee5d6
     232  248 - #feceb0
     224  252 - #fff4ee
     208  256 - #ffe7da
     228  260 - #fff8f4
     240  260 - #fdc49c
       4  264 - #d4dfec
     220  264 - #fffdfd
     128  268 - #c58849
     208  268 - #fedfcc
     236  268 - #fecfb1
     232  272 - #fed4ba
     220  276 - #fed8c1
     592  276 - #d5e0ec
     228  280 - #fcbf92
     432  300 - #df9754
     504  304 - #c28246
     332  308 - #ed9f57
     160  320 - #6d4924
     256  340 - #6c4824
     192  372 - #694623
     496  376 - #c08045
     592  400 - #d2dce7
       4  568 - #b8bec7
     292  584 - #adb2b8
     192  592 - #acb1b8
     244  592 - #abb0b7
     388  592 - #acb1b7
     592  592 - #b3b9c0
";
