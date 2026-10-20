//! The login of a dev server, only in a build with the `dev-login` feature.
//! A build without it has no such route, so a config line can never turn it
//! on for a server with real users.
//!
//! `POST /auth/dev` with an email gives a session with no password. The
//! first login of an email makes a new dev account, the next ones log back
//! into it. A dev account is never joined with a Google or Apple user of the
//! same email, in either direction, see `store::login_user`. So the route
//! reaches only the accounts it made itself.

use axum::{Json, Router, extract::State, routing::post};
use serde::Deserialize;

use crate::{
    AppError,
    auth::{
        AuthState,
        identity::{Identity, Provider},
        session, store,
        wire::PollResponse,
    },
};

/// The longest an email can be.
const MAX_EMAIL: usize = 254;

#[derive(Debug, Deserialize)]
struct DevRequest {
    email: String,
}

pub(crate) fn add_route(router: Router<AuthState>) -> Router<AuthState> {
    tracing::warn!(
        "this server is built with dev-login, POST /auth/dev logs into dev accounts with no password"
    );
    router.route("/auth/dev", post(dev_login))
}

/// The same answer as a finished poll, so a client reads it the same way.
async fn dev_login(
    State(state): State<AuthState>,
    Json(request): Json<DevRequest>,
) -> Result<Json<PollResponse>, AppError> {
    let email = request.email.trim().to_lowercase();
    if !is_email(&email) {
        return Err(AppError::BadRequest(
            "a dev login needs an email like ana@test".to_owned(),
        ));
    }

    let identity = Identity {
        provider: Provider::Dev,
        subject: email.clone(),
        email,
        name: None,
        picture: None,
        refresh_token: None,
    };
    let user_id = store::login_user(&state.db, &identity).await?;
    let token = session::create(&state.db, user_id).await?;
    let user = session::user_of(&state.db, &token).await?.ok_or(AppError::Unauthorized)?;
    tracing::warn!("dev login for user {user_id}, {}", user.email);

    Ok(Json(PollResponse::Done {
        token,
        user: user.into(),
    }))
}

fn is_email(text: &str) -> bool {
    text.len() <= MAX_EMAIL
        && text
            .split_once('@')
            .is_some_and(|(name, domain)| !name.is_empty() && !domain.is_empty() && !domain.contains('@'))
}
