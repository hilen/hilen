//! The standard way a hilen app serves its browser build.
//!
//! The trunk dist is embedded into the server binary with rust-embed and
//! served with SPA fallback, an unknown path returns `index.html` so page
//! reloads and deep links work. An unknown path under `/api/` or `/auth/`
//! is a route a client got wrong and answers the json 404 of [`AppError`]. The
//! `.wasm` file must go out as `application/wasm`, `mime_guess` resolves that
//! from the path.
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

use crate::AppError;

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
/// A file of the dist with a `<name>.br` next to it, brotli made at build
/// time, goes out as that copy to a browser that takes brotli.
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
        Router::new().fallback(move |req: Request| {
            let (client, target) = (client.clone(), target.clone());
            async move {
                if is_route_path(req.uri().path()) {
                    return missing_route(req.uri().path());
                }
                proxy(client, target, req).await
            }
        })
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

/// A file named by trunk with the hash of its content, like
/// `app-43e87976ad5e47ad.js` or `app-43e87976ad5e47ad_bg.wasm`. A new build
/// gives it a new name, so a browser may keep it for good.
fn is_hashed(path: &str) -> bool {
    let name = path.rsplit('/').next().unwrap_or(path);
    let stem = name.split('.').next().unwrap_or(name);
    let stem = stem.strip_suffix("_bg").unwrap_or(stem);
    stem.rsplit_once('-').is_some_and(|(_, hash)| {
        hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Where trunk copies the asset files of an app, see `assets.rs` of hilen.
const ASSETS_FOLDER: &str = "assets/";

/// An asset of the engine comes as `assets/<kind>/<name>?h=<hash>`, the hash
/// of `assets.json`. Only a hash of these very bytes may keep it for good,
/// an old address must never pin new bytes.
fn names_its_hash(query: Option<&str>, data: &[u8]) -> bool {
    query
        .into_iter()
        .flat_map(|q| q.split('&'))
        .filter_map(|pair| pair.strip_prefix("h="))
        .any(|hash| hash == fnv1a(data))
}

/// The hash `build.rs` of an app writes into `assets.json`, see the demo.
fn fnv1a(data: &[u8]) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// A path under `/api/` or `/auth/` names a route of the server, never a page.
fn is_route_path(path: &str) -> bool {
    ["/api", "/auth"].iter().any(|root| {
        path.strip_prefix(root)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

fn missing_route(path: &str) -> Response {
    debug!("no route for {path:?}");
    AppError::NotFound.into_response()
}

/// A path whose last part has an extension asks for a file, never for a page.
fn names_a_file(path: &str) -> bool {
    path.rsplit('/').next().is_some_and(|name| name.contains('.'))
}

/// Whether the request takes a brotli body, `br` in `Accept-Encoding` with no
/// `q=0`.
fn accepts_brotli(req: &Request) -> bool {
    req.headers()
        .get_all(header::ACCEPT_ENCODING)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .any(|item| {
            let mut parts = item.split(';').map(str::trim);
            parts.next() == Some("br") && !parts.any(|p| p.replace(' ', "") == "q=0")
        })
}

fn etag(file: &rust_embed::EmbeddedFile) -> String {
    let hash = file.metadata.sha256_hash();
    format!("\"{}\"", hex::encode(&hash[..16]))
}

async fn embedded<A: RustEmbed>(req: Request) -> Response {
    if is_route_path(req.uri().path()) {
        return missing_route(req.uri().path());
    }
    let path = req.uri().path().trim_start_matches('/');
    // Pair the asset with the path used to look it up, so mime_guess gets
    // the right extension. An SPA route like /nodes falls back to
    // index.html and must be served as text/html, not as a guess from
    // the ".nodes" ending.
    let (asset, mime_path) = if path.is_empty() {
        (A::get("index.html"), "index.html")
    } else if let Some(a) = A::get(path) {
        (Some(a), path)
    } else if names_a_file(path) {
        debug!("no {path} in the embedded dist");
        return (StatusCode::NOT_FOUND, "no such file").into_response();
    } else {
        (A::get("index.html"), "index.html")
    };
    let Some(a) = asset else {
        return (StatusCode::NOT_FOUND, "no index.html in the embedded dist").into_response();
    };
    // Only an asset of the engine comes with its hash. The hash of
    // `index.html` is public, and a link that named it made a browser keep
    // the page for a year, past every later deploy.
    let asset_by_hash = mime_path.starts_with(ASSETS_FOLDER) && names_its_hash(req.uri().query(), &a.data);
    let for_good = is_hashed(mime_path) || asset_by_hash;
    let cache = if for_good {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let brotli = A::get(&format!("{mime_path}.br"));
    let has_brotli = brotli.is_some();
    let (a, encoding) = match brotli {
        Some(br) if accepts_brotli(&req) => (br, Some("br")),
        _ => (a, None),
    };
    let tag = etag(&a);
    let fresh = req
        .headers()
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.split(',').any(|t| t.trim() == tag || t.trim() == "*"));
    let mut builder = Response::builder()
        .header(header::CACHE_CONTROL, cache)
        .header(header::ETAG, &tag);
    if has_brotli {
        builder = builder.header(header::VARY, "Accept-Encoding");
    }
    if let Some(encoding) = encoding {
        builder = builder.header(header::CONTENT_ENCODING, encoding);
    }
    if fresh {
        return builder
            .status(StatusCode::NOT_MODIFIED)
            .body(Body::empty())
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }
    let mime = mime_guess::from_path(mime_path).first_or_octet_stream();
    let mime =
        HeaderValue::from_str(mime.as_ref()).unwrap_or(HeaderValue::from_static("application/octet-stream"));
    builder
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(Body::from(a.data.into_owned()))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
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
    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode, header},
    };
    use rust_embed::RustEmbed;
    use tower::ServiceExt;

    use super::{
        INSPECT_MARKER_PARTS, find_inspect, fnv1a, is_hashed, is_route_path, names_a_file, web_mount_with,
    };

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

    #[test]
    fn trunk_names_are_hashed() {
        assert!(is_hashed("blackforge-web-43e87976ad5e47ad.js"));
        assert!(is_hashed("blackforge-web-43e87976ad5e47ad_bg.wasm"));
        assert!(is_hashed("favicon-e674b70bffdf90b1.png"));
        assert!(!is_hashed("index.html"));
        assert!(!is_hashed("assets/assets.json"));
        assert!(!is_hashed("my-photo.png"));
        assert!(!is_hashed("app-43E87976AD5E47AD.js"));
    }

    #[test]
    fn routes_live_under_api_and_auth() {
        assert!(is_route_path("/api"));
        assert!(is_route_path("/api/metrics"));
        assert!(is_route_path("/auth/nope"));
        assert!(!is_route_path("/apiary"));
        assert!(!is_route_path("/authors/ann"));
        assert!(!is_route_path("/thread/12"));
        assert!(!is_route_path("/"));
    }

    async fn answer(path: &str) -> (StatusCode, Option<String>, String) {
        let response = web_mount_with::<Clean, ()>(None)
            .oneshot(Request::get(path).body(Body::empty()).expect("request"))
            .await
            .expect("answer");
        let status = response.status();
        let kind = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|kind| kind.to_str().ok())
            .map(ToOwned::to_owned);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body");
        (status, kind, String::from_utf8_lossy(&body).into_owned())
    }

    /// A client that calls a route the server lacks must see a failure, not
    /// the page with a 200.
    #[tokio::test]
    async fn an_unknown_route_is_a_json_404_and_a_page_path_the_page() {
        let (status, kind, body) = answer("/api/metrics").await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(kind.as_deref(), Some("application/json"));
        assert_eq!(body, r#"{"error":"not found"}"#);

        let (status, ..) = answer("/auth/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND);

        let (status, kind, _) = answer("/thread/12").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(kind.as_deref(), Some("text/html"));
    }

    async fn cache_of(path: &str) -> String {
        let response = web_mount_with::<Clean, ()>(None)
            .oneshot(Request::get(path).body(Body::empty()).expect("request"))
            .await
            .expect("answer");
        response
            .headers()
            .get(header::CACHE_CONTROL)
            .and_then(|cache| cache.to_str().ok())
            .unwrap_or_default()
            .to_owned()
    }

    /// A link with the hash of `index.html` in its query made a browser keep
    /// the page for a year.
    #[tokio::test]
    async fn only_an_asset_is_kept_for_good_by_its_hash() {
        let index = Clean::get("index.html").expect("the test dist has a page");
        let hash = fnv1a(&index.data);
        for path in [
            format!("/?h={hash}"),
            format!("/settings?h={hash}"),
            format!("/index.html?h={hash}"),
        ] {
            assert_eq!(cache_of(&path).await, "no-cache", "{path}");
        }

        let asset = Clean::get("assets/assets.json").expect("the test dist has an asset");
        let path = format!("/assets/assets.json?h={}", fnv1a(&asset.data));
        assert_eq!(cache_of(&path).await, "public, max-age=31536000, immutable");
        assert_eq!(cache_of("/assets/assets.json?h=0").await, "no-cache");
    }

    #[test]
    fn a_file_has_an_extension() {
        assert!(names_a_file("assets/assets.json"));
        assert!(!names_a_file("thread/12"));
        assert!(!names_a_file("v1.2/bugs"));
    }
}
