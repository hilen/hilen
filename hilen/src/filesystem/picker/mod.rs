//! The system image picker of each platform. A browser gives no path, so
//! every platform answers with the bytes of the picked file.

#[cfg(android)]
mod android;
#[cfg(desktop)]
mod desktop;
#[cfg(all(ios, not(tvos)))]
mod ios;
#[cfg(all(hot, not(tvos)))]
pub(crate) use ios::stop;
#[cfg(wasm)]
mod web;

use log::{debug, info};

use crate::filesystem::Paths;

/// A file the user picked, read into memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickedFile {
    /// The file name with its extension, without a folder.
    pub name:         String,
    /// The MIME type, like `image/jpeg`.
    pub content_type: String,
    pub bytes:        Vec<u8>,
}

// tvOS has no picker, nothing makes a picked file there.
#[cfg(not(tvos))]
impl PickedFile {
    /// A platform that gives no type has one guessed from the name.
    pub(crate) fn new(name: String, content_type: Option<String>, bytes: Vec<u8>) -> Self {
        let content_type = content_type
            .filter(|content_type| !content_type.is_empty())
            .unwrap_or_else(|| content_type_of(&name));

        Self {
            name,
            content_type,
            bytes,
        }
    }
}

#[cfg(not(tvos))]
fn content_type_of(name: &str) -> String {
    mime_guess::from_path(name)
        .first_raw()
        .unwrap_or("application/octet-stream")
        .to_string()
}

impl Paths {
    /// Lets the user pick one image and reads it. The photo library on a
    /// phone, the file dialog on desktop, the file chooser of the browser.
    /// `None` when the user cancels or the read fails, a failure is logged.
    /// The title shows where the platform has a place for it. Await it
    /// inside `spawn`, then go back with `on_main`. In a browser start it
    /// from a tap, the browser opens the chooser only right after a user
    /// gesture. tvOS has no picker and answers `None` at once.
    pub async fn pick_image(title: &str) -> Option<PickedFile> {
        debug!("image picker opens: {title}");

        #[cfg(desktop)]
        let picked = desktop::pick(title).await;
        #[cfg(all(ios, not(tvos)))]
        let picked = ios::pick().await;
        // tvOS has no photo library an app can open.
        #[cfg(tvos)]
        let picked = std::future::ready(None::<PickedFile>).await;
        #[cfg(android)]
        let picked = android::pick(title).await;
        #[cfg(wasm)]
        let picked = web::pick().await;

        match &picked {
            Some(file) => info!(
                "image picked: {} {} {} bytes",
                file.name,
                file.content_type,
                file.bytes.len()
            ),
            None => debug!("image picker closed with no image"),
        }

        picked
    }
}

#[cfg(test)]
mod tests {
    use super::PickedFile;

    /// A platform type wins, an empty or missing one is guessed from the
    /// name, an unknown name gets the generic binary type.
    #[test]
    fn content_type_from_platform_or_name() {
        let given = PickedFile::new("photo.bin".to_string(), Some("image/heic".to_string()), vec![1]);
        assert_eq!(given.content_type, "image/heic");

        let guessed = PickedFile::new("photo.JPG".to_string(), Some(String::new()), vec![1]);
        assert_eq!(guessed.content_type, "image/jpeg");

        let png = PickedFile::new("shot.png".to_string(), None, vec![1]);
        assert_eq!(png.content_type, "image/png");

        let unknown = PickedFile::new("photo".to_string(), None, vec![1]);
        assert_eq!(unknown.content_type, "application/octet-stream");
    }
}
