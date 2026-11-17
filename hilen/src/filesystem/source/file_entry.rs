#[cfg(not_wasm)]
use std::time::SystemTime;

use chrono::NaiveDateTime;
#[cfg(not_wasm)]
use chrono::{DateTime, Local};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FileKind {
    Folder,
    #[default]
    File,
}

/// What a file is, by its extension. It picks the icon and the text of
/// the kind column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileCategory {
    Folder,
    Image,
    Video,
    Audio,
    Archive,
    Code,
    Document,
    Other,
}

impl FileCategory {
    fn of_extension(extension: &str) -> Self {
        match extension {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "heic" | "tiff" => Self::Image,
            "mp4" | "mkv" | "mov" | "avi" | "webm" | "m4v" => Self::Video,
            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" | "opus" => Self::Audio,
            "zip" | "tar" | "gz" | "tgz" | "7z" | "rar" | "xz" | "bz2" | "zst" => Self::Archive,
            "rs" | "c" | "cpp" | "h" | "hpp" | "js" | "ts" | "py" | "go" | "java" | "kt" | "swift" | "sh"
            | "json" | "toml" | "yaml" | "yml" | "xml" | "html" | "css" | "sql" | "wgsl" => Self::Code,
            "txt" | "md" | "pdf" | "doc" | "docx" | "rtf" | "log" | "csv" | "xls" | "xlsx" => Self::Document,
            _ => Self::Other,
        }
    }

    fn noun(self) -> &'static str {
        match self {
            Self::Folder => "Folder",
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Archive => "archive",
            Self::Code => "code",
            Self::Document => "document",
            Self::Other => "file",
        }
    }
}

/// One row of a listing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FileEntry {
    pub name:     String,
    pub kind:     FileKind,
    /// In bytes. `None` for a folder and where the source does not know.
    pub size:     Option<u64>,
    /// The wall clock time to show, already in the time zone the user
    /// should read. A source of this machine converts to the local zone,
    /// see `FileEntry::local_time`.
    pub modified: Option<NaiveDateTime>,
    /// Shown only while the hidden files switch is on.
    pub hidden:   bool,
}

impl FileEntry {
    pub fn folder(name: impl ToString) -> Self {
        Self {
            name: name.to_string(),
            kind: FileKind::Folder,
            ..Self::default()
        }
    }

    pub fn file(name: impl ToString, size: u64) -> Self {
        Self {
            name: name.to_string(),
            kind: FileKind::File,
            size: Some(size),
            ..Self::default()
        }
    }

    #[must_use]
    pub fn modified(mut self, at: NaiveDateTime) -> Self {
        self.modified = Some(at);
        self
    }

    #[must_use]
    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    pub fn is_folder(&self) -> bool {
        self.kind == FileKind::Folder
    }

    /// Lower case, without the dot. Empty for a folder and for a name
    /// with no dot or only a leading one, like `.gitignore`.
    pub fn extension(&self) -> String {
        if self.is_folder() {
            return String::new();
        }
        match self.name.rsplit_once('.') {
            Some((stem, extension)) if !stem.is_empty() => extension.to_lowercase(),
            _ => String::new(),
        }
    }

    pub fn category(&self) -> FileCategory {
        if self.is_folder() {
            FileCategory::Folder
        } else {
            FileCategory::of_extension(&self.extension())
        }
    }

    /// The text of the kind column: `Folder`, `PNG image`, `DAT file`,
    /// and `File` for a name with no extension.
    pub fn kind_text(&self) -> String {
        let category = self.category();
        let extension = self.extension();

        if category == FileCategory::Folder {
            category.noun().to_string()
        } else if extension.is_empty() {
            "File".to_string()
        } else {
            format!("{} {}", extension.to_uppercase(), category.noun())
        }
    }

    /// A time of the file system as the wall clock of this machine.
    #[cfg(not_wasm)]
    pub fn local_time(time: SystemTime) -> NaiveDateTime {
        DateTime::<Local>::from(time).naive_local()
    }
}

#[cfg(test)]
mod test {
    use super::{FileCategory, FileEntry};

    #[test]
    fn the_extension_is_lower_case_and_needs_a_stem() {
        assert_eq!(FileEntry::file("Photo.PNG", 1).extension(), "png");
        assert_eq!(FileEntry::file("archive.tar.gz", 1).extension(), "gz");
        assert_eq!(FileEntry::file(".gitignore", 1).extension(), "");
        assert_eq!(FileEntry::file("Makefile", 1).extension(), "");
        assert_eq!(FileEntry::folder("my.folder").extension(), "");
    }

    #[test]
    fn the_kind_text_names_the_extension() {
        assert_eq!(FileEntry::folder("src").kind_text(), "Folder");
        assert_eq!(FileEntry::file("a.png", 1).kind_text(), "PNG image");
        assert_eq!(FileEntry::file("a.mkv", 1).kind_text(), "MKV video");
        assert_eq!(FileEntry::file("a.rs", 1).kind_text(), "RS code");
        assert_eq!(FileEntry::file("a.dat", 1).kind_text(), "DAT file");
        assert_eq!(FileEntry::file("Makefile", 1).kind_text(), "File");
    }

    #[test]
    fn a_folder_is_a_folder_whatever_its_name() {
        assert_eq!(FileEntry::folder("a.png").category(), FileCategory::Folder);
        assert_eq!(FileEntry::file("a.zip", 1).category(), FileCategory::Archive);
    }
}
