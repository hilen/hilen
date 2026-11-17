use std::{cmp::Ordering, collections::BTreeSet};

use crate::filesystem::FileEntry;

/// The column a file browser sorts by.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortKey {
    #[default]
    Name,
    Size,
    Kind,
    Modified,
}

/// The order of the entries. Folders always come before files, the key
/// orders each of the 2 groups, and the name breaks a tie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileSort {
    pub key:       SortKey,
    pub ascending: bool,
}

impl Default for FileSort {
    fn default() -> Self {
        Self {
            key:       SortKey::Name,
            ascending: true,
        }
    }
}

impl FileSort {
    /// The order after a tap on a column header: the same column turns
    /// around, another column starts ascending.
    #[must_use]
    pub fn tapped(self, key: SortKey) -> Self {
        if self.key == key {
            Self {
                key,
                ascending: !self.ascending,
            }
        } else {
            Self { key, ascending: true }
        }
    }

    pub(super) fn sort(self, entries: &mut [FileEntry]) {
        entries.sort_by(|a, b| {
            let folders_first = b.is_folder().cmp(&a.is_folder());
            let by_key = match self.key {
                SortKey::Name => Ordering::Equal,
                SortKey::Size => a.size.cmp(&b.size),
                SortKey::Kind => a.kind_text().cmp(&b.kind_text()),
                SortKey::Modified => a.modified.cmp(&b.modified),
            };
            let by_name = a
                .name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then_with(|| a.name.cmp(&b.name));
            let ordered = by_key.then(by_name);
            folders_first.then(if self.ascending {
                ordered
            } else {
                ordered.reverse()
            })
        });
    }
}

/// What of a folder is shown.
#[derive(Clone, Debug, Default)]
pub(super) struct Filter {
    /// Lower case. An entry stays when its name contains it.
    pub search:      String,
    pub show_hidden: bool,
    /// Lower case, no dot. Empty lets every file through. A folder always
    /// stays, the wanted file may be inside it.
    pub extensions:  Vec<String>,
}

impl Filter {
    fn keeps(&self, entry: &FileEntry) -> bool {
        if entry.hidden && !self.show_hidden {
            return false;
        }
        if !self.search.is_empty() && !entry.name.to_lowercase().contains(&self.search) {
            return false;
        }
        entry.is_folder() || self.extensions.is_empty() || self.extensions.contains(&entry.extension())
    }

    pub(super) fn apply(&self, all: &[FileEntry], sort: FileSort) -> Vec<FileEntry> {
        let mut shown: Vec<FileEntry> = all.iter().filter(|entry| self.keeps(entry)).cloned().collect();
        sort.sort(&mut shown);
        shown
    }
}

/// The picked rows by index. The anchor is where a Shift range starts,
/// the cursor is the row the arrow keys move from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Selection {
    picked: BTreeSet<usize>,
    anchor: Option<usize>,
    cursor: Option<usize>,
}

impl Selection {
    pub(super) fn contains(&self, index: usize) -> bool {
        self.picked.contains(&index)
    }

    pub(super) fn indices(&self) -> Vec<usize> {
        self.picked.iter().copied().collect()
    }

    pub(super) fn cursor(&self) -> Option<usize> {
        self.cursor
    }

    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    /// A plain tap: only this row.
    pub(super) fn only(&mut self, index: usize) {
        self.picked = BTreeSet::from([index]);
        self.anchor = Some(index);
        self.cursor = Some(index);
    }

    /// A tap with Cmd or Ctrl: this row joins or leaves.
    pub(super) fn toggle(&mut self, index: usize) {
        if !self.picked.remove(&index) {
            self.picked.insert(index);
        }
        self.anchor = Some(index);
        self.cursor = Some(index);
    }

    /// A tap with Shift: every row from the anchor to this one, in place
    /// of what was picked.
    pub(super) fn extend_to(&mut self, index: usize) {
        let anchor = self.anchor.unwrap_or(index);
        self.picked = (anchor.min(index)..=anchor.max(index)).collect();
        self.anchor = Some(anchor);
        self.cursor = Some(index);
    }

    pub(super) fn select_all(&mut self, count: usize) {
        self.picked = (0..count).collect();
        self.anchor = (count > 0).then_some(0);
        self.cursor = count.checked_sub(1);
    }

    /// An arrow key. With no cursor yet a move down starts at the first
    /// row and a move up at the last. `extend` is Shift held. Returns the
    /// new cursor, `None` when there is no row.
    pub(super) fn step(&mut self, delta: isize, count: usize, extend: bool) -> Option<usize> {
        if count == 0 {
            return None;
        }
        let last = count - 1;
        let target = match self.cursor {
            Some(cursor) => cursor.saturating_add_signed(delta).min(last),
            None if delta >= 0 => 0,
            None => last,
        };
        if extend {
            self.extend_to(target);
        } else {
            self.only(target);
        }
        Some(target)
    }

    /// Drops the rows the check refuses, a file while only a folder can
    /// be picked.
    pub(super) fn retain(&mut self, mut keep: impl FnMut(usize) -> bool) {
        self.picked.retain(|index| keep(*index));
    }

