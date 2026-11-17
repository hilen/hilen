//! Who the active session belongs to, asked at the start of an app.

use anyhow::Result;

use crate::{
    deps::{
        hreads::{on_main, spawn},
        netrun::rest::{Call, RequestError},
    },
    login::{
        Login,
        client::{Credential, clear_active, credential, server, set_cookie_user, wait_for_resume},
        view,
        wire::{LoginUser, Me},
    },
    store::SessionStore,
};

impl Login {
    /// Who the active session belongs to, for the app launch. `None` when
    /// nobody is active, also when the server no longer knows the session,
    /// which then is forgotten here too. The other sessions stay.
    pub fn current_user(done: impl FnOnce(Result<Option<LoginUser>>) + Send + 'static) {
        spawn(async move {
            let result = fetch_current_user().await;
            on_main(move || done(result));
        });
    }
}

/// A page in cookie mode cannot see its cookie, so it asks also with no
/// session, and the server answers it `null`. An app with a token gets a 401
/// for a session the server forgot. Both mean nobody.
pub(super) async fn fetch_current_user() -> Result<Option<LoginUser>> {
    wait_for_resume().await;
    // A view never asks: a 401 for it must not drop the own session.
    if let Some(view) = view::current() {
        return Ok(view.user());
    }
    let Some(credential) = credential() else {
        return Ok(None);
    };
    let token = match &credential {
        Credential::Bearer(token) => Some(token.clone()),
        Credential::Cookie => None,
    };

    let call = credential.on(Call::get(format!("{}/auth/me", server()?)));
    match call.send::<Me>().await {
        Ok(Some(user)) => {
            match token {
                // A session of an engine with 1 login gets its user here, and
                // a new name or picture reaches the list of accounts.
                Some(token) => SessionStore::set_user(&token, user.clone())?,
                None => set_cookie_user(Some(user.clone())),
            }
            Ok(Some(user))
        }
        Ok(None) | Err(RequestError::Unauthorized) => {
            clear_active()?;
            Ok(None)
        }
        Err(error) => Err(error.into()),
    }
}
