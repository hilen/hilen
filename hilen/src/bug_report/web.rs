//! The browser half of bug reporting. The sentry crate does not build for
//! wasm, so a report and a panic become an envelope from `envelope.rs`
//! posted to the DSN's envelope endpoint.

use std::{
    fmt::Write,
    panic::PanicHookInfo,
    sync::{OnceLock, mpsc::TryRecvError},
};

use anyhow::{Result, anyhow, bail};
use log::{debug, error};
use web_sys::js_sys::{Math, Uint8Array};

use crate::{
    bug_report::{
        BugReportData,
        envelope::{
            self, Attachment, BrowserContext, Contexts, Dsn, Event, Exception, Exceptions, Mechanism, User,
        },
        log_ring::LogRing,
    },
    deps::{
        hreads::{now, sleep, spawn},
        netrun::rest::client,
    },
    window::{Screenshot, Window},
};

static DSN: OnceLock<Dsn> = OnceLock::new();

/// The app's `App::sentry_url` resolves after start, so reporting turns
/// on once this lands, the same point the native client logs its line.
pub(crate) fn set_dsn(raw: &str) {
    match Dsn::parse(raw) {
        Ok(dsn) => {
            if DSN.set(dsn).is_err() {
                error!("Sentry DSN set twice, the first one stays");
                return;
            }
            debug!("sentry ready");
        }
        Err(err) => error!("Sentry DSN is invalid, reporting stays off: {err}"),
    }
}

pub(super) fn enabled() -> bool {
    DSN.get().is_some()
}

/// The browser cannot block the main thread on a frame, so the request is
/// polled between frames, the map callback delivers on the main thread.
pub(super) async fn screenshot() -> Result<Screenshot> {
    let receiver = Window::current().request_screenshot();

    loop {
        match receiver.try_recv() {
            Ok(shot) => return Ok(shot),
            Err(TryRecvError::Empty) => sleep(0.0).await,
            Err(TryRecvError::Disconnected) => bail!("the screenshot request was dropped"),
        }
    }
}

pub(super) fn submit(data: BugReportData) {
    let Some(dsn) = DSN.get() else {
        error!("Bug report submitted with no Sentry DSN");
        return;
    };

    let envelope = match report_envelope(dsn, data) {
        Ok(envelope) => envelope,
        Err(err) => {
            error!("Bug report envelope failed: {err}");
            return;
        }
    };

    spawn(async move {
        if let Err(err) = post(dsn.envelope_url(), envelope).await {
            error!("Bug report send failed: {err}");
        }
    });
}

fn report_envelope(dsn: &Dsn, data: BugReportData) -> Result<Vec<u8>> {
    let BugReportData {
        email,
        description,
        screenshot_png,
        keys,
    } = data;

    let mut event = Event::new(event_id(), now(), "info");
    event.message = Some(description);
    event.user = Some(User { email });
    event.contexts = user_agent().map(|name| Contexts {
        browser: BrowserContext { name },
    });

    let log = LogRing::dump();
    let keys = keys.map(|keys| serde_json::to_vec_pretty(&keys)).transpose()?;

    let mut attachments = Vec::new();
    if !screenshot_png.is_empty() {
        attachments.push(Attachment {
            filename:     "screenshot.png",
            content_type: "image/png",
            bytes:        &screenshot_png,
        });
    }
    if !log.is_empty() {
        attachments.push(Attachment {
            filename:     "log.txt",
            content_type: "text/plain",
            bytes:        log.as_bytes(),
        });
    }
    if let Some(keys) = &keys {
        attachments.push(Attachment {
            filename:     "key_presses.json",
            content_type: "application/json",
            bytes:        keys,
        });
    }

    envelope::build(dsn, &event, &attachments)
}

async fn post(url: &str, envelope: Vec<u8>) -> Result<()> {
    let response = client().post(url).body(envelope).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!(
            "[{status}] {}",
            response.text().await.unwrap_or_default()
        ));
    }
    Ok(())
}

/// A wasm panic kills the instance right after the hook returns, so an
/// async fetch never leaves. A sync XHR blocks until sent and exists on
/// workers too. Nothing here may touch thread local state or a lock the
/// panicking thread may hold, a second panic aborts with no output.
pub(crate) fn report_panic(info: &PanicHookInfo) {
    let Some(dsn) = DSN.get() else {
        return;
    };

    let mut event = Event::new(event_id(), now(), "fatal");
    event.exception = Some(Exceptions {
        values: vec![Exception {
            ty:        "panic",
            value:     info.to_string(),
            mechanism: Mechanism {
                ty:      "panic",
                handled: false,
            },
        }],
    });

    let envelope = match envelope::build(dsn, &event, &[]) {
        Ok(envelope) => envelope,
        Err(err) => {
            error!("Panic envelope failed: {err}");
            return;
        }
    };

    let Ok(request) = web_sys::XmlHttpRequest::new() else {
        error!("Failed to create the Sentry panic request");
        return;
    };

    if request.open_with_async("POST", dsn.envelope_url(), false).is_err() {
        error!("Failed to open the Sentry panic request");
        return;
    }

    // The atomics build keeps wasm memory in a SharedArrayBuffer, which
    // XHR refuses as a body. `Uint8Array::from` copies into a plain one.
    if request
        .send_with_opt_buffer_source(Some(&Uint8Array::from(envelope.as_slice())))
        .is_err()
    {
        error!("Failed to send the Sentry panic event");
    }
}

/// 32 hex chars. `Math.random` exists on the main thread and on workers,
/// and 52 bits of its mantissa are random per call.
fn event_id() -> String {
    const MANTISSA: u64 = (1 << 52) - 1;

    let mut id = String::with_capacity(39);
    while id.len() < 32 {
        if write!(id, "{:013x}", Math::random().to_bits() & MANTISSA).is_err() {
            break;
        }
    }
    id.truncate(32);
    id
}

fn user_agent() -> Option<String> {
    web_sys::window()?.navigator().user_agent().ok()
}
