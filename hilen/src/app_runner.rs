use std::{collections::HashMap, path::PathBuf, sync::Once};

use anyhow::Result;
use log::debug;
use winit::{
    event::{KeyEvent, TouchPhase},
    keyboard::Key,
};

#[cfg(desktop)]
use crate::deps::hreads::{is_main_thread, wait_for_next_frame};
#[cfg(not_wasm)]
use crate::deps::refs::Own;
#[cfg(any(desktop, feature = "level"))]
use crate::gm::LossyConvert;
#[cfg(feature = "scene")]
use crate::scene_drawer::SceneDrawer;
use crate::{
    App,
    deps::{
        hreads::{from_main, invoke_dispatched},
        refs::main_lock::MainLock,
    },
    gm::flat::{Point, Size},
    pipelines::Pipelines,
    ui::{
        Cursor, Hover, Input, Theme, Touch, TouchEvent, UIDrawer, UIEvents, UIManager, ViewData,
        ViewSubviews, WeakView, ui_test::human_pause,
    },
    window::{ElementState, MouseButton, RenderFrame, Screenshot, Theme as OsTheme, Window},
};
#[cfg(feature = "level")]
use crate::{level::LevelManager, level_drawer::LevelDrawer};

#[cfg(not_wasm)]
static WINDOW_READY: parking_lot::Mutex<crate::deps::vents::OnceEvent> =
    parking_lot::Mutex::new(crate::deps::vents::OnceEvent::const_default());
static CURSOR_POSITION: MainLock<Point> = MainLock::new();

/// Scroll sensitivity. Mouse wheel line deltas are already converted to
/// pixels by `LINE_SCROLL_PIXELS` in the window crate, then scaled by this.
const SCROLL_SPEED: f32 = 0.25;

/// Mouse events use id 1 and `NO_TOUCH_ID` is 0, so real fingers start above
/// both. Keeps a finger from ever colliding with the pointer or the "no
/// capture" sentinel.
const FIRST_TOUCH_ID: usize = 2;

pub struct AppRunner {
    pub cursor_position: Point,
    touch_ids:           HashMap<u64, usize>,
    next_touch_id:       usize,
}

impl AppRunner {
    pub fn stop() {
        Window::close();
    }

    pub(crate) fn cursor_position() -> Point {
        *CURSOR_POSITION
    }

