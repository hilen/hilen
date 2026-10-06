//! Markdown as blocks. The parser makes them, `MarkdownView` draws them.

use std::ops::Range;

/// One piece of a text: a paragraph, a list, a table and so on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Block {
    Heading {
        level: u8,
        text:  Styled,
    },
    Paragraph(Styled),
    Code {
        language: String,
        text:     String,
    },
    Quote(Vec<Block>),
    List(Vec<Item>),
    Table {
        head: Vec<Styled>,
        rows: Vec<Vec<Styled>>,
    },
    Rule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Item {
    pub marker: Marker,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Marker {
    Bullet,
    Number(u64),
    /// A task of a task list, done or not.
    Task(bool),
}

/// A text with no marks left in it, and the byte ranges that are drawn
/// in another way than the rest.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Styled {
    pub text:  String,
    pub spans: Vec<Span>,
    pub links: Vec<Link>,
}

impl Styled {
    pub fn plain(text: &str) -> Self {
        Self {
            text:  text.to_string(),
            spans: Vec::new(),
            links: Vec::new(),
        }
    }
}

/// The byte range of a link and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Link {
    pub range: Range<usize>,
    pub url:   String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Span {
    pub range: Range<usize>,
    pub style: Style,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Style {
    pub face:   Face,
    pub code:   bool,
    pub strike: bool,
    pub link:   bool,
}

/// The weight and the slant of a piece of text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum Face {
    #[default]
    Regular,
    Bold,
    Italic,
    BoldItalic,
}

impl Face {
    pub fn new(bold: bool, italic: bool) -> Self {
        match (bold, italic) {
            (false, false) => Face::Regular,
            (true, false) => Face::Bold,
            (false, true) => Face::Italic,
            (true, true) => Face::BoldItalic,
        }
    }

    pub fn is_bold(self) -> bool {
        matches!(self, Face::Bold | Face::BoldItalic)
    }

    pub fn is_italic(self) -> bool {
        matches!(self, Face::Italic | Face::BoldItalic)
    }
}
