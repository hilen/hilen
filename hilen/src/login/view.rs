//! The app as another user sees it, for a user the backend lets view, like
//! the admin. The server half is `auth/view_as.rs` of `hilen-server`.
//!
//! The own session stays as it is. While a view is on, every `Login::send`
//! carries the header of the view and the server answers as that user, and
//! `Login::active_user` is that user, so the app draws itself for them with
//! the code it has. A view of a visitor sends no session at all. The view is
//! kept in memory only, a new start of the app is the own user again.
//!
//! The server refuses every write of a view, and here no call changes a
//! login while one is on: a login, a logout, a switch and a delete of the
//! account all give an error.

use anyhow::{Result, bail};
use parking_lot::Mutex;
use serde::de::DeserializeOwned;

use crate::{
    deps::{
        hreads::{on_main, spawn},
        netrun::rest::{Call, RequestError},
    },
    login::{
        Login,
        client::{CHANGED, credential, server},
        wire::{LoginUser, VIEW_HEADER},
    },
};

/// Whom the app is shown as, see [`Login::view_as`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum View {
    /// Another user of the backend.
    User(LoginUser),
    /// A visitor with no login.
    SignedOut,
}

impl View {
    /// The user of the view, `None` for a visitor.
    pub fn user(self) -> Option<LoginUser> {
        match self {
            Self::User(user) => Some(user),
            Self::SignedOut => None,
        }
    }
}

static VIEW: Mutex<Option<View>> = Mutex::new(None);

pub(super) fn current() -> Option<View> {
    VIEW.lock().clone()
}

pub(super) fn set(view: Option<View>) {
    *VIEW.lock() = view;
}

/// An error for a call that would change a login while a view is on.
pub(super) fn refuse_change(what: &str) -> Result<()> {
    if current().is_some() {
        bail!("{what} is not possible while the app is shown as another user");
    }
    Ok(())
}

/// `call` the way the app is shown: with the own session, with the header of
/// the view on top, or `None` when nothing may go out, for a visitor and
/// when nobody is logged in.
pub(super) fn dress(call: Call) -> Option<Call> {
    match current() {
        Some(View::SignedOut) => None,
        Some(View::User(user)) => Some(credential()?.on(call).header(VIEW_HEADER, user.id)),
        None => Some(credential()?.on(call)),
    }
}

/// Asks the server who the user `id` is, as the own user, and turns the
/// view on when it answers. The server answers only a user who may view.
pub(super) async fn start(id: &str) -> Result<LoginUser> {
    let Some(credential) = credential() else {
        bail!("nobody is logged in");
    };
    let call = credential
        .on(Call::get(format!("{}/auth/me", server()?)))
        .header(VIEW_HEADER, id);
    let user: LoginUser = call.send().await?;

    log::info!("the app is now shown as user {}", user.id);
    set(Some(View::User(user.clone())));
    Ok(user)
}

fn changed() {
    on_main(|| CHANGED.trigger(()));
}

impl Login {
    /// Shows the app as the user `id` sees it. `done` gets that user, and
    /// [`Login::changed`] is triggered, so every view that listens to it
    /// draws itself again. On an error nothing changes. Only a user the
    /// backend lets view gets an answer, see `ViewAs` of `hilen-server`.
    pub fn view_as(id: impl ToString, done: impl FnOnce(Result<LoginUser>) + Send + 'static) {
        let id = id.to_string();
        spawn(async move {
            let result = start(&id).await;
            on_main(move || {
                if result.is_ok() {
                    CHANGED.trigger(());
                }
                done(result);
            });
        });
    }

    /// Shows the app as a visitor with no login sees it. No request carries
    /// the session until [`Login::stop_view`]. [`Login::changed`] is
    /// triggered.
    pub fn view_signed_out() {
        log::info!("the app is now shown as a visitor with no login");
        set(Some(View::SignedOut));
        changed();
    }

    /// Ends a view, the app is the own user again. [`Login::changed`] is
    /// triggered when a view was on.
    pub fn stop_view() {
        if current().is_none() {
            return;
        }
        log::info!("the view is over, the app is the own user again");
        set(None);
        changed();
    }

    /// The view that is on, `None` for the own user.
    pub fn view() -> Option<View> {
        current()
    }

    /// [`Login::send`] as the own user, also while a view is on. For the few
    /// requests of the viewer, like the list of users to pick from.
    pub async fn send_own<Out: DeserializeOwned>(call: Call) -> Result<Out, RequestError> {
        match credential() {
            Some(credential) => credential.on(call).send().await,
            None => Err(RequestError::Unauthorized),
        }
    }
}
