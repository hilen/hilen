//! Per pixel loops the engine runs on the CPU. Their own crate so the dev
//! profile can optimize them the way it does the decoders. Inside the
//! engine they compile at `opt-level = 0`, and one 41 megapixel fixture
//! then spent 11 seconds in its mip chain at every browser start.

mod cube;
mod overlay;
mod spread;

pub use cube::*;
pub use overlay::*;
pub use spread::*;
use tiny_skia::PremultipliedColorU8;

/// Straight alpha RGBA bytes of a tiny-skia raster. Its pixels are
/// premultiplied and the engine blends straight alpha, so uploaded as
/// they are every anti aliased edge would darken a second time.
pub fn demultiply_rgba(pixels: &[PremultipliedColorU8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() * 4);
    for pixel in pixels {
        let pixel = pixel.demultiply();
        out.extend_from_slice(&[pixel.red(), pixel.green(), pixel.blue(), pixel.alpha()]);
    }
    out
}

/// Level 0 is the image itself, every next level halves it with a 2 by 2
/// box filter, an odd edge keeps its last row or column, down to 1 by 1.
/// Alpha weighted, so a transparent neighbor does not bleed its color
/// into a covered texel. Every level comes with its width and height.
/// The pixels come by value, level 0 is that same buffer and never a copy.
pub fn mip_chain(data: Vec<u8>, width: u32, height: u32, channels: u8) -> Vec<(Vec<u8>, (u32, u32))> {
    let channels = usize::from(channels);
    let mut levels = vec![(data, (width, height))];
    while let Some((pixels, (width, height))) = levels.last()
        && (*width > 1 || *height > 1)
    {
        let (width, height) = (*width, *height);
        let next = ((width / 2).max(1), (height / 2).max(1));
        let mut out = Vec::with_capacity((next.0 * next.1) as usize * channels);
        for y in 0..next.1 {
            for x in 0..next.0 {
                let x0 = (x * 2).min(width - 1);
                let x1 = (x * 2 + 1).min(width - 1);
                let y0 = (y * 2).min(height - 1);
                let y1 = (y * 2 + 1).min(height - 1);
                let texel = |px: u32, py: u32| {
                    let at = (py * width + px) as usize * channels;
                    &pixels[at..at + channels]
                };
                let four = [texel(x0, y0), texel(x1, y0), texel(x0, y1), texel(x1, y1)];
                if channels == 4 {
                    let alpha: u32 = four.iter().map(|t| u32::from(t[3])).sum();
                    for channel in 0..3 {
                        let weighted: u32 =
                            four.iter().map(|t| u32::from(t[channel]) * u32::from(t[3])).sum();
                        let value = (weighted + alpha / 2).checked_div(alpha).unwrap_or(0);
                        out.push(u8::try_from(value).expect("weighted mean fits u8"));
                    }
                    out.push(u8::try_from((alpha + 2) / 4).expect("mean alpha fits u8"));
                } else {
                    for channel in 0..channels {
                        let sum: u32 = four.iter().map(|t| u32::from(t[channel])).sum();
                        out.push(u8::try_from((sum + 2) / 4).expect("mean fits u8"));
                    }
                }
            }
        }
        levels.push((out, next));
    }
    levels
}

