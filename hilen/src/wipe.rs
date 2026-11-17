//! Text that must not stay in memory after use, like a password.
//!
//! Freeing a `String` leaves its bytes in the freed block, and a `String`
//! that grows moves to a bigger block and frees the old one the same way.
//! So a secret is never built by growing. `joined` makes the whole text in
//! one block of the exact size, and the `Zeroizing` around it writes zeros
//! over the block before it is freed.

use zeroize::Zeroizing;

/// The parts one after another, in one block of the exact size.
pub(crate) fn joined(parts: &[&str]) -> Zeroizing<String> {
    let mut text = Zeroizing::new(String::with_capacity(parts.iter().map(|part| part.len()).sum()));
    for part in parts {
        text.push_str(part);
    }
    text
}

/// The chars of `text` that `keep` accepts, in a block that never grows.
pub(crate) fn filtered(text: &str, keep: impl Fn(char) -> bool) -> Zeroizing<String> {
    let mut kept = Zeroizing::new(String::with_capacity(text.len()));
    for char in text.chars().filter(|char| keep(*char)) {
        kept.push(char);
    }
    kept
}

/// Tells a test whether a heap block was all zeros at the moment it was
/// freed. Freed memory must not be read, so the check sits in the
/// allocator, the last place where the block is still valid.
///
/// `realloc` is left to the default of `GlobalAlloc`, which allocates a new
/// block, copies and frees the old one through `dealloc`. So a secret that
/// grows in place of being rebuilt always shows up as a block freed with
/// its content, while the system allocator would hide it whenever the
/// block happens to grow where it is.
#[cfg(all(test, not_wasm))]
pub(crate) mod probe {
    use std::{
        alloc::{GlobalAlloc, Layout, System},
        cell::Cell,
        slice::from_raw_parts,
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum Freed {
        NotYet,
        Wiped,
        WithContent,
    }

    const SLOTS: usize = 8;

    thread_local! {
        /// The blocks this thread watches, by address, 0 for a free slot.
        /// No destructor and a const start, so the allocator can read it
        /// at any point of the life of a thread.
        static WATCHED: Cell<[(usize, Freed); SLOTS]> = const { Cell::new([(0, Freed::NotYet); SLOTS]) };
    }

    #[global_allocator]
    static PROBE: Probe = Probe;

    struct Probe;

    unsafe impl GlobalAlloc for Probe {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            unsafe { System.alloc(layout) }
        }

        unsafe fn dealloc(&self, block: *mut u8, layout: Layout) {
            note_free(block, layout.size());
            unsafe { System.dealloc(block, layout) }
        }
    }

    fn note_free(block: *mut u8, size: usize) {
        WATCHED.with(|watched| {
            let mut slots = watched.get();
            let Some(slot) = slots.iter_mut().find(|slot| slot.0 == block.addr() && slot.1 == Freed::NotYet)
            else {
                return;
            };
            // A watched block is the buffer of a `String` a test filled,
            // every byte of it was written.
            let bytes = unsafe { from_raw_parts(block, size) };
            slot.1 = if bytes.iter().all(|byte| *byte == 0) {
                Freed::Wiped
            } else {
                Freed::WithContent
            };
            watched.set(slots);
        });
    }

    /// A watch on the block that holds `text`. Ask it once the owner of
    /// the text dropped or replaced it, on the same thread.
    pub(crate) struct Watch(usize);

    impl Watch {
        pub(crate) fn on(text: &str) -> Self {
            assert!(!text.is_empty(), "an empty text has no block to watch");
            let address = text.as_ptr().addr();
            WATCHED.with(|watched| {
                let mut slots = watched.get();
                let slot = slots
                    .iter_mut()
                    .find(|slot| slot.0 == 0)
                    .expect("too many blocks watched at once");
                *slot = (address, Freed::NotYet);
                watched.set(slots);
            });
            Self(address)
        }

        pub(crate) fn freed(&self) -> Freed {
            WATCHED.with(|watched| {
                watched
                    .get()
                    .iter()
                    .find(|slot| slot.0 == self.0)
                    .map_or(Freed::NotYet, |slot| slot.1)
            })
        }
    }

    impl Drop for Watch {
        fn drop(&mut self) {
            WATCHED.with(|watched| {
                let mut slots = watched.get();
                for slot in slots.iter_mut().filter(|slot| slot.0 == self.0) {
                    *slot = (0, Freed::NotYet);
                }
                watched.set(slots);
            });
        }
    }
}

#[cfg(all(test, not_wasm))]
mod tests {
    use zeroize::Zeroizing;

    use super::{
        filtered, joined,
        probe::{Freed, Watch},
    };

    #[test]
    fn the_probe_tells_a_wiped_block_from_one_freed_with_its_text() {
        let plain = "a plain text of some length".to_owned();
        let plain_watch = Watch::on(&plain);
        let wiped = Zeroizing::new("a secret of the same length".to_owned());
        let wiped_watch = Watch::on(&wiped);

        assert_eq!(plain_watch.freed(), Freed::NotYet);
        assert_eq!(wiped_watch.freed(), Freed::NotYet);

        drop(plain);
        drop(wiped);

        assert_eq!(plain_watch.freed(), Freed::WithContent);
        assert_eq!(wiped_watch.freed(), Freed::Wiped);
    }

    #[test]
    fn the_probe_catches_a_secret_that_grew() {
        // The mistake `joined` exists to prevent: the wrapper wipes only the
        // last block, the one the text moved out of is freed as it is.
        let mut grown = Zeroizing::new("first part".to_owned());
        let first_block = Watch::on(&grown);

        grown.push_str(", and a second part that needs a bigger block");

        assert_eq!(first_block.freed(), Freed::WithContent);
    }

    #[test]
    fn joined_text_is_built_in_one_block_and_wiped_when_dropped() {
        let text = joined(&["correct horse ", "battery", " staple"]);
        let watch = Watch::on(&text);

        assert_eq!(text.as_str(), "correct horse battery staple");
        assert_eq!(text.capacity(), text.len());

        drop(text);

        assert_eq!(watch.freed(), Freed::Wiped);
    }

    // The steps of a secure text field on every key: the new text is built
    // next to the old one with `joined`, then takes its place.
    #[test]
    fn every_buffer_of_a_text_typed_key_by_key_is_wiped() {
        let mut buffer = Zeroizing::new(String::new());

        for (typed, key) in ["h", "u", "n", "t", "e", "r", "2"].into_iter().enumerate() {
            let edited = joined(&[buffer.as_str(), key]);
            let old = (typed > 0).then(|| Watch::on(&buffer));

            buffer = edited;

            if let Some(old) = old {
                assert_eq!(old.freed(), Freed::Wiped, "the buffer before key {typed}");
            }
        }

        assert_eq!(buffer.as_str(), "hunter2");
        let last = Watch::on(&buffer);
        drop(buffer);

        assert_eq!(last.freed(), Freed::Wiped);
    }

    #[test]
    fn filtered_text_keeps_the_accepted_chars_and_is_wiped_when_dropped() {
        let text = filtered("12a3-4b", |char| char.is_ascii_digit());
        let watch = Watch::on(&text);

        assert_eq!(text.as_str(), "1234");
        assert_eq!(text.capacity(), "12a3-4b".len());

        drop(text);

        assert_eq!(watch.freed(), Freed::Wiped);
    }
}
