use std::{net::SocketAddr, path::Path};

use anyhow::Result;
use axum::{Json, Router, routing::get};
use serde_json::json;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower_http::{services::ServeDir, trace::TraceLayer};

pub async fn build_db(database_url: &str) -> Result<PgPool> {
    let pool = PgPoolOptions::new().max_connections(8).connect(database_url).await?;
    Ok(pool)
}

pub fn build_redis(redis_url: &str) -> Result<redis::Client> {
    let client = redis::Client::open(redis_url)?;
    Ok(client)
}

/// Standard routes every hilen-server based app exposes:
///
/// - `GET /api/hello` returns `{"app": "<name>"}`
/// - `GET /api/health` returns `"ok"`
///
/// Returns a stateless `Router` that the app extends with its own
/// routes and state. Download serving is a separate concern, use
/// `download_mount()` to expose one or more download trees.
pub fn base_routes<S: Clone + Send + Sync + 'static>(app_name: &'static str) -> Router<S> {
    Router::new()
        .route(
            "/api/hello",
            get(move || async move { Json(json!({ "app": app_name })) }),
        )
        .route("/api/health", get(|| async { "ok" }))
        .layer(TraceLayer::new_for_http())
}

/// Mount a directory of release binaries at a URL prefix. Composable so
/// a single server can expose multiple per-app download trees, e.g.:
///
/// ```ignore
/// base_routes("studio")
///     .merge(download_mount("/foo/download", "/app/releases/foo"))
///     .merge(download_mount("/bar/download", "/app/releases/bar"))
/// ```
///
/// For a single-app server, the conventional call is
/// `download_mount("/download", "/app/releases")`.
pub fn download_mount<S, P>(prefix: &str, dir: P) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
    P: AsRef<Path>, {
    Router::new().nest_service(prefix, ServeDir::new(dir.as_ref()))
}

pub async fn serve(router: Router, port: u16) -> Result<()> {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