/// Scales the alpha of every level after the first so the share of texels
/// at or over `threshold` stays what it is in level 0. A box filter spreads
/// alpha thin, and a texture cut out at `threshold` then loses its far
/// leaves. The scale of each level is found by halving, in steps of 1/256
/// up to 16 times, 4 channels only.
pub fn keep_coverage<S>(levels: &mut [(Vec<u8>, S)], threshold: u8) {
    const ONE: u32 = 256;
    let covered = |pixels: &[u8], scale: u32| -> u64 {
        let over = pixels
            .iter()
            .skip(3)
            .step_by(4)
            .filter(|&&alpha| u32::from(alpha) * scale >= u32::from(threshold) * ONE)
            .count();
        u64::try_from(over).expect("a texel count fits u64")
    };
    let texels = |pixels: &[u8]| u64::try_from(pixels.len() / 4).expect("a texel count fits u64");
    let Some(((first, _), rest)) = levels.split_first_mut() else {
        return;
    };
    let (first_covered, first_texels) = (covered(first, ONE), texels(first));
    for (pixels, _) in rest {
        // The shares compared by cross multiplying the counts.
        let wanted = first_covered * texels(pixels);
        let reach = |scale: u32| covered(pixels, scale) * first_texels;
        let (mut low, mut high) = (0, ONE * 16);
        while high - low > 1 {
            let middle = u32::midpoint(low, high);
            if reach(middle) < wanted {
                low = middle;
            } else {
                high = middle;
            }
        }
        // A small level covers in steps, take the side of the step nearer
        // to the share of level 0.
        let scale = if reach(low).abs_diff(wanted) < reach(high).abs_diff(wanted) {
            low
        } else {
            high
        };
        for alpha in pixels.iter_mut().skip(3).step_by(4) {
            // Floored, so a texel is covered exactly when the search
            // counted it.
            let scaled = u32::from(*alpha) * scale / ONE;
            *alpha = u8::try_from(scaled.min(255)).expect("a clamped alpha fits u8");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{keep_coverage, mip_chain};

    /// Leaves of 64 by 64 texels: an opaque disc of radius 2 in each cell
    /// of 8 by 8, moved inside its cell by a hash, clear elsewhere.
    fn leaves() -> Vec<u8> {
        let center = |cell: i32| {
            let hash = cell.unsigned_abs().wrapping_mul(2_654_435_761) >> 29;
            i32::try_from(hash).expect("3 bits fit") - 4
        };
        (0..64 * 64)
            .flat_map(|i: i32| {
                let (x, y) = (i % 64, i / 64);
                let (cx, cy) = (x / 8, y / 8);
                let dx = x - (cx * 8 + 4 + center(cx * 31 + cy * 7) / 2);
                let dy = y - (cy * 8 + 4 + center(cx * 13 + cy * 17) / 2);
                let alpha = if dx * dx + dy * dy <= 4 { 255 } else { 0 };
                [40, 140, 50, alpha]
            })
            .collect()
    }

    /// The share of texels a cutout at half keeps, in thousandths.
    fn covered(pixels: &[u8]) -> usize {
        let over = pixels.iter().skip(3).step_by(4).filter(|&&alpha| alpha >= 128).count();
        over * 1000 / (pixels.len() / 4)
    }

    #[test]
    fn cutout_levels_keep_the_covered_share() {
        let plain = mip_chain(leaves(), 64, 64, 4);
        let target = covered(&plain[0].0);
        assert!(target.abs_diff(200) < 30, "the pattern covers {target}");
        // A box filter alone thins the leaves to almost nothing.
        assert!(covered(&plain[3].0) < target / 2);

        let mut kept = mip_chain(leaves(), 64, 64, 4);
        keep_coverage(&mut kept, 128);
        for ((pixels, (width, _)), (plain, _)) in kept[1..].iter().zip(&plain[1..]) {
            let (share, before) = (covered(pixels), covered(plain));
            assert!(
                share.abs_diff(target) <= before.abs_diff(target),
                "level of width {width} covers {share}, the box filter {before}, level 0 {target}"
            );
            // Below 16 a texel holds a whole cell of the pattern, every one
            // alike, so the share is all or nothing.
            if *width >= 16 {
                assert!(
                    share.abs_diff(target) < 60,
                    "level of width {width} covers {share}, not {target}"
                );
            }
        }
    }

    #[test]
    fn mip_chain_halves_down_to_one_texel() {
        let data = vec![255; 4 * 4 * 4];
        let levels = mip_chain(data, 4, 4, 4);
        let sizes: Vec<(u32, u32)> = levels.iter().map(|(_, size)| *size).collect();
        assert_eq!(sizes, [(4, 4), (2, 2), (1, 1)]);
        assert_eq!(levels[2].0, [255, 255, 255, 255]);
    }

    #[test]
    fn mip_chain_keeps_odd_edges_and_weights_by_alpha() {
        // A 3 by 1 row: opaque red, transparent green, opaque red.
        let data = vec![255, 0, 0, 255, 0, 255, 0, 0, 255, 0, 0, 255];
        let levels = mip_chain(data, 3, 1, 4);
        assert_eq!(levels[1].1, (1, 1));
        // The transparent green texel contributes no color, only alpha.
        assert_eq!(levels[1].0, [255, 0, 0, 128]);
    }

    #[test]
    fn mip_chain_averages_single_channel() {
        let data = vec![0, 100, 200, 100];
        let levels = mip_chain(data, 2, 2, 1);
        assert_eq!(levels[1].0, [100]);
    }

    #[test]
    fn mip_chain_level_0_is_the_given_buffer() {
        let data = vec![7; 8 * 8 * 4];
        let given = data.as_ptr();
        let levels = mip_chain(data, 8, 8, 4);
        assert_eq!(levels[0].0.as_ptr(), given, "level 0 was copied");
    }
}
