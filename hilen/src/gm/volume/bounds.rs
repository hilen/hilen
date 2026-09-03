use crate::gm::volume::Vec3;

/// An axis aligned box, the extent of a model in its own space.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bounds {
    pub min: Vec3,
    pub max: Vec3,
}

impl Bounds {
    /// The tightest box around the points, zero sized at the origin
    /// when there are none.
    pub fn of_points(points: impl IntoIterator<Item = Vec3>) -> Self {
        let mut points = points.into_iter();
        let Some(first) = points.next() else {
            return Self::default();
        };
        points.fold(
            Self {
                min: first,
                max: first,
            },
            |bounds, point| Self {
                min: bounds.min.min(point),
                max: bounds.max.max(point),
            },
        )
    }

    pub fn center(&self) -> Vec3 {
        (self.min + self.max) / 2.0
    }

    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }

    pub fn half_extents(&self) -> Vec3 {
        self.size() / 2.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn bounds_wrap_every_point() {
        let bounds = Bounds::of_points([
            Vec3::new(1.0, -2.0, 3.0),
            Vec3::new(-1.0, 2.0, 0.0),
            Vec3::new(0.0, 0.0, -3.0),
        ]);
        assert_eq!(bounds.min, Vec3::new(-1.0, -2.0, -3.0));
        assert_eq!(bounds.max, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(bounds.center(), Vec3::ZERO);
        assert_eq!(bounds.half_extents(), Vec3::new(1.0, 2.0, 3.0));
    }

    #[test]
    fn no_points_is_a_zero_box() {
        assert_eq!(Bounds::of_points([]), Bounds::default());
    }
}
