//! One pass over the views a frame draws, to learn which pixels changed
//! since the frame before. It visits the same views in the same order as
//! `UIDrawer::draw_view` and queues nothing.

use std::{ops::Deref, sync::atomic::Ordering};

use rustc_hash::FxHashMap;

use crate::{
    gm::flat::{Rect, Size},
    ui::{
        BlurView, DrawingView, ImageView, Label, ScrimView, TextSelection, UIManager, View, ViewData,
        ViewFrame, ViewSubviews,
        damage::{clip_rect, key::PaintKey, paint_rect, pixels, regions::Regions},
        view::RenderProbe,
    },
    window::Window,
};

/// What a view put on the screen in a frame.
pub(super) struct Paint {
    key:  u64,
    /// The pixels the view can touch, cut at every clip it is inside.
    rect: Rect<u32>,
}

/// By the address of the view. A freed view and a new one at its address
/// differ in their key, or draw the same pixels.
pub(super) type Paints = FxHashMap<usize, Paint>;

pub(super) struct Walk<'a> {
    last:        &'a Paints,
    seen:        &'a mut Paints,
    regions:     Regions,
    /// Why the whole frame is drawn, the first reason found.
    whole:       Option<&'static str>,
    pixel_scale: f32,
    resolution:  Size,
    scissor:     Rect<u32>,
    root_frame:  Rect,
    /// Inside a view that asked for every frame, see
    /// `ViewData::keep_frames_while`. Its pixels change with no change of
    /// what the views hold, a video draws a new picture into its image.
    volatile:    bool,
    frame:       u64,
}

impl<'a> Walk<'a> {
    /// The regions that changed against `last`, and why the frame is drawn
    /// whole when it is. `seen` gets the paint of every view of this frame.
    pub(super) fn run(last: &'a Paints, seen: &'a mut Paints) -> (Regions, Option<&'static str>) {
        let resolution = UIManager::window_resolution();
        let root = UIManager::root_view_static();

        let mut walk = Self {
            last,
            seen,
            regions: Regions::default(),
            whole: UIManager::should_draw_debug_frames().then_some("debug frames"),
            pixel_scale: UIManager::scale(),
            resolution,
            scissor: Rect::new(0, 0, pixels(resolution.width), pixels(resolution.height)),
            root_frame: *root.frame(),
            volatile: false,
            frame: Window::render_frame(),
        };

        walk.visit(root);

        // A view that is gone leaves its pixels behind.
        for (id, paint) in walk.last {
            if !walk.seen.contains_key(id) {
                walk.regions.add(paint.rect);
            }
        }

        (walk.regions, walk.whole)
    }

    fn whole(&mut self, why: &'static str) {
        self.whole.get_or_insert(why);
    }

    fn visit(&mut self, view: &dyn View) {
        let base = view.__base_view();
        let frame = base.draw_frame();
        let scale = self.pixel_scale * base.tree_scale;
        let opacity = base.tree_opacity;

        // A view a layer of the system shows is not in the frame, its old
        // pixels go like the ones of a view that is gone.
        if view.is_hidden()
            || frame.size.has_no_area()
            || opacity <= 0.0
            || base.group_alpha <= 0.0
            || base.loop_layer
        {
            return;
        }

        if base.group_alpha < 1.0 {
            self.whole("a group fade");
        }
        if base.page_hole {
            self.whole("a hole in the page");
        }
        match base.render_probe {
            RenderProbe::Plain => {}
            RenderProbe::Unknown => self.whole("a view that was never drawn"),
            RenderProbe::Custom => self.whole("a view that renders itself"),
        }
        if view
            .as_any()
            .downcast_ref::<BlurView>()
            .is_some_and(|blur| blur.blur_radius() > 0.0)
        {
            self.whole("a blur");
        }
        if view.as_any().downcast_ref::<ScrimView>().is_some() {
            self.whole("a scrim");
        }

        let outer_scissor = self.scissor;
        let outer_volatile = self.volatile;
        let clips = view.__internal_clips_to_bounds();
        if clips {
            self.scissor = clip_rect(&frame, scale, self.resolution, outer_scissor);
        }
        self.volatile |= base.keeps_frames;

        let paint = Paint {
            key:  self.key(view, &frame, scale, opacity, clips),
            rect: paint_rect(view, &frame, scale, self.resolution, self.scissor),
        };
        let id = std::ptr::from_ref(view).addr();

        match self.last.get(&id) {
            Some(old) if old.key == paint.key && old.rect == paint.rect => {}
            Some(old) => {
                self.regions.add(old.rect);
                self.regions.add(paint.rect);
            }
            None => self.regions.add(paint.rect),
        }
        self.seen.insert(id, paint);

        for child in view.subviews() {
            if child.dont_hide() || child.absolute_frame().intersects(&self.root_frame) {
                self.visit(child.deref());
            }
        }

        self.scissor = outer_scissor;
        self.volatile = outer_volatile;
    }

    /// Everything `UIDrawer::draw_view` reads to draw the view itself.
    /// A value it reads and this leaves out stays on the screen as it
    /// was, so a new input of the drawer is added here in the same change.
    fn key(&self, view: &dyn View, frame: &Rect, scale: f32, opacity: f32, clips: bool) -> u64 {
        let mut key = PaintKey::default();
        let base = view.__base_view();

        key.pod(frame)
            .pod(&scale)
            .pod(&opacity)
            .pod(view.color())
            .pod(view.border_color())
            .pod(&view.border_width())
            .pod(&view.corner_radii())
            .pod(&view.z_position())
            .pod(&self.scissor)
            .hash(&clips)
            .hash(&base.flat_depth);

        if let Some(gradient) = view.gradient() {
            key.debug(&gradient);
        }
        if let Some(shadow) = view.shadow() {
            key.debug(&shadow);
        }
        if self.volatile {
            key.hash(&self.frame);
        }

        if let Some(label) = view.as_any().downcast_ref::<Label>() {
            label.feed_paint(&mut key);
            if label.is_selectable()
                && let Some(highlight) = TextSelection::highlight(label)
            {
                key.hash(&highlight.start).hash(&highlight.end).hash(&highlight.goes_on);
            }
        } else if let Some(image_view) = view.as_any().downcast_ref::<ImageView>() {
            Self::feed_image(&mut key, image_view, scale);
        } else if let Some(drawing) = view.as_any().downcast_ref::<DrawingView>() {
            for path in drawing.paths() {
                path.feed_paint(&mut key);
            }
        }

        key.finish()
    }

    fn feed_image(key: &mut PaintKey, view: &ImageView, scale: f32) {
        let image = view.image();
        if !image.is_ok() {
            return;
        }
        // An image and the raster of an svg are freed when no frame drew
        // them for a while. A partial frame draws few of the images on
        // the screen, so all of them are marked as drawn here.
        let frame = Window::render_frame();
        image.drawn_at.store(frame, Ordering::Relaxed);
        if let Some(svg) = &image.svg {
            svg.touch(view.svg_raster_size(scale), frame);
        }

        key.hash(&image.addr())
            .pod(&image.size)
            .pod(&view.image_frame())
            .pod(&view.uv_rect())
            .hash(&view.flip_x)
            .hash(&view.flip_y)
            .debug(&view.mode);
        if let Some(cut) = view.cut() {
            key.debug(&cut);
        }
    }
}
