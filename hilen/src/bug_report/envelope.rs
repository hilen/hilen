//! The Sentry envelope for the browser, where the sentry crate does not
//! run. `sentry-types` has the same shapes, but its `Event::default`
//! reads `SystemTime::now`, which panics on wasm, and it pulls a `rand`
//! whose `getrandom` needs a cfg flag in every app's build to link for
//! the browser. So these are the few fields a report and a panic need.
//! The format is in the Sentry developer docs, sdk/data-model/envelopes.

use anyhow::{Result, anyhow, bail};
use reqwest::Url;
use serde::Serialize;

use crate::bug_report::BugReportData;

/// What the engine needs from a DSN, parsed once when the app hands it over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Dsn {
    raw:          String,
    envelope_url: String,
}

impl Dsn {
    /// A DSN is `https://<key>@<host>/<path/><project>`. The key rides in
    /// the query, not in an `X-Sentry-Auth` header, so the browser sends
    /// the POST as a simple request with no CORS preflight.
    pub(crate) fn parse(raw: &str) -> Result<Self> {
        let url = Url::parse(raw)?;

        let key = url.username();
        if key.is_empty() {
            bail!("Sentry DSN has no public key");
        }

        let host = url.host_str().ok_or_else(|| anyhow!("Sentry DSN has no host"))?;
        let port = url.port().map_or_default(|port| format!(":{port}"));

        let path = url.path().trim_matches('/');
        let (prefix, project) = match path.rsplit_once('/') {
            Some((prefix, project)) => (format!("/{prefix}"), project),
            None => (String::new(), path),
        };
        if project.is_empty() {
            bail!("Sentry DSN has no project id");
        }

        let envelope_url = format!(
            "{}://{host}{port}{prefix}/api/{project}/envelope/?sentry_key={key}&sentry_version=7&sentry_client={SDK_NAME}/{}",
            url.scheme(),
            env!("CARGO_PKG_VERSION"),
        );

        Ok(Self {
            raw: raw.to_string(),
            envelope_url,
        })
    }

    pub(crate) fn envelope_url(&self) -> &str {
        &self.envelope_url
    }
}

const SDK_NAME: &str = "hilen.wasm";

#[derive(Serialize)]
struct EnvelopeHeader<'a> {
    event_id: &'a str,
    dsn:      &'a str,
}

#[derive(Serialize)]
struct ItemHeader<'a> {
    #[serde(rename = "type")]
    ty:           &'a str,
    length:       usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    filename:     Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<&'a str>,
}

#[derive(Serialize)]
pub(crate) struct Sdk {
    name:    &'static str,
    version: &'static str,
}

#[derive(Serialize)]
pub(crate) struct User {
    pub email: String,
}

#[derive(Serialize)]
pub(crate) struct Mechanism {
    #[serde(rename = "type")]
    pub ty:      &'static str,
    pub handled: bool,
}

#[derive(Serialize)]
pub(crate) struct Exception {
    #[serde(rename = "type")]
    pub ty:        &'static str,
    pub value:     String,
    pub mechanism: Mechanism,
}

#[derive(Serialize)]
pub(crate) struct Exceptions {
    pub values: Vec<Exception>,
}

#[derive(Serialize)]
pub(crate) struct BrowserContext {
    pub name: String,
}

#[derive(Serialize)]
pub(crate) struct Contexts {
    pub browser: BrowserContext,
}

/// The fields a report and a panic fill, the rest Sentry fills itself.
#[derive(Serialize)]
pub(crate) struct Event {
    pub event_id:  String,
    /// Seconds since the Unix epoch.
    pub timestamp: f64,
    pub platform:  &'static str,
    pub level:     &'static str,
    pub release:   String,
    pub sdk:       Sdk,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message:   Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user:      Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exception: Option<Exceptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contexts:  Option<Contexts>,
}

impl Event {
    /// `event_id` is 32 lowercase hex chars, a uuid without dashes.
    pub(crate) fn new(event_id: String, timestamp: f64, level: &'static str) -> Self {
        Self {
            event_id,
            timestamp,
            platform: "other",
            level,
            // The same name the native client sends, `sentry::release_name!`.
            release: format!("{}@{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
            sdk: Sdk {
                name:    SDK_NAME,
                version: env!("CARGO_PKG_VERSION"),
            },
            message: None,
            user: None,
            exception: None,
            contexts: None,
        }
    }
}

pub(crate) struct Attachment<'a> {
    pub filename:     &'a str,
    pub content_type: &'a str,
    pub bytes:        &'a [u8],
}

