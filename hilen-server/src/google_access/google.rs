//! The 3 calls to Google: the address the user is sent to, the trade of the
//! code they come back with for tokens, and the renewal of an access token.

use anyhow::{Context, Result, bail};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::from_str;

use crate::{
    auth::identity::claims_of,
    google_access::{
        GoogleAccessConfig,
        wire::{Granted, Role},
    },
};

const IDENTITY_SCOPES: &str = "openid email profile";

fn scopes_of(config: &GoogleAccessConfig, role: Role) -> &[String] {
    match role {
        Role::Main => &config.main_scopes,
        Role::Linked => &config.linked_scopes,
    }
}

pub(crate) fn auth_url(config: &GoogleAccessConfig, role: Role, state: &str) -> Result<String> {
    let scope = [IDENTITY_SCOPES.to_owned()]
        .iter()
        .chain(scopes_of(config, role))
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");

    let url = Url::parse_with_params(
        &config.auth_url,
        [
            ("client_id", config.client_id.as_str()),
            ("redirect_uri", &config.redirect_uri()),
            ("response_type", "code"),
            ("scope", &scope),
            ("state", state),
            // Google gives a refresh token only for offline access, and only
            // on a consent screen. Without `consent` a second sign in of an
            // account comes back with no refresh token.
            ("access_type", "offline"),
            ("prompt", "consent select_account"),
        ],
    )
    .context("the Google login address does not parse")?;

    Ok(url.into())
}

/// The query string of a throwaway address is the same encoding a form body
/// wants, and it saves the `form` feature of reqwest.
async fn post_form(http: &Client, url: &str, fields: &[(&str, &str)]) -> Result<(u16, String)> {
    let form = Url::parse_with_params("http://form.invalid/", fields)?;
    let response = http
        .post(url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(form.query().unwrap_or_default().to_owned())
        .send()
        .await
        .context("the Google token request failed")?;

    let status = response.status().as_u16();
    Ok((status, response.text().await?))
}

#[derive(Deserialize)]
struct TokenAnswer {
    access_token:  String,
    refresh_token: Option<String>,
    expires_in:    i64,
    id_token:      Option<String>,
    #[serde(default)]
    scope:         String,
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

pub(crate) async fn exchange(
    http: &Client,
    config: &GoogleAccessConfig,
    role: Role,
    code: &str,
) -> Result<Granted> {
    let (status, body) = post_form(
        http,
        &config.token_url,
        &[
            ("code", code),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
            ("redirect_uri", &config.redirect_uri()),
            ("grant_type", "authorization_code"),
        ],
    )
    .await?;
    if status != 200 {
        bail!("Google refused the login code: [{status}] {body}");
    }

    // The body holds tokens, so a parse error must not carry it into a log.
    let answer: TokenAnswer =
        from_str(&body).map_err(|_| anyhow::anyhow!("the Google token answer does not parse"))?;

    // The consent screen lets a user untick a permission.
    let granted: Vec<&str> = answer.scope.split(' ').collect();
    if let Some(missing) = scopes_of(config, role).iter().find(|scope| !granted.contains(&scope.as_str())) {
        bail!("the user did not grant {missing}");
    }

    let refresh_token = answer.refresh_token.context("Google gave no refresh token")?;
    let claims: IdTokenClaims = claims_of(&answer.id_token.context("Google gave no id token")?)?;
    if !claims.email_verified {
        bail!("the email of this Google account is not verified");
    }

    Ok(Granted {
        access_token: answer.access_token,
        refresh_token,
        expires_in: answer.expires_in,
        subject: claims.sub,
        email: claims.email,
        name: claims.name,
        picture: claims.picture,
    })
}

pub(crate) struct Renewed {
    pub access_token: String,
    pub expires_in:   i64,
    /// Whose token it was, when Google says so.
    pub subject:      Option<String>,
}

/// `None` when Google no longer takes the refresh token: it was revoked, it
/// ran out, or the user changed the password.
pub(crate) async fn renew(
    http: &Client,
    config: &GoogleAccessConfig,
    refresh_token: &str,
) -> Result<Option<Renewed>> {
    let (status, body) = post_form(
        http,
        &config.token_url,
        &[
            ("refresh_token", refresh_token),
            ("client_id", &config.client_id),
            ("client_secret", &config.client_secret),
            ("grant_type", "refresh_token"),
        ],
    )
    .await?;
    if status == 400 && body.contains("invalid_grant") {
        return Ok(None);
    }
    if status != 200 {
        bail!("Google refused the token renewal: [{status}] {body}");
    }

    let answer: TokenAnswer =
        from_str(&body).map_err(|_| anyhow::anyhow!("the Google token answer does not parse"))?;
    let subject = answer
        .id_token
        .and_then(|id_token| claims_of::<IdTokenClaims>(&id_token).ok())
        .map(|claims| claims.sub);

    Ok(Some(Renewed {
        access_token: answer.access_token,
        expires_in: answer.expires_in,
        subject,
    }))
}

/// Ends a token at Google, and with a refresh token every access token made
/// from it. A token Google does not know any more counts as ended.
pub(crate) async fn revoke(http: &Client, config: &GoogleAccessConfig, token: &str) -> Result<()> {
    let (status, body) = post_form(http, &config.revoke_url, &[("token", token)]).await?;
    if status == 200 || (status == 400 && body.contains("invalid_token")) {
        return Ok(());
    }
    bail!("Google refused the revoke: [{status}] {body}");
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use super::auth_url;
    use crate::google_access::{GoogleAccessConfig, wire::Role};

    fn config() -> GoogleAccessConfig {
        GoogleAccessConfig::new("Test App", "https://app.example.com", "client id", "secret")
            .main_scopes(["scope/calendar", "scope/appdata"])
            .linked_scopes(["scope/calendar"])
    }

    #[test]
    fn the_main_account_is_asked_for_more_than_a_linked_one() -> Result<()> {
        let main = auth_url(&config(), Role::Main, "state")?;
        assert!(main.contains("scope=openid+email+profile+scope%2Fcalendar+scope%2Fappdata"));
        assert!(main.contains("access_type=offline"));
        assert!(main.contains("prompt=consent+select_account"));
        assert!(main.contains("redirect_uri=https%3A%2F%2Fapp.example.com%2Fgoogle%2Fcallback"));
        assert!(!main.contains("secret"));

        let linked = auth_url(&config(), Role::Linked, "state")?;
        assert!(linked.contains("scope=openid+email+profile+scope%2Fcalendar&"));
        Ok(())
    }
}
