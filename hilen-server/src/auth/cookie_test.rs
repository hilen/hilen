//! The session of a page in a browser, kept in a cookie no script can read,
//! `cookie.rs`, and what `GET /auth/me` answers such a page, `me.rs`.

use anyhow::{Context, Result};
use axum::{
    Router,
    body::Body,
    http::{
        StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE, COOKIE, SET_COOKIE},
    },
    response::Response,
};
use serde::Serialize;
use tower::ServiceExt;

use crate::auth::{
    cookie,
    routes_test::{apple_app, challenge_of, location, log_in_through, query_value, request, text_of},
    view_as,
};

/// The answer of a poll that asks for its session as a cookie.
async fn poll_for_cookie(app: &Router, verifier: &str, code: &str) -> Result<Response> {
    #[derive(Serialize)]
    struct Poll<'a> {
        verifier: &'a str,
        code:     &'a str,
        cookie:   bool,
    }
    Ok(app
        .clone()
        .oneshot(
            request("POST", "/auth/poll")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&Poll {
                    verifier,
                    code,
                    cookie: true,
                })?))?,
        )
        .await?)
}

/// `GET /auth/me` with these headers.
async fn me_with(app: &Router, headers: &[(&str, &str)]) -> Result<Response> {
    let mut builder = request("GET", "/auth/me");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    Ok(app.clone().oneshot(builder.body(Body::empty())?).await?)
}

/// `GET /auth/me` with the cookie, with or without the marker header.
async fn me_with_cookie(app: &Router, cookie: &str, marked: bool) -> Result<Response> {
    let mut headers = vec![(COOKIE.as_str(), cookie)];
    if marked {
        headers.push((cookie::HEADER, cookie::HEADER_VALUE));
    }
    me_with(app, &headers).await
}

// The failure this prevents: a page in a browser that keeps its token where
// any script of the origin can read it.
#[tokio::test]
async fn a_page_gets_its_session_as_a_cookie_no_script_can_read() -> Result<()> {
    let app = apple_app(|config| config).await?;
    let verifier = "the secret of the page";
    let response = log_in_through(
        &app,
        &format!("/auth/apple?challenge={}&port=40000", challenge_of(verifier)),
    )
    .await?;
    let code = query_value(&location(&response)?, "code")?;

    let response = poll_for_cookie(&app, verifier, &code).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let set_cookie = response
        .headers()
        .get(SET_COOKIE)
        .context("the poll set no cookie")?
        .to_str()?
        .to_owned();
    let body = text_of(response).await?;
    assert!(body.starts_with(r#"{"status":"kept","user":{"#), "{body}");
    assert!(!body.contains("token"), "the answer holds the token: {body}");
    for attribute in [
        "HttpOnly",
        "SameSite=Strict",
        "Secure",
        "Path=/;",
        "Max-Age=7776000",
    ] {
        assert!(
            set_cookie.contains(attribute),
            "{attribute} is missing in {set_cookie}"
        );
    }
    let cookie = set_cookie.split(';').next().context("the cookie has no value")?.to_owned();

    let response = me_with_cookie(&app, &cookie, true).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(text_of(response).await?.contains("ana@example.com"));
    assert_eq!(
        me_with_cookie(&app, &cookie, false).await?.status(),
        StatusCode::UNAUTHORIZED,
        "a request a page of another site could make counts the cookie"
    );

    let response = app
        .clone()
        .oneshot(
            request("POST", "/auth/logout")
                .header(COOKIE, &cookie)
                .header(cookie::HEADER, cookie::HEADER_VALUE)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let cleared = response
        .headers()
        .get(SET_COOKIE)
        .context("the logout kept the cookie")?
        .to_str()?;
    assert!(cleared.starts_with("hilen_session=; Max-Age=0;"), "{cleared}");

    let response = me_with_cookie(&app, &cookie, true).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        text_of(response).await?,
        "null",
        "the ended session still names its user"
    );
    Ok(())
}

// The failure this prevents: a page in a browser with no session asked who
// is signed in, got a 401, and the browser logged it as an error in its
// console at every visit of a person who was not signed in.
#[tokio::test]
async fn a_page_with_no_session_learns_it_with_no_401() -> Result<()> {
    let app = apple_app(|config| config).await?;
    let marked = (cookie::HEADER, cookie::HEADER_VALUE);
    let stale = (COOKIE.as_str(), "hilen_session=a-session-the-server-never-made");

    let response = me_with(&app, &[marked]).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers().get(SET_COOKIE).is_none(),
        "a page with no cookie got one"
    );
    assert_eq!(text_of(response).await?, "null");

    let response = me_with(&app, &[marked, stale]).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let dropped = response
        .headers()
        .get(SET_COOKIE)
        .context("the stale cookie stayed")?
        .to_str()?
        .to_owned();
    assert!(dropped.starts_with("hilen_session=; Max-Age=0;"), "{dropped}");
    assert_eq!(text_of(response).await?, "null");

    // Every other request with no live session keeps the 401. An app with a
    // token learns from it that the server forgot the session.
    let forgotten = (AUTHORIZATION.as_str(), "Bearer a-token-the-server-never-made");
    let view = (view_as::HEADER, "00000000-0000-0000-0000-000000000001");
    for headers in [
        vec![],
        vec![stale],
        vec![forgotten],
        vec![marked, forgotten],
        vec![marked, view],
    ] {
        let response = me_with(&app, &headers).await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{headers:?}");
    }
    Ok(())
}
