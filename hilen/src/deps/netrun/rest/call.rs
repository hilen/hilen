use std::{any::type_name, collections::BTreeMap, fmt::Display};

use log::debug;
use reqwest::StatusCode;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Error as JsonError, from_str, to_string};
use thiserror::Error;

use crate::deps::{
    hreads::now,
    netrun::rest::{Method, client::shared_client},
};

/// Why a request gave no parsed answer.
#[derive(Debug, Error)]
pub enum RequestError {
    /// The server answered 401, the app sends the user back to the login.
    /// `Login::send` also gives it when nobody is logged in.
    #[error("[401 Unauthorized]")]
    Unauthorized,
    /// Any other status that is not a success.
    #[error("[{status}] {url}: {body}")]
    Status {
        status: StatusCode,
        url:    String,
        body:   String,
    },
    /// The request did not reach the server, or its answer did not arrive.
    #[error("failed to send the request: {0}")]
    Send(#[from] reqwest::Error),
    #[error("failed to write the request body: {0}")]
    Body(#[source] JsonError),
    #[error("failed to parse {expected} from {body}: {source}")]
    Parse {
        expected: &'static str,
        body:     String,
        source:   JsonError,
    },
}

impl RequestError {
    /// The status of the answer, `None` when no answer with a failing
    /// status came.
    pub fn status(&self) -> Option<StatusCode> {
        match self {
            Self::Unauthorized => Some(StatusCode::UNAUTHORIZED),
            Self::Status { status, .. } => Some(*status),
            Self::Send(_) | Self::Body(_) | Self::Parse { .. } => None,
        }
    }
}

/// What a request carries.
enum Body {
    Json(Result<String, JsonError>),
    Bytes(Vec<u8>),
}

/// One request to a full URL made at run time, with headers and a body,
/// JSON or plain bytes, on the client of the engine. Every other request
/// of `net::rest` and of the login goes through it.
#[must_use = "a call does nothing until it is sent"]
pub struct Call {
    method:  Method,
    url:     String,
    headers: BTreeMap<String, String>,
    body:    Option<Body>,
}

impl Call {
    pub fn new(method: Method, url: impl ToString) -> Self {
        Self {
            method,
            url: url.to_string(),
            headers: BTreeMap::new(),
            body: None,
        }
    }

    pub fn get(url: impl ToString) -> Self {
        Self::new(Method::Get, url)
    }

    pub fn post(url: impl ToString) -> Self {
        Self::new(Method::Post, url)
    }

    pub fn put(url: impl ToString) -> Self {
        Self::new(Method::Put, url)
    }

    pub fn patch(url: impl ToString) -> Self {
        Self::new(Method::Patch, url)
    }

    pub fn delete(url: impl ToString) -> Self {
        Self::new(Method::Delete, url)
    }

    pub fn header(mut self, key: impl ToString, value: impl ToString) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn headers(mut self, headers: impl IntoIterator<Item = (impl ToString, impl ToString)>) -> Self {
        for (key, value) in headers {
            self.headers.insert(key.to_string(), value.to_string());
        }
        self
    }

    /// Adds `Authorization: Bearer <token>`.
    pub fn bearer(self, token: impl Display) -> Self {
        self.header("authorization", format!("Bearer {token}"))
    }

    /// The JSON body. It goes out as `application/json` unless a header
    /// names another content type. A body that cannot be written fails the
    /// send.
    pub fn body(mut self, body: impl Serialize) -> Self {
        self.body = Some(Body::Json(to_string(&body)));
        self
    }

    /// A body of plain bytes, like a file. They go out as they are, as
    /// `application/octet-stream` unless a header names another content
    /// type.
    pub fn bytes(mut self, bytes: Vec<u8>) -> Self {
        self.body = Some(Body::Bytes(bytes));
        self
    }

    /// Sends the request and parses the answer. An empty answer parses as
    /// `()`.
    pub async fn send<Out: DeserializeOwned>(self) -> Result<Out, RequestError> {
        let Self {
            method,
            url,
            headers,
            body,
        } = self;
        let body = match body {
            Some(Body::Json(json)) => {
                Some(("application/json", json.map_err(RequestError::Body)?.into_bytes()))
            }
            Some(Body::Bytes(bytes)) => Some(("application/octet-stream", bytes)),
            None => None,
        };
        let started = now();

        let client = shared_client();
        let mut request = match method {
            Method::Get => client.get(&url),
            Method::Post => client.post(&url),
            Method::Put => client.put(&url),
            Method::Patch => client.patch(&url),
            Method::Delete => client.delete(&url),
        };

        let has_content_type = headers.keys().any(|key| key.eq_ignore_ascii_case("content-type"));
        for (key, value) in headers {
            request = request.header(key, value);
        }
        if let Some((content_type, body)) = body {
            if !has_content_type {
                request = request.header("content-type", content_type);
            }
            request = request.body(body);
        }

        let answer = async {
            let response = request.send().await?;
            let status = response.status();
            Ok::<_, reqwest::Error>((status, response.text().await?))
        }
        .await;

        // The body is not logged, a login request carries a secret in it.
        let millis = (now() - started) * 1000.0;
        let (status, body) = match answer {
            Ok(answer) => answer,
            Err(error) => {
                debug!("{method} {url} failed in {millis:.0} ms: {error}");
                return Err(error.into());
            }
        };
        debug!("{method} {url}: {status} in {millis:.0} ms");

        if status == StatusCode::UNAUTHORIZED {
            return Err(RequestError::Unauthorized);
        }
        if !status.is_success() {
            return Err(RequestError::Status { status, url, body });
        }

        parse(&body)
    }
}

fn parse<Out: DeserializeOwned>(body: &str) -> Result<Out, RequestError> {
    // A 204 or an empty 200 body parses as JSON null, so endpoints that
    // answer with nothing work with `()` outputs.
    let json = if body.trim().is_empty() { "null" } else { body };

    from_str(json).map_err(|source| RequestError::Parse {
        expected: type_name::<Out>(),
        body: body.to_owned(),
        source,
    })
}

#[cfg(all(test, not_wasm))]
mod tests {
    use anyhow::Result;
    use reqwest::StatusCode;

    use super::{Call, RequestError, parse};
    use crate::deps::netrun::test_server::{Post, PostPatch, start_test_server};

    #[test]
    fn empty_body_parses_as_null() {
        parse::<()>("").unwrap();
        parse::<()>("  \n").unwrap();
        assert_eq!(parse::<Option<u32>>("").unwrap(), None);
        assert!(matches!(parse::<u32>(""), Err(RequestError::Parse { .. })));
    }

    #[tokio::test]
    async fn a_header_reaches_the_server() -> Result<()> {
        let base_url = start_test_server().await;

        let seen: Option<String> = Call::get(format!("{base_url}/header/x-test"))
            .header("x-test", 7)
            .send()
            .await?;

        assert_eq!(seen.as_deref(), Some("7"));

        Ok(())
    }

    #[tokio::test]
    async fn a_bearer_token_reaches_the_server() -> Result<()> {
        let base_url = start_test_server().await;

        let seen: Option<String> = Call::get(format!("{base_url}/header/authorization"))
            .bearer("abc")
            .send()
            .await?;

        assert_eq!(seen.as_deref(), Some("Bearer abc"));

        Ok(())
    }

    /// The path has an id made at run time and the method is a PUT, the 2
    /// things the older calls could not do together with a body.
    #[tokio::test]
    async fn a_body_reaches_the_server() -> Result<()> {
        let base_url = start_test_server().await;
        let id = 5;
        let rename = PostPatch {
            title: "renamed".to_string(),
        };

        let post: Post = Call::put(format!("{base_url}/posts/{id}")).body(&rename).send().await?;

        assert_eq!(post.id, 5);
        assert_eq!(post.title, "renamed");

        Ok(())
    }

    /// Bytes that are no text, they must arrive as they were sent.
    #[tokio::test]
    async fn bytes_reach_the_server_as_they_are() -> Result<()> {
        let base_url = start_test_server().await;
        let bytes: Vec<u8> = (0..=255).chain([0, 255, 0x89, b'\n']).collect();
        let sum = bytes.iter().map(|byte| u64::from(*byte)).sum::<u64>();

        let seen: (usize, u64, Option<String>) =
            Call::post(format!("{base_url}/bytes")).bytes(bytes.clone()).send().await?;

        assert_eq!(
            seen,
            (bytes.len(), sum, Some("application/octet-stream".to_string()))
        );

        let seen: (usize, u64, Option<String>) = Call::post(format!("{base_url}/bytes"))
            .header("content-type", "image/png")
            .bytes(bytes.clone())
            .send()
            .await?;

        assert_eq!(seen.2.as_deref(), Some("image/png"));

        Ok(())
    }

    #[tokio::test]
    async fn a_401_is_the_unauthorized_error() {
        let base_url = start_test_server().await;

        let error = Call::get(format!("{base_url}/status/401"))
            .send::<()>()
            .await
            .expect_err("A 401 must not parse as an answer");

        assert!(matches!(error, RequestError::Unauthorized));
        assert_eq!(error.status(), Some(StatusCode::UNAUTHORIZED));
    }

    #[tokio::test]
    async fn another_failing_status_keeps_its_code_and_body() {
        let base_url = start_test_server().await;
        let url = format!("{base_url}/status/503");

        let error = Call::get(&url)
            .send::<()>()
            .await
            .expect_err("A 503 must not parse as an answer");

        assert_eq!(error.status(), Some(StatusCode::SERVICE_UNAVAILABLE));
        assert_eq!(
            error.to_string(),
            format!("[503 Service Unavailable] {url}: status 503")
        );
        let RequestError::Status { status, body, .. } = error else {
            panic!("A 503 must be the status error, got {error:?}");
        };
        assert_eq!(status, 503);
        assert_eq!(body, "status 503");
    }

    #[tokio::test]
    async fn an_empty_answer_parses_as_unit() -> Result<()> {
        let base_url = start_test_server().await;

        Call::post(format!("{base_url}/empty")).send::<()>().await?;

        Ok(())
    }

    #[tokio::test]
    async fn an_answer_of_another_shape_is_the_parse_error() {
        let base_url = start_test_server().await;

        let error = Call::get(format!("{base_url}/users"))
            .send::<u32>()
            .await
            .expect_err("A list of users must not parse as a number");

        assert!(matches!(error, RequestError::Parse { .. }));
        assert_eq!(error.status(), None);
    }
}
