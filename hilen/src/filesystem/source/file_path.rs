/// A place inside a [`FileSource`](super::FileSource): a root and the
/// names under it. Not a `PathBuf`, a source can be another machine with
/// another separator, an archive or a cloud drive, so the path is a list
/// of names and the source says how it reads as text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FilePath {
    parts: Vec<String>,
}

impl FilePath {
    /// The top of a source, `/` on a Unix disk, `C:\` on Windows, any name
    /// in a source of the app.
    pub fn root(name: impl ToString) -> Self {
        Self {
            parts: vec![name.to_string()],
        }
    }

    /// The root first, then every name under it.
    pub fn new(parts: impl IntoIterator<Item = impl ToString>) -> Self {
        Self {
            parts: parts.into_iter().map(|part| part.to_string()).collect(),
        }
    }

    #[must_use]
    pub fn join(&self, name: impl ToString) -> Self {
        let mut parts = self.parts.clone();
        parts.push(name.to_string());
        Self { parts }
    }

    /// The folder this path is in, `None` for a root and for the empty
    /// path.
    pub fn parent(&self) -> Option<Self> {
        (self.parts.len() > 1).then(|| self.prefix(self.parts.len() - 1))
    }

    /// The first `len` parts, the path a crumb leads to.
    #[must_use]
    pub fn prefix(&self, len: usize) -> Self {
        Self {
            parts: self.parts[..len.min(self.parts.len())].to_vec(),
        }
    }

    /// The last part, the root name for a root.
    pub fn name(&self) -> &str {
        self.parts.last().map_or("", String::as_str)
    }

    pub fn root_name(&self) -> &str {
        self.parts.first().map_or("", String::as_str)
    }

    /// The names under the root.
    pub fn names(&self) -> &[String] {
        self.parts.get(1..).unwrap_or_default()
    }

    pub fn parts(&self) -> &[String] {
        &self.parts
    }

    pub fn is_root(&self) -> bool {
        self.parts.len() == 1
    }

    /// No source and no folder yet, what a view starts with.
    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }

    /// The text of the default source: the parts with `/` between them,
    /// and a root named `/` not doubled.
    pub fn slash_text(&self) -> String {
        if self.root_name() == "/" {
            format!("/{}", self.names().join("/"))
        } else {
            self.parts.join("/")
        }
    }

    /// Reads what `slash_text` wrote. `None` for a text with no part.
    pub fn from_slash_text(text: &str) -> Option<Self> {
        let text = text.trim();
        let names = text.split('/').filter(|part| !part.is_empty());

        let path = if text.starts_with('/') {
            Self::new(["/"].into_iter().chain(names))
        } else {
            Self::new(names)
        };

        (!path.is_empty()).then_some(path)
    }
}

#[cfg(test)]
mod test {
    use super::FilePath;

    #[test]
    fn parent_stops_at_the_root() {
        let path = FilePath::new(["/", "home", "me"]);
        assert_eq!(path.name(), "me");
        assert_eq!(path.parent(), Some(FilePath::new(["/", "home"])));
        assert_eq!(FilePath::root("/").parent(), None);
        assert_eq!(FilePath::default().parent(), None);
    }

    #[test]
    fn slash_text_round_trips() {
        for parts in [vec!["/"], vec!["/", "a", "b"], vec!["Disk"], vec!["Disk", "a"]] {
            let path = FilePath::new(parts);
            assert_eq!(FilePath::from_slash_text(&path.slash_text()), Some(path));
        }
        assert_eq!(FilePath::root("/").slash_text(), "/");
        assert_eq!(FilePath::new(["/", "a", "b"]).slash_text(), "/a/b");
        assert_eq!(FilePath::new(["Disk", "a"]).slash_text(), "Disk/a");
    }

    #[test]
    fn a_typed_text_is_cleaned() {
        assert_eq!(
            FilePath::from_slash_text("  /a//b/ "),
            Some(FilePath::new(["/", "a", "b"]))
        );
        assert_eq!(FilePath::from_slash_text("   "), None);
    }

    #[test]
    fn prefix_is_the_path_of_a_crumb() {
        let path = FilePath::new(["/", "a", "b"]);
        assert_eq!(path.prefix(1), FilePath::root("/"));
        assert_eq!(path.prefix(2), FilePath::new(["/", "a"]));
        assert_eq!(path.prefix(9), path);
    }
}
