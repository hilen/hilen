use std::collections::BTreeMap;

use anyhow::{Result, anyhow};
use futures_util::StreamExt;
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
    let response = client().get(&url).send().await?;
    let status = response.status();

    // Without this a block page or an error page downloads as if it were the
    // file, and the caller only sees a body of the wrong size.
    if !status.is_success() {
        return Err(anyhow!("[{status}] Failed to download {url}"));
    }

    let total = response.content_length();
    let mut bytes = Vec::with_capacity(usize::try_from(total.unwrap_or_default()).unwrap_or_default());
    on_progress(0, total);

    // `bytes_stream` is the one chunked read reqwest has on both native and
    // wasm, `chunk()` does not exist on the wasm target.
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        bytes.extend_from_slice(&chunk?);
        on_progress(bytes.len() as u64, total);
    }

    Ok(bytes)
}

#[cfg(all(test, not_wasm))]
mod tests {
    use super::*;
    use crate::deps::netrun::test_server::{Empty, FILE_SIZE, NewPost, Post, PostPatch, start_test_server};

    #[tokio::test]
    async fn test_download() -> Result<()> {
        let base_url = start_test_server().await;

        let bytes = download(format!("{base_url}/file")).await?;

        assert_eq!(bytes.len(), FILE_SIZE);

        Ok(())
    }

    #[tokio::test]
    async fn test_download_with_progress() -> Result<()> {
        let base_url = start_test_server().await;
        let size = u64::try_from(FILE_SIZE)?;
        let mut reports: Vec<(u64, Option<u64>)> = vec![];

        let bytes = download_with_progress(format!("{base_url}/file"), |done, total| {
            reports.push((done, total));
        })
        .await?;

        assert_eq!(bytes.len(), FILE_SIZE);
        assert_eq!(reports.first(), Some(&(0, Some(size))));
        assert_eq!(reports.last(), Some(&(size, Some(size))));
        assert!(reports.windows(2).all(|w| w[0].0 <= w[1].0));

        Ok(())
    }

    #[tokio::test]
    async fn test_download_missing_file() -> Result<()> {
        let base_url = start_test_server().await;

        let error = download(format!("{base_url}/no-such-file"))
            .await
            .expect_err("A missing file must not download as bytes");

        assert!(error.to_string().starts_with("[404 Not Found]"));

        Ok(())
    }

    /// The server answers a POST with 201, which also proves every 2xx is
    /// accepted, not only 200.
    #[tokio::test]
    async fn test_post() -> Result<()> {
        let base_url = start_test_server().await;
        let new = NewPost {
            title:   "netrun".to_string(),
            user_id: 1,
        };

        let created: Post = post(format!("{base_url}/posts"), new).await?;

        assert_eq!(created.id, 101);
        assert_eq!(created.title, "netrun");
        assert_eq!(created.user_id, Some(1));

        Ok(())
    }

    #[tokio::test]
    async fn test_patch() -> Result<()> {
        let base_url = start_test_server().await;
        let rename = PostPatch {
            title: "renamed".to_string(),
        };

        let patched: Post = patch(format!("{base_url}/posts/1"), rename).await?;

        assert_eq!(patched.id, 1);
        assert_eq!(patched.title, "renamed");
        assert_eq!(patched.user_id, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete() -> Result<()> {
        let base_url = start_test_server().await;

        let deleted: Empty = delete(format!("{base_url}/posts/1")).await?;

        assert_eq!(deleted, Empty {});

        Ok(())
    }
}
