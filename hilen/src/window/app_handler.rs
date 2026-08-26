use std::{
    process::exit,
    sync::atomic::{AtomicBool, Ordering},
};

use log::{debug, error};
use plat::Platform;
use winit::{
    application::ApplicationHandler,
    event::{MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowId,
};

use crate::{
    deps::refs::main_lock::MainLock,
    gm::flat::Point,
    window::{Window, WindowEvents, state::State},
};

static APP_HANDLER: MainLock<Option<AppHandler>> = MainLock::new();

/// Rough pixel height of one mouse wheel line. Wheel deltas arrive in lines,
/// trackpads send pixels, this converts lines to pixels. The result is
/// scaled again by `SCROLL_SPEED` in the engine.
const LINE_SCROLL_PIXELS: f32 = 28.0;

/// What the event loop delivers through its user event channel.
pub(crate) enum UserEvent {
    /// The GPU window finished async setup on a worker and is ready to install.
    /// Boxed because it dwarfs the other variant.
    WindowReady(Box<Window>),
    /// A nudge that only exists to wake the loop from `ControlFlow::Wait` when
    /// background work queued a main thread callback. Handled by drawing a
    /// frame in `about_to_wait`, so the handler itself does nothing. Wasm
    /// polls every iteration and never waits, so it has no wake.
    #[cfg(not_wasm)]
    Wake,
}

#[allow(clippy::large_enum_variant)]
enum AppHandlerState {
    Ready(Window),
    Init(Option<EventLoopProxy<UserEvent>>),
}

impl AppHandlerState {
    // Only the native frame pacing asks, wasm polls every iteration.
    #[cfg(not_wasm)]
    fn ready(&self) -> bool {
        !self.not_ready()
    }

    fn not_ready(&self) -> bool {
        matches!(self, Self::Init(_))
    }
}

pub struct AppHandler {
    state:                       AppHandlerState,
    pub(crate) te_window_events: Box<dyn WindowEvents>,
    pub(crate) close:            AtomicBool,
}

impl AppHandler {
    pub(crate) fn new(
        app: impl WindowEvents + 'static,
        event_loop: &EventLoop<UserEvent>,
    ) -> &'static mut Self {
        let handler = APP_HANDLER.get_mut();

        *handler = Some(Self {
            state:            AppHandlerState::Init(Some(event_loop.create_proxy())),
            te_window_events: Box::new(app),
            close:            AtomicBool::new(false),
        });

        handler.as_mut().expect("Failed to get handler")
    }
}

impl AppHandler {
    /// Run without a window, an event loop or a display. Frames render to an
    /// offscreen texture in a plain loop until [`Window::close`] is called.
    #[cfg(not_wasm)]
    pub(crate) fn run_headless(app: impl WindowEvents + 'static, size: crate::gm::flat::Size<u32>) {
        let handler = APP_HANDLER.get_mut();

        *handler = Some(Self {
            state:            AppHandlerState::Init(None),
            te_window_events: Box::new(app),
            close:            AtomicBool::new(false),
        });

        let handler = handler.as_mut().expect("Failed to get handler");

        let window = match crate::deps::hreads::unasync(Window::create_headless(size)) {
            Ok(window) => window,
            Err(err) => {
                error!("Fatal: could not create headless window: {err:?}");
                eprintln!("Fatal: could not create headless window: {err:?}");
                exit(1);
            }
        };

        handler.state = AppHandlerState::Ready(window);
        handler.te_window_events.window_ready();

        while !handler.close.load(Ordering::Relaxed) {
            let window = Self::window();

            window.state.update();

            window.state.render();
        }
    }

    #[cfg(desktop)]
    fn placement_changed() {
        if let Some(placement) = Window::placement() {
            crate::app::app().window_placement_changed(&placement);
        }
    }

    #[cfg(not(desktop))]
    fn placement_changed() {}

    pub(crate) fn close() {
        Self::current().close.store(true, Ordering::Relaxed);
    }

    pub fn current() -> &'static mut Self {
        APP_HANDLER
            .get_mut()
            .as_mut()
            .expect("AppHandler has not been initialized yet.")
    }

    pub(crate) fn window() -> &'static mut Window {
        let AppHandlerState::Ready(ref mut window) = Self::current().state else {
            panic!("Window is not initialized yet")
        };
        window
    }
}

