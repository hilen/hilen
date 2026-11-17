//! Pushes to iPhones through APNs, the push service of Apple, see
//! docs/push.md. The app gets its device token from `Push::on_token` of the
//! engine and hands it to its backend, the backend keeps it and sends here.
//!
//! ```ignore
//! let apns = ApnsConfig::from_env("vladas.pirkti")?.map(Apns::new).transpose()?;
//! let alert = ApnsAlert::new("Weekly", "Dave added milk").thread("list-7");
//! match apns.send(&token, sandbox, &alert, &Target { list: 7 }).await? {
//!     Delivery::Sent => {}
//!     Delivery::TokenGone => forget_token(&token).await?,
//! }
//! ```

#[cfg(test)]
mod test;

use std::{
    env,
    fmt::{self, Debug, Formatter},
    time::Instant,
};

use anyhow::{Context, Result, anyhow, bail};
use parking_lot::Mutex;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

use crate::{
    apple_key::{now, signed_token},
    web::install_tls_provider,
};

pub const APNS_HOST: &str = "https://api.push.apple.com";
pub const APNS_SANDBOX_HOST: &str = "https://api.sandbox.push.apple.com";

/// APNs takes a provider token for up to 1 hour, and refuses a new one more
/// often than every 20 minutes.
const TOKEN_SECONDS: u64 = 50 * 60;

/// The reasons of APNs for a token that will never take a push again. The
/// backend forgets such a token.
const GONE_REASONS: [&str; 3] = ["BadDeviceToken", "DeviceTokenNotForTopic", "Unregistered"];

/// The key of a team for APNs, from the Keys page of Apple Developer, and
/// the app the pushes are for.
#[derive(Clone)]
pub struct ApnsConfig {
    pub team_id:      String,
    pub key_id:       String,
    /// The text of the `.p8` file Apple hands out once.
    pub private_key:  String,
    /// The bundle id of the app.
    pub topic:        String,
    pub host:         String,
    pub sandbox_host: String,
}

impl ApnsConfig {
    pub fn new(
        team_id: impl ToString,
        key_id: impl ToString,
        private_key: impl ToString,
        topic: impl ToString,
    ) -> Self {
        Self {
            team_id:      team_id.to_string(),
            key_id:       key_id.to_string(),
            private_key:  private_key.to_string(),
            topic:        topic.to_string(),
            host:         APNS_HOST.to_owned(),
            sandbox_host: APNS_SANDBOX_HOST.to_owned(),
        }
    }

    /// From `APNS_TEAM_ID`, `APNS_KEY_ID` and `APNS_KEY`, the text of the
    /// `.p8` file. `None` when `APNS_KEY` is not set or empty, a server then
    /// sends no push.
    pub fn from_env(topic: impl ToString) -> Result<Option<Self>> {
        let Some(private_key) = env::var("APNS_KEY").ok().filter(|key| !key.trim().is_empty()) else {
            return Ok(None);
        };
        let team_id = env::var("APNS_TEAM_ID").context("APNS_TEAM_ID required with APNS_KEY")?;
        let key_id = env::var("APNS_KEY_ID").context("APNS_KEY_ID required with APNS_KEY")?;
        Ok(Some(Self::new(team_id, key_id, private_key, topic)))
    }
}

impl Debug for ApnsConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ApnsConfig")
            .field("team_id", &self.team_id)
            .field("key_id", &self.key_id)
            .field("topic", &self.topic)
            .field("host", &self.host)
            .field("sandbox_host", &self.sandbox_host)
            .finish_non_exhaustive()
    }
}

/// What the user sees on the lock screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApnsAlert {
    pub title:    String,
    pub body:     String,
    /// Pushes with the same thread are grouped on the lock screen.
    pub thread:   Option<String>,
    /// A push with the same id takes the place of the one shown before.
    pub collapse: Option<String>,
}

impl ApnsAlert {
    pub fn new(title: impl ToString, body: impl ToString) -> Self {
        Self {
            title:    title.to_string(),
            body:     body.to_string(),
            thread:   None,
            collapse: None,
        }
    }

    pub fn thread(mut self, thread: impl ToString) -> Self {
        self.thread = Some(thread.to_string());
        self
    }

    pub fn collapse(mut self, collapse: impl ToString) -> Self {
        self.collapse = Some(collapse.to_string());
        self
    }
}

/// What became of a push.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    Sent,
    /// The token will never take a push again, the app was removed or the
    /// token is of another app. The backend forgets it.
    TokenGone,
}

pub struct Apns {
    http:   Client,
    config: ApnsConfig,
    /// The provider token and the time it was signed.
    token:  Mutex<Option<(String, u64)>>,
}

