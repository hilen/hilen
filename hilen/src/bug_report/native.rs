use log::error;
use sentry::protocol::Attachment;

use crate::bug_report::{BugReportData, log_ring::LogRing};

pub(super) fn enabled() -> bool {
    sentry::Hub::current().client().is_some()
}

/// Sending is non blocking, the sentry client hands the envelope to its
/// own transport thread.
pub(super) fn submit(data: BugReportData) {
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
