//! The Apple routes from the first page to the deleted account, on a SQLite
//! file, against a small server that stands in for Apple. Apple refuses
//! `localhost` as a return address, so the real one cannot be asked here.

use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow};
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{
        Request, StatusCode,
        header::{AUTHORIZATION, CONTENT_TYPE, LOCATION},
        request::Builder,
    },
    response::Response,
    routing::post,
    serve,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use http_body_util::BodyExt;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::from_slice;
use sha2::{Digest, Sha256};
use tokio::{net::TcpListener, spawn};
use tower::ServiceExt;

use crate::auth::{AppleConfig, AuthConfig, AuthState, apple::test_key, auth_routes, store_test::sqlite};

#[derive(Serialize)]
struct Tokens {
    id_token:      String,
    refresh_token: &'static str,
}

/// Every form body the stand in got, the token requests and the revokes.
type Received = Arc<Mutex<Vec<String>>>;

async fn token(State(received): State<Received>, body: String) -> Json<Tokens> {
    received.lock().expect("the test lock").push(body);
    let claims = r#"{"sub":"001.abc","email":"ana@example.com","email_verified":"true"}"#;
    Json(Tokens {
        id_token:      format!("header.{}.signature", URL_SAFE_NO_PAD.encode(claims)),
        refresh_token: "refresh-1",
    })
}

async fn revoke(State(received): State<Received>, body: String) {
    received.lock().expect("the test lock").push(body);
}

/// Starts the stand in on a free port and gives its origin.
async fn fake_apple(received: Received) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let app = Router::new()
        .route("/auth/token", post(token))
        .route("/auth/revoke", post(revoke))
        .with_state(received);
    spawn(async move {
        if let Err(error) = serve(listener, app).await {
            eprintln!("the stand in for Apple stopped: {error}");
        }
    });
    Ok(origin)
}

#[derive(Deserialize)]
struct Done {
    status: String,
    token:  String,
    user:   DoneUser,
}

#[derive(Deserialize)]
struct DoneUser {
    name:  String,
    email: String,
}

async fn text_of(response: Response) -> Result<String> {
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(String::from_utf8(bytes.to_vec())?)
}

fn request(method: &str, uri: &str) -> Builder {
    Request::builder().method(method).uri(uri)
}

fn location(response: &Response) -> Result<Url> {
    Ok(Url::parse(
        response
            .headers()
            .get(LOCATION)
            .context("the redirect has no address")?
            .to_str()?,
    )?)
}

fn query_value(url: &Url, name: &str) -> Result<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
        .ok_or_else(|| anyhow!("{url} has no {name}"))
}

fn challenge_of(verifier: &str) -> String {
    hex::encode(Sha256::digest(verifier.as_bytes()))
}

/// The login routes with Apple on, against the stand in for Apple.
async fn apple_app(config: impl FnOnce(AuthConfig) -> AuthConfig) -> Result<Router> {
    let apple_origin = fake_apple(Received::default()).await?;
    let mut apple = AppleConfig::new("com.example.web", "TEAM123456", "KEY1234567", test_key()?);
    apple.token_url = format!("{apple_origin}/auth/token");
    apple.revoke_url = format!("{apple_origin}/auth/revoke");
    let base = AuthConfig::new("Test App", "https://app.example.com", "id", "secret").with_apple(apple);
    let (_pool, db) = sqlite().await?;
    Ok(auth_routes(AuthState::new(db, config(base))))
}

/// A person opens `link` and logs in at Apple. The answer is the last
/// response their browser gets from this server.
async fn log_in_through(app: &Router, link: &str) -> Result<Response> {
    let response = app.clone().oneshot(request("GET", link).body(Body::empty())?).await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER, "{link}");
    let oauth_state = query_value(&location(&response)?, "state")?;

    let form = Url::parse_with_params(
        "http://form.invalid/",
        [("code", "the code"), ("state", &*oauth_state)],
    )?;
    Ok(app
        .clone()
        .oneshot(
            request("POST", "/auth/apple/callback")
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form.query().unwrap_or_default().to_owned()))?,
        )
        .await?)
}

/// The status of a poll, `pending` or `done`.
async fn poll_status(app: &Router, verifier: &str, code: Option<&str>) -> Result<String> {
    #[derive(Deserialize)]
    struct Status {
        status: String,
    }
    #[derive(Serialize)]
    struct Poll<'a> {
        verifier: &'a str,
        code:     Option<&'a str>,
    }

    let response = app
        .clone()
        .oneshot(
            request("POST", "/auth/poll")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_string(&Poll { verifier, code })?))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let status: Status = from_slice(&response.into_body().collect().await?.to_bytes())?;
    Ok(status.status)
}

