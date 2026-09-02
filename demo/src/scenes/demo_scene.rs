use hilen::{
    gm::{
        LossyConvert,
        volume::{Quat, Shape3, Vec3},
    },
    refs::manage::DataManager,
    scene::{Body, Light, Material, NodeTemplates, SceneCreation, SceneSetup, Sky, Wall, scene},
    ui::{Color, Image},
};

const PYRAMID_ROWS: usize = 4;
/// Blocks of the brick wall, each shows one tile of the texture.
const WALL_BLOCKS: usize = 4;

/// The 3D playground with everything the scene module draws: a floor
/// under a gradient sky it reflects, a pyramid of crates to knock over,
/// a brick wall with a normal map behind it, a row of balls from chrome
/// to matte, glass panes, a glass cube and a glass ball, and the balls
/// the page drops onto it, lit by the sun, a warm lamp, a cold spot and
/// a lamp grazing the wall so its bricks stand out. Walked through by
/// a first person player.
#[scene]
#[derive(Default)]
pub struct DemoScene {}

impl DemoScene {
    /// A ball with a random color and a random finish, from a matte
    /// dielectric to a polished metal, every fourth one made of glass.
    pub fn drop_ball(&mut self) {
        let x = fastrand::f32() * 4.0 - 2.0;
        let z = fastrand::f32() * 4.0 - 2.0;
        let alpha = if fastrand::u8(..4) == 0 { 0.5 } else { 1.0 };
        self.make_node::<Body>(Shape3::Ball(0.5), Vec3::new(x, 9.0, z))
            .set_color(Color::random().with_alpha(alpha))
            .set_metallic(f32::from(fastrand::bool()))
            .set_roughness(fastrand::f32());
    }
}

impl SceneSetup for DemoScene {
    fn needs_physics(&self) -> bool {
        true
    }

    fn setup(&mut self) {
        self.sky = Some(Sky::gradient(
            Color::hex("#3a7bd5"),
            Color::hex("#d9e4f0"),
            Color::hex("#5a4a3a"),
        ));

        self.lights.push(
            Light::point(Vec3::new(4.0, 3.0, 4.0))
                .color(Color::hex("#ffb060"))
                .intensity(6.0)
                .range(14.0),
        );
        self.lights.push(
            Light::spot(Vec3::new(-4.0, 7.0, 2.0), Vec3::new(0.5, -1.0, -0.3), 0.4)
                .color(Color::hex("#60d0ff"))
                .intensity(40.0)
                .range(16.0),
        );

        self.lights.push(
            Light::point(Vec3::new(-4.5, 2.2, -4.4))
                .color(Color::hex("#fff0d0"))
                .intensity(3.0)
                .range(9.0),
        );

        self.make_node::<Wall>(Shape3::Plane(30.0), Vec3::ZERO)
            .set_color(Color::hex("#95a5a6"))
            .set_roughness(0.8);

        self.add_player(Vec3::new(0.0, 1.0, 9.0));

        let crate_texture = Image::get("crate_box.png");

        for row in 0..PYRAMID_ROWS {
            let count = PYRAMID_ROWS - row;
            for i in 0..count {
                let x = (i.lossy_convert() - (count - 1).lossy_convert() / 2.0) * 1.05;
                let y = 0.5 + row.lossy_convert() * 1.0;
                let mut cube = self.make_node::<Body>(Shape3::cube(1.0), Vec3::new(x, y, 0.0));
                cube.set_color(Color::hex("#ffffff")).set_roughness(0.7);
                cube.material.texture = Some(crate_texture);
            }
        }

        // A face of a box shows the texture once, so a wall of two
        // meter blocks gets bricks at a real size.
        let bricks = Material {
            color:        Color::hex("#ffffff"),
            metallic:     0.0,
            roughness:    0.8,
            texture:      Some(Image::get("bricks.jpg")),
            normal_map:   Some(Image::get("bricks_normal.jpg")),
            normal_scale: 1.5,
        };
        for i in 0..WALL_BLOCKS {
            let x = (i.lossy_convert() - (WALL_BLOCKS - 1).lossy_convert() / 2.0) * 2.0;
            self.make_node::<Wall>(Shape3::cuboid(2.0, 2.0, 0.5), Vec3::new(x, 1.0, -5.0))
                .set_material(bricks);
        }

        for (i, (metallic, roughness)) in
            [(1.0, 0.0), (1.0, 0.4), (0.0, 0.2), (0.0, 0.9)].into_iter().enumerate()
        {
            let x = 5.0 + i.lossy_convert() * 1.6;
            self.make_node::<Wall>(Shape3::Ball(0.7), Vec3::new(x, 0.7, -2.0))
                .set_material(Material {
                    color: Color::hex("#e8ecef"),
                    metallic,
                    roughness,
                    ..Material::default()
                });
        }

        self.make_node::<Wall>(Shape3::cuboid(4.0, 2.5, 0.1), Vec3::new(-5.0, 1.25, 1.0))
            .set_rotation(Quat::from_rotation_y(0.6))
            .set_color(Color::hex("#a0d8ff").with_alpha(0.35))
            .set_roughness(0.05);
        self.make_node::<Wall>(Shape3::cuboid(3.0, 2.0, 0.1), Vec3::new(6.0, 1.0, 3.0))
            .set_rotation(Quat::from_rotation_y(-0.5))
            .set_color(Color::hex("#ffd080").with_alpha(0.35))
            .set_roughness(0.05);
        self.make_node::<Wall>(Shape3::cube(1.2), Vec3::new(3.5, 0.6, 3.5))
            .set_rotation(Quat::from_rotation_y(0.3))
            .set_color(Color::hex("#a0ffc0").with_alpha(0.4))
            .set_roughness(0.1);
        self.make_node::<Wall>(Shape3::Ball(0.8), Vec3::new(-3.0, 0.8, 4.0))
            .set_color(Color::hex("#ff6060").with_alpha(0.5))
            .set_roughness(0.2);
    }
}
