use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use hilen_server::{AppLinks, app_links_mount};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

const APPLE: &str = "/.well-known/apple-app-site-association";
const ANDROID: &str = "/.well-known/assetlinks.json";
const HASH: &str = "146de983c5730650d8eeb9952f34fc6416a08342e61dbea88a0496b23fcf44e5";
const HASH_COLONS: &str =
    "14:6D:E9:83:C5:73:06:50:D8:EE:B9:95:2F:34:FC:64:16:A0:83:42:E6:1D:BE:A8:8A:04:96:B2:3F:CF:44:E5";

async fn fetch(app: Router, path: &str) -> (StatusCode, Option<String>, Option<Value>) {
    let request = Request::builder().uri(path).body(Body::empty()).unwrap();
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let kind = response
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|value| value.to_str().unwrap().to_string());
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json = serde_json::from_slice(&body).ok();
    (status, kind, json)
}

fn both() -> Router {
    let links = AppLinks::new()
        .apple("ABCDE12345", "vladas.shopping")
        .android("com.example.shopping", [HASH])
        .unwrap();
    app_links_mount(links)
}

#[tokio::test]
async fn ios_reads_the_app_id_of_the_team_and_the_bundle() {
    let (status, kind, json) = fetch(both(), APPLE).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(kind.as_deref(), Some("application/json"));
    assert_eq!(
        json.unwrap(),
        json!({
            "applinks": {
                "apps": [],
                "details": [{
                    "appID": "ABCDE12345.vladas.shopping",
                    "appIDs": ["ABCDE12345.vladas.shopping"],
                    "paths": ["*"],
                    "components": [{ "/": "*" }],
                }],
            },
        })
    );
}

#[tokio::test]
async fn android_reads_the_package_and_the_key_hash() {
    let (status, kind, json) = fetch(both(), ANDROID).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(kind.as_deref(), Some("application/json"));
    assert_eq!(
        json.unwrap(),
        json!([{
            "relation": ["delegate_permission/common.handle_all_urls"],
            "target": {
                "namespace": "android_app",
                "package_name": "com.example.shopping",
                "sha256_cert_fingerprints": [HASH_COLONS],
            },
        }])
    );
}

#[tokio::test]
async fn a_phone_with_no_app_gets_no_file() {
    let apple_only = app_links_mount(AppLinks::new().apple("ABCDE12345", "vladas.shopping"));
    assert_eq!(fetch(apple_only, ANDROID).await.0, StatusCode::NOT_FOUND);

    let android_only = app_links_mount(AppLinks::new().android("com.example.shopping", [HASH]).unwrap());
    assert_eq!(fetch(android_only, APPLE).await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn the_routes_merge_into_the_router_of_a_backend() {
    let app = Router::new()
        .route("/health", axum::routing::get(|| async { "ok" }))
        .merge(both());

    assert_eq!(fetch(app.clone(), "/health").await.0, StatusCode::OK);
    assert_eq!(fetch(app, APPLE).await.0, StatusCode::OK);
}

#[test]
fn an_android_app_needs_a_right_key_hash() {
    assert!(AppLinks::new().android("com.example.shopping", ["14:6D"]).is_err());
    assert!(AppLinks::new().android("com.example.shopping", Vec::<&str>::new()).is_err());
}