impl ApplicationHandler<UserEvent> for AppHandler {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let AppHandlerState::Init(proxy) = &mut self.state
            && let Some(proxy) = proxy.take()
        {
            let mut win_attr = winit::window::Window::default_attributes();

            #[cfg(not_wasm)]
            {
                win_attr = win_attr.with_title("hilen");
            }

            #[cfg(wasm)]
            {
                use winit::platform::web::WindowAttributesExtWebSys;
                win_attr = win_attr.with_append(true);
            }

            let window = event_loop.create_window(win_attr).expect("create window err.");

            let render_size = if Platform::IOS {
                window.outer_size()
            } else {
                window.inner_size()
            };

            crate::deps::hreads::block_on(async move {
                if let Err(err) = Window::start_internal(render_size, window, proxy).await {
                    // fern logging can be swallowed on iOS, so also print to stderr.
                    // Exit instead of panicking. A panic here unwinds across the
                    // Objective-C run loop and turns into an opaque EXC_BAD_ACCESS.
                    error!("Fatal: could not start engine window: {err:?}");
                    eprintln!("Fatal: could not start engine window: {err:?}");
                    exit(1);
                }
            });
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::WindowReady(window) => {
                self.state = AppHandlerState::Ready(*window);
                self.te_window_events.window_ready();
            }
            // Waking the loop was the whole point. `about_to_wait` runs right
            // after this and draws a frame because a redraw was requested.
            #[cfg(not_wasm)]
            UserEvent::Wake => {}
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent) {
        // Any event other than a redraw can change what is on screen: input,
        // resize, theme, a dropped file. Ask for a frame so the change shows.
        // A redraw already draws, so it must not ask for another or the loop
        // never sleeps. Only native waits, wasm keeps its own polling cadence.
        #[cfg(not_wasm)]
        let request_frame = self.state.ready() && !matches!(event, WindowEvent::RedrawRequested);

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::CursorMoved { position, .. } => {
                self.te_window_events.mouse_moved((position.x, position.y).into());
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.te_window_events.mouse_event(state, button);
            }
            WindowEvent::CursorLeft { .. } => {
                self.te_window_events.cursor_left();
            }
            WindowEvent::Touch(touch) => {
                self.te_window_events.touch_event(touch);
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                MouseScrollDelta::LineDelta(x, y) => {
                    let point: Point = (x, y).into();
                    self.te_window_events.mouse_scroll(point * LINE_SCROLL_PIXELS);
                }
                MouseScrollDelta::PixelDelta(delta) => {
                    self.te_window_events.mouse_scroll((delta.x, delta.y).into());
                }
            },
            WindowEvent::KeyboardInput { event, .. } => {
                if Window::quit_on_escape() && event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    event_loop.exit();
                }
                self.te_window_events.key_event(event);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.te_window_events.modifiers_changed(modifiers);
            }
            WindowEvent::DroppedFile(path) => {
                self.te_window_events.dropped_file(path);
            }
            WindowEvent::ThemeChanged(theme) => {
                self.te_window_events.theme_changed(theme);
            }
            WindowEvent::Resized(_physical_size) => {
                if self.state.not_ready() {
                    return;
                }

                State::resize();
                Self::placement_changed();

                // Configuring the surface resizes the backing buffer, which
                // clears it. Leaving that for the next frame presents an empty
                // buffer, and a window drag fires resizes far faster than
                // frames, so the app reads as blank the whole time it is
                // dragged. Drawing here means nothing empty is ever shown.
                Self::window().state.render();
            }
            WindowEvent::Moved(_position) => {
                if self.state.not_ready() {
                    return;
                }
                Self::placement_changed();
            }
            WindowEvent::ScaleFactorChanged {
                scale_factor,
                inner_size_writer: _,
            } => {
                debug!("Scale factor: {scale_factor}");
            }
            WindowEvent::RedrawRequested => {
                if self.state.not_ready() {
                    return;
                }

                if self.close.load(Ordering::Relaxed) {
                    event_loop.exit();
                }

                Self::window().state.update();

                Self::window().state.render();
            }
            _ => {}
        }

        #[cfg(not_wasm)]
        if request_frame {
            crate::window::request_frame();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.not_ready() {
            return;
        }

        let Some(window) = Window::winit_window() else {
            return;
        };

        #[cfg(not_wasm)]
        {
            let continuous = crate::window::continuous_render_active();
            let pending = crate::window::take_needs_render();
            if !continuous && !pending {
                event_loop.set_control_flow(ControlFlow::Wait);
                return;
            }
            event_loop.set_control_flow(if continuous {
                ControlFlow::Poll
            } else {
                ControlFlow::Wait
            });
        }

        // Wait, not Poll. The redraw below re-arms a rAF every
        // iteration, so the loop is paced by the display. Poll spins
        // the runner through scheduler.postTask between frames, and
        // Firefox starves rAF under that flood, freezing the app.
        #[cfg(wasm)]
        event_loop.set_control_flow(ControlFlow::Wait);

        window.request_redraw();
    }
}
