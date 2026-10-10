use std::fs::read;

use log::error;
use rfd::AsyncFileDialog;

use crate::filesystem::picker::PickedFile;

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
