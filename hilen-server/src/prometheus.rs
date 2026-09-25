//! Prometheus metrics for an app backend.
//!
//! The app records with the re-exported `metrics` macros, like
//! `metrics::counter!("news_scans_total").increment(1)`, and mounts
//! [`metrics_mount`] to serve them. A stack with a public host may publish
//! only one port, so the metrics share it and a Bearer token keeps them
//! private.

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

/// Install the process wide recorder. Call it once at startup, before the
/// first metric is recorded, a metric recorded earlier is lost. A second
/// call fails.
pub fn install_metrics() -> Result<PrometheusHandle> {
    PrometheusBuilder::new()
        .install_recorder()
        .context("a metrics recorder is already installed")
}

/// `GET /metrics` in the Prometheus text format, only for a request with
/// `Authorization: Bearer <token>`. Anything else gets 401, so a public
/// host does not leak the numbers.
pub fn metrics_mount<S>(handle: PrometheusHandle, token: String) -> Router<S>
where S: Clone + Send + Sync + 'static {
    Router::new().route("/metrics", get(render)).with_state(MetricsState {
        handle,
        expected: format!("Bearer {token}"),
    })
}

#[derive(Clone)]
struct MetricsState {
    handle:   PrometheusHandle,
    expected: String,
}

async fn render(State(state): State<MetricsState>, headers: HeaderMap) -> Response {
    let authorized = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == state.expected);

    if !authorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    (
        [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
        state.handle.render(),
    )
        .into_response()
}
