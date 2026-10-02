use std::{
    ops::{Deref, DerefMut},
    sync::atomic::Ordering,
};

use wgpu::RenderPass;

use crate::{
    deps::refs::main_lock::MainLock,
    gm::{
        LossyConvert,
        color::{CLEAR, TURQUOISE},
        flat::{CornerRadii, Rect, Size},
    },
    pipelines::Pipelines,
    render::{
        ImageKey, UIBackdropPipeline, UIBlurPipeline, UIClipPipeline, UIGradientPipeline, UIGroupPipeline,
        UIImageRectPipeline, UIPathPipeline, UIRectPipeline, UIShadowPipeline,
        data::{PathData, RectView, UIImageInstance, UIRectInstance, UIShadowInstance},
    },
    ui::{
        BlurView, DrawingView, ImageView, Label, ScrimView, UIManager, View, ViewData, ViewFrame, ViewLayout,
        ViewSubviews, label_drawer::TextSections,
    },
    window::{RenderFrame, Window, image::Svg},
};

static GRADIENT_DRAWER: MainLock<UIGradientPipeline> = MainLock::new();
pub(super) static IMAGE_RECT_DRAWER: MainLock<UIImageRectPipeline> = MainLock::new();
static SHADOW_DRAWER: MainLock<UIShadowPipeline> = MainLock::new();
static SCRIM_DRAWER: MainLock<UIRectPipeline> = MainLock::new();
static BLUR_DRAWER: MainLock<UIBlurPipeline> = MainLock::new();
static BACKDROP_DRAWER: MainLock<UIBackdropPipeline> = MainLock::new();
static PATH_DRAWER: MainLock<UIPathPipeline> = MainLock::new();
static CLIP_DRAWER: MainLock<UIClipPipeline> = MainLock::new();
static GROUP_DRAWER: MainLock<UIGroupPipeline> = MainLock::new();

/// Set during update when a visible `BlurView` wants a blur, read by
/// the window before it picks the frame's render target.
static NEEDS_SAMPLING: MainLock<bool> = MainLock::new();

struct DrawContext<'a> {
    text_sections: TextSections<'a>,
    /// The z of the nearest text queued since the last flush, smaller
    /// is nearer. Only valid while `text_sections` is not empty.
    nearest_text:  f32,
    /// Paths collected since the last flush, drawn under the scissor
    /// that was current when their view was visited.
    paths:         Vec<&'a PathData>,
    debug_frames:  bool,
    scale:         f32,
    resolution:    Size,
    /// What the current pass has set, reapplied after a blur barrier
    /// reopens the pass.
    scissor:       Rect<u32>,
    /// How many rounded clips the visited view is inside, the stencil
    /// reference of the pass. Reapplied with the scissor.
    clip_depth:    u32,
    /// How many group fades the visited view is inside, the index of the
    /// image the next group draws into.
    group_level:   usize,
}

pub struct UIDrawer;

impl UIDrawer {
    pub(crate) fn update() {
        UIManager::commit_animations();
        *NEEDS_SAMPLING.get_mut() = false;
        Self::update_view(UIManager::root_view().deref_mut(), 1.0);
    }

    pub(crate) fn needs_sampleable_frame() -> bool {
        *NEEDS_SAMPLING
    }

    pub fn draw(render_frame: &mut RenderFrame) {
        let resolution = UIManager::window_resolution();
        let display_rect: Rect<u32> = Size::<u32>::new(
            resolution.width.lossy_convert(),
            resolution.height.lossy_convert(),
        )
        .into();

        let mut ctx = DrawContext {
            text_sections: vec![],
            nearest_text: 0.0,
            paths: vec![],
            debug_frames: UIManager::should_draw_debug_frames(),
            scale: UIManager::scale(),
            resolution,
            scissor: display_rect,
            clip_depth: 0,
            group_level: 0,
        };

        let root = UIManager::root_view_static();
        Self::draw_view(render_frame, root, &mut ctx);

        Self::flush_pipelines(render_frame.pass(), resolution, &mut ctx.paths);
        Svg::drop_stale_everywhere();
        Self::drop_stale_text_effects();
        scissor(render_frame.pass(), display_rect);

        Self::flush_text(render_frame.pass(), &mut ctx.text_sections);

        // The scrim flushes after everything including text, so its
        // translucent color dims the whole frame drawn so far. The
        // modal above it owns the depth buffer and stays untouched.
        SCRIM_DRAWER.get_mut().draw(
            render_frame.pass(),
            RectView {
                resolution,
                _padding: 0,
            },
        );

        // When the frame rendered into the intermediate scene texture,
        // copy the finished scene to the real surface.
        let scene = render_frame.scene_view().clone();
        if let Some(pass) = render_frame.present_pass() {
            BLUR_DRAWER.get_mut().present(pass, &scene);
        }
    }

