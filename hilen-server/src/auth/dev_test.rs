//! The dev login against a real SQLite file. It reaches only the accounts it
//! made itself, never a Google or Apple user of the same email.

use anyhow::Result;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use http_body_util::BodyExt;
use serde::Deserialize;
use serde_json::from_slice;
use tower::ServiceExt;

use crate::{
    Db,
    auth::{
        AuthConfig, AuthState, auth_routes,
        identity::{Identity, Provider},
        session, store,
        store_test::sqlite,
    },
};

#[derive(Deserialize)]
struct Done {
    status: String,
    token:  String,
    user:   DoneUser,
}

#[derive(Deserialize)]
struct DoneUser {
    id:    String,
    email: String,
}

async fn dev_app() -> Result<(Router, Db)> {
    let config = AuthConfig::new("Test App", "https://app.example.com", "id", "secret");
    let (_pool, db) = sqlite().await?;
    Ok((auth_routes(AuthState::new(db.clone(), config)), db))
}

async fn dev_login(app: &Router, email: &str) -> Result<(StatusCode, Vec<u8>)> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/dev")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::json!({ "email": email }).to_string()))?,
        )
        .await?;
    let status = response.status();
    Ok((status, response.into_body().collect().await?.to_bytes().to_vec()))
}

async fn logged_in(app: &Router, email: &str) -> Result<Done> {
    let (status, body) = dev_login(app, email).await?;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    let done: Done = from_slice(&body)?;
    assert_eq!(done.status, "done");
    Ok(done)
}

fn google(email: &str) -> Identity {
    Identity {
        provider:      Provider::Google,
        subject:       "google-sub-1".to_owned(),
        email:         email.to_owned(),
        name:          Some("Ana".to_owned()),
        picture:       None,
        refresh_token: None,
    }
}

#[tokio::test]
async fn a_new_email_makes_an_account_and_the_same_email_logs_back_into_it() -> Result<()> {
    let (app, db) = dev_app().await?;

    let first = logged_in(&app, "dev1@test").await?;
    let again = logged_in(&app, " Dev1@Test ").await?;
    let other = logged_in(&app, "dev2@test").await?;

    assert_eq!(first.user.id, again.user.id);
    assert_eq!(again.user.email, "dev1@test");
    assert_ne!(first.user.id, other.user.id);
    assert_ne!(first.token, again.token, "every login is a session of its own");

    let user = session::user_of(&db, &again.token).await?.expect("the token is a live session");
    assert_eq!(user.id.to_string(), first.user.id);
    Ok(())
}

#[tokio::test]
async fn a_dev_login_never_reaches_a_google_user_of_the_same_email() -> Result<()> {
    let (app, db) = dev_app().await?;
    let google_user = store::login_user(&db, &google("ana@example.com")).await?;

    let dev = logged_in(&app, "ana@example.com").await?;
    assert_ne!(dev.user.id, google_user.to_string());
    Ok(())
}

#[tokio::test]
async fn a_google_login_never_joins_a_dev_account_of_the_same_email() -> Result<()> {
    let (app, db) = dev_app().await?;
    let dev = logged_in(&app, "ana@example.com").await?;

    let google_user = store::login_user(&db, &google("Ana@Example.com")).await?;
    assert_ne!(google_user.to_string(), dev.user.id);

    // The Google user is a user of its own from then on.
    assert_eq!(
        store::login_user(&db, &google("ana@example.com")).await?,
        google_user
    );
    assert_eq!(logged_in(&app, "ana@example.com").await?.user.id, dev.user.id);
    Ok(())
}

#[tokio::test]
async fn a_text_that_is_no_email_is_refused() -> Result<()> {
    let (app, _db) = dev_app().await?;
    let long = format!("{}@test", "a".repeat(260));
    for text in ["", "  ", "ana", "@test", "ana@", "a@b@c", long.as_str()] {
        let (status, _) = dev_login(&app, text).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{text:?}");
    }
    Ok(())
}
