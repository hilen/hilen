//! The session of a page in a browser, kept in a cookie no script can read.
//!
//! A page asks for it with `"cookie": true` in its poll. The server then sets
//! the token as an `HttpOnly` cookie and the answer holds only the user, so no
//! script of the origin ever sees the token, not even a bad one of another
//! page on the same host. The cookie is `SameSite=Strict` and lives only on
//! the path of `PUBLIC_ORIGIN`.
//!
//! A request counts the cookie only together with the header [`HEADER`]. A
//! page of another site cannot send that header unless CORS allows it, so a
//! form or a link from there never acts with the cookie.

use axum::{
    http::{
        HeaderMap, HeaderValue,
        header::{COOKIE, SET_COOKIE},
    },
    response::{IntoResponse, Response},
};
use reqwest::Url;

use crate::{AppError, auth::AuthConfig};

/// The header a page sends with every request that uses the cookie, with the
/// value [`HEADER_VALUE`].
pub const HEADER: &str = "x-hilen-session";
pub const HEADER_VALUE: &str = "cookie";

const NAME: &str = "hilen_session";

/// A session lasts 90 days from its last use, the cookie as long from the
/// login. A user who comes back later logs in again.
const MAX_AGE_SECONDS: u32 = 90 * 24 * 60 * 60;

/// The token of the session cookie, only when the request also has the
/// header that a page of another site cannot send.
pub(crate) fn token(headers: &HeaderMap) -> Option<&str> {
    let marked = headers.get(HEADER).and_then(|value| value.to_str().ok()) == Some(HEADER_VALUE);
    if !marked {
        return None;
    }
    headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, value)| *name == NAME && !value.is_empty())
        .map(|(_, value)| value)
}

/// The answer to a finished poll of a page: the cookie with the token, and
/// `body` with no token in it.
pub(crate) fn keep(config: &AuthConfig, token: &str, body: impl IntoResponse) -> Result<Response, AppError> {
    let value = format!(
        "{NAME}={token}; Max-Age={MAX_AGE_SECONDS}; {}",
        attributes(config)
    );
    Ok(([(SET_COOKIE, header_value(&value)?)], body).into_response())
}

/// An empty answer that drops the cookie when the request came with it.
pub(crate) fn ended(config: &AuthConfig, headers: &HeaderMap) -> Result<Response, AppError> {
    if token(headers).is_none() {
        return Ok(().into_response());
    }
    let value = format!("{NAME}=; Max-Age=0; {}", attributes(config));
    Ok(([(SET_COOKIE, header_value(&value)?)], ()).into_response())
}

fn attributes(config: &AuthConfig) -> String {
    let secure = if config.public_origin.starts_with("https://") {
        "; Secure"
    } else {
        ""
    };
    format!(
        "Path={}; HttpOnly; SameSite=Strict{secure}",
        path_of(&config.public_origin)
    )
}

/// The path of the origin, `/control-center` for an app behind a proxy at
/// that path, `/` for one at the root.
fn path_of(origin: &str) -> String {
    let path = Url::parse(origin).map_or_default(|url| url.path().trim_end_matches('/').to_owned());
    if path.is_empty() { "/".to_owned() } else { path }
}

fn header_value(value: &str) -> Result<HeaderValue, AppError> {
    HeaderValue::from_str(value).map_err(|error| AppError::Internal(anyhow::anyhow!("a bad cookie: {error}")))
}

#[cfg(test)]
mod test {
    use axum::http::{HeaderMap, HeaderValue, header::COOKIE};

    use super::{HEADER, HEADER_VALUE, path_of, token};

    fn headers(cookie: &'static str, marked: bool) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(COOKIE, HeaderValue::from_static(cookie));
        if marked {
            headers.insert(HEADER, HeaderValue::from_static(HEADER_VALUE));
        }
        headers
    }

    #[test]
    fn the_cookie_counts_only_with_the_header() {
        let cookie = "theme=dark; hilen_session=abc; other=1";
        assert_eq!(token(&headers(cookie, true)), Some("abc"));
        assert_eq!(token(&headers(cookie, false)), None);
        assert_eq!(token(&headers("hilen_session=", true)), None);
        assert_eq!(token(&headers("theme=dark", true)), None);
    }

    #[test]
    fn the_cookie_lives_on_the_path_of_the_origin() {
        assert_eq!(path_of("https://host.example/control-center"), "/control-center");
        assert_eq!(path_of("https://host.example/control-center/"), "/control-center");
        assert_eq!(path_of("https://host.example"), "/");
        assert_eq!(path_of("not a url"), "/");
    }
}
