use std::ops::{Deref, DerefMut};

use refs::{Weak, main_lock::MainLock};
use wgpu::RenderPass;
use wgpu_text::{Section, Text, TextBuilder, glyph_brush::HorizontalAlign};

use crate::{
    gm::{
        LossyConvert,
        color::{CLEAR, TURQUOISE},
        flat::{CornerRadii, Rect, Size},
    },
    pipelines::Pipelines,
    render::{
        UIBackdropPipeline, UIBlurPipeline, UIGradientPipeline, UIImageRectPipeline, UIPathPipeline,
        UIRectPipeline, UIShadowPipeline,
        data::{PathData, RectView, UIImageInstance, UIRectInstance, UIShadowInstance},
    },
    ui::{
        BlurView, DrawingView, ImageView, Label, ScrimView, TextAlignment, UIManager, View, ViewData,
        ViewFrame, ViewLayout, ViewSubviews,
    },
    window::{Font, RenderFrame, ShapedParams},
};

static GRADIENT_DRAWER: MainLock<UIGradientPipeline> = MainLock::new();
static IMAGE_RECT_DRAWER: MainLock<UIImageRectPipeline> = MainLock::new();
static SHADOW_DRAWER: MainLock<UIShadowPipeline> = MainLock::new();
static SCRIM_DRAWER: MainLock<UIRectPipeline> = MainLock::new();
static BLUR_DRAWER: MainLock<UIBlurPipeline> = MainLock::new();
static BACKDROP_DRAWER: MainLock<UIBackdropPipeline> = MainLock::new();
static PATH_DRAWER: MainLock<UIPathPipeline> = MainLock::new();

/// Set during update when a visible `BlurView` wants a blur, read by
/// the window before it picks the frame's render target.
static NEEDS_SAMPLING: MainLock<bool> = MainLock::new();

type TextSections<'a> = Vec<(Weak<Font>, Vec<(Section<'a>, ShapedParams)>)>;

struct DrawContext<'a> {
    text_sections: TextSections<'a>,
    /// Paths collected since the last flush, drawn under the scissor
    /// that was current when their view was visited.
    paths:         Vec<&'a PathData>,
    debug_frames:  bool,
    scale:         f32,
    resolution:    Size,
    /// What the current pass has set, reapplied after a blur barrier
    /// reopens the pass.
    scissor:       Rect<u32>,
}

pub struct UIDrawer;

impl UIDrawer {
    pub(crate) fn update() {
        UIManager::commit_animations();
        *NEEDS_SAMPLING.get_mut() = false;
        Self::update_view(UIManager::root_view().deref_mut());
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
            paths: vec![],
            debug_frames: UIManager::should_draw_debug_frames(),
            scale: UIManager::scale(),
            resolution,
            scissor: display_rect,
        };

        let root = UIManager::root_view_static();
        Self::precache_text(root, ctx.scale);
        Self::draw_view(render_frame, root, &mut ctx);

        Self::flush_pipelines(render_frame.pass(), resolution, &mut ctx.paths);
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

    fn precache_text(view: &dyn View, scale: f32) {
        let mut text_sections = vec![];
        Self::collect_text(view, scale, &mut text_sections);

        for (mut font, sections) in text_sections {
            for (section, params) in sections {
                font.queue_shaped(section, params);
            }
            font.process_queued().unwrap();
        }
    }

    fn collect_text<'a>(view: &'a dyn View, scale: f32, sections: &mut TextSections<'a>) {
        let frame = *view.absolute_frame();

        if view.is_hidden() || frame.size.has_no_area() {
            return;
        }

        if let Some(label) = view.as_any().downcast_ref::<Label>()
            && !label.text.is_empty()
        {
            Self::draw_label(&frame, label, sections, scale);
        }

        let root_frame = UIManager::root_view_static().frame();

        for subview in view.subviews() {
            if subview.dont_hide() || subview.absolute_frame().intersects(root_frame) {
                Self::collect_text(subview.deref(), scale, sections);
            }
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

        BACKDROP_DRAWER.get_mut().draw(
            pass,
            RectView {
                resolution: ctx.resolution,
                _padding:   0,
            },
            UIRectInstance::new(
                *frame,
                *view.color(),
                *view.border_color(),
                view.border_width(),
                view.corner_radii(),
                view.z_position(),
                ctx.scale,
            ),
            BLUR_DRAWER.output_bind(),
        );
    }

