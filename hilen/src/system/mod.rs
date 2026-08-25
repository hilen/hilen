//! Access to OS services that are not part of the window: the clipboard,
//! links and browser history.

#[cfg(android)]
mod android_jni;
mod clipboard;
mod open_url;
mod router;
mod updater;

pub use clipboard::Clipboard;
pub use open_url::open_url;
pub use router::Router;
#[cfg(wasm)]
pub(crate) use router::install_popstate_listener;
pub use updater::{UpdateArtifact, UpdateInfo, UpdateManifest, UpdateSource, Updater};
