use std::sync::{
    LazyLock,
    atomic::{AtomicU64, Ordering},
};

use anyhow::{Result, anyhow, bail};
use parking_lot::Mutex;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::de::DeserializeOwned;
use serde_json::{from_str, to_string};
use sha2::{Digest, Sha256};

use crate::{
    deps::{
        hreads::{now, on_main, sleep, spawn},
        netrun::rest::client,
    },
    login::wire::{LoginUser, PollRequest, PollResponse},
    store::SessionStore,
    system::open_url,
};

const POLL_SECONDS: f32 = 2.0;
/// The server drops a login nobody finished after the same time.
const GIVE_UP_SECONDS: f64 = 600.0;
/// A laptop that just woke up or a server restart fails a few polls in a row.
const MAX_FAILURES: u32 = 5;

static SERVER: Mutex<Option<String>> = Mutex::new(None);
static CLIENT: LazyLock<Client> = LazyLock::new(client);

/// Every `start` and every `cancel` moves it on. A poll loop that sees another
/// number than its own knows it is no longer wanted.
static ATTEMPT: AtomicU64 = AtomicU64::new(0);

/// Google login against the `auth` routes of a `hilen-server` backend.
///
/// The app never talks to Google. It opens a page of its own backend in the
/// browser and asks the backend every two seconds if the login is done. The
/// page address carries only the hash of a secret made here, the secret itself
/// goes out with the polls, so the browser history is of no use to anyone.
///
/// Every callback runs on the main thread.
pub struct GoogleLogin;

impl GoogleLogin {
    /// The origin of the app backend, like `https://myapp.example.com`. Call
    /// it once at launch, before anything else here.
    pub fn set_server(origin: impl ToString) {
        *SERVER.lock() = Some(origin.to_string().trim_end_matches('/').to_owned());
    }

    /// Opens the login page and waits for the user to finish there. Call it
    /// straight from a tap, a browser blocks a page opened any later. A
    /// cancelled login never calls `done`.
    pub fn start(done: impl FnOnce(Result<LoginUser>) + Send + 'static) {
        let attempt = ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;

        let opened = server().and_then(|server| {
            let verifier = new_verifier()?;
            open_url(format!("{server}/auth/google?challenge={}", challenge(&verifier)))?;
            Ok((server, verifier))
        });

        let (server, verifier) = match opened {
            Ok(opened) => opened,
            Err(error) => return done(Err(error)),
        };

        spawn(async move {
            if let Some(result) = wait_for_login(&server, &verifier, attempt).await {
                on_main(move || done(result));
            }
        });
    }

    /// Stops waiting for a login started with `start`.
    pub fn cancel() {
        ATTEMPT.fetch_add(1, Ordering::SeqCst);
    }

    /// The session token, for the `Authorization: Bearer` header of the app's
    /// own requests. `None` when nobody is logged in.
    pub fn token() -> Option<String> {
        SessionStore::load()
    }

    /// Who the stored session belongs to, for the app launch. `None` when
    /// nobody is logged in, also when the server no longer knows the session,
    /// which then is forgotten here too.
    pub fn current_user(done: impl FnOnce(Result<Option<LoginUser>>) + Send + 'static) {
        spawn(async move {
            let result = fetch_current_user().await;
            on_main(move || done(result));
        });
    }

    /// Ends the session on the server and forgets it here. The local half
    /// happens even when the server cannot be reached.
    pub fn logout(done: impl FnOnce(Result<()>) + Send + 'static) {
        spawn(async move {
            let result = end_session().await;
            on_main(move || done(result));
        });
    }
}

#[cfg(feature = "ui-tests")]
impl GoogleLogin {
    /// Puts another server in and hands the old one back, for a test that has
    /// to be sure a tap cannot open a real login page.
    pub(super) fn swap_server(server: Option<String>) -> Option<String> {
        use std::mem::replace;

        replace(&mut *SERVER.lock(), server)
    }
}

