use std::{future::pending, net::SocketAddr, path::Path, sync::Arc, time::Duration};

use anyhow::Result;
use axum::{Json, Router, routing::get};
use serde_json::json;
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};
use tokio::{signal::ctrl_c, sync::Notify, time::timeout};
use tower_http::{services::ServeDir, trace::TraceLayer};

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
    serve_on(SocketAddr::from(([0, 0, 0, 0], port)), router).await
}

/// [`serve`] with an explicit bind address, for an app that must stay on
/// loopback, for example behind a proxy sidecar sharing its network
/// namespace.
pub async fn serve_on(addr: SocketAddr, router: Router) -> Result<()> {
    let listener = bind(addr).await?;
    serve_listener(listener, router).await
}

/// Bind separately from serving when startup work runs between the two.
/// Binding first keeps the healthcheck endpoint reachable even if that
/// work hangs, requests just wait in the accept queue until
/// [`serve_listener`] runs.
pub async fn bind(addr: SocketAddr) -> Result<tokio::net::TcpListener> {
    tracing::info!("listening on {addr}");
    Ok(tokio::net::TcpListener::bind(addr).await?)
}

pub async fn serve_listener(listener: tokio::net::TcpListener, router: Router) -> Result<()> {
    serve_until(listener, router, stop_signal(), SHUTDOWN_LIMIT).await
}

/// A websocket or a long poll never ends by itself, so a stop must not wait
/// for it. A container that ignores the signal holds a deploy for 10 seconds,
/// then Docker kills it.
const SHUTDOWN_LIMIT: Duration = Duration::from_secs(3);

async fn serve_until(
    listener: tokio::net::TcpListener,
    router: Router,
    stop: impl Future<Output = ()> + Send + 'static,
    limit: Duration,
) -> Result<()> {
    let stopping = Arc::new(Notify::new());
    let stopped = Arc::clone(&stopping);
    let server = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        stop.await;
        stopped.notify_one();
    })
    .into_future();
    tokio::pin!(server);

    tokio::select! {
        result = &mut server => return Ok(result?),
        () = stopping.notified() => {}
    }
    if let Ok(result) = timeout(limit, server).await {
        Ok(result?)
    } else {
        tracing::warn!("open connections did not end in {limit:?}, stopping anyway");
        Ok(())
    }
}

/// Ends at the signal Docker stops a container with, or at Ctrl+C. A signal
/// that cannot be listened to only warns, the server must keep running.
async fn stop_signal() {
    #[cfg(unix)]
    let terminate = async {
        match signal(SignalKind::terminate()) {
            Ok(mut terminate) => {
                terminate.recv().await;
            }
            Err(error) => {
                tracing::warn!("cannot listen for the stop signal: {error}");
                pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = pending::<()>();

    let interrupt = async {
        if let Err(error) = ctrl_c().await {
            tracing::warn!("cannot listen for Ctrl+C: {error}");
            pending::<()>().await;
        }
    };

    tokio::select! {
        () = terminate => {}
        () = interrupt => {}
    }
    tracing::info!("stopping");
}

#[cfg(test)]
mod shutdown_tests {
    use std::{process::Command, time::Instant};

    use axum::routing::get;
    use tokio::{
        io::AsyncWriteExt,
        net::{TcpListener, TcpStream},
        spawn,
        sync::oneshot,
        time::sleep,
    };

    use super::*;

    /// Docker stops a container with this signal. Without a handler the deploy
    /// waits 10 seconds for the kill.
    #[cfg(unix)]
    #[tokio::test]
    async fn the_stop_signal_ends_the_server_at_once() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let server = spawn(serve_listener(listener, Router::new()));
        // A signal before the handler is in place would kill the test run.
        sleep(Duration::from_millis(300)).await;

        let started = Instant::now();
        let status = Command::new("kill")
            .args(["-TERM", &std::process::id().to_string()])
            .status()
            .unwrap();
        assert!(status.success());

        server.await.unwrap().unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[tokio::test]
    async fn a_request_that_never_ends_does_not_hold_the_stop() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let router = Router::new().route("/", get(pending::<()>));
        let (stop, stopped) = oneshot::channel::<()>();
        let limit = Duration::from_millis(200);
        let server = spawn(serve_until(
            listener,
            router,
            async move {
                stopped.await.unwrap();
            },
            limit,
        ));

        let mut client = TcpStream::connect(address).await.unwrap();
        client.write_all(b"GET / HTTP/1.1\r\nHost: test\r\n\r\n").await.unwrap();
        sleep(Duration::from_millis(100)).await;

        let started = Instant::now();
        stop.send(()).unwrap();
        server.await.unwrap().unwrap();
        assert!(started.elapsed() >= limit);
        assert!(started.elapsed() < Duration::from_secs(2));
    }
}
