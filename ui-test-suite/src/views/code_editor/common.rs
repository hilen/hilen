use std::ops::Range;

use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    ui::{
        CodeEditor, CodePos, Color, DynamicColor, Label, ModifiersState, TextAlignment, UIColor, View,
        ViewData, ViewFrame, ViewSubviews, WeakView,
    },
    ui_test::{inject_keys, inject_modifiers, inject_touches},
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

/// `key` with the command modifier, Cmd on a Mac and Ctrl elsewhere, and
/// with `more` held too.
pub(super) fn command_with(more: ModifiersState, key: char) {
    inject_modifiers(ModifiersState::SUPER | more);
    inject_keys(key);
    inject_modifiers(ModifiersState::empty());
}

pub(super) fn command(key: char) {
    command_with(ModifiersState::empty(), key);
}

/// The keys that show the replace row of the find bar: Cmd Alt F on a
/// Mac, Ctrl H elsewhere.
pub(super) fn replace_keys() {
    let (modifiers, key) = if cfg!(target_os = "macos") {
        (ModifiersState::SUPER | ModifiersState::ALT, 'f')
    } else {
        (ModifiersState::CONTROL, 'h')
    };
    inject_modifiers(modifiers);
    inject_keys(key);
    inject_modifiers(ModifiersState::empty());
}

/// The middle of a part of the find bar on the screen, `name` is its
/// field in `FindBar`, like `case` or `replacement`.
pub(super) fn bar_point(editor: Weak<CodeEditor>, name: &'static str) -> (f32, f32) {
    from_main(move || {
        let prefix = format!("FindBar.{name}:");
        let part =
            labeled(editor.weak_view(), &prefix).unwrap_or_else(|| panic!("no {prefix} in the editor"));
        let frame = part.absolute_frame();
        (frame.x() + frame.width() / 2.0, frame.y() + frame.height() / 2.0)
    })
}

/// A click on a part of the find bar, see `bar_point`.
pub(super) fn click_bar(editor: Weak<CodeEditor>, name: &'static str) {
    inject_touches(click(bar_point(editor, name)));
    wait_for_next_frame();
}

fn labeled(view: WeakView, prefix: &str) -> Option<WeakView> {
    view.subviews().iter().find_map(|sub| {
        let sub = sub.weak();
        if sub.label().starts_with(prefix) {
            Some(sub)
        } else {
            labeled(sub, prefix)
        }
    })
}

/// A match of the find bar on 1 line.
pub(super) fn span(line: usize, from: usize, to: usize) -> (CodePos, CodePos) {
    (CodePos::new(line, from), CodePos::new(line, to))
}

/// The matches of the find bar and the index of the current one.
pub(super) fn find_state(editor: Weak<CodeEditor>) -> (Vec<(CodePos, CodePos)>, Option<usize>) {
    from_main(move || (editor.find_matches().to_vec(), editor.current_match()))
}
