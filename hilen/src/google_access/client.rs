use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Context, Result, anyhow};
use hilen_session::DeviceKey;
use parking_lot::Mutex;
use serde_json::from_slice;
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    deps::{
        hreads::{now, on_main, sleep, spawn},
        netrun::rest::{Call, RequestError},
    },
    google_access::wire::{
        GoogleAccount, PollRequest, PollResponse, RefreshRequest, RefreshResponse, RevokeRequest, Role,
    },
    system::open_url,
};

const POLL_SECONDS: f32 = 2.0;
/// The server drops a sign in nobody finished after the same time.
const GIVE_UP_SECONDS: f64 = 600.0;
/// A laptop that just woke up or a server restart fails a few polls in a row.
const MAX_FAILURES: u32 = 5;

static SERVER: Mutex<Option<String>> = Mutex::new(None);

/// Every `sign_in` and every `cancel` moves it on. A poll loop that sees
/// another number than its own knows it is no longer wanted.
static ATTEMPT: AtomicU64 = AtomicU64::new(0);

/// A fresh access token for a Google API.
#[derive(Clone, PartialEq, Eq)]
pub struct AccessToken {
    pub token:      String,
    /// Seconds it lives from now.
    pub expires_in: i64,
}

#[derive(Debug, Error)]
pub enum RenewError {
    /// Google no longer takes the refresh token. It was revoked or ran out,
    /// the user signs in to this account again.
    #[error("the Google account has to sign in again")]
    SignInAgain,
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Sign in to Google against the `google_access` routes of a `hilen-server`
/// backend, and the renewal of access tokens through it.
///
/// The app never holds the Google client secret. It opens a page of its own
/// backend in the browser and asks the backend every two seconds if the sign
/// in is done. The tokens come back sealed for a key that is made here for
/// this one sign in, so the backend cannot read what it hands over.
///
/// The callback of `sign_in` runs on the main thread.
pub struct GoogleAccess;

impl GoogleAccess {
    /// The origin of the app backend, like `https://myapp.example.com`. Call
    /// it once at launch, before anything else here.
    pub fn set_server(origin: impl ToString) {
        *SERVER.lock() = Some(origin.to_string().trim_end_matches('/').to_owned());
    }

    /// Opens the sign in page and waits for the user to finish there. Call it
    /// straight from a tap, a browser blocks a page opened any later. A
    /// cancelled sign in never calls `done`.
    pub fn sign_in(role: Role, done: impl FnOnce(Result<GoogleAccount>) + Send + 'static) {
        let attempt = ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;

        let opened = Waiting::new(role).and_then(|waiting| {
            log::info!("google sign in started, role {}", role.name());
            open_url(&waiting.page)?;
            Ok(waiting)
        });

        let waiting = match opened {
            Ok(waiting) => waiting,
            Err(error) => return done(Err(error)),
        };

        spawn(async move {
            if let Some(result) = waiting.account(attempt).await {
                on_main(move || done(result));
            }
        });
    }

    /// Stops waiting for a sign in that is on its way. The page in the
    /// browser stays, it just leads nowhere any more.
    pub fn cancel() {
        ATTEMPT.fetch_add(1, Ordering::SeqCst);
    }

    /// A fresh access token for the account of this refresh token. Pass the
    /// count of linked accounts with the token of the main account, the
    /// backend keeps it for its metrics, and `None` for a linked one.
    pub async fn renew(refresh_token: &str, linked_accounts: Option<u32>) -> Result<AccessToken, RenewError> {
        let call = Call::post(format!("{}/google/refresh", server()?)).body(RefreshRequest {
            refresh_token,
            linked_accounts,
            platform: platform(),
            version: version(),
        });

        match call.send::<RefreshResponse>().await {
            Ok(renewed) => Ok(AccessToken {
                token:      renewed.access_token,
                expires_in: renewed.expires_in,
            }),
            Err(RequestError::Unauthorized) => Err(RenewError::SignInAgain),
            Err(error) => Err(RenewError::Other(error.into())),
        }
    }

