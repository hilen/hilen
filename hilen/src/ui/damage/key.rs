use std::hash::{Hash, Hasher};

use bytemuck::{Pod, bytes_of};
use rustc_hash::FxHasher;

/// Everything a view puts on the screen, folded into 1 number. 2 frames
/// with the same key of a view have the same pixels of it.
#[derive(Default)]
pub(crate) struct PaintKey {
    hasher: FxHasher,
}

impl PaintKey {
    /// A float or a plain struct of floats, by its bytes.
    pub(crate) fn pod<T: Pod>(&mut self, value: &T) -> &mut Self {
        self.hasher.write(bytes_of(value));
        self
    }

    pub(crate) fn hash<T: Hash + ?Sized>(&mut self, value: &T) -> &mut Self {
        value.hash(&mut self.hasher);
        self
    }

    /// A value with no bytes and no hash of its own, by its debug text.
    /// Only for what few views have, it allocates.
    pub(crate) fn debug(&mut self, value: &impl std::fmt::Debug) -> &mut Self {
        self.hash(&format!("{value:?}"))
    }

    pub(crate) fn finish(&self) -> u64 {
        self.hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::PaintKey;
    use crate::gm::color::{Color, RED};

    fn key(feed: impl FnOnce(&mut PaintKey)) -> u64 {
        let mut key = PaintKey::default();
        feed(&mut key);
        key.finish()
    }

    #[test]
    fn the_same_values_give_the_same_key() {
        assert_eq!(
            key(|key| {
                key.pod(&RED).hash("text");
            }),
            key(|key| {
                key.pod(&RED).hash("text");
            })
        );
    }

    #[test]
    fn a_changed_value_gives_another_key() {
        let faded: Color = RED.with_alpha(0.5);
        assert_ne!(
            key(|key| {
                key.pod(&RED);
            }),
            key(|key| {
                key.pod(&faded);
            })
        );
        assert_ne!(
            key(|key| {
                key.hash("a");
            }),
            key(|key| {
                key.hash("b");
            })
        );
    }
}
