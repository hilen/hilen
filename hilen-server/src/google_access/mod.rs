//! Google API access for an app that keeps its Google tokens on the device.
//! The engine client is `hilen::google_access`, the two halves only share the
//! wire.
//!
//! One Google client of the type "Web application" serves every platform, so
//! a refresh token made on one device works on every other one. Its secret
//! stays here. The tokens only pass through:
//!
//! - The app opens `/google/start` in the browser, with a challenge and a
//!   public key of its own. This server runs the Google web login.
//! - The tokens Google gives are sealed for that public key right away, see
//!   [`seal`]. The app polls `/google/poll` and opens them with its private
//!   key. Nothing this server can read is ever written down.
//! - To renew an access token the app posts its refresh token to
//!   `/google/refresh`. This server adds the client secret, asks Google and
//!   forgets both tokens.
//! - `/google/revoke` ends a refresh token at Google, for an account the user
//!   unlinks or signs out of.
//!
//! The database holds the sign ins that are on their way and one row per
//! main account for metrics, the email, the name and when it was seen.
//!
//! ```ignore
//! let db = build_db(&config.database_url).await?;
//! google_access::migrate(&db).await?;
//!
//! let config = GoogleAccessConfig::from_env("My App")?
//!     .main_scopes(["https://www.googleapis.com/auth/calendar"])
//!     .linked_scopes(["https://www.googleapis.com/auth/calendar"]);
//! let app = base_routes("my-app").merge(google_access_routes(GoogleAccessState::new(db, config)));
//! ```

mod google;
mod routes;
pub(crate) mod seal;
mod store;
#[cfg(test)]
mod test;
mod wire;

use std::{
    env,
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

use anyhow::{Context, Result};
use reqwest::Client;

pub use self::routes::google_access_routes;
use crate::{Db, web::install_tls_provider};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const GOOGLE_REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

/// Creates and updates `google_pending` and `google_users`, on Postgres or on
/// SQLite. The record of what ran is in a table of its own, like the one of
/// [`auth::migrate`](crate::auth::migrate).
pub async fn migrate(db: impl Into<Db>) -> Result<()> {
    match db.into() {
        Db::Postgres(pool) => {
            let mut migrator = sqlx::migrate!("./migrations_google");
            migrator.dangerous_set_table_name("_hilen_google_migrations");
            migrator.run(&pool).await
        }
        Db::Sqlite(pool) => {
            let mut migrator = sqlx::migrate!("./migrations_google_sqlite");
            migrator.dangerous_set_table_name("_hilen_google_migrations");
            migrator.run(&pool).await
        }
    }
    .context("the hilen google access migrations failed")
}

/// One Google client of the type "Web application" per app. Its allowed
/// redirect address is [`GoogleAccessConfig::redirect_uri`].
#[derive(Clone)]
pub struct GoogleAccessConfig {
    /// For a person to read, on the page the browser ends on.
    pub app_name:      String,
    /// Where the world reaches this server, like `https://myapp.example.com`.
    pub public_origin: String,
    pub client_id:     String,
    pub client_secret: String,
    /// Asked of the main account, on top of `openid email profile`.
    pub main_scopes:   Vec<String>,
    /// Asked of an account that is linked to a main one.
    pub linked_scopes: Vec<String>,
    pub auth_url:      String,
    pub token_url:     String,
    pub revoke_url:    String,
}

impl GoogleAccessConfig {
    pub fn new(
        app_name: impl ToString,
        public_origin: impl ToString,
        client_id: impl ToString,
        client_secret: impl ToString,
    ) -> Self {
        Self {
            app_name:      app_name.to_string(),
            public_origin: public_origin.to_string().trim_end_matches('/').to_owned(),
            client_id:     client_id.to_string(),
            client_secret: client_secret.to_string(),
            main_scopes:   Vec::new(),
            linked_scopes: Vec::new(),
            auth_url:      GOOGLE_AUTH_URL.to_owned(),
            token_url:     GOOGLE_TOKEN_URL.to_owned(),
            revoke_url:    GOOGLE_REVOKE_URL.to_owned(),
        }
    }

    /// Reads `PUBLIC_ORIGIN`, `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`.
    pub fn from_env(app_name: &str) -> Result<Self> {
        Ok(Self::new(
            app_name,
            env::var("PUBLIC_ORIGIN").context("PUBLIC_ORIGIN required")?,
            env::var("GOOGLE_CLIENT_ID").context("GOOGLE_CLIENT_ID required")?,
            env::var("GOOGLE_CLIENT_SECRET").context("GOOGLE_CLIENT_SECRET required")?,
        ))
    }

    #[must_use]
    pub fn main_scopes<Scope: ToString>(mut self, scopes: impl IntoIterator<Item = Scope>) -> Self {
        self.main_scopes = scopes.into_iter().map(|scope| scope.to_string()).collect();
        self
    }

    #[must_use]
    pub fn linked_scopes<Scope: ToString>(mut self, scopes: impl IntoIterator<Item = Scope>) -> Self {
        self.linked_scopes = scopes.into_iter().map(|scope| scope.to_string()).collect();
        self
    }

    /// The address to allow in the Google console.
    pub fn redirect_uri(&self) -> String {
        format!("{}/google/callback", self.public_origin)
    }
}

// Written by hand to keep the client secret out of every log line.
impl Debug for GoogleAccessConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("GoogleAccessConfig")
            .field("app_name", &self.app_name)
            .field("public_origin", &self.public_origin)
            .field("client_id", &self.client_id)
            .field("main_scopes", &self.main_scopes)
            .field("linked_scopes", &self.linked_scopes)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct GoogleAccessState {
    pub(crate) db:     Db,
    pub(crate) config: Arc<GoogleAccessConfig>,
    pub(crate) http:   Client,
}

impl GoogleAccessState {
    pub fn new(db: impl Into<Db>, config: GoogleAccessConfig) -> Self {
        install_tls_provider();

        Self {
            db:     db.into(),
            config: Arc::new(config),
            http:   Client::new(),
        }
    }
}
