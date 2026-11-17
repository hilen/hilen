use std::{io, path::Path};

#[cfg(android)]
static ANDROID_APP: std::sync::OnceLock<crate::AndroidApp> = std::sync::OnceLock::new();

/// Handed over from the android start so asset reads can reach the
/// `AAssetManager` before the event loop consumes the `AndroidApp`.
#[cfg(android)]
pub(crate) fn set_android_app(app: crate::AndroidApp) {
    assert!(ANDROID_APP.set(app).is_ok(), "Double setting of AndroidApp");
}

/// The private files folder of the app, none before the android start
/// handed the `AndroidApp` over.
#[cfg(android)]
pub(crate) fn android_data_path() -> Option<std::path::PathBuf> {
    ANDROID_APP.get()?.internal_data_path()
}

/// Android assets live inside the APK, not on the filesystem, so reads go
/// through the `AAssetManager`.
#[cfg(android)]
pub(crate) fn read_bytes(path: &Path) -> io::Result<Vec<u8>> {
    use std::{ffi::CString, io::Read};

    let app = ANDROID_APP.get().ok_or_else(|| io::Error::other("AndroidApp is not set"))?;
    let name = CString::new(path.to_string_lossy().as_bytes()).map_err(io::Error::other)?;
    let mut asset = app.asset_manager().open(&name).ok_or(io::ErrorKind::NotFound)?;
    let mut data = Vec::new();
    asset.read_to_end(&mut data)?;
    Ok(data)
}

/// A file on disk wins. An asset that is not there is read from the copy
/// the app packed into its binary, the normal case of an installed
/// desktop app, see `embedded`.
#[cfg(not_android)]
pub(crate) fn read_bytes(path: &Path) -> io::Result<Vec<u8>> {
    let read = std::fs::read(path);
    #[cfg(desktop)]
    if let Err(err) = &read
        && err.kind() == io::ErrorKind::NotFound
        && let Some(assets) = crate::assets::Assets::started_path()
        && let Some(packed) = crate::embedded::read(&assets, path)
    {
        return Ok(packed.to_vec());
    }
    read
}
