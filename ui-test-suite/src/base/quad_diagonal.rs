use std::fmt::Write;

use anyhow::Result;
use hilen::{
    refs::Weak,
    ui::{
        BLUE, BlurView, Color, Container, GREEN, Image, ImageView, RED, Setup, ViewData, ViewFrame, ViewTest,
        WHITE, YELLOW, view,
    },
    ui_test::check_colors,
};

const SIDE: u32 = 100;

/// One square per pipeline that anti aliases a rounded box edge: the
/// rect, a rect with a rounded border, a gradient, an image, a clipping
/// container and a backdrop blur. The probes sit on both diagonals of
/// every square, the pixels where the two triangles of the quad meet.
/// Under `SwiftShader`, Chrome's software WebGPU, `fwidth` returned zero in
/// the pixel quads on that seam and the shaders blanked them, so the CI web
/// lane read the clear color through every view.
#[view]
struct QuadDiagonal {
    #[init]
    rect:     Container,
    bordered: Container,
    gradient: Container,
    image:    ImageView,
    clip:     Clip,
    blur:     BlurView,
}

/// Clips its child to its bounds, so the clip pipeline writes the mask
/// the child is drawn through.
#[view]
struct Clip {
    #[init]
    fill: Container,
}

impl Setup for Clip {
    fn setup(self: Weak<Self>) {
        self.fill.set_color(BLUE).place().back();
    }

    fn clips_to_bounds(&self) -> bool {
        true
    }
}

impl Setup for QuadDiagonal {
    fn setup(mut self: Weak<Self>) {
        self.rect.set_color(RED).set_frame((40, 40, SIDE, SIDE));

        self.bordered
            .set_color(GREEN)
            .set_border_color(WHITE)
            .set_border_width(4)
            .set_corner_radius(20)
            .set_frame((200, 40, SIDE, SIDE));

        self.gradient.set_gradient(YELLOW, YELLOW).set_frame((360, 40, SIDE, SIDE));

        let side = 4;
        let pixels = vec![[255, 255, 0, 255]; side * side].concat();
        self.image
            .set_image(Image::from_raw_data(
                pixels,
                "quad_diagonal_solid",
                (4, 4).into(),
                4,
            ))
            .set_frame((40, 200, SIDE, SIDE));

        self.clip.set_frame((200, 200, SIDE, SIDE));

        self.blur.set_blur_radius(10).set_color(WHITE).set_frame((360, 200, SIDE, SIDE));
    }
}

/// Probes along both diagonals of the square at `x y`, from `inset`
/// pixels in, every 10 pixels, all expecting `color`.
fn diagonal_probes(probes: &mut String, x: u32, y: u32, inset: u32, color: Color) -> Result<()> {
    let hex = color.as_hex();
    for k in (inset..SIDE - inset).step_by(10) {
        writeln!(probes, "{:>4} {:>4} - {hex}", x + k, y + k)?;
        writeln!(probes, "{:>4} {:>4} - {hex}", x + SIDE - 1 - k, y + k)?;
    }
    Ok(())
}

impl ViewTest for QuadDiagonal {
    fn perform_test(_view: Weak<Self>) -> Result<()> {
        let mut probes = String::new();

        diagonal_probes(&mut probes, 40, 40, 2, RED)?;
        // The rounded corners and the border band cover the first pixels
        // of a diagonal, the fill starts past them.
        diagonal_probes(&mut probes, 200, 40, 12, GREEN)?;
        diagonal_probes(&mut probes, 360, 40, 2, YELLOW)?;
        diagonal_probes(&mut probes, 40, 200, 2, YELLOW)?;
        diagonal_probes(&mut probes, 200, 200, 2, BLUE)?;
        diagonal_probes(&mut probes, 360, 200, 2, WHITE)?;

        check_colors(&probes)
    }
}
