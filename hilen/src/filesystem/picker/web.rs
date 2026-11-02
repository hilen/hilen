use anyhow::{Result, anyhow};
use log::{error, warn};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Blob, BlobPropertyBag, File, FileList, HtmlElement, HtmlInputElement, Url,
    js_sys::{Array, Promise, Reflect, Uint8Array},
    wasm_bindgen::{JsCast, JsValue},
};

use crate::{
    deps::hreads::after,
    filesystem::picker::{PickedFile, folder_of, save::SavedFile},
};

/// The link of a download is let go after this many seconds. A browser
/// that starts the download later than the click still has it then.
const LINK_SECONDS: f32 = 60.0;

/// Saves the bytes as a download named `name`. The browser asks no
/// question, or its own one, and never tells the page where the file
/// went or whether it was kept.
pub(super) fn download(name: &str, content_type: &str, bytes: &[u8]) -> Option<SavedFile> {
    match start_download(name, content_type, bytes) {
        Ok(()) => Some(SavedFile {
            name: name.to_string(),
            path: None,
        }),
        Err(err) => {
            error!("download of {name} failed: {err}");
            None
        }
    }
}

fn start_download(name: &str, content_type: &str, bytes: &[u8]) -> Result<()> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or_else(|| anyhow!("no document"))?;

    let parts = Array::new();
    parts.push(&Uint8Array::from(bytes));
    let options = BlobPropertyBag::new();
    options.set_type(content_type);
    let blob = Blob::new_with_u8_array_sequence_and_options(&parts, &options)
        .map_err(|err| anyhow!("failed to make the blob: {err:?}"))?;
    let url =
        Url::create_object_url_with_blob(&blob).map_err(|err| anyhow!("failed to make the link: {err:?}"))?;

    // A link that is never added to the page downloads on a click too.
    let link: HtmlElement = document
        .create_element("a")
        .map_err(|err| anyhow!("failed to create the link: {err:?}"))?
        .dyn_into()
        .map_err(|err| anyhow!("the link is not an element: {err:?}"))?;
    link.set_attribute("href", &url)
        .and_then(|()| link.set_attribute("download", name))
        .map_err(|err| anyhow!("failed to set the link: {err:?}"))?;
    link.click();

    after(LINK_SECONDS, move || {
        if let Err(err) = Url::revoke_object_url(&url) {
            warn!("the link of a download stays: {err:?}");
        }
    });
    Ok(())
}

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

/// `accept` as in `pick`, every file with `folder` false. With it the chooser
/// asks for a folder and gives each file in it.
pub(super) async fn pick_many(accept: &str, folder: bool) -> Vec<PickedFile> {
    match chosen_files(accept, folder).await {
        Ok(files) => read_all(files).await,
        Err(err) => {
            error!("file chooser failed: {err}");
            Vec::new()
        }
    }
}

/// Reads every file, one that fails is logged and left out.
pub(crate) async fn read_all(files: Option<FileList>) -> Vec<PickedFile> {
    let Some(files) = files else {
        return Vec::new();
    };
    let mut picked = Vec::new();
    for index in 0..files.length() {
        let Some(file) = files.get(index) else {
            continue;
        };
        match read(&file).await {
            Ok(read) => picked.push(read.in_folder(folder_of(&relative_path(&file)))),
            Err(err) => error!("picked file {} failed to read: {err}", file.name()),
        }
    }
    picked
}

/// `web_sys` has no call for this property, every browser has it.
fn relative_path(file: &File) -> String {
    Reflect::get(file, &JsValue::from_str("webkitRelativePath"))
        .ok()
        .and_then(|path| path.as_string())
        .unwrap_or_default()
}

async fn chosen_files(accept: &str, folder: bool) -> Result<Option<FileList>> {
    let input = file_input(accept)?;
    input.set_multiple(true);
    if folder {
        // No browser has a typed setter for it, the attribute works in all.
        input
            .set_attribute("webkitdirectory", "")
            .map_err(|err| anyhow!("failed to ask for a folder: {err:?}"))?;
    }
    if !answered(&input).await {
        return Ok(None);
    }
    Ok(input.files())
}

/// The input is never added to the page, a click on a detached input
/// opens the chooser too. A browser older than the `cancel` event, Chrome
/// 113 and Safari 16.4, never answers a cancel, so the future then stays
/// pending.
async fn chosen_file(accept: &str) -> Result<Option<File>> {
    let input = file_input(accept)?;
    if !answered(&input).await {
        return Ok(None);
    }
    Ok(input.files().and_then(|files| files.get(0)))
}

fn file_input(accept: &str) -> Result<HtmlInputElement> {
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
    Ok(input)
}

/// Opens the chooser. True when the user picked, false on a cancel.
async fn answered(input: &HtmlInputElement) -> bool {
    // A cancel rejects, so the await tells the two answers apart.
    let answered = Promise::new(&mut |resolve, reject| {
        input.set_onchange(Some(&resolve));
        input.set_oncancel(Some(&reject));
    });

    input.click();

    JsFuture::from(answered).await.is_ok()
}

async fn read(file: &File) -> Result<PickedFile> {
    let buffer = JsFuture::from(file.array_buffer()).await.map_err(|err| anyhow!("{err:?}"))?;

    Ok(PickedFile::new(
        file.name(),
        Some(file.type_()),
        Uint8Array::new(&buffer).to_vec(),
    ))
}
