//! Access to OS services that are not part of the window: the clipboard
//! and links.

#[cfg(android)]
mod android_jni;
mod clipboard;
mod open_url;

pub use clipboard::Clipboard;
pub use open_url::open_url;
