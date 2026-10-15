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
    system::browser_sheet,
};

const POLL_SECONDS: f32 = 2.0;
/// How often a wait between 2 polls looks for a cancel and for a closed
/// sheet, so both end the wait at once.
const LOOK_SECONDS: f32 = 0.1;
/// The server drops a login nobody finished after the same time.
const GIVE_UP_SECONDS: f64 = 600.0;
/// A laptop that just woke up or a server restart fails a few polls in a row.
const MAX_FAILURES: u32 = 5;

static SERVER: Mutex<Option<String>> = Mutex::new(None);

/// Every `start` and every `cancel` moves it on. A poll loop that sees another
/// number than its own knows it is no longer wanted.
static ATTEMPT: AtomicU64 = AtomicU64::new(0);

/// The attempt whose browser sheet the user closed by hand. Its poll loop
/// asks the server one more time and ends.
static CLOSED: AtomicU64 = AtomicU64::new(0);

/// How the wait for a login ended.
enum Waited {
    Done(Result<LoginUser>),
    /// `Login::cancel`, or a newer attempt took over.
    Cancelled,
    /// The user closed the browser sheet by hand with the login not done.
    Closed,
}

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
    ///
    /// On an iPhone the page opens in a browser sheet over the app, and the
    /// sheet closes by itself when the login is done. A sheet the user closes
    /// by hand before that is a cancel.
    pub fn start(provider: Provider, done: impl FnOnce(Result<LoginUser>) + Send + 'static) {
        Self::start_with_close(provider, done, || {});
    }

    /// `start` for a login button. `closed` runs when the user closed the
    /// browser sheet by hand with the login not done, so the button can end
    /// its waiting look. Only an iPhone has such a sheet.
    pub(crate) fn start_with_close(
        provider: Provider,
        done: impl FnOnce(Result<LoginUser>) + Send + 'static,
        closed: impl FnOnce() + Send + 'static,
    ) {
        let attempt = ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;

        let opened = server().and_then(|server| {
            let verifier = new_verifier()?;
            log::info!("{} login started", provider.route());
            let page = format!(
                "{server}/auth/{}?challenge={}",
                provider.route(),
                challenge(&verifier)
            );
            let sheet = browser_sheet::open(page, move || {
                log::info!("the user closed the login sheet");
                CLOSED.store(attempt, Ordering::SeqCst);
            })?;
            Ok((server, verifier, sheet))
        });

        let (server, verifier, sheet) = match opened {
            Ok(opened) => opened,
            Err(error) => return done(Err(error)),
        };

        spawn(async move {
            let waited = wait_for_login(&server, &verifier, attempt).await;
            // Does nothing when the user closed the sheet, and where a
            // platform has none.
            browser_sheet::close(sheet);

            match waited {
                Waited::Done(result) => on_main(move || done(result)),
                Waited::Closed => on_main(closed),
                Waited::Cancelled => {}
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

            // Nothing opens here, so there is no sheet to close.
            if let Waited::Done(result) = wait_for_login(&server, &verifier, attempt).await {
                on_main(move || done(result));
            }
        });
    }

    /// Stops waiting for a login started with `start` or `start_with_code`.
    pub fn cancel() {
        ATTEMPT.fetch_add(1, Ordering::SeqCst);
    }

    /// The token of the active session, for the `Authorization: Bearer`
    /// header of the app's own requests. `None` when nobody is logged in.
    pub fn token() -> Option<String> {
        SessionStore::load()
    }

    /// Everybody logged in on this device, in the order they logged in. A
    /// new login adds to the list, it does not log the others out.
    pub fn accounts() -> Vec<LoginUser> {
        SessionStore::sessions()
            .into_iter()
            .filter_map(|session| session.user)
            .collect()
    }

    /// The user of the active session as it was stored, with no request.
    /// `None` when nobody is active. `current_user` asks the server.
    pub fn active_user() -> Option<LoginUser> {
        SessionStore::active_user()
    }

    /// Makes the session of this user, an `id` of `accounts`, the active one.
    /// Every request after it goes out as that user.
    pub fn switch(id: &str) -> Result<()> {
        SessionStore::switch(id)?;
        log::info!("the active login is now user {id}");
        Ok(())
    }

    /// Ends the session of this user on the server and forgets it here, the
    /// other sessions stay. Nobody is active after it when it was the active
    /// one. The local half happens even when the server cannot be reached.
    pub fn forget(id: &str, done: impl FnOnce(Result<()>) + Send + 'static) {
        let id = id.to_owned();
        spawn(async move {
            let result = forget_session(&id).await;
            on_main(move || done(result));
        });
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

    /// Who the active session belongs to, for the app launch. `None` when
    /// nobody is active, also when the server no longer knows the session,
    /// which then is forgotten here too. The other sessions stay.
    pub fn current_user(done: impl FnOnce(Result<Option<LoginUser>>) + Send + 'static) {
        spawn(async move {
            let result = fetch_current_user().await;
            on_main(move || done(result));
        });
    }

    /// Ends the active session on the server and forgets it here, the other
    /// sessions stay and none of them becomes active. The local half happens
    /// even when the server cannot be reached.
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

async fn wait_for_login(server: &str, verifier: &str, attempt: u64) -> Waited {
    let give_up_at = now() + GIVE_UP_SECONDS;
    let mut failures = 0;

    loop {
        let closed = wait_for_next_poll(attempt).await;

        if ATTEMPT.load(Ordering::SeqCst) != attempt {
            log::debug!("login attempt {attempt} was cancelled, the wait ends");
            return Waited::Cancelled;
        }
        if now() > give_up_at {
            return Waited::Done(Err(anyhow!("the login took too long, try again")));
        }

        // A user can finish the login and close the sheet before the next
        // poll, so a closed sheet still gets 1 poll.
        match poll(server, verifier).await {
            Ok(PollResponse::Done { token, user }) => {
                log::info!("login done");
                return Waited::Done(SessionStore::add(&token, user.clone()).map(|()| user));
            }
            Ok(PollResponse::Pending) if closed => {
                log::info!("the login sheet was closed before the login was done, the wait ends");
                return Waited::Closed;
            }
            Ok(PollResponse::Pending) => failures = 0,
            Err(error) if closed => {
                log::warn!("the last login poll after the sheet was closed failed: {error}");
                return Waited::Closed;
            }
            Err(error) => {
                failures += 1;
                log::warn!("login poll {failures} of {MAX_FAILURES} failed: {error}");
                if failures >= MAX_FAILURES {
                    return Waited::Done(Err(error));
                }
            }
        }
    }
}

/// Sleeps until the next poll is due. It ends early for a cancel and for a
/// sheet the user closed. The answer says whether the sheet of this attempt
/// was closed.
async fn wait_for_next_poll(attempt: u64) -> bool {
    let poll_at = now() + f64::from(POLL_SECONDS);

    loop {
        if CLOSED.load(Ordering::SeqCst) == attempt {
            return true;
        }
        if ATTEMPT.load(Ordering::SeqCst) != attempt || now() >= poll_at {
            return false;
        }
        sleep(LOOK_SECONDS).await;
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
    let Some(token) = SessionStore::load() else {
        return Ok(None);
    };

    let call = Call::get(format!("{}/auth/me", server()?)).bearer(token.clone());
    match call.send::<LoginUser>().await {
        Ok(user) => {
            // A session of an engine with 1 login gets its user here, and a
            // new name or picture reaches the list of accounts.
            SessionStore::set_user(&token, user.clone())?;
            Ok(Some(user))
        }
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
    logout_on_server(token).await
}

async fn forget_session(id: &str) -> Result<()> {
    let Some(token) = SessionStore::remove(id)? else {
        return Ok(());
    };
    log::info!("the login of user {id} is forgotten on this device");
    logout_on_server(token).await
}

async fn logout_on_server(token: String) -> Result<()> {
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
    use std::{
        env::temp_dir,
        sync::{
            Arc,
            atomic::{AtomicU32, Ordering},
        },
    };

    use anyhow::Result;
    #[cfg(not_wasm)]
    use axum::{
        Json, Router,
        http::{HeaderMap, StatusCode},
        routing::{get, post},
        serve,
    };
    #[cfg(not_wasm)]
    use parking_lot::Mutex;
    #[cfg(not_wasm)]
    use serde_json::{Value, json};
    #[cfg(not_wasm)]
    use serial_test::serial;
    #[cfg(not_wasm)]
    use tokio::{net::TcpListener, spawn};

    #[cfg(not_wasm)]
    use super::{
        ATTEMPT, CLOSED, POLL_SECONDS, Waited, end_session, fetch_current_user, forget_session,
        wait_for_login,
    };
    use super::{challenge, login_code, new_verifier};
    #[cfg(not_wasm)]
    use crate::{
        deps::{
            hreads::now,
            netrun::{
                rest::{Call, RequestError},
                test_server::start_test_server,
            },
        },
        login::{Login, LoginUser},
        store::SessionStore,
    };

    #[cfg(not_wasm)]
    fn user(id: &str) -> LoginUser {
        LoginUser {
            id:      id.to_owned(),
            email:   format!("{id}@example.com"),
            name:    id.to_uppercase(),
            picture: None,
        }
    }

    /// Stands in for `/auth/me` and `/auth/logout` of a backend. It knows the
    /// session `token-ann` only, and keeps the token of every logout.
    #[cfg(not_wasm)]
    async fn account_server() -> Result<Arc<Mutex<Vec<String>>>> {
        let logouts = Arc::new(Mutex::new(Vec::new()));
        let kept = logouts.clone();
        let bearer = |headers: &HeaderMap| {
            headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .trim_start_matches("Bearer ")
                .to_owned()
        };

        let app = Router::new()
            .route(
                "/auth/me",
                get(move |headers: HeaderMap| async move {
                    if bearer(&headers) == "token-ann" {
                        Ok(Json(user("ann")))
                    } else {
                        Err(StatusCode::UNAUTHORIZED)
                    }
                }),
            )
            .route(
                "/auth/logout",
                post(move |headers: HeaderMap| async move {
                    kept.lock().push(bearer(&headers));
                    Json(())
                }),
            );

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        Login::set_server(format!("http://{}", listener.local_addr()?));
        spawn(async move {
            if let Err(error) = serve(listener, app).await {
                eprintln!("the account server stopped: {error}");
            }
        });
        Ok(logouts)
    }

    #[cfg(not_wasm)]
    fn account_ids() -> Vec<String> {
        Login::accounts().into_iter().map(|user| user.id).collect()
    }

    /// A root of its own with no session left from an earlier run.
    #[cfg(not_wasm)]
    fn fresh_root(name: &str) -> Result<()> {
        SessionStore::set_root(temp_dir().join(name));
        for user in Login::accounts() {
            SessionStore::remove(&user.id)?;
        }
        Ok(())
    }

    /// The user changed the name at Google. `current_user` puts the new one
    /// into the stored session, so the list of accounts shows it.
    #[cfg(not_wasm)]
    #[tokio::test]
    #[serial(on_disk_root)]
    async fn current_user_puts_the_new_user_into_its_session() -> Result<()> {
        account_server().await?;
        fresh_root("hilen-login-accounts-me-test")?;
        let old = LoginUser {
            name: "Old name".to_owned(),
            ..user("ann")
        };
        SessionStore::add("token-ann", old)?;

        assert_eq!(fetch_current_user().await?, Some(user("ann")));
        assert_eq!(Login::active_user(), Some(user("ann")));
        assert_eq!(Login::accounts(), [user("ann")]);

        SessionStore::clear()?;
        Ok(())
    }

    /// The server forgot the session of bob. Only bob goes, ann stays and
    /// nobody is active.
    #[cfg(not_wasm)]
    #[tokio::test]
    #[serial(on_disk_root)]
    async fn a_session_the_server_forgot_drops_only_that_account() -> Result<()> {
        account_server().await?;
        fresh_root("hilen-login-accounts-401-test")?;
        SessionStore::add("token-ann", user("ann"))?;
        SessionStore::add("token-bob", user("bob"))?;

        assert_eq!(fetch_current_user().await?, None);
        assert_eq!(account_ids(), ["ann"]);
        assert_eq!(Login::token(), None);

        Login::switch("ann")?;
        assert_eq!(fetch_current_user().await?, Some(user("ann")));

        SessionStore::clear()?;
        Ok(())
    }

    /// `forget` and `logout` each end 1 session on the server, with the
    /// token of that session, and keep the others here.
    #[cfg(not_wasm)]
    #[tokio::test]
    #[serial(on_disk_root)]
    async fn forget_and_logout_end_1_session_each() -> Result<()> {
        let logouts = account_server().await?;
        fresh_root("hilen-login-accounts-forget-test")?;
        SessionStore::add("token-ann", user("ann"))?;
        SessionStore::add("token-bob", user("bob"))?;
        SessionStore::add("token-cid", user("cid"))?;

        forget_session("ann").await?;
        assert_eq!(account_ids(), ["bob", "cid"]);
        assert_eq!(Login::token().as_deref(), Some("token-cid"));

        end_session().await?;
        assert_eq!(account_ids(), ["bob"]);
        assert_eq!(Login::token(), None);
        assert_eq!(*logouts.lock(), ["token-ann", "token-cid"]);

        forget_session("bob").await?;
        assert_eq!(Login::accounts(), []);
        Ok(())
    }

    /// Stands in for `/auth/poll` of a backend. It gives every poll the same
    /// answer and counts the polls.
    #[cfg(not_wasm)]
    async fn poll_server(answer: Value) -> Result<(String, Arc<AtomicU32>)> {
        let polls = Arc::new(AtomicU32::new(0));
        let counted = polls.clone();

        let app = Router::new().route(
            "/auth/poll",
            post(move || async move {
                counted.fetch_add(1, Ordering::SeqCst);
                Json(answer)
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let origin = format!("http://{}", listener.local_addr()?);
        spawn(async move {
            if let Err(error) = serve(listener, app).await {
                eprintln!("the poll server stopped: {error}");
            }
        });
        Ok((origin, polls))
    }

    /// A new attempt whose browser sheet the user already closed.
    #[cfg(not_wasm)]
    fn closed_attempt() -> u64 {
        let attempt = ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;
        CLOSED.store(attempt, Ordering::SeqCst);
        attempt
    }

    /// The 3 tests of the wait move the attempt number, so they share the
    /// lock of the session file, which the second one writes.
    #[cfg(not_wasm)]
    #[tokio::test]
    #[serial(on_disk_root)]
    async fn a_closed_sheet_ends_the_wait_after_1_more_poll() -> Result<()> {
        let (server, polls) = poll_server(json!({ "status": "pending" })).await?;
        let attempt = closed_attempt();
        let started = now();

        let waited = wait_for_login(&server, "verifier", attempt).await;

        assert!(matches!(waited, Waited::Closed));
        assert_eq!(polls.load(Ordering::SeqCst), 1);
        // It did not sleep until the next poll was due.
        assert!(now() - started < f64::from(POLL_SECONDS));
        Ok(())
    }

    /// The user finished the login and closed the sheet before the next
    /// poll. That is a login, not a cancel.
    #[cfg(not_wasm)]
    #[tokio::test]
    #[serial(on_disk_root)]
    async fn a_login_done_before_the_sheet_was_closed_still_logs_in() -> Result<()> {
        let user = json!({ "id": "1", "email": "a@b.c", "name": "A", "picture": null });
        let (server, polls) =
            poll_server(json!({ "status": "done", "token": "token-9", "user": user })).await?;
        SessionStore::set_root(temp_dir().join("hilen-login-closed-sheet-test"));
        let attempt = closed_attempt();

        let Waited::Done(result) = wait_for_login(&server, "verifier", attempt).await else {
            panic!("a finished login did not end the wait as done");
        };

        assert_eq!(result?.email, "a@b.c");
        assert_eq!(SessionStore::load().as_deref(), Some("token-9"));
        assert_eq!(polls.load(Ordering::SeqCst), 1);

        SessionStore::clear()?;
        Ok(())
    }

    #[cfg(not_wasm)]
    #[tokio::test]
    #[serial(on_disk_root)]
    async fn a_cancel_ends_the_wait_with_no_poll() -> Result<()> {
        let (server, polls) = poll_server(json!({ "status": "pending" })).await?;
        let attempt = ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;
        Login::cancel();
        let started = now();

        let waited = wait_for_login(&server, "verifier", attempt).await;

        assert!(matches!(waited, Waited::Cancelled));
        assert_eq!(polls.load(Ordering::SeqCst), 0);
        assert!(now() - started < f64::from(POLL_SECONDS));
        Ok(())
    }

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

        SessionStore::add("token-7", user("ann"))?;
        let seen: Option<String> = Login::send(Call::get(&url)).await?;
        assert_eq!(seen.as_deref(), Some("Bearer token-7"));

        // A request that went out would get a 200 with no header in it.
        SessionStore::clear()?;
        let logged_out = Login::send::<Option<String>>(Call::get(&url)).await;
        assert!(matches!(logged_out, Err(RequestError::Unauthorized)));

        Ok(())
    }
}
