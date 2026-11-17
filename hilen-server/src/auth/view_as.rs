//! The app as another user sees it, for a few users like the admin.
//!
//! A viewer stays signed in as themselves and adds the header [`HEADER`] with
//! the id of another user to a request. The [`User`] extractor then gives the
//! route that other user, so the answer is what that user would get. No
//! session of the other user is ever made, and a request without the header
//! is the viewer again.
//!
//! The header counts only when all of these hold, everything else is a 403
//! and a warning in the log:
//!
//! - the session belongs to a user of [`ViewAs::new`],
//! - the route is inside [`ViewAs::layer`], or it is `/auth/me` or
//!   `/auth/providers` of a config with
//!   [`AuthConfig::with_view_as`](crate::auth::AuthConfig::with_view_as),
//! - the request is a `GET` or a `HEAD`, a view never writes.
//!
//! A route that reads with a `GET` and still saves something, like a mark of
//! what the user has seen, takes a [`Viewing`] and skips that write.
//!
//! ```ignore
//! let view_as = ViewAs::new(admins);
//! let auth = AuthState::new(db.clone(), AuthConfig::from_env("My App")?.with_view_as(view_as.clone()));
//! let app = base_routes("my-app")
//!     .merge(auth_routes(auth))
//!     .merge(view_as.layer(page_routes()))
//!     .merge(other_routes())
//!     .with_state(db);
//! ```

use std::{convert::Infallible, future::ready, sync::Arc};

use axum::{
    Extension, Router,
    extract::FromRequestParts,
    http::{HeaderMap, Method, request::Parts},
};
use sqlx::types::Uuid;
use tracing::{debug, warn};

use crate::{
    AppError, Db,
    auth::{User, store},
};

/// The header of a view, its value is the id of the user to view as.
pub const HEADER: &str = "x-hilen-view-as";

/// Who may see the app as another user, and through [`ViewAs::layer`] on
/// which routes.
#[derive(Clone, Debug)]
pub struct ViewAs {
    viewers: Arc<[Uuid]>,
}

impl ViewAs {
    /// `viewers` are the only users whose view header counts.
    pub fn new(viewers: impl IntoIterator<Item = Uuid>) -> Self {
        Self {
            viewers: viewers.into_iter().collect(),
        }
    }

    /// Turns the view on for every route of `router`. A route outside of it
    /// answers a request with the view header with a 403.
    pub fn layer<S>(&self, router: Router<S>) -> Router<S>
    where S: Clone + Send + Sync + 'static {
        router.layer(Extension(self.clone()))
    }
}

/// Whether the request is a view of another user. For a route that saves
/// something on a read: it skips that write when this is true.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewing(pub bool);

impl<S: Sync> FromRequestParts<S> for Viewing {
    type Rejection = Infallible;

    fn from_request_parts(
        parts: &mut Parts,
        _: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        ready(Ok(Self(is_view(&parts.headers))))
    }
}

pub(crate) fn is_view(headers: &HeaderMap) -> bool {
    headers.contains_key(HEADER)
}

/// The user a route gets for a request of `own`, and who views when it is
/// another one.
pub(crate) struct Seen {
    pub user:   User,
    pub viewer: Option<Uuid>,
}

/// `own` for a request with no view header, the user of the header for a
/// view that is allowed, and a 403 for every other one.
pub(crate) async fn seen(parts: &Parts, db: &Db, own: User) -> Result<Seen, AppError> {
    let Some(value) = parts.headers.get(HEADER) else {
        return Ok(Seen {
            user:   own,
            viewer: None,
        });
    };
    let path = parts.uri.path();

    let allowed = parts
        .extensions
        .get::<ViewAs>()
        .is_some_and(|view| view.viewers.contains(&own.id));
    if !allowed {
        warn!("user {} asked for {path} as another user and was refused", own.id);
        return Err(AppError::Forbidden);
    }
    if parts.method != Method::GET && parts.method != Method::HEAD {
        warn!(
            "user {} sent a {} to {path} as another user, a view never writes",
            own.id, parts.method
        );
        return Err(AppError::Forbidden);
    }

    let id = value
        .to_str()
        .ok()
        .and_then(|text| Uuid::parse_str(text).ok())
        .ok_or_else(|| AppError::BadRequest(format!("{HEADER} is no user id")))?;
    let user = store::user(db, id).await?.ok_or(AppError::NotFound)?;

    debug!("user {} reads {path} as user {}", own.id, user.id);
    Ok(Seen {
        user,
        viewer: Some(own.id),
    })
}