    fn flush_pipelines(pass: &mut RenderPass, resolution: Size, paths: &mut Vec<&PathData>) {
        let rect_view = RectView {
            resolution,
            _padding: 0,
        };

        Pipelines::rect().draw(pass, rect_view);
        IMAGE_RECT_DRAWER.get_mut().draw(pass, rect_view);
        GRADIENT_DRAWER.get_mut().draw(pass, rect_view);

        let path_drawer = PATH_DRAWER.get_mut();
        for path in paths.drain(..) {
            if path.visible() {
                path_drawer.draw(pass, path);
            }
        }

        // Shadows go last. A shadow shares its view's z, so the view
        // drawn above already owns the depth buffer inside its shape
        // and masks the shadow there. Everything farther is already
        // drawn, so the soft band blends over it.
        SHADOW_DRAWER.get_mut().draw(pass, rect_view);
    }

    fn flush_text(pass: &mut RenderPass, text_sections: &mut TextSections) {
        for (mut font, sections) in text_sections.drain(..) {
            for (section, params) in sections {
                font.queue_shaped(section, params);
            }
            font.process_queued().unwrap();
            font.brush.draw(pass);
        }
    }

    /// Everything drawn so far flushes into the scene texture and gets
    /// blurred, then the pass reopens and the blurred backdrop draws
    /// at the view's frame. Subviews and later views draw on top.
    fn blur_barrier(render_frame: &mut RenderFrame, view: &BlurView, frame: &Rect, ctx: &mut DrawContext) {
        Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
        Self::flush_text(render_frame.pass(), &mut ctx.text_sections);

        let (encoder, scene) = render_frame.split();
        BLUR_DRAWER.get_mut().blur(
            encoder,
            scene,
            ctx.resolution.lossy_convert(),
            view.blur_radius() * ctx.scale,
        );

        let pass = render_frame.pass();
        scissor(pass, ctx.scissor);
        pass.set_stencil_reference(ctx.clip_depth);

        BACKDROP_DRAWER.get_mut().draw(
            pass,
            RectView {
                resolution: ctx.resolution,
                _padding:   0,
            },
            UIRectInstance::new(
                *frame,
                view.color().faded(view.__base_view().tree_opacity),
                view.border_color().faded(view.__base_view().tree_opacity),
                view.border_width(),
                view.corner_radii(),
                view.z_position(),
                ctx.scale,
            ),
            BLUR_DRAWER.output_bind(),
        );
    }

    fn update_view(view: &mut dyn View, parent_opacity: f32) {
        if view.is_hidden() {
            return;
        }
        let opacity = parent_opacity * view.opacity();
        // A group draws itself and its subviews at full strength, its
        // picture is what fades.
        let base = view.__base_view();
        let grouped = base.group_opacity && opacity < 1.0;
        base.group_alpha = if grouped { opacity } else { 1.0 };
        let opacity = if grouped { 1.0 } else { opacity };
        base.tree_opacity = opacity;
        view.layout();
        view.calculate_absolute_frame();
        view.update();
        view.trigger_events();

        if let Some(blur) = view.as_any().downcast_ref::<BlurView>()
            && blur.blur_radius() > 0.0
        {
            *NEEDS_SAMPLING.get_mut() = true;
        }

        // A child's update() may add views to this list, reallocating it.
        // Indexing re-borrows the list on every step, so only the Weak is
        // held while child code runs. An iterator would dangle.
        let mut i = 0;
        while i < view.subviews().len() {
            let mut child = view.subviews()[i].weak();
            Self::update_view(child.deref_mut(), opacity);
            i += 1;
        }
    }

    fn draw_view<'a>(render_frame: &mut RenderFrame, view: &'a dyn View, ctx: &mut DrawContext<'a>) {
        let frame = *view.absolute_frame();

        // A view faded to nothing draws nothing, and neither does anything
        // inside it.
        let opacity = view.__base_view().tree_opacity;
        let group_alpha = view.__base_view().group_alpha;

        if view.is_hidden() || frame.size.has_no_area() || opacity <= 0.0 || group_alpha <= 0.0 {
            return;
        }

        let grouped = group_alpha < 1.0;
        let outer_clip_depth = ctx.clip_depth;
        if grouped {
            Self::enter_group(render_frame, ctx);
        }

