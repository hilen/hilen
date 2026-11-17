//! The content of a loop at phase 0 folded into 1 number. Pictures of a
//! content are drawn once and drawn again only when it changes, by a
//! setter, a theme, a scale or a size. The place of the view is not in it,
//! a layer moves without new pictures.

use std::{mem::discriminant, ops::Deref};

use crate::{
    gm::flat::{Point, Rect},
    ui::{DrawingView, ImageView, Label, UIManager, View, ViewData, ViewSubviews, damage::PaintKey},
    window::Window,
};

pub(super) fn content(view: &dyn View, pixels: Rect<u32>, own: bool) -> u64 {
    let mut key = PaintKey::default();
    key.hash(&pixels.width())
        .hash(&pixels.height())
        .hash(&own)
        .pod(&UIManager::scale())
        .pod(&Window::screen_scale());
    feed(&mut key, view, view.__base_view().__absolute_frame.origin);
    key.finish()
}

fn feed(key: &mut PaintKey, view: &dyn View, origin: Point) {
    let base = view.__base_view();
    let frame = base.__absolute_frame;
    if view.is_hidden() || frame.size.has_no_area() || base.tree_opacity <= 0.0 {
        key.hash(&false);
        return;
    }

    key.hash(&true)
        .pod(&moved(frame, origin))
        .pod(&base.tree_opacity)
        .pod(view.color())
        .pod(view.border_color())
        .pod(&view.border_width())
        .pod(&view.corner_radii())
        .hash(&view.__internal_clips_to_bounds());

    if let Some(gradient) = view.gradient() {
        key.debug(&gradient);
    }
    if let Some(shadow) = view.shadow() {
        key.debug(&shadow);
    }

    if let Some(label) = view.as_any().downcast_ref::<Label>() {
        label.feed_paint(key);
    } else if let Some(image_view) = view.as_any().downcast_ref::<ImageView>() {
        let image = image_view.image();
        if image.is_ok() {
            key.hash(&image.addr())
                .hash(&image.size.width)
                .hash(&image.size.height)
                .pod(&moved(image_view.image_frame(), origin))
                .pod(&image_view.uv_rect())
                .hash(&image_view.flip_x)
                .hash(&image_view.flip_y)
                .hash(&discriminant(&image_view.mode));
            if let Some(cut) = image_view.cut() {
                key.debug(&cut);
            }
        }
    } else if let Some(drawing) = view.as_any().downcast_ref::<DrawingView>() {
        for path in drawing.paths() {
            path.feed_shape(key);
        }
    }

    key.hash(&view.subviews().len());
    for child in view.subviews() {
        feed(key, child.deref(), origin);
    }
}

fn moved(rect: Rect, origin: Point) -> Rect {
    Rect::new(
        rect.x() - origin.x,
        rect.y() - origin.y,
        rect.width(),
        rect.height(),
    )
}
