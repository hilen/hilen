//! Whether a layer of the system can show a loop right. The layer lies over
//! the whole frame, so a view stays on the frame path when anything of the
//! frame should be in front of it or change it: a clip that cuts it, a view
//! drawn over it, a blur over it, a fade of a group it is in, a scale of a
//! canvas it is in.

use std::ops::Deref;

use crate::{
    gm::{
        LossyConvert,
        flat::{Rect, Size},
    },
    ui::{
        BlurView, DrawingView, ImageView, Label, ScrimView, UIManager, View, ViewData, ViewFrame,
        ViewSubviews, WeakView, damage::paint_rect, view::RenderProbe,
    },
};

/// Pixels a picture holds past the frame of the view, the soft edge of a
/// rect and the rounding of a frame to whole pixels. The same as the edge
/// of `ui::damage`.
const EDGE: f32 = 2.0;

/// A loop that a layer can show if nothing covers it.
pub(super) struct Candidate {
    pub(super) view:   WeakView,
    pub(super) pixels: Rect<u32>,
}

/// The pixels of the window the pictures of the loop of `view` cover. For
/// pictures the view has itself `area` is where they go, in pixels of the
/// window. None when the view and the views around it keep a layer from
/// showing it right.
pub(super) fn pixels(view: &dyn View, area: Option<Rect>) -> Option<Rect<u32>> {
    let base = view.__base_view();
    // A canvas zooms what is in it, a fade fades it, and a layer shows its
    // pictures at their own size and at full strength.
    if base.tree_scale.to_bits() != 1.0_f32.to_bits() || base.tree_opacity < 1.0 || base.group_alpha < 1.0 {
        return None;
    }

    let grow = inside(view, true)?;
    let scale = UIManager::scale();
    let rect = match area {
        Some(area) => whole(area, 0.0)?,
        None => whole(base.__absolute_frame * scale, grow * scale + EDGE)?,
    };

    if !in_window(rect) || !in_clips(view, rect, scale) {
        return None;
    }
    Some(rect)
}

/// The rect grown by `grow` in whole pixels, none when it has no area.
fn whole(rect: Rect, grow: f32) -> Option<Rect<u32>> {
    let left = (rect.x() - grow).floor();
    let top = (rect.y() - grow).floor();
    let right = (rect.max_x() + grow).ceil();
    let bottom = (rect.max_y() + grow).ceil();
    if left < 0.0 || top < 0.0 || right <= left || bottom <= top {
        return None;
    }
    Some(Rect::new(left, top, right - left, bottom - top).lossy_convert())
}

fn in_window(rect: Rect<u32>) -> bool {
    let window: Size<u32> = UIManager::window_resolution().lossy_convert();
    rect.x() + rect.width() <= window.width && rect.y() + rect.height() <= window.height
}

/// Every view above that clips leaves the whole rect. A rounded clip only
/// counts inside its corners.
fn in_clips(view: &dyn View, rect: Rect<u32>, scale: f32) -> bool {
    let mut ancestor = *view.superview();
    while ancestor.is_ok() {
        let base = ancestor.__base_view();
        if base.group_alpha < 1.0 {
            return false;
        }
        if ancestor.__internal_clips_to_bounds() {
            let radii = ancestor.corner_radii();
            let radius = radii
                .top_left
                .max(radii.top_right)
                .max(radii.bottom_left)
                .max(radii.bottom_right);
            let clip = base.__absolute_frame * scale;
            let inset = radius * scale;
            let (left, top, right, bottom) = edges(rect);
            if left < clip.x() + inset
                || top < clip.y() + inset
                || right > clip.max_x() - inset
                || bottom > clip.max_y() - inset
            {
                return false;
            }
        }
        ancestor = *ancestor.superview();
    }
    true
}

fn edges(rect: Rect<u32>) -> (f32, f32, f32, f32) {
    let rect: Rect = rect.lossy_convert();
    (rect.x(), rect.y(), rect.max_x(), rect.max_y())
}

