//! A table drawn with box chars, read into rows and cells so the view can
//! draw it as a real table, see `MarkdownBoxTables::Table`.

use std::ops::Range;

use crate::ui::views::complex::markdown::model::{Block, Item, Styled};

/// The chars that stand between 2 cells of a line.
const BARS: [char; 9] = ['│', '┃', '║', '╎', '╏', '┆', '┇', '┊', '┋'];

/// The chars a rule line draws its lines with, 1 of them makes a rule.
const RULES: [char; 9] = ['─', '━', '═', '┄', '┅', '┈', '┉', '╌', '╍'];

/// The cells of a table as byte ranges of its text: per row, per cell, the
/// trimmed piece of every line of the row, empty pieces left out.
type Rows = Vec<Vec<Vec<Range<usize>>>>;

/// Every code block and every paragraph that is a box table becomes a
/// table, also inside a quote and a list.
pub(super) fn real_tables(blocks: Vec<Block>) -> Vec<Block> {
    blocks
        .into_iter()
        .map(|block| match block {
            Block::Code { language, text } => match rows_of(&text) {
                Some(rows) => table(&rows, |pieces| plain(&text, pieces)),
                None => Block::Code { language, text },
            },
            Block::Paragraph(styled) => match rows_of(&styled.text) {
                Some(rows) => table(&rows, |pieces| styled.pieces(pieces)),
                None => Block::Paragraph(styled),
            },
            Block::Quote(blocks) => Block::Quote(real_tables(blocks)),
            Block::List(items) => Block::List(
                items
                    .into_iter()
                    .map(|item| Item {
                        marker: item.marker,
                        blocks: real_tables(item.blocks),
                    })
                    .collect(),
            ),
            other => other,
        })
        .collect()
}

fn table(rows: &Rows, cell: impl Fn(&[Range<usize>]) -> Styled) -> Block {
    let mut rows = rows.iter().map(|row| row.iter().map(|pieces| cell(pieces)).collect::<Vec<_>>());
    let head = rows.next().unwrap_or_default();
    Block::Table {
        head,
        rows: rows.collect(),
    }
}

fn plain(text: &str, pieces: &[Range<usize>]) -> Styled {
    let words: Vec<&str> = pieces.iter().map(|range| &text[range.clone()]).collect();
    Styled::plain(&words.join(" "))
}

/// The rows of `text` when all of it is 1 box table: a rule line first
/// and last, and every line between a rule or a line of cells, all with
/// the same count of cells and at least 2. `None` for anything else, a
/// box around a note or a drawing stays as typed.
fn rows_of(text: &str) -> Option<Rows> {
    let lines: Vec<(usize, &str)> = lines_of(text)
        .map(|(start, line)| {
            let trimmed = line.trim_start();
            (start + line.len() - trimmed.len(), trimmed.trim_end())
        })
        .collect();
    let first = lines.iter().position(|(_, line)| !line.is_empty())?;
    let last = lines.iter().rposition(|(_, line)| !line.is_empty())?;
    let lines = &lines[first..=last];
    if !is_rule(lines[0].1) || !is_rule(lines[lines.len() - 1].1) {
        return None;
    }

    let mut rows: Rows = Vec::new();
    let mut row: Vec<Vec<Range<usize>>> = Vec::new();
    let mut columns = None;
    for &(start, line) in lines {
        if is_rule(line) {
            if !row.is_empty() {
                rows.push(std::mem::take(&mut row));
            }
            continue;
        }
        let cells = cells_of(start, line)?;
        if *columns.get_or_insert(cells.len()) != cells.len() {
            return None;
        }
        if row.is_empty() {
            row = vec![Vec::new(); cells.len()];
        }
        for (pieces, cell) in row.iter_mut().zip(cells) {
            if !cell.is_empty() {
                pieces.push(cell);
            }
        }
    }
    (columns? >= 2 && !rows.is_empty()).then_some(rows)
}

/// Every line of `text` with the byte it starts at.
fn lines_of(text: &str) -> impl Iterator<Item = (usize, &str)> {
    let mut start = 0;
    text.split('\n').map(move |line| {
        let at = start;
        start += line.len() + 1;
        (at, line)
    })
}

/// A line of only box chars and spaces with at least 1 char of a line.
fn is_rule(line: &str) -> bool {
    line.chars().any(|char| RULES.contains(&char))
        && line.chars().all(|char| char == ' ' || is_box_char(char))
}

fn is_box_char(char: char) -> bool {
    ('\u{2500}'..='\u{257F}').contains(&char)
}

