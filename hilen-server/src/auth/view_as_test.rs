//! The view of another user, `view_as.rs`, on routes with a real database.
//! Ana is the viewer, Bob is a user like any other.

use anyhow::Result;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header::AUTHORIZATION},
    routing::{get, post},
};
use http_body_util::BodyExt;
use sqlx::types::Uuid;
use tower::ServiceExt;

use crate::{
    Db,
    auth::{
        AuthConfig, AuthState, User, ViewAs, Viewing, auth_routes,
        identity::{Identity, Provider},
        session, store,
        store_test::{on_postgres, sqlite},
        user_of_token,
        view_as::HEADER,
    },
};

const ANA: &str = "ana@example.com";
const BOB: &str = "bob@example.com";

async fn whoami(user: User) -> String {
    user.email
}

async fn maybe(user: Option<User>) -> String {
    user.map_or_else(|| "nobody".to_owned(), |user| user.email)
}

async fn viewing(Viewing(viewing): Viewing) -> String {
    viewing.to_string()
}

struct Stage {
    app:       Router,
    db:        Db,
    ana:       Uuid,
    bob:       Uuid,
    ana_token: String,
    bob_token: String,
}

async fn user(db: &Db, subject: &str, email: &str) -> Result<(Uuid, String)> {
    let id = store::login_user(
        db,
        &Identity {
            provider:      Provider::Google,
            subject:       subject.to_owned(),
            email:         email.to_owned(),
            name:          Some(subject.to_owned()),
            picture:       None,
            refresh_token: None,
        },
    )
    .await?;
    Ok((id, session::create(db, id).await?))
}

async fn stage(db: Db) -> Result<Stage> {
    let (ana, ana_token) = user(&db, "ana", ANA).await?;
    let (bob, bob_token) = user(&db, "bob", BOB).await?;

    let view_as = ViewAs::new([ana]);
    let config =
        AuthConfig::new("Test App", "https://app.example.com", "id", "secret").with_view_as(view_as.clone());
    let inside = Router::new()
        .route("/whoami", get(whoami))
        .route("/write", post(whoami))
        .route("/maybe", get(maybe))
        .route("/viewing", get(viewing));
    let outside = Router::new().route("/outside", get(whoami)).route("/outside-maybe", get(maybe));
    let app = auth_routes(AuthState::new(db.clone(), config))
        .merge(view_as.layer(inside))
        .merge(outside)
        .with_state(db.clone());

    Ok(Stage {
        app,
        db,
        ana,
        bob,
        ana_token,
        bob_token,
    })
}

