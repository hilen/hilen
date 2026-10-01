//! The database of a backend. Postgres for a backend that runs next to its
//! database, SQLite for one that ships as 1 binary with 1 data file.

use std::str::FromStr;

use anyhow::Result;
use axum::extract::FromRef;
use sqlx::{
    PgPool, SqlitePool,
    postgres::PgPoolOptions,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};

pub async fn build_db(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new().max_connections(8).connect(database_url).await?;
    Ok(pool)
}

/// Opens a SQLite file, `sqlite://data.db`, and creates it when it is not
/// there. Foreign keys are on, SQLite has them off by default. The journal
/// is WAL, so a reader never waits for the writer.
pub async fn build_sqlite(database_url: &str) -> Result<SqlitePool> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new().max_connections(8).connect_with(options).await?;
    Ok(pool)
}

/// Either database, for the parts of this crate that run on both, the login
/// routes and the [`User`](crate::auth::User) extractor. A pool converts into
/// it, so code that has a `PgPool` or a `SqlitePool` passes it as it is.
#[derive(Clone, Debug)]
pub enum Db {
    Postgres(PgPool),
    Sqlite(SqlitePool),
}

impl From<PgPool> for Db {
    fn from(pool: PgPool) -> Self {
        Self::Postgres(pool)
    }
}

impl From<&PgPool> for Db {
    fn from(pool: &PgPool) -> Self {
        Self::Postgres(pool.clone())
    }
}

impl From<SqlitePool> for Db {
    fn from(pool: SqlitePool) -> Self {
        Self::Sqlite(pool)
    }
}

impl From<&SqlitePool> for Db {
    fn from(pool: &SqlitePool) -> Self {
        Self::Sqlite(pool.clone())
    }
}

/// A router whose state is the pool itself gives out the `Db` too.
impl FromRef<PgPool> for Db {
    fn from_ref(pool: &PgPool) -> Self {
        pool.into()
    }
}

impl FromRef<SqlitePool> for Db {
    fn from_ref(pool: &SqlitePool) -> Self {
        pool.into()
    }
}