/// How far the views of the subtree paint past their frames, in points.
/// None when the subtree has a view that a picture cannot hold: one that
/// draws itself, a blur, a scrim, a hole in the page, a group fade, a view
/// that wants every frame like a video, another loop, or a view that was
/// never drawn.
fn inside(view: &dyn View, top: bool) -> Option<f32> {
    let base = view.__base_view();
    if view.is_hidden() || base.__absolute_frame.size.has_no_area() || base.tree_opacity <= 0.0 {
        return Some(0.0);
    }
    if (!top && base.loop_id != 0)
        || base.render_probe != RenderProbe::Plain
        || base.keeps_frames
        || base.page_hole
        || base.group_alpha < 1.0
        || view.as_any().downcast_ref::<ScrimView>().is_some()
        || view
            .as_any()
            .downcast_ref::<BlurView>()
            .is_some_and(|blur| blur.blur_radius() > 0.0)
    {
        return None;
    }

    let mut grow: f32 = 0.0;
    if let Some(shadow) = view.shadow() {
        grow = shadow.radius * 2.0 + shadow.offset.x.abs().max(shadow.offset.y.abs());
    }
    if let Some(label) = view.as_any().downcast_ref::<Label>() {
        if let Some(outline) = label.text_outline() {
            grow = grow.max(outline.width * 2.0);
        }
        if let Some(shadow) = label.text_shadow() {
            grow = grow.max(shadow.blur * 2.0 + shadow.offset.x.abs().max(shadow.offset.y.abs()));
        }
    }

    for child in view.subviews() {
        grow = grow.max(inside(child.deref(), false)?);
    }
    Some(grow)
}

/// For each candidate, whether a view outside it that is nearer to the
/// viewer paints over its pixels. The layer would hide such a view.
pub(super) fn covered(candidates: &[Candidate]) -> Vec<bool> {
    let mut covered = vec![false; candidates.len()];
    if candidates.is_empty() {
        return covered;
    }
    let depths: Vec<f32> = candidates.iter().map(|candidate| candidate.view.z_position()).collect();
    visit(UIManager::root_view_static(), candidates, &depths, &mut covered);
    covered
}

fn visit(view: &dyn View, candidates: &[Candidate], depths: &[f32], covered: &mut [bool]) {
    let base = view.__base_view();
    if view.is_hidden() || base.__absolute_frame.size.has_no_area() || base.tree_opacity <= 0.0 {
        return;
    }
    // Its own subtree is what the pictures hold.
    let address = std::ptr::from_ref(view).addr();
    if candidates.iter().any(|candidate| candidate.view.addr() == address) {
        return;
    }

    if paints(view) {
        let pixels = painted(view);
        let custom = base.render_probe == RenderProbe::Custom;
        let depth = match view.as_any().downcast_ref::<Label>() {
            Some(_) => view.z_position() - UIManager::additional_z_offset(),
            None => view.z_position(),
        };
        for (index, candidate) in candidates.iter().enumerate() {
            if !covered[index] && overlap(pixels, candidate.pixels) && (custom || depth < depths[index]) {
                covered[index] = true;
            }
        }
    }

    let root = UIManager::root_view_static().frame();
    for child in view.subviews() {
        if child.dont_hide() || child.absolute_frame().intersects(root) {
            visit(child.deref(), candidates, depths, covered);
        }
    }
}

/// Whether the drawer puts any pixel of the view itself on the screen.
fn paints(view: &dyn View) -> bool {
    let base = view.__base_view();
    view.color().a > 0.0
        || (view.border_color().a > 0.0 && view.border_width() > 0.0)
        || view.gradient().is_some()
        || view.shadow().is_some()
        || base.page_hole
        || base.render_probe == RenderProbe::Custom
        || view.as_any().downcast_ref::<BlurView>().is_some()
        || view.as_any().downcast_ref::<ScrimView>().is_some()
        || view
            .as_any()
            .downcast_ref::<Label>()
            .is_some_and(|label| !label.text.is_empty())
        || view
            .as_any()
            .downcast_ref::<ImageView>()
            .is_some_and(|image| image.image().is_ok())
        || view
            .as_any()
            .downcast_ref::<DrawingView>()
            .is_some_and(|drawing| !drawing.paths().is_empty())
}

/// The pixels the view itself can paint. A path counts by the frame of its
/// view here, the damage walk gives it the whole clip.
fn painted(view: &dyn View) -> Rect<u32> {
    let base = view.__base_view();
    let resolution = UIManager::window_resolution();
    let window: Rect<u32> = Rect::new(0.0, 0.0, resolution.width, resolution.height).lossy_convert();
    let scale = UIManager::scale() * base.tree_scale;
    let frame = base.draw_frame();

    if view.as_any().downcast_ref::<DrawingView>().is_some() {
        return whole(frame * scale, EDGE).unwrap_or(window);
    }
    paint_rect(view, &frame, scale, resolution, window)
}

fn overlap(a: Rect<u32>, b: Rect<u32>) -> bool {
    a.x() < b.x() + b.width()
        && b.x() < a.x() + a.width()
        && a.y() < b.y() + b.height()
        && b.y() < a.y() + a.height()
}
