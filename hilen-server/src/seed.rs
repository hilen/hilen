//! Test data for a dev server, only in a build with the `dev-login` feature.
//!
//! An app gives [`run`] a function that writes its own rows. It runs only when
//! the start was asked for it with `HILEN_SEED=1`, after the migrations, and
//! never on a database that holds an account of a real login. Accounts come
//! from [`dev_users`], made the way `POST /auth/dev` makes them, so each one
//! signs in there with its email.
//!
//! ```ignore
//! auth::migrate(&db).await?;
//! sqlx::migrate!("./migrations").run(&db).await?;
//! seed::run(&db, || async {
//!     let players = seed::dev_users(&db, (1..=300).map(|n| format!("player{n}@test"))).await?;
//!     write_threads(&db, &players).await
//! })
//! .await?;
//! ```

use std::future::Future;

use anyhow::{Result, bail};
use sqlx::types::Uuid;

use crate::{
    Db,
    auth::{
        identity::{Identity, Provider},
        store::{self, on_db},
    },
};

/// The variable that asks a start to seed.
pub const SEED_ENV: &str = "HILEN_SEED";

/// This start was asked to seed, `HILEN_SEED=1`.
pub fn asked() -> bool {
    std::env::var(SEED_ENV).is_ok_and(|value| value == "1")
}

/// Runs `seed` when the start was asked for it. Refuses a database that has
/// an account of a real login, a seed only ever goes into a dev database.
pub async fn run<F, Fut>(db: impl Into<Db>, seed: F) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<()>>, {
    run_if(asked(), db, seed).await
}

/// `run` with the answer to "was this start asked to seed" given.
pub(crate) async fn run_if<F, Fut>(asked: bool, db: impl Into<Db>, seed: F) -> Result<()>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<()>>, {
    if !asked {
        return Ok(());
    }
    let db = db.into();
    let real = real_accounts(&db).await?;
    if real > 0 {
        bail!(
            "{SEED_ENV}=1 on a database with {real} accounts of a real login, the seed writes only into a dev database"
        );
    }
    tracing::warn!("{SEED_ENV}=1, seeding the dev database");
    seed().await?;
    tracing::warn!("the seed is done");
    Ok(())
}

/// A dev account for each email, the one `POST /auth/dev` makes and logs
/// into. An email that has one already gets it back. The ids come in the order
/// of the emails.
pub async fn dev_users(db: impl Into<Db>, emails: impl IntoIterator<Item = String>) -> Result<Vec<Uuid>> {
    let db = db.into();
    let mut ids = Vec::new();
    for email in emails {
        let email = email.trim().to_lowercase();
        let identity = Identity {
            provider: Provider::Dev,
            subject: email.clone(),
            email,
            name: None,
            picture: None,
            refresh_token: None,
        };
        ids.push(store::login_user(&db, &identity).await?);
    }
    tracing::info!("seeded {} dev accounts", ids.len());
    Ok(ids)
}

/// The accounts with an identity of any provider but the dev login.
async fn real_accounts(db: &Db) -> Result<i64> {
    let both = "SELECT COUNT(DISTINCT user_id) FROM identities WHERE provider <> $1";
    let (count,): (i64,) = on_db!(db, both, both, |sql, pool| sqlx::query_as(sql)
        .bind(Provider::Dev.name())
        .fetch_one(pool)
        .await)?;
    Ok(count)
}

#[cfg(test)]
#[path = "seed_test.rs"]
mod seed_test;
