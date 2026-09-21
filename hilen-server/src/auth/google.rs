//! The two calls of the Google web login. The user goes to `auth_url`, Google
//! sends them back with a code, and `exchange` trades that code for who they
//! are.

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::{from_slice, from_str};

use crate::auth::AuthConfig;

/// Who Google says the user is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GoogleIdentity {
    /// Google's own id of the account. It never changes, an email can.
    pub sub:     String,
    pub email:   String,
    pub name:    String,
    pub picture: Option<String>,
}

pub(crate) fn auth_url(config: &AuthConfig, state: &str) -> Result<String> {
    let url = Url::parse_with_params(
        &config.google_auth_url,
        &[
            ("client_id", config.google_client_id.as_str()),
            ("redirect_uri", &config.redirect_uri()),
            ("response_type", "code"),
            ("scope", "openid email profile"),
            ("state", state),
            // Without it Google logs a user with one account straight in and
            // they never see which account the app got.
            ("prompt", "select_account"),
        ],
    )
    .context("the Google login address does not parse")?;

    Ok(url.into())
}

pub(crate) async fn exchange(http: &Client, config: &AuthConfig, code: &str) -> Result<GoogleIdentity> {
    // The query string of a throwaway address is the same encoding a form
    // body wants, and it saves the `form` feature of reqwest.
    let form = Url::parse_with_params(
        "http://form.invalid/",
        &[
            ("code", code),
            ("client_id", &config.google_client_id),
            ("client_secret", &config.google_client_secret),
            ("redirect_uri", &config.redirect_uri()),
            ("grant_type", "authorization_code"),
        ],
    )?;

    let response = http
        .post(&config.google_token_url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(form.query().unwrap_or_default().to_owned())
        .send()
        .await
        .context("the Google token request failed")?;

    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("Google refused the login code: [{status}] {body}");
    }

    let tokens: TokenResponse = from_str(&body).context("the Google token answer does not parse")?;
    identity_of(&tokens.id_token)
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: String,
}

#[derive(Deserialize)]
struct IdTokenClaims {
    sub:            String,
    email:          String,
    #[serde(default)]
    email_verified: bool,
    name:           Option<String>,
    picture:        Option<String>,
}

/// Reads the claims out of the id token. The signature is not checked, and
/// does not have to be: the token came straight from Google over TLS in answer
/// to our own request, nobody else had a hand on it.
fn identity_of(id_token: &str) -> Result<GoogleIdentity> {
    let payload = id_token
        .split('.')
        .nth(1)
        .ok_or_else(|| anyhow!("the id token has no payload part"))?;
    let json = URL_SAFE_NO_PAD.decode(payload).context("the id token payload is not base64")?;
    let claims: IdTokenClaims = from_slice(&json).context("the id token payload does not parse")?;

    if !claims.email_verified {
        bail!("the email of this Google account is not verified");
    }

    Ok(GoogleIdentity {
        name:    claims.name.unwrap_or_else(|| claims.email.clone()),
        sub:     claims.sub,
        email:   claims.email,
        picture: claims.picture,
    })
}

#[cfg(test)]
mod test {
    use anyhow::Result;
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};

    use super::{auth_url, identity_of};
    use crate::auth::AuthConfig;

    fn config() -> AuthConfig {
        AuthConfig::new(
            "Test App",
            "https://app.example.com",
            "client id",
            "client secret",
        )
    }

    fn id_token(payload: &str) -> String {
        format!("header.{}.signature", URL_SAFE_NO_PAD.encode(payload))
    }

    #[test]
    fn auth_url_carries_the_login() -> Result<()> {
        let url = auth_url(&config(), "the state")?;

        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?"));
        assert!(url.contains("client_id=client+id"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fapp.example.com%2Fauth%2Fgoogle%2Fcallback"));
        assert!(url.contains("state=the+state"));
        assert!(url.contains("scope=openid+email+profile"));
        // The secret belongs only in the token request.
        assert!(!url.contains("secret"));
        Ok(())
    }

    #[test]
    fn identity_from_a_token() -> Result<()> {
        let token = id_token(
            r#"{"sub":"123","email":"a@b.c","email_verified":true,"name":"Anna","picture":"https://p/x.png"}"#,
        );
        let identity = identity_of(&token)?;

        assert_eq!(identity.sub, "123");
        assert_eq!(identity.email, "a@b.c");
        assert_eq!(identity.name, "Anna");
        assert_eq!(identity.picture.as_deref(), Some("https://p/x.png"));
        Ok(())
    }

    #[test]
    fn a_missing_name_falls_back_to_the_email() -> Result<()> {
        let token = id_token(r#"{"sub":"123","email":"a@b.c","email_verified":true}"#);
        assert_eq!(identity_of(&token)?.name, "a@b.c");
        Ok(())
    }

    #[test]
    fn an_unverified_email_is_refused() {
        let token = id_token(r#"{"sub":"123","email":"a@b.c","email_verified":false}"#);
        assert!(identity_of(&token).is_err());
    }

    #[test]
    fn garbage_is_refused() {
        assert!(identity_of("no dots at all").is_err());
        assert!(identity_of("a.!!!.c").is_err());
    }
}
