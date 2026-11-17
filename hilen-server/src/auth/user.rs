use axum::{
    extract::{FromRef, FromRequestParts, OptionalFromRequestParts},
    http::{HeaderMap, header::AUTHORIZATION, request::Parts},
};
use sqlx::types::Uuid;

use crate::{
    AppError, Db,
    auth::{
        cookie, session,
        view_as::{Seen, seen},
    },
};

/// The logged in user. Put it in the arguments of a route and the route only
/// runs for a request with a live session, everybody else gets a 401. The
/// session comes from the bearer header, or from the cookie of a page in a
/// browser, see `cookie.rs`. For a view of another user it is that user, see
/// `view_as.rs`.
///
/// A route that also answers a visitor takes an `Option<User>`. It is `None`
/// for a request with no session or with one that ended.
///
/// ```ignore
/// async fn my_mods(user: User, State(db): State<PgPool>) -> Result<Json<Mods>, AppError> {
///     load_mods(&db, user.id).await
/// }
/// ```
///
/// The state of the router has to give out the [`Db`]. A state that is a
/// `PgPool` or a `SqlitePool` does, and so does one with `#[derive(FromRef)]`
/// and a `Db` field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct User {
    pub id:      Uuid,
    pub email:   String,
    pub name:    String,
    pub picture: Option<String>,
}

impl<S> FromRequestParts<S> for User
where
    Db: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let seen = Seen::from_request_parts(parts, state).await?;
        Ok(seen.user)
    }
}

impl<S> OptionalFromRequestParts<S> for User
where
    Db: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Option<Self>, Self::Rejection> {
        let db = Db::from_ref(state);
        let Some(own) = own_user(parts, &db).await? else {
            return Ok(None);
        };
        Ok(Some(seen(parts, &db, own).await?.user))
    }
}

impl<S> FromRequestParts<S> for Seen
where
    Db: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let db = Db::from_ref(state);
        let own = own_user(parts, &db).await?.ok_or(AppError::Unauthorized)?;
        seen(parts, &db, own).await
    }
}

/// The user of the session of a request, whatever it asks to be seen as.
async fn own_user(parts: &Parts, db: &Db) -> Result<Option<User>, AppError> {
    let Some(token) = bearer_token(&parts.headers).or_else(|| cookie::token(&parts.headers)) else {
        return Ok(None);
    };
    Ok(session::user_of(db, token).await?)
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
