//! Per pixel loops the engine runs on the CPU. Their own crate so the dev
//! profile can optimize them the way it does the decoders. Inside the
//! engine they compile at `opt-level = 0`, and one 41 megapixel fixture
//! then spent 11 seconds in its mip chain at every browser start.

mod cube;

pub use cube::*;
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
pub fn mip_chain(data: &[u8], width: u32, height: u32, channels: u8) -> Vec<(Vec<u8>, (u32, u32))> {
    let channels = usize::from(channels);
    let mut levels = vec![(data.to_vec(), (width, height))];
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

#[cfg(test)]
mod tests {
    use super::mip_chain;

    #[test]
    fn mip_chain_halves_down_to_one_texel() {
        let data = vec![255; 4 * 4 * 4];
        let levels = mip_chain(&data, 4, 4, 4);
        let sizes: Vec<(u32, u32)> = levels.iter().map(|(_, size)| *size).collect();
        assert_eq!(sizes, [(4, 4), (2, 2), (1, 1)]);
        assert_eq!(levels[2].0, [255, 255, 255, 255]);
    }

    #[test]
    fn mip_chain_keeps_odd_edges_and_weights_by_alpha() {
        // A 3 by 1 row: opaque red, transparent green, opaque red.
        let data = vec![255, 0, 0, 255, 0, 255, 0, 0, 255, 0, 0, 255];
        let levels = mip_chain(&data, 3, 1, 4);
        assert_eq!(levels[1].1, (1, 1));
        // The transparent green texel contributes no color, only alpha.
        assert_eq!(levels[1].0, [255, 0, 0, 128]);
    }

    #[test]
    fn mip_chain_averages_single_channel() {
        let data = vec![0, 100, 200, 100];
        let levels = mip_chain(&data, 2, 2, 1);
        assert_eq!(levels[1].0, [100]);
    }
}
