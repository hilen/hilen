use std::f32::consts::TAU;

use anyhow::Result;
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    gm::{Shape, color::Color},
    level::{
        Banner, LevelCreation, LevelManager, LevelSetup, LevelTest, MaterialId, SpriteTemplates, Terrain,
        TerrainMaterial, level,
    },
    refs::Weak,
    ui::{ImageFilter, Point},
    ui_test::{check_colors, checkpoint, set_record_probe_count},
    window::image::{Image, ToImage},
};

/// Ground drawn like a pixel art platformer draws it: a 16 pixel hero 2.6
/// units tall, every ground pixel as big as a hero pixel, 32 points to a
/// unit. A smooth slope, sampled every half unit, crosses the chunk border
/// at y 0 in the middle of the screen. Grass hangs from the slope into the
/// chunk below and must not lose its lower part there, the notch this test
/// was written for. Then a pit is dug from the top, open to the sky, and
/// grows grass with an end piece on each side, and a covered tunnel stays
/// bare.
#[level]
#[derive(Default)]
struct GrassAcrossChunkBorder {
    terrain: Weak<Terrain>,
    dirt:    Option<MaterialId>,
}

/// One pixel of the art in level units.
const PIXEL: f32 = 2.6 / 16.0;
const POINTS_PER_UNIT: f32 = 32.0;
const OUTLINE: &str = "#434a5f";

fn pixel_art(name: &str) -> Weak<Image> {
    let mut image = name.to_image();
    image.set_filter(ImageFilter::Nearest);
    image
}

/// The surface height along x, a slope that crosses y 0 at x 0 and
/// rises less than 50 degrees everywhere.
fn surface(x: f32) -> f32 {
    -1.8 * (TAU * x / 30.0).sin() + 0.3 * (TAU * x / 7.0).sin()
}

impl LevelSetup for GrassAcrossChunkBorder {
    fn setup(&mut self) {
        let mut terrain = Terrain::new();
        let line = Color::hex(OUTLINE);
        let dirt = terrain.add_material(
            TerrainMaterial::new(pixel_art("game/ground_dirt.png"), 96.0 * PIXEL)
                .with_surface(pixel_art("game/ground_grass.png"), 9.0 * PIXEL)
                .with_surface_end(pixel_art("game/ground_grass_end.png"), 4.0 * PIXEL)
                .with_edge(line, PIXEL)
                .with_blend(1.5),
        );
        let stone = terrain.add_material(
            TerrainMaterial::new(pixel_art("game/ground_stone.png"), 96.0 * PIXEL).with_edge(line, PIXEL),
        );

        let mut outline = vec![Point::new(-20.0, -20.0), Point::new(20.0, -20.0)];
        outline.extend((0u8..=80).rev().map(|i| {
            let x = -20.0 + f32::from(i) * 0.5;
            Point::new(x, surface(x))
        }));
        terrain.add_polygon(dirt, &outline, &[]);
        let mut rock = vec![Point::new(-20.0, -20.0), Point::new(20.0, -20.0)];
        rock.extend((0u8..=80).rev().map(|i| {
            let x = -20.0 + f32::from(i) * 0.5;
            Point::new(x, surface(x) - 6.0)
        }));
        terrain.add_polygon(stone, &rock, &[]);

        self.dirt = Some(dirt);
        self.terrain = self.add_terrain(terrain);

        let hero = Point::new(-6.0, surface(-6.0) + 1.3);
        self.make_sprite::<Banner>(Shape::Rect((16.0 * PIXEL, 16.0 * PIXEL).into()), hero)
            .set_image(pixel_art("game/hero.png"));
    }
}

impl LevelTest for GrassAcrossChunkBorder {
    fn perform_test(level: Weak<Self>) -> Result<()> {
        set_record_probe_count(160);
        // The runner sets its own zoom after setup, so the test sets it here.
        from_main(|| {
            LevelManager::set_points_per_unit(POINTS_PER_UNIT);
            *LevelManager::camera_pos() = Point::new(0.0, 0.0);
        });
        wait_for_next_frame();

        checkpoint(
            "the slope crosses the chunk border at y 0 mid screen, its grass keeps its full height there",
        )?;
        check_colors("")?;

        from_main(move || {
            let mut terrain = level.terrain;
            let dirt = level.dirt.expect("set in setup");
            // A pit from the top, open to the sky.
            assert!(terrain.carve_circle((4.0, surface(4.0)), 1.6, dirt));
            // A tunnel under the ground, covered.
            assert!(terrain.carve_circle((-3.0, surface(-3.0) - 3.5), 1.4, dirt));
        });
        wait_for_next_frame();

        checkpoint(
            "the pit grows grass with an end piece on each side, the tunnel under the slope stays bare",
        )?;
        check_colors("")?;

        from_main(|| {
            LevelManager::set_pixel_art(Some(PIXEL));
            *LevelManager::camera_pos() = Point::new(0.0, 0.0);
        });
        wait_for_next_frame();

        checkpoint(
            "pixel art: the slope, the pit and the tunnel break into hard pixel steps, no pixel is half grass half sky",
        )?;
        check_colors("")
    }
}
