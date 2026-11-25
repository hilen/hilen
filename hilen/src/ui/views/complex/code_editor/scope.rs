//! The parts of code where a code editor treats the keys apart. A
//! language definition gives them with its colors, see
//! `CodeHighlighter::runs`.

/// A part of code where a code editor treats the keys apart, like a
/// string, where a typed `/*` opens no comment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeScope {
    String,
    Comment,
}
