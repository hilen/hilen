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
        .map(|v| v.to_str().unwrap().to_string())
        .unwrap_or_default();
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
