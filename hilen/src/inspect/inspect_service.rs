use std::sync::OnceLock;
#[cfg(not_wasm)]
use std::{env::var, hint::black_box, io::ErrorKind, time::Duration};

use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
#[cfg(not_wasm)]
use log::{debug, info, warn};
#[cfg(all(not_wasm, not(apple)))]
use mdns_sd::{ServiceDaemon, ServiceInfo};
#[cfg(ios)]
use objc2_foundation::NSBundle;
#[cfg(not_wasm)]
use tokio::{net::TcpListener, time::sleep};

#[cfg(not_wasm)]
use crate::deps::hreads::log_spawn;
#[cfg(any(wasm, sound))]
use crate::deps::hreads::on_main;
#[cfg(not_wasm)]
use crate::filesystem::Paths;
#[cfg(apple)]
use crate::inspect::bonjour;
#[cfg(hot)]
use crate::inspect::hot_swap;
#[cfg(all(not_wasm, not(apple)))]
use crate::inspect::protocol::SERVICE_TYPE;
#[cfg(not_wasm)]
use crate::inspect::{
    MARKER,
    protocol::{FIXED_PORT, serve},
};
#[cfg(not_wasm)]
use crate::ui::UIManager;
use crate::{
    app::hilen_app_build_time,
    deps::hreads::from_main,
    inspect::{
        edit_log,
        protocol::{AppCommand, InspectorCommand, TestFailureRepr},
        window,
    },
    window::Screenshot,
};
#[cfg(sound)]
use crate::{audio::Sound, deps::refs::manage::DataManager};

pub struct InspectService;

static APP_STARTED: OnceLock<u64> = OnceLock::new();

/// How many times a start asks for the fixed port. A hot build starts right
/// after the stop of the build before it, whose listener may need a moment
/// to let the port go, and a tool on a VPN finds the new build only there.
#[cfg(all(not_wasm, hot))]
const PORT_TRIES: u32 = 10;
#[cfg(all(not_wasm, not(hot)))]
const PORT_TRIES: u32 = 1;
#[cfg(not_wasm)]
const PORT_WAIT: Duration = Duration::from_millis(100);

/// The address the server listens on. Xcode reaches an iPhone, an iPad and
/// an Apple TV over a tunnel that carries only IPv6, and `hilen-inspect
/// --device` asks the app there. `::` takes IPv4 too on these systems. On
/// Windows it would take IPv6 only.
#[cfg(all(not_wasm, any(target_os = "ios", target_os = "tvos")))]
const ANY: &str = "::";
#[cfg(all(not_wasm, not(any(target_os = "ios", target_os = "tvos"))))]
const ANY: &str = "0.0.0.0";

/// The listener of the server, on `port` when it is free and on a free
/// port of the system when another app has it.
#[cfg(not_wasm)]
async fn bind(port: u16, tries: u32) -> Result<TcpListener> {
    bind_on(ANY, port, tries).await
}

#[cfg(not_wasm)]
async fn bind_on(address: &str, port: u16, tries: u32) -> Result<TcpListener> {
    for attempt in 1..=tries {
        let result = if taken_elsewhere(port).await {
            Err(ErrorKind::AddrInUse.into())
        } else {
            TcpListener::bind((address, port)).await
        };
        match result {
            Ok(listener) => return Ok(listener),
            Err(err) if err.kind() == ErrorKind::AddrInUse => {
                if attempt < tries {
                    sleep(PORT_WAIT).await;
                }
            }
            Err(err) => return Err(err.into()),
        }
    }
    debug!("Inspect port {port} is taken by another app, a free one is used");
    Ok(TcpListener::bind((address, 0)).await?)
}

/// Whether another app holds `port` on IPv4 or on IPv6. An app in the iOS
/// simulator listens on `::` of the Mac, a Mac app on `0.0.0.0`, and macOS
/// lets both have the same port. A tool at 127.0.0.1 then reached the Mac
/// app and never the one in the simulator.
#[cfg(not_wasm)]
async fn taken_elsewhere(port: u16) -> bool {
    for address in ["0.0.0.0", "::"] {
        // A system with no IPv6 answers with another error, the port is
        // not taken there.
        if let Err(err) = TcpListener::bind((address, port)).await
            && err.kind() == ErrorKind::AddrInUse
        {
            return true;
        }
    }
    false
}

/// A build without debug assertions starts the server only with this set to
/// `1` at launch, so a release binary built outside the release scripts still
/// ships with the server off.
#[cfg(not_wasm)]
const ENABLE_ENV: &str = "HILEN_INSPECT";