/// The trimmed cells of a line that starts and ends with a bar, as byte
/// ranges of the whole text, `start` is where the line starts.
fn cells_of(start: usize, line: &str) -> Option<Vec<Range<usize>>> {
    let first = line.chars().next()?;
    let last = line.chars().next_back()?;
    if !BARS.contains(&first) || !BARS.contains(&last) || line.len() <= first.len_utf8() {
        return None;
    }
    let mut cells = Vec::new();
    let mut cell_start = first.len_utf8();
    for (index, char) in line.char_indices().skip(1) {
        if BARS.contains(&char) {
            let piece = &line[cell_start..index];
            let lead = piece.len() - piece.trim_start().len();
            let trimmed = piece.trim();
            let from = start + cell_start + lead;
            cells.push(from..from + trimmed.len());
            cell_start = index + char.len_utf8();
        }
    }
    Some(cells)
}

#[cfg(test)]
mod tests {
    use super::real_tables;
    use crate::ui::views::complex::markdown::{
        model::{Block, Kind, Styled},
        parse::parse_lines,
    };

    fn texts(cells: &[Styled]) -> Vec<&str> {
        cells.iter().map(|cell| cell.text.as_str()).collect()
    }

    fn code(text: &str) -> Vec<Block> {
        real_tables(vec![Block::Code {
            language: String::new(),
            text:     text.to_string(),
        }])
    }

    /// The table of a chat on 2026-11-15 that fell apart on a phone.
    const STEPS: &str = "\
╭──────┬───────┬───────────┬───────╮
│ Step │ Name  │ Leibniz   │ Unit  │
├──────┼───────┼───────────┼───────┤
│ 1    │ speed │ dx/dt     │ m/s   │
├──────┼───────┼───────────┼───────┤
│ 2    │ accel │ d^2x/dt^2 │ m/s^2 │
╰──────┴───────┴───────────┴───────╯
";

    #[test]
    fn a_box_table_in_a_code_block_becomes_a_table() {
        let [Block::Table { head, rows }] = &code(STEPS)[..] else {
            panic!("a table expected");
        };
        assert_eq!(texts(head), ["Step", "Name", "Leibniz", "Unit"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(texts(&rows[0]), ["1", "speed", "dx/dt", "m/s"]);
        assert_eq!(texts(&rows[1]), ["2", "accel", "d^2x/dt^2", "m/s^2"]);
    }

    #[test]
    fn the_lines_of_1_row_join_into_its_cells() {
        let text = "\
┌──────┬──────────────┐
│ Name │ What it does │
├──────┼──────────────┤
│ rate │ the change   │
│      │ of 1 step    │
└──────┴──────────────┘";
        let [Block::Table { rows, .. }] = &code(text)[..] else {
            panic!("a table expected");
        };
        assert_eq!(texts(&rows[0]), ["rate", "the change of 1 step"]);
    }

    #[test]
    fn a_box_that_is_no_table_stays_code() {
        let note = "┌──────────┐\n│ one note │\n└──────────┘";
        let uneven = "┌───┬───┐\n│ a │ b │\n│ c │\n└───┴───┘";
        let mixed = "┌───┬───┐\n│ a │ b │\n└───┴───┘\nthen a line of code";
        let open = "│ a │ b │\n│ c │ d │";
        for text in [note, uneven, mixed, open] {
            assert!(
                matches!(&code(text)[..], [Block::Code { .. }]),
                "{text} became a table"
            );
        }
    }

    #[test]
    fn a_paragraph_table_keeps_its_marks_and_others_stay() {
        let text = "Before.\n\n╭───┬────────╮\n│ a │ `code` │\n╰───┴────────╯\n\nAfter.";
        let blocks = real_tables(parse_lines(text, true, false));
        let [
            Block::Paragraph(_),
            Block::Table { head, .. },
            Block::Paragraph(_),
        ] = &blocks[..]
        else {
            panic!("a paragraph, a table and a paragraph expected, got {blocks:?}");
        };
        assert_eq!(texts(head), ["a", "code"]);
        let code = &head[1];
        assert!(
            code.spans
                .iter()
                .any(|span| span.style.kind == Kind::Code && span.range == (0..4)),
            "the code mark of the cell is gone: {:?}",
            code.spans
        );
    }

    #[test]
    fn a_table_inside_a_list_item_becomes_a_table() {
        let mut text = String::from("- the steps:\n\n  ```\n");
        for line in STEPS.lines() {
            text.push_str("  ");
            text.push_str(line);
            text.push('\n');
        }
        text.push_str("  ```\n");
        let blocks = real_tables(parse_lines(&text, false, false));
        let [Block::List(items)] = &blocks[..] else {
            panic!("a list expected, got {blocks:?}");
        };
        assert!(
            items[0].blocks.iter().any(|block| matches!(block, Block::Table { .. })),
            "no table in the item: {:?}",
            items[0].blocks
        );
    }
}
