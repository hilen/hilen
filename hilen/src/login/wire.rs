//! What travels between an app and the login routes of `hilen-server`. The
//! server crate never links this one, so `hilen-server/src/auth/wire.rs` holds
//! the same shapes. A change here needs the same change there.

pub use hilen_session::LoginUser;
use serde::{Deserialize, Serialize};

/// `code` is the one time code that came back to the app at the end of the
/// login. Only the login of a device with no keyboard has none.
#[derive(Debug, Serialize)]
pub(crate) struct PollRequest<'a> {
    pub verifier: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code:     Option<&'a str>,
}

/// Asks for a short code a person types on a phone, for a device with no
/// keyboard. `provider` is `google` or `apple`.
#[derive(Debug, Serialize)]
pub(crate) struct CodeRequest<'a> {
    pub challenge: &'a str,
    pub provider:  &'a str,
}

/// The login of a dev account, `POST /auth/dev` of a backend built with the
/// `dev-login` feature. The answer is a finished `PollResponse`.
#[cfg(feature = "inspect")]
#[derive(Debug, Serialize)]
pub(crate) struct DevRequest<'a> {
    pub email: &'a str,
}

#[derive(Debug, Deserialize)]
pub(crate) struct CodeResponse {
    pub code: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum PollResponse {
    /// The user has not finished in the browser yet, or has not opened the
    /// page at all.
    Pending,
    Done {
        token: String,
        user:  LoginUser,
    },
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use serde_json::{from_str, to_string};

    use super::{CodeRequest, CodeResponse, PollRequest, PollResponse};

    #[test]
    fn code_shapes() -> Result<()> {
        assert_eq!(
            to_string(&CodeRequest {
                challenge: "c",
                provider:  "google",
            })?,
            r#"{"challenge":"c","provider":"google"}"#
        );
        let response: CodeResponse = from_str(r#"{"code":"ABC234"}"#)?;
        assert_eq!(response.code, "ABC234");
        Ok(())
    }

    #[test]
    fn request_shape() -> Result<()> {
        assert_eq!(
            to_string(&PollRequest {
                verifier: "abc",
                code:     None,
            })?,
            r#"{"verifier":"abc"}"#
        );
        assert_eq!(
            to_string(&PollRequest {
                verifier: "abc",
                code:     Some("c0de"),
            })?,
            r#"{"verifier":"abc","code":"c0de"}"#
        );
        Ok(())
    }

    #[test]
    fn response_shapes() -> Result<()> {
        assert!(matches!(
            from_str(r#"{"status":"pending"}"#)?,
            PollResponse::Pending
        ));

        let done =
            r#"{"status":"done","token":"t","user":{"id":"1","email":"a@b.c","name":"A","picture":null}}"#;
        let PollResponse::Done { token, user } = from_str(done)? else {
            panic!("a done response parsed as pending");
        };
        assert_eq!(token, "t");
        assert_eq!(user.name, "A");
        assert_eq!(user.picture, None);
        Ok(())
    }
}
