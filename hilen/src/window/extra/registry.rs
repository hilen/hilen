//! The extra windows of a desktop or a headless run, by window slot.

use std::sync::Arc;

use anyhow::{Result, bail, ensure};
use log::{debug, info, warn};
use plat::Platform;

use super::{AppWindow, WindowLevel, WindowSpec};
use crate::{
    deps::{
        hreads::assert_main_thread,
        refs::{Own, main_lock::MainLock},
    },
    gm::{LossyConvert, flat::Size},
    ui::{DELETED_VIEWS, TouchStack, UIManager, View, ViewData},
    window::{
        Window,
        app_handler::AppHandler,
        screen::Screen,
        slots::{self, MAX_WINDOWS, within},
        state::State,
    },
};

type MakeRoot = Box<dyn FnOnce() -> Own<dyn View>>;

struct Extra {
    generation: u32,
    spec:       WindowSpec,
    /// Made into the root view once the window is up.
    make_root:  Option<MakeRoot>,
    window:     Option<Window>,
    #[cfg(macos)]
    front_app:  Option<crate::window::extra_mac::FrontApp>,
    /// The class of a window that is a panel now, see `extra_mac`.
    #[cfg(macos)]
    class:      Option<&'static objc2::runtime::AnyClass>,
}

static EXTRAS: MainLock<Vec<Option<Extra>>> = MainLock::new();

fn extras() -> &'static mut Vec<Option<Extra>> {
    let extras = EXTRAS.get_mut();
    if extras.is_empty() {
        extras.resize_with(MAX_WINDOWS, || None);
    }
    extras
}

fn extra(slot: usize, generation: u32) -> Option<&'static mut Extra> {
    assert_main_thread();
    extras().get_mut(slot)?.as_mut().filter(|extra| extra.generation == generation)
}

pub(super) fn open_slots() -> Vec<usize> {
    let mut slots = vec![0];
    if EXTRAS.is_set() {
        slots.extend(
            extras()
                .iter()
                .enumerate()
                .filter(|(_, extra)| extra.is_some())
                .map(|(slot, _)| slot),
        );
    }
    slots
}

/// The extra windows that are up.
fn ready_slots() -> Vec<usize> {
    if !EXTRAS.is_set() {
        return vec![];
    }
    extras()
        .iter()
        .enumerate()
        .filter(|(_, extra)| extra.as_ref().is_some_and(|extra| extra.window.is_some()))
        .map(|(slot, _)| slot)
        .collect()
}

pub(crate) fn any_open() -> bool {
    open_slots().len() > 1
}

/// The `Window` of an extra slot that is up.
pub(crate) fn window_of(slot: usize) -> Option<&'static mut Window> {
    extras().get_mut(slot)?.as_mut()?.window.as_mut()
}

/// The slot of the winit window `id`, `None` for the main window.
pub(crate) fn slot_of_id(id: winit::window::WindowId) -> Option<usize> {
    ready_slots().into_iter().find(|slot| {
        window_of(*slot)
            .and_then(|window| window.screen.winit_window().map(winit::window::Window::id))
            .is_some_and(|winit| winit == id)
    })
}

pub(super) fn generation_of(slot: usize) -> Option<u32> {
    Some(extras().get(slot)?.as_ref()?.generation)
}

pub(super) fn open(spec: WindowSpec, make_root: MakeRoot) -> Result<AppWindow> {
    assert_main_thread();
    ensure!(Platform::DESKTOP || Window::headless(), "A phone has 1 window");

    let Some(slot) = (1..MAX_WINDOWS).find(|slot| extras()[*slot].is_none()) else {
        bail!("All {MAX_WINDOWS} windows are open");
    };
    slots::renew(slot);
    let generation = slots::generation(slot);

    info!("Window {slot} opens: {} at {:?}", spec.title, spec.place.frame());

    #[cfg(macos)]
    let front_app = (spec.level == WindowLevel::AboveAll)
        .then(crate::window::extra_mac::FrontApp::now)
        .flatten();

    extras()[slot] = Some(Extra {
        generation,
        spec,
        make_root: Some(make_root),
        window: None,
        #[cfg(macos)]
        front_app,
        #[cfg(macos)]
        class: None,
    });

    if Window::headless() {
        build_headless(slot);
    } else {
        crate::window::redraw::send_user_event(crate::window::UserEvent::OpenWindow(slot));
    }

    Ok(AppWindow { slot, generation })
}

#[cfg(feature = "inspect")]
pub(super) fn title(slot: usize, generation: u32) -> Option<String> {
    extra(slot, generation).map(|extra| extra.spec.title.clone())
}

pub(super) fn is_open(slot: usize, generation: u32) -> bool {
    extra(slot, generation).is_some()
}

pub(super) fn is_ready(slot: usize, generation: u32) -> bool {
    extra(slot, generation).is_some_and(|extra| extra.window.is_some())
}

pub(super) fn within_window<R>(slot: usize, generation: u32, action: impl FnOnce() -> R) -> Result<R> {
    ensure!(is_ready(slot, generation), "Window {slot} is not up");
    Ok(within(slot, action))
}

pub(super) fn focus(slot: usize, generation: u32) {
    if let Some(winit) = extra(slot, generation)
        .and_then(|extra| extra.window.as_ref())
        .and_then(|window| window.screen.winit_window())
    {
        winit.focus_window();
    }
}

