use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use serde::{Serialize, de::DeserializeOwned};

use crate::deps::netrun::rest::{Method, client::client, request::request_object};

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
    download_with_progress(url, |_, _| {}).await
}

/// Downloads in chunks and reports the bytes so far plus the total from
/// the Content-Length header, `None` when the server sent no length.
pub async fn download_with_progress(
    url: impl ToString,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>> {
    let url = url.to_string();
    let mut response = client().get(&url).send().await?;
    let status = response.status();

    // Without this a block page or an error page downloads as if it were the
    // file, and the caller only sees a body of the wrong size.
    if !status.is_success() {
        return Err(anyhow!("[{status}] Failed to download {url}"));
    }

    let total = response.content_length();
    let mut bytes = Vec::with_capacity(usize::try_from(total.unwrap_or_default()).unwrap_or_default());
    on_progress(0, total);

    while let Some(chunk) = response.chunk().await? {
        bytes.extend_from_slice(&chunk);
        on_progress(bytes.len() as u64, total);
    }

    Ok(bytes)
}

#[cfg(all(test, not_wasm))]
mod tests {
    use serde_json::{Value, json};

    use super::*;

    #[cfg(not_wasm)]
    mod not_wasm_tests {
        use super::*;

        /// The url points at one pinned commit, so the bytes cannot change
        /// under the test. It used to download an image from a news site and
        /// one CI run got a 134 byte page in place of the image.
        #[tokio::test]
        async fn test_download() -> Result<()> {
            let bytes = download(
                "https://raw.githubusercontent.com/VladasZ/netrun/4b28fc5e444f0b4f9d08f4b26e3a3241995d6daf/Cargo.lock",
            )
            .await?;
            assert_eq!(bytes.len(), 97126);
            Ok(())
        }

        #[tokio::test]
        async fn test_download_with_progress() -> Result<()> {
            let mut reports: Vec<(u64, Option<u64>)> = vec![];
            let bytes = download_with_progress(
                "https://raw.githubusercontent.com/VladasZ/netrun/4b28fc5e444f0b4f9d08f4b26e3a3241995d6daf/Cargo.lock",
                |done, total| reports.push((done, total)),
            )
            .await?;
            assert_eq!(bytes.len(), 97126);
            assert_eq!(reports.first(), Some(&(0, Some(97126))));
            assert_eq!(reports.last(), Some(&(97126, Some(97126))));
            assert!(reports.windows(2).all(|w| w[0].0 <= w[1].0));
            Ok(())
        }

        #[tokio::test]
        async fn test_download_missing_file() -> Result<()> {
            let error = download(
                "https://raw.githubusercontent.com/VladasZ/netrun/4b28fc5e444f0b4f9d08f4b26e3a3241995d6daf/no-such-file",
            )
            .await
            .expect_err("A missing file must not download as bytes");

            assert!(error.to_string().starts_with("[404 Not Found]"));

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
