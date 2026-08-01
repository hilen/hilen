use std::{
    mem::take,
    ops::Deref,
    path::PathBuf,
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
};

use hreads::{assert_main_thread, from_main, on_main};
use parking_lot::Mutex;
use plat::Platform;
use refs::{Own, Weak};

use crate::{
    gm::{
        ToF32,
        color::Color,
        flat::{Point, Rect, Size},
    },
    ui::{Keymap, RootView, Setup, TouchStack, UIAnimation, UIEvent, View, ViewData, ViewFrame, WeakView},
    window::Window,
};

pub(crate) static DELETED_VIEWS: Mutex<Vec<Own<dyn View>>> = Mutex::new(Vec::new());

static ANIMATIONS: Mutex<Vec<UIAnimation>> = Mutex::new(Vec::new());

static UI_MANAGER: OnceLock<UIManager> = OnceLock::new();

/// False while an app is still doing async startup, a loading screen fetching
/// assets for one. The test autorun waits for ready before tearing the root
/// down, since a mid load teardown frees views the load task still touches. An
/// app with no such phase leaves it at the ready default.
static APP_READY: AtomicBool = AtomicBool::new(true);

/// Actions parked by `on_app_ready` while the app is not ready yet, run in
/// order the moment it becomes ready.
static ON_APP_READY: Mutex<Vec<Box<dyn FnOnce() + Send>>> = Mutex::new(Vec::new());

#[cfg(ios)]
static IOS_KEYBOARD_INIT: std::sync::Once = std::sync::Once::new();

pub struct UIManager {
    pub(crate) root_view: Own<RootView>,

    touch_disabled: AtomicBool,

    cursor_position: Mutex<Point>,

    draw_debug_frames: AtomicBool,

    scale:         AtomicU32,
    manual_scale:  AtomicU32,
    scale_changed: UIEvent<f32>,

    on_drop_file: UIEvent<PathBuf>,

    draw_touches: AtomicBool,

    keymap: Own<Keymap>,

    selected_view: Mutex<WeakView>,

    app_instance_id: String,
}

impl UIManager {
    pub(crate) const ROOT_VIEW_Z_OFFSET: f32 = 0.5;
    pub(crate) const MODAL_Z_OFFSET: f32 = 0.4;
    pub const DEBUG_Z_OFFSET: f32 = 0.3;

    pub(crate) const fn subview_z_offset() -> f32 {
        0.000_01
    }

    pub(crate) const fn additional_z_offset() -> f32 {
        Self::subview_z_offset() / 100.0
    }

    pub fn fps() -> f32 {
        Window::current().fps()
    }

    pub fn frame_drawn() -> u32 {
        Window::current().frame_drawn()
    }

    pub fn scale() -> f32 {
        f32::from_bits(Self::get().scale.load(Ordering::Relaxed))
    }

    pub(crate) fn cursor_position() -> Point {
        *Self::get().cursor_position.lock()
    }

    pub(crate) fn set_cursor_position(pos: Point) {
        *Self::get().cursor_position.lock() = pos;
    }

    pub fn set_scale(scale: impl ToF32) {
        on_main(move || {
            let sf = Self::get();
            let scale = scale.to_f32();

            let manual_scale = f32::from_bits(sf.manual_scale.load(Ordering::Relaxed));

            let scale = if manual_scale == 0.0 { scale } else { manual_scale };

            sf.scale.store(scale.to_bits(), Ordering::Relaxed);
            sf.scale_changed.trigger(scale);
        });
    }

    pub fn override_scale(scale: impl ToF32) {
        assert_main_thread();
        let sf = Self::get();

        let scale = scale.to_f32();

        sf.manual_scale.store(scale.to_bits(), Ordering::Relaxed);

        Self::set_scale(scale);
    }

    /// The override in effect, 0.0 meaning none. The test harness pins scale 1
    /// and has to put this back, or the app stays at scale 1 on a screen that
    /// is not, which on a phone means half sized UI.
    pub(crate) fn scale_override() -> f32 {
        f32::from_bits(Self::get().manual_scale.load(Ordering::Relaxed))
    }

