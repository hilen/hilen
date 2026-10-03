//! The calls of Sign in with Apple, the web flow. The user goes to `auth_url`,
//! Apple posts them back with a code, and `exchange` trades that code for who
//! they are. `revoke` ends what Apple gave out, for a user who deletes the
//! account.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow, bail};
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use reqwest::{Client, Url};
use ring::{
    rand::SystemRandom,
    signature::{ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair},
};
use serde::{Deserialize, Serialize};
use serde_json::{from_str, to_vec};

use crate::auth::{
    AppleConfig, AuthConfig,
    identity::{Identity, Provider, claims_of},
};

/// Who the client secret is for, the `aud` of its token.
const AUDIENCE: &str = "https://appleid.apple.com";
/// A client secret is made for one request, Apple allows up to 6 months.
const SECRET_SECONDS: u64 = 300;

pub(crate) fn auth_url(config: &AuthConfig, apple: &AppleConfig, state: &str) -> Result<String> {
    let url = Url::parse_with_params(
        &apple.auth_url,
        [
            ("client_id", apple.service_id.as_str()),
            ("redirect_uri", &config.apple_redirect_uri()),
            ("response_type", "code"),
            // Apple gives the name and the email only to a form post.
            ("response_mode", "form_post"),
            ("scope", "name email"),
            ("state", state),
        ],
    )
    .context("the Apple login address does not parse")?;

    Ok(url.into())
}

/// `user` is the `user` field of the form Apple posted, it is there on the
/// first login of an account only.
pub(crate) async fn exchange(
    http: &Client,
    config: &AuthConfig,
    apple: &AppleConfig,
    code: &str,
    user: Option<&str>,
) -> Result<Identity> {
    let body = post_form(
        http,
        &apple.token_url,
        &[
            ("client_id", &apple.service_id),
            ("client_secret", &client_secret(apple, now()?)?),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", &config.apple_redirect_uri()),
        ],
    )
    .await
    .context("Apple refused the login code")?;

    let tokens: TokenResponse = from_str(&body).context("the Apple token answer does not parse")?;
    let mut identity = identity_of(&tokens.id_token)?;
    identity.name = user.and_then(name_of);
    identity.refresh_token = tokens.refresh_token;
    Ok(identity)
}

/// Ends the refresh token, and with it the tie between the Apple account and
/// this app. Apple asks for it when a user deletes the account.
pub(crate) async fn revoke(http: &Client, apple: &AppleConfig, refresh_token: &str) -> Result<()> {
    post_form(
        http,
        &apple.revoke_url,
        &[
            ("client_id", &apple.service_id),
            ("client_secret", &client_secret(apple, now()?)?),
            ("token", refresh_token),
            ("token_type_hint", "refresh_token"),
        ],
    )
    .await
    .context("Apple refused the token revoke")?;
    Ok(())
}

async fn post_form(http: &Client, url: &str, fields: &[(&str, &str)]) -> Result<String> {
    // The query string of a throwaway address is the same encoding a form
    // body wants, and it saves the `form` feature of reqwest.
    let form = Url::parse_with_params("http://form.invalid/", fields)?;

    let response = http
        .post(url)
        .header("content-type", "application/x-www-form-urlencoded")
        .body(form.query().unwrap_or_default().to_owned())
        .send()
        .await
        .with_context(|| format!("the request to {url} failed"))?;

    let status = response.status();
    let body = response.text().await?;
    if !status.is_success() {
        bail!("[{status}] {body}");
    }
    Ok(body)
}

fn now() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("the clock is before 1970")?
        .as_secs())
}

#[derive(Serialize)]
struct SecretHeader<'a> {
    alg: &'a str,
    kid: &'a str,
}

#[derive(Serialize)]
struct SecretClaims<'a> {
    iss: &'a str,
    iat: u64,
    exp: u64,
    aud: &'a str,
    sub: &'a str,
}

