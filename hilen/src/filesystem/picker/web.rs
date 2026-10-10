use anyhow::{Result, anyhow};
use log::error;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    File, HtmlInputElement,
    js_sys::{Promise, Uint8Array},
    wasm_bindgen::JsCast,
};

use crate::filesystem::picker::PickedFile;

/// `accept` is the filter of the chooser, like `image/*` or `.pdf,.txt`.
/// An empty one lets every file through.
pub(super) async fn pick(accept: &str) -> Option<PickedFile> {
    match chosen_file(accept).await {
        Ok(Some(file)) => match read(&file).await {
            Ok(picked) => Some(picked),
            Err(err) => {
                error!("picked file {} failed to read: {err}", file.name());
                None
            }
        },
        Ok(None) => None,
        Err(err) => {
            error!("file chooser failed: {err}");
            None
        }
    }
}

/// The input is never added to the page, a click on a detached input
/// opens the chooser too. A browser older than the `cancel` event, Chrome
/// 113 and Safari 16.4, never answers a cancel, so the future then stays
/// pending.
async fn chosen_file(accept: &str) -> Result<Option<File>> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| anyhow!("no document"))?;

    let input: HtmlInputElement = document
        .create_element("input")
        .map_err(|err| anyhow!("failed to create the file input: {err:?}"))?
        .dyn_into()
        .map_err(|err| anyhow!("the file input is not an input: {err:?}"))?;
    input.set_type("file");
    if !accept.is_empty() {
        input.set_accept(accept);
    }

    // A cancel rejects, so the await tells the two answers apart.
    let answered = Promise::new(&mut |resolve, reject| {
        input.set_onchange(Some(&resolve));
        input.set_oncancel(Some(&reject));
    });

    input.click();

    if JsFuture::from(answered).await.is_err() {
        return Ok(None);
    }

    Ok(input.files().and_then(|files| files.get(0)))
}

async fn read(file: &File) -> Result<PickedFile> {
    let buffer = JsFuture::from(file.array_buffer()).await.map_err(|err| anyhow!("{err:?}"))?;

    Ok(PickedFile::new(
        file.name(),
        Some(file.type_()),
        Uint8Array::new(&buffer).to_vec(),
    ))
}
