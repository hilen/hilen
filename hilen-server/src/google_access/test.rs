//! The Google access routes from the first page to a renewed token, against
//! a small server that stands in for Google. It runs on a SQLite file, and on
//! a Postgres when one is at hand, see `auth/store_test.rs` for the command.

use std::{
    env::temp_dir,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use axum::{
    Json, Router,
    body::Body,
    extract::State,
    http::{
        Request, StatusCode,
        header::{CONTENT_TYPE, LOCATION},
    },
    response::{IntoResponse, Response},
    routing::post,
    serve,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use hilen_session::DeviceKey;
use http_body_util::BodyExt;
use reqwest::Url;
use serde::Deserialize;
use serde_json::{from_slice, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::{net::TcpListener, spawn};
use tower::ServiceExt;

use crate::{
    Db,
    auth::{session::new_token, store::on_db, store_test::on_empty_postgres},
    build_sqlite,
    google_access::{GoogleAccessConfig, GoogleAccessState, google_access_routes, migrate, wire::Granted},
};

const CALENDAR: &str = "https://www.googleapis.com/auth/calendar";
const APPDATA: &str = "https://www.googleapis.com/auth/drive.appdata";

/// Every form body the stand in got.
type Received = Arc<Mutex<Vec<String>>>;

fn id_token(subject: &str, email: &str) -> String {
    let claims = json!({ "sub": subject, "email": email, "email_verified": true, "name": "Ana" });
    format!("header.{}.signature", URL_SAFE_NO_PAD.encode(claims.to_string()))
}

/// A code or a refresh token names the account it stands for, `main` or
/// `linked`. The refresh token `revoked` is refused the way Google does it.
async fn token(State(received): State<Received>, body: String) -> Response {
    received.lock().expect("the test lock").push(body.clone());

    if body.contains("refresh_token=revoked") {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "invalid_grant" }))).into_response();
    }

    let (subject, email, scope) = if body.contains("main") {
        (
            "sub-main",
            "ana@example.com",
            format!("openid {CALENDAR} {APPDATA}"),
        )
    } else {
        ("sub-linked", "work@example.com", format!("openid {CALENDAR}"))
    };

    if body.contains("grant_type=refresh_token") {
        return Json(json!({
            "access_token": format!("access-2-{subject}"),
            "expires_in": 3599,
            "id_token": id_token(subject, email),
        }))
        .into_response();
    }

    Json(json!({
        "access_token": format!("access-1-{subject}"),
        "refresh_token": format!("refresh-1-{subject}"),
        "expires_in": 3599,
        "scope": scope,
        "id_token": id_token(subject, email),
    }))
    .into_response()
}

/// Google answers a revoke of a token it does not know with `invalid_token`.
async fn revoke(State(received): State<Received>, body: String) -> Response {
    received.lock().expect("the test lock").push(body.clone());
    if body.contains("token=unknown") {
        return (StatusCode::BAD_REQUEST, Json(json!({ "error": "invalid_token" }))).into_response();
    }
    StatusCode::OK.into_response()
}

async fn fake_google(received: Received) -> Result<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let app = Router::new()
        .route("/token", post(token))
        .route("/revoke", post(revoke))
        .with_state(received);
    spawn(async move {
        if let Err(error) = serve(listener, app).await {
            eprintln!("the stand in for Google stopped: {error}");
        }
    });
    Ok(origin)
}

async fn body_of(response: Response) -> Result<Vec<u8>> {
    Ok(response.into_body().collect().await?.to_bytes().to_vec())
}

fn post_json(uri: &str, body: &serde_json::Value) -> Result<Request<Body>> {
    Ok(Request::builder()
        .method("POST")
        .uri(uri)
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))?)
}

#[derive(Deserialize)]
struct Poll {
    status: String,
    sealed: Option<String>,
}