        view.before_render(render_frame.pass());

        let clips = view.__internal_clips_to_bounds();
        let parent_scissor = ctx.scissor;

        if clips {
            Self::enter_scissor(render_frame, &frame, ctx);
        }

        if let Some(blur) = view.as_any().downcast_ref::<BlurView>()
            && blur.blur_radius() > 0.0
        {
            Self::draw_shadow(view, &frame, ctx.scale, opacity);
            Self::blur_barrier(render_frame, blur, &frame, ctx);
        } else {
            // Rects flush before text and write depth, so text queued
            // behind a translucent rect would fail the depth test and
            // vanish instead of showing through. Flush it first, but
            // only when the rect is in front of all queued text. Text
            // writes depth over whole glyph boxes, so a rect behind it,
            // like a text selection, would get holes around the glyphs.
            if is_translucent(view, opacity)
                && !ctx.text_sections.is_empty()
                && view.z_position() < ctx.nearest_text
            {
                Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
                Self::flush_text(render_frame.pass(), &mut ctx.text_sections);
            }
            // Queued after the flush above, never before it. A shadow
            // shares its view's z and the first draw at a z wins, so a
            // shadow flushed ahead of its view would cover it.
            Self::draw_shadow(view, &frame, ctx.scale, opacity);
            Self::draw_background(view, &frame, ctx.scale, opacity);
        }

        Self::draw_content(view, &frame, ctx, opacity);

        if ctx.debug_frames {
            Self::draw_debug_frame(view, &frame, ctx.scale);
        }

        // A scissor is a rectangle, so a clipping view with rounded
        // corners also masks its subtree to its outline through the
        // stencil. The view's own shape is flushed first, under the
        // mask of whatever clip it is inside, so only its subtree is
        // cut at the corners.
        let rounded_clip = clips && view.corner_radii() != CornerRadii::default();
        let clip_shape = Self::clip_shape(view, &frame, ctx.scale);

        if rounded_clip {
            Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
            Self::flush_text(render_frame.pass(), &mut ctx.text_sections);
            let pass = render_frame.pass();
            CLIP_DRAWER.get_mut().enter(pass, ctx.resolution, clip_shape);
            ctx.clip_depth += 1;
            pass.set_stencil_reference(ctx.clip_depth);
        }

        let root_frame = UIManager::root_view_static().frame();

        for view in view.subviews() {
            if view.dont_hide() || view.absolute_frame().intersects(root_frame) {
                Self::draw_view(render_frame, view.deref(), ctx);
            }
        }

        if rounded_clip {
            Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
            Self::flush_text(render_frame.pass(), &mut ctx.text_sections);
            let pass = render_frame.pass();
            CLIP_DRAWER.get_mut().leave(pass, ctx.resolution, clip_shape);
            ctx.clip_depth -= 1;
            pass.set_stencil_reference(ctx.clip_depth);
        }

        if clips {
            Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
            Self::flush_text(render_frame.pass(), &mut ctx.text_sections);
            scissor(render_frame.pass(), parent_scissor);
            ctx.scissor = parent_scissor;
        }

