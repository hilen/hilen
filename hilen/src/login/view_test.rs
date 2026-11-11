//! The view of another user, `view.rs`, against small stand in servers. The
//! view and the session file are shared state, so every test takes the lock
//! of the tests that move the storage root.

use std::env::temp_dir;

use anyhow::Result;
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
    routing::get,
    serve,
};
use serial_test::serial;
use tokio::{net::TcpListener, spawn};

use super::{end_every_session, end_session, fetch_current_user, forget_session};
use crate::{
    deps::netrun::{
        rest::{Call, RequestError},
        test_server::start_test_server,
    },
    login::{
        Login, LoginUser, View,
        view::{current, set, start},
        wire::VIEW_HEADER,
    },
    store::SessionStore,
};

fn user(id: &str) -> LoginUser {
    LoginUser {
        id:      id.to_owned(),
        email:   format!("{id}@example.com"),
        name:    id.to_uppercase(),
        picture: None,
    }
}

/// Ann is logged in, alone, in a root of its own. The view is off again
/// when the test ends, also on a panic.
struct Ann;

impl Ann {
    fn logged_in(root: &str) -> Result<Self> {
        hilen_session::SecretStore::keep_in_files();
        SessionStore::set_root(temp_dir().join(root));
        set(None);
        for user in Login::accounts() {
            SessionStore::remove(&user.id)?;
        }
        SessionStore::add("token-ann", user("ann"))?;
        Ok(Self)
    }
}

impl Drop for Ann {
    fn drop(&mut self) {
        set(None);
    }
}

#[tokio::test]
#[serial(on_disk_root)]
async fn a_view_sends_its_header_with_the_own_session() -> Result<()> {
    let base_url = start_test_server().await;
    let view_header = format!("{base_url}/header/{VIEW_HEADER}");
    let authorization = format!("{base_url}/header/authorization");
    let ann = Ann::logged_in("hilen-login-view-send-test")?;

    let seen: Option<String> = Login::send(Call::get(&view_header)).await?;
    assert_eq!(seen, None);

    set(Some(View::User(user("bob"))));
    let seen: Option<String> = Login::send(Call::get(&view_header)).await?;
    assert_eq!(seen.as_deref(), Some("bob"));
    let seen: Option<String> = Login::send(Call::get(&authorization)).await?;
    assert_eq!(seen.as_deref(), Some("Bearer token-ann"));
    assert_eq!(Login::active_user(), Some(user("bob")));
    assert_eq!(Login::accounts(), [user("bob")]);

    // The viewer still reaches the server as themselves.
    let seen: Option<String> = Login::send_own(Call::get(&view_header)).await?;
    assert_eq!(seen, None);
    let seen: Option<String> = Login::send_own(Call::get(&authorization)).await?;
    assert_eq!(seen.as_deref(), Some("Bearer token-ann"));

    drop(ann);
    assert_eq!(Login::active_user(), Some(user("ann")));
    Ok(())
}

#[tokio::test]
#[serial(on_disk_root)]
async fn a_view_of_a_visitor_sends_nothing_and_keeps_the_session() -> Result<()> {
    let base_url = start_test_server().await;
    let authorization = format!("{base_url}/header/authorization");
    let ann = Ann::logged_in("hilen-login-view-visitor-test")?;

    set(Some(View::SignedOut));
    // A request that went out would get a 200 with no header in it.
    let sent = Login::send::<Option<String>>(Call::get(&authorization)).await;
    assert!(matches!(sent, Err(RequestError::Unauthorized)));
    let sent = Login::send_bytes(Call::get(&authorization)).await;
    assert!(matches!(sent, Err(RequestError::Unauthorized)));
    assert_eq!(Login::token(), None);
    assert_eq!(Login::active_user(), None);
    assert_eq!(Login::accounts(), []);
    assert_eq!(fetch_current_user().await?, None);

    drop(ann);
    assert_eq!(Login::token().as_deref(), Some("token-ann"));
    assert_eq!(Login::active_user(), Some(user("ann")));
    Ok(())
}

/// Stands in for `/auth/me` of a backend where Ann may view. It knows Bob
/// and nobody else.
async fn view_server() -> Result<()> {
    let app = Router::new().route(
        "/auth/me",
        get(|headers: HeaderMap| async move {
            let header = |name: &str| headers.get(name).and_then(|value| value.to_str().ok());
            if header("authorization") != Some("Bearer token-ann") {
                return Err(StatusCode::UNAUTHORIZED);
            }
            match header(VIEW_HEADER) {
                None => Ok(Json(user("ann"))),
                Some("bob") => Ok(Json(user("bob"))),
                Some(_) => Err(StatusCode::NOT_FOUND),
            }
        }),
    );

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    Login::set_server(format!("http://{}", listener.local_addr()?));
    spawn(async move {
        if let Err(error) = serve(listener, app).await {
            eprintln!("the view server stopped: {error}");
        }
    });
    Ok(())
}

#[tokio::test]
#[serial(on_disk_root)]
async fn a_view_starts_only_when_the_server_names_the_user() -> Result<()> {
    view_server().await?;
    let ann = Ann::logged_in("hilen-login-view-start-test")?;

    assert!(start("eve").await.is_err());
    assert_eq!(current(), None);

    assert_eq!(start("bob").await?, user("bob"));
    assert_eq!(current(), Some(View::User(user("bob"))));

    // A second view that fails leaves the first one on.
    assert!(start("eve").await.is_err());
    assert_eq!(current(), Some(View::User(user("bob"))));

    drop(ann);
    Ok(())
}

#[tokio::test]
#[serial(on_disk_root)]
async fn no_login_changes_while_a_view_is_on() -> Result<()> {
    view_server().await?;
    let ann = Ann::logged_in("hilen-login-view-change-test")?;

    for view in [View::User(user("bob")), View::SignedOut] {
        set(Some(view.clone()));

        assert!(end_session().await.is_err(), "{view:?}");
        assert!(end_every_session().await.is_err(), "{view:?}");
        assert!(forget_session("ann").await.is_err(), "{view:?}");
        assert!(Login::switch("ann").is_err(), "{view:?}");
        // The server is never asked, so a 401 for the view drops nothing.
        assert_eq!(fetch_current_user().await?, view.clone().user(), "{view:?}");

        set(None);
        assert_eq!(Login::token().as_deref(), Some("token-ann"), "{view:?}");
        assert_eq!(fetch_current_user().await?, Some(user("ann")), "{view:?}");
    }

    drop(ann);
    Ok(())
}
