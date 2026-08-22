use crate::gm::{ToF32, color::Color};

pub type U8Color = Color<u8>;

impl U8Color {
    pub const fn const_rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub(crate) const fn parse_hex(hex: &str) -> Self {
        const fn hex_char_to_u8(c: u8) -> u8 {
            match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                b'A'..=b'F' => c - b'A' + 10,
                _ => panic!("Invalid hex character"),
            }
        }

        let bytes = hex.as_bytes();

        // Determine the starting offset (skip '#' if present)
        let offset = if !bytes.is_empty() && bytes[0] == b'#' {
            1
        } else {
            0
        };

        let r = hex_char_to_u8(bytes[offset]) * 16 + hex_char_to_u8(bytes[offset + 1]);
        let g = hex_char_to_u8(bytes[offset + 2]) * 16 + hex_char_to_u8(bytes[offset + 3]);
        let b = hex_char_to_u8(bytes[offset + 4]) * 16 + hex_char_to_u8(bytes[offset + 5]);

        Self { r, g, b, a: 255 }
    }

    pub fn as_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub(crate) fn diff_u8(self, other: Self) -> i16 {
        (i16::from(self.r) - i16::from(other.r)).abs()
            + (i16::from(self.g) - i16::from(other.g)).abs()
            + (i16::from(self.b) - i16::from(other.b)).abs()
            + (i16::from(self.a) - i16::from(other.a)).abs()
    }
}

impl From<U8Color> for Color {
    fn from(value: U8Color) -> Self {
        Color::rgba(
            f32::from(value.r) / 255.0,
            f32::from(value.g) / 255.0,
            f32::from(value.b) / 255.0,
            f32::from(value.a) / 255.0,
        )
    }
}

impl<R: ToF32, G: ToF32, B: ToF32> From<(R, G, B)> for Color {
    fn from(value: (R, G, B)) -> Self {
        Color::rgba(
            value.0.to_f32() / 255.0,
            value.1.to_f32() / 255.0,
            value.2.to_f32() / 255.0,
            1.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gm::color::{BLACK, GRAY, GREEN, RED, WHITE};

    #[test]
    fn test_color_from_u8() {
        assert_eq!(U8Color::const_rgb(0, 0, 0).as_hex(), "#000000");
        assert_eq!(U8Color::const_rgb(255, 255, 255).as_hex(), "#ffffff");
        assert_eq!(BLACK.as_hex(), "#000000");
        assert_eq!(WHITE.as_hex(), "#ffffff");
        assert_eq!(RED.as_hex(), "#ff0000");
        assert_eq!(GREEN.as_hex(), "#00ff00");
        assert_eq!(GRAY.as_hex(), "#bcbcbc");
    }

    #[test]
    fn test_color_from_hex() {
        assert_eq!(U8Color::parse_hex("858AE3"), U8Color::const_rgb(133, 138, 227));
        assert_eq!(U8Color::parse_hex("#50C5B7"), U8Color::const_rgb(80, 197, 183));
    }
}
