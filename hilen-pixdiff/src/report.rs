use image::{Rgb, RgbImage};

use crate::diff::Region;

/// Print the ranked regions, pixel coordinates plus points at `scale`
/// pixels per point, with the mean hex of each side.
pub fn print(regions: &[Region], scale: u32) {
    if regions.is_empty() {
        println!("no differences");
        return;
    }
    println!("{} difference regions, largest first:", regions.len());
    for (index, region) in regions.iter().enumerate() {
        let bounds = region.bounds;
        let scale = scale.max(1);
        println!(
            "{:>3}. {}x{} at ({}, {}) px, {}x{} at ({}, {}) pt, {} cells, a {} b {}",
            index + 1,
            bounds.w,
            bounds.h,
            bounds.x,
            bounds.y,
            bounds.w / scale,
            bounds.h / scale,
            bounds.x / scale,
            bounds.y / scale,
            region.cells,
            hex(region.mean_a),
            hex(region.mean_b),
        );
    }
}

fn hex(color: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", color[0], color[1], color[2])
}

/// The second image with every difference region outlined in red, the
/// quickest way to see where the two captures disagree.
pub fn heatmap(base: &RgbImage, regions: &[Region]) -> RgbImage {
    const RED: Rgb<u8> = Rgb([220, 38, 38]);
    let mut out = base.clone();
    let (width, height) = out.dimensions();
    for region in regions {
        let bounds = region.bounds;
        let right = (bounds.x + bounds.w).min(width) - 1;
        let bottom = (bounds.y + bounds.h).min(height) - 1;
        for x in bounds.x..=right {
            for y in [bounds.y, bottom] {
                out.put_pixel(x, y, RED);
            }
        }
        for y in bounds.y..=bottom {
            for x in [bounds.x, right] {
                out.put_pixel(x, y, RED);
            }
        }
    }
    out
}
