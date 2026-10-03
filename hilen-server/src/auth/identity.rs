//! Who a provider says the user is, the same shape for Google and Apple.

use anyhow::{Context, Result, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::de::DeserializeOwned;
use serde_json::from_slice;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Provider {
    Google,
    Apple,
}

impl Provider {
    /// The text in the `provider` column.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Apple => "apple",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Identity {
    pub provider:      Provider,
    /// The provider's own id of the account. It never changes, an email can.
    pub subject:       String,
    /// Always a verified one, a first login through a second provider finds
    /// its user by it.
    pub email:         String,
    /// Apple sends the name on the first login only.
    pub name:          Option<String>,
    pub picture:       Option<String>,
    /// Apple only, kept to revoke it when the user deletes the account.
    pub refresh_token: Option<String>,
}

/// Reads the claims out of an id token. The signature is not checked, and
/// does not have to be: the token came straight from the provider over TLS in
/// answer to our own request, nobody else had a hand on it.
pub(crate) fn claims_of<Claims: DeserializeOwned>(id_token: &str) -> Result<Claims> {
    let payload = id_token
        .split('.')
        .nth(1)
        .ok_or_else(|| anyhow!("the id token has no payload part"))?;
    let json = URL_SAFE_NO_PAD.decode(payload).context("the id token payload is not base64")?;
    from_slice(&json).context("the id token payload does not parse")
}
