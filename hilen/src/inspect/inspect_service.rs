use std::sync::OnceLock;
#[cfg(not_wasm)]
use std::{env::var, hint::black_box};

use anyhow::Result;
use base64::{Engine, engine::general_purpose::STANDARD};
#[cfg(not_wasm)]
use chrono::Local;
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
#[cfg(not_wasm)]
use log::{info, warn};
#[cfg(all(not_wasm, not(ios)))]
use mdns_sd::{ServiceDaemon, ServiceInfo};
#[cfg(ios)]
use objc2_foundation::NSBundle;
#[cfg(not_wasm)]
use tokio::net::TcpListener;

#[cfg(not_wasm)]
use crate::deps::hreads::log_spawn;
#[cfg(any(wasm, feature = "audio"))]
use crate::deps::hreads::on_main;
#[cfg(ios)]
use crate::inspect::bonjour;
#[cfg(hot)]
use crate::inspect::hot_swap;
#[cfg(all(not_wasm, not(ios)))]
use crate::inspect::protocol::SERVICE_TYPE;
#[cfg(not_wasm)]
use crate::inspect::{MARKER, protocol::serve};
use crate::{
    app::hilen_app_build_time,
    deps::hreads::from_main,
    gm::flat::Point,
    inspect::{
        edit_log,
        protocol::{AppCommand, EditEntry, InspectorCommand, Key, TestFailureRepr, UIRequest, UIResponse},
        view_conversion::{ViewToInspect, weak_to_id},
    },
    ui::{
        Button, Input, Label, ModifiersState, TextField, Touch, TouchEvent, UIManager, ViewData, ViewFrame,
        ViewSubviews, WeakView,
    },
    window::{MouseButton, Screenshot},
};
#[cfg(feature = "audio")]
use crate::{audio::Sound, deps::refs::manage::DataManager};

pub struct InspectService;