    /// The same entries picked again after the rows moved, each found by
    /// what `old` says it was and where `new` says it is now.
    pub(super) fn remap(&mut self, new_index: impl Fn(usize) -> Option<usize>) {
        self.picked = self.picked.iter().filter_map(|index| new_index(*index)).collect();
        self.anchor = self.anchor.and_then(&new_index);
        self.cursor = self.cursor.and_then(&new_index);
    }
}

#[cfg(test)]
mod test {
    use chrono::{NaiveDate, NaiveDateTime};

    use super::{FileSort, Filter, Selection, SortKey};
    use crate::filesystem::FileEntry;

    fn names(entries: &[FileEntry]) -> Vec<&str> {
        entries.iter().map(|entry| entry.name.as_str()).collect()
    }

    fn day(day: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 1, day).unwrap().and_hms_opt(0, 0, 0).unwrap()
    }

    fn entries() -> Vec<FileEntry> {
        vec![
            FileEntry::file("b.txt", 30).modified(day(3)),
            FileEntry::folder("Zoo").modified(day(1)),
            FileEntry::file("A.png", 10).modified(day(2)),
            FileEntry::folder("alpha").modified(day(5)),
            FileEntry::file(".env", 5).hidden(),
        ]
    }

    #[test]
    fn folders_come_first_in_both_directions() {
        let mut list = entries();
        FileSort::default().sort(&mut list);
        assert_eq!(names(&list), ["alpha", "Zoo", ".env", "A.png", "b.txt"]);

        FileSort {
            key:       SortKey::Name,
            ascending: false,
        }
        .sort(&mut list);
        assert_eq!(names(&list), ["Zoo", "alpha", "b.txt", "A.png", ".env"]);
    }

    #[test]
    fn size_and_date_order_the_files() {
        let mut list = entries();
        FileSort {
            key:       SortKey::Size,
            ascending: false,
        }
        .sort(&mut list);
        assert_eq!(names(&list), ["Zoo", "alpha", "b.txt", "A.png", ".env"]);

        FileSort {
            key:       SortKey::Modified,
            ascending: true,
        }
        .sort(&mut list);
        assert_eq!(names(&list), ["Zoo", "alpha", ".env", "A.png", "b.txt"]);
    }

    #[test]
    fn a_header_tap_turns_the_same_column_around() {
        let sort = FileSort::default();
        assert!(!sort.tapped(SortKey::Name).ascending);
        let by_size = sort.tapped(SortKey::Size);
        assert_eq!(by_size.key, SortKey::Size);
        assert!(by_size.ascending);
    }

    #[test]
    fn the_filter_hides_and_searches_and_keeps_folders() {
        let all = entries();
        let sort = FileSort::default();

        let plain = Filter::default().apply(&all, sort);
        assert_eq!(names(&plain), ["alpha", "Zoo", "A.png", "b.txt"]);

        let hidden = Filter {
            show_hidden: true,
            ..Filter::default()
        };
        assert_eq!(hidden.apply(&all, sort).len(), 5);

        let search = Filter {
            search: "a".to_string(),
            ..Filter::default()
        };
        assert_eq!(names(&search.apply(&all, sort)), ["alpha", "A.png"]);

        let images = Filter {
            extensions: vec!["png".to_string()],
            ..Filter::default()
        };
        assert_eq!(names(&images.apply(&all, sort)), ["alpha", "Zoo", "A.png"]);
    }

    #[test]
    fn taps_pick_one_toggle_and_range() {
        let mut selection = Selection::default();
        selection.only(2);
        assert_eq!(selection.indices(), [2]);

        selection.toggle(5);
        assert_eq!(selection.indices(), [2, 5]);
        selection.toggle(2);
        assert_eq!(selection.indices(), [5]);

        // The range starts at the row touched last, also when that
        // touch took the row out, and replaces what was picked.
        selection.extend_to(3);
        assert_eq!(selection.indices(), [2, 3]);
        selection.extend_to(7);
        assert_eq!(selection.indices(), [2, 3, 4, 5, 6, 7]);
    }

    #[test]
    fn arrows_move_clamp_and_extend() {
        let mut selection = Selection::default();
        assert_eq!(selection.step(1, 0, false), None);
        assert_eq!(selection.step(1, 4, false), Some(0));
        assert_eq!(selection.step(-1, 4, false), Some(0));
        assert_eq!(selection.step(1, 4, true), Some(1));
        assert_eq!(selection.step(1, 4, true), Some(2));
        assert_eq!(selection.indices(), [0, 1, 2]);
        assert_eq!(selection.step(9, 4, false), Some(3));
        assert_eq!(selection.indices(), [3]);

        let mut from_bottom = Selection::default();
        assert_eq!(from_bottom.step(-1, 4, false), Some(3));
    }

    #[test]
    fn a_remap_follows_the_entries_to_their_new_rows() {
        let mut selection = Selection::default();
        selection.only(1);
        selection.toggle(3);
        // Row 1 moved to 0, row 3 is gone.
        selection.remap(|index| (index == 1).then_some(0));
        assert_eq!(selection.indices(), [0]);
        assert_eq!(selection.cursor(), None);
    }
}
