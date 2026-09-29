use anyhow::Result;
use hilen::{
    gm::Shape,
    level::{Banner, LevelCreation, LevelSetup, LevelTest, SpriteTemplates, level},
    refs::Weak,
    ui::{Image, ImageFilter},
    ui_test::check_colors,
};

/// A 4 by 4 image with a transparent top row, two green rows and an
/// opaque red bottom row, drawn 30 pixels a texel with the nearest filter.
/// The top edge of each copy sits just below the center of a pixel row.
/// Under MSAA that pixel is partly covered and runs its fragment at the
/// center, outside the quad. The repeating sampler wrapped the uv there
/// and drew the red row along the top edge.
#[level]
#[derive(Default)]
struct SpriteEdges {}

impl LevelSetup for SpriteEdges {
    fn setup(&mut self) {
        const CLEAR: [u8; 4] = [0, 0, 0, 0];
        const GREEN: [u8; 4] = [40, 160, 60, 255];
        const RED: [u8; 4] = [220, 30, 30, 255];
        let pixels = [[CLEAR; 4], [GREEN; 4], [GREEN; 4], [RED; 4]].concat().concat();
        let mut image = Image::from_raw_data(pixels, "sprite_edges_rows", (4, 4).into(), 4);
        image.set_filter(ImageFilter::Nearest);

        for (x, y) in [(-16.0, 0.048), (0.0, 0.04), (16.0, 0.025)] {
            self.make_sprite::<Banner>(Shape::Rect((12, 12).into()), (x, y))
                .set_image(image);
        }
    }
}

impl LevelTest for SpriteEdges {
    fn perform_test(_level: Weak<Self>) -> Result<()> {
        check_colors(EDGES)
    }
}

const EDGES: &str = r"
 140  238 - #597c95
 140  239 - #597c95
 140  240 - #597c95
 300  238 - #597c95
 300  239 - #597c95
 300  240 - #597c95
 460  238 - #597c95
 460  239 - #597c95
 460  240 - #597c95
 140  280 - #28a03c
 300  300 - #28a03c
 460  345 - #dc1e1e
";
