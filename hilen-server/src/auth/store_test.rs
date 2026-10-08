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
    AssertSqlSafe, PgPool, SqlitePool,
    postgres::{PgConnectOptions, PgPoolOptions},
    types::Uuid,
};
use tower::ServiceExt;

use crate::{
    Db,
    auth::{
        User,
        identity::{Identity, Provider},
        migrate,
        session::{self, hash, new_token},
        store, user_of_token,
    },
    build_sqlite,
};

const POSTGRES_URL: &str = "HILEN_TEST_POSTGRES_URL";

/// A fresh database file with the login tables. A file and not memory, each
/// connection of a pool would get a memory database of its own.
pub(crate) async fn sqlite() -> Result<(SqlitePool, Db)> {
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
    on_empty_postgres(async |pool: PgPool| {
        migrate(&pool).await?;
        test(Db::from(&pool)).await
    })
    .await
}

/// The same with no tables yet, for a test that migrates by itself.
pub(crate) async fn on_empty_postgres<Test>(test: Test) -> Result<()>
where Test: AsyncFnOnce(PgPool) -> Result<()> {
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
    let result = test(pool.clone()).await;

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

fn identity(name: &str) -> Identity {
    Identity {
        provider:      Provider::Google,
        subject:       "google-sub-1".to_owned(),
        email:         "ana@example.com".to_owned(),
        name:          Some(name.to_owned()),
        picture:       Some("https://p/ana.png".to_owned()),
        refresh_token: None,
    }
}

/// An Apple login. Apple sends the name on the first one only.
fn apple_identity(email: &str, name: Option<&str>, refresh_token: &str) -> Identity {
    Identity {
        provider:      Provider::Apple,
        subject:       "apple-sub-1".to_owned(),
        email:         email.to_owned(),
        name:          name.map(ToOwned::to_owned),
        picture:       None,
        refresh_token: Some(refresh_token.to_owned()),
    }
}

async fn count(db: &Db, sql: &'static str) -> Result<i64> {
    let (count,): (i64,) = match db {
        Db::Postgres(pool) => sqlx::query_as(sql).fetch_one(pool).await?,
        Db::Sqlite(pool) => sqlx::query_as(sql).fetch_one(pool).await?,
    };
    Ok(count)
}

async fn a_login_runs_from_start_to_session(db: Db) -> Result<()> {
    // The page opens, then reloads: the second start replaces the state.
    assert!(store::start_pending(&db, "challenge", "state-1").await?);
    assert!(store::start_pending(&db, "challenge", "state-2").await?);
    assert!(!store::is_waiting(&db, "state-1", 10).await?);
    assert!(store::is_waiting(&db, "state-2", 10).await?);

    // Nothing to hand over before Google has answered.
    assert_eq!(store::take_finished(&db, "challenge", 10).await?, None);

    let user_id = store::login_user(&db, &identity("Ana")).await?;
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
    let first = store::login_user(&db, &identity("Ana")).await?;
    let second = store::login_user(&db, &identity("Ana Maria")).await?;
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
    assert_eq!(count(&db, "SELECT count(*) FROM pending_logins").await?, 0);

    let user_id = store::login_user(&db, &identity("Ana")).await?;
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

async fn google_and_apple_on_one_email_are_one_user(db: Db) -> Result<()> {
    let google = store::login_user(&db, &identity("Ana")).await?;
    // The email differs in case only, and Apple brings no name this time.
    let apple = store::login_user(&db, &apple_identity("Ana@Example.com", None, "refresh-1")).await?;
    assert_eq!(google, apple, "the same email is the same user");
    assert_eq!(count(&db, "SELECT count(*) FROM users").await?, 1);
    assert_eq!(count(&db, "SELECT count(*) FROM identities").await?, 2);

    let token = session::create(&db, google).await?;
    let user = session::user_of(&db, &token).await?.expect("the session has its user");
    assert_eq!(user.name, "Ana", "a login with no name keeps the name");
    assert_eq!(
        user.picture.as_deref(),
        Some("https://p/ana.png"),
        "a login with no picture keeps the picture"
    );
    Ok(())
}

async fn another_email_is_another_user(db: Db) -> Result<()> {
    let google = store::login_user(&db, &identity("Ana")).await?;
    let relay = "x7k2@privaterelay.appleid.com";
    let apple = store::login_user(&db, &apple_identity(relay, Some("Ana Maria"), "refresh-1")).await?;
    assert_ne!(google, apple, "a hidden email cannot be tied to the Google user");

    let token = session::create(&db, apple).await?;
    let user = session::user_of(&db, &token).await?.expect("the session has its user");
    assert_eq!(user.name, "Ana Maria", "the name of the first Apple login");

    // The second Apple login has no name and a new refresh token.
    let again = store::login_user(&db, &apple_identity(relay, None, "refresh-2")).await?;
    assert_eq!(again, apple, "one Apple account, one id");
    let user = session::user_of(&db, &token).await?.expect("the session has its user");
    assert_eq!(user.name, "Ana Maria", "the name stays");
    assert_eq!(
        store::apple_refresh_tokens(&db, apple).await?,
        ["refresh-2"],
        "the newest token is the one to revoke"
    );
    assert_eq!(
        store::apple_refresh_tokens(&db, google).await?,
        Vec::<String>::new()
    );
    Ok(())
}

async fn a_new_user_with_no_name_is_named_by_the_email(db: Db) -> Result<()> {
    let user_id = store::login_user(&db, &apple_identity("ana@example.com", None, "refresh-1")).await?;
    let token = session::create(&db, user_id).await?;
    let user = session::user_of(&db, &token).await?.expect("the session has its user");
    assert_eq!(user.name, "ana@example.com");
    Ok(())
}

async fn a_token_alone_finds_its_user(db: Db) -> Result<()> {
    let user_id = store::login_user(&db, &identity("Ana")).await?;
    let token = session::create(&db, user_id).await?;

    let user = user_of_token(&db, &token).await?.expect("a live token has its user");
    assert_eq!(
        (user.id, user.email.as_str(), user.name.as_str()),
        (user_id, "ana@example.com", "Ana")
    );
    assert_eq!(user_of_token(&db, "not a token").await?, None);
    assert_eq!(user_of_token(&db, "").await?, None);

    session::delete(&db, &token).await?;
    assert_eq!(user_of_token(&db, &token).await?, None, "a logout ends the token");
    Ok(())
}

async fn a_deleted_user_leaves_nothing(db: Db) -> Result<()> {
    let user_id = store::login_user(&db, &identity("Ana")).await?;
    store::login_user(&db, &apple_identity("ana@example.com", None, "refresh-1")).await?;
    let token = session::create(&db, user_id).await?;
    assert!(store::start_pending(&db, "challenge", "state").await?);
    store::finish_pending(&db, "state", user_id).await?;

    store::delete_user(&db, user_id).await?;

    assert_eq!(session::user_of(&db, &token).await?, None);
    for table in [
        "SELECT count(*) FROM users",
        "SELECT count(*) FROM identities",
        "SELECT count(*) FROM sessions",
        "SELECT count(*) FROM pending_logins",
    ] {
        assert_eq!(count(&db, table).await?, 0, "{table}");
    }

    // The same account can come back, as a new user.
    let back = store::login_user(&db, &identity("Ana")).await?;
    assert_ne!(back, user_id);
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
    let user_id = store::login_user(&db, &identity("Ana")).await?;
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
async fn sqlite_google_and_apple_on_one_email_are_one_user() -> Result<()> {
    google_and_apple_on_one_email_are_one_user(sqlite().await?.1).await
}

#[tokio::test]
async fn sqlite_another_email_is_another_user() -> Result<()> {
    another_email_is_another_user(sqlite().await?.1).await
}

#[tokio::test]
async fn sqlite_a_new_user_with_no_name_is_named_by_the_email() -> Result<()> {
    a_new_user_with_no_name_is_named_by_the_email(sqlite().await?.1).await
}

#[tokio::test]
async fn sqlite_a_token_alone_finds_its_user() -> Result<()> {
    a_token_alone_finds_its_user(sqlite().await?.1).await
}

#[tokio::test]
async fn sqlite_a_deleted_user_leaves_nothing() -> Result<()> {
    a_deleted_user_leaves_nothing(sqlite().await?.1).await
}

/// A database made before the identities table, with a Google user, a live
/// session and a row of an app table that points at the user. The second
/// migration builds `users` again on SQLite, and none of the 3 may get lost.
#[tokio::test]
async fn sqlite_a_database_of_the_first_migration_moves_over() -> Result<()> {
    let path = temp_dir().join(format!("hilen-auth-test-{}.db", new_token()?));
    let pool = build_sqlite(&format!("sqlite://{}", path.display())).await?;
    let db = Db::from(&pool);

    let mut first = sqlx::migrate!("./migrations_sqlite");
    first.dangerous_set_table_name("_hilen_auth_migrations");
    first.migrations = first.migrations.iter().take(1).cloned().collect();
    first.run(&pool).await?;

    let user_id = Uuid::from_u128(7);
    let token = "the token of an old session";
    sqlx::query("INSERT INTO users (id, google_sub, email, name) VALUES ($1, 'google-sub-1', 'ana@example.com', 'Ana')")
        .bind(user_id)
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, unixepoch() + 600)")
        .bind(hash(token))
        .bind(user_id)
        .execute(&pool)
        .await?;
    sqlx::query("CREATE TABLE notes (user_id BLOB NOT NULL REFERENCES users (id) ON DELETE CASCADE)")
        .execute(&pool)
        .await?;
    sqlx::query("INSERT INTO notes (user_id) VALUES ($1)")
        .bind(user_id)
        .execute(&pool)
        .await?;

    migrate(&pool).await?;

    let user = session::user_of(&db, token).await?.expect("the old session still logs in");
    assert_eq!((user.id, user.name.as_str()), (user_id, "Ana"));
    assert_eq!(count(&db, "SELECT count(*) FROM notes").await?, 1);
    assert_eq!(
        store::login_user(&db, &identity("Ana")).await?,
        user_id,
        "the Google account is still this user"
    );

    // Foreign keys are back on and point at the new `users` table.
    let (foreign_keys,): (i64,) = sqlx::query_as("PRAGMA foreign_keys").fetch_one(&pool).await?;
    assert_eq!(foreign_keys, 1);
    store::delete_user(&db, user_id).await?;
    assert_eq!(count(&db, "SELECT count(*) FROM notes").await?, 0);
    assert_eq!(count(&db, "SELECT count(*) FROM sessions").await?, 0);
    Ok(())
}

/// The Postgres twin of the test above.
#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_a_database_of_the_first_migration_moves_over() -> Result<()> {
    on_empty_postgres(async |pool: PgPool| {
        let db = Db::from(&pool);

        let mut first = sqlx::migrate!("./migrations");
        first.dangerous_set_table_name("_hilen_auth_migrations");
        first.migrations = first.migrations.iter().take(1).cloned().collect();
        first.run(&pool).await?;

        let user_id = Uuid::from_u128(7);
        let token = "the token of an old session";
        sqlx::query(
            "INSERT INTO users (id, google_sub, email, name) VALUES ($1, 'google-sub-1', 'ana@example.com', 'Ana')",
        )
        .bind(user_id)
        .execute(&pool)
        .await?;
        sqlx::query(
            "INSERT INTO sessions (token_hash, user_id, expires_at) VALUES ($1, $2, now() + interval '10 minutes')",
        )
        .bind(hash(token))
        .bind(user_id)
        .execute(&pool)
        .await?;

        migrate(&pool).await?;

        let user = session::user_of(&db, token).await?.expect("the old session still logs in");
        assert_eq!((user.id, user.name.as_str()), (user_id, "Ana"));
        assert_eq!(
            store::login_user(&db, &identity("Ana")).await?,
            user_id,
            "the Google account is still this user"
        );
        Ok(())
    })
    .await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_google_and_apple_on_one_email_are_one_user() -> Result<()> {
    on_postgres(google_and_apple_on_one_email_are_one_user).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_another_email_is_another_user() -> Result<()> {
    on_postgres(another_email_is_another_user).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_a_new_user_with_no_name_is_named_by_the_email() -> Result<()> {
    on_postgres(a_new_user_with_no_name_is_named_by_the_email).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_a_token_alone_finds_its_user() -> Result<()> {
    on_postgres(a_token_alone_finds_its_user).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of this file"]
async fn postgres_a_deleted_user_leaves_nothing() -> Result<()> {
    on_postgres(a_deleted_user_leaves_nothing).await
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
