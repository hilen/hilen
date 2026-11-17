mod edit;
mod find;
mod find_bar;
mod finding;
mod input;
mod line;
mod style;
mod text;
mod view;

#[cfg(test)]
mod tests;

pub use find::{FindError, FindOptions};
pub use style::CodeEditorStyle;
pub use text::CodePos;
pub use view::CodeEditor;
