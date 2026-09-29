use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{Shape, color::Color},
    level::{
        Banner, LevelCreation, LevelSetup, LevelTest, Light, MaterialId, SpriteTemplates, Terrain,
        TerrainMaterial, level,
    },
    refs::Weak,
    ui::{ImageFilter, Point},
    ui_test::{check_colors, checkpoint, set_record_probe_count},
    window::image::{Image, ToImage},
};

/// A hill of dirt with grass along its walkable slopes and tops, a layer
/// of stone under it that shows on the right, a 1 pixel outline along
/// every edge, a raised ledge on the left and a cliff on the right where
/// grass runs end, and a cutout in front. The slopes cross the chunk
/// border at y 0, where the grass must not break. Then a crater is dug
/// into the hilltop and grows grass, being open to the sky, a covered
/// tunnel under the right slope stays bare, and a hole is cut into the
/// stone. Last the lights go down to a torch in the crater.
#[level]
#[derive(Default)]
struct TerrainGround {
    terrain: Weak<Terrain>,
    dirt:    Option<MaterialId>,
    stone:   Option<MaterialId>,
}

fn pixel_art(name: &str) -> Weak<Image> {
    let mut image = name.to_image();
    image.set_filter(ImageFilter::Nearest);
    image
}

/// The ground fill images are 96 pixels square. 6 of their pixels to a level
/// unit draws them about as big as the pixels of the cutout.
const PIXELS_PER_UNIT: f32 = 6.0;
const OUTLINE: &str = "#434a5f";

fn points(list: &[(f32, f32)]) -> Vec<Point> {
    list.iter().map(|&(x, y)| Point::new(x, y)).collect()
}

impl LevelSetup for TerrainGround {
    fn setup(&mut self) {
        let mut terrain = Terrain::new();
        let tile = 96.0 / PIXELS_PER_UNIT;
        let line = 1.0 / PIXELS_PER_UNIT;
        let dirt = terrain.add_material(
            TerrainMaterial::new(pixel_art("game/ground_dirt.png"), tile)
                .with_surface(pixel_art("game/ground_grass.png"), 9.0 / PIXELS_PER_UNIT)
                .with_surface_end(pixel_art("game/ground_grass_end.png"), 4.0 / PIXELS_PER_UNIT)
                .with_edge(Color::hex(OUTLINE), line)
                .with_blend(1.5),
        );
        let stone = terrain.add_material(
            TerrainMaterial::new(pixel_art("game/ground_stone.png"), tile)
                .with_edge(Color::hex(OUTLINE), line),
        );

        terrain.add_polygon(
            dirt,
            &points(&[
                (-32.0, -32.0),
                (32.0, -32.0),
                (32.0, -16.0),
                (26.0, -16.0),
                (26.0, -5.5),
                (20.0, -5.5),
                (5.0, 5.0),
                (-5.0, 5.0),
                (-18.0, -4.0),
                (-32.0, -4.0),
            ]),
            &[],
        );
        // A raised ledge on the left plateau, grass meets its wall from
        // below and ends at its top edge.
        terrain.add_polygon(
            dirt,
            &points(&[(-32.0, -4.0), (-26.0, -4.0), (-26.0, -1.0), (-32.0, -1.0)]),
            &[],
        );
        terrain.add_polygon(
            stone,
            &points(&[
                (-32.0, -32.0),
                (32.0, -32.0),
                (32.0, -16.0),
                (26.0, -16.0),
                (26.0, -10.0),
                (22.0, -12.0),
                (14.0, -14.0),
                (6.0, -13.0),
                (-4.0, -15.0),
                (-14.0, -13.0),
                (-24.0, -16.0),
                (-32.0, -14.0),
            ]),
            &[],
        );

        self.dirt = Some(dirt);
        self.stone = Some(stone);
        self.terrain = self.add_terrain(terrain);

        self.make_sprite::<Banner>(Shape::Rect((6, 10).into()), (23, -0.5))
            .set_image("game/frisk.png");
    }
}

impl LevelTest for TerrainGround {
    fn perform_test(level: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        checkpoint(
            "dirt hill, 1 pixel outline, grass unbroken across the chunk border at y 0, grass ends at the ledge and the cliff",
        )?;
        check_colors("")?;

        from_main(move || {
            let mut terrain = level.terrain;
            let dirt = level.dirt.expect("set in setup");
            let stone = level.stone.expect("set in setup");
            assert!(terrain.carve_circle((0, 5), 6.0, dirt));
            // A tunnel under the right slope, covered, so its floor stays bare.
            assert!(terrain.carve_circle((10, -3), 2.0, dirt));
            assert!(terrain.carve_circle((-18, -20), 4.0, stone));
            assert!(
                !terrain.carve_circle((-18, -20), 4.0, dirt),
                "the stone hole holds no dirt"
            );
        });
        wait_for_next_frame();

        checkpoint(
            "crater in the hilltop open to the sky with grass on its floor, bare tunnel under the right slope, hole in the stone",
        )?;
        check_colors("")?;

        from_main(move || {
            let mut level = level;
            level.ambient_light = Color::hex("#202030");
            level.add_light(
                Light::new((0, 0), 18)
                    .with_color(Color::hex("#ffb050"))
                    .with_intensity(1.3)
                    .with_falloff(1.2),
            );
        });
        wait_for_next_frame();

        checkpoint("dark level, a warm torch in the crater lights the ground around it")?;
        check_colors("")
    }
}
