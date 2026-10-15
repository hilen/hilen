//! The save dialog of each platform. The app hands over the bytes, so a
//! phone and a browser save too: the rfd dialog on desktop, the Files
//! sheet on iOS, the system file creator on Android and a download in a
//! browser.

#[cfg(not(tvos))]
use std::path::Path;
use std::path::PathBuf;

use log::{debug, info};

#[cfg(not(tvos))]
use super::clean_extensions;
use crate::filesystem::Paths;

/// The name a dialog suggests when the app gives none.
#[cfg(not(tvos))]
const UNTITLED: &str = "Untitled";

/// Where the user saved a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedFile {
    /// The file name the user kept or typed, with its extension.
    pub name: String,
    /// The path on desktop. A phone and a browser give no path.
    pub path: Option<PathBuf>,
}

/// The name the dialog starts with: no folder, never empty, and with the
/// first of the extensions when it has none of them.
#[cfg(not(tvos))]
pub(super) fn suggested_name(name: &str, extensions: &[String]) -> String {
    let name = name.rsplit(['/', '\\']).next().unwrap_or_default().trim();
    let name = if name.is_empty() { UNTITLED } else { name };

    let has_one = extension_of(Path::new(name)).is_some_and(|extension| extensions.contains(&extension));
    match extensions.first() {
        Some(first) if !has_one => format!("{name}.{first}"),
        _ => name.to_string(),
    }
}

/// The path the dialog answered, with the first extension when the user
/// typed a name with none. Some Linux dialogs give the name as typed.
#[cfg(any(desktop, test))]
pub(super) fn with_extension(path: PathBuf, extensions: &[String]) -> PathBuf {
    match extensions.first() {
        Some(first) if extension_of(&path).is_none() => {
            let mut name = path.file_name().unwrap_or_default().to_os_string();
            name.push(".");
            name.push(first);
            path.with_file_name(name)
        }
        _ => path,
    }
}

/// The name of the filter of a desktop dialog, like `STL, 3MF`.
#[cfg(any(desktop, test))]
pub(super) fn filter_name(extensions: &[String]) -> String {
    extensions
        .iter()
        .map(|extension| extension.to_uppercase())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(not(tvos))]
fn extension_of(path: &Path) -> Option<String> {
    path.extension().map(|extension| extension.to_string_lossy().to_lowercase())
}

impl Paths {
    /// Asks the user where to save a file and gives the path, the app
    /// writes the file itself. `name` is the suggested file name and
    /// `extensions` the kinds offered, like `&["stl", "3mf"]`. The first
    /// extension goes on a name that has none. `None` on a cancel.
    /// Desktop only, it answers `None` at once everywhere else, there
    /// `save_file_bytes` saves. Await it inside `spawn`, then go back with
    /// `on_main`.
    pub async fn save_file(title: &str, name: &str, extensions: &[&str]) -> Option<PathBuf> {
        debug!("save dialog opens: {title} {name} {extensions:?}");

        #[cfg(desktop)]
        let path = {
            let extensions = clean_extensions(extensions);
            super::desktop::save_path(title, &suggested_name(name, &extensions), &extensions).await
        };
        // The API is async on every platform, here the answer is immediate.
        #[cfg(not(desktop))]
        let path = std::future::ready(None::<PathBuf>).await;

        match &path {
            Some(path) => info!("save path chosen: {}", path.display()),
            None => debug!("save dialog closed with no path"),
        }
        path
    }

    /// Lets the user save `bytes` as a file, on every platform: the rfd
    /// dialog on desktop, the Files sheet on iOS, the system file creator
    /// on Android, a download in the browser. `name` is the suggested file
    /// name and `extensions` the kinds offered, the first one goes on a
    /// name that has none. `None` on a cancel or a failed write, a failure
    /// is logged. A browser saves with no question and answers with the
    /// suggested name, it does not tell where the file went or whether
    /// the user kept it. In a browser start it from a tap. tvOS has no
    /// files and answers `None`. Await it inside `spawn`, then go back
    /// with `on_main`.
    pub async fn save_file_bytes(
        title: &str,
        name: &str,
        extensions: &[&str],
        bytes: Vec<u8>,
    ) -> Option<SavedFile> {
        debug!(
            "save dialog opens: {title} {name} {extensions:?} {} bytes",
            bytes.len()
        );

        #[cfg(not(tvos))]
        let extensions = clean_extensions(extensions);
        #[cfg(not(tvos))]
        let name = suggested_name(name, &extensions);

        #[cfg(desktop)]
        let saved = super::desktop::save(title, &name, &extensions, &bytes).await;
        #[cfg(all(ios, not(tvos)))]
        let saved = super::ios_save::save(&name, &bytes).await;
        // tvOS has no files an app can save into.
        #[cfg(tvos)]
        let saved = std::future::ready(None::<SavedFile>).await;
        #[cfg(android)]
        let saved = super::android::save(&name, &super::content_type_of(&name), &bytes).await;
        // The API is async on every platform, a download starts at once.
        #[cfg(wasm)]
        let saved = std::future::ready(super::web::download(
            &name,
            &super::content_type_of(&name),
            &bytes,
        ))
        .await;

        match &saved {
            Some(file) => info!("file saved: {} {} bytes", file.name, bytes.len()),
            None => debug!("save dialog closed with no file"),
        }
        saved
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{filter_name, suggested_name, with_extension};
    use crate::filesystem::picker::clean_extensions;

    fn cleaned(extensions: &[&str]) -> Vec<String> {
        clean_extensions(extensions)
    }

    #[test]
    fn the_suggested_name_gets_an_extension() {
        let stl = cleaned(&["stl", "3mf"]);
        assert_eq!(suggested_name("part", &stl), "part.stl");
        assert_eq!(suggested_name("part.3MF", &stl), "part.3MF");
        assert_eq!(suggested_name("part.v2", &stl), "part.v2.stl");
        assert_eq!(suggested_name("part.scad", &cleaned(&[])), "part.scad");
    }

    #[test]
    fn the_suggested_name_has_no_folder_and_is_never_empty() {
        let scad = cleaned(&[".SCAD"]);
        assert_eq!(suggested_name("models/box/part.scad", &scad), "part.scad");
        assert_eq!(suggested_name(r"C:\models\part", &scad), "part.scad");
        assert_eq!(suggested_name("  ", &scad), "Untitled.scad");
        assert_eq!(suggested_name("models/", &cleaned(&[])), "Untitled");
    }

    #[test]
    fn a_typed_name_with_no_extension_gets_the_first() {
        let stl = cleaned(&["stl", "3mf"]);
        assert_eq!(
            with_extension(PathBuf::from("/home/me/part"), &stl),
            PathBuf::from("/home/me/part.stl")
        );
        assert_eq!(
            with_extension(PathBuf::from("/home/me/part.3mf"), &stl),
            PathBuf::from("/home/me/part.3mf")
        );
        // Another extension the user typed stays.
        assert_eq!(
            with_extension(PathBuf::from("/home/me/part.obj"), &stl),
            PathBuf::from("/home/me/part.obj")
        );
        assert_eq!(
            with_extension(PathBuf::from("/home/me/part"), &cleaned(&[])),
            PathBuf::from("/home/me/part")
        );
    }

    #[test]
    fn the_filter_is_named_by_its_extensions() {
        assert_eq!(filter_name(&cleaned(&["stl", ".3mf"])), "STL, 3MF");
    }
}
