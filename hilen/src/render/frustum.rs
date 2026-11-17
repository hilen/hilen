use crate::gm::volume::{Mat4, Vec3, Vec4};

/// `HILEN_CULL=0` draws every node in every pass, the A/B lever for a
/// measurement of the culling.
pub(crate) fn enabled() -> bool {
    #[cfg(not_wasm)]
    {
        use std::sync::OnceLock;

        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| !std::env::var("HILEN_CULL").is_ok_and(|value| value == "0"))
    }
    #[cfg(wasm)]
    {
        true
    }
}

/// The 6 planes of the clip volume of a view projection with depth from
/// 0 to 1, each facing in and of unit length, so a node outside one of
/// them can be left out of a pass. The GPU clips the same volume, so
/// leaving such a node out draws the same picture.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Frustum([Vec4; 6]);

impl Frustum {
    pub(crate) fn of(view_proj: Mat4) -> Self {
        let [x, y, z, w] = [0, 1, 2, 3].map(|index| view_proj.row(index));
        Self([w + x, w - x, w + y, w - y, z, w - z].map(|plane| {
            let length = plane.truncate().length();
            if length > 0.0 { plane / length } else { plane }
        }))
    }

    /// Whether some of the sphere at `center` with `radius` can be inside,
    /// `center` in the space the view projection takes.
    pub(crate) fn holds(&self, center: Vec3, radius: f32) -> bool {
        self.0.iter().all(|plane| plane.truncate().dot(center) + plane.w >= -radius)
    }
}

#[cfg(test)]
mod test {
    use glam::camera::rh::{
        proj::directx::{orthographic, perspective},
        view::look_at_mat4,
    };

    use super::*;

    fn camera() -> Frustum {
        let view = look_at_mat4(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
        Frustum::of(perspective(1.0, 1.0, 0.1, 100.0) * view)
    }

    #[test]
    fn a_camera_holds_what_is_in_front_of_it() {
        let frustum = camera();
        assert!(frustum.holds(Vec3::new(0.0, 0.0, -10.0), 0.5));
        assert!(frustum.holds(Vec3::new(0.0, 0.0, -99.0), 0.5));
    }

    #[test]
    fn a_camera_leaves_out_what_is_behind_aside_or_past_its_far_end() {
        let frustum = camera();
        assert!(!frustum.holds(Vec3::new(0.0, 0.0, 10.0), 0.5));
        assert!(!frustum.holds(Vec3::new(50.0, 0.0, -10.0), 0.5));
        assert!(!frustum.holds(Vec3::new(0.0, 0.0, -120.0), 0.5));
    }

    // A tree whose trunk is out of the view can still put its crown in
    // it, the whole sphere has to be out.
    #[test]
    fn a_sphere_over_an_edge_is_held() {
        let frustum = camera();
        assert!(frustum.holds(Vec3::new(0.0, 0.0, 1.0), 2.0));
        assert!(frustum.holds(Vec3::new(8.0, 0.0, -10.0), 4.0));
    }

    #[test]
    fn a_box_of_the_sun_holds_its_own_inside() {
        let view = look_at_mat4(Vec3::new(0.0, 50.0, 0.0), Vec3::ZERO, Vec3::Z);
        let frustum = Frustum::of(orthographic(-10.0, 10.0, -10.0, 10.0, 0.0, 100.0) * view);
        assert!(frustum.holds(Vec3::ZERO, 1.0));
        assert!(frustum.holds(Vec3::new(9.0, -40.0, 9.0), 0.5));
        assert!(!frustum.holds(Vec3::new(30.0, 0.0, 0.0), 1.0));
        assert!(!frustum.holds(Vec3::new(0.0, 60.0, 0.0), 1.0));
    }
}
