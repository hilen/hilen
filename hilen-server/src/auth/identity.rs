//! Who a provider says the user is, the same shape for every provider.

use anyhow::{Context, Result, anyhow};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::de::DeserializeOwned;
use serde_json::from_slice;

/// Where a user signs in. An app names the ones a user may use with
/// [`crate::auth::AuthConfig::with_only_providers`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Google,
    Apple,
    Discord,
    /// The login of a dev server, see `dev.rs`.
    #[cfg(feature = "dev-login")]
    Dev,
}

impl Provider {
    /// The text in the `provider` column. `dev` is the name of a dev
    /// account, `login_user` keeps such users away from every other one.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Apple => "apple",
            Self::Discord => "discord",
            #[cfg(feature = "dev-login")]
            Self::Dev => "dev",
        }
    }

    /// The provider of a `provider` column, none for a name of another
    /// build, like `dev` in a release.
    pub(crate) fn of_name(name: &str) -> Option<Self> {
        match name {
            "google" => Some(Self::Google),
            "apple" => Some(Self::Apple),
            "discord" => Some(Self::Discord),
            #[cfg(feature = "dev-login")]
            "dev" => Some(Self::Dev),
            _ => None,
        }
    }

    /// The name a person reads, like `Google`.
    pub(crate) const fn title(self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::Apple => "Apple",
            Self::Discord => "Discord",
            #[cfg(feature = "dev-login")]
            Self::Dev => "the dev login",
        }
    }

    /// A first login through this provider finds a user of the same email.
    /// A dev login never does, it makes its own account.
    pub(crate) const fn joins_by_email(self) -> bool {
        match self {
            Self::Google | Self::Apple | Self::Discord => true,
            #[cfg(feature = "dev-login")]
            Self::Dev => false,
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
