use crate::{deps::refs::Weak, gm::volume::Vec3, scene::Node};

/// An over the shoulder camera for a `Player`, the way most third person
/// games frame their hero: the camera turns around a pivot above the
/// player with its yaw and pitch, sits `distance` behind it and a little
/// to the right, and looks where the player looks, so the player stands
/// on one side of the view and the aim line runs clear through the other.
/// A wall between the pivot and the camera pulls the camera in front of
/// it, so the view never ends up inside or behind scenery.
#[derive(Clone)]
pub struct ThirdPerson {
    /// How far above the capsule's center the camera turns around.
    pub pivot_height: f32,
    /// How far behind the pivot the camera sits with nothing in the way,
    /// what a zoom changes.
    pub distance:     f32,
    /// How far to the right of the pivot, the over the shoulder shift.
    pub shoulder:     f32,
    /// The closest a wall can push the camera to the pivot.
    pub min_distance: f32,
    /// The gap kept between the camera and a wall it was pulled in by.
    pub margin:       f32,
    /// Nodes the camera looks through instead of being pulled in by, like
    /// the hitbox of a swing in front of the player.
    pub skip:         Vec<Weak<dyn Node>>,
}

impl Default for ThirdPerson {
    fn default() -> Self {
        Self {
            pivot_height: 0.6,
            distance:     6.0,
            shoulder:     0.3,
            min_distance: 0.5,
            margin:       0.2,
            skip:         vec![],
        }
    }
}

impl ThirdPerson {
    /// Where the camera wants to be from the pivot with nothing in the
    /// way, looking along `forward` with `right` to its right.
    pub fn reach(&self, forward: Vec3, right: Vec3) -> Vec3 {
        right * self.shoulder - forward * self.distance
    }

    /// Where the camera goes. `hit` is how far along the reach from the
    /// pivot the first wall is, if any. The camera then keeps `margin`
    /// in front of it, closing in along its distance while the shoulder
    /// shift stays, and never closer than `min_distance`.
    pub fn place(&self, pivot: Vec3, forward: Vec3, right: Vec3, hit: Option<f32>) -> Vec3 {
        let full = self.reach(forward, right).length();
        let distance = match hit {
            Some(hit) if full > 0.0 => {
                let room = (hit - self.margin).max(0.0) / full;
                (self.distance * room).clamp(self.min_distance.min(self.distance), self.distance)
            }
            _ => self.distance,
        };
        pivot + right * self.shoulder - forward * distance
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const FORWARD: Vec3 = Vec3::NEG_Z;
    const RIGHT: Vec3 = Vec3::X;

    #[test]
    fn with_nothing_in_the_way_it_sits_behind_and_to_the_right() {
        let rig = ThirdPerson::default();
        let camera = rig.place(Vec3::Y, FORWARD, RIGHT, None);
        assert!(camera.distance(Vec3::new(0.3, 1.0, 6.0)) < 1e-5);
    }

    // A wall halfway along pulls the camera in to just short of it, the
    // shoulder shift stays so the player keeps their side of the view.
    #[test]
    fn a_wall_pulls_it_in_front_of_the_wall() {
        let rig = ThirdPerson::default();
        let full = rig.reach(FORWARD, RIGHT).length();
        let camera = rig.place(Vec3::ZERO, FORWARD, RIGHT, Some(full / 2.0));
        let expected = 6.0 * (full / 2.0 - 0.2) / full;
        assert!((camera.z - expected).abs() < 1e-5, "{camera}");
        assert!((camera.x - 0.3).abs() < 1e-5);
    }

    #[test]
    fn a_wall_at_the_pivot_stops_it_at_the_least_distance() {
        let rig = ThirdPerson::default();
        let camera = rig.place(Vec3::ZERO, FORWARD, RIGHT, Some(0.05));
        assert!((camera.z - 0.5).abs() < 1e-5, "{camera}");
    }
}
