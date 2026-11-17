#[cfg(not_wasm)]
use std::path::Path;

use anyhow::{Result, anyhow};
use futures_util::StreamExt;
use serde::{Serialize, de::DeserializeOwned};

use crate::deps::netrun::rest::{Call, RequestError, client::shared_client};

pub async fn get<T: DeserializeOwned>(url: impl ToString) -> Result<T, RequestError> {
    Call::get(url).send().await
}

pub async fn post<T: DeserializeOwned>(url: impl ToString, body: impl Serialize) -> Result<T, RequestError> {
    Call::post(url).body(body).send().await
}

pub async fn patch<T: DeserializeOwned>(url: impl ToString, body: impl Serialize) -> Result<T, RequestError> {
    Call::patch(url).body(body).send().await
}

pub async fn delete<T: DeserializeOwned>(url: impl ToString) -> Result<T, RequestError> {
    Call::delete(url).send().await
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
    let response = shared_client().get(&url).send().await?;
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

/// Downloads into the file `path` chunk by chunk, so a big file never sits
/// in memory, and reports the bytes so far plus the total like
/// `download_with_progress`. The bytes go to `<path>.part` first and get
/// their name only when all of them came, so a broken download never looks
/// like a whole file.
#[cfg(not_wasm)]
pub async fn download_to_file(
    url: impl ToString,
    path: impl AsRef<Path>,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<()> {
    use tokio::{fs, io::AsyncWriteExt};

    let url = url.to_string();
    let path = path.as_ref();
    let response = shared_client().get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(anyhow!("[{status}] Failed to download {url}"));
    }

    let mut part = path.as_os_str().to_owned();
    part.push(".part");
    let total = response.content_length();
    let mut file = fs::File::create(&part).await?;
    let mut done = 0;
    on_progress(0, total);

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk).await?;
        done += chunk.len() as u64;
        on_progress(done, total);
    }
    file.flush().await?;
    drop(file);

    if let Some(total) = total
        && done != total
    {
        fs::remove_file(&part).await?;
        return Err(anyhow!("{url} ended after {done} of {total} bytes"));
    }
    fs::rename(&part, path).await?;
    Ok(())
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
    async fn test_download_to_file() -> Result<()> {
        use std::{env::temp_dir, process::id};

        use tokio::fs::{create_dir_all, read, remove_dir_all};

        let base_url = start_test_server().await;
        let dir = temp_dir().join(format!("hilen-download-{}", id()));
        create_dir_all(&dir).await?;
        let path = dir.join("file.bin");
        let size = u64::try_from(FILE_SIZE)?;
        let mut last = None;

        download_to_file(format!("{base_url}/file"), &path, |done, total| {
            last = Some((done, total));
        })
        .await?;

        assert_eq!(read(&path).await?.len(), FILE_SIZE);
        assert_eq!(last, Some((size, Some(size))));
        assert!(!dir.join("file.bin.part").exists());
        remove_dir_all(&dir).await?;
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
