use std::sync::atomic::Ordering;

use wgpu::RenderPass;

use super::{IMAGE_RECT_DRAWER, UIDrawer, set_viewport};
use crate::{
    deps::refs::Weak,
    gm::{
        color::CLEAR,
        flat::{CornerRadii, Point, Rect},
    },
    render::{
        ImageKey,
        data::{RectView, UIImageInstance},
    },
    ui::{UIManager, ViewFrame},
    window::{Window, image::Image},
};

/// Farther than the root view and every view under it, nearer than the
/// clear of 1 of the depth.
const BEHIND_VIEWS_Z: f32 = 0.99;

impl UIDrawer {
    /// Draws `image` over the whole window at once, behind every view. Queued
    /// with the views it would come after their rects, and a see through
    /// rect would blend over an empty frame.
    pub(crate) fn draw_behind_views(pass: &mut RenderPass, image: Weak<Image>) {
        image.drawn_at.store(Window::render_frame(), Ordering::Relaxed);
        let resolution = UIManager::window_resolution();
        set_viewport(pass, resolution);
        let frame = Rect {
            origin: Point::default(),
            size:   UIManager::root_view().size(),
        };
        let images = IMAGE_RECT_DRAWER.get_mut();
        images.add_with_image(
            UIImageInstance::new(
                frame,
                (0, 0, 1, 1).into(),
                CLEAR,
                0.0,
                CornerRadii::default(),
                BEHIND_VIEWS_Z,
                false,
                false,
                UIManager::scale(),
            ),
            ImageKey {
                image,
                raster: None,
                depth: None,
            },
        );
        images.draw(
            pass,
            RectView {
                resolution,
                _padding: 0,
            },
        );
    }
}
