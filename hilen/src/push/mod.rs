//! Remote notifications, the pushes a backend sends through APNs. iOS only
//! for now, every other system answers that it has no push, so the same app
//! code runs everywhere. See docs/push.md.
//!
//! ```ignore
//! use hilen::{dispatch::spawn, push::Push};
//!
//! Push::on_token(weak, |token| send_to_backend(token.hex, token.sandbox));
//! Push::on_open(weak, |message| open_page(message.data::<Target>()));
//! spawn(async { Push::request().await });
//! ```
//!
//! The app needs `push = true` in the `[ios]` table of `hilen.toml`, which
//! writes the `aps-environment` entitlement.

#[cfg(target_os = "ios")]
pub(crate) mod ios;
#[cfg(target_os = "ios")]
use ios as system;

#[cfg(not(target_os = "ios"))]
mod none;
use anyhow::{Result, anyhow};
use futures_channel::oneshot;
use log::{debug, info, warn};
#[cfg(not(target_os = "ios"))]
use none as system;
use parking_lot::Mutex;
use serde::de::DeserializeOwned;

use crate::{
    deps::{hreads::on_main_later, refs::Weak},
    ui::UIEvent,
};

/// How a system answers a request for the permission. It may call it on any
/// thread, once.
type Done = Box<dyn FnOnce(Result<bool>) + Send>;

static ON_TOKEN: UIEvent<PushToken> = UIEvent::const_new();
static ON_OPEN: UIEvent<PushMessage> = UIEvent::const_new();

/// The last token APNs gave in this run.
static TOKEN: Mutex<Option<PushToken>> = Mutex::new(None);

/// Pushes the user opened while nobody listened, oldest first.
static PENDING: Mutex<Vec<PushMessage>> = Mutex::new(Vec::new());

/// The address of this install for APNs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushToken {
    /// The device token in lower case hex, the form the path of a request to
    /// APNs takes.
    pub hex:     String,
    /// The token belongs to the sandbox of APNs, `api.sandbox.push.apple.com`.
    /// A development build and the simulator have such a token. A build of
    /// the store, also one for testers, has one of production.
    pub sandbox: bool,
}

/// A push the user tapped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushMessage {
    /// The whole payload as JSON, the `aps` dictionary included.
    pub json: String,
}

impl PushMessage {
    /// The payload as the type of the app. Its fields are the keys the
    /// backend put next to `aps`, a field it does not name is ignored.
    pub fn data<T: DeserializeOwned>(&self) -> Result<T> {
        Ok(serde_json::from_str(&self.json)?)
    }
}

pub struct Push;

impl Push {
    /// Whether this system takes pushes in the engine. Only iOS for now.
    pub const fn is_supported() -> bool {
        cfg!(target_os = "ios")
    }

    /// The token APNs gave in this run, `None` before it came. An app sends
    /// it to its backend once more at a logout, so the backend forgets it.
    pub fn token() -> Option<PushToken> {
        TOKEN.lock().clone()
    }

    /// Asks the user for the permission to show pushes, then registers the
    /// app with APNs. iOS shows its dialog only the first time, later calls
    /// answer at once with the choice of the user. `Ok(true)` when pushes
    /// are allowed, the token then comes to `on_token`. An app calls it on
    /// every start, APNs can change the token.
    pub async fn request() -> Result<bool> {
        info!("push permission asked");
        let (sender, answer) = oneshot::channel();
        system::request(Box::new(move |result| {
            if sender.send(result).is_err() {
                debug!("push permission answer dropped, the caller stopped waiting");
            }
        }));
        let result = answer
            .await
            .unwrap_or_else(|_| Err(anyhow!("the system ended the push request with no answer")));

        match &result {
            Ok(true) => info!("push allowed, the app registers with APNs"),
            Ok(false) => info!("push not allowed by the user"),
            Err(err) => warn!("push permission not asked: {err:#}"),
        }
        result
    }

    /// Calls `action` with the token of APNs, while `subscriber` lives, on
    /// the main thread. A token that came before goes to the new subscriber
    /// too, and a subscriber may get the same token again.
    pub fn on_token<U: ?Sized>(subscriber: Weak<U>, action: impl FnMut(PushToken) + Send + 'static) {
        ON_TOKEN.val(subscriber, action);
        if TOKEN.lock().is_some() {
            on_main_later(deliver_token);
        }
    }

