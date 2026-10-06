use std::{
    fs::{self, DirEntry, Metadata},
    path::{Component, Path, PathBuf},
    thread,
};

use anyhow::{Context, Result, anyhow, bail};

use super::{Changed, FileEntry, FilePath, FilePlace, FileSource, Listed, PlaceKind};
use crate::filesystem::Paths;

/// The disk of this machine as a [`FileSource`]. Read only unless made
/// with [`LocalFiles::writable`], a delete here is for good, nothing goes
/// to a trash.
///
/// Every listing and change runs on its own thread, a slow disk or a
/// network mount never holds a frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct LocalFiles {
    writable: bool,
}

impl LocalFiles {
    pub fn new() -> Self {
        Self::default()
    }

    /// With new folder, rename and delete.
    pub fn writable() -> Self {
        Self { writable: true }
    }

    /// The path on this disk. An empty path gives an empty `PathBuf`.
    pub fn path_buf(path: &FilePath) -> PathBuf {
        let mut buf = PathBuf::from(path.root_name());
        buf.extend(path.names());
        buf
    }

    /// The path of an absolute path of this disk, `None` for a relative
    /// one. The root is `/` on Unix and the drive with its slash,
    /// `C:\`, on Windows.
    pub fn file_path(path: &Path) -> Option<FilePath> {
        if !path.is_absolute() {
            return None;
        }

        let mut root = PathBuf::new();
        let mut names = vec![];

        for component in path.components() {
            match component {
                Component::Prefix(_) | Component::RootDir => root.push(component),
                Component::Normal(name) => names.push(name.to_string_lossy().into_owned()),
                Component::CurDir => (),
                Component::ParentDir => {
                    names.pop();
                }
            }
        }

        let root = root.to_string_lossy().into_owned();
        Some(FilePath::new([root].into_iter().chain(names)))
    }

    /// The roots of this disk: `/`, or every drive letter that answers.
    fn roots() -> Vec<FilePath> {
        if cfg!(windows) {
            ('A'..='Z')
                .map(|letter| format!("{letter}:\\"))
                .filter(|drive| Path::new(drive).exists())
                .map(FilePath::root)
                .collect()
        } else {
            vec![FilePath::root("/")]
        }
    }

    fn read(path: &FilePath) -> Result<Vec<FileEntry>> {
        let dir = Self::path_buf(path);
        let entries = fs::read_dir(&dir).with_context(|| format!("Failed to open {}", dir.display()))?;

        let mut list = vec![];
        for entry in entries {
            // One entry that vanished or cannot be read does not hide
            // the rest of the folder.
            match entry {
                Ok(entry) => list.push(Self::entry(&entry)),
                Err(error) => log::warn!("skipped an entry of {}: {error}", dir.display()),
            }
        }
        Ok(list)
    }

    fn entry(entry: &DirEntry) -> FileEntry {
        let name = entry.file_name().to_string_lossy().into_owned();

        // `fs::metadata` follows a link, so a link to a folder opens like
        // one. A broken link has only its own metadata.
        let metadata = fs::metadata(entry.path()).or_else(|_| entry.metadata());

        let Ok(metadata) = metadata else {
            return FileEntry {
                hidden: name.starts_with('.'),
                name,
                ..FileEntry::default()
            };
        };

        let mut result = if metadata.is_dir() {
            FileEntry::folder(&name)
        } else {
            FileEntry::file(&name, metadata.len())
        };
        result.modified = metadata.modified().ok().map(FileEntry::local_time);
        result.hidden = name.starts_with('.') || hidden_attribute(&metadata);
        result
    }

    fn check_name(name: &str) -> Result<()> {
        let bad = name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']);
        if bad {
            bail!("\"{name}\" is not a file name");
        }
        Ok(())
    }
}

#[cfg(windows)]
fn hidden_attribute(metadata: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;

    metadata.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0
}

#[cfg(not(windows))]
fn hidden_attribute(_: &Metadata) -> bool {
    false
}

