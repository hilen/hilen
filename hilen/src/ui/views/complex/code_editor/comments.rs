//! The comments of a language, what Cmd / of a `CodeEditor` puts in and
//! takes out, the way the language configs of Zed name them.

/// The comments of 1 language. A language with line comments toggles
/// those: the first one goes in, any of them comes out. A language with
/// none toggles a block comment around the lines.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommentRules {
    /// Like `// ` and `/// `, a space at the end goes in and comes out
    /// with it.
    pub line:  Vec<String>,
    /// The start and the end of a block comment, like `<!--` and `-->`.
    pub block: Option<(String, String)>,
}

impl CommentRules {
    /// No comments, Cmd / does nothing. The default of a new editor.
    pub fn none() -> Self {
        Self::default()
    }

    /// The comments Zed has for a file by its name or its path: Rust,
    /// TOML and Markdown have their own, plain text has none.
    pub fn for_file(file_name: &str) -> Self {
        let name = file_name.rsplit(['/', '\\']).next().unwrap_or(file_name);
        let extension = name.rsplit_once('.').map(|(_, extension)| extension);
        match (name, extension) {
            (_, Some("rs")) => Self::rust(),
            ("Cargo.lock" | "Pipfile" | "uv.lock", _) | (_, Some("toml")) => Self::toml(),
            (_, Some("md" | "mdx" | "mdwn" | "mdc" | "markdown" | "MD")) => Self::markdown(),
            _ => Self::none(),
        }
    }

    pub fn rust() -> Self {
        Self::lines(&["// ", "/// ", "//! "])
    }

    pub fn toml() -> Self {
        Self::lines(&["# "])
    }

    pub fn markdown() -> Self {
        Self {
            line:  Vec::new(),
            block: Some(("<!--".to_string(), "-->".to_string())),
        }
    }

    fn lines(prefixes: &[&str]) -> Self {
        Self {
            line:  prefixes.iter().map(ToString::to_string).collect(),
            block: None,
        }
    }

    /// How long the line comment is that `code` starts with, the longest
    /// that fits: a whole prefix with its space, or one without it.
    /// 0 when `code` starts with none.
    pub(super) fn line_prefix_len(&self, code: &str) -> usize {
        self.line
            .iter()
            .filter_map(|prefix| {
                let bare = prefix.trim_end_matches(' ');
                let rest = code.strip_prefix(bare)?;
                let spaces = rest
                    .bytes()
                    .zip(prefix[bare.len()..].bytes())
                    .take_while(|(have, want)| have == want)
                    .count();
                Some(bare.len() + spaces)
            })
            .max()
            .unwrap_or(0)
    }
}
