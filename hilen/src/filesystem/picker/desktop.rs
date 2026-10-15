use std::{
    fs::{read, write},
    path::PathBuf,
};

use log::error;
use rfd::AsyncFileDialog;

use crate::filesystem::picker::{
    PickedFile,
    save::{SavedFile, filter_name, with_extension},
};

/// The extensions the dialog offers. The bytes go out as they are, so a
/// format the engine cannot decode is still fine to attach.
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "heic", "heif", "bmp", "tif", "tiff",
];

pub(super) async fn pick_image(title: &str) -> Option<PickedFile> {
    let dialog = AsyncFileDialog::new().set_title(title).add_filter("Images", IMAGE_EXTENSIONS);

    pick(dialog).await
}

pub(super) async fn pick_file(title: &str, extensions: &[String]) -> Option<PickedFile> {
    let dialog = AsyncFileDialog::new().set_title(title);

    // A filter with no extension lets no file through on some systems.
    let dialog = if extensions.is_empty() {
        dialog
    } else {
        dialog.add_filter(title, extensions)
    };

    pick(dialog).await
}

/// The save dialog, `name` is the suggested file name.
pub(super) async fn save_path(title: &str, name: &str, extensions: &[String]) -> Option<PathBuf> {
    let dialog = AsyncFileDialog::new().set_title(title).set_file_name(name);

    // A filter with no extension lets no file through on some systems.
    let dialog = if extensions.is_empty() {
        dialog
    } else {
        dialog.add_filter(filter_name(extensions), extensions)
    };

    let handle = dialog.save_file().await?;
    Some(with_extension(handle.path().to_owned(), extensions))
}

pub(super) async fn save(title: &str, name: &str, extensions: &[String], bytes: &[u8]) -> Option<SavedFile> {
    let path = save_path(title, name, extensions).await?;

    if let Err(err) = write(&path, bytes) {
        error!("file {} failed to save: {err}", path.display());
        return None;
    }

    let name = path
        .file_name()
        .map_or_else(|| name.to_string(), |name| name.to_string_lossy().into_owned());
    Some(SavedFile {
        name,
        path: Some(path),
    })
}

async fn pick(dialog: AsyncFileDialog) -> Option<PickedFile> {
    let handle = dialog.pick_file().await?;

    let path = handle.path();
    let bytes = match read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            error!("picked file {} failed to read: {err}", path.display());
            return None;
        }
    };

    Some(PickedFile::new(handle.file_name(), None, bytes))
}
