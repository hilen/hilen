//! The system image picker and file picker of each platform. A browser
//! gives no path, so every platform answers with the bytes of the picked
//! file.

#[cfg(android)]
mod android;
#[cfg(desktop)]
mod desktop;
#[cfg(all(ios, not(tvos)))]
mod ios;
#[cfg(all(ios, not(tvos)))]
mod ios_document;
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

/// Lets go of what the pickers hold of this build, when a hot build stops.
#[cfg(all(hot, not(tvos)))]
pub(crate) fn stop() {
    ios::stop();
    ios_document::stop();
}

/// Lower case and with no dot in front, the form every platform filter
/// is built from.
#[cfg(not(tvos))]
fn clean_extensions(extensions: &[&str]) -> Vec<String> {
    extensions
        .iter()
        .map(|extension| extension.trim().trim_start_matches('.').to_lowercase())
        .filter(|extension| !extension.is_empty())
        .collect()
}

/// The `accept` of a file input, like `.pdf,.txt`.
#[cfg(any(wasm, test))]
fn accept_of(extensions: &[String]) -> String {
    extensions
        .iter()
        .map(|extension| format!(".{extension}"))
        .collect::<Vec<_>>()
        .join(",")
}

/// The MIME types of the extensions, each once, for a chooser that
/// filters by type. Empty when an extension has no known type. The
/// chooser then shows every file, a file of that kind would be greyed
/// out otherwise.
#[cfg(any(android, test))]
fn mime_types(extensions: &[String]) -> Vec<String> {
    let mut types: Vec<String> = Vec::new();

    for extension in extensions {
        let Some(mime) = mime_guess::from_ext(extension).first_raw() else {
            return Vec::new();
        };
        if !types.iter().any(|known| known == mime) {
            types.push(mime.to_string());
        }
    }

    types
}

impl Paths {
    /// Lets the user pick one file of any kind and reads it. The Files
    /// browser on iOS, the system file chooser on Android, the file dialog
    /// on desktop, the file chooser of the browser. `extensions` narrows
    /// what can be picked, like `&["pdf", "txt"]`, an empty list lets every
    /// file through. `None` when the user cancels or the read fails, a
    /// failure is logged. The whole file is held in memory. Await it inside
    /// `spawn`, then go back with `on_main`. In a browser start it from a
    /// tap, the browser opens the chooser only right after a user gesture.
    /// tvOS has no picker and answers `None` at once. `pick_file` gives a
    /// path in place of the bytes, on desktop only.
    pub async fn pick_file_bytes(title: &str, extensions: &[&str]) -> Option<PickedFile> {
        debug!("file picker opens: {title} {extensions:?}");

        #[cfg(not(tvos))]
        let extensions = clean_extensions(extensions);

        #[cfg(desktop)]
        let picked = desktop::pick_file(title, &extensions).await;
        #[cfg(all(ios, not(tvos)))]
        let picked = ios_document::pick(extensions).await;
        // tvOS has no Files browser an app can open.
        #[cfg(tvos)]
        let picked = std::future::ready(None::<PickedFile>).await;
        #[cfg(android)]
        let picked = {
            let types = mime_types(&extensions);
            android::pick(title, android::Kind::File(&types)).await
        };
        #[cfg(wasm)]
        let picked = web::pick(&accept_of(&extensions)).await;

        match &picked {
            Some(file) => info!(
                "file picked: {} {} {} bytes",
                file.name,
                file.content_type,
                file.bytes.len()
            ),
            None => debug!("file picker closed with no file"),
        }

        picked
    }

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
        let picked = desktop::pick_image(title).await;
        #[cfg(all(ios, not(tvos)))]
        let picked = ios::pick().await;
        // tvOS has no photo library an app can open.
        #[cfg(tvos)]
        let picked = std::future::ready(None::<PickedFile>).await;
        #[cfg(android)]
        let picked = android::pick(title, android::Kind::Image).await;
        #[cfg(wasm)]
        let picked = web::pick("image/*").await;

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
    use super::{PickedFile, accept_of, clean_extensions, mime_types};

    const NONE: [String; 0] = [];

    fn cleaned(extensions: &[&str]) -> Vec<String> {
        clean_extensions(extensions)
    }

    #[test]
    fn extensions_lose_the_dot_and_the_case() {
        assert_eq!(cleaned(&[".PDF", "txt", " Md ", "", "."]), ["pdf", "txt", "md"]);
        assert_eq!(cleaned(&[]), NONE);
    }

    #[test]
    fn accept_of_a_file_input() {
        assert_eq!(accept_of(&cleaned(&["pdf", ".txt"])), ".pdf,.txt");
        assert_eq!(accept_of(&[]), "");
    }

    /// 2 extensions of 1 type give it once. One unknown extension gives
    /// no filter at all, or its files could not be picked.
    #[test]
    fn mime_types_of_extensions() {
        assert_eq!(
            mime_types(&cleaned(&["jpg", "jpeg", "pdf"])),
            ["image/jpeg", "application/pdf"]
        );
        assert_eq!(mime_types(&cleaned(&["pdf", "no-such-extension"])), NONE);
        assert_eq!(mime_types(&[]), NONE);
    }

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
