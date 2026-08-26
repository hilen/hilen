use std::hash::{Hash, Hasher};

use crate::gm::{
    color::{Color, U8Color},
    num::lossy_convert::LossyConvert,
};

/// Encoded sRGB color. Channels hold the same 0..1 fractions a CSS or
/// Figma value means, so `Color::rgb(0.133, 0.773, 0.369)` is
/// `#22c55e`. The pipeline blends these values as they are, the same
/// math browsers and design tools use, and never converts them.
impl Color<f32> {
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self::rgba(r, g, b, 1.0)
    }

    /// The same color a stylesheet or Figma shows, `Color::hex("#22c55e")`.
    pub const fn hex(hex: &str) -> Self {
        let color = U8Color::parse_hex(hex);
        Self::rgba(
            color.r as f32 / 255.0,
            color.g as f32 / 255.0,
            color.b as f32 / 255.0,
            1.0,
        )
    }
}

impl From<Color> for U8Color {
    fn from(value: Color) -> Self {
        U8Color::rgba(
            (255.0 * value.r).round().lossy_convert(),
            (255.0 * value.g).round().lossy_convert(),
            (255.0 * value.b).round().lossy_convert(),
            (255.0 * value.a).round().lossy_convert(),
        )
    }
}

impl From<&str> for Color<f32> {
    fn from(value: &str) -> Self {
        U8Color::parse_hex(value).into()
    }
}

pub const BLACK: Color = Color::hex("#000000");
pub const WHITE: Color = Color::hex("#ffffff");
pub const RED: Color = Color::hex("#ff0000");
pub const GREEN: Color = Color::hex("#00ff00");
pub const BLUE: Color = Color::hex("#0000e7");
pub const LIGHT_BLUE: Color = Color::hex("#00daff");
pub const GRAY_BLUE: Color = Color::hex("#597c95");
pub const YELLOW: Color = Color::hex("#ffff00");
pub const ORANGE: Color = Color::hex("#ffcb00");
pub const PURPLE: Color = Color::hex("#ff00ff");
pub const TURQUOISE: Color = Color::hex("#00ffff");
pub const GRAY: Color = Color::hex("#bcbcbc");
pub const BROWN: Color = Color::hex("#daaa7c");
pub const LIGHT_GRAY: Color = Color::hex("#e7e7e7");
pub const LIGHTER_GRAY: Color = Color::hex("#f3f3f3");
pub const CLEAR: Color = Color::rgba(0.0, 0.0, 0.0, 0.0);

impl Color {
    pub(crate) const ALL: [Color; 13] = [
        BLACK,
        WHITE,
        RED,
        GREEN,
        BLUE,
        LIGHT_BLUE,
        YELLOW,
        ORANGE,
        PURPLE,
        TURQUOISE,
        BROWN,
        LIGHT_GRAY,
        LIGHTER_GRAY,
    ];

    pub fn random() -> Self {
        *Self::ALL.get(fastrand::usize(..Self::ALL.len())).unwrap()
    }

    /// The `#rrggbb` form of the color, alpha dropped. Public so an app
    /// can tint its own inline svg icons the way the engine's `Tinted`
    /// image does.
    pub fn as_hex(&self) -> String {
        U8Color::from(*self).as_hex()
    }
}

impl Hash for Color<f32> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u32(self.r.to_bits());
        state.write_u32(self.g.to_bits());
        state.write_u32(self.b.to_bits());
        state.write_u32(self.a.to_bits());
    }
}

#[cfg(test)]
mod tests {
    use crate::gm::color::{CLEAR, Color, U8Color, WHITE};

    #[test]
    fn color_diff() {
        assert!((WHITE.diff(CLEAR) - 4.0).abs() < f32::EPSILON);
        assert!(WHITE.diff(WHITE).abs() < f32::EPSILON);
        assert!((WHITE.diff(WHITE.with_alpha(0.9)) - 0.1).abs() < 0.000_001);
    }

    #[test]
    fn color_to_u8() {
        let color: U8Color = Color::rgba(0.5, 1.0, 0.1, 0.0).into();
        assert_eq!(color, U8Color::rgba(128, 255, 26, 0));
    }

    #[test]
    fn hex_round_trip() {
        assert_eq!(Color::hex("#22c55e").as_hex(), "#22c55e");
        assert_eq!(Color::hex("#0a0a0a").as_hex(), "#0a0a0a");
        assert_eq!(Color::hex("#ffffff").as_hex(), "#ffffff");
    }
}
