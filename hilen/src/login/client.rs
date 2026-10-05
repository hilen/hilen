use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::{Result, anyhow, bail};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};

use crate::{
    deps::{
        hreads::{now, on_main, sleep, spawn},
        netrun::rest::{Call, RequestError},
    },
    login::wire::{CodeRequest, CodeResponse, LoginUser, PollRequest, PollResponse},
    store::SessionStore,
    system::open_url,
};

const POLL_SECONDS: f32 = 2.0;
/// The server drops a login nobody finished after the same time.
const GIVE_UP_SECONDS: f64 = 600.0;
/// A laptop that just woke up or a server restart fails a few polls in a row.
const MAX_FAILURES: u32 = 5;

static SERVER: Mutex<Option<String>> = Mutex::new(None);

/// Every `start` and every `cancel` moves it on. A poll loop that sees another
/// number than its own knows it is no longer wanted.
static ATTEMPT: AtomicU64 = AtomicU64::new(0);

/// Who the user logs in with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    Google,
    /// The backend needs its Apple config, without it the page answers 404.
    Apple,
}

impl Provider {
    /// The route of the backend that starts this login.
    const fn route(self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::Apple => "apple",
        }
    }
}

/// What a device with no keyboard shows, so its user finishes the login on a
/// phone, see `Login::start_with_code`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoginCode {
    /// 6 letters and digits, with none that look alike.
    pub code: String,
    /// The page where the code is typed, `<server>/auth/code`.
    pub page: String,
    /// The same page with the code already in it. A `QrCodeView` of this
    /// link takes a phone straight to the login.
    pub link: String,
}

/// Login against the `auth` routes of a `hilen-server` backend, with Google
/// or with Apple.
///
/// The app never talks to the provider. It opens a page of its own backend in
/// the browser and asks the backend every two seconds if the login is done.
/// The page address carries only the hash of a secret made here, the secret
/// itself goes out with the polls, so the browser history is of no use to
/// anyone.
///
/// Every callback runs on the main thread.
pub struct Login;

impl Login {
    /// The origin of the app backend, like `https://myapp.example.com`. Call
    /// it once at launch, before anything else here.
    pub fn set_server(origin: impl ToString) {
        *SERVER.lock() = Some(origin.to_string().trim_end_matches('/').to_owned());
    }

