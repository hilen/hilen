use std::fs::read;

use log::error;
use rfd::AsyncFileDialog;

use crate::filesystem::picker::PickedFile;

/// The extensions the dialog offers. The bytes go out as they are, so a
/// format the engine cannot decode is still fine to attach.
const IMAGE_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "heic", "heif", "bmp", "tif", "tiff",
];

pub(super) async fn pick(title: &str) -> Option<PickedFile> {
    let handle = AsyncFileDialog::new()
        .set_title(title)
        .add_filter("Images", IMAGE_EXTENSIONS)
        .pick_file()
        .await?;

    let path = handle.path();
    let bytes = match read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            error!("picked image {} failed to read: {err}", path.display());
            return None;
        }
    };

    Some(PickedFile::new(handle.file_name(), None, bytes))
}
