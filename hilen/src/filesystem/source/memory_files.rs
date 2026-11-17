use std::{collections::BTreeMap, mem::take};

use anyhow::{Result, anyhow, bail};
use chrono::NaiveDateTime;
use parking_lot::Mutex;

use super::{Changed, FileEntry, FilePath, FilePlace, FileSource, Listed, PlaceKind};

#[derive(Default)]
struct State {
    /// Every folder with its entries. A folder with no entry is here too,
    /// so a listing tells an empty folder from a missing one.
    folders:   BTreeMap<FilePath, Vec<FileEntry>>,
    places:    Vec<FilePlace>,
    writable:  bool,
    /// Answers kept back until `answer_held`, for a view that has to show
    /// its wait.
    holding:   bool,
    held:      Vec<Box<dyn FnOnce() + Send>>,
    /// The error the next listing gives instead of its entries.
    fail_next: Option<String>,
}

/// A tree of folders and files held in memory, as a [`FileSource`]. For a
/// test, a demo, or anything read up front, the index of an archive.
///
/// Built with a chain: `MemoryFiles::new("Disk").folder("Photos")
/// .file("Photos/cat.png", 1200)`. Paths in the chain are relative to the
/// root, with `/` between the names, and the folders above a path are made
/// with it.
pub struct MemoryFiles {
    root:  FilePath,
    state: Mutex<State>,
}

impl MemoryFiles {
    /// One root with this name, which is also the first place.
    pub fn new(root: impl ToString) -> Self {
        let root = FilePath::root(root);
        let state = State {
            folders: BTreeMap::from([(root.clone(), vec![])]),
            places: vec![FilePlace::new(root.name(), root.clone()).kind(PlaceKind::Drive)],
            ..State::default()
        };
        Self {
            root,
            state: Mutex::new(state),
        }
    }

    pub fn root(&self) -> FilePath {
        self.root.clone()
    }

    /// The path of a `/` separated text under the root.
    pub fn path(&self, relative: &str) -> FilePath {
        let names = relative.split('/').filter(|name| !name.is_empty());
        FilePath::new(self.root.parts().iter().map(String::as_str).chain(names))
    }

    #[must_use]
    pub fn folder(self, relative: &str) -> Self {
        let path = self.path(relative);
        self.state.lock().make_folders(&path);
        self
    }

    #[must_use]
    pub fn file(self, relative: &str, size: u64) -> Self {
        let path = self.path(relative);
        self.state.lock().put(&path, FileEntry::file(path.name(), size));
        self
    }

    /// Sets the changed time of an entry made before in the chain.
    #[must_use]
    pub fn modified(self, relative: &str, at: NaiveDateTime) -> Self {
        self.edit(relative, |entry| entry.modified = Some(at))
    }

    /// Marks an entry made before in the chain as hidden.
    #[must_use]
    pub fn hidden(self, relative: &str) -> Self {
        self.edit(relative, |entry| entry.hidden = true)
    }

    /// Another row for the sidebar, after the root.
    #[must_use]
    pub fn place(self, title: &str, relative: &str, kind: PlaceKind) -> Self {
        let place = FilePlace::new(title, self.path(relative)).kind(kind);
        self.state.lock().places.push(place);
        self
    }

    /// With new folder, rename and delete.
    #[must_use]
    pub fn writable(self) -> Self {
        self.state.lock().writable = true;
        self
    }

    /// While on, every answer waits for `answer_held`.
    pub fn set_holding(&self, holding: bool) {
        self.state.lock().holding = holding;
    }

    /// Gives every answer kept back so far, in the order they were asked.
    pub fn answer_held(&self) {
        // The lock is gone before an answer runs, an answer may list again.
        let held = take(&mut self.state.lock().held);
        for answer in held {
            answer();
        }
    }

    /// The next listing fails with this text.
    pub fn fail_next_listing(&self, error: impl ToString) {
        self.state.lock().fail_next = Some(error.to_string());
    }