async fn poll(app: &Router, verifier: &str) -> Result<Poll> {
    let response = app
        .clone()
        .oneshot(post_json("/google/poll", &json!({ "verifier": verifier }))?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    Ok(from_slice(&body_of(response).await?)?)
}

/// Every text column of the 2 tables in one string, to look for a token in.
async fn everything_stored(db: &Db) -> Result<String> {
    let pending = "SELECT challenge || state || device_key || COALESCE(sealed, '') FROM google_pending";
    let users = "SELECT subject || email || name || platform || app_version FROM google_users";
    let mut all = String::new();
    for text in [pending, users] {
        let rows: Vec<(String,)> = on_db!(db, text, text, |sql, pool| sqlx::query_as(sql)
            .fetch_all(pool)
            .await)?;
        all.extend(rows.into_iter().map(|(text,)| text));
    }
    Ok(all)
}

async fn users(db: &Db) -> Result<Vec<(String, String, String, i32)>> {
    let both = "SELECT email, platform, app_version, linked_accounts FROM google_users ORDER BY email";
    Ok(on_db!(db, both, both, |sql, pool| sqlx::query_as(sql)
        .fetch_all(pool)
        .await)?)
}

/// One sign in from the start page to the opened tokens.
async fn sign_in(app: &Router, db: &Db, role: &str) -> Result<Granted> {
    // The real device code of the engine opens what this server seals.
    let device = DeviceKey::new()?;
    let key = device.public();
    let verifier = format!("the secret of the app for {role}");
    let challenge = hex::encode(Sha256::digest(verifier.as_bytes()));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!(
                    "/google/start?challenge={challenge}&key={key}&role={role}&platform=macos&version=0.1.0"
                ))
                .body(Body::empty())?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    let location = Url::parse(
        response
            .headers()
            .get(LOCATION)
            .context("the redirect has no address")?
            .to_str()?,
    )?;
    let state = location
        .query_pairs()
        .find(|(name, _)| name == "state")
        .context("the redirect has no state")?
        .1
        .into_owned();

    assert_eq!(poll(app, &verifier).await?.status, "pending");

    // Google sends the browser back with a code.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/google/callback?code=code-{role}&state={state}"))
                .body(Body::empty())?,
        )
        .await?;
    let html = String::from_utf8(body_of(response).await?)?;
    assert!(html.contains("You are signed in to Test App"), "{html}");

    // What waits for the app is in the database, and no token can be read
    // there.
    let stored = everything_stored(db).await?;
    assert!(!stored.contains("access-1"), "an access token is stored");
    assert!(!stored.contains("refresh-1"), "a refresh token is stored");

    let done = poll(app, &verifier).await?;
    assert_eq!(done.status, "done");
    let sealed = done.sealed.context("a finished poll has no sealed text")?;
    let granted: Granted = from_slice(&device.open(&challenge, &sealed)?)?;
    assert!(device.open("another challenge", &sealed).is_err());
    assert!(DeviceKey::new()?.open(&challenge, &sealed).is_err());

    // The hand over happens once.
    assert_eq!(poll(app, &verifier).await?.status, "pending");

    Ok(granted)
}

