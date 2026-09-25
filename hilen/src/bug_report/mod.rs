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
#[cfg(wasm)]
mod web;

use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
use log::{error, warn};

/// Only the dialog UI test builds `KeyPress` values from outside.
#[cfg(feature = "ui-tests")]
pub(crate) use crate::bug_report::input_ring::KeyPress;
#[cfg(wasm)]
pub(crate) use crate::bug_report::web::{report_panic, set_dsn};
pub(crate) use crate::bug_report::{
    input_ring::InputRing,
    report_view::{BugReportData, BugReportInput, BugReportView},
};
use crate::{
    bug_report::log_ring::LogRing,
    gm::flat::Size,
    ui::{Alert, ModalView},
    window::Screenshot,
};

static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

/// The looping gif the report dialog shows between the description and the
/// screenshot, registered by the app. The karkas dialogs play a rooster
/// there, the engine has no opinion on the content.
static ANIMATION: std::sync::RwLock<Option<&'static [u8]>> = std::sync::RwLock::new(None);

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
        if !Self::enabled() {
            warn!("Bug reporting is disabled, the app returns no Sentry DSN");
            return;
        }

        if DIALOG_OPEN.swap(true, Ordering::AcqRel) {
            return;
        }

        let keys = InputRing::snapshot();

        // The screenshot waits for a rendered frame, which the main
        // thread itself drives, so the capture must not block it.
        #[cfg(not_wasm)]
        std::thread::spawn(move || Self::show(crate::AppRunner::take_screenshot(), keys));
        #[cfg(wasm)]
        crate::deps::hreads::spawn(async move { Self::show(web::screenshot().await, keys) });
    }

    fn show(shot: Result<Screenshot>, keys: Vec<input_ring::KeyPress>) {
        let (screenshot_png, screenshot_rgba, screenshot_size) =
            shot.and_then(Self::encode).unwrap_or_else(|err| {
                error!("Bug report screenshot failed: {err}");
                (Vec::new(), Vec::new(), Size::default())
            });

        BugReportView::show_modally_with_input(
            BugReportInput {
                screenshot_png,
                screenshot_rgba,
                screenshot_size,
                log_bytes: LogRing::dump().len(),
                keys,
            },
            |data: Option<BugReportData>| {
                DIALOG_OPEN.store(false, Ordering::Release);

                if let Some(data) = data {
                    #[cfg(not_wasm)]
                    native::submit(data);
                    #[cfg(wasm)]
                    web::submit(data);
                    Alert::show("Bug report sent. Thank you!");
                }
            },
        );
    }

    /// The PNG goes to Sentry, the raw RGBA feeds the thumbnail texture
    /// in the dialog.
    fn encode(shot: Screenshot) -> Result<(Vec<u8>, Vec<u8>, Size<u32>)> {
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

        Ok((png, bytes, shot.size))
    }

    pub(crate) fn push_log_line(line: String) {
        LogRing::push(line);
    }
}