    fn edit(self, relative: &str, change: impl FnOnce(&mut FileEntry)) -> Self {
        let path = self.path(relative);
        match self.state.lock().entry_mut(&path) {
            Some(entry) => change(entry),
            None => log::error!("MemoryFiles has no entry {relative}"),
        }
        self
    }

    fn answer(&self, answer: impl FnOnce() + Send + 'static) {
        let mut state = self.state.lock();
        if state.holding {
            state.held.push(Box::new(answer));
        } else {
            drop(state);
            answer();
        }
    }
}

impl State {
    fn make_folders(&mut self, path: &FilePath) {
        for len in 2..=path.parts().len() {
            let folder = path.prefix(len);
            if !self.folders.contains_key(&folder) {
                self.put(&folder, FileEntry::folder(folder.name()));
                self.folders.insert(folder, vec![]);
            }
        }
    }

    /// Adds the entry to its parent, the folders above made first.
    fn put(&mut self, path: &FilePath, entry: FileEntry) {
        let Some(parent) = path.parent() else {
            return;
        };
        self.make_folders(&parent);
        let entries = self.folders.entry(parent).or_default();
        entries.retain(|old| old.name != entry.name);
        entries.push(entry);
    }

    fn entry_mut(&mut self, path: &FilePath) -> Option<&mut FileEntry> {
        let entries = self.folders.get_mut(&path.parent()?)?;
        entries.iter_mut().find(|entry| entry.name == path.name())
    }

    fn check_new_name(&self, parent: &FilePath, name: &str) -> Result<()> {
        if name.is_empty() || name.contains('/') {
            bail!("\"{name}\" is not a file name");
        }
        let Some(entries) = self.folders.get(parent) else {
            bail!("No such folder");
        };
        if entries.iter().any(|entry| entry.name == name) {
            bail!("{name} already exists");
        }
        Ok(())
    }

    fn check_writable(&self) -> Result<()> {
        if !self.writable {
            bail!("This file source is read only");
        }
        Ok(())
    }

    fn make_folder(&mut self, parent: &FilePath, name: &str) -> Result<()> {
        self.check_writable()?;
        self.check_new_name(parent, name)?;
        self.make_folders(&parent.join(name));
        Ok(())
    }

    fn rename(&mut self, path: &FilePath, name: &str) -> Result<()> {
        self.check_writable()?;
        let Some(parent) = path.parent() else {
            bail!("A root cannot be renamed");
        };
        self.check_new_name(&parent, name)?;
        let Some(entry) = self.entry_mut(path) else {
            bail!("No such entry");
        };
        entry.name = name.to_string();

        // The folder and everything under it moves to the new name.
        let depth = path.parts().len();
        let moved: Vec<FilePath> =
            self.folders.keys().filter(|key| key.prefix(depth) == *path).cloned().collect();
        for old in moved {
            let mut parts = old.parts().to_vec();
            parts[depth - 1] = name.to_string();
            if let Some(entries) = self.folders.remove(&old) {
                self.folders.insert(FilePath::new(parts), entries);
            }
        }
        Ok(())
    }

    fn delete(&mut self, path: &FilePath) -> Result<()> {
        self.check_writable()?;
        let Some(parent) = path.parent() else {
            bail!("A root cannot be deleted");
        };
        let Some(entries) = self.folders.get_mut(&parent) else {
            bail!("No such folder");
        };
        let before = entries.len();
        entries.retain(|entry| entry.name != path.name());
        if entries.len() == before {
            bail!("No such entry");
        }
        let depth = path.parts().len();
        self.folders.retain(|key, _| key.prefix(depth) != *path);
        Ok(())
    }
}

impl FileSource for MemoryFiles {
    fn places(&self) -> Vec<FilePlace> {
        self.state.lock().places.clone()
    }

    fn list(&self, path: &FilePath, done: Listed) {
        let result = {
            let mut state = self.state.lock();
            match state.fail_next.take() {
                Some(error) => Err(anyhow!(error)),
                None => state
                    .folders
                    .get(path)
                    .cloned()
                    .ok_or_else(|| anyhow!("No such folder: {}", path.slash_text())),
            }
        };
        self.answer(move || done(result));
    }

