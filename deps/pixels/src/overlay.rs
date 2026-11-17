//! A picture laid over a YUV frame, what the export of a video does with an
//! overlay. The picture is turned into the 3 planes of the frame once, then
//! every frame blends them in at the opacity of that moment.

/// The cover of a sample at full alpha and full opacity.
const FULL: u32 = 255 * 256;

/// One plane of a picture that goes over a plane of a frame: where it lies
/// in that plane, and for every sample its value times its alpha, and the
/// alpha. Row after row, `width` wide.
pub struct OverlayPlane {
    pub left:    usize,
    pub top:     usize,
    pub width:   usize,
    pub samples: Vec<u8>,
    pub alpha:   Vec<u8>,
}

/// Every color of straight alpha RGBA pixels times its alpha.
pub fn premultiply_rgba(rgba: &mut [u8]) {
    for pixel in rgba.as_chunks_mut::<4>().0 {
        let alpha = u32::from(pixel[3]);
        for channel in &mut pixel[..3] {
            let value = (u32::from(*channel) * alpha + 127) / 255;
            *channel = u8::try_from(value).expect("a channel times its alpha fits u8");
        }
    }
}

/// The part of `length` samples from `start` that lies inside `limit`: the
/// first place in the frame, how many, and how many of the picture are cut
/// off before it.
fn clip(start: i64, length: usize, limit: usize) -> Option<(usize, usize, usize)> {
    let end = start.checked_add(i64::try_from(length).ok()?)?.min(i64::try_from(limit).ok()?);
    let first = start.max(0);
    if end <= first {
        return None;
    }
    Some((
        usize::try_from(first).ok()?,
        usize::try_from(end - first).ok()?,
        usize::try_from(first - start).ok()?,
    ))
}

/// A sample times its alpha from a premultiplied pixel. A weight is a part
/// of 65536, the 4th one is for the alpha and carries the offset of the
/// sample.
fn weigh(weights: &[i32; 4], pixel: &[u8]) -> i64 {
    weights
        .iter()
        .zip(pixel)
        .map(|(weight, channel)| i64::from(*weight) * i64::from(*channel))
        .sum()
}

fn sample(sum: i64) -> u8 {
    u8::try_from(((sum + 32768) >> 16).clamp(0, 255)).expect("a clamped sample fits u8")
}

/// The luma and the 2 chroma planes of a picture for a YUV 4:2:0 frame.
/// `rgba` is premultiplied, `width` wide, and its top left pixel lies at
/// `place` in the frame, which can be outside of it. `frame` is the width
/// and the height of the frame. `weights` has a row per plane, see `weigh`.
/// None when no pixel of the picture is inside the frame.
pub fn overlay_planes(
    rgba: &[u8],
    width: usize,
    place: (i64, i64),
    frame: (usize, usize),
    weights: &[[i32; 4]; 3],
) -> Option<[OverlayPlane; 3]> {
    if width == 0 {
        return None;
    }
    let (left, columns, skip_x) = clip(place.0, width, frame.0)?;
    let (top, rows, skip_y) = clip(place.1, rgba.len() / 4 / width, frame.1)?;

    // A chroma sample covers 2 by 2 pixels of the frame. The ones the
    // picture does not reach count as clear.
    let (chroma_left, chroma_top) = (left / 2, top / 2);
    let chroma_width = (left + columns).div_ceil(2) - chroma_left;
    let chroma_rows = (top + rows).div_ceil(2) - chroma_top;
    let mut sums = vec![[0_i64; 3]; chroma_width * chroma_rows];

    let mut luma = Vec::with_capacity(columns * rows);
    let mut luma_alpha = Vec::with_capacity(columns * rows);
    for row in 0..rows {
        let start = ((skip_y + row) * width + skip_x) * 4;
        let line = rgba[start..start + columns * 4].as_chunks::<4>().0;
        let chroma_row = (top.midpoint(row) - chroma_top) * chroma_width;
        for (column, pixel) in line.iter().enumerate() {
            luma.push(sample(weigh(&weights[0], pixel)));
            luma_alpha.push(pixel[3]);
            let sum = &mut sums[chroma_row + left.midpoint(column) - chroma_left];
            sum[0] += weigh(&weights[1], pixel);
            sum[1] += weigh(&weights[2], pixel);
            sum[2] += i64::from(pixel[3]);
        }
    }

    let chroma = |index: usize| OverlayPlane {
        left:    chroma_left,
        top:     chroma_top,
        width:   chroma_width,
        samples: sums.iter().map(|sum| sample(sum[index] / 4)).collect(),
        alpha:   sums
            .iter()
            .map(|sum| u8::try_from((sum[2] + 2) / 4).expect("a mean alpha fits u8"))
            .collect(),
    };
    let (blue, red) = (chroma(0), chroma(1));
    Some([
        OverlayPlane {
            left,
            top,
            width: columns,
            samples: luma,
            alpha: luma_alpha,
        },
        blue,
        red,
    ])
}

/// Blends a plane of a picture into the same plane of a frame. `target` has
/// `stride` bytes a row. `opacity` goes from 0, nothing, to 256, the picture
/// as it is.
pub fn blend_plane(target: &mut [u8], stride: usize, plane: &OverlayPlane, opacity: u32) {
    if plane.width == 0 || opacity == 0 {
        return;
    }
    let opacity = opacity.min(256);
    let rows = plane
        .samples
        .chunks_exact(plane.width)
        .zip(plane.alpha.chunks_exact(plane.width));
    for (row, (samples, alphas)) in rows.enumerate() {
        let start = (plane.top + row) * stride + plane.left;
        let Some(line) = target.get_mut(start..start + plane.width) else {
            return;
        };
        for ((pixel, sample), alpha) in line.iter_mut().zip(samples).zip(alphas) {
            let kept = u32::from(*pixel) * (FULL - u32::from(*alpha) * opacity);
            let added = u32::from(*sample) * opacity * 255;
            let value = ((kept + added + FULL / 2) / FULL).min(255);
            *pixel = u8::try_from(value).expect("a clamped sample fits u8");
        }
    }
}