    fn update_view(view: &mut dyn View) {
        if view.is_hidden() {
            return;
        }
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
            Self::update_view(child.deref_mut());
            i += 1;
        }
    }

    #[allow(clippy::too_many_lines)]
    fn draw_view<'a>(render_frame: &mut RenderFrame, view: &'a dyn View, ctx: &mut DrawContext<'a>) {
        let frame = *view.absolute_frame();

        if view.is_hidden() || frame.size.has_no_area() {
            return;
        }

        view.before_render(render_frame.pass());

        let clips = view.__internal_clips_to_bounds();
        let parent_scissor = ctx.scissor;

        if clips {
            // Text is deferred, so everything queued outside this clip
            // flushes now under the parent scissor. The subtree's text
            // then flushes under this clip before it is restored.
            Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
            Self::flush_text(render_frame.pass(), &mut ctx.text_sections);
            let mut frame = frame * ctx.scale;
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
            let clip_rect = clip_rect.intersection(&parent_scissor);
            scissor(render_frame.pass(), clip_rect);
            ctx.scissor = clip_rect;
        }

        if let Some(shadow) = view.shadow()
            && shadow.radius > 0.0
            && shadow.color.a > 0.0
        {
            SHADOW_DRAWER.get_mut().add(UIShadowInstance {
                position:     frame.origin + shadow.offset,
                size:         frame.size,
                color:        shadow.color,
                corner_radii: view.corner_radii(),
                blur:         shadow.radius,
                z_position:   view.z_position(),
                scale:        ctx.scale,
                padding:      0.0,
            });
        }

        if let Some(blur) = view.as_any().downcast_ref::<BlurView>()
            && blur.blur_radius() > 0.0
        {
            Self::blur_barrier(render_frame, blur, &frame, ctx);
        } else if view.as_any().downcast_ref::<ScrimView>().is_some() {
            if view.color().a > 0.0 {
                SCRIM_DRAWER.get_mut().add(UIRectInstance::new(
                    frame,
                    *view.color(),
                    *view.border_color(),
                    view.border_width(),
                    view.corner_radii(),
                    view.z_position(),
                    ctx.scale,
                ));
            }
        } else if let Some(gradient) = view.gradient() {
            GRADIENT_DRAWER.get_mut().add(gradient.instance(
                frame,
                view.corner_radii(),
                view.z_position(),
                ctx.scale,
            ));
        } else if view.color().a > 0.0 || view.border_color().a > 0.0 {
            Pipelines::rect().add(UIRectInstance::new(
                frame,
                *view.color(),
                *view.border_color(),
                view.border_width(),
                view.corner_radii(),
                view.z_position(),
                ctx.scale,
            ));
        }

        if let Some(image_view) = view.as_any().downcast_ref::<ImageView>() {
            if image_view.image().is_ok() {
                let image = image_view.image();

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
                    ),
                    image,
                );
            }
        } else if let Some(label) = view.as_any().downcast_ref::<Label>()
            && !label.text.is_empty()
        {
            Self::draw_label(&frame, label, &mut ctx.text_sections, ctx.scale);
        } else if let Some(drawing) = view.as_any().downcast_ref::<DrawingView>() {
            ctx.paths.extend(drawing.paths());
        }

        if ctx.debug_frames {
            for rect in frame.to_borders(2.0) {
                Pipelines::rect().add(UIRectInstance::new(
                    rect,
                    TURQUOISE,
                    CLEAR,
                    0.0,
                    CornerRadii::default(),
                    view.z_position() - 0.2,
                    ctx.scale,
                ));
            }
        }

        let root_frame = UIManager::root_view_static().frame();

        for view in view.subviews() {
            if view.dont_hide() || view.absolute_frame().intersects(root_frame) {
                Self::draw_view(render_frame, view.deref(), ctx);
            }
        }

        if clips {
            Self::flush_pipelines(render_frame.pass(), ctx.resolution, &mut ctx.paths);
            Self::flush_text(render_frame.pass(), &mut ctx.text_sections);
            scissor(render_frame.pass(), parent_scissor);
            ctx.scissor = parent_scissor;
        }
    }

    fn draw_label<'a>(frame: &Rect, label: &'a Label, sections: &mut TextSections<'a>, scale: f32) {
        let frame = frame * scale;

        let center = frame.center();

        let margin = 16.0;

        let font = label.font();

        let params = ShapedParams {
            tracking:  label.letter_spacing() * scale,
            multiline: label.is_multiline(),
            h_align:   match label.alignment {
                TextAlignment::Left => HorizontalAlign::Left,
                TextAlignment::Center => HorizontalAlign::Center,
                TextAlignment::Right => HorizontalAlign::Right,
            },
        };

        let mut text = Text::new(&label.text)
            .with_scale(label.text_size() * scale * font.em_scale())
            .with_color(label.text_color().as_slice())
            .with_z(label.z_position() - UIManager::additional_z_offset());

        // After `with_color`, which sets both ends of the ramp so that a label
        // without a gradient stays flat.
        if let Some(end) = label.text_end_color() {
            text = text.with_end_color(end.as_slice());
        }

        let section = Section::new()
            .add_text(text)
            .with_bounds((
                frame.width() - if label.alignment.center() { 0.0 } else { margin },
                frame.height(),
            ))
            .with_screen_position((
                match label.alignment {
                    TextAlignment::Left => frame.x() + margin,
                    TextAlignment::Center => center.x,
                    TextAlignment::Right => frame.max_x() - margin,
                },
                center.y,
            ));

        match sections.iter_mut().find(|(f, _)| f.name == font.name) {
            Some((_, list)) => list.push((section, params)),
            None => sections.push((font, vec![(section, params)])),
        }
    }
}

fn scissor(pass: &mut RenderPass, rect: Rect<u32>) {
    pass.set_scissor_rect(rect.x(), rect.y(), rect.width(), rect.height());
}

/// Map the clip space of what comes next onto `area` at the frame origin,
/// instead of onto the whole frame. A game or a level fills the root view, and
/// a UI test pins the root to a canvas smaller than the window. Without this
/// the scene would stretch across the whole frame and land on different pixels
/// on every screen. Reset it with the full window once the scene is drawn.
pub(crate) fn set_viewport(pass: &mut RenderPass, area: Size) {
    pass.set_viewport(0.0, 0.0, area.width, area.height, 0.0, 1.0);
}
