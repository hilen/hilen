//! The login on SQLite, against a real database file. Every query of
//! `store.rs` runs here in the order a login runs them.

use std::env::temp_dir;

use anyhow::Result;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header::AUTHORIZATION},
    routing::get,
};
use sqlx::SqlitePool;
use tower::ServiceExt;

use crate::{
    Db,
    auth::{
        User,
        google::GoogleIdentity,
        migrate,
        session::{self, hash, new_token},
        store,
    },
    build_sqlite,
};

/// A fresh database file with the login tables. A file and not memory, each
/// connection of a pool would get a memory database of its own.
async fn fresh() -> Result<(SqlitePool, Db)> {
    let path = temp_dir().join(format!("hilen-auth-test-{}.db", new_token()?));
    let pool = build_sqlite(&format!("sqlite://{}", path.display())).await?;
    migrate(&pool).await?;
    let db = Db::from(&pool);
    Ok((pool, db))
}

fn identity(name: &str) -> GoogleIdentity {
    GoogleIdentity {
        sub:     "google-sub-1".to_owned(),
        email:   "ana@example.com".to_owned(),
        name:    name.to_owned(),
        picture: None,
    }
}

#[tokio::test]
async fn a_login_runs_from_start_to_session() -> Result<()> {
    let (_pool, db) = fresh().await?;

    // The page opens, then reloads: the second start replaces the state.
    assert!(store::start_pending(&db, "challenge", "state-1").await?);
    assert!(store::start_pending(&db, "challenge", "state-2").await?);
    assert!(!store::is_waiting(&db, "state-1", 10).await?);
    assert!(store::is_waiting(&db, "state-2", 10).await?);

    // Nothing to hand over before Google has answered.
    assert_eq!(store::take_finished(&db, "challenge", 10).await?, None);

    let user_id = store::upsert_user(&db, &identity("Ana")).await?;
    store::finish_pending(&db, "state-2", user_id).await?;
    assert!(!store::is_waiting(&db, "state-2", 10).await?);

    // A finished login cannot be started over by somebody else.
    assert!(!store::start_pending(&db, "challenge", "state-3").await?);

    // The hand over happens once.
    assert_eq!(store::take_finished(&db, "challenge", 10).await?, Some(user_id));
    assert_eq!(store::take_finished(&db, "challenge", 10).await?, None);

    let token = session::create(&db, user_id).await?;
    let user = session::user_of(&db, &token).await?.expect("a fresh session has its user");
    assert_eq!((user.id, user.name.as_str()), (user_id, "Ana"));
    assert_eq!(session::user_of(&db, "not a token").await?, None);

    session::delete(&db, &token).await?;
    assert_eq!(session::user_of(&db, &token).await?, None);
    Ok(())
}

#[tokio::test]
async fn the_same_google_account_is_the_same_user() -> Result<()> {
    let (_pool, db) = fresh().await?;

    let first = store::upsert_user(&db, &identity("Ana")).await?;
    let second = store::upsert_user(&db, &identity("Ana Maria")).await?;
    assert_eq!(first, second, "one account, one id");

    let token = session::create(&db, first).await?;
    let user = session::user_of(&db, &token).await?.expect("the session has its user");
    assert_eq!(user.name, "Ana Maria", "the name follows Google");
    Ok(())
}

#[tokio::test]
async fn old_logins_and_ended_sessions_are_gone() -> Result<()> {
    let (pool, db) = fresh().await?;

    assert!(store::start_pending(&db, "challenge", "state").await?);
    sqlx::query("UPDATE pending_logins SET created_at = unixepoch() - 11 * 60")
        .execute(&pool)
        .await?;
    assert!(
        !store::is_waiting(&db, "state", 10).await?,
        "11 minutes is too old"
    );
    store::prune_pending(&db, 10).await?;
    let (left,): (i64,) = sqlx::query_as("SELECT count(*) FROM pending_logins").fetch_one(&pool).await?;
    assert_eq!(left, 0);

    let user_id = store::upsert_user(&db, &identity("Ana")).await?;
    let token = session::create(&db, user_id).await?;

    // Not used for 2 days: the next use moves the end forward again.
    sqlx::query("UPDATE sessions SET last_used_at = unixepoch() - 2 * 86400, expires_at = unixepoch() + 60")
        .execute(&pool)
        .await?;
    assert!(session::user_of(&db, &token).await?.is_some());
    let (seconds_left,): (i64,) =
        sqlx::query_as("SELECT expires_at - unixepoch() FROM sessions WHERE token_hash = $1")
            .bind(hash(&token))
            .fetch_one(&pool)
            .await?;
    assert!(
        seconds_left > 89 * 86400,
        "the session got its 90 days again, {seconds_left}"
    );

    sqlx::query("UPDATE sessions SET expires_at = unixepoch() - 1")
        .execute(&pool)
        .await?;
    assert_eq!(
        session::user_of(&db, &token).await?,
        None,
        "an ended session logs nobody in"
    );

    // A user that goes takes the sessions along, foreign keys are on.
    let token = session::create(&db, user_id).await?;
    sqlx::query("DELETE FROM users").execute(&pool).await?;
    assert_eq!(session::user_of(&db, &token).await?, None);
    Ok(())
}

async fn whoami(user: User) -> String {
    user.email
}

/// The `User` extractor on a router whose state is the SQLite pool itself.
#[tokio::test]
async fn a_route_with_a_user_needs_a_session() -> Result<()> {
    let (pool, db) = fresh().await?;
    let user_id = store::upsert_user(&db, &identity("Ana")).await?;
    let token = session::create(&db, user_id).await?;

    let app = Router::new().route("/whoami", get(whoami)).with_state(pool);
    let ask = |authorization: Option<String>| {
        let mut request = Request::builder().uri("/whoami");
        if let Some(value) = authorization {
            request = request.header(AUTHORIZATION, value);
        }
        app.clone()
            .oneshot(request.body(Body::empty()).expect("a request with no body"))
    };

    assert_eq!(ask(None).await?.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        ask(Some("Bearer wrong".to_owned())).await?.status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        ask(Some(format!("Bearer {token}"))).await?.status(),
        StatusCode::OK
    );
    Ok(())
}