static APP_STARTED: OnceLock<u64> = OnceLock::new();

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

        let debug_build = cfg!(debug_assertions);
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
            let listener = TcpListener::bind("0.0.0.0:0").await?;
            let port = listener.local_addr()?.port();

            let app_id = UIManager::app_instance_id();

            // An iPhone lets only its own Bonjour service announce.
            #[cfg(ios)]
            bonjour::announce(app_id, port)?;
            // The daemon lives to the end of this task, like the listener.
            #[cfg(not(ios))]
            let mdns = ServiceDaemon::new()?;
            #[cfg(not(ios))]
            mdns.register(
                ServiceInfo::new(
                    SERVICE_TYPE,
                    app_id,
                    &format!("{app_id}.local."),
                    "",
                    port,
                    &[("app_id", app_id)][..],
                )?
                .enable_addr_auto(),
            )?;

            info!("Inspect server on port: {port}");

            serve(listener, Self::process_command).await
        });
    }

    #[cfg(hot)]
    fn hot(result: Result<AppCommand>) -> AppCommand {
        result.unwrap_or_else(|err| AppCommand::Error(format!("{err:#}")))
    }

    pub fn process_command(command: InspectorCommand) -> AppCommand {
        match command {
            #[cfg(feature = "audio")]
            InspectorCommand::PlaySound => {
                on_main(|| {
                    Sound::get("retro.wav").play();
                });

                AppCommand::Ok
            }
            #[cfg(not(feature = "audio"))]
            InspectorCommand::PlaySound => AppCommand::Error("App built without the audio feature".into()),
            InspectorCommand::Screenshot => match Self::screenshot() {
                Ok(screenshot) => screenshot,
                Err(err) => AppCommand::Error(format!("Screenshot failed: {err}")),
            },
            InspectorCommand::ListEdits => AppCommand::Edits(edit_log::all()),
            InspectorCommand::RunTests => Self::run_tests(),
            // Compiled in, so it reports when this running code was built, not
            // when the bundle around it was linked. See `hilen_app_build_time`.
            InspectorCommand::GetBuildTime => AppCommand::BuildTime(hilen_app_build_time()),
            InspectorCommand::GetStartTime => {
                AppCommand::StartTime(*APP_STARTED.get().expect("App start time was not recorded"))
            }
            InspectorCommand::Quit => Self::quit(),
            InspectorCommand::Record {
                frames,
                input,
                wait_ms,
            } => Self::record(frames, input, wait_ms),
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
            InspectorCommand::HotSwap { library, root } => {
                Self::hot(hot_swap::swap(&library, root.as_deref()))
            }
            #[cfg(not(hot))]
            InspectorCommand::HotInfo
            | InspectorCommand::HotFiles { .. }
            | InspectorCommand::HotChunk { .. }
            | InspectorCommand::HotSwap { .. } => AppCommand::Error("This app is not a hot build".into()),
            InspectorCommand::UI(ui) => {
                Self::lay_out_covered();
                Self::process_ui_command(ui)
            }
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
    fn run_tests() -> AppCommand {
        #[cfg(wasm)]
        if let Err(err) = Self::preload_assets() {
            return AppCommand::Error(format!("Asset preload for tests failed: {err}"));
        }

        let report = crate::ui_test::run_all_tests();

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
        let shot = crate::AppRunner::take_screenshot()?;
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
        if crate::window::occluded()
            && let Err(err) = crate::AppRunner::take_screenshot()
        {
            warn!("No frame for the covered window, view positions can be old: {err}");
        }
    }

    #[cfg(wasm)]
    pub(super) fn lay_out_covered() {}

    pub(super) fn process_ui_command(command: UIRequest) -> AppCommand {
        match command {
            UIRequest::SetScale(scale) => {
                from_main(move || {
                    UIManager::set_scale(scale);
                });

                // send_ui dispatches a fresh from_main, so the snapshot runs
                // one frame later, after layout applied the new scale.
                Self::send_ui()
            }
            UIRequest::GetUI => Self::send_ui(),
            UIRequest::EditRule {
                view_id,
                rule_index,
                offset,
                enabled,
            } => Self::apply(move || {
                let view = find_view(&view_id)?;

                let rules_count = view.place().get_rules().len();
                if rule_index >= rules_count {
                    return Err(format!(
                        "Rule index {rule_index} is out of range, view has {rules_count} rules"
                    ));
                }

                let mut rule = view.place().edit_rule(rule_index);
                let old = format!("offset: {}, enabled: {}", rule.offset(), rule.enabled);
                rule.set_offset(offset);
                rule.enabled = enabled;

                Ok(entry(
                    view,
                    format!("rule {rule_index}"),
                    old,
                    format!("offset: {offset}, enabled: {enabled}"),
                ))
            }),
            UIRequest::SetText { view_id, text } => Self::apply(move || {
                let view = find_view(&view_id)?;
                let old = set_text(view, &text)?;
                Ok(entry(view, "text", old, text))
            }),
            UIRequest::SetColor { view_id, color } => Self::apply(move || {
                let view = find_view(&view_id)?;
                let old = *view.color();
                view.set_color(color);
                Ok(entry(view, "color", format!("{old}"), format!("{color}")))
            }),
            UIRequest::Tap {
                view_id,
                modifiers,
                right,
                force,
            } => match from_main(move || Self::tap(&view_id, modifiers, right, force)) {
                // The snapshot runs a frame later, so a page swap or modal
                // the tap triggered is already in the tree.
                Ok(note) => Self::send_ui_with(note),
                Err(err) => AppCommand::Error(err),
            },
            #[cfg(any(desktop, wasm))]
            UIRequest::Hover { view_id, wait_ms } => Self::hover(view_id, wait_ms),
            #[cfg(not(any(desktop, wasm)))]
            UIRequest::Hover { .. } => AppCommand::Error("Hover needs a pointer on desktop or web".into()),
            UIRequest::Drag { from, to, steps } => {
                from_main(move || Self::drag(from.into(), to.into(), steps));
                Self::send_ui()
            }
            UIRequest::Scroll { view_id, dx, dy } => match from_main(move || Self::scroll(view_id, dx, dy)) {
                Ok(()) => Self::send_ui(),
                Err(err) => AppCommand::Error(err),
            },
            #[cfg(desktop)]
            UIRequest::Resize { width, height } => Self::resize(width, height),
            #[cfg(not(desktop))]
            UIRequest::Resize { .. } => AppCommand::Error("Resize works only on desktop".into()),
            UIRequest::Hold { keys, ms } => Self::hold(keys, ms),
            UIRequest::Keys { keys, modifiers } => {
                // The modifiers hold only for this one request and release
                // right after it, so a Cmd from an inspect request can never
                // stay stuck on whatever the app receives next.
                from_main(move || {
                    Input::set_modifiers(modifiers);
                    for key in keys {
                        match key {
                            Key::Char(ch) => Input::on_char(ch),
                            // A real named key also arrives as its text,
                            // Backspace as 0x08, and the text field edits
                            // on that char. Same two calls as the winit
                            // key handler in `app_runner`.
                            Key::Named(key) => {
                                Input::on_key(key);
                                if let Some(text) = key.to_text() {
                                    Input::on_char(text.chars().last().expect("Key text is empty"));
                                }
                            }
                        }
                    }
                    Input::set_modifiers(ModifiersState::empty());
                });

                // The snapshot runs a frame later, so a palette or page a
                // hotkey opened is already in the tree.
                Self::send_ui()
            }
        }
    }

    fn scroll(view_id: Option<String>, dx: f32, dy: f32) -> Result<(), String> {
        let position = if let Some(id) = view_id {
            find_view(&id)?.absolute_frame().center()
        } else {
            let window = UIManager::root_view().frame().size;
            Point::new(window.width / 2.0, window.height / 2.0)
        };
        // The wheel goes to the scroll view under the cursor, so the
        // cursor moves to the aim point first.
        UIManager::set_cursor_position(position);
        Input::on_scroll(Point::new(dx, dy));
        Ok(())
    }

    // The touch pipeline takes physical pixels, so every point scales up.
    fn drag(from: Point, to: Point, steps: usize) {
        use crate::gm::LossyConvert;

        let scale = UIManager::scale();
        let touch = |position: Point, event: TouchEvent| {
            Input::process_touch_event(Touch {
                id: 1,
                position: position * scale,
                event,
                button: MouseButton::Left,
            });
        };
        touch(from, TouchEvent::Began);
        let steps = steps.max(2);
        for step in 1..=steps {
            let t: f32 = step.lossy_convert();
            let all: f32 = steps.lossy_convert();
            let position = Point {
                x: from.x + (to.x - from.x) * t / all,
                y: from.y + (to.y - from.y) * t / all,
            };
            touch(position, TouchEvent::Moved);
        }
        touch(to, TouchEvent::Ended);
    }

    /// The tap itself, on the main thread. Refuses hidden and offscreen
    /// targets, since such a tap lands nowhere while looking like a
    /// success to the client. Refuses a tap point another view sits over
    /// too, that touch once pressed a reset button, `force` sends it
    /// anyway and the reply then carries the covering warning.
    fn tap(
        view_id: &str,
        modifiers: ModifiersState,
        right: bool,
        force: bool,
    ) -> Result<Option<String>, String> {
        let view = find_view(view_id)?;

        if view.is_hidden_in_tree() {
            return Err(format!("View {} is hidden", view.label()));
        }

        let center = view.absolute_frame().center();
        let window = UIManager::root_view().frame().size;
        if center.x < 0.0 || center.y < 0.0 || center.x > window.width || center.y > window.height {
            return Err(format!(
                "View {} center ({}, {}) is outside the {}x{} window. Scroll it into view or resize the window.",
                view.label(),
                center.x,
                center.y,
                window.width,
                window.height,
            ));
        }
        if !view.contains_visible(center) {
            return Err(format!(
                "View {} center is cut off by a scroll view. Scroll it into view first.",
                view.label()
            ));
        }

        let note = covering_note(view, center);
        if let Some(note) = &note
            && !force
        {
            return Err(format!(
                "View {} is not tapped, {note}. Pass --force to send the touch anyway.",
                view.label()
            ));
        }

        // The touch pipeline takes physical pixels and converts to
        // points itself, so the center scales up first.
        let position = center * UIManager::scale();

        // Held for this tap only and released right after, like the keys
        // request, so a Cmd cannot leak into later input.
        Input::set_modifiers(modifiers);
        let button = if right {
            MouseButton::Right
        } else {
            MouseButton::Left
        };
        for event in [TouchEvent::Began, TouchEvent::Ended] {
            Input::process_touch_event(Touch {
                id: 1,
                position,
                event,
                button,
            });
        }
        Input::set_modifiers(ModifiersState::empty());

        Ok(note)
    }

    // Applies an edit on the main thread. On success records it to the edit
    // log and replies with a fresh tree. send_ui dispatches its own
    // from_main, so the snapshot runs one frame later, after layout.
    fn apply(edit: impl FnOnce() -> Result<EditEntry, String> + Send + 'static) -> AppCommand {
        match from_main(edit) {
            Ok(entry) => {
                edit_log::record(entry);
                Self::send_ui()
            }
            Err(err) => AppCommand::Error(err),
        }
    }

    /// A size the OS does not give, like one above the screen, is an
    /// error answer with the size the window has instead.
    #[cfg(desktop)]
    fn resize(width: f32, height: f32) -> AppCommand {
        use crate::gm::LossyConvert;

        let (size, scale) = from_main(move || {
            let scale = UIManager::scale();
            let size: (u32, u32) = ((width * scale).lossy_convert(), (height * scale).lossy_convert());
            (size, scale)
        });
        let Err(actual) = crate::AppRunner::request_window_size(size.into()) else {
            return Self::send_ui();
        };
        let actual_width: f32 = actual.width.lossy_convert();
        let actual_height: f32 = actual.height.lossy_convert();
        let has = format!("{} by {}", actual_width / scale, actual_height / scale);
        AppCommand::Error(format!(
            "The window did not take {width} by {height} points, it is {has}. A window cannot be bigger than the screen."
        ))
    }

    fn send_ui() -> AppCommand {
        Self::send_ui_with(None)
    }

    pub(super) fn send_ui_with(note: Option<String>) -> AppCommand {
        Self::lay_out_covered();
        from_main(move || {
            let scale = UIManager::scale();
            let root = UIManager::root_view().view_to_inspect();
            UIResponse::SendUI { scale, root, note }.into()
        })
    }
}

/// Names the deepest visible view under the tap point when it is not the
/// target or inside it, so a tap that would land on a covering view is
/// refused instead of silently doing the wrong thing. Frame containment is
/// an approximation of the draw order, `force` is the way past a wrong call.
fn covering_note(target: WeakView, point: Point) -> Option<String> {
    let top = deepest_at(UIManager::root_view(), point)?;

    if weak_to_id(top) == weak_to_id(target) || is_inside(target, top) || is_inside(top, target) {
        return None;
    }

    Some(format!(
        "the deepest view at the tap point is {} {}, the touch would land there instead",
        top.label(),
        weak_to_id(top),
    ))
}

fn deepest_at(view: WeakView, point: Point) -> Option<WeakView> {
    if view.is_hidden() || !view.absolute_frame().contains(point) {
        return None;
    }
    if let Some(deepest_child) = view.subviews().iter().rev().find_map(|sub| deepest_at(sub.weak(), point)) {
        return Some(deepest_child);
    }
    // A fully transparent view with nothing in it, like an empty overlay
    // host, draws nothing at the point and would only produce false
    // covering warnings.
    if view.color().a == 0.0 && view.subviews().is_empty() {
        return None;
    }
    Some(view)
}

fn is_inside(ancestor: WeakView, view: WeakView) -> bool {
    fn contains(view: WeakView, id: &str) -> bool {
        if weak_to_id(view) == id {
            return true;
        }
        view.subviews().iter().any(|sub| contains(sub.weak(), id))
    }

    contains(ancestor, &weak_to_id(view))
}

fn entry(view: WeakView, what: impl ToString, old: impl ToString, new: impl ToString) -> EditEntry {
    EditEntry {
        timestamp: timestamp(),
        view:      view.label().to_string(),
        view_id:   weak_to_id(view),
        what:      what.to_string(),
        old:       old.to_string(),
        new:       new.to_string(),
    }
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

fn timestamp() -> String {
    #[cfg(not_wasm)]
    {
        Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
    }
    #[cfg(wasm)]
    {
        String::from(web_sys::js_sys::Date::new_0().to_iso_string())
    }
}

fn set_text(view: WeakView, text: &str) -> Result<String, String> {
    if let Some(label) = view.downcast::<Label>() {
        let old = label.text().to_string();
        label.set_text(text);
        return Ok(old);
    }
    if let Some(button) = view.downcast::<Button>() {
        let old = button.text().to_string();
        button.set_text(text);
        return Ok(old);
    }
    if let Some(field) = view.downcast::<TextField>() {
        let old = field.text().to_string();
        field.set_text(text);
        return Ok(old);
    }
    Err(format!("View {} has no text", view.label()))
}

pub(super) fn find_view(id: &str) -> Result<WeakView, String> {
    fn search(view: WeakView, id: &str) -> Option<WeakView> {
        if weak_to_id(view) == id {
            return Some(view);
        }

        view.subviews().iter().find_map(|sub| search(sub.weak(), id))
    }

    search(UIManager::root_view(), id).ok_or_else(|| format!("View not found: {id}"))
}

#[cfg(all(test, not_wasm))]
mod test {
    use super::listener_allowed;

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