    #[cfg(not_wasm)]
    pub(crate) fn setup_log(app_targets: &'static [&'static str]) {
        use chrono::Local;
        use fern::Dispatch;
        use log::{Level, LevelFilter, info, warn};

        #[cfg(target_os = "ios")]
        let output = fern::Output::call(|record| crate::ios_log::log(&record.args().to_string()));
        // Android swallows stdout, logcat is the only output that reaches
        // the developer, and going through the one dispatch feeds the bug
        // report ring the same lines as every other platform.
        #[cfg(target_os = "android")]
        let output: Box<dyn log::Log> = Box::new(android_logger::AndroidLogger::new(
            android_logger::Config::default().with_max_level(LevelFilter::Info),
        ));
        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        let output = std::io::stdout();

        let mut dispatch = Dispatch::new()
            .level(LevelFilter::Warn)
            .level_for("hilen", LevelFilter::Debug)
            .level_for("inspector", LevelFilter::Debug)
            .level_for("netrun", LevelFilter::Debug);

        for target in app_targets {
            dispatch = dispatch.level_for(*target, LevelFilter::Debug);
        }

        // The bug report ring keeps the same formatted lines the console
        // shows, so a report attaches exactly what the log printed.
        let ring = fern::Output::call(|record| {
            crate::bug_report::BugReport::push_log_line(record.args().to_string());
        });

        // The file gets a timestamp per line, the console stays as it is so
        // the lines tests grep for keep their shape.
        let file = match crate::log_file::create().and_then(|path| Ok((fern::log_file(&path)?, path))) {
            Ok((file, path)) => Some((
                Dispatch::new()
                    .format(|out, message, _| {
                        out.finish(format_args!("{} {message}", Local::now().format("%H:%M:%S%.3f")));
                    })
                    .chain(file),
                path,
            )),
            Err(err) => {
                eprintln!("log file: {err:#}");
                None
            }
        };

        let (file, path) = match file {
            Some((file, path)) => (Some(file), Some(path)),
            None => (None, None),
        };

        let mut dispatch = dispatch
            .format(|out, message, record| {
                let level_icon = match record.level() {
                    Level::Error => "🔴",
                    Level::Warn => "🟡",
                    Level::Info => "🟢",
                    Level::Debug => "🔵",
                    Level::Trace => "⚪",
                };

                let location = false;
                let module = false;

                let mut log = format!("{level_icon} {message}");

                if location {
                    log = format!(
                        "[{}::{}] {}",
                        record.file().unwrap_or_default(),
                        record.line().unwrap_or_default(),
                        log
                    );
                }

                if module {
                    log = format!("{} {}", record.module_path().unwrap_or_default(), log);
                }

                out.finish(format_args!("{log}"));
            })
            .chain(output)
            .chain(ring);

        if let Some(file) = file {
            dispatch = dispatch.chain(file);
        }

        dispatch.apply().expect("Failed to initialize logging");

        debug!("logs setup");
        match path {
            Some(path) => info!("log file {}", path.display()),
            None => warn!("no log file for this launch, see stderr"),
        }
    }

    #[cfg(not_wasm)]
    pub(crate) async fn setup_sentry(app: &dyn App) -> Option<sentry::ClientInitGuard> {
        let sentry_url = crate::config::Config::sentry_url(app).await?;

        let client = sentry::init((
            sentry_url,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                // Apps opt into Sentry by returning a DSN. Include user context, such as IPs and
                // HTTP headers, for richer diagnostics.
                send_default_pii: true,
                ..Default::default()
            },
        ));

        debug!("sentry ready");

        Some(client)
    }

    pub fn new(app: Box<dyn App>) -> Self {
        // A crate nested in a monorepo runs from its own directory and keeps
        // its assets there, so a cwd with an assets folder wins over the git
        // root, which would be the monorepo root.
        #[cfg(desktop)]
        {
            let root = std::env::current_dir()
                .ok()
                .filter(|dir| dir.join("assets").exists())
                .unwrap_or_else(|| crate::filesystem::Paths::git_root().expect("git_root()"));

            crate::assets::Assets::init(root);
        }
        #[cfg(mobile)]
        crate::assets::Assets::init(std::path::PathBuf::default());

        crate::app::set_app(app);

        Self {
            cursor_position: Point::default(),
            touch_ids:       HashMap::new(),
            next_touch_id:   FIRST_TOUCH_ID,
        }
    }

    /// Winit gives each finger a `u64` id that can be 0 and can outrange
    /// `usize` on wasm. Remap it to a fresh non-zero engine id per finger so
    /// two fingers stay independent and never clash with the mouse id. The
    /// mapping is dropped on `Ended` so a finished finger frees its id.
    fn engine_touch_id(&mut self, winit_id: u64, event: TouchEvent) -> usize {
        let id = if let Some(id) = self.touch_ids.get(&winit_id) {
            *id
        } else {
            let id = self.next_touch_id;
            self.next_touch_id += 1;
            self.touch_ids.insert(winit_id, id);
            id
        };

        if event == TouchEvent::Ended {
            self.touch_ids.remove(&winit_id);
        }

        id
    }

    #[cfg(not_wasm)]
    pub fn start_with_actor(
        actions: impl std::future::Future<Output = Result<()>> + Send + 'static,
    ) -> Result<()> {
        Self::start_with_actor_impl(actions, false);
        Ok(())
    }

    /// Run without a window or a display. Frames render to an offscreen
    /// texture. Screenshots and `check_colors` still work.
    #[cfg(not_wasm)]
    pub fn start_headless_with_actor(
        actions: impl std::future::Future<Output = Result<()>> + Send + 'static,
    ) -> Result<()> {
        Self::start_with_actor_impl(actions, true);
        Ok(())
    }

    #[cfg(not_wasm)]
    fn start_with_actor_impl(
        actions: impl std::future::Future<Output = Result<()>> + Send + 'static,
        headless: bool,
    ) {
        use crate::ui::{Setup, View};

        #[derive(Default)]
        struct ActorApp;

        impl App for ActorApp {
            fn make_root_view(&self) -> Own<dyn View> {
                crate::ui::Container::new()
            }
        }

        WINDOW_READY.lock().sub(|| {
            crate::deps::hreads::unasync(actions).unwrap();
        });

        if headless {
            crate::app_starter::hilen_start_with_app_headless(Box::new(ActorApp));
        } else {
            crate::app_starter::hilen_start_with_app(Box::new(ActorApp));
        }
    }

    pub fn set_window_title(title: impl Into<String>) {
        Window::set_title(title);
    }

    #[cfg(desktop)]
    pub fn set_window_size(size: impl Into<Size<u32>> + Send + 'static) {
        let size = size.into();

        from_main(move || {
            Window::current().set_size(size);
        });

        if is_main_thread() {
            return;
        }

        // In windowed mode the OS applies the resize later. A touch injected
        // before it lands is processed against the old layout and misses
        // every view. Wait until the new size is real.
        for _ in 0..100 {
            let current: Size<u32> = from_main(Window::inner_size).lossy_convert();
            if current == size {
                return;
            }
            wait_for_next_frame();
        }

        panic!("Window did not resize to {size:?}");
    }

    pub fn take_screenshot() -> Result<Screenshot> {
        human_pause();

        let recv = from_main(|| Window::current().request_screenshot());
        let screenshot = recv.recv()?;
        Ok(screenshot)
    }

    pub fn fps() -> f32 {
        Window::current().fps()
    }

    /// Runs the whole UI suite from the browser page, when the
    /// `hilen_run_tests` query flag is set, since a page has no env vars.
    /// The suite runs on a worker thread sharing wasm memory, exactly
    /// like the native worker task, and the driver reads the
    /// `HILEN_TEST_RESULT` console line instead of an exit code, since a
    /// page has no exit status. `hilen_test_only` narrows the run to a
    /// comma separated list of test names, camel case only, spaces do
    /// not survive a url. `hilen_test_skip` drops tests the same way, the
    /// driver relists panicked ones there when it relaunches the page,
    /// since a wasm panic aborts the instance and cannot be caught.
    /// `hilen_human` is the browser spelling of `--human`, the driver must
    /// drop its report timeout since the run holds until space.
    #[cfg(all(wasm, feature = "ui-tests"))]
    fn spawn_test_autorun() {
        // The browser spelling of `--present`, `hilen_test_only` names the
        // one view. The page stays on that view.
        if crate::web::query_flag("hilen_present") {
            UIManager::on_app_ready(|| {
                let name = crate::web::query_param("hilen_test_only").unwrap_or_default();
                crate::deps::hreads::spawn(async move {
                    if let Err(err) = crate::assets::Assets::load_all_groups().await {
                        log::error!("Asset preload for present failed: {err}");
                    }
                    crate::deps::hreads::spawn_thread(move || {
                        if let Err(err) = crate::ui_test::present_test(&name) {
                            log::error!("hilen_present failed: {err}");
                        }
                    });
                });
            });
            return;
        }

        if !crate::web::query_flag("hilen_run_tests") {
            return;
        }

        UIManager::on_app_ready(|| {
            use std::sync::atomic::{AtomicBool, Ordering};

            // The app marks itself ready again when the suite hands the
            // UI back, and this callback refires. Native exits before
            // that, a page stays alive, so run once.
            static RAN: AtomicBool = AtomicBool::new(false);

            if RAN.swap(true, Ordering::Relaxed) {
                return;
            }

            // Read on the main thread, a worker has no window.
            if crate::web::query_flag("hilen_human") {
                crate::ui_test::enable_human_mode();
            }

            let only = crate::web::query_param("hilen_test_only");
            let skip = crate::web::query_param("hilen_test_skip");

            // A browser serves sync `get` only from memory, so every
            // asset group downloads before the suite starts. Native
            // reads any file from disk on demand, this keeps both
            // runs seeing the same assets. A miss is fatal, a suite on
            // the default font fails on pixels far from the cause.
            crate::deps::hreads::spawn(async move {
                if let Err(err) = crate::assets::Assets::load_all_groups().await {
                    panic!("Asset preload for tests failed: {err}");
                }

                Self::spawn_test_worker(only, skip);
            });
        });
    }

    #[cfg(all(wasm, feature = "ui-tests"))]
    fn spawn_test_worker(only: Option<String>, skip: Option<String>) {
        crate::deps::hreads::spawn_thread(move || {
            use std::collections::BTreeMap;

            let mut tests = crate::ui_test::registered_test_maps();

            if let Some(only) = &only {
                crate::ui_test::keep_named(&mut tests, only);
            }

            // Checked before the skip pass, a test the filter matched but a
            // panic rerun skips is already counted failed by the driver.
            let filter_matched_nothing = only.is_some() && tests.iter().all(BTreeMap::is_empty);

            // The driver reports skipped tests as failures itself, this
            // rerun only has to survive them.
            if let Some(skip) = skip {
                crate::ui_test::drop_named(&mut tests, &skip);
            }

            // A filter with a typo must fail loudly, a green `0 tests`
            // report is indistinguishable from a pass.
            let report = if filter_matched_nothing {
                crate::ui_test::TestRunReport {
                    total:    0,
                    failures: vec![crate::ui_test::TestFailure {
                        name:   only.unwrap_or_default(),
                        detail: "hilen_test_only matched no registered tests".to_string(),
                    }],
                }
            } else {
                crate::ui_test::run_test_maps(&tests)
            };

            for failure in &report.failures {
                log::error!("TEST FAILED: {}\n{}", failure.name, failure.detail);
            }

            let failed = report.failures.len();
            log::info!("HILEN_TEST_RESULT {} tests, {failed} failed", report.total);

            // A driver has no console access without a browser automation
            // protocol, it reads the report over the inspect socket instead.
            #[cfg(feature = "inspect")]
            crate::inspect::web_transport::push(crate::inspect::AppCommand::TestResults {
                total:    report.total,
                failures: report
                    .failures
                    .into_iter()
                    .map(|f| crate::inspect::protocol::TestFailureRepr {
                        name:   f.name,
                        detail: f.detail,
                    })
                    .collect(),
            });
        });
    }

    /// Runs the whole UI suite and exits, when `HILEN_RUN_TESTS` is set.
    ///
    /// The tests drive the main thread through `from_main`, so the run has to
    /// live on a worker task while the main loop keeps pumping. That is the
    /// same reason `InspectService` runs `run_all_tests` off the main
    /// thread. This exists so a simulator or device run is a single launch
    /// with an exit code, no inspector connection and no mDNS to
    /// disambiguate.
    #[cfg(all(not_wasm, feature = "ui-tests"))]
    fn spawn_test_autorun() {
        use std::process::exit;

        // The device spelling of `--present`, `HILEN_TEST_ONLY` names the
        // one view. The app stays up with that view until it is killed.
        if std::env::var("HILEN_PRESENT").is_ok() {
            UIManager::on_app_ready(|| {
                crate::deps::hreads::spawn(async {
                    let name = std::env::var("HILEN_TEST_ONLY").unwrap_or_default();
                    if let Err(err) = crate::ui_test::present_test(&name) {
                        println!("HILEN_PRESENT failed: {err}");
                        exit(1);
                    }
                    println!("HILEN_PRESENT {name}");
                });
            });
            return;
        }

        if std::env::var("HILEN_RUN_TESTS").is_err() {
            return;
        }

        // Wait for the app to finish any async startup before running. An app
        // can swap a loading screen for its real UI once assets land, and
        // tearing that root down mid load frees views the load task still
        // touches. An app with no loading phase is ready at once.
        // The device spelling of `--human`, see the browser `hilen_human`
        // above.
        if std::env::var("HILEN_HUMAN").is_ok() {
            crate::ui_test::enable_human_mode();
        }

        UIManager::on_app_ready(|| {
            crate::deps::hreads::spawn(async {
                let mut tests = crate::ui_test::registered_test_maps();

                // Run only the named tests when set, a comma separated list, to
                // isolate cases on a device or simulator where the whole suite
                // is slow to reach them. Order in each map is still
                // alphabetical.
                if let Ok(only) = std::env::var("HILEN_TEST_ONLY") {
                    crate::ui_test::keep_named(&mut tests, &only);
                }

                let report = crate::ui_test::run_test_maps(&tests);

                for failure in &report.failures {
                    println!("TEST FAILED: {}\n{}", failure.name, failure.detail);
                }

                let failed = report.failures.len();
                println!("HILEN_TEST_RESULT {} tests, {failed} failed", report.total);

                exit(i32::from(failed != 0));
            });
        });
    }
}

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
            #[cfg(feature = "level")]
            {
                *LevelManager::update_interval() = 1.0 / Window::display_refresh_rate().lossy_convert();
            }

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
        UIDrawer::update();
        // After layout, so a row that replaced the dead hovered one
        // already has its frame when hover re-picks.
        #[cfg(any(desktop, wasm))]
        Hover::refresh_dead();
    }

    #[cfg(feature = "scene")]
    fn prepare(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if UIManager::window_resolution().has_no_area() {
            return;
        }

        SceneDrawer::prepare(encoder);
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
        Input::on_scroll(delta * SCROLL_SPEED);
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
        }
    }

    fn touch_event(&mut self, touch: winit::event::Touch) -> bool {
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
            return;
        }

        #[cfg(not_wasm)]
        if let winit::keyboard::PhysicalKey::Code(code) = event.physical_key {
            use winit::keyboard::KeyCode;

            use crate::bug_report::{BugReport, InputRing};

            InputRing::record(code);

            let modifiers = InputRing::modifiers();

            if code == KeyCode::KeyR
                && modifiers.shift_key()
                && (modifiers.control_key() || modifiers.super_key())
            {
                BugReport::open();
            }
        }

        if let Key::Named(key) = event.logical_key {
            Input::on_key(key);
        }

        if let Some(ch) = event.logical_key.to_text() {
            Input::on_char(ch.chars().last().unwrap());
        }
    }

    fn modifiers_changed(&mut self, modifiers: winit::event::Modifiers) {
        Input::set_modifiers(modifiers.state());
        #[cfg(not_wasm)]
        crate::bug_report::InputRing::set_modifiers(modifiers.state());
    }

    fn dropped_file(&mut self, path: PathBuf) {
        UIManager::trigger_drop_file(path);
    }

    fn theme_changed(&mut self, theme: OsTheme) {
        Theme::set_system(theme.into());
    }
}