pub(super) fn close(slot: usize, generation: u32) {
    if extra(slot, generation).is_none() {
        return;
    }
    let Some(extra) = extras()[slot].take() else {
        return;
    };
    info!("Window {slot} closes: {}", extra.spec.title);

    if let Some(root) = UIManager::take_root(slot) {
        let root: Own<dyn View> = root;
        DELETED_VIEWS.lock().push(root);
    }
    TouchStack::forget_window(slot);
    crate::ui::loops::forget_window(slot);

    #[cfg(macos)]
    if let (Some(class), Some(winit)) = (
        extra.class,
        extra.window.as_ref().and_then(|window| window.screen.winit_window()),
    ) {
        crate::window::extra_mac::put_class_back(winit, class);
    }
    // The surface goes with the window, in its own order.
    drop(extra.window);
    slots::leave(slot);

    #[cfg(macos)]
    if extra.spec.level == WindowLevel::AboveAll
        && !extras().iter().flatten().any(|other| other.spec.level == WindowLevel::AboveAll)
        && let Some(front_app) = extra.front_app
    {
        front_app.activate();
    }

    crate::window::request_frame();
}

fn build_headless(slot: usize) {
    let main = AppHandler::main_window();
    let extra = extras()[slot].as_mut().expect("Slot was just filled");
    let size: Size<u32> = extra.spec.place.frame().size.lossy_convert();
    extra.window = Some(Window::extra(main, Screen::Headless { size }));
    finish(slot);
}

/// Makes the winit window and the surface of `slot`, inside a turn of the
/// loop, the only place winit makes a window.
pub(crate) fn create(slot: usize, event_loop: &winit::event_loop::ActiveEventLoop) {
    if let Err(err) = try_create(slot, event_loop) {
        warn!("Window {slot} did not open: {err:#}");
        extras()[slot] = None;
    }
}

fn try_create(slot: usize, event_loop: &winit::event_loop::ActiveEventLoop) -> Result<()> {
    use winit::dpi::{LogicalPosition, LogicalSize};

    let Some(extra) = extras()[slot].as_mut() else {
        debug!("Window {slot} closed before it was up");
        return Ok(());
    };
    let frame = extra.spec.place.frame();
    let mut attributes = winit::window::Window::default_attributes()
        .with_title(extra.spec.title.clone())
        .with_decorations(extra.spec.decorations)
        .with_position(LogicalPosition::new(frame.origin.x, frame.origin.y))
        .with_inner_size(LogicalSize::new(frame.size.width, frame.size.height));
    if extra.spec.level == WindowLevel::AboveAll {
        // Hidden until it is on every desktop, see `extra_mac::show_here`.
        attributes = attributes
            .with_window_level(winit::window::WindowLevel::AlwaysOnTop)
            .with_resizable(false)
            .with_visible(!cfg!(macos));
    }

    let winit = Arc::new(event_loop.create_window(attributes)?);
    let inner = winit.inner_size();
    ensure!(
        inner.width > 0 && inner.height > 0,
        "The window has no area, {inner:?}"
    );

    let main = AppHandler::main_window();
    let surface = crate::window::surface::Surface::new(
        &main.instance,
        &main.adapter,
        &main.device,
        crate::window::window::surface_config_with_size((inner.width, inner.height)),
        winit.clone(),
    )?;

    #[cfg(macos)]
    if extra.spec.level == WindowLevel::AboveAll {
        extra.class = Some(crate::window::extra_mac::raise_above_all(&winit)?);
    }

    extra.window = Some(Window::extra(
        main,
        Screen::Windowed {
            winit_window: winit.clone(),
            surface:      Some(surface),
            size:         Size::new(inner.width, inner.height),
        },
    ));
    finish(slot);
    // A window over everything takes the keys as a panel, the app is not
    // activated, see `extra_mac`.
    #[cfg(macos)]
    if extra.spec.level == WindowLevel::AboveAll {
        crate::window::extra_mac::show_here(&winit)?;
    } else {
        winit.focus_window();
    }
    #[cfg(not(macos))]
    winit.focus_window();
    winit.request_redraw();
    Ok(())
}

/// The root view of `slot`, once its window is up.
fn finish(slot: usize) {
    let make_root = extras()[slot].as_mut().and_then(|extra| extra.make_root.take());
    within(slot, || {
        UIManager::set_scale(UIManager::display_scale());
        if let Some(make_root) = make_root {
            let mut root = UIManager::root_view();
            let view = root.add_subview_to_root(make_root());
            view.place().back();
        }
        State::resize();
    });
    info!("Window {slot} is up");
}

/// The frame of an extra window. The queued callbacks, the removed views and
/// the animations run with the frame of the main window. A covered main
/// window draws no frame, then this frame runs them first.
pub(crate) fn frame(slot: usize) {
    if window_of(slot).is_none() {
        return;
    }
    if crate::window::occluded() {
        within(0, AppHandler::run_shared);
    }
    within(slot, || {
        let window = AppHandler::window();
        window.state.update_window();
        window.state.render();
    });
}

/// Every extra window draws again, after a change or while something moves.
pub(crate) fn request_redraws() {
    for slot in ready_slots() {
        if let Some(winit) = window_of(slot).and_then(|window| window.screen.winit_window()) {
            winit.request_redraw();
        }
    }
}

/// The frames of the headless extra windows, 1 turn of the headless loop.
pub(crate) fn headless_frames() {
    for slot in ready_slots() {
        frame(slot);
    }
}