        if grouped {
            Self::leave_group(render_frame, ctx, outer_clip_depth, group_alpha);
        }
    }

    /// Everything drawn so far flushes into the pass it belongs to, then
    /// the passes draw into the image of the group. That image starts
    /// empty with a stencil of its own, so no rounded clip is entered in
    /// it yet. The scissor stays, the image has the size of the frame.
    fn enter_group(render_frame: &mut RenderFrame, ctx: &mut DrawContext<'_>) {
        Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
        Self::flush_text(render_frame.pass(), &mut ctx.text_sections);

        let target = GROUP_DRAWER.get_mut().target(ctx.group_level, ctx.resolution.lossy_convert());
        render_frame.push_target(target);
        ctx.group_level += 1;
        ctx.clip_depth = 0;

        let pass = render_frame.pass();
        scissor(pass, ctx.scissor);
        pass.set_stencil_reference(0);
    }

    /// The views of the group flush into its image, then the image draws
    /// once, with the opacity, into the pass the group sits in.
    fn leave_group(render_frame: &mut RenderFrame, ctx: &mut DrawContext<'_>, clip_depth: u32, opacity: f32) {
        Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
        Self::flush_text(render_frame.pass(), &mut ctx.text_sections);

        render_frame.pop_target();
        ctx.group_level -= 1;
        ctx.clip_depth = clip_depth;

        let pass = render_frame.pass();
        scissor(pass, ctx.scissor);
        pass.set_stencil_reference(clip_depth);
        GROUP_DRAWER.get_mut().draw(pass, ctx.group_level, opacity);
    }

    /// Text is deferred, so everything queued outside this clip
    /// flushes now under the parent scissor. The subtree's text
    /// then flushes under this clip before it is restored.
    fn enter_scissor(render_frame: &mut RenderFrame, frame: &Rect, ctx: &mut DrawContext<'_>) {
        Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
        Self::flush_text(render_frame.pass(), &mut ctx.text_sections);
        let mut frame = *frame * ctx.scale;
        frame.origin.clip_positive();

        if frame.max_x() > ctx.resolution.width {
            frame.size.width -= frame.max_x() - ctx.resolution.width;
        }

        if frame.max_y() > ctx.resolution.height {
            frame.size.height -= frame.max_y() - ctx.resolution.height;
        }

        // A clip view fully past the right or bottom edge drives these
        // subtractions negative. Converting a negative Rect to Rect<u32>
        // panics, so clamp the clipped size to an empty rect instead.
        // max(0.0) also turns a NaN size into 0.
        frame.size.width = frame.size.width.max(0.0);
        frame.size.height = frame.size.height.max(0.0);

        let clip_rect: Rect<u32> = frame.lossy_convert();
        let clip_rect = clip_rect.intersection(&ctx.scissor);
        scissor(render_frame.pass(), clip_rect);
        ctx.scissor = clip_rect;
    }

    fn draw_shadow(view: &dyn View, frame: &Rect, scale: f32, opacity: f32) {
        if let Some(shadow) = view.shadow()
            && shadow.radius > 0.0
            && shadow.color.a > 0.0
        {
            SHADOW_DRAWER.get_mut().add(UIShadowInstance {
                position: frame.origin + shadow.offset,
                size: frame.size,
                color: shadow.color.faded(opacity),
                corner_radii: view.corner_radii(),
                blur: shadow.radius,
                z_position: view.z_position(),
                scale,
                padding: 0.0,
            });
        }
    }

    fn draw_background(view: &dyn View, frame: &Rect, scale: f32, opacity: f32) {
        if view.as_any().downcast_ref::<ScrimView>().is_some() {
            if view.color().a > 0.0 {
                SCRIM_DRAWER.get_mut().add(UIRectInstance::new(
                    *frame,
                    view.color().faded(opacity),
                    view.border_color().faded(opacity),
                    view.border_width(),
                    view.corner_radii(),
                    view.z_position(),
                    scale,
                ));
            }
        } else if let Some(gradient) = view.gradient() {
            GRADIENT_DRAWER.get_mut().add(
                gradient
                    .instance(
                        *frame,
                        view.corner_radii(),
                        *view.border_color(),
                        view.border_width(),
                        view.z_position(),
                        scale,
                    )
                    .faded(opacity),
            );
        } else if view.color().a > 0.0 || view.border_color().a > 0.0 {
            Pipelines::rect().add(UIRectInstance::new(
                *frame,
                view.color().faded(opacity),
                view.border_color().faded(opacity),
                view.border_width(),
                view.corner_radii(),
                view.z_position(),
                scale,
            ));
        }
    }

    fn draw_content<'a>(view: &'a dyn View, frame: &Rect, ctx: &mut DrawContext<'a>, opacity: f32) {
        if let Some(image_view) = view.as_any().downcast_ref::<ImageView>() {
            if image_view.image().is_ok() {
                let image = image_view.image();
                image.drawn_at.store(Window::render_frame(), Ordering::Relaxed);
                if image_view.cut().is_some() {
                    Self::draw_cut(image_view, ctx.scale, opacity);
                    return;
                }

                let raster = image.svg.as_ref().map(|svg| {
                    let size = image_view.raster_size(ctx.scale);
                    svg.touch(size, Window::render_frame());
                    (size.width, size.height)
                });

                IMAGE_RECT_DRAWER.get_mut().add_with_image(
                    UIImageInstance::new(
                        image_view.image_frame(),
                        image_view.uv_rect(),
                        *view.border_color(),
                        view.border_width(),
                        view.corner_radii(),
                        view.z_position(),
                        image_view.flip_x,
                        image_view.flip_y,
                        ctx.scale,
                    )
                    .with_opacity(opacity),
                    ImageKey { image, raster },
                );
            }
        } else if let Some(label) = view.as_any().downcast_ref::<Label>()
            && !label.text.is_empty()
        {
            let z = label.z_position() - UIManager::additional_z_offset();
            ctx.nearest_text = if ctx.text_sections.is_empty() {
                z
            } else {
                ctx.nearest_text.min(z)
            };
            Self::draw_label(frame, label, &mut ctx.text_sections, ctx.scale, opacity);
            Self::draw_text_effects(frame, label, ctx.scale, opacity);
            Self::draw_color_glyphs(frame, label, ctx.scale, opacity);
            Self::draw_underlines(frame, label, ctx.scale, opacity);
        } else if let Some(drawing) = view.as_any().downcast_ref::<DrawingView>() {
            ctx.paths.extend(drawing.paths());
        }
    }

    /// A cut image draws as up to 3 quads of the same texture, see
    /// `ImageView::set_cut`.
    fn draw_cut(image_view: &ImageView, scale: f32, opacity: f32) {
        let Some(cut) = image_view.cut() else {
            return;
        };
        let image = image_view.image();
        let image_size: Size = image.size.into();

        // An svg rasterizes whole at the scale of the view, the quads then
        // show parts of that raster.
        let raster = image.svg.as_ref().map(|svg| {
            let natural = cut.scaled_size(*image_view.absolute_frame(), image_size) * scale;
            let size = Size::new(
                natural.width.round().max(1.0).lossy_convert(),
                natural.height.round().max(1.0).lossy_convert(),
            );
            svg.touch(size, Window::render_frame());
            (size.width, size.height)
        });

        let drawer = IMAGE_RECT_DRAWER.get_mut();
        for (frame, uv) in cut.parts(*image_view.absolute_frame(), image_size, scale) {
            drawer.add_with_image(
                UIImageInstance::new(
                    frame,
                    uv,
                    CLEAR,
                    0.0,
                    CornerRadii::default(),
                    image_view.z_position(),
                    false,
                    false,
                    scale,
                )
                .with_opacity(opacity),
                ImageKey { image, raster },
            );
        }
    }

    fn draw_debug_frame(view: &dyn View, frame: &Rect, scale: f32) {
        for rect in frame.to_borders(2.0) {
            Pipelines::rect().add(UIRectInstance::new(
                rect,
                TURQUOISE,
                CLEAR,
                0.0,
                CornerRadii::default(),
                view.z_position() - 0.2,
                scale,
            ));
        }
    }

    /// Children are cut at the inside of the border, the CSS padding
    /// box, so a full height child never paints over it. The inner
    /// arc of a border is the outer radius minus the border width.
    fn clip_shape(view: &dyn View, frame: &Rect, scale: f32) -> UIRectInstance {
        let inset = if view.border_color().a > 0.0 {
            view.border_width()
        } else {
            0.0
        };
        let radii = view.corner_radii();
        UIRectInstance::new(
            Rect::new(
                frame.x() + inset,
                frame.y() + inset,
                frame.width() - inset * 2.0,
                frame.height() - inset * 2.0,
            ),
            CLEAR,
            CLEAR,
            0.0,
            CornerRadii {
                top_left:     (radii.top_left - inset).max(0.0),
                top_right:    (radii.top_right - inset).max(0.0),
                bottom_left:  (radii.bottom_left - inset).max(0.0),
                bottom_right: (radii.bottom_right - inset).max(0.0),
            },
            view.z_position(),
            scale,
        )
    }
}

/// A scrim is left out, it already flushes after all text. `opacity` is
/// the fade of the view's tree, it makes a solid color translucent too.
fn is_translucent(view: &dyn View, opacity: f32) -> bool {
    let partial = |alpha: f32| alpha > 0.0 && alpha * opacity < 1.0;
    view.as_any().downcast_ref::<ScrimView>().is_none()
        && (partial(view.color().a) || partial(view.border_color().a))
}

fn scissor(pass: &mut RenderPass, rect: Rect<u32>) {
    pass.set_scissor_rect(rect.x(), rect.y(), rect.width(), rect.height());
}

/// Map the clip space of what comes next onto `area` at the frame origin,
/// instead of onto the whole frame. A game or a level fills the root view, and
/// a UI test pins the root to a canvas smaller than the window. Without this
/// the scene would stretch across the whole frame and land on different pixels
/// on every screen. Reset it with the full window once the scene is drawn.
#[cfg(any(feature = "level", feature = "scene"))]
pub(crate) fn set_viewport(pass: &mut RenderPass, area: Size) {
    pass.set_viewport(0.0, 0.0, area.width, area.height, 0.0, 1.0);
}