    fn can_write(&self) -> bool {
        self.state.lock().writable
    }

    fn make_folder(&self, parent: &FilePath, name: &str, done: Changed) {
        let result = self.state.lock().make_folder(parent, name);
        self.answer(move || done(result));
    }

    fn rename(&self, path: &FilePath, new_name: &str, done: Changed) {
        let result = self.state.lock().rename(path, new_name);
        self.answer(move || done(result));
    }

    fn delete(&self, path: &FilePath, done: Changed) {
        let result = self.state.lock().delete(path);
        self.answer(move || done(result));
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, Mutex};

    use anyhow::Result;

    use super::MemoryFiles;
    use crate::filesystem::{FileEntry, FilePath, FileSource};

    fn list(source: &MemoryFiles, path: &FilePath) -> Result<Vec<String>> {
        let out = Arc::new(Mutex::new(None));
        let slot = out.clone();
        source.list(path, Box::new(move |result| *slot.lock().unwrap() = Some(result)));
        let result: Result<Vec<FileEntry>> = out.lock().unwrap().take().expect("no answer yet");
        Ok(result?.into_iter().map(|entry| entry.name).collect())
    }

    fn done(run: impl FnOnce(super::Changed)) -> Result<()> {
        let out = Arc::new(Mutex::new(None));
        let slot = out.clone();
        run(Box::new(move |result| *slot.lock().unwrap() = Some(result)));
        out.lock().unwrap().take().expect("no answer yet")
    }

    fn tree() -> MemoryFiles {
        MemoryFiles::new("Disk")
            .folder("Empty")
            .file("Photos/Trip/cat.png", 10)
            .file("note.txt", 5)
    }

    #[test]
    fn a_file_makes_the_folders_above_it() {
        let source = tree();
        assert_eq!(
            list(&source, &source.root()).unwrap(),
            ["Empty", "Photos", "note.txt"]
        );
        assert_eq!(list(&source, &source.path("Photos")).unwrap(), ["Trip"]);
        assert_eq!(list(&source, &source.path("Photos/Trip")).unwrap(), ["cat.png"]);
        assert_eq!(
            list(&source, &source.path("Empty")).unwrap(),
            Vec::<String>::new()
        );
        assert!(list(&source, &source.path("Nope")).is_err());
    }

    #[test]
    fn a_read_only_tree_refuses_changes() {
        let source = tree();
        assert!(done(|answer| source.make_folder(&source.root(), "New", answer)).is_err());
    }

    #[test]
    fn rename_moves_everything_under_a_folder() {
        let source = tree().writable();
        done(|answer| source.rename(&source.path("Photos"), "Pictures", answer)).unwrap();
        assert_eq!(list(&source, &source.path("Pictures/Trip")).unwrap(), ["cat.png"]);
        assert!(list(&source, &source.path("Photos")).is_err());
        assert!(done(|answer| source.rename(&source.path("Pictures"), "Empty", answer)).is_err());
    }

    #[test]
    fn delete_takes_the_folder_and_what_is_in_it() {
        let source = tree().writable();
        done(|answer| source.delete(&source.path("Photos"), answer)).unwrap();
        assert_eq!(list(&source, &source.root()).unwrap(), ["Empty", "note.txt"]);
        assert!(list(&source, &source.path("Photos/Trip")).is_err());
    }

    #[test]
    fn held_answers_wait_and_a_failure_is_used_once() {
        let source = tree();
        source.set_holding(true);
        let out = Arc::new(Mutex::new(vec![]));
        let slot = out.clone();
        source.list(
            &source.root(),
            Box::new(move |result| slot.lock().unwrap().push(result.is_ok())),
        );
        assert!(out.lock().unwrap().is_empty());
        source.answer_held();
        assert_eq!(*out.lock().unwrap(), [true]);

        source.set_holding(false);
        source.fail_next_listing("offline");
        assert!(list(&source, &source.root()).is_err());
        assert!(list(&source, &source.root()).is_ok());
    }
}
