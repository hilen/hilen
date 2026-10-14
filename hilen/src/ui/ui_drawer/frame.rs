//! One frame of the view tree: the whole window, or only the regions that
//! changed since the frame before, see `ui::damage`.

use super::{BLUR_DRAWER, DrawContext, UIDrawer, scissor};
use crate::{
    gm::{
        LossyConvert,
        color::CLEAR,
        flat::{CornerRadii, Rect, Size},
    },
    pipelines::Pipelines,
    render::data::UIRectInstance,
    ui::{
        Label, UIManager, View, ViewFrame, damage,
        view::{RenderProbe, renders_itself},
    },
    window::{RenderFrame, image::Svg},
};

/// Farther than every view, the root sits at `UIManager::ROOT_VIEW_Z_OFFSET`.
const FILL_Z: f32 = 0.999;

/// How far the fill of a region reaches past it, in pixels. The edge of a
/// rect is soft, and the scissor cuts the soft part away.
const FILL_OVER: f32 = 4.0;

impl UIDrawer {
    pub fn draw(render_frame: &mut RenderFrame) {
        let resolution = UIManager::window_resolution();
        let display_rect: Rect<u32> = Size::<u32>::new(
            resolution.width.lossy_convert(),
            resolution.height.lossy_convert(),
        )
        .into();

        let plan = damage::plan();

        let root = UIManager::root_view_static();

        match &plan.regions {
            None => {
                Self::draw_region(render_frame, root, display_rect, resolution, false);
                Self::drop_stale_text_effects();
            }
            // A text effect counts as used in the frames that draw it. A
            // partial frame draws few of the ones on the screen, so none
            // is dropped in it.
            Some(regions) => {
                for region in regions {
                    Self::fill_region(*region, plan.fill);
                    Self::draw_region(render_frame, root, *region, resolution, true);
                }
            }
        }
        // The walk of `ui::damage` marks every svg on the screen in every
        // frame, so this holds for a partial frame too.
        Svg::drop_stale_everywhere();

        // When the frame rendered into the intermediate scene texture,
        // copy the finished scene to the real surface.
        let scene = render_frame.scene_view().clone();
        if let Some(pass) = render_frame.present_pass() {
            BLUR_DRAWER.get_mut().present(pass, &scene);
        }
    }

    /// Runs the `before_render` of a view and keeps whether the view has
    /// one of its own, see `RenderProbe`.
    pub(super) fn before_render(render_frame: &mut RenderFrame, view: &dyn View) {
        let pass = render_frame.pass();
        view.__base_view().render_probe = if renders_itself(|| view.before_render(pass)) {
            RenderProbe::Custom
        } else {
            RenderProbe::Plain
        };
    }

    /// A view with no pixel in the region of a partial frame queues
    /// nothing. What it would queue is noted, so the flushes of the views
    /// after it come where a whole frame has them.
    pub(super) fn note_unseen(
        render_frame: &mut RenderFrame,
        view: &dyn View,
        frame: &Rect,
        ctx: &mut DrawContext,
        opacity: f32,
    ) {
        Self::flush_before_rect(render_frame, view, frame, ctx, opacity);
        if let Some(label) = view.as_any().downcast_ref::<Label>()
            && !label.text.is_empty()
        {
            ctx.note_text(label.z_position() - UIManager::additional_z_offset());
            ctx.unseen_text = true;
        }
    }

    /// Draws 1 view and its subviews alone, at their place in the window,
    /// cut at `region`. A picture of a loop for a layer of the system, see
    /// `ui::loops`. The frame starts clear, so what is behind the view is
    /// not in it.
    #[cfg(apple)]
    pub(crate) fn draw_subtree(render_frame: &mut RenderFrame, view: &dyn View, region: Rect<u32>) {
        Self::draw_region(render_frame, view, region, UIManager::window_resolution(), false);
    }

    /// Draws every view under `root` that has pixels in `region`, cut at
    /// its edges.
    fn draw_region(
        render_frame: &mut RenderFrame,
        root: &dyn View,
        region: Rect<u32>,
        resolution: Size,
        partial: bool,
    ) {
        let mut ctx = DrawContext {
            text_sections: vec![],
            nearest_text: 0.0,
            paths: vec![],
            pictures: vec![],
            debug_frames: UIManager::should_draw_debug_frames(),
            scale: UIManager::scale(),
            pixel_scale: UIManager::scale(),
            resolution,
            scissor: region,
            clip_depth: 0,
            group_level: 0,
            partial,
            unseen_text: false,
        };

        scissor(render_frame.pass(), region);

        Self::draw_view(render_frame, root, &mut ctx);

        Self::flush_pipelines(render_frame.pass(), resolution, &mut ctx.paths);
        scissor(render_frame.pass(), region);

        Self::flush_queued_text(render_frame, &mut ctx);
    }

    /// A region starts from the clear color like a whole frame does. The
    /// pass keeps the color of the frame before, so the old pixels of the
    /// region are painted over first, behind everything.
    fn fill_region(region: Rect<u32>, color: crate::gm::color::Color) {
        let scale = UIManager::scale();
        let x: f32 = region.x().lossy_convert();
        let y: f32 = region.y().lossy_convert();
        let width: f32 = region.width().lossy_convert();
        let height: f32 = region.height().lossy_convert();

        Pipelines::rect().add(UIRectInstance::new(
            Rect::new(
                (x - FILL_OVER) / scale,
                (y - FILL_OVER) / scale,
                (width + FILL_OVER * 2.0) / scale,
                (height + FILL_OVER * 2.0) / scale,
            ),
            color,
            CLEAR,
            0.0,
            CornerRadii::default(),
            FILL_Z,
            scale,
        ));
    }
}
