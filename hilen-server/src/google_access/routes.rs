use axum::{
    Json, Router,
    extract::{Query, State},
    response::{Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;
use serde_json::to_vec;
use sha2::{Digest, Sha256};

use crate::{
    AppError,
    auth::{
        routes::{is_challenge, page},
        session::new_token,
    },
    google_access::{
        GoogleAccessState, google, seal, store,
        wire::{PollRequest, PollResponse, RefreshRequest, RefreshResponse, RevokeRequest, Role, StartQuery},
    },
};

/// How long the user has to get through the pages of Google, and the app to
/// pick the tokens up after that.
const PENDING_MINUTES: i32 = 10;
/// A platform or a version longer than this is not one.
const LABEL_LEN: usize = 32;

/// The Google access routes, with their state already in, so the result
/// merges into an app router of any state:
///
/// - `GET /google/start?challenge=&key=&role=` sends the browser on to Google.
///   `role` is `main` or `linked`, `platform` and `version` are optional and
///   kept for metrics.
/// - `GET /google/callback` is where Google sends it back.
/// - `POST /google/poll` hands the waiting app its sealed tokens, once.
/// - `POST /google/refresh` renews an access token, 401 when Google no longer
///   takes the refresh token and the user has to sign in again.
/// - `POST /google/revoke` ends a token at Google.
pub fn google_access_routes<S>(state: GoogleAccessState) -> Router<S> {
    Router::new()
        .route("/google/start", get(start))
        .route("/google/callback", get(callback))
        .route("/google/poll", post(poll))
        .route("/google/refresh", post(refresh))
        .route("/google/revoke", post(revoke))
        .with_state(state)
}

fn label(text: &str) -> String {
    text.chars().filter(char::is_ascii_graphic).take(LABEL_LEN).collect()
}

async fn start(
    State(state): State<GoogleAccessState>,
    Query(mut query): Query<StartQuery>,
) -> Result<Redirect, AppError> {
    if !is_challenge(&query.challenge) {
        return Err(AppError::BadRequest(
            "the challenge is not a SHA-256 in hex".to_owned(),
        ));
    }
    seal::device_key(&query.key).map_err(|error| AppError::BadRequest(error.to_string()))?;
    query.platform = label(&query.platform);
    query.version = label(&query.version);

    store::prune(&state.db, PENDING_MINUTES).await?;

    let oauth_state = new_token()?;
    if !store::start(&state.db, &query, &oauth_state).await? {
        return Err(AppError::BadRequest(
            "this sign in is already finished".to_owned(),
        ));
    }

    Ok(Redirect::to(&google::auth_url(
        &state.config,
        query.role,
        &oauth_state,
    )?))
}

#[derive(Deserialize)]
struct GoogleAnswer {
    code:  Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// A person looks at the answer of this one, so every outcome is a page.
async fn callback(State(state): State<GoogleAccessState>, Query(answer): Query<GoogleAnswer>) -> Response {
    let app = &state.config.app_name;

    let (Some(code), Some(oauth_state)) = (answer.code, answer.state) else {
        tracing::info!("google sign in stopped: {:?}", answer.error);
        return page(app, "Sign in was cancelled", "You can close this tab.");
    };

    match finish(&state, &oauth_state, &code).await {
        Ok(true) => page(
            app,
            &format!("You are signed in to {app}"),
            "You can close this tab and go back to the app.",
        ),
        Ok(false) => page(
            app,
            "This sign in link is too old",
            "Go back to the app and start again.",
        ),
        Err(error) => {
            tracing::error!("google sign in failed: {error:?}");
            page(app, "Sign in did not work", "Go back to the app and try again.")
        }
    }
}

/// Trades the code for tokens and seals them for the waiting device. False
/// when no sign in waits for this `state`, and then Google is not asked.
async fn finish(state: &GoogleAccessState, oauth_state: &str, code: &str) -> anyhow::Result<bool> {
    let Some(waiting) = store::waiting(&state.db, oauth_state, PENDING_MINUTES).await? else {
        return Ok(false);
    };

    let granted = google::exchange(&state.http, &state.config, waiting.role, code).await?;

    if waiting.role == Role::Main {
        store::main_signed_in(
            &state.db,
            &granted.subject,
            &granted.email,
            granted.name.as_deref().unwrap_or(&granted.email),
            &waiting.platform,
            &waiting.app_version,
        )
        .await?;
    }

    let sealed = seal::seal(
        &seal::device_key(&waiting.device_key)?,
        &waiting.challenge,
        &to_vec(&granted)?,
    )?;
    store::finish(&state.db, oauth_state, &sealed).await?;
    tracing::info!(
        "google sign in done, role {}, platform {}",
        waiting.role.name(),
        waiting.platform
    );

    Ok(true)
}

async fn poll(
    State(state): State<GoogleAccessState>,
    Json(request): Json<PollRequest>,
) -> Result<Json<PollResponse>, AppError> {
    let challenge = hex::encode(Sha256::digest(request.verifier.as_bytes()));

    Ok(Json(
        match store::take(&state.db, &challenge, PENDING_MINUTES).await? {
            Some(sealed) => PollResponse::Done { sealed },
            None => PollResponse::Pending,
        },
    ))
}

/// The tokens of this request are never written down or logged.
async fn refresh(
    State(state): State<GoogleAccessState>,
    Json(request): Json<RefreshRequest>,
) -> Result<Json<RefreshResponse>, AppError> {
    let Some(renewed) = google::renew(&state.http, &state.config, &request.refresh_token).await? else {
        return Err(AppError::Unauthorized);
    };

    if let Some(subject) = &renewed.subject {
        store::main_seen(
            &state.db,
            subject,
            &label(&request.platform),
            &label(&request.version),
            request.linked_accounts,
        )
        .await?;
    }

    Ok(Json(RefreshResponse {
        access_token: renewed.access_token,
        expires_in:   renewed.expires_in,
    }))
}

/// The token of this request is never written down or logged.
async fn revoke(
    State(state): State<GoogleAccessState>,
    Json(request): Json<RevokeRequest>,
) -> Result<(), AppError> {
    google::revoke(&state.http, &state.config, &request.token).await?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::label;

    #[test]
    fn a_label_is_short_and_plain() {
        assert_eq!(label("macos"), "macos");
        assert_eq!(label("a b\nc"), "abc");
        assert_eq!(label(&"x".repeat(100)).len(), 32);
    }
}