impl Stage {
    /// The status and the text of an answer. `token` is the session, `view`
    /// the value of the view header.
    async fn ask(
        &self,
        method: &str,
        path: &str,
        token: Option<&str>,
        view: Option<&str>,
    ) -> Result<(StatusCode, String)> {
        let mut request = Request::builder().method(method).uri(path);
        if let Some(token) = token {
            request = request.header(AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(view) = view {
            request = request.header(HEADER, view);
        }
        let response = self.app.clone().oneshot(request.body(Body::empty())?).await?;
        let status = response.status();
        let bytes = response.into_body().collect().await?.to_bytes();
        Ok((status, String::from_utf8(bytes.to_vec())?))
    }

    async fn get(&self, path: &str, token: Option<&str>, view: Option<&str>) -> Result<(StatusCode, String)> {
        self.ask("GET", path, token, view).await
    }

    async fn status(&self, path: &str, token: Option<&str>, view: Option<&str>) -> Result<StatusCode> {
        Ok(self.get(path, token, view).await?.0)
    }
}

async fn a_viewer_reads_as_another_user(db: Db) -> Result<()> {
    let stage = stage(db).await?;
    let ana = Some(stage.ana_token.as_str());
    let bob = stage.bob.to_string();

    assert_eq!(
        stage.get("/whoami", ana, None).await?,
        (StatusCode::OK, ANA.to_owned())
    );
    assert_eq!(
        stage.get("/whoami", ana, Some(&bob)).await?,
        (StatusCode::OK, BOB.to_owned())
    );
    assert_eq!(
        stage.get("/maybe", ana, Some(&bob)).await?,
        (StatusCode::OK, BOB.to_owned())
    );
    // The view made no session and changed none: the next request is Ana.
    assert_eq!(
        stage.get("/whoami", ana, None).await?,
        (StatusCode::OK, ANA.to_owned())
    );
    Ok(())
}

#[tokio::test]
async fn sqlite_a_viewer_reads_as_another_user() -> Result<()> {
    a_viewer_reads_as_another_user(sqlite().await?.1).await
}

#[tokio::test]
#[ignore = "needs a Postgres, see the top of store_test.rs"]
async fn postgres_a_viewer_reads_as_another_user() -> Result<()> {
    on_postgres(a_viewer_reads_as_another_user).await
}

#[tokio::test]
async fn only_a_viewer_may_view() -> Result<()> {
    let stage = stage(sqlite().await?.1).await?;
    let bob = Some(stage.bob_token.as_str());
    let ana = stage.ana.to_string();

    // Bob is no viewer, not even of himself.
    assert_eq!(
        stage.status("/whoami", bob, Some(&ana)).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        stage.status("/whoami", bob, Some(&stage.bob.to_string())).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        stage.status("/maybe", bob, Some(&ana)).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        stage.status("/auth/me", bob, Some(&ana)).await?,
        StatusCode::FORBIDDEN
    );
    // The header gives nothing to a request with no session.
    assert_eq!(
        stage.status("/whoami", None, Some(&ana)).await?,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        stage.status("/whoami", Some("wrong"), Some(&ana)).await?,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        stage.get("/maybe", None, Some(&ana)).await?,
        (StatusCode::OK, "nobody".to_owned())
    );
    Ok(())
}

#[tokio::test]
async fn a_route_outside_the_layer_refuses_a_view() -> Result<()> {
    let stage = stage(sqlite().await?.1).await?;
    let ana = Some(stage.ana_token.as_str());
    let bob = stage.bob.to_string();

    assert_eq!(
        stage.status("/outside", ana, Some(&bob)).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        stage.status("/outside-maybe", ana, Some(&bob)).await?,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        stage.get("/outside", ana, None).await?,
        (StatusCode::OK, ANA.to_owned())
    );
    Ok(())
}

#[tokio::test]
async fn a_view_never_writes() -> Result<()> {
    let stage = stage(sqlite().await?.1).await?;
    let ana = Some(stage.ana_token.as_str());
    let bob = stage.bob.to_string();

    assert_eq!(
        stage.ask("POST", "/write", ana, None).await?,
        (StatusCode::OK, ANA.to_owned())
    );
    assert_eq!(
        stage.ask("POST", "/write", ana, Some(&bob)).await?.0,
        StatusCode::FORBIDDEN
    );

    // The routes that end a session or a user: all refused, and both users
    // and their sessions are still there.
    for path in ["/auth/logout", "/auth/logout/all", "/auth/delete"] {
        assert_eq!(
            stage.ask("POST", path, ana, Some(&bob)).await?.0,
            StatusCode::FORBIDDEN,
            "{path}"
        );
    }
    let of =
        async |token: &str| Ok::<_, sqlx::Error>(user_of_token(&stage.db, token).await?.map(|user| user.id));
    assert_eq!(of(&stage.ana_token).await?, Some(stage.ana));
    assert_eq!(of(&stage.bob_token).await?, Some(stage.bob));
    Ok(())
}

#[tokio::test]
async fn a_view_of_nobody_is_refused() -> Result<()> {
    let stage = stage(sqlite().await?.1).await?;
    let ana = Some(stage.ana_token.as_str());

    assert_eq!(
        stage.status("/whoami", ana, Some("bob")).await?,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        stage.status("/whoami", ana, Some("")).await?,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        stage.status("/whoami", ana, Some(&Uuid::nil().to_string())).await?,
        StatusCode::NOT_FOUND
    );
    Ok(())
}

#[tokio::test]
async fn auth_me_names_the_viewed_user() -> Result<()> {
    let stage = stage(sqlite().await?.1).await?;
    let ana = Some(stage.ana_token.as_str());
    let bob = stage.bob.to_string();

    let (status, own) = stage.get("/auth/me", ana, None).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(own.contains(ANA), "{own}");

    let (status, viewed) = stage.get("/auth/me", ana, Some(&bob)).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(viewed.contains(BOB) && !viewed.contains(ANA), "{viewed}");
    assert_eq!(
        stage.status("/auth/providers", ana, Some(&bob)).await?,
        StatusCode::OK
    );
    Ok(())
}

#[tokio::test]
async fn auth_me_refuses_a_view_without_the_config() -> Result<()> {
    let db = sqlite().await?.1;
    let (ana, token) = user(&db, "ana", ANA).await?;
    let (bob, _) = user(&db, "bob", BOB).await?;
    let config = AuthConfig::new("Test App", "https://app.example.com", "id", "secret");
    let app: Router = auth_routes(AuthState::new(db, config));

    let response = app
        .oneshot(
            Request::builder()
                .uri("/auth/me")
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(HEADER, bob.to_string())
                .body(Body::empty())?,
        )
        .await?;
    assert_ne!(ana, bob);
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    Ok(())
}

#[tokio::test]
async fn a_route_learns_that_it_is_viewed() -> Result<()> {
    let stage = stage(sqlite().await?.1).await?;
    let ana = Some(stage.ana_token.as_str());

    assert_eq!(
        stage.get("/viewing", ana, None).await?,
        (StatusCode::OK, "false".to_owned())
    );
    assert_eq!(
        stage.get("/viewing", ana, Some(&stage.bob.to_string())).await?,
        (StatusCode::OK, "true".to_owned())
    );
    Ok(())
}