impl Apns {
    /// Fails when the key is no P-256 key, so a wrong secret shows at the
    /// start of the server and not at the first push.
    pub fn new(config: ApnsConfig) -> Result<Self> {
        install_tls_provider();
        let apns = Self {
            http: Client::new(),
            config,
            token: Mutex::new(None),
        };
        apns.provider_token(now()?)?;
        tracing::info!("APNs ready for {}, key {}", apns.config.topic, apns.config.key_id);
        Ok(apns)
    }

    /// Sends 1 push to the device of `token`, the hex the app got from
    /// `Push::on_token`. `sandbox` is the flag of that token. The fields of
    /// `data` go next to `aps` in the payload, the app reads them with
    /// `PushMessage::data`, so `data` serializes to a map.
    pub async fn send(
        &self,
        token: &str,
        sandbox: bool,
        alert: &ApnsAlert,
        data: &impl Serialize,
    ) -> Result<Delivery> {
        let host = if sandbox {
            &self.config.sandbox_host
        } else {
            &self.config.host
        };
        let url = format!("{host}/3/device/{token}");
        let body = payload(alert, data)?;
        let start = Instant::now();

        let mut request = self
            .http
            .post(&url)
            .bearer_auth(self.provider_token(now()?)?)
            .header("apns-topic", &self.config.topic)
            .header("apns-push-type", "alert")
            .header("apns-priority", "10")
            .header("content-type", "application/json")
            .body(body);
        if let Some(collapse) = &alert.collapse {
            request = request.header("apns-collapse-id", collapse);
        }
        let response = request
            .send()
            .await
            .with_context(|| format!("the push to APNs at {host} failed"))?;
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        let device = token.get(..8).unwrap_or(token);
        tracing::debug!(
            "APNs answered {status} for {device}..., sandbox {sandbox}, in {:?}",
            start.elapsed()
        );

        let delivery = delivery(status, &text);
        if let Err(err) = &delivery
            && err.to_string().contains("ExpiredProviderToken")
        {
            // A clock that jumped makes APNs refuse the token, the next push
            // signs a new one.
            *self.token.lock() = None;
        }
        match &delivery {
            Ok(Delivery::TokenGone) => tracing::info!("APNs says the token {device}... is gone: {text}"),
            Ok(Delivery::Sent) => {}
            Err(err) => tracing::warn!("APNs refused a push to {device}...: {err:#}"),
        }
        delivery
    }

    /// The signed token of the team, made again after `TOKEN_SECONDS`.
    fn provider_token(&self, now: u64) -> Result<String> {
        let mut cached = self.token.lock();
        if let Some((token, signed)) = cached.as_ref()
            && now.saturating_sub(*signed) < TOKEN_SECONDS
        {
            return Ok(token.clone());
        }
        let claims = Claims {
            iss: &self.config.team_id,
            iat: now,
        };
        let token = signed_token(&self.config.private_key, &self.config.key_id, &claims)?;
        tracing::debug!("APNs provider token signed for key {}", self.config.key_id);
        *cached = Some((token.clone(), now));
        Ok(token)
    }
}

#[derive(Serialize)]
struct Claims<'a> {
    iss: &'a str,
    iat: u64,
}

#[derive(Serialize)]
struct Payload<'a, T: Serialize> {
    aps:  Aps<'a>,
    #[serde(flatten)]
    data: &'a T,
}

#[derive(Serialize)]
struct Aps<'a> {
    alert:  Alert<'a>,
    sound:  &'static str,
    #[serde(rename = "thread-id", skip_serializing_if = "Option::is_none")]
    thread: Option<&'a str>,
}

#[derive(Serialize)]
struct Alert<'a> {
    title: &'a str,
    body:  &'a str,
}

/// The answer of APNs to a refused push.
#[derive(Deserialize)]
struct Refusal {
    reason: String,
}

fn payload(alert: &ApnsAlert, data: &impl Serialize) -> Result<String> {
    let payload = Payload {
        aps: Aps {
            alert:  Alert {
                title: &alert.title,
                body:  &alert.body,
            },
            sound:  "default",
            thread: alert.thread.as_deref(),
        },
        data,
    };
    serde_json::to_string(&payload).context("the push payload does not serialize, data must be a map")
}

fn delivery(status: StatusCode, body: &str) -> Result<Delivery> {
    if status.is_success() {
        return Ok(Delivery::Sent);
    }
    let reason = serde_json::from_str::<Refusal>(body)
        .map(|refusal| refusal.reason)
        .map_err(|err| anyhow!("APNs answered {status} with no reason: {err}"))?;
    if status == StatusCode::GONE || GONE_REASONS.contains(&reason.as_str()) {
        return Ok(Delivery::TokenGone);
    }
    bail!("APNs answered {status}, {reason}")
}
