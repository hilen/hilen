use anyhow::{Result, bail};

use super::{FileEntry, FilePath};

/// The answer of a listing. Call it once, from any thread.
pub type Listed = Box<dyn FnOnce(Result<Vec<FileEntry>>) + Send>;

/// The answer of a change. Call it once, from any thread.
pub type Changed = Box<dyn FnOnce(Result<()>) + Send>;

/// What a place in the sidebar of a file browser is, it picks the icon.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlaceKind {
    Home,
    /// A root of the source, a disk or a drive.
    Drive,
    #[default]
    Folder,
}

/// A row of the sidebar of a file browser.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilePlace {
    pub title: String,
    pub path:  FilePath,
    pub kind:  PlaceKind,
}

impl FilePlace {
    pub fn new(title: impl ToString, path: FilePath) -> Self {
        Self {
            title: title.to_string(),
            path,
            kind: PlaceKind::Folder,
        }
    }

    #[must_use]
    pub fn kind(mut self, kind: PlaceKind) -> Self {
        self.kind = kind;
        self
    }
}

/// Where a `FileBrowser` gets its folders from. The engine brings
/// `LocalFiles` for the disk of this machine and `MemoryFiles` for a tree
/// held in memory. An app writes its own for another machine, an archive
/// or a cloud drive.
///
/// A listing and a change answer through a callback, so a source may ask
/// the network and may fail. The browser shows the wait and the error.
pub trait FileSource: Send + Sync + 'static {
    /// The rows of the sidebar: the home folder, the roots or drives and
    /// whatever else the source knows. The first one is where a browser
    /// starts.
    fn places(&self) -> Vec<FilePlace>;

    /// The entries of a folder, in any order, hidden ones included.
    fn list(&self, path: &FilePath, done: Listed);

    /// The path as the user reads and types it.
    fn path_text(&self, path: &FilePath) -> String {
        path.slash_text()
    }

    /// The path of a typed or pasted text, `None` when it is not one.
    fn parse_path(&self, text: &str) -> Option<FilePath> {
        FilePath::from_slash_text(text)
    }

    /// Whether the browser offers new folder, rename and delete.
    fn can_write(&self) -> bool {
        false
    }

    fn make_folder(&self, _parent: &FilePath, _name: &str, done: Changed) {
        done(read_only());
    }

    fn rename(&self, _path: &FilePath, _new_name: &str, done: Changed) {
        done(read_only());
    }

    /// Removes a file, or a folder with everything in it.
    fn delete(&self, _path: &FilePath, done: Changed) {
        done(read_only());
    }
}

fn read_only() -> Result<()> {
    bail!("This file source is read only")
}
