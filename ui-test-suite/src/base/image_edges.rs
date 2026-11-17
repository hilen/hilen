use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{Image, ImageFilter, ImageView, Setup, ViewFrame, ViewTest, view},
    ui_test::check_colors,
};

/// A 4 by 4 image with a transparent top row, two green rows and an
/// opaque red bottom row, drawn 30 pixels a texel with the nearest filter.
/// The top edge of each copy sits just below the center of a pixel row.
/// Under MSAA that pixel is partly covered and runs its fragment at the
/// center, outside the quad. The repeating sampler wrapped the uv there
/// and drew the red row along the top edge, faded by the edge coverage.
#[view]
struct ImageEdges {
    #[init]
    near: ImageView,
    mid:  ImageView,
    far:  ImageView,
}

impl Setup for ImageEdges {
    fn setup(self: Weak<Self>) {
        const CLEAR: [u8; 4] = [0, 0, 0, 0];
        const GREEN: [u8; 4] = [40, 160, 60, 255];
        const RED: [u8; 4] = [220, 30, 30, 255];
        let pixels = [[CLEAR; 4], [GREEN; 4], [GREEN; 4], [RED; 4]].concat().concat();
        let mut image = Image::from_raw_data(pixels, "image_edges_rows", (4, 4).into(), 4);
        image.set_filter(ImageFilter::Nearest);

        self.near.set_image(image).set_frame((40.0, 39.52, 120.0, 120.0));
        self.mid.set_image(image).set_frame((200.0, 39.6, 120.0, 120.0));
        self.far.set_image(image).set_frame((360.0, 39.75, 120.0, 120.0));
    }
}

impl ViewTest for ImageEdges {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        check_colors(EDGES)
    }
}

const EDGES: &str = r"
 100   39 - #597c95
 100   40 - #597c95
 100   41 - #597c95
 260   39 - #597c95
 260   40 - #597c95
 260   41 - #597c95
 420   39 - #597c95
 420   40 - #597c95
 420   41 - #597c95
 100   80 - #28a03c
 260  120 - #28a03c
 420  145 - #dc1e1e
";
