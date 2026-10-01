use axum::{
    Json, Router,
    extract::{Query, State},
    http::HeaderMap,
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::{
    AppError,
    auth::{
        AuthState, User, google, session, store,
        user::bearer_token,
        wire::{PollRequest, PollResponse, UserInfo},
    },
};

/// How long the user has to get through the Google pages.
const PENDING_MINUTES: i32 = 10;

/// The login routes, with their state already in, so the result merges into
/// an app router of any state:
///
/// - `GET /auth/google?challenge=` sends the browser on to Google.
/// - `GET /auth/google/callback` is where Google sends it back.
/// - `POST /auth/poll` hands the waiting app its session, once.
/// - `GET /auth/me` is the user of a session.
/// - `POST /auth/logout` ends a session.
pub fn auth_routes<S>(state: AuthState) -> Router<S> {
    Router::new()
        .route("/auth/google", get(start))
        .route("/auth/google/callback", get(callback))
        .route("/auth/poll", post(poll))
        .route("/auth/me", get(me))
        .route("/auth/logout", post(logout))
        .with_state(state)
}

#[derive(Deserialize)]
struct StartQuery {
    challenge: String,
}

async fn start(
    State(state): State<AuthState>,
    Query(query): Query<StartQuery>,
) -> Result<Redirect, AppError> {
    if !is_challenge(&query.challenge) {
        return Err(AppError::BadRequest(
            "the challenge is not a SHA-256 in hex".to_owned(),
        ));
    }

    store::prune_pending(&state.db, PENDING_MINUTES).await?;

    let oauth_state = session::new_token()?;

    if !store::start_pending(&state.db, &query.challenge, &oauth_state).await? {
        return Err(AppError::BadRequest("this login is already finished".to_owned()));
    }

    Ok(Redirect::to(&google::auth_url(&state.config, &oauth_state)?))
}

#[derive(Deserialize)]
struct CallbackQuery {
    code:  Option<String>,
    state: Option<String>,
    error: Option<String>,
}

/// A person looks at the answer of this one, so every outcome is a page.
async fn callback(State(state): State<AuthState>, Query(query): Query<CallbackQuery>) -> Response {
    let app = &state.config.app_name;

    let (Some(code), Some(oauth_state)) = (query.code, query.state) else {
        tracing::info!("google login stopped: {:?}", query.error);
        return page(app, "Sign in was cancelled", "You can close this tab.");
    };

    match finish(&state, &code, &oauth_state).await {
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
            tracing::error!("google login failed: {error:?}");
            page(app, "Sign in did not work", "Go back to the app and try again.")
        }
    }
}

/// False when no login waits for this `state`, it timed out or never was.
async fn finish(state: &AuthState, code: &str, oauth_state: &str) -> anyhow::Result<bool> {
    if !store::is_waiting(&state.db, oauth_state, PENDING_MINUTES).await? {
        return Ok(false);
    }

    let identity = google::exchange(&state.http, &state.config, code).await?;
    let user_id = store::upsert_user(&state.db, &identity).await?;
    store::finish_pending(&state.db, oauth_state, user_id).await?;

    Ok(true)
}

async fn poll(
    State(state): State<AuthState>,
    Json(request): Json<PollRequest>,
) -> Result<Json<PollResponse>, AppError> {
    let challenge = hex::encode(Sha256::digest(request.verifier.as_bytes()));

    let Some(user_id) = store::take_finished(&state.db, &challenge, PENDING_MINUTES).await? else {
        return Ok(Json(PollResponse::Pending));
    };

    let token = session::create(&state.db, user_id).await?;
    let user = session::user_of(&state.db, &token).await?.ok_or(AppError::Unauthorized)?;

    Ok(Json(PollResponse::Done {
        token,
        user: user.into(),
    }))
}

async fn me(user: User) -> Json<UserInfo> {
    Json(user.into())
}

async fn logout(State(state): State<AuthState>, headers: HeaderMap) -> Result<(), AppError> {
    let token = bearer_token(&headers).ok_or(AppError::Unauthorized)?;
    session::delete(&state.db, token).await?;
    Ok(())
}

fn is_challenge(text: &str) -> bool {
    text.len() == 64 && text.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn page(app: &str, title: &str, text: &str) -> Response {
    let (app, title, text) = (escape(app), escape(title), escape(text));

    Html(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{app}</title>
<style>
body {{ font-family: system-ui, sans-serif; background: #f7f8fa; color: #17191d; display: grid; place-items: center; min-height: 100vh; margin: 0; }}
main {{ text-align: center; padding: 24px; }}
h1 {{ font-size: 22px; font-weight: 600; margin: 0 0 8px; }}
p {{ color: #5f6368; margin: 0; }}
@media (prefers-color-scheme: dark) {{ body {{ background: #131314; color: #e3e3e3; }} p {{ color: #9aa0a6; }} }}
</style>
</head>
<body><main><h1>{title}</h1><p>{text}</p></main></body>
</html>"#
    ))
    .into_response()
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod test {
    use super::{escape, is_challenge};

    #[test]
    fn challenge_shape() {
        assert!(is_challenge(&"a1".repeat(32)));
        assert!(!is_challenge(&"a1".repeat(31)));
        assert!(!is_challenge(&"zz".repeat(32)));
        assert!(!is_challenge(""));
    }

    #[test]
    fn page_text_is_escaped() {
        assert_eq!(
            escape(r#"<b>"A&B"</b>"#),
            "&lt;b&gt;&quot;A&amp;B&quot;&lt;/b&gt;"
        );
    }
}
