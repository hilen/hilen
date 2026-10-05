//! In-app bug reporting through Sentry. A report is a normal Sentry event
//! carrying the reporter's description, with the screenshot, the recent
//! log and the opted in key presses as event attachments. It rides the
//! DSN the app already returns from `App::sentry_url`, so there is no
//! separate endpoint and no receiving server. Native builds send through
//! the sentry crate. It does not run on wasm, so the browser builds the
//! envelope itself and posts it, see `web.rs`.

#[cfg(any(wasm, test))]
mod envelope;
mod input_ring;
mod log_ring;
#[cfg(not_wasm)]
mod native;
mod report_view;
mod style;
#[cfg(wasm)]
mod web;

use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use log::{error, warn};

/// Only the dialog UI test builds `KeyPress` values from outside.
#[cfg(feature = "ui-tests")]
pub(crate) use crate::bug_report::input_ring::KeyPress;
pub use crate::bug_report::style::BugReportStyle;
#[cfg(wasm)]
pub(crate) use crate::bug_report::web::{report_panic, set_dsn};
pub(crate) use crate::bug_report::{
    input_ring::InputRing,
    report_view::{BugReportData, BugReportInput, BugReportView},
};
use crate::{
    bug_report::log_ring::LogRing,
    deps::hreads::on_main,
    gm::flat::Size,
    ui::{Alert, ModalView},
    window::Screenshot,
};

/// A screenshot of the app for a bug report, from `BugReport::capture`.
/// `png` is what goes to Sentry. `rgba` and `size` are the same pixels
/// for a thumbnail, `Image::from_raw_data(rgba, name, size, 4)`. All of
/// it is empty when the capture failed.
#[derive(Debug, Clone, Default)]
pub struct BugReportScreenshot {
    pub png:  Vec<u8>,
    pub rgba: Vec<u8>,
    pub size: Size<u32>,
}

static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

/// The looping gif the report dialog shows between the description and the
/// screenshot, registered by the app. The karkas dialogs play a rooster
/// there, the engine has no opinion on the content.
static ANIMATION: std::sync::RwLock<Option<&'static [u8]>> = std::sync::RwLock::new(None);

/// The email the report form opens with, set by an app that knows who
/// is signed in.
static EMAIL: std::sync::RwLock<Option<String>> = std::sync::RwLock::new(None);

pub struct BugReport;

impl BugReport {
    /// Register a looping gif the report dialog shows between the
    /// description and the screenshot. Call once at launch with
    /// `include_bytes!` data. Without one the dialog shows no animation.
    pub fn set_animation(gif: &'static [u8]) {
        *ANIMATION.write().expect("animation lock") = Some(gif);
    }

