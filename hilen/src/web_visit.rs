#![cfg(target_arch = "wasm32")]

//! What a page did on its way up, kept so that a browser that froze or
//! died can still tell about it. A GPU process that hangs or a tab that is
//! killed runs no more script, so nothing can be sent at that moment. Each
//! step of the start is written to local storage at once instead, and a
//! heartbeat keeps the time alive, the frame count and the last log lines
//! fresh. `pagehide` marks a visit that ended normally. The next start of
//! the page finds a visit that never ended and posts it to the path the
//! app returns from `App::visit_report_path`. A panic is posted at once,
//! the page still runs while the hook does. See `docs/web-visit.md`.

use std::{
    collections::VecDeque,
    fmt::Write,
    panic::PanicHookInfo,
    sync::{
        OnceLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

use log::{debug, error, info, warn};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use web_sys::wasm_bindgen::{JsCast, closure::Closure};

const KEY: &str = "hilen_visit";
const HEARTBEAT_SECS: f32 = 5.0;
const LOG_LINES: usize = 40;
const LOG_LINE_CHARS: usize = 300;

static PATH: OnceLock<String> = OnceLock::new();
static VISIT: Mutex<Option<Visit>> = Mutex::new(None);
static FRAMES: AtomicU64 = AtomicU64::new(0);
static FIRST_FRAME: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Deserialize, Default)]
struct Step {
    name: String,
    /// Since the start of the visit.
    ms:   f64,
}

#[derive(Serialize, Deserialize, Default)]
struct Visit {
    id:       String,
    /// Unix time of the start, in milliseconds.
    started:  f64,
    url:      String,
    agent:    String,
    screen:   String,
    steps:    Vec<Step>,
    /// How long the page ran, as of the last heartbeat or step.
    alive_ms: f64,
    frames:   u64,
    /// The tab was in the background at the last write. A phone can kill
    /// such a tab with no `pagehide`, so it is no proof of a crash.
    hidden:   bool,
    /// `pagehide` came, the visit ended normally.
    closed:   bool,
    log:      VecDeque<String>,
}

#[derive(Serialize)]
struct Report<'a> {
    /// `unfinished` for a visit found on the next start, `panic` for a
    /// panic sent while it happens.
    kind:  &'a str,
    panic: Option<String>,
    visit: &'a Visit,
}

fn now_ms() -> f64 {
    web_sys::js_sys::Date::now()
}

fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

fn save(visit: &mut Visit) {
    visit.alive_ms = now_ms() - visit.started;
    visit.frames = FRAMES.load(Ordering::Relaxed);
    let Some(storage) = storage() else {
        return;
    };
    match serde_json::to_string(visit) {
        Ok(text) => {
            if let Err(err) = storage.set_item(KEY, &text) {
                warn!("Failed to save the visit record: {err:?}");
            }
        }
        Err(err) => warn!("Failed to write the visit record: {err}"),
    }
}

fn with_visit(change: impl FnOnce(&mut Visit)) {
    let mut guard = VISIT.lock();
    if let Some(visit) = guard.as_mut() {
        change(visit);
        save(visit);
    }
}

/// Starts the record of this visit and sends the one of the visit before
/// when it never ended. Runs once, on the main thread of the page.
pub(crate) fn start(path: &str) {
    let Some(window) = web_sys::window() else {
        return;
    };
    if PATH.set(path.to_string()).is_err() {
        warn!("The visit report started twice, the first path stays");
        return;
    }

    if let Some(storage) = storage() {
        match storage.get_item(KEY) {
            Ok(Some(text)) => match serde_json::from_str::<Visit>(&text) {
                Ok(last) if !last.closed => {
                    let step = last.steps.last().map_or("none", |step| step.name.as_str());
                    warn!(
                        "The last visit did not end, it stopped after {step} at {:.0} ms",
                        last.alive_ms
                    );
                    send(&Report {
                        kind:  "unfinished",
                        panic: None,
                        visit: &last,
                    });
                }
                Ok(_) => debug!("The last visit ended normally"),
                Err(err) => warn!("The last visit record is unreadable, it is dropped: {err}"),
            },
            Ok(None) => debug!("No record of a last visit"),
            Err(err) => warn!("Failed to read the last visit record: {err:?}"),
        }
    }

    let screen = window.screen().ok();
    let size = |value: Result<i32, _>| value.unwrap_or_default();
    let started = now_ms();
    let random = web_sys::js_sys::Math::random().to_string();
    let mut visit = Visit {
        id: format!("{started:.0}-{}", random.trim_start_matches("0.")),
        started,
        url: window.location().href().unwrap_or_default(),
        agent: window.navigator().user_agent().unwrap_or_default(),
        screen: format!(
            "{}x{} at {}",
            screen.as_ref().map_or(0, |screen| size(screen.width())),
            screen.as_ref().map_or(0, |screen| size(screen.height())),
            window.device_pixel_ratio()
        ),
        ..Visit::default()
    };
    visit.steps.push(Step {
        name: "start".into(),
        ms:   0.0,
    });
    save(&mut visit);
    info!("Visit record {} started", visit.id);
    *VISIT.lock() = Some(visit);

    listen(&window, "pagehide", |visit| visit.closed = true);
    // A page that comes back from the back and forward cache runs on.
    listen(&window, "pageshow", |visit| visit.closed = false);
    let document = window.document();
    listen_on(
        document.as_ref().map(AsRef::as_ref),
        "visibilitychange",
        move |visit| {
            visit.hidden = web_sys::window()
                .and_then(|window| window.document())
                .is_some_and(|document| document.hidden());
        },
    );

    crate::deps::hreads::spawn(async {
        loop {
            crate::deps::hreads::sleep(HEARTBEAT_SECS).await;
            with_visit(|_| {});
        }
    });
}

