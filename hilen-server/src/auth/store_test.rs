//! The login against a real database. Every query of `store.rs` runs here in
//! the order a login runs them, on a SQLite file, and on a Postgres when
//! one is at hand. The Postgres tests are ignored by default, they need
//! `HILEN_TEST_POSTGRES_URL`, the address of a server where the user may
//! create databases:
//!
//! ```text
//! docker run -d --rm --name hilen-pg -e POSTGRES_PASSWORD=pw -p 55432:5432 postgres:17
//! HILEN_TEST_POSTGRES_URL=postgres://postgres:pw@localhost:55432/postgres \
//!     cargo test -p hilen-server postgres -- --ignored
//! ```

use std::{
    env::{temp_dir, var},
    str::FromStr,
};

use anyhow::{Context, Result};
use axum::{
    Router,
    body::Body,
    extract::FromRef,
    http::{Request, StatusCode, header::AUTHORIZATION},
    routing::get,
};
use sqlx::{
    AssertSqlSafe, SqlitePool,
    postgres::{PgConnectOptions, PgPoolOptions},
};
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

const POSTGRES_URL: &str = "HILEN_TEST_POSTGRES_URL";

/// A fresh database file with the login tables. A file and not memory, each
/// connection of a pool would get a memory database of its own.
async fn sqlite() -> Result<(SqlitePool, Db)> {
    let path = temp_dir().join(format!("hilen-auth-test-{}.db", new_token()?));
    let pool = build_sqlite(&format!("sqlite://{}", path.display())).await?;
    migrate(&pool).await?;
    let db = Db::from(&pool);
    Ok((pool, db))
}