    pub(crate) fn animation() -> Option<&'static [u8]> {
        *ANIMATION.read().expect("animation lock")
    }

    /// The test harness snapshot hand-back, see `prepare_harness`. Also
    /// clears a test's animation so it cannot leak into the next test.
    pub(crate) fn restore_animation(gif: Option<&'static [u8]>) {
        *ANIMATION.write().expect("animation lock") = gif;
    }

    /// The email the report form opens with, for an app that knows who is
    /// signed in. The reporter can still change it. An empty string clears
    /// it, call that on sign out.
    pub fn set_email(email: impl Into<String>) {
        let email = email.into();
        *EMAIL.write().expect("email lock") = (!email.is_empty()).then_some(email);
    }

    pub(crate) fn email() -> Option<String> {
        EMAIL.read().expect("email lock").clone()
    }

    /// The test harness snapshot hand-back, like `restore_animation`.
    pub(crate) fn restore_email(email: Option<String>) {
        *EMAIL.write().expect("email lock") = email;
    }

    /// Bug reporting works only when the app opted into Sentry by
    /// returning a DSN from `App::sentry_url`.
    pub fn enabled() -> bool {
        #[cfg(not_wasm)]
        {
            native::enabled()
        }
        #[cfg(wasm)]
        {
            web::enabled()
        }
    }

    /// Captures a screenshot, then presents the report dialog. On
    /// desktop `Ctrl/Cmd+Shift+R` calls this, on a touch platform and in
    /// the browser the app calls it from its own affordance.
    pub fn open() {
        if Self::disabled() {
            return;
        }

        if DIALOG_OPEN.swap(true, Ordering::AcqRel) {
            return;
        }

        let keys = InputRing::snapshot();

        Self::capture(move |screenshot| Self::show(screenshot, keys));
    }

    /// The first half of `open`, for an app with a report dialog of its
    /// own. Takes a screenshot of the app as it is now, so call it before
    /// the dialog shows. `done` runs on the main thread. A failed capture
    /// logs an error and hands an empty screenshot. It works without a
    /// DSN too, check `enabled` before offering a report.
    pub fn capture(done: impl FnOnce(BugReportScreenshot) + Send + 'static) {
        // The screenshot waits for a rendered frame, which the main
        // thread itself drives, so the capture must not block it.
        #[cfg(not_wasm)]
        std::thread::spawn(move || {
            let screenshot = Self::encode_or_empty(crate::AppRunner::take_screenshot());
            on_main(move || done(screenshot));
        });
        #[cfg(wasm)]
        crate::deps::hreads::spawn(async move {
            let screenshot = Self::encode_or_empty(web::screenshot().await);
            on_main(move || done(screenshot));
        });
    }

    /// The second half of `open`, for an app with a report dialog of its
    /// own. Sends a Sentry event with the description as its message, the
    /// email as its user and the recent log lines attached. The screenshot
    /// is attached when given, pass the `png` of `capture`. A send never
    /// attaches the recent key presses, their opt in is a checkbox of the
    /// engine dialog only. Without a DSN it logs a warning and sends
    /// nothing. It does not block, on native the sentry client sends from
    /// its own thread, in the browser a task posts the event.
    pub fn send(email: impl Into<String>, description: impl Into<String>, screenshot_png: Option<Vec<u8>>) {
        Self::submit(BugReportData {
            email:          email.into(),
            description:    description.into(),
            screenshot_png: screenshot_png.unwrap_or_default(),
            keys:           None,
        });
    }

    fn disabled() -> bool {
        let disabled = !Self::enabled();
        if disabled {
            warn!("Bug reporting is disabled, the app returns no Sentry DSN");
        }
        disabled
    }

    fn submit(data: BugReportData) {
        if Self::disabled() {
            return;
        }

        #[cfg(not_wasm)]
        native::submit(data);
        #[cfg(wasm)]
        web::submit(data);
    }

    fn show(screenshot: BugReportScreenshot, keys: Vec<input_ring::KeyPress>) {
        BugReportView::show_modally_with_input(
            BugReportInput {
                screenshot,
                log_bytes: LogRing::dump().len(),
                keys,
            },
            |data: Option<BugReportData>| {
                DIALOG_OPEN.store(false, Ordering::Release);

                if let Some(data) = data {
                    Self::submit(data);
                    Alert::show("Bug report sent. Thank you!");
                }
            },
        );
    }

    fn encode_or_empty(shot: Result<Screenshot>) -> BugReportScreenshot {
        shot.and_then(Self::encode).unwrap_or_else(|err| {
            error!("Bug report screenshot failed: {err}");
            BugReportScreenshot::default()
        })
    }

    fn encode(shot: Screenshot) -> Result<BugReportScreenshot> {
        let mut rgba = Vec::with_capacity(shot.data.len() * 4);
        for color in &shot.data {
            rgba.extend_from_slice(&[color.r, color.g, color.b, 255]);
        }

        let mut png = Vec::new();
        PngEncoder::new(&mut png).write_image(
            &rgba,
            shot.size.width,
            shot.size.height,
            ExtendedColorType::Rgba8,
        )?;

        Ok(BugReportScreenshot {
            png,
            rgba,
            size: shot.size,
        })
    }

    pub(crate) fn push_log_line(line: String) {
        LogRing::push(line);
    }
}
