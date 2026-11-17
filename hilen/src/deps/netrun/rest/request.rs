use std::{any::type_name, borrow::Borrow, collections::BTreeMap, marker::PhantomData};

use serde::{Serialize, de::DeserializeOwned};

use crate::deps::netrun::rest::{Call, Method, RequestError, RestAPI};

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
    pub async fn send(&self, param: impl Borrow<In>) -> Result<Out, RequestError> {
        self.with_headers(param, self.api.headers()).await
    }

    pub async fn with_token(
        &self,
        param: impl Borrow<In>,
        token: impl ToString,
    ) -> Result<Out, RequestError> {
        self.with_headers(param, [("token".to_string(), token.to_string())]).await
    }

    pub async fn with_headers(
        &self,
        param: impl Borrow<In>,
        headers: impl Into<BTreeMap<String, String>>,
    ) -> Result<Out, RequestError> {
        let call = Call::new(self.method, self.full_url()).headers(headers.into());

        if self.method.get() {
            call.send().await
        } else {
            call.body(param.borrow()).send().await
        }
    }
}

#[cfg(target_arch = "wasm32")]
type RequestFuture<T> = std::pin::Pin<Box<dyn Future<Output = T>>>;

#[cfg(not(target_arch = "wasm32"))]
type RequestFuture<T> = std::pin::Pin<Box<dyn Future<Output = T> + Send>>;

impl<Out: DeserializeOwned + 'static> IntoFuture for Request<(), Out> {
    type Output = Result<Out, RequestError>;
    type IntoFuture = RequestFuture<Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move { self.send(()).await })
    }
}
