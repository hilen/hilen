//! Google and Apple login for an app backend. The engine client is
//! `hilen::login`, the two halves only share the wire.
//!
//! An app never sends its user to a provider by itself. It opens
//! `/auth/google?challenge=` or `/auth/apple?challenge=` of this server in the
//! browser, this server runs the normal web login of that provider, and the
//! app polls `/auth/poll` until the login is done and its session token comes
//! back. The provider secrets stay here.
//!
//! ```ignore
//! let db = build_db(&config.database_url).await?;
//! auth::migrate(&db).await?;
//!
//! let auth = AuthState::new(db.clone(), AuthConfig::from_env("My App")?);
//! let app = base_routes("my-app").merge(auth_routes(auth)).merge(my_routes()).with_state(db);
//! ```
//!
//! After that any route that takes a [`User`] argument is for logged in users
//! only.

mod apple;
mod google;
pub(crate) mod identity;
pub(crate) mod routes;
#[cfg(test)]
mod routes_test;
pub(crate) mod session;
pub(crate) mod store;
#[cfg(test)]
pub(crate) mod store_test;
mod user;
mod wire;

use std::{
    env,
    fmt::{self, Debug, Formatter},
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::extract::FromRef;
use reqwest::Client;

pub use self::{routes::auth_routes, user::User, wire::UserInfo};
use crate::{Db, web::install_tls_provider};

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const APPLE_AUTH_URL: &str = "https://appleid.apple.com/auth/authorize";
const APPLE_TOKEN_URL: &str = "https://appleid.apple.com/auth/token";
const APPLE_REVOKE_URL: &str = "https://appleid.apple.com/auth/revoke";

/// Creates and updates `users`, `identities`, `sessions` and `pending_logins`,
/// on Postgres or on SQLite, each from its own files. The record of what ran
/// is in a table of its own, so the app's migrator on the same database never
/// sees these files and never complains about them.
pub async fn migrate(db: impl Into<Db>) -> Result<()> {
    match db.into() {
        Db::Postgres(pool) => {
            let mut migrator = sqlx::migrate!("./migrations");
            migrator.dangerous_set_table_name("_hilen_auth_migrations");
            migrator.run(&pool).await
        }
        Db::Sqlite(pool) => {
            let mut migrator = sqlx::migrate!("./migrations_sqlite");
            migrator.dangerous_set_table_name("_hilen_auth_migrations");
            migrator.run(&pool).await
        }
    }
    .context("the hilen auth migrations failed")
}

/// One Google client of the type "Web application" per app. Its allowed
/// redirect address is [`AuthConfig::redirect_uri`]. Apple is optional, see
/// [`AppleConfig`].
#[derive(Clone, Debug)]
pub struct AuthConfig {
    /// For a person to read, on the page the browser ends on.
    pub app_name:             String,
    /// Where the world reaches this server, like `https://myapp.example.com`.
    pub public_origin:        String,
    pub google_client_id:     String,
    pub google_client_secret: String,
    pub google_auth_url:      String,
    pub google_token_url:     String,
    /// `None` leaves the Apple routes answering 404.
    pub apple:                Option<AppleConfig>,
}

impl AuthConfig {
    pub fn new(
        app_name: impl ToString,
        public_origin: impl ToString,
        google_client_id: impl ToString,
        google_client_secret: impl ToString,
    ) -> Self {
        Self {
            app_name:             app_name.to_string(),
            public_origin:        public_origin.to_string().trim_end_matches('/').to_owned(),
            google_client_id:     google_client_id.to_string(),
            google_client_secret: google_client_secret.to_string(),
            google_auth_url:      GOOGLE_AUTH_URL.to_owned(),
            google_token_url:     GOOGLE_TOKEN_URL.to_owned(),
            apple:                None,
        }
    }

    /// Turns Sign in with Apple on.
    #[must_use]
    pub fn with_apple(mut self, apple: AppleConfig) -> Self {
        self.apple = Some(apple);
        self
    }

    /// Reads `PUBLIC_ORIGIN`, `GOOGLE_CLIENT_ID` and `GOOGLE_CLIENT_SECRET`,
    /// and the Apple names of [`AppleConfig::from_env`] when they are set.
    pub fn from_env(app_name: &str) -> Result<Self> {
        let mut config = Self::new(
            app_name,
            env::var("PUBLIC_ORIGIN").context("PUBLIC_ORIGIN required")?,
            env::var("GOOGLE_CLIENT_ID").context("GOOGLE_CLIENT_ID required")?,
            env::var("GOOGLE_CLIENT_SECRET").context("GOOGLE_CLIENT_SECRET required")?,
        );
        config.apple = AppleConfig::from_env()?;
        Ok(config)
    }

    /// The address to allow in the Google console.
    pub fn redirect_uri(&self) -> String {
        format!("{}/auth/google/callback", self.public_origin)
    }

    /// The return address to allow on the Apple service id.
    pub fn apple_redirect_uri(&self) -> String {
        format!("{}/auth/apple/callback", self.public_origin)
    }
}

/// Sign in with Apple, the web flow. It needs a service id tied to the App ID
/// of the app, with [`AuthConfig::apple_redirect_uri`] as its return address,
/// and a key with Sign in with Apple enabled.
#[derive(Clone)]
pub struct AppleConfig {
    /// The service id, Apple's client id of the web flow.
    pub service_id:  String,
    pub team_id:     String,
    pub key_id:      String,
    /// The text of the `.p8` file Apple hands out once.
    pub private_key: String,
    pub auth_url:    String,
    pub token_url:   String,
    pub revoke_url:  String,
}

impl AppleConfig {
    pub fn new(
        service_id: impl ToString,
        team_id: impl ToString,
        key_id: impl ToString,
        private_key: impl ToString,
    ) -> Self {
        Self {
            service_id:  service_id.to_string(),
            team_id:     team_id.to_string(),
            key_id:      key_id.to_string(),
            private_key: private_key.to_string(),
            auth_url:    APPLE_AUTH_URL.to_owned(),
            token_url:   APPLE_TOKEN_URL.to_owned(),
            revoke_url:  APPLE_REVOKE_URL.to_owned(),
        }
    }

    /// Reads `APPLE_SERVICE_ID`, `APPLE_TEAM_ID`, `APPLE_KEY_ID` and
    /// `APPLE_PRIVATE_KEY`. `None` when there is no `APPLE_SERVICE_ID`, with it
    /// the other 3 are required. The key may have its line breaks written as
    /// `\n`, for an env file that holds one line per value.
    pub fn from_env() -> Result<Option<Self>> {
        let Ok(service_id) = env::var("APPLE_SERVICE_ID") else {
            return Ok(None);
        };

        Ok(Some(Self::new(
            service_id,
            env::var("APPLE_TEAM_ID").context("APPLE_TEAM_ID required with APPLE_SERVICE_ID")?,
            env::var("APPLE_KEY_ID").context("APPLE_KEY_ID required with APPLE_SERVICE_ID")?,
            env::var("APPLE_PRIVATE_KEY")
                .context("APPLE_PRIVATE_KEY required with APPLE_SERVICE_ID")?
                .replace("\\n", "\n"),
        )))
    }
}

// Written by hand to keep the private key out of every log line.
impl Debug for AppleConfig {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AppleConfig")
            .field("service_id", &self.service_id)
            .field("team_id", &self.team_id)
            .field("key_id", &self.key_id)
            .finish_non_exhaustive()
    }
}

#[derive(Clone)]
pub struct AuthState {
    pub(crate) db:     Db,
    pub(crate) config: Arc<AuthConfig>,
    pub(crate) http:   Client,
}

impl AuthState {
    pub fn new(db: impl Into<Db>, config: AuthConfig) -> Self {
        install_tls_provider();

        Self {
            db:     db.into(),
            config: Arc::new(config),
            http:   Client::new(),
        }
    }
}

impl FromRef<AuthState> for Db {
    fn from_ref(state: &AuthState) -> Self {
        state.db.clone()
    }
}

#[cfg(test)]
mod test {
    use super::AuthConfig;

    #[test]
    fn redirect_uri_ignores_a_trailing_slash() {
        let config = AuthConfig::new("App", "https://app.example.com/", "id", "secret");
        assert_eq!(
            config.redirect_uri(),
            "https://app.example.com/auth/google/callback"
        );
    }
}