/// The attack: somebody makes a login link with a verifier of their own and
/// gets another person to log in with it. The person who logged in holds the
/// code that went back, the one who made the link polls with the verifier
/// and never gets a session, on any way back.
#[tokio::test]
async fn a_login_started_by_another_client_never_gives_it_a_session() -> Result<()> {
    let app = apple_app(|config| config).await?;
    let attacker = "the verifier of the one who made the link";
    let challenge = challenge_of(attacker);

    // To a native app on the device of the person who logged in.
    let response = log_in_through(&app, &format!("/auth/apple?challenge={challenge}&port=51234")).await?;
    let back = location(&response)?;
    assert_eq!(back.host_str(), Some("127.0.0.1"));
    assert_eq!(back.port(), Some(51234));
    let code = query_value(&back, "code")?;
    assert_eq!(code.len(), 64);

    for _ in 0..3 {
        assert_eq!(poll_status(&app, attacker, None).await?, "pending");
    }
    assert_eq!(
        poll_status(&app, attacker, Some(&"a1".repeat(32))).await?,
        "pending"
    );

    // A code that went back from this login does not open another login.
    let other = "the verifier of another app";
    let response = log_in_through(
        &app,
        &format!("/auth/apple?challenge={}&port=51235", challenge_of(other)),
    )
    .await?;
    assert_eq!(location(&response)?.port(), Some(51235));
    assert_eq!(poll_status(&app, other, Some(&code)).await?, "pending");

    // To a page of this server, the code stays in the browser of the person.
    let page_verifier = "the verifier behind a page link";
    let page_challenge = challenge_of(page_verifier);
    let response = log_in_through(&app, &format!("/auth/apple?challenge={page_challenge}&page=1")).await?;
    assert_eq!(response.status(), StatusCode::OK);
    let text = text_of(response).await?;
    assert!(text.contains(&format!(
        r#"localStorage.setItem("hilen-login-{page_challenge}", ""#
    )));
    assert_eq!(poll_status(&app, page_verifier, None).await?, "pending");

    // A link with no way back is refused before the provider is asked.
    let response = app
        .clone()
        .oneshot(
            request("GET", &format!("/auth/apple?challenge={}", challenge_of("bare"))).body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    // And a short code that would finish by the verifier alone is not given.
    let response = app
        .oneshot(
            request("POST", "/auth/code")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"challenge":"{challenge}","provider":"apple"}}"#
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}

