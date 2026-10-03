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
        .oneshot(request("GET", &format!("/auth/apple?challenge={challenge}")).body(Body::empty())?)
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
    assert!(text_of(response).await?.contains("You are signed in to Test App"));

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
                .body(Body::from(format!(r#"{{"verifier":"{verifier}"}}"#)))?,
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
