//! The sender against a small server that stands in for APNs. It speaks
//! plain HTTP/1, the real one only HTTP/2, which reqwest picks by itself
//! over TLS.

use std::sync::Arc;

use anyhow::{Result, anyhow};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    serve,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{Value, json};
use tokio::{net::TcpListener, spawn};

use super::{Apns, ApnsAlert, ApnsConfig, Delivery, TOKEN_SECONDS, delivery, payload};
use crate::apple_key::test_key;

/// What the stand in got: the host it was reached as, the token, the
/// headers and the body.
#[derive(Debug, Clone)]
struct Got {
    host:    String,
    token:   String,
    headers: HeaderMap,
    body:    Value,
}

type Received = Arc<Mutex<Vec<Got>>>;

#[derive(Serialize)]
struct Target {
    list: u32,
}

/// The token says the answer: `gone` is removed, `bad` is no token,
/// `busy` is a server error, anything else takes the push.
async fn device(
    State(received): State<Received>,
    Path((host, token)): Path<(String, String)>,
    headers: HeaderMap,
    body: String,
) -> Response {
    let body = serde_json::from_str(&body).unwrap_or(Value::Null);
    received.lock().push(Got {
        host,
        token: token.clone(),
        headers,
        body,
    });
    match token.as_str() {
        "gone" => (StatusCode::GONE, Json(json!({ "reason": "Unregistered" }))).into_response(),
        "bad" => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "reason": "BadDeviceToken" })),
        )
            .into_response(),
        "busy" => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "reason": "ServiceUnavailable" })),
        )
            .into_response(),
        _ => StatusCode::OK.into_response(),
    }
}

/// A stand in for APNs and a sender pointed at it. The production host and
/// the sandbox host are told apart by the first part of the path.
async fn stand_in() -> Result<(Apns, Received)> {
    let received = Received::default();
    let app = Router::new()
        .route("/{host}/3/device/{token}", post(device))
        .with_state(received.clone());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    spawn(async move { serve(listener, app).await });

    let mut config = ApnsConfig::new("TEAM123456", "KEY1234567", test_key()?, "vladas.pirkti");
    config.host = format!("http://{address}/production");
    config.sandbox_host = format!("http://{address}/sandbox");
    Ok((Apns::new(config)?, received))
}

fn alert() -> ApnsAlert {
    ApnsAlert::new("Weekly", "Dave added milk, bread")
        .thread("list-7")
        .collapse("added-7")
}

#[tokio::test]
async fn a_push_reaches_the_host_of_its_token() -> Result<()> {
    let (apns, received) = stand_in().await?;
    assert_eq!(
        apns.send("a1b2c3", false, &alert(), &Target { list: 7 }).await?,
        Delivery::Sent
    );
    assert_eq!(
        apns.send("d4e5f6", true, &alert(), &Target { list: 7 }).await?,
        Delivery::Sent
    );

    let got = received.lock().clone();
    assert_eq!(got.len(), 2);
    assert_eq!(
        (got[0].host.as_str(), got[0].token.as_str()),
        ("production", "a1b2c3")
    );
    assert_eq!(
        (got[1].host.as_str(), got[1].token.as_str()),
        ("sandbox", "d4e5f6")
    );

    let headers = &got[0].headers;
    assert_eq!(headers["apns-topic"], "vladas.pirkti");
    assert_eq!(headers["apns-push-type"], "alert");
    assert_eq!(headers["apns-priority"], "10");
    assert_eq!(headers["apns-collapse-id"], "added-7");
    let bearer = headers["authorization"].to_str()?;
    let token = bearer.strip_prefix("Bearer ").ok_or_else(|| anyhow!("no bearer: {bearer}"))?;
    let parts: Vec<&str> = token.split('.').collect();
    assert_eq!(parts.len(), 3);
    assert_eq!(
        String::from_utf8(URL_SAFE_NO_PAD.decode(parts[0])?)?,
        r#"{"alg":"ES256","kid":"KEY1234567"}"#
    );
    let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1])?)?;
    assert_eq!(claims["iss"], "TEAM123456");

    assert_eq!(
        got[0].body,
        json!({
            "aps": {
                "alert": { "title": "Weekly", "body": "Dave added milk, bread" },
                "sound": "default",
                "thread-id": "list-7",
            },
            "list": 7,
        })
    );
    // Both pushes went with the same provider token.
    assert_eq!(got[0].headers["authorization"], got[1].headers["authorization"]);
    Ok(())
}

#[tokio::test]
async fn a_removed_token_is_gone() -> Result<()> {
    let (apns, _) = stand_in().await?;
    assert_eq!(
        apns.send("gone", false, &alert(), &Target { list: 7 }).await?,
        Delivery::TokenGone
    );
    assert_eq!(
        apns.send("bad", true, &alert(), &Target { list: 7 }).await?,
        Delivery::TokenGone
    );
    Ok(())
}

#[tokio::test]
async fn a_refused_push_is_an_error() -> Result<()> {
    let (apns, _) = stand_in().await?;
    let err = apns
        .send("busy", false, &alert(), &Target { list: 7 })
        .await
        .err()
        .ok_or_else(|| anyhow!("a refused push passed"))?;
    assert!(err.to_string().contains("ServiceUnavailable"), "{err:#}");
    Ok(())
}

#[test]
fn the_provider_token_is_signed_again_after_its_time() -> Result<()> {
    let apns = Apns::new(ApnsConfig::new("TEAM123456", "KEY1234567", test_key()?, "app"))?;
    // `new` signed one at the real time already.
    *apns.token.lock() = None;
    let first = apns.provider_token(1000)?;
    assert_eq!(apns.provider_token(1000 + TOKEN_SECONDS - 1)?, first);
    assert_ne!(apns.provider_token(1000 + TOKEN_SECONDS)?, first);
    Ok(())
}

#[test]
fn a_key_on_1_line_works() -> Result<()> {
    let pem = test_key()?;
    let body: String = pem.lines().filter(|line| !line.starts_with("-----")).collect();
    Apns::new(ApnsConfig::new("TEAM123456", "KEY1234567", body, "app"))?;
    Ok(())
}

#[test]
fn a_wrong_key_stops_the_start() {
    assert!(Apns::new(ApnsConfig::new("TEAM123456", "KEY1234567", "not a key", "app")).is_err());
}

#[test]
fn a_payload_with_no_thread_has_no_thread_id() -> Result<()> {
    let text = payload(&ApnsAlert::new("a", "b"), &Target { list: 1 })?;
    assert_eq!(
        text,
        r#"{"aps":{"alert":{"title":"a","body":"b"},"sound":"default"},"list":1}"#
    );
    assert!(payload(&ApnsAlert::new("a", "b"), &5).is_err());
    Ok(())
}

#[test]
fn an_answer_with_no_reason_is_an_error() {
    assert!(delivery(StatusCode::BAD_REQUEST, "").is_err());
    assert_eq!(delivery(StatusCode::OK, "").ok(), Some(Delivery::Sent));
}