/// Every item is a JSON header line, then its payload and a newline. The
/// header carries the payload length, so a binary payload with newlines
/// in it is fine.
pub(crate) fn build(dsn: &Dsn, event: &Event, attachments: &[Attachment]) -> Result<Vec<u8>> {
    let mut out = serde_json::to_vec(&EnvelopeHeader {
        event_id: &event.event_id,
        dsn:      &dsn.raw,
    })?;
    out.push(b'\n');

    let payload = serde_json::to_vec(event)?;
    push_item(
        &mut out,
        &ItemHeader {
            ty:           "event",
            length:       payload.len(),
            filename:     None,
            content_type: None,
        },
        &payload,
    )?;

    for attachment in attachments {
        push_item(
            &mut out,
            &ItemHeader {
                ty:           "attachment",
                length:       attachment.bytes.len(),
                filename:     Some(attachment.filename),
                content_type: Some(attachment.content_type),
            },
            attachment.bytes,
        )?;
    }

    Ok(out)
}

/// A bug report on top of `event`, which brings the id, the time and the
/// browser. Each of the attachment, the log and the key presses becomes an
/// attachment only when the report has it.
pub(crate) fn report(dsn: &Dsn, mut event: Event, data: BugReportData, log: &str) -> Result<Vec<u8>> {
    let BugReportData {
        email,
        description,
        attachment,
        keys,
    } = data;

    event.message = Some(description);
    event.user = Some(User { email });

    let keys = keys.map(|keys| serde_json::to_vec_pretty(&keys)).transpose()?;

    let mut attachments = Vec::new();
    if let Some(attachment) = &attachment {
        attachments.push(Attachment {
            filename:     &attachment.file_name,
            content_type: &attachment.content_type,
            bytes:        &attachment.bytes,
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

    build(dsn, &event, &attachments)
}

fn push_item(out: &mut Vec<u8>, header: &ItemHeader, payload: &[u8]) -> Result<()> {
    serde_json::to_writer(&mut *out, header)?;
    out.push(b'\n');
    out.extend_from_slice(payload);
    out.push(b'\n');
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde::Deserialize;

    use super::{Attachment, Dsn, Event, User, build, report};
    use crate::bug_report::{BugReportAttachment, BugReportData};

    const DSN: &str = "https://abc123@o42.ingest.sentry.io/7001";

    #[derive(Deserialize)]
    struct Header {
        event_id: String,
        dsn:      String,
    }

    #[derive(Deserialize)]
    struct Item {
        #[serde(rename = "type")]
        ty:           String,
        length:       usize,
        filename:     Option<String>,
        content_type: Option<String>,
    }

    #[derive(Deserialize)]
    struct ParsedUser {
        email: String,
    }

    #[derive(Deserialize)]
    struct ParsedEvent {
        event_id: String,
        level:    String,
        message:  String,
        user:     ParsedUser,
    }

    /// Reads one header line and the payload it announces.
    fn next_item<'a>(rest: &mut &'a [u8]) -> Result<(Item, &'a [u8])> {
        let newline = rest.iter().position(|byte| *byte == b'\n').expect("item header line");
        let item: Item = serde_json::from_slice(&rest[..newline])?;
        let payload = &rest[newline + 1..newline + 1 + item.length];
        assert_eq!(rest[newline + 1 + item.length], b'\n');
        *rest = &rest[newline + 2 + item.length..];
        Ok((item, payload))
    }

    #[test]
    fn dsn_to_envelope_url() -> Result<()> {
        assert_eq!(
            Dsn::parse(DSN)?.envelope_url(),
            format!(
                "https://o42.ingest.sentry.io/api/7001/envelope/?sentry_key=abc123&sentry_version=7&sentry_client=hilen.wasm/{}",
                env!("CARGO_PKG_VERSION")
            )
        );

        let self_hosted = Dsn::parse("http://key@localhost:9000/sentry/5")?;
        assert!(
            self_hosted
                .envelope_url()
                .starts_with("http://localhost:9000/sentry/api/5/envelope/?sentry_key=key&")
        );

        assert!(Dsn::parse("https://o42.ingest.sentry.io/7001").is_err());
        assert!(Dsn::parse("https://abc@o42.ingest.sentry.io/").is_err());
        assert!(Dsn::parse("not a url").is_err());

        Ok(())
    }

    #[test]
    fn envelope_items_round_trip() -> Result<()> {
        let dsn = Dsn::parse(DSN)?;

        let mut event = Event::new("0123456789abcdef0123456789abcdef".to_string(), 1.5, "info");
        event.message = Some("the button does nothing".to_string());
        event.user = Some(User {
            email: "a@b.c".to_string(),
        });

        // A binary payload with a newline inside, the length must carry it.
        let png = [0x89, b'P', b'N', b'G', b'\n', 0];

        let bytes = build(
            &dsn,
            &event,
            &[
                Attachment {
                    filename:     "screenshot.png",
                    content_type: "image/png",
                    bytes:        &png,
                },
                Attachment {
                    filename:     "log.txt",
                    content_type: "text/plain",
                    bytes:        b"line 1\nline 2\n",
                },
            ],
        )?;

        let newline = bytes.iter().position(|byte| *byte == b'\n').expect("envelope header line");
        let header: Header = serde_json::from_slice(&bytes[..newline])?;
        assert_eq!(header.event_id, event.event_id);
        assert_eq!(header.dsn, DSN);

        let mut rest = &bytes[newline + 1..];

        let (item, payload) = next_item(&mut rest)?;
        assert_eq!(item.ty, "event");
        assert!(item.filename.is_none());
        let parsed: ParsedEvent = serde_json::from_slice(payload)?;
        assert_eq!(parsed.event_id, event.event_id);
        assert_eq!(parsed.level, "info");
        assert_eq!(parsed.message, "the button does nothing");
        assert_eq!(parsed.user.email, "a@b.c");

        let (item, payload) = next_item(&mut rest)?;
        assert_eq!(item.ty, "attachment");
        assert_eq!(item.filename.as_deref(), Some("screenshot.png"));
        assert_eq!(item.content_type.as_deref(), Some("image/png"));
        assert_eq!(payload, png);

        let (item, payload) = next_item(&mut rest)?;
        assert_eq!(item.filename.as_deref(), Some("log.txt"));
        assert_eq!(payload, b"line 1\nline 2\n");

        assert_eq!(rest.len(), 0);

        Ok(())
    }

    /// Filename, content type and payload of each attachment, in envelope
    /// order.
    type Attachments = Vec<(String, String, Vec<u8>)>;

    /// The event of a report and its attachments.
    fn parse_report(bytes: &[u8]) -> Result<(ParsedEvent, Attachments)> {
        let newline = bytes.iter().position(|byte| *byte == b'\n').expect("envelope header line");
        let mut rest = &bytes[newline + 1..];

        let (item, payload) = next_item(&mut rest)?;
        assert_eq!(item.ty, "event");
        let event: ParsedEvent = serde_json::from_slice(payload)?;

        let mut attachments = Vec::new();
        while !rest.is_empty() {
            let (item, payload) = next_item(&mut rest)?;
            assert_eq!(item.ty, "attachment");
            attachments.push((
                item.filename.expect("attachment filename"),
                item.content_type.expect("attachment content type"),
                payload.to_vec(),
            ));
        }

        Ok((event, attachments))
    }

    /// The browser envelope of a report with the given attachment and the
    /// log, as `BugReport::send` makes it, with no key presses.
    fn send(attachment: Option<BugReportAttachment>, log: &str) -> Result<Vec<u8>> {
        report(
            &Dsn::parse(DSN)?,
            Event::new("0123456789abcdef0123456789abcdef".to_string(), 1.5, "info"),
            BugReportData {
                email: "a@b.c".to_string(),
                description: "the button does nothing".to_string(),
                attachment,
                keys: None,
            },
            log,
        )
    }

    fn log_item(log: &str) -> (String, String, Vec<u8>) {
        (
            "log.txt".to_string(),
            "text/plain".to_string(),
            log.as_bytes().to_vec(),
        )
    }

    /// The screenshot is an attachment only when the app gave one.
    #[test]
    fn report_envelope_with_and_without_screenshot() -> Result<()> {
        let png = vec![0x89, b'P', b'N', b'G', b'\n', 0];
        let log = "line 1\nline 2\n";

        let (event, attachments) =
            parse_report(&send(Some(BugReportAttachment::screenshot(png.clone())), log)?)?;
        assert_eq!(event.message, "the button does nothing");
        assert_eq!(event.user.email, "a@b.c");
        assert_eq!(
            attachments,
            [
                ("screenshot.png".to_string(), "image/png".to_string(), png),
                log_item(log),
            ]
        );

        let (event, attachments) = parse_report(&send(None, log)?)?;
        assert_eq!(event.message, "the button does nothing");
        assert_eq!(attachments, [log_item(log)]);

        Ok(())
    }

    /// A picked JPEG keeps its own name and type in the envelope, the
    /// browser path no longer labels every image as the PNG screenshot.
    #[test]
    fn report_envelope_with_a_jpeg() -> Result<()> {
        let jpeg = vec![0xFF, 0xD8, 0xFF, 0xE0, b'\n', 0];
        let log = "line 1\n";

        let (event, attachments) = parse_report(&send(
            Some(BugReportAttachment {
                file_name:    "IMG_0001.jpeg".to_string(),
                content_type: "image/jpeg".to_string(),
                bytes:        jpeg.clone(),
            }),
            log,
        )?)?;
        assert_eq!(event.message, "the button does nothing");
        assert_eq!(
            attachments,
            [
                ("IMG_0001.jpeg".to_string(), "image/jpeg".to_string(), jpeg),
                log_item(log),
            ]
        );

        Ok(())
    }
}
