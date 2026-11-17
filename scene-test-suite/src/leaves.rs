use hilen::{
    gm::LossyConvert,
    refs::Weak,
    ui::{Image, Size},
};

/// Texels along each side of the texture, and leaves along each side.
const TEXELS: u32 = 128;
const LEAVES: u32 = 4;
const LEAF: [u8; 3] = [0x3f, 0xa3, 0x4d];
/// The color between the leaves, seen only where nothing cuts it out.
const GAP: [u8; 3] = [0x14, 0x40, 0x1c];

/// A grid of round leaves. A leaf's alpha is one at its middle and falls
/// evenly to zero at its rim, where the leaves touch, so the threshold
/// of a cutout is the size the leaves are cut to. Between the leaves the
/// alpha is zero.
pub(crate) fn leaves() -> Weak<Image> {
    let cell: f32 = (TEXELS / LEAVES).lossy_convert();
    let radius = cell / 2.0;
    let mut data = Vec::with_capacity((TEXELS * TEXELS * 4) as usize);

    for y in 0..TEXELS {
        for x in 0..TEXELS {
            let dx: f32 = (x % (TEXELS / LEAVES)).lossy_convert() - radius + 0.5;
            let dy: f32 = (y % (TEXELS / LEAVES)).lossy_convert() - radius + 0.5;
            let alpha = (1.0 - (dx * dx + dy * dy).sqrt() / radius).max(0.0);
            data.extend(if alpha > 0.0 { LEAF } else { GAP });
            data.push((alpha * 255.0).round().lossy_convert());
        }
    }

    Image::from_raw_data(data, "cutout_leaves", Size::new(TEXELS, TEXELS), 4)
}