    /// Drop the override and go back to the screen's own scale.
    pub(crate) fn restore_scale_override(scale: f32) {
        assert_main_thread();
        let sf = Self::get();

        sf.manual_scale.store(scale.to_bits(), Ordering::Relaxed);

        let real = Window::screen_scale();
        Self::set_scale(if scale == 0.0 { real } else { scale });
    }

    pub(crate) fn on_scale_changed<U: ?Sized>(subscriber: Weak<U>, mut cb: impl FnMut(f32) + Send + 'static) {
        Self::get().scale_changed.val(subscriber, move |scale| {
            cb(scale);
        });
    }

    pub(crate) fn unselect_view() {
        let this = Self::get();
        let mut selected_view = this.selected_view.lock();
        if selected_view.is_null() {
            return;
        }
        selected_view.__base_view().is_selected = false;
        selected_view.__internal_on_selection_changed(false);
        *selected_view = Weak::default();
    }

    pub(crate) fn set_selected(mut view: WeakView, selected: bool) {
        let this = Self::get();

        let mut selected_view = this.selected_view.lock();

        if let Some(selected) = selected_view.get_mut() {
            selected.__internal_on_selection_changed(false);
            *selected_view = Weak::default();
        }

        if selected {
            *selected_view = view;
        }

        view.__base_view().is_selected = selected;
        view.__internal_on_selection_changed(selected);
    }
}

impl UIManager {
    fn init() -> Self {
        let mut root_view = RootView::new();
        root_view.__base_view().view_label = "Root view".to_string();
        root_view.setup_root();

        Self {
            root_view,
            touch_disabled: false.into(),
            cursor_position: Mutex::new(Point::default()),
            draw_debug_frames: false.into(),
            scale: AtomicU32::new(1.0f32.to_bits()),
            manual_scale: AtomicU32::new(0.0f32.to_bits()),
            scale_changed: UIEvent::default(),
            on_drop_file: UIEvent::default(),
            draw_touches: false.into(),
            keymap: Own::default(),
            selected_view: Mutex::new(Weak::default()),
            app_instance_id: netrun::System::generate_app_instance_id(),
        }
    }

    pub(crate) fn get() -> &'static Self {
        UI_MANAGER.get_or_init(Self::init)
    }

    pub fn app_instance_id() -> &'static str {
        &Self::get().app_instance_id
    }

    /// An app with async startup calls this with `false` while it loads and
    /// `true` once its real UI is up. Setting `true` runs everything parked on
    /// `on_app_ready`.
    pub fn set_app_ready(ready: bool) {
        let parked = {
            let mut queue = ON_APP_READY.lock();
            APP_READY.store(ready, Ordering::Relaxed);
            if ready { take(&mut *queue) } else { vec![] }
        };
        for action in parked {
            action();
        }
    }

    /// Runs `action` once the app is ready, right now if it already is. The
    /// store and this check share the queue lock, so a concurrent
    /// `set_app_ready` can neither drop the action nor run it twice.
    pub(crate) fn on_app_ready(action: impl FnOnce() + Send + 'static) {
        let mut queue = ON_APP_READY.lock();
        if APP_READY.load(Ordering::Relaxed) {
            drop(queue);
            action();
        } else {
            queue.push(Box::new(action));
        }
    }

    pub(crate) fn window_resolution() -> Size {
        let size = if Platform::IOS {
            Window::render_size()
        } else {
            Window::inner_size()
        };
        (size.width, size.height).into()
    }

    /// Pixels the app renders into. The same as the window in an app, because
    /// the root fills it. A UI test pins the root to a fixed canvas instead, so
    /// a game or a level lands on the same pixels on any screen.
    pub(crate) fn render_area() -> Size {
        let size = Self::root_view().size();
        let scale = Self::scale();
        (size.width * scale, size.height * scale).into()
    }

    pub(crate) fn display_scale() -> f32 {
        Window::screen_scale()
    }

    pub fn root_view() -> Weak<RootView> {
        Self::get().root_view.weak()
    }

    pub(crate) fn root_view_static() -> &'static RootView {
        Self::get().root_view.deref()
    }

    pub(crate) fn free_deleted_views() {
        DELETED_VIEWS.lock().clear();
        TouchStack::get().clear_freed();
    }

    pub fn enable_debug_frames() {
        Self::get().draw_debug_frames.store(true, Ordering::Relaxed);
    }

    pub fn disable_debug_frames() {
        Self::get().draw_debug_frames.store(false, Ordering::Relaxed);
    }

    pub(crate) fn should_draw_debug_frames() -> bool {
        Self::get().draw_debug_frames.load(Ordering::Relaxed)
    }

    pub(crate) fn draw_touches() -> bool {
        Self::get().draw_touches.load(Ordering::Relaxed)
    }

    pub fn set_display_touches(display: bool) {
        Self::get().draw_touches.store(display, Ordering::Relaxed);
    }

    pub fn keymap() -> &'static Keymap {
        Self::get().keymap.deref()
    }

    pub fn cloud_storage_dir() -> Option<PathBuf> {
        #[cfg(ios)]
        {
            let path = unsafe { crate::ui::mobile::ios::test_engine_ios_get_icloud_storage_path() };

            if path.is_null() {
                return None;
            }

            let path = unsafe { std::ffi::CStr::from_ptr(path) };

            let Ok(path) = path.to_str() else {
                log::error!("Failed to get cloud storage path");
                return None;
            };

            Some(PathBuf::from(path))
        }
        #[cfg(not(ios))]
        None
    }
}

