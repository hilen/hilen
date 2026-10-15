use super::{
    edit::LinesChanged,
    text::{CodePos, CodeText},
};

fn text(lines: &str) -> CodeText {
    let mut text = CodeText::default();
    text.set_text(lines);
    text
}

fn at(line: usize, byte: usize) -> CodePos {
    CodePos::new(line, byte)
}

fn typed(text: &mut CodeText, chars: &str) {
    for ch in chars.chars() {
        text.type_char(ch);
    }
}

#[test]
fn text_keeps_its_lines() {
    let code = text("a\r\nbb\n\nccc");
    assert_eq!(code.lines(), ["a", "bb", "", "ccc"]);
    assert_eq!(code.text(), "a\nbb\n\nccc");
    assert_eq!(text("").lines(), [""]);
}

#[test]
fn typing_and_a_new_line_keep_the_indent() {
    let mut code = text("    cube(1);");
    code.move_end(false);
    code.new_line();
    typed(&mut code, "x");
    assert_eq!(code.lines(), ["    cube(1);", "    x"]);

    // After an open bracket the new line is 1 level deeper.
    let mut code = text("module a() {");
    code.move_end(false);
    code.new_line();
    assert_eq!(code.lines(), ["module a() {", "    "]);
    assert_eq!(code.cursor(), at(1, 4));
}

#[test]
fn a_change_says_which_lines_it_replaced() {
    let mut code = text("ab\ncd\nef");
    code.select(at(0, 1), at(2, 1));
    let changed = code.insert("X\nY");
    assert_eq!(code.lines(), ["aX", "Yf"]);
    assert_eq!(
        changed,
        LinesChanged {
            start:    0,
            removed:  3,
            inserted: 2,
        }
    );
    assert_eq!(code.cursor(), at(1, 1));
}

#[test]
fn backspace_joins_lines_and_steps_back_to_a_tab_stop() {
    let mut code = text("a\nb");
    code.move_to(at(1, 0), false);
    code.delete_backward();
    assert_eq!(code.lines(), ["ab"]);
    assert_eq!(code.cursor(), at(0, 1));

    let mut code = text("      x");
    code.move_to(at(0, 6), false);
    code.delete_backward();
    assert_eq!(code.lines(), ["    x"]);
    code.delete_backward();
    assert_eq!(code.lines(), ["x"]);

    // At the very start nothing goes.
    assert_eq!(code.delete_backward(), None);
}

#[test]
fn delete_joins_the_next_line() {
    let mut code = text("a\nb");
    code.move_end(false);
    code.delete_forward();
    assert_eq!(code.lines(), ["ab"]);
    code.move_to_end(false);
    assert_eq!(code.delete_forward(), None);
}

#[test]
fn undo_takes_back_a_run_of_typing() {
    let mut code = text("");
    typed(&mut code, "abc");
    code.move_left(false);
    typed(&mut code, "X");
    assert_eq!(code.lines(), ["abXc"]);

    // The move ended the first run, so 2 undos take back both.
    code.undo();
    assert_eq!(code.lines(), ["abc"]);
    assert_eq!(code.cursor(), at(0, 2));
    code.undo();
    assert_eq!(code.lines(), [""]);
    assert!(!code.can_undo());

    code.redo();
    code.redo();
    assert_eq!(code.lines(), ["abXc"]);
    assert_eq!(code.cursor(), at(0, 3));
    assert!(!code.can_redo());

    // A new change after an undo drops the steps that could come back.
    code.undo();
    typed(&mut code, "!");
    assert!(!code.can_redo());
    assert_eq!(code.lines(), ["ab!c"]);
}

#[test]
fn undo_of_a_change_over_lines() {
    let mut code = text("one\ntwo\nthree");
    code.select(at(0, 1), at(2, 2));
    code.delete_selection();
    assert_eq!(code.lines(), ["oree"]);
    code.undo();
    assert_eq!(code.lines(), ["one", "two", "three"]);
    assert_eq!(code.selected_text(), "ne\ntwo\nth");
}