fn server() -> Result<String> {
    SERVER
        .lock()
        .clone()
        .ok_or_else(|| anyhow!("no login server, call GoogleLogin::set_server at launch"))
}

fn new_verifier() -> Result<String> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|error| anyhow!("no random bytes for the login: {error}"))?;
    Ok(hex::encode(bytes))
}

/// What the browser gets to see of the verifier.
fn challenge(verifier: &str) -> String {
    hex::encode(Sha256::digest(verifier.as_bytes()))
}

/// `None` when the attempt was cancelled or replaced by a newer one.
async fn wait_for_login(server: &str, verifier: &str, attempt: u64) -> Option<Result<LoginUser>> {
    let give_up_at = now() + GIVE_UP_SECONDS;
    let mut failures = 0;

    loop {
        sleep(POLL_SECONDS).await;

        if ATTEMPT.load(Ordering::SeqCst) != attempt {
            return None;
        }
        if now() > give_up_at {
            return Some(Err(anyhow!("the login took too long, try again")));
        }

        match poll(server, verifier).await {
            Ok(PollResponse::Pending) => failures = 0,
            Ok(PollResponse::Done { token, user }) => {
                return Some(SessionStore::save(&token).map(|()| user));
            }
            Err(error) => {
                failures += 1;
                log::warn!("login poll {failures} of {MAX_FAILURES} failed: {error}");
                if failures >= MAX_FAILURES {
                    return Some(Err(error));
                }
            }
        }
    }
}

async fn poll(server: &str, verifier: &str) -> Result<PollResponse> {
    let body = to_string(&PollRequest { verifier })?;
    let request = CLIENT
        .post(format!("{server}/auth/poll"))
        .header("content-type", "application/json")
        .body(body);

    match send(request).await? {
        Reply::Ok(response) => Ok(response),
        Reply::Unauthorized => bail!("the server refused the login poll"),
    }
}

async fn fetch_current_user() -> Result<Option<LoginUser>> {
    let Some(token) = SessionStore::load() else {
        return Ok(None);
    };

    let request = CLIENT.get(format!("{}/auth/me", server()?)).bearer_auth(token);

    match send(request).await? {
        Reply::Ok(user) => Ok(Some(user)),
        Reply::Unauthorized => {
            SessionStore::clear()?;
            Ok(None)
        }
    }
}

async fn end_session() -> Result<()> {
    let Some(token) = SessionStore::load() else {
        return Ok(());
    };
    SessionStore::clear()?;

    let request = CLIENT.post(format!("{}/auth/logout", server()?)).bearer_auth(token);

    // An unknown session is as logged out as it gets.
    match send::<()>(request).await? {
        Reply::Ok(()) | Reply::Unauthorized => Ok(()),
    }
}

enum Reply<Out> {
    Ok(Out),
    Unauthorized,
}

async fn send<Out: DeserializeOwned>(request: RequestBuilder) -> Result<Reply<Out>> {
    let response = request.send().await?;
    let status = response.status();
    let body = response.text().await?;

    if status == StatusCode::UNAUTHORIZED {
        return Ok(Reply::Unauthorized);
    }
    if !status.is_success() {
        bail!("[{status}] {body}");
    }

    // An empty body parses as JSON null, so a route that answers with nothing
    // works with a `()` output.
    let json = if body.trim().is_empty() {
        "null"
    } else {
        body.as_str()
    };
    Ok(Reply::Ok(from_str(json)?))
}

#[cfg(test)]
mod test {
    use anyhow::Result;

    use super::{challenge, new_verifier};

    #[test]
    fn verifier_is_fresh_every_time() -> Result<()> {
        let first = new_verifier()?;
        assert_eq!(first.len(), 64);
        assert_ne!(first, new_verifier()?);
        Ok(())
    }

    #[test]
    fn challenge_is_the_sha256_of_the_verifier() {
        // The known SHA-256 of "abc". The server checks a poll the same way.
        assert_eq!(
            challenge("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
