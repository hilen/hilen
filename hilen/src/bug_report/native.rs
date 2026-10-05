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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use anyhow::Result;
    use parking_lot::Mutex;
    use sentry::{
        Client, ClientOptions, Envelope, Hub, Scope, Transport,
        protocol::{EnvelopeItem, Event},
    };

    use crate::bug_report::{BugReport, log_ring::LogRing};

    /// Keeps every envelope the client hands over instead of posting it.
    #[derive(Default)]
    struct Captured(Mutex<Vec<Envelope>>);

    impl Transport for Captured {
        fn send_envelope(&self, envelope: Envelope) {
            self.0.lock().push(envelope);
        }
    }

    /// Filename and payload of each attachment, in envelope order.
    type Attachments = Vec<(String, Vec<u8>)>;

    /// Runs `send` on a hub of its own with a DSN and returns the one event
    /// it produced and its attachments as filename and payload.
    fn sent(send: impl FnOnce()) -> Result<(Event<'static>, Attachments)> {
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
                EnvelopeItem::Attachment(item) => {
                    attachments.push((item.filename.clone(), item.buffer.clone()));
                }
                other => panic!("unexpected envelope item {other:?}"),
            }
        }

        Ok((event.expect("the envelope has no event"), attachments))
    }

    /// `BugReport::send` on native. The event carries the description and
    /// the email, the log is always attached, the screenshot only when
    /// given, and key presses never.
    #[test]
    fn send_with_and_without_screenshot() -> Result<()> {
        LogRing::push("a line for the log attachment".to_string());
        let png = vec![0x89, b'P', b'N', b'G', b'\n', 0];

        let (event, attachments) =
            sent(|| BugReport::send("a@b.c", "the button does nothing", Some(png.clone())))?;
        assert_eq!(event.message.as_deref(), Some("the button does nothing"));
        assert_eq!(event.user.and_then(|user| user.email).as_deref(), Some("a@b.c"));
        let names: Vec<&str> = attachments.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["screenshot.png", "log.txt"]);
        assert_eq!(attachments[0].1, png);

        let (event, attachments) = sent(|| BugReport::send("a@b.c", "the button does nothing", None))?;
        assert_eq!(event.message.as_deref(), Some("the button does nothing"));
        let names: Vec<&str> = attachments.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["log.txt"]);

        Ok(())
    }
}
