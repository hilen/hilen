mod completion;
mod decor;
mod edit;
mod find;
mod find_bar;
mod finding;
mod inlay;
mod input;
mod line;
mod panel;
mod pointer;
mod style;
mod text;
mod view;

#[cfg(test)]
mod tests;

pub use completion::Completion;
pub use decor::MarkKind;
pub use find::{FindError, FindOptions};
pub use inlay::Inlay;
pub use pointer::CodePress;
pub use style::CodeEditorStyle;
pub use text::CodePos;
pub use view::CodeEditor;
