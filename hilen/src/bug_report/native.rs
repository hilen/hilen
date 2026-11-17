use log::error;
use sentry::{Scope, protocol::Attachment};

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
        attachment,
        keys,
    } = data;

    sentry::with_scope(
        move |scope| {
            scope.set_user(Some(sentry::User {
                email: Some(email),
                ..Default::default()
            }));

            if let Some(attachment) = attachment {
                scope.add_attachment(Attachment {
                    buffer:       attachment.bytes,
                    filename:     attachment.file_name,
                    content_type: Some(attachment.content_type),
                    ty:           None,
                });
            }

            attach_log(scope);

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

/// `BugReport::send_warning` on native. Non blocking like `submit`.
pub(super) fn warning(message: &str, values: Vec<(String, String)>) {
    sentry::with_scope(
        move |scope| {
            for (key, value) in values {
                scope.set_extra(&key, value.into());
            }

            attach_log(scope);
        },
        || sentry::capture_message(message, sentry::Level::Warning),
    );
}

fn attach_log(scope: &mut Scope) {
    let log = LogRing::dump();

    if !log.is_empty() {
        scope.add_attachment(Attachment {
            buffer:       log.into_bytes(),
            filename:     "log.txt".to_string(),
            content_type: Some("text/plain".to_string()),
            ty:           None,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use parking_lot::Mutex;
    use sentry::{
        Client, ClientOptions, Envelope, Hub, Level, Scope, Transport,
        protocol::{EnvelopeItem, Event},
    };

    use crate::bug_report::{BugReport, BugReportAttachment, log_ring::LogRing};

    /// Keeps every envelope the client hands over instead of posting it.
    #[derive(Default)]
    struct Captured(Mutex<Vec<Envelope>>);

    impl Transport for Captured {
        fn send_envelope(&self, envelope: Envelope) {
            self.0.lock().push(envelope);
        }
    }

    /// Filename, content type and payload of an attachment.
    type Sent = (String, Option<String>, Vec<u8>);

    /// Runs `send` on a hub of its own with a DSN and returns the one event
    /// it produced and its attachments in envelope order.
    fn sent(send: impl FnOnce()) -> Result<(Event<'static>, Vec<Sent>)> {
        let transport = Arc::new(Captured::default());
        let mut options = ClientOptions::default();
        options.dsn = Some("https://key@sentry.invalid/1".parse()?);
        options.transport = Some(Arc::new(transport.clone()));
        let client = Client::with_options(options);

        Hub::run(
            Arc::new(Hub::new(Some(Arc::new(client)), Arc::new(Scope::default()))),
            send,
        );

        let mut envelopes = transport.0.lock();
        assert_eq!(envelopes.len(), 1);
        let envelope = envelopes.remove(0);

        let mut event = None;
        let mut attachments = Vec::new();
        for item in envelope.items() {
            match item {
                EnvelopeItem::Event(item) => event = Some(item.clone()),
                EnvelopeItem::Attachment(item) => attachments.push((
                    item.filename.clone(),
                    item.content_type.clone(),
                    item.buffer.clone(),
                )),
                other => panic!("unexpected envelope item {other:?}"),
            }
        }

        Ok((*event.expect("the envelope has no event"), attachments))
    }

    fn names(attachments: &[Sent]) -> Vec<&str> {
        attachments.iter().map(|(name, ..)| name.as_str()).collect()
    }

    /// `BugReport::send` on native. The event carries the description and
    /// the email, the log is always attached, the screenshot only when
    /// given and not empty, and key presses never.
    #[test]
    fn send_with_and_without_screenshot() -> Result<()> {
        LogRing::push("a line for the log attachment".to_string());
        let png = vec![0x89, b'P', b'N', b'G', b'\n', 0];

        let (event, attachments) = sent(|| {
            BugReport::send(
                "a@b.c",
                "the button does nothing",
                Some(BugReportAttachment::screenshot(png.clone())),
            );
        })?;
        assert_eq!(event.message.as_deref(), Some("the button does nothing"));
        assert_eq!(event.user.and_then(|user| user.email).as_deref(), Some("a@b.c"));
        assert_eq!(names(&attachments), ["screenshot.png", "log.txt"]);
        assert_eq!(attachments[0].1.as_deref(), Some("image/png"));
        assert_eq!(attachments[0].2, png);

        let (event, attachments) = sent(|| BugReport::send("a@b.c", "the button does nothing", None))?;
        assert_eq!(event.message.as_deref(), Some("the button does nothing"));
        assert_eq!(names(&attachments), ["log.txt"]);

        let (event, attachments) = sent(|| {
            BugReport::send(
                "a@b.c",
                "the button does nothing",
                Some(BugReportAttachment::screenshot(Vec::new())),
            );
        })?;
        assert_eq!(event.message.as_deref(), Some("the button does nothing"));
        assert_eq!(names(&attachments), ["log.txt"]);

        Ok(())
    }

    /// `BugReport::send_warning` on native. The event is a warning with
    /// the message, every pair as an extra value, no user, and the log.
    #[test]
    fn send_warning_with_values() -> Result<()> {
        LogRing::push("a line for the log attachment".to_string());
        let url = "https://example.com/story/42".to_string();
        let sentences = 17_usize;

        let (event, attachments) = sent(|| {
            BugReport::send_warning(
                "a story has more audio than sentences",
                &[
                    ("url", &url),
                    ("sentences", &sentences),
                    ("audio", &19_u32),
                    ("ratio", &1.5_f32),
                ],
            );
        })?;
        assert_eq!(
            event.message.as_deref(),
            Some("a story has more audio than sentences")
        );
        assert_eq!(event.level, Level::Warning);
        assert!(event.user.is_none());
        let extra: Vec<(&str, Option<&str>)> =
            event.extra.iter().map(|(key, value)| (key.as_str(), value.as_str())).collect();
        assert_eq!(
            extra,
            [
                ("audio", Some("19")),
                ("ratio", Some("1.5")),
                ("sentences", Some("17")),
                ("url", Some("https://example.com/story/42")),
            ]
        );
        assert_eq!(names(&attachments), ["log.txt"]);

        let (event, _) = sent(|| BugReport::send_warning("no values at all", &[]))?;
        assert_eq!(event.message.as_deref(), Some("no values at all"));
        assert!(event.extra.is_empty());

        Ok(())
    }

    /// With no DSN `send_warning` sends nothing, the hub has a transport
    /// to catch an event only through a client, so no client means none.
    #[test]
    fn send_warning_without_a_dsn() {
        Hub::run(Arc::new(Hub::new(None, Arc::new(Scope::default()))), || {
            assert!(!BugReport::enabled());
            BugReport::send_warning("nobody gets this", &[("value", &1)]);
        });
    }

    /// A picked JPEG goes out under its own name and type, not as the PNG
    /// screenshot.
    #[test]
    fn send_with_a_picked_jpeg() -> Result<()> {
        LogRing::push("a line for the log attachment".to_string());
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, b'\n', 0];

        let (event, attachments) = sent(|| {
            BugReport::send(
                "a@b.c",
                "the photo in the reader is upside down",
                Some(BugReportAttachment {
                    file_name:    "IMG_0001.jpeg".to_string(),
                    content_type: "image/jpeg".to_string(),
                    bytes:        jpeg.clone(),
                }),
            );
        })?;
        assert_eq!(
            event.message.as_deref(),
            Some("the photo in the reader is upside down")
        );
        assert_eq!(names(&attachments), ["IMG_0001.jpeg", "log.txt"]);
        assert_eq!(attachments[0].1.as_deref(), Some("image/jpeg"));
        assert_eq!(attachments[0].2, jpeg);

        Ok(())
    }
}