    /// Ends a refresh token at Google. Google then drops every token the
    /// account gave this app, on every device, so this is for an account
    /// the user takes away for good.
    pub async fn revoke(refresh_token: &str) -> Result<()> {
        let call =
            Call::post(format!("{}/google/revoke", server()?)).body(RevokeRequest { token: refresh_token });
        Ok(call.send::<()>().await?)
    }
}

#[cfg(any(test, feature = "ui-tests"))]
impl GoogleAccess {
    pub(super) fn swap_server(server: Option<String>) -> Option<String> {
        use std::mem::replace;

        replace(&mut *SERVER.lock(), server)
    }
}

fn server() -> Result<String> {
    SERVER
        .lock()
        .clone()
        .ok_or_else(|| anyhow!("no Google access server, call GoogleAccess::set_server at launch"))
}

const fn platform() -> &'static str {
    if cfg!(target_arch = "wasm32") {
        "web"
    } else {
        std::env::consts::OS
    }
}

/// A test binary has no registered app and so no version.
fn version() -> &'static str {
    #[cfg(all(desktop, not(test)))]
    {
        crate::app::hilen_app_version()
    }
    #[cfg(not(all(desktop, not(test))))]
    {
        ""
    }
}

/// A sign in that is on its way through the browser.
struct Waiting {
    server:    String,
    /// Goes out with the polls only, the page address carries its hash.
    verifier:  String,
    challenge: String,
    device:    DeviceKey,
    /// The page the browser opens.
    page:      String,
}

impl Waiting {
    fn new(role: Role) -> Result<Self> {
        let server = server()?;

        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(|error| anyhow!("no random bytes for the sign in: {error}"))?;
        let verifier = hex::encode(bytes);
        let challenge = hex::encode(Sha256::digest(verifier.as_bytes()));
        let device = DeviceKey::new()?;

        let page = format!(
            "{server}/google/start?challenge={challenge}&key={}&role={}&platform={}&version={}",
            device.public(),
            role.name(),
            platform(),
            version(),
        );

        Ok(Self {
            server,
            verifier,
            challenge,
            device,
            page,
        })
    }

    /// Asks until the user is done in the browser. `None` when this sign in
    /// was cancelled or another one started.
    async fn account(&self, attempt: u64) -> Option<Result<GoogleAccount>> {
        let give_up_at = now() + GIVE_UP_SECONDS;
        let mut failures = 0;

        loop {
            sleep(POLL_SECONDS).await;

            if ATTEMPT.load(Ordering::SeqCst) != attempt {
                return None;
            }
            if now() > give_up_at {
                return Some(Err(anyhow!("the sign in took too long, try again")));
            }

            match self.poll().await {
                Ok(None) => failures = 0,
                Ok(Some(account)) => {
                    log::info!("google sign in done for {}", account.email);
                    return Some(Ok(account));
                }
                Err(error) => {
                    failures += 1;
                    log::warn!("google sign in poll {failures} of {MAX_FAILURES} failed: {error:#}");
                    if failures >= MAX_FAILURES {
                        return Some(Err(error));
                    }
                }
            }
        }
    }

    async fn poll(&self) -> Result<Option<GoogleAccount>> {
        let call = Call::post(format!("{}/google/poll", self.server)).body(PollRequest {
            verifier: &self.verifier,
        });

        match call.send().await? {
            PollResponse::Pending => Ok(None),
            PollResponse::Done { sealed } => {
                let plain = self.device.open(&self.challenge, &sealed)?;
                // A parse error would carry the tokens into a log.
                let account = from_slice(&plain).ok().context("the opened sign in is not an account")?;
                Ok(Some(account))
            }
        }
    }
}

#[cfg(all(test, not_wasm))]
pub(super) mod test {
    use std::env::temp_dir;

    use anyhow::{Context, Result};
    use axum::{
        Json, Router,
        http::StatusCode,
        response::{IntoResponse, Response},
        routing::post,
        serve,
    };
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use hilen_server::{
        build_sqlite,
        google_access::{GoogleAccessConfig, GoogleAccessState, google_access_routes, migrate},
    };
    use reqwest::{Client, Url, header::LOCATION, redirect::Policy};
    use serde_json::json;
    use serial_test::serial;
    use tokio::{net::TcpListener, spawn};

    use super::{GoogleAccess, RenewError, Waiting};
    use crate::google_access::Role;

    const CALENDAR: &str = "https://www.googleapis.com/auth/calendar";

