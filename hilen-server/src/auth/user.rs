use axum::{
    extract::{FromRef, FromRequestParts},
    http::{HeaderMap, header::AUTHORIZATION, request::Parts},
};
use sqlx::{PgPool, types::Uuid};

use crate::{AppError, auth::session};

/// The logged in user. Put it in the arguments of a route and the route only
/// runs for a request with a live session, everybody else gets a 401.
///
/// ```ignore
/// async fn my_mods(user: User, State(db): State<PgPool>) -> Result<Json<Mods>, AppError> {
///     load_mods(&db, user.id).await
/// }
/// ```
///
/// The state of the router has to give out the `PgPool`, which a state that
/// is the pool does, and so does one with `#[derive(FromRef)]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id:      Uuid,
    pub email:   String,
    pub name:    String,
    pub picture: Option<String>,
}

impl<S> FromRequestParts<S> for User
where
    PgPool: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let token = bearer_token(&parts.headers).ok_or(AppError::Unauthorized)?;
        let db = PgPool::from_ref(state);

        session::user_of(&db, token).await?.ok_or(AppError::Unauthorized)
    }
}

/// The token of an `Authorization: Bearer <token>` header.
pub(crate) fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let (scheme, token) = value.split_once(' ')?;

    (scheme.eq_ignore_ascii_case("bearer") && !token.trim().is_empty()).then(|| token.trim())
}

#[cfg(test)]
mod test {
    use axum::http::{HeaderMap, HeaderValue, header::AUTHORIZATION};

    use super::bearer_token;

    fn headers(value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static(value));
        headers
    }

    #[test]
    fn reads_a_bearer_token() {
        assert_eq!(bearer_token(&headers("Bearer abc")), Some("abc"));
        assert_eq!(bearer_token(&headers("bearer abc")), Some("abc"));
    }

    #[test]
    fn refuses_everything_else() {
        assert_eq!(bearer_token(&HeaderMap::new()), None);
        assert_eq!(bearer_token(&headers("Basic abc")), None);
        assert_eq!(bearer_token(&headers("Bearer ")), None);
        assert_eq!(bearer_token(&headers("abc")), None);
    }
}
