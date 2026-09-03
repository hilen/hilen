use crate::gm::color::Color;

/// Distance fog. Every surface blends towards `color` with its distance
/// from the camera, untouched up to `start` units away and wholly fog
/// from `end` on, so a big level fades out instead of ending at the far
/// plane. The sky is fog colored at the horizon and clears with height,
/// gone where the view direction rises past `height`, the sine of that
/// elevation, so the fogged ground meets the sky. Without a sky the fog
/// color fills the background.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fog {
    pub color:  Color,
    pub start:  f32,
    pub end:    f32,
    pub height: f32,
}

impl Fog {
    /// The fog clears by 24 degrees up, the default `height` of 0.4.
    pub fn new(color: Color, start: f32, end: f32) -> Self {
        Self {
            color,
            start,
            end,
            height: 0.4,
        }
    }

    /// The start and the reach the shader reads, one over the length of
    /// the fade so the fragment stage multiplies instead of dividing.
    /// An `end` at or before `start` is a hard cut at `start`.
    pub(crate) fn range(&self) -> (f32, f32) {
        let length = (self.end - self.start).max(1e-4);
        (self.start, 1.0 / length)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn the_range_inverts_the_fade_length() {
        let (start, inverse) = Fog::new(Color::hex("#ffffff"), 10.0, 60.0).range();
        assert!((start - 10.0).abs() < f32::EPSILON);
        assert!((inverse - 0.02).abs() < 1e-6);
    }

    // The shader saturates `(distance - start) * inverse`, so a zero or
    // backwards fade must not divide by zero and lands as a hard cut.
    #[test]
    fn a_backwards_fade_is_a_hard_cut() {
        let (start, inverse) = Fog::new(Color::hex("#ffffff"), 20.0, 20.0).range();
        assert!((start - 20.0).abs() < f32::EPSILON);
        assert!(inverse.is_finite() && inverse > 1000.0);
    }
}