/// The 1 app that runs the server in every build, the engine's own demo.
/// It ships only through `TestFlight`, to be inspected on a phone, and an
/// app that is hot loaded into it runs under this id too. No other app may
/// be added here, a shipped app must never carry a server that is on.
#[cfg(not_wasm)]
const ALWAYS_ON_BUNDLE_ID: &str = "vladas.test-engine";

#[cfg(not_wasm)]
fn listener_allowed(debug_build: bool, enable: Option<&str>, bundle_id: Option<&str>) -> bool {
    debug_build || enable == Some("1") || bundle_id == Some(ALWAYS_ON_BUNDLE_ID)
}

/// The bundle id of the running app, only an iOS app is asked.
#[cfg(not_wasm)]
fn bundle_id() -> Option<String> {
    #[cfg(ios)]
    {
        NSBundle::mainBundle().bundleIdentifier().map(|id| id.to_string())
    }
    #[cfg(not(ios))]
    {
        None
    }
}

impl InspectService {
    pub(crate) fn record_app_start() {
        APP_STARTED.get_or_init(current_unix_seconds);
    }

    /// Takes the app off the network. The listener and its clients are
    /// tasks, they end with the runtime.
    #[cfg(hot)]
    pub(crate) fn stop() {
        bonjour::stop();
    }

    #[cfg(not_wasm)]
    pub(crate) fn start_listening() {
        black_box(MARKER);

        // A hot build is a release build and never ships, the loader takes
        // its next library over this server.
        let debug_build = cfg!(debug_assertions) || cfg!(hot);
        if !listener_allowed(
            debug_build,
            var(ENABLE_ENV).ok().as_deref(),
            bundle_id().as_deref(),
        ) {
            info!("Inspect server is off in a release build, set {ENABLE_ENV}=1 to start it");
            return;
        }
        if !debug_build {
            warn!(
                "Inspect server is on in a release build. Anyone on the network can read and drive this app."
            );
        }

        log_spawn(async {
            let listener = bind(FIXED_PORT, PORT_TRIES).await?;
            let port = listener.local_addr()?.port();

            let app_id = UIManager::app_instance_id();

            // An iPhone lets only its own Bonjour service announce, and on
            // a Mac the system owns the mDNS port.
            #[cfg(apple)]
            bonjour::announce(app_id, &Self::app_name(), port)?;
            // The daemon lives to the end of this task, like the listener.
            #[cfg(not(apple))]
            let mdns = ServiceDaemon::new()?;
            #[cfg(not(apple))]
            mdns.register(
                ServiceInfo::new(
                    SERVICE_TYPE,
                    app_id,
                    &format!("{app_id}.local."),
                    "",
                    port,
                    &[("app_id", app_id), ("name", Self::app_name().as_str())][..],
                )?
                .enable_addr_auto(),
            )?;

            info!("Inspect server on port: {port}");

            serve(listener, Self::process_command).await
        });
    }

    /// The name an inspector shows next to the id of this app. It is the
    /// name of the exe, a test runner has no project name to tell.
    #[cfg(not_wasm)]
    fn app_name() -> String {
        Paths::executable_name()
    }

    #[cfg(hot)]
    fn hot(result: Result<AppCommand>) -> AppCommand {
        result.unwrap_or_else(|err| AppCommand::Error(format!("{err:#}")))
    }

