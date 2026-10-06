//! The selectable side of a markdown view: its texts in reading order,
//! as labels for the highlight and as plain strings for a copy.

use std::mem::take;

use super::model::{Block, Marker};
use crate::{
    deps::refs::Weak,
    ui::{Label, MarkdownView, ViewTouch, text_selection::Lead},
};

impl MarkdownView {
    /// The text of this view can be selected with the mouse and copied,
    /// like the text of a text editor. Off by default. One selection goes
    /// over all its blocks. The copy is the text as it is drawn, with no
    /// markdown marks: an empty line between 2 blocks, a line break
    /// between 2 list items and 2 table rows, a tab between 2 table cells.
    /// Inside a `TableView` with `set_text_selectable` the view is a part
    /// of the selection of the whole table.
    ///
    /// A selectable view takes the touches over it. A click on a link
    /// still opens it, a drag that selects text does not.
    pub fn set_selectable(mut self: Weak<Self>, selectable: bool) -> Weak<Self> {
        if selectable == self.is_selectable() {
            return self;
        }
        self.texts = selectable.then(Vec::new);
        if selectable {
            self.enable_touch();
        } else {
            self.disable_touch();
        }
        // The labels are made again, each one marked or not.
        self.changed();
        self
    }

    pub fn is_selectable(&self) -> bool {
        self.texts.is_some()
    }

    /// The labels of the texts in reading order, as the last layout made
    /// them.
    pub(crate) fn text_labels(&self) -> Vec<Weak<Label>> {
        self.texts.iter().flatten().copied().filter(Weak::is_ok).collect()
    }

    /// The layout made a label for a text. The order of these calls is
    /// the order of `flat_texts`.
    pub(super) fn note_text(mut self: Weak<Self>, label: Weak<Label>) {
        if let Some(texts) = &mut self.texts {
            label.mark_selectable(true);
            texts.push(label);
        }
    }

    /// Every text in reading order, with what stands before it in a copy.
    /// Read from the blocks, so it needs no layout.
    pub(crate) fn flat_texts(&self) -> Vec<(String, Lead)> {
        let mut out = Vec::new();
        let mut next = Lead::Line;
        flatten(&self.blocks, Lead::Block, &mut next, &mut out);
        out
    }
}

/// What a list draws in front of an item. A task has a box, not a text.
pub(super) fn marker_text(marker: Marker) -> Option<String> {
    match marker {
        Marker::Bullet => Some("•".to_string()),
        Marker::Number(number) => Some(format!("{number}.")),
        Marker::Task(_) => None,
    }
}

/// Walks the blocks the way the layout does and gives every text the
/// lead of the place it stands at. `between` is the lead between 2
/// blocks of this list of blocks.
fn flatten(blocks: &[Block], between: Lead, next: &mut Lead, out: &mut Vec<(String, Lead)>) {
    for (index, block) in blocks.iter().enumerate() {
        if index > 0 {
            *next = between;
        }
        match block {
            Block::Paragraph(text) | Block::Heading { text, .. } => out.push((text.text.clone(), take(next))),
            Block::Code { text, .. } => out.push((text.clone(), take(next))),
            Block::Quote(blocks) => flatten(blocks, Lead::Block, next, out),
            Block::List(items) => {
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        *next = Lead::Line;
                    }
                    if let Some(marker) = marker_text(item.marker) {
                        out.push((marker, take(next)));
                        *next = Lead::Space;
                    }
                    flatten(&item.blocks, Lead::Line, next, out);
                }
            }
            Block::Table { head, rows } => {
                let all = [head.as_slice()].into_iter().chain(rows.iter().map(Vec::as_slice));
                for (row, cells) in all.enumerate() {
                    for (column, cell) in cells.iter().enumerate() {
                        if column > 0 {
                            *next = Lead::Tab;
                        } else if row > 0 {
                            *next = Lead::Line;
                        }
                        out.push((cell.text.clone(), take(next)));
                    }
                }
            }
            Block::Rule => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::flatten;
    use crate::ui::{text_selection::Lead, views::complex::markdown::parse::parse};

    fn copy(markdown: &str) -> String {
        let mut out = Vec::new();
        let mut next = Lead::Line;
        flatten(&parse(markdown), Lead::Block, &mut next, &mut out);
        let mut copy = String::new();
        for (index, (text, lead)) in out.iter().enumerate() {
            if index > 0 {
                copy.push_str(lead.as_str());
            }
            copy.push_str(text);
        }
        copy
    }

    #[test]
    fn marks_are_left_out_and_blocks_are_an_empty_line_apart() {
        assert_eq!(
            copy("# Title\n\nSome **bold** and `code` and a [link](https://a.b).\n\nNext."),
            "Title\n\nSome bold and code and a link.\n\nNext."
        );
    }

    #[test]
    fn a_list_copies_its_markers_and_one_line_per_item() {
        assert_eq!(
            copy("- one\n- two\n  - inner\n\n1. first\n2. second"),
            "• one\n• two\n• inner\n\n1. first\n2. second"
        );
    }

    #[test]
    fn a_task_copies_its_text_only() {
        assert_eq!(copy("- [x] done\n- [ ] open"), "done\nopen");
    }

    #[test]
    fn a_table_copies_tabs_between_cells_and_a_line_per_row() {
        assert_eq!(
            copy("| a | b |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |"),
            "a\tb\n1\t2\n3\t4"
        );
    }

    #[test]
    fn a_quote_and_code_keep_their_text() {
        assert_eq!(
            copy("> quoted\n\n```rust\nlet a = 1;\n```\n\nend"),
            "quoted\n\nlet a = 1;\n\nend"
        );
    }
}
