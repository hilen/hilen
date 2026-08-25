//! In-app bug reporting through Sentry. A report is a normal Sentry event
//! carrying the reporter's description, with the screenshot, the recent
//! log and the opted in key presses as event attachments. It rides the
//! DSN the app already returns from `App::sentry_url`, so there is no
//! separate endpoint and no receiving server. The sentry crate does not
//! run on wasm, so in the browser every call is a no-op, matching
//! `system::Router`.

#[cfg(not_wasm)]
mod input_ring;
#[cfg(not_wasm)]
mod log_ring;
#[cfg(not_wasm)]
mod report_view;

#[cfg(not_wasm)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not_wasm)]
use anyhow::Result;
#[cfg(not_wasm)]
use image::{ExtendedColorType, ImageEncoder, codecs::png::PngEncoder};
#[cfg(not_wasm)]
use log::{error, warn};
#[cfg(not_wasm)]
use sentry::protocol::Attachment;

/// Only the dialog UI test builds `KeyPress` values from outside.
#[cfg(all(not_wasm, feature = "ui-tests"))]
pub(crate) use crate::bug_report::input_ring::KeyPress;
#[cfg(not_wasm)]
pub(crate) use crate::bug_report::{
    input_ring::InputRing,
    report_view::{BugReportData, BugReportInput, BugReportView},
};
#[cfg(not_wasm)]
use crate::gm::flat::Size;
#[cfg(not_wasm)]
use crate::{
    bug_report::log_ring::LogRing,
    ui::{Alert, ModalView},
};

#[cfg(not_wasm)]
static DIALOG_OPEN: AtomicBool = AtomicBool::new(false);

pub struct BugReport;

#[cfg(not_wasm)]
impl BugReport {
    /// Bug reporting works only when the app opted into Sentry by
    /// returning a DSN from `App::sentry_url`.
    pub fn enabled() -> bool {
        sentry::Hub::current().client().is_some()
    }

    /// Captures a screenshot, then presents the report dialog. On
    /// desktop `Ctrl/Cmd+Shift+R` calls this, on a touch platform the
    /// app calls it from its own affordance.
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
        std::thread::spawn(move || {
            let (screenshot_png, screenshot_rgba, screenshot_size) = Self::capture().unwrap_or_else(|err| {
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
                        Self::submit(data);
                        Alert::show("Bug report sent. Thank you!");
                    }
                },
            );
        });
    }

    /// Sending is non blocking, the sentry client hands the envelope to
    /// its own transport thread.
    fn submit(data: BugReportData) {
        let BugReportData {
            email,
            description,
            screenshot_png,
            keys,
        } = data;

        sentry::with_scope(
            move |scope| {
                scope.set_user(Some(sentry::User {
                    email: Some(email),
                    ..Default::default()
                }));

                if !screenshot_png.is_empty() {
                    scope.add_attachment(Attachment {
                        buffer:       screenshot_png,
                        filename:     "screenshot.png".to_string(),
                        content_type: Some("image/png".to_string()),
                        ty:           None,
                    });
                }

                let log = LogRing::dump();

                if !log.is_empty() {
                    scope.add_attachment(Attachment {
                        buffer:       log.into_bytes(),
                        filename:     "log.txt".to_string(),
                        content_type: Some("text/plain".to_string()),
                        ty:           None,
                    });
                }

                if let Some(keys) = &keys {
                    match serde_json::to_vec_pretty(keys) {
                        Ok(json) => scope.add_attachment(Attachment {
                            buffer:       json,
                            filename:     "key_presses.json".to_string(),
                            content_type: Some("application/json".to_string()),
                            ty:           None,
                        }),
                        Err(err) => error!("Bug report key presses failed to serialize: {err}"),
                    }
                }
            },
            || sentry::capture_message(&description, sentry::Level::Info),
        );
    }

    /// The PNG goes to Sentry, the raw RGBA feeds the thumbnail texture
    /// in the dialog.
    fn capture() -> Result<(Vec<u8>, Vec<u8>, Size<u32>)> {
        let shot = crate::AppRunner::take_screenshot()?;

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

#[cfg(wasm)]
impl BugReport {
    pub fn enabled() -> bool {
        false
    }

    pub fn open() {
        log::trace!("BugReport::open in the browser is a no-op");
    }
}
