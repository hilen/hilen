//! What travels between an app and these routes. The client crate never links
//! this one, so `hilen/src/login/wire.rs` holds the same shapes. A change here
//! needs the same change there.

use serde::{Deserialize, Serialize};

use crate::auth::User;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct UserInfo {
    pub id:      String,
    pub email:   String,
    pub name:    String,
    pub picture: Option<String>,
}

impl From<User> for UserInfo {
    fn from(user: User) -> Self {
        Self {
            id:      user.id.to_string(),
            email:   user.email,
            name:    user.name,
            picture: user.picture,
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct PollRequest {
    pub verifier: String,
}

/// A device with no keyboard asks for a short code a person types on a
/// phone. `provider` is `google` or `apple`.
#[derive(Debug, Deserialize)]
pub(crate) struct CodeRequest {
    pub challenge: String,
    pub provider:  String,
}

#[derive(Debug, Serialize)]
pub(crate) struct CodeResponse {
    pub code: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum PollResponse {
    Pending,
    Done { token: String, user: UserInfo },
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::{from_str, to_string};

    use super::{CodeRequest, CodeResponse, PollResponse, UserInfo};

    #[test]
    fn code_shapes() -> Result<()> {
        let request: CodeRequest = from_str(r#"{"challenge":"c","provider":"google"}"#)?;
        assert_eq!(
            (request.challenge.as_str(), request.provider.as_str()),
            ("c", "google")
        );
        assert_eq!(
            to_string(&CodeResponse {
                code: "ABC234".to_owned(),
            })?,
            r#"{"code":"ABC234"}"#
        );
        Ok(())
    }

    #[test]
    fn response_shapes() -> Result<()> {
        assert_eq!(to_string(&PollResponse::Pending)?, r#"{"status":"pending"}"#);

        let done = PollResponse::Done {
            token: "t".to_owned(),
            user:  UserInfo {
                id:      "1".to_owned(),
                email:   "a@b.c".to_owned(),
                name:    "A".to_owned(),
                picture: None,
            },
        };
        assert_eq!(
            to_string(&done)?,
            r#"{"status":"done","token":"t","user":{"id":"1","email":"a@b.c","name":"A","picture":null}}"#
        );
        Ok(())
    }
}
