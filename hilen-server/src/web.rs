//! The standard way a hilen app serves its browser build.
//!
//! The trunk dist is embedded into the server binary with rust-embed and
//! served with SPA fallback, an unknown path returns `index.html` so page
//! reloads and deep links work. The `.wasm` file must go out as
//! `application/wasm`, `mime_guess` resolves that from the path.
//!
//! For the dev loop, set [`WEB_DEV_PROXY_ENV`] to a running `trunk serve`
//! url and page requests proxy there instead of the embedded dist, so
//! frontend changes need no server rebuild.

use std::sync::Once;

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use rust_embed::RustEmbed;
use rustls::crypto::ring::default_provider;
use tracing::debug;

/// The inspect marker of `hilen/src/inspect/mod.rs` in two pieces. They are
/// joined at run time, so this binary never holds the whole string and a scan
/// of a backend binary stays clean.
const INSPECT_MARKER_PARTS: [&str; 2] = ["hilen-inspect-", "server-compiled-in"];

/// When set, page requests are proxied to this url instead of the
/// embedded dist. Point it at a running `trunk serve`.
pub const WEB_DEV_PROXY_ENV: &str = "HILEN_WEB_DEV_PROXY";

static TLS_PROVIDER: Once = Once::new();

/// Reqwest is built with `rustls-no-provider`, so the process default has
/// to be in place before a client is built or the build panics. A second
/// install is fine, the first one stays.
pub(crate) fn install_tls_provider() {
    TLS_PROVIDER.call_once(|| {
        if default_provider().install_default().is_err() {
            debug!("rustls default crypto provider was already installed");
        }
    });
}

/// Serve the embedded trunk dist as the app's web page.
///
/// Returns a router with only a fallback, so merge it into the app router
/// after the api routes, every path no api route matched is treated as a
/// page or asset request:
///
/// ```ignore
/// #[derive(RustEmbed)]
/// #[folder = "../web/dist/"]
/// struct Web;
///
/// let app = api_routes().merge(web_mount::<Web, _>());
/// ```
///
/// Reads [`WEB_DEV_PROXY_ENV`] once at router build time.
///
/// # Panics
///
/// In a release build, when the embedded dist carries the hilen inspect
/// server. A deploy then fails at start instead of serving it.
pub fn web_mount<A, S>() -> Router<S>
where
    A: RustEmbed + 'static,
    S: Clone + Send + Sync + 'static, {
    web_mount_with::<A, S>(std::env::var(WEB_DEV_PROXY_ENV).ok())
}

/// [`web_mount`] with the dev proxy target passed explicitly.
///
/// # Panics
///
/// Same as [`web_mount`].
pub fn web_mount_with<A, S>(dev_proxy: Option<String>) -> Router<S>
where
    A: RustEmbed + 'static,
    S: Clone + Send + Sync + 'static, {
    if let Some(target) = dev_proxy {
        tracing::info!("serving web through dev proxy at {target}");
        install_tls_provider();
        let client = reqwest::Client::new();
        Router::new().fallback(move |req: Request| proxy(client.clone(), target.clone(), req))
    } else {
        if !cfg!(debug_assertions) {
            refuse_inspect::<A>();
        }
        Router::new().fallback(get(embedded::<A>))
    }
}

fn refuse_inspect<A: RustEmbed>() {
    if let Some(file) = find_inspect::<A>() {
        panic!(
            r"The embedded web dist file {file} carries the hilen inspect server.
Rebuild the dist without the `inspect` feature, see docs/inspect.md in the hilen repo."
        );
    }
}

/// The embedded wasm file that carries the inspect server, if any.
fn find_inspect<A: RustEmbed>() -> Option<String> {
    let marker = INSPECT_MARKER_PARTS.concat();

    A::iter()
        .filter(|path| path.ends_with(".wasm"))
        .find(|path| A::get(path).is_some_and(|file| contains(&file.data, marker.as_bytes())))
        .map(Into::into)
}

fn contains(data: &[u8], needle: &[u8]) -> bool {
    data.windows(needle.len()).any(|window| window == needle)
}

async fn embedded<A: RustEmbed>(req: Request) -> Response {
    let path = req.uri().path().trim_start_matches('/');
    // Pair the asset with the path used to look it up, so mime_guess gets
    // the right extension. An SPA route like /nodes falls back to
    // index.html and must be served as text/html, not as a guess from
    // the ".nodes" ending.
    let (asset, mime_path) = if path.is_empty() {
        (A::get("index.html"), "index.html")
    } else if let Some(a) = A::get(path) {
        (Some(a), path)
    } else {
        (A::get("index.html"), "index.html")
    };
    match asset {
        Some(a) => {
            let mime = mime_guess::from_path(mime_path).first_or_octet_stream();
            let mime = HeaderValue::from_str(mime.as_ref())
                .unwrap_or(HeaderValue::from_static("application/octet-stream"));
            Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(a.data.into_owned()))
                .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
        }
        None => (StatusCode::NOT_FOUND, "no index.html in the embedded dist").into_response(),
    }
}

async fn proxy(client: reqwest::Client, target: String, req: Request) -> Response {
    let path_and_query = req.uri().path_and_query().map_or("", |p| p.as_str());
    let url = format!("{target}{path_and_query}");
    let resp = match client.get(&url).send().await {
        Ok(resp) => resp,
        Err(e) => return (StatusCode::BAD_GATEWAY, format!("dev proxy: {e}")).into_response(),
    };
    let status = resp.status();
    let content_type = resp.headers().get(header::CONTENT_TYPE).cloned();
    let body = match resp.bytes().await {
        Ok(bytes) => bytes,
        Err(e) => {
            return (StatusCode::BAD_GATEWAY, format!("dev proxy body: {e}")).into_response();
        }
    };
    let mut builder = Response::builder().status(status);
    if let Some(content_type) = content_type {
        builder = builder.header(header::CONTENT_TYPE, content_type);
    }
    builder
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

#[cfg(test)]
mod test {
    use rust_embed::RustEmbed;

    use super::{INSPECT_MARKER_PARTS, find_inspect};

    #[derive(RustEmbed)]
    #[folder = "tests/inspect-dist/"]
    struct WithInspect;

    #[derive(RustEmbed)]
    #[folder = "tests/dist/"]
    struct Clean;

    #[test]
    fn dist_with_inspect_is_found() {
        assert_eq!(find_inspect::<WithInspect>().as_deref(), Some("app_bg.wasm"));
    }

    #[test]
    fn clean_dist_passes() {
        assert!(find_inspect::<Clean>().is_none());
    }

    #[test]
    fn marker_matches_the_engine() {
        let engine = include_str!("../../hilen/src/inspect/mod.rs");

        assert!(engine.contains(&INSPECT_MARKER_PARTS.concat()));
    }
}
