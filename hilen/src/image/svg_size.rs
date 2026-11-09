//! The size of an svg read from its root tag, for a browser build that has
//! no svg parser. It matches what usvg reads on the other platforms.

use crate::gm::LossyConvert;

/// The old fixed raster used by sprites and levels is eight times the svg.
const FIXED_SCALE: f32 = 8.0;

/// The side of the fixed raster. usvg rounds the size of the svg up before
/// it scales.
pub(crate) fn fixed_side(side: f32) -> u32 {
    (side.ceil() * FIXED_SCALE).round().lossy_convert()
}

/// Where the root tag starts and the text between `<svg` and its `>`.
fn root_tag(text: &str) -> Option<(usize, &str)> {
    let start = text.find("<svg")?;
    let end = text[start..].find('>')?;
    Some((start, &text[start + 4..start + end]))
}

/// Where the value of the attribute `name` sits in the text of a tag, and
/// the value.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<(usize, &'a str)> {
    let mut from = 0;
    while let Some(found) = tag[from..].find(name) {
        let at = from + found;
        from = at + name.len();
        if !tag[..at].chars().next_back().is_some_and(char::is_whitespace) {
            continue;
        }
        let rest = &tag[from..];
        let Some(after_eq) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let value = after_eq.trim_start();
        let quote = value.chars().next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let start = tag.len() - value.len() + 1;
        let end = value[1..].find(quote)?;
        return Some((start, &tag[start..start + end]));
    }
    None
}

/// A length in pixels, None for a percentage or no number.
fn length(value: &str) -> Option<f32> {
    let value = value.trim();
    let units = [
        ("px", 1.0),
        ("pt", 4.0 / 3.0),
        ("pc", 16.0),
        ("mm", 96.0 / 25.4),
        ("cm", 96.0 / 2.54),
        ("in", 96.0),
    ];
    for (unit, scale) in units {
        if let Some(number) = value.strip_suffix(unit) {
            return number.trim().parse::<f32>().ok().map(|n| n * scale);
        }
    }
    if value.ends_with('%') {
        return None;
    }
    value.parse().ok()
}

/// The width and height the svg draws at, as usvg reads them: the
/// attributes, else the view box, else 100. None with no root tag or no
/// size.
pub(crate) fn intrinsic_size(text: &str) -> Option<(f32, f32)> {
    let (_, tag) = root_tag(text)?;
    let view_box = attribute(tag, "viewBox").and_then(|(_, value)| {
        let values: Vec<f32> = value
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse().ok())
            .collect();
        (values.len() == 4).then(|| (values[2], values[3]))
    });
    let side = |name: &str, from_box: Option<f32>| {
        attribute(tag, name)
            .and_then(|(_, value)| length(value))
            .or(from_box)
            .unwrap_or(100.0)
    };
    let width = side("width", view_box.map(|(width, _)| width));
    let height = side("height", view_box.map(|(_, height)| height));
    (width > 0.0 && height > 0.0).then_some((width, height))
}

/// Sets the attribute `name` of the root tag to `value`, in place of the
/// old value or as a new attribute.
fn set_attribute(text: &mut String, name: &str, value: &str) {
    let Some((start, tag)) = root_tag(text) else {
        return;
    };
    let tag_at = start + 4;
    match attribute(tag, name) {
        Some((at, old)) => {
            let at = tag_at + at;
            text.replace_range(at..at + old.len(), value);
        }
        None => text.insert_str(tag_at, &format!(r#" {name}="{value}""#)),
    }
}

/// The svg drawn at `width` by `height` pixels. A browser turns an svg image
/// into pixels at the size on its root tag and scales that bitmap to the
/// canvas, which blurs thin strokes, so the tag gets the pixel size. A view
/// box of the old `size` keeps the drawing scaling with it.
pub(crate) fn sized(text: &str, size: (f32, f32), width: u32, height: u32) -> String {
    let mut text = text.to_string();
    let has_view_box = root_tag(&text).is_some_and(|(_, tag)| attribute(tag, "viewBox").is_some());
    if !has_view_box {
        set_attribute(&mut text, "viewBox", &format!("0 0 {} {}", size.0, size.1));
    }
    set_attribute(&mut text, "height", &height.to_string());
    set_attribute(&mut text, "width", &width.to_string());
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_from_attributes_view_box_or_default() {
        let icon = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" stroke-width="2">"#;
        assert_eq!(intrinsic_size(icon), Some((24.0, 24.0)));
        let boxed = r#"<svg viewBox="0 0 48 16"><path/></svg>"#;
        assert_eq!(intrinsic_size(boxed), Some((48.0, 16.0)));
        let units = r#"<?xml version="1.0"?><svg width="10px" height='15pt'></svg>"#;
        assert_eq!(intrinsic_size(units), Some((10.0, 20.0)));
        let percent = r#"<svg width="100%" height="100%" viewBox="0,0,30,40"></svg>"#;
        assert_eq!(intrinsic_size(percent), Some((30.0, 40.0)));
        assert_eq!(intrinsic_size("<svg></svg>"), Some((100.0, 100.0)));
        assert_eq!(intrinsic_size("<html></html>"), None);
    }

    #[test]
    fn stroke_width_is_not_the_width() {
        let tag = r#" stroke-width="2" width="5""#;
        assert_eq!(attribute(tag, "width").map(|(_, value)| value), Some("5"));
    }

    #[test]
    fn the_tag_gets_the_pixel_size() {
        let boxed = r#"<svg viewBox="0 0 48 16"><path/></svg>"#;
        assert_eq!(
            sized(boxed, (48.0, 16.0), 96, 32),
            r#"<svg width="96" height="32" viewBox="0 0 48 16"><path/></svg>"#
        );
        let icon = r#"<svg width="24" height="24" viewBox="0 0 24 24"></svg>"#;
        assert_eq!(
            sized(icon, (24.0, 24.0), 16, 16),
            r#"<svg width="16" height="16" viewBox="0 0 24 24"></svg>"#
        );
        let percent = r#"<svg width="100%" height='100%' viewBox="0 0 30 40"></svg>"#;
        assert_eq!(
            sized(percent, (30.0, 40.0), 30, 40),
            r#"<svg width="30" height='40' viewBox="0 0 30 40"></svg>"#
        );
    }

    #[test]
    fn a_view_box_goes_on_a_tag_that_has_none() {
        let plain = r#"<svg width="10" height="20"></svg>"#;
        assert_eq!(
            sized(plain, (10.0, 20.0), 30, 60),
            r#"<svg viewBox="0 0 10 20" width="30" height="60"></svg>"#
        );
    }

    #[test]
    fn fixed_raster_is_eight_times_the_rounded_up_size() {
        assert_eq!(fixed_side(24.0), 192);
        assert_eq!(fixed_side(10.5), 88);
    }
}
