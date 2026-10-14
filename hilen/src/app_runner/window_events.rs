//! What the window tells the app: its start, each frame and every input.

use std::{path::PathBuf, sync::Once};

use log::debug;
use plat::Platform;
#[cfg(desktop)]
use winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use winit::{
    event::{KeyEvent, TouchPhase},
    keyboard::{Key, PhysicalKey},
};

#[cfg(not_wasm)]
use super::WINDOW_READY;
use super::{AppRunner, CURSOR_POSITION, damage};
#[cfg(feature = "scene")]
use crate::scene_drawer::SceneDrawer;
use crate::{
    deps::hreads::invoke_dispatched,
    gm::flat::{Point, Size},
    pipelines::Pipelines,
    ui::{
        Cursor, Focus, Hover, Input, ScreenKeyboard, Theme, Touch, TouchEvent, UIDrawer, UIEvents, UIManager,
        ViewData, ViewSubviews, WeakView, damage::Damage, latin_candidate,
    },
    window::{ElementState, MouseButton, RenderFrame, Theme as OsTheme, Window},
};
#[cfg(feature = "level")]
use crate::{level::LevelManager, level_drawer::LevelDrawer};

impl crate::window::WindowEvents for AppRunner {
    fn window_ready(&mut self) {
        static INIT: Once = Once::new();

        INIT.call_once(|| {
            Pipelines::initialize();

            let mut root = UIManager::root_view();
            let view = root.add_subview_to_root(crate::app::app().make_root_view());
            view.place().back();

            UIManager::on_scale_changed(root, move |scale| {
                root.rescale_root(scale);
            });

            self.update();

            crate::window::state::State::resize();

            self.resize(
                Window::inner_position(),
                Window::outer_position(),
                Window::inner_size(),
                Window::outer_size(),
            );

            debug!("UI initialized");

            if let Some(theme) = Window::system_theme() {
                Theme::set_system(theme.into());
            }

            #[cfg(linux)]
            crate::window::wsl::theme::start();

            #[cfg(not_wasm)]
            {
                #[cfg(desktop)]
                {
                    let app = crate::app::app();
                    match app.window_placement() {
                        Some(placement) => Window::current().apply_placement(&placement),
                        None => Window::current().apply_initial_size(app.initial_size()),
                    }
                }
                #[cfg(feature = "inspect")]
                crate::inspect::InspectService::start_listening();

                #[cfg(feature = "ui-tests")]
                Self::spawn_test_autorun();
            }

            #[cfg(wasm)]
            crate::assets::start_boot_preload();

            #[cfg(all(wasm, feature = "inspect"))]
            crate::inspect::web_transport::start_if_requested();

            #[cfg(all(wasm, feature = "ui-tests"))]
            Self::spawn_test_autorun();

            UIManager::keymap().add(UIManager::root_view(), 'i', || {
                fn call_inspect(mut view: WeakView) {
                    view.__internal_inspect();
                    for sub in view.subviews() {
                        call_inspect(sub.weak());
                    }
                }

                call_inspect(UIManager::root_view());
            });

            crate::app::app().after_launch();

            #[cfg(desktop)]
            crate::system::UpdateState::check_after_launch();

            #[cfg(not_wasm)]
            crate::deps::hreads::spawn(async {
                debug!("window ready");
                WINDOW_READY.lock().trigger(());
            });
        });
    }

    fn update(&mut self) {
        UIManager::free_deleted_views();
        invoke_dispatched();
        #[cfg(feature = "scene")]
        SceneDrawer::update();
        #[cfg(feature = "level")]
        LevelDrawer::update();
        ScreenKeyboard::update();
        UIDrawer::update();
        // A hidden `VideoView` gets no update, its element is hidden here.
        #[cfg(all(wasm, feature = "video"))]
        crate::video::hide_unplaced();
        // After layout, so a row that replaced the dead hovered one
        // already has its frame when hover re-picks.
        #[cfg(any(desktop, wasm))]
        Hover::refresh_dead();
        Focus::update();
    }

    #[cfg(any(feature = "scene", feature = "level"))]
    fn prepare(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if UIManager::window_resolution().has_no_area() {
            return;
        }

        #[cfg(feature = "scene")]
        SceneDrawer::prepare(encoder);
        #[cfg(feature = "level")]
        LevelDrawer::prepare(encoder);
    }

    fn render(&mut self, frame: &mut RenderFrame) {
        if UIManager::window_resolution().has_no_area() {
            return;
        }

        #[cfg(feature = "scene")]
        SceneDrawer::draw(frame.pass());
        #[cfg(feature = "level")]
        LevelDrawer::draw(frame.pass());
        UIDrawer::draw(frame);
    }

    fn damage(&mut self) -> Damage {
        damage::of_frame()
    }

    fn needs_sampleable_frame(&self) -> bool {
        UIDrawer::needs_sampleable_frame()
    }

    fn resize(&mut self, inner_pos: Point, outer_pos: Point, inner_size: Size, outer_size: Size) {
        UIManager::set_scale(UIManager::display_scale());
        #[cfg(feature = "level")]
        LevelManager::set_scale(UIManager::display_scale());

        UIManager::root_view().resize_root(inner_pos, outer_pos, inner_size, outer_size, UIManager::scale());
        UIEvents::size_changed().trigger(());
        self.update();
    }

