mod comments;
mod completion;
mod decor;
mod edit;
mod find;
mod find_bar;
mod finding;
mod inlay;
mod input;
mod line;
mod line_edit;
mod pairs;
mod panel;
mod pointer;
mod scope;
mod snippet;
mod style;
mod text;
mod view;

#[cfg(test)]
mod edit_tests;
#[cfg(test)]
mod tests;

pub use comments::CommentRules;
pub use completion::Completion;
pub use decor::MarkKind;
pub use find::{FindError, FindOptions};
pub use inlay::Inlay;
pub use pairs::{BracketPair, BracketRules};
pub use pointer::CodePress;
pub use scope::CodeScope;
pub use style::CodeEditorStyle;
pub use text::CodePos;
pub use view::CodeEditor;