/// The app that started the login gets the code back and with its verifier
/// the session, once.
#[tokio::test]
async fn the_app_that_started_the_login_gets_the_session_with_the_code() -> Result<()> {
    let app = apple_app(|config| config).await?;
    let verifier = "the secret of the app";

    let response = log_in_through(
        &app,
        &format!("/auth/apple?challenge={}&port=40000", challenge_of(verifier)),
    )
    .await?;
    let code = query_value(&location(&response)?, "code")?;

    assert_eq!(poll_status(&app, verifier, Some(&code)).await?, "done");
    assert_eq!(poll_status(&app, verifier, Some(&code)).await?, "pending");

    // The page the app sends the browser on to.
    let response = app.oneshot(request("GET", "/auth/done").body(Body::empty())?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(text_of(response).await?.contains("You are signed in to Test App"));
    Ok(())
}

#[tokio::test]
async fn an_apple_login_runs_from_the_page_to_the_deleted_account() -> Result<()> {
    let received = Received::default();
    let apple_origin = fake_apple(received.clone()).await?;

    let mut apple = AppleConfig::new("com.example.web", "TEAM123456", "KEY1234567", test_key()?);
    apple.token_url = format!("{apple_origin}/auth/token");
    apple.revoke_url = format!("{apple_origin}/auth/revoke");
    let config = AuthConfig::new("Test App", "https://app.example.com", "id", "secret").with_apple(apple);

    let (_pool, db) = sqlite().await?;
    let app: Router = auth_routes(AuthState::new(db, config));

    let verifier = "the secret of the app";
    let challenge = hex::encode(Sha256::digest(verifier.as_bytes()));

    // The app opens the page, the browser is sent on to Apple.
    let response = app
        .clone()
        .oneshot(request("GET", &format!("/auth/apple?challenge={challenge}&page=1")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = Url::parse(
        response
            .headers()
            .get(LOCATION)
            .context("the redirect has no address")?
            .to_str()?,
    )?;
    assert_eq!(location.host_str(), Some("appleid.apple.com"));
    let (_, oauth_state) = location
        .query_pairs()
        .find(|(name, _)| name == "state")
        .ok_or_else(|| anyhow!("the redirect has no state"))?;

    // Apple posts the form back, with the name of a first login.
    let form = Url::parse_with_params(
        "http://form.invalid/",
        [
            ("code", "the code"),
            ("state", &*oauth_state),
            ("user", r#"{"name":{"firstName":"Ana","lastName":"Maria"}}"#),
        ],
    )?;
    let response = app
        .clone()
        .oneshot(
            request("POST", "/auth/apple/callback")
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form.query().unwrap_or_default().to_owned()))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let text = text_of(response).await?;
    assert!(text.contains("You are signed in to Test App"));
    let key = format!(r#"localStorage.setItem("hilen-login-{challenge}", ""#);
    let code = text
        .split_once(&key)
        .and_then(|(_, rest)| rest.get(..64))
        .context("the last page holds no code")?
        .to_owned();

    let token_request = received.lock().expect("the test lock")[0].clone();
    assert!(token_request.contains("code=the+code"));
    assert!(token_request.contains("client_id=com.example.web"));
    assert!(token_request.contains("grant_type=authorization_code"));
    assert!(token_request.contains("redirect_uri=https%3A%2F%2Fapp.example.com%2Fauth%2Fapple%2Fcallback"));

    // The app polls and gets its session.
    let response = app
        .clone()
        .oneshot(
            request("POST", "/auth/poll")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"verifier":"{verifier}","code":"{code}"}}"#
                )))?,
        )
        .await?;
    let done: Done = from_slice(&response.into_body().collect().await?.to_bytes())?;
    assert_eq!(done.status, "done");
    assert_eq!(done.user.name, "Ana Maria");
    assert_eq!(done.user.email, "ana@example.com");
    let bearer = format!("Bearer {}", done.token);

    // Only a session deletes an account.
    let response = app
        .clone()
        .oneshot(request("POST", "/auth/delete").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let response = app
        .clone()
        .oneshot(
            request("POST", "/auth/delete")
                .header(AUTHORIZATION, &bearer)
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);

    let revoke_request = received.lock().expect("the test lock")[1].clone();
    assert!(revoke_request.contains("token=refresh-1"));
    assert!(revoke_request.contains("token_type_hint=refresh_token"));

    let response = app
        .oneshot(request("GET", "/auth/me").header(AUTHORIZATION, &bearer).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[derive(Deserialize)]
struct Code {
    code: String,
}

/// A TV asks for a code, a person types it on a phone in any case and with a
/// dash, and the phone lands on the login page of the TV's own challenge.
/// That link needs no way back, the TV polls with the verifier alone.
#[tokio::test]
async fn a_short_code_leads_to_the_login_of_its_challenge() -> Result<()> {
    let config = AuthConfig::new("Test App", "https://app.example.com", "id", "secret").with_code_login();
    let (_pool, db) = sqlite().await?;
    let app: Router = auth_routes(AuthState::new(db, config));

    let challenge = "a1".repeat(32);
    let response = app
        .clone()
        .oneshot(
            request("POST", "/auth/code")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"challenge":"{challenge}","provider":"google"}}"#
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let code: Code = from_slice(&response.into_body().collect().await?.to_bytes())?;
    assert_eq!(code.code.len(), 6);

    // The page without a code is the form.
    let response = app.clone().oneshot(request("GET", "/auth/code").body(Body::empty())?).await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(text_of(response).await?.contains("Type the code from the screen."));

    let typed = format!("{}-{}", &code.code[..3], &code.code[3..]).to_lowercase();
    let response = app
        .clone()
        .oneshot(request("GET", &format!("/auth/code?code={typed}")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        response.headers().get(LOCATION).context("the redirect has no address")?,
        &format!("/auth/google?challenge={challenge}")
    );
    let response = app
        .clone()
        .oneshot(request("GET", &format!("/auth/google?challenge={challenge}")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(location(&response)?.host_str(), Some("accounts.google.com"));

    // A code nobody was given shows the form again with the reason.
    let response = app
        .clone()
        .oneshot(request("GET", "/auth/code?code=ZZZZZZ").body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(text_of(response).await?.contains("This code is not known or too old."));

    // Apple is not set up here, its code is refused like its page.
    let response = app
        .clone()
        .oneshot(
            request("POST", "/auth/code")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"challenge":"{challenge}","provider":"apple"}}"#
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .oneshot(
            request("POST", "/auth/code")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"challenge":"short","provider":"google"}"#))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    Ok(())
}

#[tokio::test]
async fn the_apple_routes_are_not_there_without_the_config() -> Result<()> {
    let config = AuthConfig::new("Test App", "https://app.example.com", "id", "secret");
    let (_pool, db) = sqlite().await?;
    let app: Router = auth_routes(AuthState::new(db, config));

    let challenge = "a1".repeat(32);
    let response = app
        .clone()
        .oneshot(request("GET", &format!("/auth/apple?challenge={challenge}")).body(Body::empty())?)
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let response = app
        .oneshot(
            request("POST", "/auth/apple/callback")
                .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("code=c&state=s"))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    Ok(())
}