    pub fn process_command(command: InspectorCommand) -> AppCommand {
        match command {
            #[cfg(sound)]
            InspectorCommand::PlaySound => {
                on_main(|| {
                    Sound::get("retro.wav").play();
                });

                AppCommand::Ok
            }
            #[cfg(not(sound))]
            InspectorCommand::PlaySound => AppCommand::Error("App built without the audio feature".into()),
            InspectorCommand::Screenshot => match Self::screenshot() {
                Ok(screenshot) => screenshot,
                Err(err) => AppCommand::Error(format!("Screenshot failed: {err}")),
            },
            InspectorCommand::ListEdits => AppCommand::Edits(edit_log::all()),
            InspectorCommand::RunTests => Self::run_tests(None),
            InspectorCommand::RunTestsNamed { names } => Self::run_tests(Some(&names)),
            // Compiled in, so it reports when this running code was built, not
            // when the bundle around it was linked. See `hilen_app_build_time`.
            InspectorCommand::GetBuildTime => AppCommand::BuildTime(hilen_app_build_time()),
            InspectorCommand::GetStartTime => {
                AppCommand::StartTime(*APP_STARTED.get().expect("App start time was not recorded"))
            }
            InspectorCommand::Quit => Self::quit(),
            #[cfg(feature = "login")]
            InspectorCommand::DevLogin { email } => Self::dev_login(email),
            #[cfg(not(feature = "login"))]
            InspectorCommand::DevLogin { .. } => {
                AppCommand::Error("App built without the login feature".into())
            }
            // The TCP transport runs it on the connection and never asks here.
            InspectorCommand::Speed { .. } => {
                AppCommand::Error("The speed test needs a TCP connection to the app".into())
            }
            InspectorCommand::Record {
                frames,
                input,
                wait_ms,
            } => Self::record(frames, input, wait_ms),
            InspectorCommand::RecordStart => Self::record_start(),
            InspectorCommand::RecordStop => Self::record_stop(),
            InspectorCommand::RecordInfo => Self::record_info(),
            InspectorCommand::RecordFrames { from, count } => Self::record_frames(from, count),
            InspectorCommand::RecordLayouts => Self::record_layouts(),
            InspectorCommand::Pause => Self::pause(),
            InspectorCommand::Step { frames } => Self::step(frames),
            InspectorCommand::Resume => Self::resume(),
            #[cfg(hot)]
            InspectorCommand::HotInfo => Self::hot(hot_swap::info()),
            #[cfg(hot)]
            InspectorCommand::HotFiles { root } => Self::hot(hot_swap::files(&root)),
            #[cfg(hot)]
            InspectorCommand::HotChunk {
                file,
                offset,
                data_base64,
            } => Self::hot(hot_swap::chunk(&file, offset, &data_base64)),
            #[cfg(hot)]
            InspectorCommand::HotBytes {
                file, offset, data, ..
            } => Self::hot(hot_swap::piece(&file, offset, &data)),
            #[cfg(hot)]
            InspectorCommand::HotSwap { library, root } => {
                Self::hot(hot_swap::swap(&library, root.as_deref()))
            }
            #[cfg(not(hot))]
            InspectorCommand::HotInfo
            | InspectorCommand::HotFiles { .. }
            | InspectorCommand::HotChunk { .. }
            | InspectorCommand::HotBytes { .. }
            | InspectorCommand::HotSwap { .. } => AppCommand::Error("This app is not a hot build".into()),
            InspectorCommand::App { name, args } => {
                match from_main(move || crate::app_commands::run(&name, &args)) {
                    Ok(text) => AppCommand::Answer(text),
                    Err(err) => AppCommand::Error(err),
                }
            }
            InspectorCommand::Windows => Self::windows(),
            InspectorCommand::InWindow { window, command } => Self::in_window(window, *command),
            InspectorCommand::UI(ui) => {
                Self::lay_out_covered();
                Self::process_ui_command(ui)
            }
        }
    }

    /// Blocks the inspector worker until the server answered, the main
    /// thread keeps drawing.
    #[cfg(feature = "login")]
    fn dev_login(email: String) -> AppCommand {
        const WAIT: std::time::Duration = std::time::Duration::from_secs(30);

        let (send, recv) = std::sync::mpsc::channel();
        crate::deps::hreads::on_main(move || {
            crate::login::Login::dev(email, move |result| {
                // The worker gave up after `WAIT` when nobody listens.
                send.send(result).ok();
            });
        });

        match recv.recv_timeout(WAIT) {
            Ok(Ok(user)) => AppCommand::LoggedIn {
                id:    user.id,
                email: user.email,
            },
            Ok(Err(error)) => AppCommand::Error(format!("{error:#}")),
            Err(error) => AppCommand::Error(format!("The dev login gave no answer: {error}")),
        }
    }

    #[cfg(desktop)]
    fn quit() -> AppCommand {
        crate::AppRunner::stop();
        AppCommand::Ok
    }

    #[cfg(not(desktop))]
    fn quit() -> AppCommand {
        AppCommand::Error("Quit works on desktop only".into())
    }

    // Runs off the main thread, on a tokio task natively and on the inspect
    // worker on wasm, which is what the tests need since they drive the main
    // thread through `from_main`.
    fn run_tests(names: Option<&str>) -> AppCommand {
        let mut maps = crate::ui_test::registered_test_maps();
        if let Some(names) = names {
            crate::ui_test::keep_named(&mut maps, names);
            if maps.iter().all(std::collections::BTreeMap::is_empty) {
                return AppCommand::Error(format!("No test of this app is named {names}"));
            }
        }

        #[cfg(wasm)]
        if let Err(err) = Self::preload_assets() {
            return AppCommand::Error(format!("Asset preload for tests failed: {err}"));
        }

        let report = crate::ui_test::run_test_maps(&maps);

        AppCommand::TestResults {
            total:    report.total,
            failures: report
                .failures
                .into_iter()
                .map(|f| TestFailureRepr {
                    name:   f.name,
                    detail: f.detail,
                })
                .collect(),
        }
    }

