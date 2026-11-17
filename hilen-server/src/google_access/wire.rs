//! What travels between an app and these routes. The client crate never links
//! this one, so `hilen/src/google_access/wire.rs` holds the same shapes. A
//! change here needs the same change there.

use serde::{Deserialize, Serialize};

/// Whose sign in this is. Only a main account is counted for metrics.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role {
    Main,
    Linked,
}

impl Role {
    /// The text in the `role` column.
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Main => "main",
            Self::Linked => "linked",
        }
    }

    pub(crate) fn of(name: &str) -> Self {
        if name == "main" { Self::Main } else { Self::Linked }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct StartQuery {
    pub challenge: String,
    /// The X25519 public key of the device, base64url.
    pub key:       String,
    pub role:      Role,
    #[serde(default)]
    pub platform:  String,
    #[serde(default)]
    pub version:   String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PollRequest {
    pub verifier: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum PollResponse {
    Pending,
    /// `sealed` opens to a [`Granted`] with the private key of the device.
    Done {
        sealed: String,
    },
}

/// What a finished sign in gives the device, as JSON inside the seal.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Granted {
    pub access_token:  String,
    pub refresh_token: String,
    /// Seconds the access token lives from now.
    pub expires_in:    i64,
    /// Google's own id of the account. It never changes, an email can.
    pub subject:       String,
    pub email:         String,
    pub name:          Option<String>,
    pub picture:       Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct RefreshRequest {
    pub refresh_token:   String,
    /// Sent with the token of the main account only, for metrics.
    pub linked_accounts: Option<i32>,
    #[serde(default)]
    pub platform:        String,
    #[serde(default)]
    pub version:         String,
}

#[derive(Deserialize)]
pub(crate) struct RevokeRequest {
    pub token: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RefreshResponse {
    pub access_token: String,
    pub expires_in:   i64,
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::{from_str, to_string};

    use super::{PollResponse, RefreshRequest, RefreshResponse, Role, StartQuery};

    #[test]
    fn request_shapes() -> Result<()> {
        let start: StartQuery = from_str(r#"{"challenge":"c","key":"k","role":"main"}"#)?;
        assert_eq!((start.role, start.platform.as_str()), (Role::Main, ""));
        assert_eq!(Role::of(start.role.name()), Role::Main);
        assert_eq!(Role::of("linked"), Role::Linked);

        let refresh: RefreshRequest = from_str(r#"{"refresh_token":"r"}"#)?;
        assert_eq!(
            (refresh.refresh_token.as_str(), refresh.linked_accounts),
            ("r", None)
        );
        Ok(())
    }

    #[test]
    fn response_shapes() -> Result<()> {
        assert_eq!(to_string(&PollResponse::Pending)?, r#"{"status":"pending"}"#);
        assert_eq!(
            to_string(&PollResponse::Done {
                sealed: "s".to_owned(),
            })?,
            r#"{"status":"done","sealed":"s"}"#
        );
        assert_eq!(
            to_string(&RefreshResponse {
                access_token: "a".to_owned(),
                expires_in:   3600,
            })?,
            r#"{"access_token":"a","expires_in":3600}"#
        );
        Ok(())
    }
}
