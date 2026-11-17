//! The clipboard of a headless run, one text or one picture kept inside
//! the process.

use anyhow::{Result, anyhow};
use zeroize::Zeroizing;

use super::image::Pixels;

enum Held {
    Nothing,
    Text(Zeroizing<String>),
    Picture(Pixels),
}

/// Wipes the text it held when another content replaces it or it is
/// cleared, a headless run may copy a secret like any other run.
pub(super) struct HeadlessStore(Held);

impl HeadlessStore {
    pub(super) const fn new() -> Self {
        Self(Held::Nothing)
    }

    pub(super) fn set(&mut self, text: &str) {
        self.0 = Held::Text(Zeroizing::new(text.to_owned()));
    }

    pub(super) fn get(&self) -> Result<String> {
        match &self.0 {
            Held::Text(text) => Ok(text.as_str().to_owned()),
            Held::Nothing | Held::Picture(_) => Err(anyhow!("The clipboard holds no text")),
        }
    }

    /// A picture takes the place of a text, like a copy in the system.
    pub(super) fn set_picture(&mut self, picture: Pixels) {
        self.0 = Held::Picture(picture);
    }

    pub(super) fn picture(&self) -> Option<Pixels> {
        match &self.0 {
            Held::Picture(picture) => Some(picture.clone()),
            Held::Nothing | Held::Text(_) => None,
        }
    }

    /// Clears the store when it holds `text`, and says whether it did.
    pub(super) fn clear_if_holds(&mut self, text: &str) -> bool {
        let holds = matches!(&self.0, Held::Text(held) if held.as_str() == text);
        if holds {
            self.0 = Held::Nothing;
        }
        holds
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::HeadlessStore;
    use crate::system::clipboard::image::{ClipboardImage, Pixels};

    /// 2 by 1 pixels, a red one and a half transparent blue one.
    fn picture() -> Pixels {
        Pixels {
            rgba:   vec![255, 0, 0, 255, 0, 0, 255, 128],
            width:  2,
            height: 1,
        }
    }

    #[test]
    fn a_text_the_store_holds_is_cleared() {
        let mut store = HeadlessStore::new();
        store.set("twelve secret words");

        assert!(store.clear_if_holds("twelve secret words"));
        assert!(store.get().is_err());
    }

    #[test]
    fn a_text_copied_since_stays() {
        let mut store = HeadlessStore::new();
        store.set("twelve secret words");
        store.set("what the user copied next");

        assert!(!store.clear_if_holds("twelve secret words"));
        assert_eq!(store.get().ok().as_deref(), Some("what the user copied next"));
    }

    #[test]
    fn an_empty_store_clears_nothing() {
        let mut store = HeadlessStore::new();

        assert!(!store.clear_if_holds("twelve secret words"));
        assert!(store.get().is_err());
    }

    #[test]
    fn a_picture_comes_back_as_a_png_with_the_same_pixels() -> Result<()> {
        let mut store = HeadlessStore::new();
        let copied = ClipboardImage::encode(&picture())?;

        // The way of `Clipboard::set_image` and `Clipboard::get_image`.
        store.set_picture(copied.decode()?);
        let held = store.picture().ok_or_else(|| anyhow::anyhow!("The store holds no picture"))?;
        let pasted = ClipboardImage::encode(&held)?;

        assert_eq!(pasted, copied);
        assert_eq!((pasted.width, pasted.height), (2, 1));
        assert_eq!(pasted.decode()?, picture());
        Ok(())
    }

    #[test]
    fn an_empty_store_holds_no_picture() {
        assert!(HeadlessStore::new().picture().is_none());
    }

    #[test]
    fn a_picture_takes_the_place_of_a_text() {
        let mut store = HeadlessStore::new();
        store.set("some text");
        store.set_picture(picture());

        assert!(store.get().is_err());
        assert_eq!(store.picture(), Some(picture()));
        assert!(!store.clear_if_holds("some text"));
        assert_eq!(store.picture(), Some(picture()));
    }

    #[test]
    fn a_text_takes_the_place_of_a_picture() {
        let mut store = HeadlessStore::new();
        store.set_picture(picture());
        store.set("some text");

        assert!(store.picture().is_none());
        assert_eq!(store.get().ok().as_deref(), Some("some text"));
    }
}