    /// Calls `action` with every push the user taps, while `subscriber`
    /// lives, on the main thread. The push that started the app goes to the
    /// first subscriber.
    pub fn on_open<U: ?Sized>(subscriber: Weak<U>, action: impl FnMut(PushMessage) + Send + 'static) {
        ON_OPEN.val(subscriber, action);
        if !PENDING.lock().is_empty() {
            on_main_later(deliver_pending);
        }
    }
}

/// Takes the token from a platform, from any thread.
#[cfg_attr(not(target_os = "ios"), cfg(test))]
fn receive_token(token: PushToken) {
    let start = token.hex.get(..8).unwrap_or(&token.hex);
    info!("push token from APNs, {start}..., sandbox {}", token.sandbox);
    *TOKEN.lock() = Some(token);
    on_main_later(deliver_token);
}

fn deliver_token() {
    let Some(token) = TOKEN.lock().clone() else {
        return;
    };
    if ON_TOKEN.has_subscribers() {
        ON_TOKEN.trigger(token);
    } else {
        info!("the push token waits for a subscriber of Push::on_token");
    }
}

/// Takes a tapped push from a platform, from any thread.
#[cfg_attr(not(target_os = "ios"), cfg(test))]
fn receive_open(json: String) {
    info!("push opened, {} bytes of payload", json.len());
    PENDING.lock().push(PushMessage { json });
    on_main_later(deliver_pending);
}

fn deliver_pending() {
    if !ON_OPEN.has_subscribers() {
        let waiting = PENDING.lock().len();
        if waiting > 0 {
            info!("{waiting} push(es) wait for a subscriber of Push::on_open");
        }
        return;
    }
    let messages: Vec<PushMessage> = PENDING.lock().drain(..).collect();
    for message in messages {
        ON_OPEN.trigger(message);
    }
}

/// Whether the provisioning profile of a build gives tokens of the sandbox
/// of APNs. A development profile says `development` for `aps-environment`,
/// one for the App Store says `production`. The profile is signed, but its
/// plist is plain text inside it.
#[cfg_attr(not(target_os = "ios"), cfg(test))]
fn profile_is_sandbox(profile: &[u8]) -> bool {
    let text = String::from_utf8_lossy(profile);
    let Some((_, after)) = text.split_once("<key>aps-environment</key>") else {
        return false;
    };
    let value = after
        .trim_start()
        .strip_prefix("<string>")
        .and_then(|rest| rest.split_once("</string>"))
        .map(|(value, _)| value);
    value == Some("development")
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde::Deserialize;

    use super::{PENDING, Push, PushMessage, profile_is_sandbox, receive_open, receive_token};
    use crate::push::PushToken;

    fn profile(environment: &str) -> Vec<u8> {
        let mut bytes = vec![0x30, 0x82, 0x2b, 0x00];
        bytes.extend_from_slice(
            format!(
                r"<plist><dict>
<key>Entitlements</key>
<dict>
	<key>aps-environment</key>
	<string>{environment}</string>
</dict>
</dict></plist>"
            )
            .as_bytes(),
        );
        bytes.extend_from_slice(&[0xff, 0x00, 0x01]);
        bytes
    }

    #[test]
    fn a_development_profile_is_the_sandbox() {
        assert!(profile_is_sandbox(&profile("development")));
        assert!(!profile_is_sandbox(&profile("production")));
    }

    #[test]
    fn a_profile_with_no_push_is_not_the_sandbox() {
        assert!(!profile_is_sandbox(b"<plist><dict></dict></plist>"));
        assert!(!profile_is_sandbox(b"<key>aps-environment</key>"));
        assert!(!profile_is_sandbox(&[]));
    }

    #[derive(Debug, PartialEq, Deserialize)]
    struct Target {
        list: u32,
    }

    #[test]
    fn the_data_of_a_push_is_the_type_of_the_app() -> Result<()> {
        let message = PushMessage {
            json: r#"{"aps":{"alert":{"title":"Weekly","body":"Dave added milk"}},"list":7}"#.to_string(),
        };
        assert_eq!(message.data::<Target>()?, Target { list: 7 });
        assert!(message.data::<u32>().is_err());
        Ok(())
    }

    #[test]
    fn a_tapped_push_waits_for_a_subscriber() {
        let json = r#"{"aps":{},"waits":true}"#.to_string();
        receive_open(json.clone());
        assert!(PENDING.lock().contains(&PushMessage { json }));
    }

    #[test]
    fn the_last_token_is_kept() {
        let token = PushToken {
            hex:     "a1b2".to_string(),
            sandbox: true,
        };
        receive_token(token.clone());
        assert_eq!(Push::token(), Some(token));
    }
}
