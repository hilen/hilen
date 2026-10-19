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
    ATTEMPT, CLOSED, POLL_SECONDS, Waited, end_session, fetch_current_user, forget_session, wait_for_login,
};
use super::{challenge, login_code, new_verifier};
#[cfg(not_wasm)]
use crate::{deps::hreads::sleep, login::hand_back::HandBack};
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

/// The sessions of a test go to a sealed file in the temp folder. A
/// workspace build turns the credential store on through `ui-test`, and
/// a test must not write into the Keychain of the machine.
#[cfg(not_wasm)]
fn test_root(name: &str) {
    hilen_session::SecretStore::keep_in_files();
    SessionStore::set_root(temp_dir().join(name));
}

/// A root of its own with no session left from an earlier run.
#[cfg(not_wasm)]
fn fresh_root(name: &str) -> Result<()> {
    test_root(name);
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

    let waited = wait_for_login(&server, "verifier", None, attempt).await;

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
    let (server, polls) = poll_server(json!({ "status": "done", "token": "token-9", "user": user })).await?;
    test_root("hilen-login-closed-sheet-test");
    let attempt = closed_attempt();

    let Waited::Done(result) = wait_for_login(&server, "verifier", None, attempt).await else {
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

    let waited = wait_for_login(&server, "verifier", None, attempt).await;

    assert!(matches!(waited, Waited::Cancelled));
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    assert!(now() - started < f64::from(POLL_SECONDS));
    Ok(())
}

/// What a poll carried.
#[cfg(not_wasm)]
#[derive(serde::Deserialize)]
struct Polled {
    verifier: String,
    code:     Option<String>,
}

/// Stands in for `/auth/poll` of a backend that hands the session out
/// only for the verifier together with `code`. It keeps every poll.
#[cfg(not_wasm)]
async fn code_server(code: String) -> Result<(String, Arc<Mutex<Vec<Option<String>>>>)> {
    let polls = Arc::new(Mutex::new(Vec::new()));
    let kept = polls.clone();

    let app = Router::new().route(
        "/auth/poll",
        post(move |Json(polled): Json<Polled>| async move {
            kept.lock().push(polled.code.clone());
            if polled.verifier == "verifier" && polled.code == Some(code) {
                let user = json!({ "id": "1", "email": "a@b.c", "name": "A", "picture": null });
                Json(json!({ "status": "done", "token": "token-back", "user": user }))
            } else {
                Json(json!({ "status": "pending" }))
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    spawn(async move {
        if let Err(error) = serve(listener, app).await {
            eprintln!("the code server stopped: {error}");
        }
    });
    Ok((origin, polls))
}

/// What the browser of the user does at the end of the login: it comes
/// back to the port of the app with the code.
#[cfg(not_wasm)]
async fn come_back(back: &HandBack, code: &str) -> Result<()> {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpStream,
    };

    let port: u16 = back.query().trim_start_matches("port=").parse()?;
    let mut stream = TcpStream::connect(("127.0.0.1", port)).await?;
    stream
        .write_all(format!("GET /?code={code} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").as_bytes())
        .await?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply).await?;
    assert!(reply.starts_with("HTTP/1.1 303"), "{reply}");
    Ok(())
}

/// The app polls only once a code came back to its port, and the code
/// goes out with the verifier. A code the server does not take, like
/// one some web page sent to the port, does not end the wait.
#[cfg(not_wasm)]
#[tokio::test]
#[serial(on_disk_root)]
async fn the_login_polls_with_the_code_that_came_back() -> Result<()> {
    let right = "c0".repeat(32);
    let (server, polls) = code_server(right.clone()).await?;
    test_root("hilen-login-hand-back-test");
    let back = HandBack::open(&server, "challenge")?;
    let attempt = ATTEMPT.fetch_add(1, Ordering::SeqCst) + 1;

    let wrong = "0f".repeat(32);
    come_back(&back, &wrong).await?;

    let waiting = wait_for_login(&server, "verifier", Some(&back), attempt);
    let sent = async {
        // The wait has polled with the wrong code by then.
        sleep(0.5).await;
        come_back(&back, &right).await
    };
    let (waited, sent) = tokio::join!(waiting, sent);
    sent?;

    let Waited::Done(result) = waited else {
        panic!("the code that came back did not end the wait as done");
    };
    assert_eq!(result?.email, "a@b.c");
    assert_eq!(SessionStore::load().as_deref(), Some("token-back"));
    assert_eq!(*polls.lock(), [Some(wrong), Some(right)]);

    SessionStore::clear()?;
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
    test_root("hilen-login-send-test");

    SessionStore::add("token-7", user("ann"))?;
    let seen: Option<String> = Login::send(Call::get(&url)).await?;
    assert_eq!(seen.as_deref(), Some("Bearer token-7"));

    // A request that went out would get a 200 with no header in it.
    SessionStore::clear()?;
    let logged_out = Login::send::<Option<String>>(Call::get(&url)).await;
    assert!(matches!(logged_out, Err(RequestError::Unauthorized)));

    Ok(())
}
