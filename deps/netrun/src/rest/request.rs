use std::{any::type_name, borrow::Borrow, collections::BTreeMap, marker::PhantomData};

use anyhow::{Result, anyhow};
use log::{debug, error};
use reqwest::Client;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{from_str, to_string};

use crate::rest::{Method, Response, RestAPI};

#[derive(Debug)]
pub struct Request<In: Serialize, Out: DeserializeOwned> {
    path:   &'static str,
    api:    &'static RestAPI,
    method: Method,
    _p:     PhantomData<fn(In) -> Out>,
}

impl<In: Serialize, Out: DeserializeOwned> Copy for Request<In, Out> {}
impl<In: Serialize, Out: DeserializeOwned> Clone for Request<In, Out> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<In: Serialize, Out: DeserializeOwned> Request<In, Out> {
    pub(crate) const fn new(path: &'static str, api: &'static RestAPI, method: Option<Method>) -> Self {
        let method = match method {
            Some(method) => method,
            None => Method::Post,
        };

        Self {
            path,
            api,
            method,
            _p: PhantomData,
        }
    }

    pub const fn method(&self) -> Method {
        self.method
    }

    pub const fn path(&self) -> &'static str {
        self.path
    }

    pub fn full_url(&self) -> String {
        format!("{}/{}", self.api.base_url(), self.path)
    }

    pub fn description(&self) -> String {
        format!("{} {}->{}", self.path, type_name::<In>(), type_name::<Out>())
    }
}

impl<In: Serialize, Out: DeserializeOwned> Request<In, Out> {
    pub async fn send(&self, param: impl Borrow<In>) -> Result<Out> {
        request_object(
            self.api.client(),
            self.method,
            self.full_url(),
            self.api.headers(),
            to_body(param)?,
        )
        .await
    }

    pub async fn with_token(&self, param: impl Borrow<In>, token: impl ToString) -> Result<Out> {
        self.with_headers(param, [("token".to_string(), token.to_string())]).await
    }

    pub async fn with_headers(
        &self,
        param: impl Borrow<In>,
        headers: impl Into<BTreeMap<String, String>>,
    ) -> Result<Out> {
        request_object(
            self.api.client(),
            self.method,
            self.full_url(),
            headers.into(),
            to_body(param)?,
        )
        .await
    }
}

pub(crate) async fn request_object<T>(
    client: &Client,
    method: Method,
    url: impl ToString,
    headers: BTreeMap<String, String>,
    body: String,
) -> Result<T>
where
    T: DeserializeOwned,
{
    let url = url.to_string();

    let response = raw_request(client, method, &url, headers, body).await?;

    if response.status == 404 {
        Err(anyhow!("Endpoint {url} not found. 404."))
    } else if !response.status.is_success() {
        Err(anyhow!("[{}] {}", response.status, response.body))
    } else {
        Ok(parse(&response.body)?)
    }
}

fn parse<T: DeserializeOwned>(json: impl ToString) -> Result<T> {
    let mut json = json.to_string();

    // A 204 or an empty 200 body parses as JSON null, so endpoints that
    // answer with nothing work with `()` outputs.
    if json.trim().is_empty() {
        json = "null".to_string();
    }

    match from_str(&json) {
        Ok(obj) => Ok(obj),
        Err(error) => {
            let message = format!("Failed to parse {} from {json}. Error: {error}", type_name::<T>());
            error!("{message}");
            Err(anyhow!(message))
        }
    }
}

async fn raw_request(
    client: &Client,
    method: Method,
    url: impl ToString,
    headers: BTreeMap<String, String>,
    body: String,
) -> Result<Response> {
    let url = url.to_string();

    debug!("Request: {url} - {method}. Body: {body:?}");

    let mut request = match method {
        Method::Get => client.get(&url),
        Method::Post => client.post(&url),
        Method::Patch => client.patch(&url),
        Method::Delete => client.delete(&url),
    };

    request = request.header("content-type", "application/json");

    for (key, value) in headers {
        request = request.header(key, value);
    }

    let request = if method.get() { request } else { request.body(body) };

    let response = request.send().await.map_err(|e| {
        error!("Failed to send request: {e:?}");
        e
    })?;

    let status = response.status();
    let body = response.text().await?;

    let response = Response { url, status, body };

    debug!("Response: {} - {}", response.url, response.status);

    Ok(response)
}

fn to_body<Param: Serialize>(param: impl Borrow<Param>) -> Result<String> {
    Ok(to_string(param.borrow())?)
}

#[cfg(target_arch = "wasm32")]
type RequestFuture<T> = std::pin::Pin<Box<dyn Future<Output = T>>>;

#[cfg(not(target_arch = "wasm32"))]
type RequestFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send>>;

impl<Out: DeserializeOwned + 'static> IntoFuture for Request<(), Out> {
    type Output = Result<Out>;
    type IntoFuture = RequestFuture<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send(()).await })
    }
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn empty_body_parses_as_null() {
        parse::<()>("").unwrap();
        parse::<()>("  \n").unwrap();
        assert_eq!(parse::<Option<u32>>("").unwrap(), None);
        assert!(parse::<u32>("").is_err());
    }
}