    fn mouse_moved(&mut self, position: Point) -> bool {
        self.cursor_position = position;
        *CURSOR_POSITION.get_mut() = position;
        Input::process_touch_event(Touch {
            id: 1,
            position,
            event: TouchEvent::Moved,
            button: MouseButton::Left,
        })
    }

    fn mouse_event(&mut self, state: ElementState, button: MouseButton) -> bool {
        Input::process_touch_event(Touch {
            id: 1,
            position: self.cursor_position,
            event: state.into(),
            button,
        })
    }

    fn mouse_scroll(&mut self, delta: Point) {
        Input::on_wheel(delta);
    }

    fn pinch(&mut self, growth: f32, phase: TouchPhase) {
        Input::on_trackpad_pinch(growth, phase);
    }

    fn mouse_motion(&mut self, delta: Point) {
        Cursor::add_motion(delta);
    }

    fn cursor_left(&mut self) {
        Hover::clear();
    }

    /// A window that lost focus cannot hold the mouse, another app has
    /// it now.
    fn focus_changed(&mut self, focused: bool) {
        if !focused {
            Cursor::release();
            // The release of a button held while focus left goes to
            // the other app, it would stay held here for good.
            crate::ui::Mouse::clear();
        }
    }

    fn touch_event(&mut self, touch: winit::event::Touch) -> bool {
        if Platform::TVOS {
            self.remote_surface(&touch);
            return true;
        }

        let event = match touch.phase {
            TouchPhase::Started => TouchEvent::Began,
            TouchPhase::Moved => TouchEvent::Moved,
            TouchPhase::Ended | TouchPhase::Cancelled => TouchEvent::Ended,
        };

        Input::process_touch_event(Touch {
            id: self.engine_touch_id(touch.id, event),
            position: (touch.location.x, touch.location.y).into(),
            event,
            button: MouseButton::Left,
        })
    }

    fn key_event(&mut self, event: KeyEvent) {
        if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
            crate::ui::Keys::set(code, event.state.is_pressed());
        }

        if !event.state.is_pressed() {
            if let Key::Named(key) = event.logical_key {
                Input::on_key_up(back_as_escape(key));
            }
            return;
        }

        if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
            use crate::bug_report::InputRing;

            InputRing::record(code);

            // The browser keeps Cmd+Shift+R as its hard reload, the page
            // never sees it, see `install_reload_shortcut_listener`.
            #[cfg(not_wasm)]
            {
                let modifiers = InputRing::modifiers();

                if code == winit::keyboard::KeyCode::KeyR
                    && modifiers.shift_key()
                    && (modifiers.control_key() || modifiers.super_key())
                {
                    crate::bug_report::BugReport::open();
                }
            }
        }

        // The play and pause button of the remote of an Apple TV comes as a
        // key. On a Mac the system takes that key and sends a media command,
        // so an app reads it the same way on both.
        #[cfg(tvos)]
        if event.logical_key == Key::Named(winit::keyboard::NamedKey::MediaPlayPause) {
            crate::system::deliver_media_command(crate::system::MediaCommand::Toggle);
            return;
        }

        if let Key::Named(key) = event.logical_key {
            Input::on_key(back_as_escape(key));
        }

        if let Some(text) = event.logical_key.to_text() {
            let typed = text.chars().last().unwrap();
            let code = match event.physical_key {
                PhysicalKey::Code(code) => Some(code),
                PhysicalKey::Unidentified(_) => None,
            };
            Input::on_char(typed, latin_candidate(typed, code, system_latin(&event)));
        }
    }

    fn modifiers_changed(&mut self, modifiers: winit::event::Modifiers) {
        Input::set_modifiers(modifiers.state());
        crate::bug_report::InputRing::set_modifiers(modifiers.state());
    }

    fn hovered_file(&mut self, path: PathBuf) {
        UIManager::trigger_hover_file(path);
    }

    fn hovered_file_cancelled(&mut self) {
        UIManager::trigger_hover_file_end();
    }

    fn dropped_file(&mut self, path: PathBuf) {
        UIManager::trigger_drop_file(path);
    }

    fn theme_changed(&mut self, theme: OsTheme) {
        Theme::set_system(theme.into());
    }
}

/// The Latin letter the system gives the physical key, for a binding while
/// the layout types other letters. Only the desktop systems tell it.
fn system_latin(event: &KeyEvent) -> Option<char> {
    #[cfg(desktop)]
    return event.latin_key().and_then(|key| key.chars().next());
    #[cfg(not(desktop))]
    {
        _ = event;
        None
    }
}

/// The Back key of a remote or of a keyboard with media keys does what
/// Escape does on a desktop, it closes the modal or the screen on top.
fn back_as_escape(key: winit::keyboard::NamedKey) -> winit::keyboard::NamedKey {
    use winit::keyboard::NamedKey;

    match key {
        NamedKey::GoBack | NamedKey::BrowserBack => NamedKey::Escape,
        key => key,
    }
}