async fn the_whole_flow(db: Db) -> Result<()> {
    let received = Received::default();
    let google = fake_google(received.clone()).await?;

    let mut config = GoogleAccessConfig::new("Test App", "https://app.example.com", "id", "the-secret")
        .main_scopes([CALENDAR, APPDATA])
        .linked_scopes([CALENDAR]);
    config.token_url = format!("{google}/token");
    config.revoke_url = format!("{google}/revoke");
    let app: Router = google_access_routes(GoogleAccessState::new(db.clone(), config));

    // The main account signs in and is counted.
    let main = sign_in(&app, &db, "main").await?;
    assert_eq!(main.access_token, "access-1-sub-main");
    assert_eq!(main.refresh_token, "refresh-1-sub-main");
    assert_eq!(
        (main.subject.as_str(), main.email.as_str()),
        ("sub-main", "ana@example.com")
    );
    assert_eq!(
        users(&db).await?,
        [(
            "ana@example.com".to_owned(),
            "macos".to_owned(),
            "0.1.0".to_owned(),
            0
        )]
    );

    // A linked account signs in and leaves no trace.
    let linked = sign_in(&app, &db, "linked").await?;
    assert_eq!(linked.refresh_token, "refresh-1-sub-linked");
    assert_eq!(users(&db).await?.len(), 1);
    assert!(!everything_stored(&db).await?.contains("work@example.com"));

    // The code went to Google with the secret, the app never had it.
    let exchange = received.lock().expect("the test lock")[0].clone();
    assert!(exchange.contains("client_secret=the-secret"));
    assert!(exchange.contains("grant_type=authorization_code"));

    // The main account renews its token and says how many accounts it has.
    let response = app
        .clone()
        .oneshot(post_json(
            "/google/refresh",
            &json!({
                "refresh_token": main.refresh_token,
                "linked_accounts": 2,
                "platform": "ios",
                "version": "0.2.0",
            }),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let renewed: serde_json::Value = from_slice(&body_of(response).await?)?;
    assert_eq!(
        renewed,
        json!({ "access_token": "access-2-sub-main", "expires_in": 3599 })
    );
    assert_eq!(
        users(&db).await?,
        [(
            "ana@example.com".to_owned(),
            "ios".to_owned(),
            "0.2.0".to_owned(),
            2
        )]
    );

    // A linked account renews too, and still leaves no trace.
    let response = app
        .clone()
        .oneshot(post_json(
            "/google/refresh",
            &json!({ "refresh_token": linked.refresh_token }),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(users(&db).await?.len(), 1);

    // A token Google no longer takes sends the app back to the sign in.
    let response = app
        .clone()
        .oneshot(post_json(
            "/google/refresh",
            &json!({ "refresh_token": "revoked" }),
        )?)
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    // A revoke goes on to Google, and a token Google forgot is fine too.
    for token in [linked.refresh_token.as_str(), "unknown"] {
        let response = app
            .clone()
            .oneshot(post_json("/google/revoke", &json!({ "token": token }))?)
            .await?;
        assert_eq!(response.status(), StatusCode::OK);
    }
    assert!(
        received
            .lock()
            .expect("the test lock")
            .contains(&"token=refresh-1-sub-linked".to_owned())
    );

    let stored = everything_stored(&db).await?;
    assert!(!stored.contains("refresh-1") && !stored.contains("access-"));
    Ok(())
}

#[tokio::test]
async fn sqlite_tokens_pass_through_and_nothing_readable_is_stored() -> Result<()> {
    let path = temp_dir().join(format!("hilen-google-test-{}.db", new_token()?));
    let pool = build_sqlite(&format!("sqlite://{}", path.display())).await?;
    migrate(&pool).await?;
    the_whole_flow(Db::from(&pool)).await
}

#[tokio::test]
#[ignore = "needs HILEN_TEST_POSTGRES_URL"]
async fn postgres_tokens_pass_through_and_nothing_readable_is_stored() -> Result<()> {
    on_empty_postgres(async |pool: PgPool| {
        migrate(&pool).await?;
        the_whole_flow(Db::from(&pool)).await
    })
    .await
}

#[tokio::test]
async fn a_start_with_a_bad_key_or_challenge_is_refused() -> Result<()> {
    let path = temp_dir().join(format!("hilen-google-test-{}.db", new_token()?));
    let pool = build_sqlite(&format!("sqlite://{}", path.display())).await?;
    migrate(&pool).await?;
    let config = GoogleAccessConfig::new("Test App", "https://app.example.com", "id", "secret");
    let app: Router = google_access_routes(GoogleAccessState::new(&pool, config));

    let challenge = "a1".repeat(32);
    for uri in [
        format!("/google/start?challenge={challenge}&key=short&role=main"),
        format!("/google/start?challenge=nothex&key={}&role=main", "A".repeat(43)),
    ] {
        let response = app.clone().oneshot(Request::builder().uri(uri).body(Body::empty())?).await?;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }
    Ok(())
}
