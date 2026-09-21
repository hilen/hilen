//! Google login for an app backend. The engine client is `hilen::login`, the
//! two halves only share the wire.
//!
//! An app never sends its user to Google by itself. It opens
//! `/auth/google?challenge=` of this server in the browser, this server runs
//! the normal Google web login, and the app polls `/auth/poll` until the login
//! is done and its session token comes back. The Google secret stays here.
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

mod google;
mod routes;
mod session;
mod user;
mod wire;

use std::{env, sync::Arc};

use anyhow::{Context, Result};
use axum::extract::FromRef;
use reqwest::Client;
use sqlx::PgPool;

pub use self::{routes::auth_routes, user::User, wire::UserInfo};
use crate::web::install_tls_provider;

const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Creates and updates `users`, `sessions` and `pending_logins`. The record
/// of what ran is in a table of its own, so the app's migrator on the same
/// database never sees these files and never complains about them.
pub async fn migrate(db: &PgPool) -> Result<()> {
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.dangerous_set_table_name("_hilen_auth_migrations");
    migrator.run(db).await.context("the hilen auth migrations failed")
}

/// One Google client of the type "Web application" per app. Its allowed
/// redirect address is [`AuthConfig::redirect_uri`].
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

    /// The address to allow in the Google console.
    pub fn redirect_uri(&self) -> String {
        format!("{}/auth/google/callback", self.public_origin)
    }
}

#[derive(Clone)]
pub struct AuthState {
    pub(crate) db:     PgPool,
    pub(crate) config: Arc<AuthConfig>,
    pub(crate) http:   Client,
}

impl AuthState {
    pub fn new(db: PgPool, config: AuthConfig) -> Self {
        install_tls_provider();

        Self {
            db,
            config: Arc::new(config),
            http: Client::new(),
        }
    }
}

impl FromRef<AuthState> for PgPool {
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