impl FileSource for LocalFiles {
    fn places(&self) -> Vec<FilePlace> {
        let mut places = vec![];

        let mut add = |title: &str, path: Option<PathBuf>, kind: PlaceKind| {
            let path = path.filter(|path| path.is_dir());
            if let Some(path) = path.as_deref().and_then(Self::file_path) {
                places.push(FilePlace::new(title, path).kind(kind));
            }
        };

        add("Home", Some(Paths::home()), PlaceKind::Home);
        add("Desktop", dirs::desktop_dir(), PlaceKind::Folder);
        add("Documents", dirs::document_dir(), PlaceKind::Folder);
        add("Downloads", dirs::download_dir(), PlaceKind::Folder);

        for root in Self::roots() {
            let title = root.name().to_string();
            places.push(FilePlace::new(title, root).kind(PlaceKind::Drive));
        }

        places
    }

    fn list(&self, path: &FilePath, done: Listed) {
        let path = path.clone();
        thread::spawn(move || done(Self::read(&path)));
    }

    fn path_text(&self, path: &FilePath) -> String {
        Self::path_buf(path).display().to_string()
    }

    fn parse_path(&self, text: &str) -> Option<FilePath> {
        let text = text.trim();

        let path = match text.strip_prefix('~') {
            Some(rest) => Paths::home().join(rest.trim_start_matches(['/', '\\'])),
            None => PathBuf::from(text),
        };

        Self::file_path(&path)
    }

    fn can_write(&self) -> bool {
        self.writable
    }

    fn make_folder(&self, parent: &FilePath, name: &str, done: Changed) {
        if !self.writable {
            return done(Err(anyhow!("This file source is read only")));
        }
        let dir = Self::path_buf(parent).join(name);
        let name = name.to_string();
        thread::spawn(move || {
            done(Self::check_name(&name).and_then(|()| {
                fs::create_dir(&dir).with_context(|| format!("Failed to create {}", dir.display()))
            }));
        });
    }

    fn rename(&self, path: &FilePath, new_name: &str, done: Changed) {
        if !self.writable {
            return done(Err(anyhow!("This file source is read only")));
        }
        let from = Self::path_buf(path);
        let name = new_name.to_string();
        thread::spawn(move || {
            done(Self::check_name(&name).and_then(|()| {
                let to = from.with_file_name(&name);
                if to.exists() {
                    bail!("{} already exists", to.display());
                }
                fs::rename(&from, &to).with_context(|| format!("Failed to rename {}", from.display()))
            }));
        });
    }

    fn delete(&self, path: &FilePath, done: Changed) {
        if !self.writable {
            return done(Err(anyhow!("This file source is read only")));
        }
        // A root has no parent to stay in, and nobody means to wipe a disk.
        if path.is_root() || path.is_empty() {
            return done(Err(anyhow!("A root cannot be deleted")));
        }
        let target = Self::path_buf(path);
        thread::spawn(move || {
            // The link itself, never what it points at.
            let is_dir = fs::symlink_metadata(&target).is_ok_and(|metadata| metadata.is_dir());
            let result = if is_dir {
                fs::remove_dir_all(&target)
            } else {
                fs::remove_file(&target)
            };
            done(result.with_context(|| format!("Failed to delete {}", target.display())));
        });
    }
}

#[cfg(test)]
mod test {
    use std::{
        env::temp_dir,
        fs,
        path::{Path, PathBuf},
        process,
        sync::mpsc::channel,
    };

    use anyhow::Result;

    use super::LocalFiles;
    use crate::filesystem::{FileEntry, FilePath, FileSource};

    /// A fresh folder per test, removed when the test ends.
    struct TempFolder(PathBuf);

    impl TempFolder {
        fn new(name: &str) -> Self {
            let path = temp_dir().join(format!("hilen-local-files-{}-{name}", process::id()));
            if path.exists() {
                fs::remove_dir_all(&path).unwrap();
            }
            fs::create_dir_all(&path).unwrap();
            // The temp dir of a Mac is a link, the listing follows it.
            Self(path.canonicalize().unwrap())
        }

        fn path(&self) -> FilePath {
            LocalFiles::file_path(&self.0).unwrap()
        }
    }

    impl Drop for TempFolder {
        fn drop(&mut self) {
            if let Err(error) = fs::remove_dir_all(&self.0) {
                eprintln!("failed to remove {}: {error}", self.0.display());
            }
        }
    }

    fn list(source: LocalFiles, path: &FilePath) -> Result<Vec<FileEntry>> {
        let (sender, receiver) = channel();
        source.list(
            path,
            Box::new(move |result| {
                sender.send(result).unwrap();
            }),
        );
        receiver.recv().unwrap()
    }

