//! What travels between an app and the Google access routes of
//! `hilen-server`. The server crate never links this one, so
//! `hilen-server/src/google_access/wire.rs` holds the same shapes. A change
//! here needs the same change there.

use std::fmt::{self, Debug, Formatter};

use serde::{Deserialize, Serialize};

/// Whose sign in it is. The main account is the one the user of the app is
/// known by, a linked one is another Google account of the same person. The
/// backend asks each for its own set of permissions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Main,
    Linked,
}

impl Role {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Linked => "linked",
        }
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct PollRequest<'a> {
    pub verifier: &'a str,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum PollResponse {
    Pending,
    /// `sealed` opens to a [`GoogleAccount`] with the key of this device.
    Done {
        sealed: String,
    },
}

/// A Google account right after its sign in, with its tokens.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoogleAccount {
    /// Google's own id of the account. It never changes, an email can.
    pub subject:       String,
    pub email:         String,
    pub name:          Option<String>,
    /// A link to the profile picture.
    pub picture:       Option<String>,
    /// Lives until the user or Google ends it. Keep it in a `SecretStore`.
    pub refresh_token: String,
    pub access_token:  String,
    /// Seconds the access token lives from the sign in on.
    pub expires_in:    i64,
}

// Written by hand to keep the tokens out of every log line.
impl Debug for GoogleAccount {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GoogleAccount")
            .field("subject", &self.subject)
            .field("email", &self.email)
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

#[derive(Serialize)]
pub(crate) struct RefreshRequest<'a> {
    pub refresh_token:   &'a str,
    /// Sent with the token of the main account only, for metrics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub linked_accounts: Option<u32>,
    pub platform:        &'a str,
    pub version:         &'a str,
}

#[derive(Serialize)]
pub(crate) struct RevokeRequest<'a> {
    pub token: &'a str,
}

#[derive(Deserialize)]
pub(crate) struct RefreshResponse {
    pub access_token: String,
    pub expires_in:   i64,
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::{from_str, to_string};

    use super::{GoogleAccount, PollRequest, PollResponse, RefreshRequest};

    #[test]
    fn request_shapes() -> Result<()> {
        assert_eq!(to_string(&PollRequest { verifier: "v" })?, r#"{"verifier":"v"}"#);
        assert_eq!(
            to_string(&RefreshRequest {
                refresh_token:   "r",
                linked_accounts: None,
                platform:        "macos",
                version:         "1.0.0",
            })?,
            r#"{"refresh_token":"r","platform":"macos","version":"1.0.0"}"#
        );
        Ok(())
    }

    #[test]
    fn response_shapes() -> Result<()> {
        assert!(matches!(
            from_str(r#"{"status":"pending"}"#)?,
            PollResponse::Pending
        ));
        assert!(matches!(
            from_str(r#"{"status":"done","sealed":"s"}"#)?,
            PollResponse::Done { sealed } if sealed == "s"
        ));
        Ok(())
    }

    #[test]
    fn an_account_never_prints_its_tokens() -> Result<()> {
        let account: GoogleAccount = from_str(
            r#"{"subject":"s","email":"a@b.c","name":null,"picture":null,
                "refresh_token":"secret-r","access_token":"secret-a","expires_in":3599}"#,
        )?;
        let printed = format!("{account:?}");
        assert!(printed.contains("a@b.c"));
        assert!(!printed.contains("secret"));
        Ok(())
    }
}