    /// Opens the login page and waits for the user to finish there. Call it
    /// straight from a tap, a browser blocks a page opened any later. A
    /// cancelled login never calls `done`.
    pub fn start(provider: Provider, done: impl FnOnce(Result<LoginUser>) + Send + 'static) {
        let attempt = ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;

        let opened = server().and_then(|server| {
            let verifier = new_verifier()?;
            log::info!("{} login started", provider.route());
            open_url(format!(
                "{server}/auth/{}?challenge={}",
                provider.route(),
                challenge(&verifier)
            ))?;
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

    /// The login of a device with no keyboard and no browser of its own, a
    /// TV. Nothing opens here. The server gives a short code, `shown` gets
    /// it to put on the screen, and the user finishes the login on a phone,
    /// through the QR code of `LoginCode::link` or by typing the code into
    /// `LoginCode::page`. Then `done` gets the user, as after `start`.
    /// `done` is not called when no code came, `shown` has the error then.
    pub fn start_with_code(
        provider: Provider,
        shown: impl FnOnce(Result<LoginCode>) + Send + 'static,
        done: impl FnOnce(Result<LoginUser>) + Send + 'static,
    ) {
        let attempt = ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;

        spawn(async move {
            let started = async {
                let server = server()?;
                let verifier = new_verifier()?;
                let code = ask_code(&server, &challenge(&verifier), provider).await?;
                log::info!("{} login with a code started", provider.route());
                Ok::<_, anyhow::Error>((server, verifier, code))
            }
            .await;

            let (server, verifier, code) = match started {
                Ok(started) => started,
                Err(error) => {
                    log::warn!("no login code: {error:#}");
                    return on_main(move || shown(Err(error)));
                }
            };

            on_main(move || shown(Ok(code)));

            if let Some(result) = wait_for_login(&server, &verifier, attempt).await {
                on_main(move || done(result));
            }
        });
    }

    /// Stops waiting for a login started with `start` or `start_with_code`.
    pub fn cancel() {
        ATTEMPT.fetch_add(1, Ordering::SeqCst);
    }

    /// The session token, for the `Authorization: Bearer` header of the app's
    /// own requests. `None` when nobody is logged in.
    pub fn token() -> Option<String> {
        SessionStore::load()
    }

    /// Sends a request of the app with the session token as its
    /// `Authorization: Bearer` header, to any route behind the login.
    /// When nobody is logged in nothing is sent and the answer is
    /// `RequestError::Unauthorized`, the same as a session the server no
    /// longer knows, so 1 match arm sends the user back to the login.
    /// It runs on the calling task, call it inside `spawn`.
    pub async fn send<Out: DeserializeOwned>(call: Call) -> Result<Out, RequestError> {
        match Self::token() {
            Some(token) => call.bearer(token).send().await,
            None => Err(RequestError::Unauthorized),
        }
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

    /// Removes the account on the server for good, with everything the
    /// server keeps for it, and forgets the session here. The session stays
    /// when the server cannot be reached, so the user can try again.
    pub fn delete_account(done: impl FnOnce(Result<()>) + Send + 'static) {
        spawn(async move {
            let result = delete_user().await;
            on_main(move || done(result));
        });
    }
}

#[cfg(feature = "ui-tests")]
impl Login {
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
        .ok_or_else(|| anyhow!("no login server, call Login::set_server at launch"))
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
                log::info!("login done");
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

async fn ask_code(server: &str, challenge: &str, provider: Provider) -> Result<LoginCode> {
    let call = Call::post(format!("{server}/auth/code")).body(CodeRequest {
        challenge,
        provider: provider.route(),
    });

    match call.send::<CodeResponse>().await {
        Ok(response) => Ok(login_code(server, response.code)),
        Err(RequestError::Unauthorized) => bail!("the server refused to give a login code"),
        Err(error) => Err(error.into()),
    }
}

fn login_code(server: &str, code: String) -> LoginCode {
    let page = format!("{server}/auth/code");
    LoginCode {
        link: format!("{page}?code={code}"),
        page,
        code,
    }
}

async fn poll(server: &str, verifier: &str) -> Result<PollResponse> {
    let call = Call::post(format!("{server}/auth/poll")).body(PollRequest { verifier });

    match call.send().await {
        Ok(response) => Ok(response),
        Err(RequestError::Unauthorized) => bail!("the server refused the login poll"),
        Err(error) => Err(error.into()),
    }
}

async fn fetch_current_user() -> Result<Option<LoginUser>> {
    if SessionStore::load().is_none() {
        return Ok(None);
    }

    match Login::send(Call::get(format!("{}/auth/me", server()?))).await {
        Ok(user) => Ok(Some(user)),
        Err(RequestError::Unauthorized) => {
            SessionStore::clear()?;
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}

async fn end_session() -> Result<()> {
    let Some(token) = SessionStore::load() else {
        return Ok(());
    };
    SessionStore::clear()?;

    let call = Call::post(format!("{}/auth/logout", server()?)).bearer(token);

    // An unknown session is as logged out as it gets.
    match call.send::<()>().await {
        Ok(()) | Err(RequestError::Unauthorized) => Ok(()),
        Err(error) => Err(error.into()),
    }
}

async fn delete_user() -> Result<()> {
    if SessionStore::load().is_none() {
        bail!("nobody is logged in");
    }

    // A session the server no longer knows has no account to delete.
    match Login::send::<()>(Call::post(format!("{}/auth/delete", server()?))).await {
        Ok(()) | Err(RequestError::Unauthorized) => {
            log::info!("the account is deleted");
            SessionStore::clear()
        }
        Err(error) => Err(error.into()),
    }
}

#[cfg(test)]
mod test {
    #[cfg(not_wasm)]
    use std::env::temp_dir;

    use anyhow::Result;
    #[cfg(not_wasm)]
    use serial_test::serial;

    use super::{challenge, login_code, new_verifier};
    #[cfg(not_wasm)]
    use crate::{
        deps::netrun::{
            rest::{Call, RequestError},
            test_server::start_test_server,
        },
        login::Login,
        store::SessionStore,
    };

    #[test]
    fn a_login_code_has_its_page_and_its_link() {
        let code = login_code("https://app.example.com", "ABC234".to_owned());
        assert_eq!(code.page, "https://app.example.com/auth/code");
        assert_eq!(code.link, "https://app.example.com/auth/code?code=ABC234");
    }

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

    /// The session file is shared state, the same lock as the other tests
    /// that move the storage root.
    #[cfg(not_wasm)]
    #[tokio::test]
    #[serial(on_disk_root)]
    async fn send_adds_the_session_token_and_sends_nothing_without_one() -> Result<()> {
        let base_url = start_test_server().await;
        let url = format!("{base_url}/header/authorization");
        SessionStore::set_root(temp_dir().join("hilen-login-send-test"));

        SessionStore::save("token-7")?;
        let seen: Option<String> = Login::send(Call::get(&url)).await?;
        assert_eq!(seen.as_deref(), Some("Bearer token-7"));

        // A request that went out would get a 200 with no header in it.
        SessionStore::clear()?;
        let logged_out = Login::send::<Option<String>>(Call::get(&url)).await;
        assert!(matches!(logged_out, Err(RequestError::Unauthorized)));

        Ok(())
    }
}
