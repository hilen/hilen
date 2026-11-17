//! The calls of the Discord login. The user goes to `auth_url`, Discord sends
//! them back with a code, and `exchange` trades that code for who they are.
//! Discord has no id token, its answer is an access token that reads the user.

use anyhow::{Context, Result, bail};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::from_str;

use crate::auth::{
    AuthConfig, DiscordConfig,
    identity::{Identity, Provider},
};

pub(crate) fn auth_url(config: &AuthConfig, discord: &DiscordConfig, state: &str) -> Result<String> {
    let url = Url::parse_with_params(
        &discord.auth_url,
        [
            ("client_id", discord.client_id.as_str()),
            ("redirect_uri", &config.discord_redirect_uri()),
            ("response_type", "code"),
            ("scope", "identify email"),
            ("state", state),
        ],
    )
    .context("the Discord login address does not parse")?;

    Ok(url.into())
}

pub(crate) async fn exchange(
    http: &Client,
    config: &AuthConfig,
    discord: &DiscordConfig,
    code: &str,
) -> Result<Identity> {
    // The query string of a throwaway address is the same encoding a form
    // body wants, and it saves the `form` feature of reqwest.
    let form = Url::parse_with_params(
        "http://form.invalid/",
        [
            ("client_id", discord.client_id.as_str()),
            ("client_secret", &discord.client_secret),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &config.discord_redirect_uri()),
        ],
    )?;

    let response = http
        .post(&discord.token_url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(form.query().unwrap_or_default().to_owned())
        .send()
        .await
        .context("the Discord token request failed")?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("Discord refused the login code: [{status}] {body}");
    }
    let tokens: TokenResponse = from_str(&body).context("the Discord token answer does not parse")?;

    let response = http
        .get(&discord.user_url)
        .bearer_auth(&tokens.access_token)
        .send()
        .await
        .context("the Discord user request failed")?;
    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("Discord did not give the user: [{status}] {body}");
    }

    identity_of(&body)
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id:          String,
    username:    String,
    global_name: Option<String>,
    avatar:      Option<String>,
    email:       Option<String>,
    #[serde(default)]
    verified:    bool,
}

fn identity_of(body: &str) -> Result<Identity> {
    let user: DiscordUser = from_str(body).context("the Discord user answer does not parse")?;

    // A first login through a second provider finds its user by the email, so
    // an email nobody proved must not get that far.
    let Some(email) = user.email.filter(|_| user.verified) else {
        bail!("this Discord account has no verified email");
    };
    let picture = user
        .avatar
        .map(|avatar| format!("https://cdn.discordapp.com/avatars/{}/{avatar}.png", user.id));

    Ok(Identity {
        provider: Provider::Discord,
        subject: user.id,
        email,
        name: Some(user.global_name.unwrap_or(user.username)),
        picture,
        refresh_token: None,
    })
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use super::{auth_url, identity_of};
    use crate::auth::{AuthConfig, DiscordConfig};

    #[test]
    fn auth_url_carries_the_login() -> Result<()> {
        let discord = DiscordConfig::new("client id", "client secret");
        let config = AuthConfig::new("App", "https://app.example.com", "id", "secret");
        let url = auth_url(&config, &discord, "the state")?;

        assert!(url.starts_with("https://discord.com/oauth2/authorize?"));
        assert!(url.contains("client_id=client+id"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fapp.example.com%2Fauth%2Fdiscord%2Fcallback"));
        assert!(url.contains("scope=identify+email"));
        assert!(url.contains("state=the+state"));
        assert!(!url.contains("secret"));
        Ok(())
    }

    #[test]
    fn identity_from_a_user() -> Result<()> {
        let identity = identity_of(
            r#"{"id":"228","username":"vladasz.","global_name":"Vladas","avatar":"abc","email":"a@b.c","verified":true}"#,
        )?;

        assert_eq!(identity.subject, "228");
        assert_eq!(identity.email, "a@b.c");
        assert_eq!(identity.name.as_deref(), Some("Vladas"));
        assert_eq!(
            identity.picture.as_deref(),
            Some("https://cdn.discordapp.com/avatars/228/abc.png")
        );
        Ok(())
    }

    #[test]
    fn a_user_with_no_display_name_and_no_avatar() -> Result<()> {
        let identity = identity_of(
            r#"{"id":"228","username":"vladasz.","global_name":null,"avatar":null,"email":"a@b.c","verified":true}"#,
        )?;

        assert_eq!(identity.name.as_deref(), Some("vladasz."));
        assert_eq!(identity.picture, None);
        Ok(())
    }

    #[test]
    fn an_unverified_email_is_refused() {
        assert!(identity_of(r#"{"id":"1","username":"a","email":"a@b.c","verified":false}"#).is_err());
    }

    #[test]
    fn no_email_is_refused() {
        assert!(identity_of(r#"{"id":"1","username":"a","email":null,"verified":true}"#).is_err());
    }
}