    /// Stands in for the token address of Google.
    async fn token(body: String) -> Response {
        if body.contains("refresh_token=revoked") {
            return (StatusCode::BAD_REQUEST, Json(json!({ "error": "invalid_grant" }))).into_response();
        }

        let claims =
            json!({ "sub": "sub-1", "email": "ana@example.com", "email_verified": true, "name": "Ana" });
        let id_token = format!("header.{}.signature", URL_SAFE_NO_PAD.encode(claims.to_string()));

        if body.contains("grant_type=refresh_token") {
            return Json(json!({ "access_token": "access-2", "expires_in": 3599, "id_token": id_token }))
                .into_response();
        }

        Json(json!({
            "access_token": "access-1",
            "refresh_token": "refresh-1",
            "expires_in": 3599,
            "scope": format!("openid {CALENDAR}"),
            "id_token": id_token,
        }))
        .into_response()
    }

    pub(in crate::google_access) async fn serve_on_a_free_port(app: Router) -> Result<String> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let origin = format!("http://{}", listener.local_addr()?);
        spawn(async move {
            if let Err(error) = serve(listener, app).await {
                eprintln!("a test server stopped: {error}");
            }
        });
        Ok(origin)
    }

    /// The real routes of `hilen-server` on a fresh SQLite file, with the
    /// stand in for Google behind them.
    pub(in crate::google_access) async fn backend() -> Result<String> {
        let google = serve_on_a_free_port(
            Router::new()
                .route("/token", post(token))
                .route("/revoke", post(async || StatusCode::OK)),
        )
        .await?;

        let path = temp_dir().join(format!("hilen-google-client-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        let pool = build_sqlite(&format!("sqlite://{}", path.display())).await?;
        migrate(&pool).await?;

        let mut config = GoogleAccessConfig::new("Test App", "https://app.example.com", "id", "secret")
            .main_scopes([CALENDAR])
            .linked_scopes([CALENDAR]);
        config.token_url = format!("{google}/token");
        config.revoke_url = format!("{google}/revoke");

        serve_on_a_free_port(google_access_routes(GoogleAccessState::new(&pool, config))).await
    }

    /// The whole client against the real server: the page it opens, the
    /// sealed tokens it opens with its own key, and a renewal.
    #[tokio::test]
    #[serial(google_access_server)]
    async fn a_sign_in_and_a_renewal_against_the_real_routes() -> Result<()> {
        let origin = backend().await?;
        let before = GoogleAccess::swap_server(Some(origin.clone()));

        let waiting = Waiting::new(Role::Main)?;
        assert!(waiting.poll().await?.is_none());

        // What the browser does: the page sends it on to Google, and Google
        // sends it back with a code.
        let browser = Client::builder().redirect(Policy::none()).build()?;
        let response = browser.get(&waiting.page).send().await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        let location = Url::parse(
            response
                .headers()
                .get(LOCATION)
                .context("the page did not send the browser on")?
                .to_str()?,
        )?;
        let state = location
            .query_pairs()
            .find(|(name, _)| name == "state")
            .context("no state for Google")?
            .1
            .into_owned();
        let page = browser
            .get(format!("{origin}/google/callback?code=code&state={state}"))
            .send()
            .await?
            .text()
            .await?;
        assert!(page.contains("You are signed in to Test App"), "{page}");

        let account = waiting.poll().await?.context("the sign in is not done")?;
        assert_eq!(account.email, "ana@example.com");
        assert_eq!(account.refresh_token, "refresh-1");
        assert_eq!(account.access_token, "access-1");
        // The hand over happens once.
        assert!(waiting.poll().await?.is_none());

        let renewed = GoogleAccess::renew(&account.refresh_token, Some(1))
            .await
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        assert_eq!((renewed.token.as_str(), renewed.expires_in), ("access-2", 3599));

        assert!(matches!(
            GoogleAccess::renew("revoked", None).await,
            Err(RenewError::SignInAgain)
        ));

        GoogleAccess::swap_server(before);
        Ok(())
    }

    #[test]
    #[serial(google_access_server)]
    fn the_page_carries_the_hash_and_the_public_key_only() -> Result<()> {
        let before = GoogleAccess::swap_server(Some("https://app.example.com/".to_owned()));
        GoogleAccess::set_server("https://app.example.com/");

        let waiting = Waiting::new(Role::Linked)?;
        assert!(waiting.page.starts_with("https://app.example.com/google/start?challenge="));
        assert!(waiting.page.contains(&format!("challenge={}", waiting.challenge)));
        assert!(waiting.page.contains(&format!("key={}", waiting.device.public())));
        assert!(waiting.page.contains("role=linked"));
        assert!(!waiting.page.contains(&waiting.verifier));

        GoogleAccess::swap_server(before);
        Ok(())
    }
}
