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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use parking_lot::Mutex;
    use sentry::{
        Client, ClientOptions, Envelope, Hub, Scope, Transport,
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
        let client = Client::with_options(ClientOptions {
            dsn: Some("https://key@sentry.invalid/1".parse()?),
            transport: Some(Arc::new(transport.clone())),
            ..Default::default()
        });

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

        Ok((event.expect("the envelope has no event"), attachments))
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
