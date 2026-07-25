use std::collections::BTreeMap;

use anyhow::Result;
use serde::{Serialize, de::DeserializeOwned};

use crate::rest::{Method, client::client, request::request_object};

pub async fn get<T: DeserializeOwned>(url: impl ToString) -> Result<T> {
    request_object(
        &client(),
        Method::Get,
        url,
        BTreeMap::default(),
        "null".to_owned(),
    )
    .await
}

pub async fn post<T: DeserializeOwned>(url: impl ToString, body: impl Serialize) -> Result<T> {
    request_object(
        &client(),
        Method::Post,
        url,
        BTreeMap::default(),
        serde_json::to_string(&body)?,
    )
    .await
}

pub async fn patch<T: DeserializeOwned>(url: impl ToString, body: impl Serialize) -> Result<T> {
    request_object(
        &client(),
        Method::Patch,
        url,
        BTreeMap::default(),
        serde_json::to_string(&body)?,
    )
    .await
}

pub async fn delete<T: DeserializeOwned>(url: impl ToString) -> Result<T> {
    request_object(&client(), Method::Delete, url, BTreeMap::default(), String::new()).await
}

pub async fn download(url: impl ToString) -> Result<Vec<u8>> {
    let url = url.to_string();
    let bytes = client().get(&url).send().await?.bytes().await?;
    Ok(bytes.to_vec())
}

#[cfg(all(test, not_wasm))]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    #[cfg(not_wasm)]
    mod not_wasm_tests {
        use super::*;

        #[tokio::test]
        async fn test_download() -> Result<()> {
            let bytes = download("https://www.lrt.lt/img/2026/02/26/2327389-490277-615x345.jpg").await?;
            assert_eq!(bytes.len(), 40246);
            Ok(())
        }

        /// jsonplaceholder answers a POST with 201, which also proves
        /// every 2xx is accepted, not only 200.
        #[tokio::test]
        async fn test_post() -> Result<()> {
            let created: Value = post(
                "https://jsonplaceholder.typicode.com/posts",
                json!({ "title": "netrun", "userId": 1 }),
            )
            .await?;
            assert_eq!(created["id"], 101);
            assert_eq!(created["title"], "netrun");
            Ok(())
        }

        #[tokio::test]
        async fn test_patch() -> Result<()> {
            let patched: Value = patch(
                "https://jsonplaceholder.typicode.com/posts/1",
                json!({ "title": "renamed" }),
            )
            .await?;
            assert_eq!(patched["id"], 1);
            assert_eq!(patched["title"], "renamed");
            Ok(())
        }

        #[tokio::test]
        async fn test_delete() -> Result<()> {
            let deleted: Value = delete("https://jsonplaceholder.typicode.com/posts/1").await?;
            assert_eq!(deleted, json!({}));
            Ok(())
        }
    }
}