    /// A browser serves sync asset `get` only from memory, so every group
    /// downloads before the suite starts, same as the test autorun. Blocks the
    /// calling worker while the download runs on the main thread.
    #[cfg(wasm)]
    fn preload_assets() -> Result<()> {
        let (send, recv) = std::sync::mpsc::channel();

        on_main(move || {
            crate::deps::hreads::spawn(async move {
                let result = crate::assets::Assets::load_all_groups().await;
                send.send(result).expect("Asset preload signal receiver is gone");
            });
        });

        recv.recv().expect("Asset preload signal sender is gone")
    }

    fn screenshot() -> Result<AppCommand> {
        let shot = match window::target() {
            Some(window) => window.take_screenshot()?,
            None => crate::AppRunner::take_screenshot()?,
        };
        let png = Self::encode_png(&shot)?;

        Ok(AppCommand::Screenshot {
            width:      shot.size.width,
            height:     shot.size.height,
            png_base64: STANDARD.encode(&png),
        })
    }

    /// The frame as PNG bytes, what the screenshot command answers with and
    /// what a browser test failure pushes to the driver.
    pub(crate) fn encode_png(shot: &Screenshot) -> Result<Vec<u8>> {
        let mut bytes = Vec::with_capacity(shot.data.len() * 4);
        for color in &shot.data {
            bytes.extend_from_slice(&[color.r, color.g, color.b, 255]);
        }

        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(
            &bytes,
            shot.size.width,
            shot.size.height,
            ExtendedColorType::Rgba8,
        )?;

        Ok(png)
    }

    /// A covered window draws no frame, and a frame is where layout runs, so
    /// a screen made by the last command still has every view at the
    /// origin. A tap there once pressed the view that sat at 0, 0. A
    /// screenshot is the one frame a covered window draws, see
    /// `frame_pacing`, so one is taken and dropped.
    #[cfg(not_wasm)]
    pub(super) fn lay_out_covered() {
        // An extra window draws whenever the main one asks for frames.
        if window::target().is_none()
            && crate::window::occluded()
            && let Err(err) = crate::AppRunner::take_screenshot()
        {
            warn!("No frame for the covered window, view positions can be old: {err}");
        }
    }

    #[cfg(wasm)]
    pub(super) fn lay_out_covered() {}
}

fn current_unix_seconds() -> u64 {
    #[cfg(not_wasm)]
    {
        use std::time::{SystemTime, UNIX_EPOCH};

        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System clock is before the unix epoch")
            .as_secs()
    }
    #[cfg(wasm)]
    {
        use crate::gm::LossyConvert;

        crate::deps::hreads::now().lossy_convert()
    }
}

#[cfg(all(test, not_wasm))]
mod test {
    use anyhow::Result;
    use tokio::{net::TcpListener, runtime::Builder};

    use super::{bind, bind_on, listener_allowed};

    #[test]
    fn the_first_app_gets_the_port_and_the_next_one_a_free_port() -> Result<()> {
        Builder::new_current_thread().enable_all().build()?.block_on(async {
            let first = TcpListener::bind("0.0.0.0:0").await?;
            let port = first.local_addr()?.port();

            let second = bind(port, 2).await?;
            assert_ne!(second.local_addr()?.port(), port);

            drop(first);
            let third = bind(port, 1).await?;
            assert_eq!(third.local_addr()?.port(), port);
            Ok(())
        })
    }

    // An app in the iOS simulator listens on `::` of the Mac. macOS let it
    // take a port a Mac app held on IPv4, and a tool at 127.0.0.1 found the
    // Mac app.
    #[test]
    fn a_port_held_on_ipv4_is_not_taken_on_ipv6() -> Result<()> {
        Builder::new_current_thread().enable_all().build()?.block_on(async {
            let held = TcpListener::bind("0.0.0.0:0").await?;
            let port = held.local_addr()?.port();

            let next = bind_on("::", port, 1).await?;
            assert_ne!(next.local_addr()?.port(), port);
            Ok(())
        })
    }

    #[test]
    fn debug_build_always_listens() {
        assert!(listener_allowed(true, None, None));
    }

    #[test]
    fn release_build_of_the_demo_always_listens() {
        assert!(listener_allowed(false, None, Some("vladas.test-engine")));
        assert!(!listener_allowed(false, None, Some("vladas.test-engine.hot")));
        assert!(!listener_allowed(false, None, Some("vladas.skaityk")));
    }

    #[test]
    fn release_build_listens_only_when_asked() {
        assert!(!listener_allowed(false, None, None));
        assert!(!listener_allowed(false, Some(""), None));
        assert!(!listener_allowed(false, Some("0"), None));
        assert!(!listener_allowed(false, Some("true"), None));
        assert!(listener_allowed(false, Some("1"), None));
    }
}
