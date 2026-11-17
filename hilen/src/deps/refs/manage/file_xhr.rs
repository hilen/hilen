//! A page opened from `file://`, like an app packed whole for an LG webOS
//! TV, cannot `fetch` its own files. Chromium 79 there fails with `Failed to
//! fetch`, while an XHR of the same file works.

use anyhow::{Result, anyhow};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    XmlHttpRequest, XmlHttpRequestResponseType,
    js_sys::{ArrayBuffer, Promise, Uint8Array},
    wasm_bindgen::{JsCast, JsValue, closure::Closure},
};

pub(super) fn is_file(url: &str) -> bool {
    url.starts_with("file:")
}

/// The status of a file request is 0 also when it worked. A missing file
/// leaves no response, that is the error.
pub(super) async fn read(url: &str) -> Result<Vec<u8>> {
    let request =
        XmlHttpRequest::new().map_err(|err| anyhow!("Failed to create a request for {url}: {err:?}"))?;
    request
        .open("GET", url)
        .map_err(|err| anyhow!("Failed to open a request for {url}: {err:?}"))?;
    request.set_response_type(XmlHttpRequestResponseType::Arraybuffer);

    // loadend comes once after a load and after an error, so 1 listener
    // is always called and never leaks.
    let ended = Promise::new(&mut |resolve, _reject| {
        let on_end = Closure::once_into_js(move || {
            if let Err(err) = resolve.call0(&JsValue::NULL) {
                log::error!("Failed to resolve a file request: {err:?}");
            }
        });
        request.set_onloadend(Some(on_end.unchecked_ref()));
    });

    request
        .send()
        .map_err(|err| anyhow!("Failed to send a request for {url}: {err:?}"))?;
    JsFuture::from(ended)
        .await
        .map_err(|err| anyhow!("Request for {url} failed: {err:?}"))?;

    let response = request.response().map_err(|err| anyhow!("No response for {url}: {err:?}"))?;
    let Ok(buffer) = response.dyn_into::<ArrayBuffer>() else {
        return Err(anyhow!("No file at {url}"));
    };
    Ok(Uint8Array::new(&buffer).to_vec())
}
