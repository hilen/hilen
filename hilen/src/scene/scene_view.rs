use std::sync::atomic::{AtomicU64, Ordering};

use crate::{
    self as hilen,
    deps::refs::{Weak, manage::DataManager},
    gm::{
        LossyConvert,
        color::Color,
        flat::{Point, Size},
    },
    scene::{Scene, SceneManager},
    ui::{
        ImageView, Pinch, Scrollable, Setup, Touch, TouchStack, UIManager, View, ViewData, ViewFrame,
        ViewTouch, keep_frames_exempt, view,
    },
    window::image::Image,
};

/// Radians of orbit, or of a player's look, per point of drag.
const ORBIT_SPEED: f32 = 0.008;
const LOOK_SPEED: f32 = 0.004;
/// The zoom per point of wheel, the feel of the demo page.
const WHEEL_ZOOM: f32 = 0.002;
/// A touch that moved less than this is a tap, not a drag.
const TAP_SLOP: f32 = 6.0;

/// Gives every view its own image name.
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// A press on the view, from its start to its end, in its own points.
#[derive(Clone, Copy, Debug)]
struct Press {
    start:    Point,
    last:     Point,
    /// Moved past the tap slop, from then on it turns the camera and
    /// can no longer end as a tap.
    dragging: bool,
    /// Began on the scene's handle, which gets the whole drag.
    handle:   bool,
}

/// A live scene inside the frame of a view. The scene draws into an image
/// of the view's size in pixels, with the camera aspect of that frame, and
/// the view shows the image like an `ImageView`. So other views sit next to
/// it and over it, and it clips and redraws like any view.
///
/// It runs the one scene of `SceneManager`, so `set_scene` here replaces a
/// scene that ran over the whole window and the other way round. The scene
/// stops when the view is gone.
///
/// A drag turns the camera around its target, or the head of the player
/// when the scene has one, the wheel and a pinch zoom, and a tap picks a
/// node like a touch on a scene over the whole window does.
/// `set_camera_controls(false)` leaves the drag and the zoom to the app.
#[view]
pub struct SceneView {
    id:         u64,
    background: Color,
    controls:   bool,
    press:      Option<Press>,
    target:     Weak<Image>,

    #[init]
    picture: ImageView,
}

impl SceneView {
    /// Runs `scene` in this view. Any scene that ran before, in a view or
    /// over the whole window, stops.
    pub fn set_scene<T: Scene + 'static>(self: Weak<Self>, scene: T) -> Weak<T> {
        let scene = SceneManager::set_scene_in(scene, self);
        keep_frames_exempt(self.weak_view(), move || self.hosts_scene());
        scene
    }

    /// Whether the running scene draws in this view.
    pub fn hosts_scene(&self) -> bool {
        SceneManager::host().is_some_and(|host| host.raw() == self.weak().raw())
    }

    /// What shows where the scene draws nothing, clear by default so the
    /// views behind show through. A sky or fog covers it.
    pub fn set_background(&mut self, color: impl Into<Color>) -> &mut Self {
        self.background = color.into();
        self
    }

    pub fn background(&self) -> Color {
        self.background
    }

    /// Whether a drag turns the camera and the wheel and a pinch zoom it,
    /// on by default. A tap picks a node either way.
    pub fn set_camera_controls(&mut self, on: bool) -> &mut Self {
        self.controls = on;
        self
    }

    /// The pixels the scene draws in, the frame of the view on the screen.
    pub(crate) fn pixel_area(&self) -> Size {
        self.absolute_frame().size * UIManager::scale()
    }

    /// The image of `pixel_area` the scene draws into this frame, made
    /// again when the size changed. None while the view shows nothing.
    pub(crate) fn frame_target(mut self: Weak<Self>) -> Option<Weak<Image>> {
        if self.is_hidden_in_tree() {
            return None;
        }
        let area = self.pixel_area();
        let size: Size<u32> = Size::new(
            area.width.round().lossy_convert(),
            area.height.round().lossy_convert(),
        );
        if size.width == 0 || size.height == 0 {
            return None;
        }
        if self.target.is_ok() && self.target.size == size {
            return Some(self.target);
        }
        if self.target.is_ok() {
            self.target.free();
        }
        let name = format!("scene-view-{}-{}x{}", self.id, size.width, size.height);
        self.target = Image::render_target(&name, size);
        self.picture.set_image(self.target);
        Some(self.target)
    }

    /// A point of this view in the pixels of the scene.
    fn pixel(&self, point: Point) -> Point {
        point * (UIManager::scale() * self.__base_view().tree_scale)
    }

    fn on_touch(mut self: Weak<Self>, touch: Touch) {
        if !self.hosts_scene() {
            self.press = None;
            return;
        }
        let pixel = self.pixel(touch.position);
        let mut scene = SceneManager::scene_weak();
        let (camera, area) = (scene.camera, SceneManager::area());
        if touch.is_began() {
            let handle = scene.handle.as_mut().is_some_and(|handle| handle.press(pixel, &camera, area));
            self.press = Some(Press {
                start: touch.position,
                last: touch.position,
                dragging: false,
                handle,
            });
            return;
        }
        let Some(mut press) = self.press else {
            return;
        };
        if press.handle {
            if let Some(handle) = scene.handle.as_mut() {
                if touch.is_ended() {
                    handle.release(pixel, &camera, area);
                } else {
                    handle.drag_to(pixel, &camera, area);
                }
            }
            if touch.is_ended() {
                self.press = None;
            }
            return;
        }
        if touch.is_ended() {
            self.press = None;
            if !press.dragging {
                scene.add_touch(pixel);
            }
            return;
        }
        // A click jitters a pixel or two, that must not turn the view.
        if !press.dragging {
            if (touch.position - press.start).length() < TAP_SLOP {
                return;
            }
            press.dragging = true;
            press.last = touch.position;
        }
        let delta = touch.position - press.last;
        press.last = touch.position;
        self.press = Some(press);
        if self.controls {
            turn(delta);
        }
    }

    fn zoom(&self, factor: f32) {
        if !self.controls || !self.hosts_scene() {
            return;
        }
        let mut scene = SceneManager::scene_weak();
        if scene.player.is_none() {
            scene.camera.zoom(factor);
        }
    }
}

/// A drag of `delta` points turns the camera around its target, or the
/// head of the player.
fn turn(delta: Point) {
    let mut scene = SceneManager::scene_weak();
    match scene.player.as_mut() {
        Some(player) => player.look(delta.x * LOOK_SPEED, -delta.y * LOOK_SPEED),
        None => scene.camera.orbit(-delta.x * ORBIT_SPEED, delta.y * ORBIT_SPEED),
    }
}

impl Setup for SceneView {
    fn setup(mut self: Weak<Self>) {
        self.id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        self.controls = true;
        self.picture.place().back();

        self.enable_touch();
        self.touch().all.val(move |touch| self.on_touch(touch));

        self.enable_pinch();
        self.touch().pinch.val(self, move |pinch: Pinch| self.zoom(1.0 / pinch.scale));

        TouchStack::enable_scroll(self);
    }
}

impl Scrollable for SceneView {
    fn __process_scroll_touch(&mut self, _touch: Touch) -> bool {
        false
    }

    fn __moves_sideways(&self) -> bool {
        true
    }

    fn __process_wheel_scroll(&mut self, delta: Point) {
        self.zoom((-delta.y * WHEEL_ZOOM).exp());
    }
}

impl Drop for SceneView {
    fn drop(&mut self) {
        if self.target.is_ok() {
            self.target.free();
        }
    }
}