/// Runs `test` on a fresh Postgres database with the login tables and
/// drops the database afterwards, passed or not.
async fn on_postgres<Test>(test: Test) -> Result<()>
where Test: AsyncFnOnce(Db) -> Result<()> {
    let url = var(POSTGRES_URL)
        .with_context(|| format!("set {POSTGRES_URL} to a Postgres that may create databases"))?;
    let admin = PgPoolOptions::new().max_connections(1).connect(&url).await?;
    // The name is made here from hex digits, nothing from outside is in it.
    let name = format!("hilen_auth_test_{}", &new_token()?[..16]);
    sqlx::query(AssertSqlSafe(format!(r#"CREATE DATABASE "{name}""#)))
        .execute(&admin)
        .await?;

    let options = PgConnectOptions::from_str(&url)?.database(&name);
    let pool = PgPoolOptions::new().max_connections(8).connect_with(options).await?;
    let result = match migrate(&pool).await {
        Ok(()) => test(Db::from(&pool)).await,
        Err(error) => Err(error),
    };

    pool.close().await;
    sqlx::query(AssertSqlSafe(format!(r#"DROP DATABASE "{name}""#)))
        .execute(&admin)
        .await?;
    result
}

/// One statement of a test, in the words of the database in hand.
async fn run(db: &Db, postgres: &'static str, sqlite: &'static str) -> Result<()> {
    match db {
        Db::Postgres(pool) => {
            sqlx::query(postgres).execute(pool).await?;
        }
        Db::Sqlite(pool) => {
            sqlx::query(sqlite).execute(pool).await?;
        }
    }
    Ok(())
}

async fn pending_count(db: &Db) -> Result<i64> {
    let sql = "SELECT count(*) FROM pending_logins";
    let (count,): (i64,) = match db {
        Db::Postgres(pool) => sqlx::query_as(sql).fetch_one(pool).await?,
        Db::Sqlite(pool) => sqlx::query_as(sql).fetch_one(pool).await?,
    };
    Ok(count)
}

/// Seconds until the session of `token` ends.
async fn seconds_left(db: &Db, token: &str) -> Result<i64> {
    let (seconds,): (i64,) =
        match db {
            Db::Postgres(pool) => sqlx::query_as(
                "SELECT extract(epoch FROM expires_at - now())::BIGINT FROM sessions WHERE token_hash = $1",
            )
            .bind(hash(token))
            .fetch_one(pool)
            .await?,
            Db::Sqlite(pool) => {
                sqlx::query_as("SELECT expires_at - unixepoch() FROM sessions WHERE token_hash = $1")
                    .bind(hash(token))
                    .fetch_one(pool)
                    .await?
            }
        };
    Ok(seconds)
}

fn identity(name: &str) -> GoogleIdentity {
    GoogleIdentity {
        sub:     "google-sub-1".to_owned(),
        email:   "ana@example.com".to_owned(),
        name:    name.to_owned(),
        picture: None,
    }
}

async fn a_login_runs_from_start_to_session(db: Db) -> Result<()> {
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

async fn the_same_google_account_is_the_same_user(db: Db) -> Result<()> {
    let first = store::upsert_user(&db, &identity("Ana")).await?;
    let second = store::upsert_user(&db, &identity("Ana Maria")).await?;
    assert_eq!(first, second, "one account, one id");

    let token = session::create(&db, first).await?;
    let user = session::user_of(&db, &token).await?.expect("the session has its user");
    assert_eq!(user.name, "Ana Maria", "the name follows Google");
    Ok(())
}

async fn old_logins_and_ended_sessions_are_gone(db: Db) -> Result<()> {
    assert!(store::start_pending(&db, "challenge", "state").await?);
    run(
        &db,
        "UPDATE pending_logins SET created_at = now() - interval '11 minutes'",
        "UPDATE pending_logins SET created_at = unixepoch() - 11 * 60",
    )
    .await?;
    assert!(
        !store::is_waiting(&db, "state", 10).await?,
        "11 minutes is too old"
    );
    store::prune_pending(&db, 10).await?;
    assert_eq!(pending_count(&db).await?, 0);

    let user_id = store::upsert_user(&db, &identity("Ana")).await?;
    let token = session::create(&db, user_id).await?;

    // Not used for 2 days: the next use moves the end forward again.
    run(
        &db,
        "UPDATE sessions SET last_used_at = now() - interval '2 days', expires_at = now() + interval '60 \
         seconds'",
        "UPDATE sessions SET last_used_at = unixepoch() - 2 * 86400, expires_at = unixepoch() + 60",
    )
    .await?;
    assert!(session::user_of(&db, &token).await?.is_some());
    let left = seconds_left(&db, &token).await?;
    assert!(left > 89 * 86400, "the session got its 90 days again, {left}");

    run(
        &db,
        "UPDATE sessions SET expires_at = now() - interval '1 second'",
        "UPDATE sessions SET expires_at = unixepoch() - 1",
    )
    .await?;
    assert_eq!(
        session::user_of(&db, &token).await?,
        None,
        "an ended session logs nobody in"
    );

    // A user that goes takes the sessions along, foreign keys are on.
    let token = session::create(&db, user_id).await?;
    run(&db, "DELETE FROM users", "DELETE FROM users").await?;
    assert_eq!(session::user_of(&db, &token).await?, None);
    Ok(())
}

async fn whoami(user: User) -> String {
    user.email
}

/// The `User` extractor on a router whose state is `state`, a pool itself.
async fn a_route_with_a_user_needs_a_session<State>(state: State, db: Db) -> Result<()>
where
    State: Clone + Send + Sync + 'static,
    Db: FromRef<State>, {
    let user_id = store::upsert_user(&db, &identity("Ana")).await?;
    let token = session::create(&db, user_id).await?;

    let app = Router::new().route("/whoami", get(whoami)).with_state(state);
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

#[tokio::test]
async fn sqlite_a_login_runs_from_start_to_session() -> Result<()> {
    a_login_runs_from_start_to_session(sqlite().await?.1).await
}

#[tokio::test]
async fn sqlite_the_same_google_account_is_the_same_user() -> Result<()> {
    the_same_google_account_is_the_same_user(sqlite().await?.1).await
}

#[tokio::test]
async fn sqlite_old_logins_and_ended_sessions_are_gone() -> Result<()> {
    old_logins_and_ended_sessions_are_gone(sqlite().await?.1).await
}

#[tokio::test]
async fn sqlite_a_route_with_a_user_needs_a_session() -> Result<()> {
    let (pool, db) = sqlite().await?;
    a_route_with_a_user_needs_a_session(pool, db).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_a_login_runs_from_start_to_session() -> Result<()> {
    on_postgres(a_login_runs_from_start_to_session).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_the_same_google_account_is_the_same_user() -> Result<()> {
    on_postgres(the_same_google_account_is_the_same_user).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_old_logins_and_ended_sessions_are_gone() -> Result<()> {
    on_postgres(old_logins_and_ended_sessions_are_gone).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_a_route_with_a_user_needs_a_session() -> Result<()> {
    on_postgres(async |db: Db| {
        let Db::Postgres(pool) = db.clone() else {
            anyhow::bail!("the Postgres test got another database");
        };
        a_route_with_a_user_needs_a_session(pool, db).await
    })
    .await
}
