use crate::{
    gm::LossyConvert,
    refs::{Weak, manage::DataManager},
    ui::{Color, Image, UIManager},
};

// The svg icons are drawn in #000000 and the tint replaces that hex
// before rasterizing.

const SPLIT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 16 16" fill="none" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="5" height="10" rx="1" /><rect x="9" y="3" width="5" height="10" rx="1" /></svg>"##;

const UNIFIED: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 16 16" fill="none" stroke="#000000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="12" height="10" rx="1" /><line x1="2" y1="8" x2="14" y2="8" /></svg>"##;

const X: &str = r#"<line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />"#;

pub fn tinted(name: &str, svg: &str, color: Color) -> Weak<Image> {
    let hex = color.as_hex();
    let data = svg.replace("#000000", &hex);
    Image::load(data.as_bytes(), format!("icon:{name}:{hex}"))
}

// A feather style icon on its view box, rasterized at the exact number
// of physical pixels it is drawn at, so the strokes stay as thick as the
// browser draws them instead of thinning out through a downscale.
pub fn stroked(
    name: &str,
    view_box: u32,
    body: &str,
    stroke_width: f32,
    size: f32,
    color: Color,
) -> Weak<Image> {
    let px: u32 = (size * UIManager::scale()).round().max(1.0).lossy_convert();
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{px}" height="{px}" viewBox="0 0 {view_box} {view_box}" fill="none" stroke="#000000" stroke-width="{stroke_width}" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"##
    );
    tinted(&format!("{name}:{px}"), &svg, color)
}

pub fn split(color: Color) -> Weak<Image> {
    tinted("split", SPLIT, color)
}

pub fn unified(color: Color) -> Weak<Image> {
    tinted("unified", UNIFIED, color)
}

pub fn x(size: f32, color: Color) -> Weak<Image> {
    stroked("x", 24, X, 2.0, size, color)
}