/// Apple has no fixed client secret. It is a token the server signs with its
/// Apple key, ES256, that says which team and which service id ask.
fn client_secret(apple: &AppleConfig, now: u64) -> Result<String> {
    let header = SecretHeader {
        alg: "ES256",
        kid: &apple.key_id,
    };
    let claims = SecretClaims {
        iss: &apple.team_id,
        iat: now,
        exp: now + SECRET_SECONDS,
        aud: AUDIENCE,
        sub: &apple.service_id,
    };
    let message = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(to_vec(&header)?),
        URL_SAFE_NO_PAD.encode(to_vec(&claims)?)
    );

    let random = SystemRandom::new();
    let key = EcdsaKeyPair::from_pkcs8(
        &ECDSA_P256_SHA256_FIXED_SIGNING,
        &key_bytes(&apple.private_key)?,
        &random,
    )
    .map_err(|error| anyhow!("the Apple private key is not a P-256 key: {error}"))?;
    let signature = key
        .sign(&random, message.as_bytes())
        .map_err(|error| anyhow!("the Apple client secret was not signed: {error}"))?;

    Ok(format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature)))
}

/// The bytes inside the `.p8` file Apple hands out, a PEM text.
fn key_bytes(pem: &str) -> Result<Vec<u8>> {
    let base64: String = pem.lines().map(str::trim).filter(|line| !line.starts_with("-----")).collect();
    STANDARD.decode(base64).context("the Apple private key is not a PEM text")
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token:      String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct IdTokenClaims {
    sub:            String,
    email:          Option<String>,
    email_verified: Option<Flag>,
}

/// Apple writes its true as the text "true" in some tokens.
#[derive(Deserialize)]
#[serde(untagged)]
enum Flag {
    Bool(bool),
    Text(String),
}

impl Flag {
    fn is_true(&self) -> bool {
        match self {
            Self::Bool(value) => *value,
            Self::Text(text) => text == "true",
        }
    }
}

fn identity_of(id_token: &str) -> Result<Identity> {
    let claims: IdTokenClaims = claims_of(id_token)?;

    let Some(email) = claims.email else {
        bail!("Apple sent no email for this account");
    };
    if !claims.email_verified.is_some_and(|flag| flag.is_true()) {
        bail!("the email of this Apple account is not verified");
    }

    Ok(Identity {
        provider: Provider::Apple,
        subject: claims.sub,
        email,
        name: None,
        picture: None,
        refresh_token: None,
    })
}

#[derive(Deserialize)]
struct PostedUser {
    name: Option<PostedName>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostedName {
    first_name: Option<String>,
    last_name:  Option<String>,
}

/// The name out of the `user` field of the posted form. `None` when the
/// person gave no name.
fn name_of(user: &str) -> Option<String> {
    let posted: PostedUser = match from_str(user) {
        Ok(posted) => posted,
        Err(error) => {
            tracing::warn!("the user field of an Apple login does not parse: {error}");
            return None;
        }
    };
    let name = posted.name?;
    let full = [name.first_name, name.last_name]
        .into_iter()
        .flatten()
        .map(|part| part.trim().to_owned())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    (!full.is_empty()).then_some(full)
}

/// A fresh key the way Apple hands one out, a PEM text.
#[cfg(test)]
pub(crate) fn test_key() -> Result<String> {
    let document = EcdsaKeyPair::generate_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &SystemRandom::new())
        .map_err(|error| anyhow!("no test key: {error}"))?;
    Ok(format!(
        "-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n",
        STANDARD.encode(document)
    ))
}

#[cfg(test)]
mod test {
    use anyhow::{Result, anyhow};
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use ring::{
        rand::SystemRandom,
        signature::{
            ECDSA_P256_SHA256_FIXED, ECDSA_P256_SHA256_FIXED_SIGNING, EcdsaKeyPair, KeyPair,
            UnparsedPublicKey,
        },
    };

    use super::{auth_url, client_secret, identity_of, key_bytes, name_of, test_key};
    use crate::auth::{AppleConfig, AuthConfig};

    fn apple(private_key: &str) -> AppleConfig {
        AppleConfig::new("com.example.web", "TEAM123456", "KEY1234567", private_key)
    }

    fn config() -> AuthConfig {
        AuthConfig::new("Test App", "https://app.example.com", "id", "secret")
    }

    fn id_token(payload: &str) -> String {
        format!("header.{}.signature", URL_SAFE_NO_PAD.encode(payload))
    }

    #[test]
    fn auth_url_carries_the_login() -> Result<()> {
        let url = auth_url(&config(), &apple("no key needed"), "the state")?;

        assert!(url.starts_with("https://appleid.apple.com/auth/authorize?"));
        assert!(url.contains("client_id=com.example.web"));
        assert!(url.contains("redirect_uri=https%3A%2F%2Fapp.example.com%2Fauth%2Fapple%2Fcallback"));
        assert!(url.contains("response_mode=form_post"));
        assert!(url.contains("scope=name+email"));
        assert!(url.contains("state=the+state"));
        Ok(())
    }

    #[test]
    fn client_secret_is_signed_by_the_key() -> Result<()> {
        let pem = test_key()?;
        let secret = client_secret(&apple(&pem), 1000)?;

        let parts: Vec<&str> = secret.split('.').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(
            String::from_utf8(URL_SAFE_NO_PAD.decode(parts[0])?)?,
            r#"{"alg":"ES256","kid":"KEY1234567"}"#
        );
        assert_eq!(
            String::from_utf8(URL_SAFE_NO_PAD.decode(parts[1])?)?,
            r#"{"iss":"TEAM123456","iat":1000,"exp":1300,"aud":"https://appleid.apple.com","sub":"com.example.web"}"#
        );

        let random = SystemRandom::new();
        let key = EcdsaKeyPair::from_pkcs8(&ECDSA_P256_SHA256_FIXED_SIGNING, &key_bytes(&pem)?, &random)
            .map_err(|error| anyhow!("the test key does not load: {error}"))?;
        let message = format!("{}.{}", parts[0], parts[1]);
        UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, key.public_key())
            .verify(message.as_bytes(), &URL_SAFE_NO_PAD.decode(parts[2])?)
            .map_err(|error| anyhow!("the signature does not fit the key: {error}"))?;
        Ok(())
    }

    #[test]
    fn a_wrong_key_is_refused() {
        assert!(client_secret(&apple("not a key"), 1000).is_err());
        assert!(
            client_secret(
                &apple("-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----"),
                1000
            )
            .is_err()
        );
    }

    #[test]
    fn identity_from_a_token() -> Result<()> {
        let token = id_token(r#"{"sub":"001.abc","email":"a@b.c","email_verified":"true"}"#);
        let identity = identity_of(&token)?;
        assert_eq!(identity.subject, "001.abc");
        assert_eq!(identity.email, "a@b.c");

        let token = id_token(r#"{"sub":"001.abc","email":"a@b.c","email_verified":true}"#);
        assert_eq!(identity_of(&token)?.email, "a@b.c");
        Ok(())
    }

    #[test]
    fn a_token_without_a_verified_email_is_refused() {
        assert!(identity_of(&id_token(r#"{"sub":"1"}"#)).is_err());
        assert!(identity_of(&id_token(r#"{"sub":"1","email":"a@b.c"}"#)).is_err());
        assert!(
            identity_of(&id_token(
                r#"{"sub":"1","email":"a@b.c","email_verified":"false"}"#
            ))
            .is_err()
        );
    }

    #[test]
    fn name_from_the_posted_user() {
        let posted = r#"{"name":{"firstName":"Ana","lastName":"Maria"},"email":"a@b.c"}"#;
        assert_eq!(name_of(posted).as_deref(), Some("Ana Maria"));
        assert_eq!(
            name_of(r#"{"name":{"firstName":"Ana","lastName":""}}"#).as_deref(),
            Some("Ana")
        );
        assert_eq!(name_of(r#"{"email":"a@b.c"}"#), None);
        assert_eq!(name_of("not json"), None);
    }
}
