use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use hilen_server::{metrics::counter, metrics_mount};
use http_body_util::BodyExt;
use metrics_exporter_prometheus::PrometheusBuilder;
use tower::ServiceExt;

const TOKEN: &str = "secret";

async fn fetch(app: Router, authorization: Option<&str>) -> (StatusCode, String) {
    let mut request = Request::builder().uri("/metrics");
    if let Some(value) = authorization {
        request = request.header(header::AUTHORIZATION, value);
    }
    let resp = app.oneshot(request.body(Body::empty()).unwrap()).await.unwrap();
    let status = resp.status();
    let body = resp.into_body().collect().await.unwrap().to_bytes().to_vec();
    (status, String::from_utf8(body).unwrap())
}

// A local recorder, the global one can be installed only once per process.
fn app_with_counter() -> Router {
    let recorder = PrometheusBuilder::new().build_recorder();
    let handle = recorder.handle();
    metrics::with_local_recorder(&recorder, || counter!("test_scans_total").increment(3));
    metrics_mount(handle, TOKEN.to_string())
}

#[tokio::test]
async fn token_reads_the_metrics() {
    let (status, body) = fetch(app_with_counter(), Some("Bearer secret")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("test_scans_total 3"), "{body}");
}

#[tokio::test]
async fn missing_token_is_refused() {
    let (status, body) = fetch(app_with_counter(), None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(body.is_empty());
}

#[tokio::test]
async fn wrong_token_is_refused() {
    let (status, _) = fetch(app_with_counter(), Some("Bearer nope")).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
}
