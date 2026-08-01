use std::{collections::BTreeMap, sync::OnceLock};

use parking_lot::Mutex;
use reqwest::Client;
use serde::{Serialize, de::DeserializeOwned};

use crate::rest::{Method, Request};

#[derive(Debug)]
pub struct RestAPI {
    base_url: &'static str,
    headers:  Mutex<BTreeMap<String, String>>,
    client:   OnceLock<Client>,
}

impl RestAPI {
    pub const fn new(base_url: &'static str) -> Self {
        Self {
            base_url,
            headers: Mutex::new(BTreeMap::new()),
            client: OnceLock::new(),
        }
    }
}

impl RestAPI {
    pub fn base_url(&self) -> &str {
        self.base_url
    }

    pub(crate) fn client(&self) -> &Client {
        self.client.get_or_init(crate::rest::client::client)
    }

    pub fn headers(&self) -> BTreeMap<String, String> {
        self.headers.lock().clone()
    }

    pub fn remove_header(&self, key: impl ToString) {
        self.headers.lock().remove(&key.to_string());
    }

    pub fn clear_all_headers(&self) {
        self.headers.lock().clear();
    }

    pub fn add_header(&self, key: impl ToString, value: impl ToString) {
        self.headers.lock().insert(key.to_string(), value.to_string());
    }

    pub fn set_access_token(&self, token: impl ToString) {
        self.add_header("token", token);
    }
}

impl RestAPI {
    pub const fn request<In: Serialize, Out: DeserializeOwned>(
        &'static self,
        path: &'static str,
    ) -> Request<In, Out> {
        Request::new(path, self, None)
    }

    pub const fn get<In: Serialize, Out: DeserializeOwned>(
        &'static self,
        path: &'static str,
    ) -> Request<In, Out> {
        Request::new(path, self, Some(Method::Get))
    }
}