#[test]
fn tab_indents_with_spaces() {
    let mut code = text("ab");
    code.move_to(at(0, 1), false);
    code.indent();
    assert_eq!(code.lines(), ["a   b"]);
    assert_eq!(code.cursor(), at(0, 4));

    // A selection over lines: each gets 1 level, an empty line none, and
    // a last line touched only at its start is left out.
    let mut code = text("a\n\nb\nc");
    code.select(at(0, 1), at(3, 0));
    code.indent();
    assert_eq!(code.lines(), ["    a", "", "    b", "c"]);
    assert_eq!(code.selection(), Some((at(0, 5), at(3, 0))));

    code.outdent();
    assert_eq!(code.lines(), ["a", "", "b", "c"]);
    assert_eq!(code.selection(), Some((at(0, 1), at(3, 0))));
}

#[test]
fn shift_tab_takes_up_to_1_level() {
    let mut code = text("      x\n  y\n\tz\nw");
    code.select(at(0, 0), at(3, 1));
    code.outdent();
    assert_eq!(code.lines(), ["  x", "y", "z", "w"]);

    let mut code = text("x");
    assert_eq!(code.outdent(), None);
    assert!(!code.can_undo());
}

#[test]
fn up_and_down_keep_the_column() {
    let mut code = text("abcdef\nab\nabcdef");
    code.move_to(at(0, 5), false);
    code.move_lines(1, false);
    assert_eq!(code.cursor(), at(1, 2));
    code.move_lines(1, false);
    assert_eq!(code.cursor(), at(2, 5));
    code.move_lines(1, false);
    assert_eq!(code.cursor(), at(2, 6));
    code.move_lines(-10, false);
    assert_eq!(code.cursor(), at(0, 0));
}

#[test]
fn home_goes_to_the_code_then_to_the_line_start() {
    let mut code = text("    x = 1;");
    code.move_end(false);
    code.move_home(false);
    assert_eq!(code.cursor(), at(0, 4));
    code.move_home(false);
    assert_eq!(code.cursor(), at(0, 0));
}

#[test]
fn word_jumps() {
    let mut code = text("cube([10, 20]);\nnext");
    code.move_word_right(false);
    assert_eq!(code.cursor(), at(0, 4));
    code.move_word_right(false);
    assert_eq!(code.cursor(), at(0, 6));
    code.move_word_right(false);
    assert_eq!(code.cursor(), at(0, 8));
    code.move_word_right(false);
    assert_eq!(code.cursor(), at(0, 9));
    code.move_word_right(false);
    assert_eq!(code.cursor(), at(0, 12));

    code.move_end(false);
    code.move_word_right(false);
    assert_eq!(code.cursor(), at(1, 0));
    code.move_word_left(false);
    assert_eq!(code.cursor(), at(0, 15));
    code.move_word_left(false);
    assert_eq!(code.cursor(), at(0, 12));
    code.move_word_left(true);
    assert_eq!(code.selected_text(), "20");
}

#[test]
fn arrows_leave_a_selection_at_its_edge() {
    let mut code = text("abcdef");
    code.select(at(0, 4), at(0, 1));
    code.move_right(false);
    assert_eq!(code.cursor(), at(0, 4));
    assert_eq!(code.selection(), None);
    code.select(at(0, 1), at(0, 4));
    code.move_left(false);
    assert_eq!(code.cursor(), at(0, 1));
}

#[test]
fn double_and_triple_click_selections() {
    let mut code = text("translate([1, 2]) my_part();\nnext");
    code.select_word(at(0, 21));
    assert_eq!(code.selected_text(), "my_part");
    code.select_line(0);
    assert_eq!(code.selected_text(), "translate([1, 2]) my_part();\n");
    code.select_line(1);
    assert_eq!(code.selected_text(), "next");
}

#[test]
fn the_selected_part_of_each_line() {
    let mut code = text("abc\ndef\nghi");
    code.select(at(0, 1), at(2, 2));
    assert_eq!(code.selected_on(0), Some((1..3, true)));
    assert_eq!(code.selected_on(1), Some((0..3, true)));
    assert_eq!(code.selected_on(2), Some((0..2, false)));
    code.select_all();
    assert_eq!(code.selected_text(), "abc\ndef\nghi");
}

#[test]
fn chars_of_many_bytes() {
    let mut code = text("añb");
    code.move_end(false);
    code.move_left(false);
    assert_eq!(code.cursor(), at(0, 3));
    code.delete_backward();
    assert_eq!(code.lines(), ["ab"]);
    // A byte inside a char lands on its start.
    let code = text("ñ");
    assert_eq!(code.clamp(at(0, 1)), at(0, 0));
}