impl UIManager {
    pub(crate) fn touch_disabled() -> bool {
        Self::get().touch_disabled.load(Ordering::Relaxed)
    }

    fn disable_touch() {
        Self::get().touch_disabled.store(true, Ordering::Relaxed);
    }

    fn enable_touch() {
        Self::get().touch_disabled.store(false, Ordering::Relaxed);
    }
}

impl UIManager {
    pub(crate) fn open_keyboard(#[allow(unused_variables)] frame: &Rect) {
        #[cfg(ios)]
        {
            crate::ui::ui_manager::IOS_KEYBOARD_INIT.call_once(|| {
                unsafe { crate::ui::mobile::ios::test_engine_ios_init_text_field() };
            });

            unsafe {
                crate::ui::mobile::ios::test_engine_ios_open_keyboard(
                    frame.origin.x,
                    frame.origin.y,
                    frame.size.width,
                    frame.size.height,
                )
            }
        }
    }

    pub(crate) fn close_keyboard() -> Option<String> {
        #[cfg(ios)]
        unsafe {
            let str_ptr = crate::ui::mobile::ios::test_engine_ios_close_keyboard();
            let cstr = std::ffi::CStr::from_ptr(str_ptr);
            return cstr.to_string_lossy().into_owned().into();
        }

        #[cfg(not(ios))]
        None
    }

    pub fn set_view<T: View + 'static>(view: Own<T>) -> Weak<T> {
        from_main(move || {
            let weak = view.weak();
            let mut root = UIManager::root_view();
            root.clear_root();
            let view = root.add_subview_to_root(view);
            view.place().back();

            weak
        })
    }
}

impl UIManager {
    pub(crate) fn trigger_drop_file(file: PathBuf) {
        Self::get().on_drop_file.trigger(file);
    }

    pub fn on_drop_file<T: ?Sized>(subscriber: Weak<T>, action: impl FnMut(PathBuf) + Send + 'static) {
        Self::get().on_drop_file.val(subscriber, action);
    }

    pub fn set_clear_color(color: impl Into<Color>) {
        Window::set_clear_color(color);
    }

    pub(crate) fn add_animation(anim: UIAnimation) {
        ANIMATIONS.lock().push(anim);
    }

    pub(crate) fn has_live_animations() -> bool {
        !ANIMATIONS.lock().is_empty()
    }

    pub(crate) fn commit_animations() {
        let mut animations = ANIMATIONS.lock();

        if animations.is_empty() {
            return;
        }

        animations.retain_mut(|a| {
            let retain = a.active();

            if retain {
                a.commit();
            } else {
                a.on_finish.trigger(());
            }

            retain
        });

        drop(animations);

        // An animation is live, so keep the loop drawing. This also fires once
        // after the last one finishes, so its settled and on_finish state
        // reaches the screen before the loop goes back to sleep.
        crate::window::request_frame();
    }
}

pub struct TouchLock;

impl TouchLock {
    pub fn new() -> Self {
        UIManager::disable_touch();
        TouchLock
    }
}

impl Drop for TouchLock {
    fn drop(&mut self) {
        UIManager::enable_touch();
    }
}
