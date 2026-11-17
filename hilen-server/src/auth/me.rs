//! `GET /auth/me`, the user of a session.
//!
//! A page in a browser cannot read its own session cookie, so it asks here
//! at every start, also when it has no session. A browser logs every 401 as
//! an error in its console, and the visit of a person who is not signed in
//! must log none. So a request of such a page with no live session gets a
//! 200 with `null`, which holds no data of any user, and a stale cookie is
//! dropped with it.
//!
//! Every other request with no live session keeps the 401: a bearer token
//! the server forgot, which tells an app to forget it too, a request without
//! the header of the cookie, which a page of another site could make, and a
//! view of another user.

use axum::{
    Json,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
};

use crate::{
    AppError,
    auth::{
        AuthState, cookie,
        user::bearer_token,
        view_as::{Seen, is_view},
        wire::Me,
    },
};

pub(super) async fn me(
    State(state): State<AuthState>,
    headers: HeaderMap,
    seen: Result<Seen, AppError>,
) -> Result<Response, AppError> {
    match seen {
        Ok(seen) => {
            if let Some(viewer) = seen.viewer {
                tracing::info!("user {viewer} starts a view as user {}", seen.user.id);
            }
            let me: Me = Some(seen.user.into());
            Ok(Json(me).into_response())
        }
        Err(AppError::Unauthorized) if a_page_with_no_session(&headers) => {
            tracing::debug!("a page with no live session asked who is signed in");
            let nobody: Me = None;
            cookie::dropped(&state.config, &headers, Json(nobody))
        }
        Err(error) => Err(error),
    }
}

/// A request of a page that keeps its session in the cookie, with no bearer
/// token and no view. It gets here only when it has no live session.
fn a_page_with_no_session(headers: &HeaderMap) -> bool {
    cookie::marked(headers) && bearer_token(headers).is_none() && !is_view(headers)
}
