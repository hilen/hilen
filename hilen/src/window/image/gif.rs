use std::io::Cursor;

use anyhow::{Result, ensure};
use image::{AnimationDecoder, codecs::gif::GifDecoder};

use crate::gm::{LossyConvert, flat::Size};

/// One decoded gif frame, already composited to the full canvas by the decoder,
/// so every frame is `size` and disposal is handled. `pixels` is straight RGBA.
pub(crate) struct GifFrame {
    pub pixels: Vec<u8>,
    /// Seconds this frame shows before the next one.
    pub delay:  f32,
}

pub(crate) struct DecodedGif {
    pub size:   Size<u32>,
    pub frames: Vec<GifFrame>,
}

/// Slowest a frame may show. A gif with a zero delay means "as fast as
/// possible", which the frame stepper would spin on, so clamp it the way
/// browsers do to a small floor.
const MIN_DELAY: f32 = 0.01;

pub(crate) fn decode_gif(data: &[u8]) -> Result<DecodedGif> {
    let decoder = GifDecoder::new(Cursor::new(data.to_vec()))?;
    let frames = decoder.into_frames().collect_frames()?;
    ensure!(!frames.is_empty(), "gif has no frames");

    let first = frames[0].buffer();
    let size = Size::new(first.width(), first.height());

    let frames = frames
        .into_iter()
        .map(|frame| {
            let (numerator, denominator) = frame.delay().numer_denom_ms();
            let numerator: f32 = numerator.lossy_convert();
            let denominator: f32 = denominator.max(1).lossy_convert();
            let delay = (numerator / denominator / 1000.0).max(MIN_DELAY);
            GifFrame {
                pixels: frame.into_buffer().into_raw(),
                delay,
            }
        })
        .collect();

    Ok(DecodedGif { size, frames })
}
