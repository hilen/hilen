//! The clipboard of a headless run, one text kept inside the process.

use anyhow::{Result, anyhow};
use zeroize::Zeroizing;

/// Wipes the text it held when another one replaces it or it is cleared,
/// a headless run may copy a secret like any other run.
pub(super) struct HeadlessStore(Option<Zeroizing<String>>);

impl HeadlessStore {
    pub(super) const fn new() -> Self {
        Self(None)
    }

    pub(super) fn set(&mut self, text: &str) {
        self.0 = Some(Zeroizing::new(text.to_owned()));
    }

    pub(super) fn get(&self) -> Result<String> {
        self.0
            .as_ref()
            .map(|text| text.as_str().to_owned())
            .ok_or_else(|| anyhow!("The clipboard holds no text"))
    }

    /// Clears the store when it holds `text`, and says whether it did.
    pub(super) fn clear_if_holds(&mut self, text: &str) -> bool {
        let holds = self.0.as_ref().is_some_and(|held| held.as_str() == text);
        if holds {
            self.0 = None;
        }
        holds
    }
}

#[cfg(test)]
mod tests {
    use super::HeadlessStore;

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
}