    fn change(run: impl FnOnce(super::Changed)) -> Result<()> {
        let (sender, receiver) = channel();
        run(Box::new(move |result| {
            sender.send(result).unwrap();
        }));
        receiver.recv().unwrap()
    }

    fn names(mut entries: Vec<FileEntry>) -> Vec<String> {
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries.into_iter().map(|entry| entry.name).collect()
    }

    #[test]
    fn a_path_round_trips_through_the_disk_form() {
        let folder = TempFolder::new("round-trip");
        let path = folder.path();
        assert_eq!(LocalFiles::path_buf(&path), folder.0);
        assert_eq!(LocalFiles::file_path(Path::new("relative/path")), None);
        assert_eq!(
            LocalFiles::new().parse_path(&LocalFiles::new().path_text(&path)),
            Some(path)
        );
    }

    #[test]
    fn a_listing_has_folders_files_sizes_and_hidden_marks() {
        let folder = TempFolder::new("listing");
        fs::create_dir(folder.0.join("sub")).unwrap();
        fs::write(folder.0.join("note.txt"), "12345").unwrap();
        fs::write(folder.0.join(".secret"), "").unwrap();

        let entries = list(LocalFiles::new(), &folder.path()).unwrap();
        assert_eq!(names(entries.clone()), [".secret", "note.txt", "sub"]);

        let find = |name: &str| entries.iter().find(|entry| entry.name == name).unwrap();
        assert!(find("sub").is_folder());
        assert_eq!(find("sub").size, None);
        assert_eq!(find("note.txt").size, Some(5));
        assert!(find("note.txt").modified.is_some());
        assert!(find(".secret").hidden);
        assert!(!find("note.txt").hidden);
    }

    #[test]
    fn a_missing_folder_is_an_error_not_an_empty_list() {
        let folder = TempFolder::new("missing");
        let gone = folder.path().join("nothing-here");
        assert!(list(LocalFiles::new(), &gone).is_err());
    }

    #[test]
    fn a_read_only_source_changes_nothing() {
        let folder = TempFolder::new("read-only");
        let source = LocalFiles::new();
        assert!(!source.can_write());
        assert!(change(|done| source.make_folder(&folder.path(), "new", done)).is_err());
        assert!(!folder.0.join("new").exists());
    }

    #[test]
    fn a_writable_source_makes_renames_and_deletes() {
        let folder = TempFolder::new("writable");
        let source = LocalFiles::writable();
        let root = folder.path();

        change(|done| source.make_folder(&root, "new", done)).unwrap();
        assert!(folder.0.join("new").is_dir());
        fs::write(folder.0.join("new").join("inside.txt"), "x").unwrap();

        change(|done| source.rename(&root.join("new"), "renamed", done)).unwrap();
        assert_eq!(names(list(source, &root).unwrap()), ["renamed"]);

        // A folder goes with what is in it.
        change(|done| source.delete(&root.join("renamed"), done)).unwrap();
        assert_eq!(names(list(source, &root).unwrap()), Vec::<String>::new());
    }

    #[test]
    fn a_bad_name_and_a_taken_name_are_refused() {
        let folder = TempFolder::new("names");
        let source = LocalFiles::writable();
        let root = folder.path();
        fs::write(folder.0.join("a.txt"), "a").unwrap();
        fs::write(folder.0.join("b.txt"), "b").unwrap();

        assert!(change(|done| source.make_folder(&root, "a/b", done)).is_err());
        assert!(change(|done| source.make_folder(&root, "..", done)).is_err());
        assert!(change(|done| source.rename(&root.join("a.txt"), "b.txt", done)).is_err());
        assert_eq!(fs::read_to_string(folder.0.join("b.txt")).unwrap(), "b");
    }

    #[test]
    fn a_root_is_never_deleted() {
        let source = LocalFiles::writable();
        let root = LocalFiles::roots().remove(0);
        assert!(change(|done| source.delete(&root, done)).is_err());
    }

    #[test]
    fn the_places_start_with_home_and_end_with_the_roots() {
        let places = LocalFiles::new().places();
        assert_eq!(places.first().unwrap().title, "Home");
        assert!(places.last().unwrap().path.is_root());
    }
}