fn listen(window: &web_sys::Window, event: &str, change: impl Fn(&mut Visit) + 'static) {
    listen_on(Some(window.as_ref()), event, change);
}

fn listen_on(target: Option<&web_sys::EventTarget>, event: &str, change: impl Fn(&mut Visit) + 'static) {
    let Some(target) = target else {
        return;
    };
    let listener = Closure::<dyn FnMut()>::new(move || with_visit(&change));
    if let Err(err) = target.add_event_listener_with_callback(event, listener.as_ref().unchecked_ref()) {
        warn!("Failed to listen to {event} for the visit record: {err:?}");
    }
    // The listener lives as long as the page.
    listener.forget();
}

/// Records a step of the start, written to storage before the next call
/// can hang.
pub(crate) fn step(name: impl Into<String>) {
    with_visit(|visit| {
        let ms = now_ms() - visit.started;
        visit.steps.push(Step {
            name: name.into(),
            ms,
        });
    });
}

/// Counts a presented frame, and records the first one as a step.
pub(crate) fn frame() {
    FRAMES.fetch_add(1, Ordering::Relaxed);
    if !FIRST_FRAME.swap(true, Ordering::Relaxed) {
        step("first frame");
    }
}

/// Keeps the line for the record. It is written with the next step or
/// heartbeat, never on its own, a log line can come often.
pub(crate) fn push_log(level: log::Level, line: &str) {
    if PATH.get().is_none() {
        return;
    }
    // The logger can run inside a locked change of the visit.
    let Some(mut guard) = VISIT.try_lock() else {
        return;
    };
    let Some(visit) = guard.as_mut() else {
        return;
    };
    if visit.log.len() == LOG_LINES {
        visit.log.pop_front();
    }
    let mut kept = String::new();
    let _ = write!(kept, "{level} ");
    kept.extend(line.chars().take(LOG_LINE_CHARS));
    visit.log.push_back(kept);
}

/// Sends the panic with the record of this visit and marks the visit as
/// ended, so the next start does not send it again. Only a try lock, the
/// panicking code may hold the record.
pub(crate) fn report_panic(info: &PanicHookInfo) {
    if PATH.get().is_none() {
        return;
    }
    let Some(mut guard) = VISIT.try_lock() else {
        error!("The visit record was locked, the panic is not reported");
        return;
    };
    let Some(visit) = guard.as_mut() else {
        return;
    };
    visit.steps.push(Step {
        name: "panic".into(),
        ms:   now_ms() - visit.started,
    });
    visit.closed = true;
    save(visit);
    send(&Report {
        kind: "panic",
        panic: Some(info.to_string()),
        visit,
    });
}

/// A sync request, a panic kills the instance right after the hook.
fn send(report: &Report) {
    let Some(path) = PATH.get() else {
        return;
    };
    let body = match serde_json::to_string(report) {
        Ok(body) => body,
        Err(err) => {
            error!("Failed to write the visit report: {err}");
            return;
        }
    };
    let Ok(request) = web_sys::XmlHttpRequest::new() else {
        error!("Failed to make the visit report request");
        return;
    };
    let sent = request
        .open_with_async("POST", path, false)
        .and_then(|()| request.set_request_header("Content-Type", "application/json"))
        .and_then(|()| request.send_with_opt_str(Some(&body)));
    match sent {
        Ok(()) => info!(
            "Sent the {} visit report to {path}: {}",
            report.kind,
            request.status().unwrap_or(0)
        ),
        Err(err) => error!(
            "Failed to send the {} visit report to {path}: {err:?}",
            report.kind
        ),
    }
}
