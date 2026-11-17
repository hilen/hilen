use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::get,
};
use hilen_server::web_mount_with;
use http_body_util::BodyExt;
use rust_embed::RustEmbed;
use tower::ServiceExt;

#[derive(RustEmbed)]
#[folder = "tests/dist/"]
struct Dist;

fn app() -> Router {
    web_mount_with::<Dist, ()>(None)
}

async fn fetch(path: &str) -> (StatusCode, String, Vec<u8>) {
    let resp = app()
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = resp.status();
    let mime = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .map_or_default(|v| v.to_str().unwrap().to_string());
    let body = resp.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, mime, body)
}

#[tokio::test]
async fn root_serves_index() {
    let (status, mime, body) = fetch("/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mime, "text/html");
    assert!(String::from_utf8(body).unwrap().contains("test index"));
}

#[tokio::test]
async fn exact_asset_with_mime() {
    let (status, mime, body) = fetch("/app.js").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mime, "text/javascript");
    assert!(String::from_utf8(body).unwrap().contains("console.log"));
}

#[tokio::test]
async fn wasm_gets_wasm_mime() {
    // Browsers refuse to stream-compile a module served with any other type.
    let (status, mime, body) = fetch("/app_bg.wasm").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mime, "application/wasm");
    assert_eq!(&body[..4], b"\0asm");
}

#[tokio::test]
async fn nested_asset_is_found() {
    let (status, mime, _) = fetch("/assets/assets.json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mime, "application/json");
}

#[tokio::test]
async fn spa_route_falls_back_to_index_as_html() {
    // The fallback must not let mime_guess see the route as a ".nodes" file.
    let (status, mime, body) = fetch("/nodes").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(mime, "text/html");
    assert!(String::from_utf8(body).unwrap().contains("test index"));
}

#[tokio::test]
async fn dev_proxy_passes_through() {
    let upstream = Router::new().route(
        "/anything",
        get(|| async { ([(header::CONTENT_TYPE, "text/plain")], "from trunk serve") }),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, upstream).await.unwrap() });

    let proxied = web_mount_with::<Dist, ()>(Some(format!("http://{addr}")));
    let resp = proxied
        .oneshot(Request::builder().uri("/anything").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&body[..], b"from trunk serve");
}

async fn headers(path: &str, if_none_match: Option<&str>) -> (StatusCode, header::HeaderMap) {
    let mut req = Request::builder().uri(path);
    if let Some(tag) = if_none_match {
        req = req.header(header::IF_NONE_MATCH, tag);
    }
    let resp = app().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    (resp.status(), resp.headers().clone())
}

fn cache_of(map: &header::HeaderMap) -> &str {
    map.get(header::CACHE_CONTROL).unwrap().to_str().unwrap()
}

#[tokio::test]
async fn hashed_file_is_kept_for_good() {
    // Without it a browser downloads the whole wasm again on every visit.
    let (status, map) = headers("/page-0123456789abcdef.js", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cache_of(&map), "public, max-age=31536000, immutable");
}

#[tokio::test]
async fn index_and_plain_files_are_checked_each_time() {
    for path in ["/", "/nodes", "/app.js", "/assets/assets.json"] {
        let (status, map) = headers(path, None).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(cache_of(&map), "no-cache", "{path}");
        assert!(map.get(header::ETAG).is_some(), "{path}");
    }
}

#[tokio::test]
async fn known_etag_answers_not_modified() {
    let (_, map) = headers("/app.js", None).await;
    let tag = map.get(header::ETAG).unwrap().to_str().unwrap().to_string();
    let (status, _) = headers("/app.js", Some(&tag)).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    let (status, _) = headers("/app.js", Some("\"other\"")).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn missing_file_is_not_found() {
    // A missing manifest answered with index.html reads as broken json.
    let (status, mime, _) = fetch("/assets/missing.json").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_ne!(mime, "text/html");
}

async fn with_encoding(path: &str, accept: Option<&str>) -> (header::HeaderMap, Vec<u8>) {
    let mut req = Request::builder().uri(path);
    if let Some(accept) = accept {
        req = req.header(header::ACCEPT_ENCODING, accept);
    }
    let resp = app().oneshot(req.body(Body::empty()).unwrap()).await.unwrap();
    let map = resp.headers().clone();
    (map, resp.into_body().collect().await.unwrap().to_bytes().to_vec())
}

#[tokio::test]
async fn brotli_copy_goes_to_a_browser_that_takes_it() {
    let (map, body) = with_encoding("/app.js", Some("gzip, deflate, br, zstd")).await;
    assert_eq!(map.get(header::CONTENT_ENCODING).unwrap(), "br");
    assert_eq!(map.get(header::CONTENT_TYPE).unwrap(), "text/javascript");
    assert_eq!(map.get(header::VARY).unwrap(), "Accept-Encoding");
    assert_eq!(body, include_bytes!("dist/app.js.br"));
}

#[tokio::test]
async fn plain_file_goes_to_a_browser_without_brotli() {
    for accept in [None, Some("gzip"), Some("br;q=0, gzip")] {
        let (map, body) = with_encoding("/app.js", accept).await;
        assert!(map.get(header::CONTENT_ENCODING).is_none(), "{accept:?}");
        // A cache in between must keep the 2 answers apart.
        assert_eq!(map.get(header::VARY).unwrap(), "Accept-Encoding", "{accept:?}");
        assert_eq!(body, include_bytes!("dist/app.js"), "{accept:?}");
    }
}

#[tokio::test]
async fn brotli_and_plain_have_their_own_etag() {
    let (plain, _) = with_encoding("/app.js", None).await;
    let (br, _) = with_encoding("/app.js", Some("br")).await;
    assert_ne!(plain.get(header::ETAG), br.get(header::ETAG));
}

#[tokio::test]
async fn asset_with_its_own_hash_is_kept_for_good() {
    let data = include_bytes!("dist/assets/assets.json");
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    let (_, map) = headers(&format!("/assets/assets.json?h={hash:016x}"), None).await;
    assert_eq!(cache_of(&map), "public, max-age=31536000, immutable");
    // An old address must not pin the new bytes.
    let (_, map) = headers("/assets/assets.json?h=0000000000000000", None).await;
    assert_eq!(cache_of(&map), "no-cache");
}
