use std::ops::Range;

use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{CodeEditor, CodePos, Color, DynamicColor, Label, TextAlignment, UIColor, ViewFrame},
    ui_test::inject_touches,
};

use crate::text_points::{click, drag};

/// A small `.scad` file, what fluorit edits.
pub(super) const SCAD: &str = r"// a plate with a hole
size = [40, 30, 10];

module plate() {
    difference() {
        cube(size);
        translate([20, 15, -1])
            cylinder(h = 12, r = 5);
    }
}

plate();
undefined_part();";

const KEYWORD: UIColor = UIColor::Dynamic(DynamicColor::new(Color::hex("#cf222e"), Color::hex("#ff7b72")));
const CALL: UIColor = UIColor::Dynamic(DynamicColor::new(Color::hex("#8250df"), Color::hex("#d2a8ff")));
const NUMBER: UIColor = UIColor::Dynamic(DynamicColor::new(Color::hex("#0550ae"), Color::hex("#79c0ff")));
const COMMENT: UIColor = UIColor::Dynamic(DynamicColor::new(Color::hex("#6e7781"), Color::hex("#8b949e")));

const KEYWORDS: [&str; 1] = ["module"];
const CALLS: [&str; 5] = ["difference", "cube", "translate", "cylinder", "plate"];

/// The colors of a line, the way the highlighter of an app gives them:
/// a comment, the keyword, the calls of the language and the numbers.
pub(super) fn runs(line: &str) -> Vec<(Range<usize>, UIColor)> {
    if let Some(start) = line.find("//") {
        return vec![(start..line.len(), COMMENT)];
    }
    let mut runs = Vec::new();
    let mut start = None;
    for (byte, ch) in line.char_indices().chain([(line.len(), ' ')]) {
        let in_word = ch.is_alphanumeric() || ch == '_';
        match (start, in_word) {
            (None, true) => start = Some(byte),
            (Some(from), false) => {
                let word = &line[from..byte];
                let color = if KEYWORDS.contains(&word) {
                    Some(KEYWORD)
                } else if CALLS.contains(&word) {
                    Some(CALL)
                } else if word.starts_with(|ch: char| ch.is_ascii_digit()) {
                    Some(NUMBER)
                } else {
                    None
                };
                if let Some(color) = color {
                    runs.push((from..byte, color));
                }
                start = None;
            }
            _ => {}
        }
    }
    runs
}

/// Colors every line of the editor.
pub(super) fn color_all(mut editor: Weak<CodeEditor>) {
    let lines = editor.lines().to_vec();
    for (index, line) in lines.iter().enumerate() {
        editor.set_color_runs(index, runs(line));
    }
}

/// The status label under the editor, which says what the step shows.
pub(super) fn status_style(label: Weak<Label>) {
    label.set_text_size(16).set_multiline(true);
    label.set_alignment(TextAlignment::Left);
}

/// Where `pos` is on the screen. Call it on the main thread.
pub(super) fn screen_point(editor: Weak<CodeEditor>, pos: CodePos) -> (f32, f32) {
    let point = editor.point_of(pos).expect("the line is not laid out");
    let frame = editor.absolute_frame();
    (frame.x() + point.x, frame.y() + point.y)
}

/// A click on the caret place of `pos`.
pub(super) fn click_at(editor: Weak<CodeEditor>, pos: CodePos) {
    let point = from_main(move || screen_point(editor, pos));
    inject_touches(click(point));
    wait_for_next_frame();
}

/// A drag from the caret place of `from` to that of `to`.
pub(super) fn drag_over(editor: Weak<CodeEditor>, from: CodePos, to: CodePos) {
    let (from, to) = from_main(move || (screen_point(editor, from), screen_point(editor, to)));
    inject_touches(drag(from, to));
    wait_for_next_frame();
}

/// The lines of the text for a status label: each in quotes, so the
/// spaces of an indent show.
pub(super) fn quoted(lines: &[String]) -> String {
    lines.iter().map(|line| format!("{line:?}")).collect::<Vec<_>>().join("\n")
}
