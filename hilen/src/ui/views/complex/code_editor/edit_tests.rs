//! The unit tests of the history of the undo and of the line keys of a
//! code editor, see `line_edit.rs`.

use super::{
    comments::CommentRules,
    tests::{at, text, typed},
};

#[test]
fn a_late_change_joins_the_undo_step_before_it() {
    let mut code = text("fn main() {\n    \n}");
    code.move_to(at(1, 4), false);
    // The name of a completion goes in, its use line comes later.
    code.insert("HashMap");
    let version = code.version;
    code.replace_joined(version, at(0, 0), at(0, 0), "use std::collections::HashMap;\n\n");
    let both = "use std::collections::HashMap;\n\nfn main() {\n    HashMap\n}";
    assert_eq!(code.text(), both);
    assert_eq!(code.cursor(), at(3, 11));

    // 1 undo takes back both, 1 redo puts both back.
    code.undo();
    assert_eq!(code.text(), "fn main() {\n    \n}");
    assert_eq!(code.cursor(), at(1, 4));
    assert!(!code.can_undo());
    code.redo();
    assert_eq!(code.text(), both);
    assert_eq!(code.cursor(), at(3, 11));
    assert!(!code.can_redo());

    // An undo and a redo change the version too.
    let version = code.version;
    code.undo();
    code.redo();
    assert_ne!(code.version, version);

    // A text that changed since the version gets a step of its own.
    let version = code.version;
    typed(&mut code, "::new()");
    code.replace_joined(version, at(0, 0), at(0, 0), "// top\n");
    code.undo();
    assert_eq!(code.lines()[0], "use std::collections::HashMap;");
    assert_eq!(code.lines()[3], "    HashMap::new()");
    code.undo();
    assert_eq!(code.text(), both);
}

#[test]
fn a_late_snippet_joins_the_undo_step_before_it() {
    let mut code = text("impl  {}");
    code.move_to(at(0, 5), false);
    code.insert("Add");
    let version = code.version;
    code.insert_snippet_joined(version, at(0, 5), at(0, 8), "Add for ${1:Type} ");
    assert_eq!(code.lines(), ["impl Add for Type  {}"]);
    assert_eq!(code.selected_text(), "Type");
    code.undo();
    assert_eq!(code.lines(), ["impl  {}"]);
    assert!(!code.can_undo());
}

#[test]
fn a_word_goes_back_and_forward_and_a_line_start_joins() {
    let mut code = text("let value\nnext");
    code.move_to(at(0, 9), false);
    code.delete_word_backward();
    assert_eq!(code.lines(), ["let ", "next"]);
    assert_eq!(code.cursor(), at(0, 4));
    // The undo puts the cursor back with no selection.
    code.undo();
    assert_eq!(code.cursor(), at(0, 9));
    assert_eq!(code.selection(), None);

    code.move_to(at(1, 0), false);
    code.delete_word_backward();
    assert_eq!(code.lines(), ["let valuenext"]);
    code.move_to(at(0, 0), false);
    code.delete_word_forward();
    assert_eq!(code.lines(), [" valuenext"]);
}

#[test]
fn a_line_start_and_a_line_end_take_the_rest_of_the_line() {
    let mut code = text("    let x = 1;\nb");
    code.move_to(at(0, 8), false);
    code.delete_to_line_start();
    assert_eq!(code.lines(), ["x = 1;", "b"]);
    assert_eq!(code.cursor(), at(0, 0));
    code.move_to(at(0, 1), false);
    code.delete_to_line_end();
    assert_eq!(code.lines(), ["x", "b"]);
    // At the end of the line the next one joins, at a start the one above.
    code.delete_to_line_end();
    assert_eq!(code.lines(), ["xb"]);
    code.move_to(at(0, 0), false);
    assert_eq!(code.delete_to_line_start(), None);
}

#[test]
fn ctrl_k_cuts_to_the_line_end_then_the_break() {
    let mut code = text("let x\ny");
    code.move_to(at(0, 3), false);
    let (killed, _) = code.kill_to_line_end().unwrap();
    assert_eq!(
        (killed.as_str(), code.lines()),
        (" x", &["let".to_string(), "y".to_string()][..])
    );
    let (killed, _) = code.kill_to_line_end().unwrap();
    assert_eq!(killed, "\n");
    assert_eq!(code.lines(), ["lety"]);
}

#[test]
fn whole_lines_go_and_the_cursor_keeps_its_column() {
    let mut code = text("abcd\nxy\nefgh");
    code.move_to(at(1, 2), false);
    code.delete_lines();
    assert_eq!(code.lines(), ["abcd", "efgh"]);
    assert_eq!(code.cursor(), at(1, 2));
    // The last line goes with the break before it.
    code.delete_lines();
    assert_eq!(code.lines(), ["abcd"]);
    assert_eq!(code.cursor(), at(0, 2));
    code.delete_lines();
    assert_eq!(code.lines(), [""]);

    // A selection over 2 lines takes both, 1 undo brings both back.
    let mut code = text("a\nb\nc");
    code.select(at(0, 0), at(1, 1));
    code.delete_lines();
    assert_eq!(code.lines(), ["c"]);
    code.undo();
    assert_eq!(code.lines(), ["a", "b", "c"]);
}

#[test]
fn lines_move_up_and_down_with_the_selection() {
    let mut code = text("a\nb\nc\nd");
    code.move_to(at(0, 1), false);
    code.move_block(true);
    assert_eq!(code.lines(), ["b", "a", "c", "d"]);
    assert_eq!(code.cursor(), at(1, 1));
    code.move_block(false);
    assert_eq!(code.lines(), ["a", "b", "c", "d"]);
    assert_eq!(code.move_block(false), None);

    // The lines of a selection that ends at a line start, that line not.
    code.select(at(1, 0), at(3, 0));
    code.move_block(true);
    assert_eq!(code.lines(), ["a", "d", "b", "c"]);
    assert_eq!(code.selection(), Some((at(2, 0), at(3, 1))));
    assert_eq!(code.move_block(true), None);
}

#[test]
fn a_duplicate_keeps_the_cursor_on_the_lower_or_the_upper_copy() {
    let mut code = text("a\nb");
    code.move_to(at(0, 1), false);
    code.duplicate_lines(false);
    assert_eq!(code.lines(), ["a", "a", "b"]);
    assert_eq!(code.cursor(), at(1, 1));
    code.undo();
    code.duplicate_lines(true);
    assert_eq!(code.lines(), ["a", "a", "b"]);
    assert_eq!(code.cursor(), at(0, 1));
}

#[test]
fn new_lines_below_and_above_take_the_indent() {
    let mut code = text("fn a() {\n}");
    code.move_to(at(0, 2), false);
    code.new_line_below();
    assert_eq!(code.lines(), ["fn a() {", "    ", "}"]);
    assert_eq!(code.cursor(), at(1, 4));

    let mut code = text("    x");
    code.move_to(at(0, 5), false);
    code.new_line_above();
    assert_eq!(code.lines(), ["    ", "    x"]);
    assert_eq!(code.cursor(), at(0, 4));
    // 1 undo takes the new line back and puts the cursor back.
    code.undo();
    assert_eq!(code.lines(), ["    x"]);
    assert_eq!(code.cursor(), at(0, 5));
}

#[test]
fn each_select_line_takes_1_line_more() {
    let mut code = text("a\nb\nc");
    code.select_lines();
    assert_eq!(code.selected_text(), "a\n");
    code.select_lines();
    assert_eq!(code.selected_text(), "a\nb\n");
    code.move_to(at(2, 0), false);
    code.select_lines();
    assert_eq!(code.selected_text(), "c");
}

#[test]
fn joined_lines_lose_their_indent_and_line_comment() {
    let rust = CommentRules::rust();
    let mut code = text("a\n    b\n    // c\n\nd");
    code.join_lines(&rust);
    assert_eq!(code.lines(), ["a b", "    // c", "", "d"]);
    assert_eq!(code.cursor(), at(0, 1));
    code.join_lines(&rust);
    assert_eq!(code.lines(), ["a b c", "", "d"]);
    assert_eq!(code.cursor(), at(0, 3));
    // An empty line joins with no space.
    code.join_lines(&rust);
    assert_eq!(code.lines(), ["a b c", "d"]);
    code.move_to(at(1, 0), false);
    assert_eq!(code.join_lines(&rust), None);
}

#[test]
fn a_line_comment_goes_in_and_out() {
    let rust = CommentRules::rust();
    let mut code = text("fn a() {\n    let x;\n}");
    code.move_to(at(1, 4), false);
    code.toggle_comment(&rust);
    assert_eq!(code.lines()[1], "    // let x;");
    assert_eq!(code.cursor(), at(1, 7));
    code.toggle_comment(&rust);
    assert_eq!(code.lines()[1], "    let x;");
    assert_eq!(code.cursor(), at(1, 4));
    // A comment with no space comes out too, and a doc comment.
    let mut code = text("//x\n/// y");
    code.select(at(0, 0), at(1, 1));
    code.toggle_comment(&rust);
    assert_eq!(code.lines(), ["x", "y"]);

    // The lines of a block get the comment at their smallest indent, an
    // empty line in it too, and lose it again together.
    let mut code = text("    a\n\n    b");
    code.select(at(0, 0), at(2, 5));
    code.toggle_comment(&rust);
    assert_eq!(code.lines(), ["//     a", "// ", "//     b"]);
    code.toggle_comment(&rust);
    assert_eq!(code.lines(), ["    a", "", "    b"]);

    let mut code = text("a = 1");
    code.toggle_comment(&CommentRules::toml());
    assert_eq!(code.lines(), ["# a = 1"]);
    assert_eq!(code.toggle_comment(&CommentRules::none()), None);
}

#[test]
fn a_block_comment_goes_around_the_lines() {
    let markdown = CommentRules::markdown();
    let mut code = text("hello");
    code.move_to(at(0, 5), false);
    code.toggle_comment(&markdown);
    assert_eq!(code.lines(), ["<!--hello-->"]);
    // A cursor at the line end stays in front of the closer.
    assert_eq!(code.cursor(), at(0, 9));
    code.toggle_comment(&markdown);
    assert_eq!(code.lines(), ["hello"]);

    let mut code = text("  a\nb");
    code.select(at(0, 0), at(1, 1));
    code.toggle_comment(&markdown);
    assert_eq!(code.lines(), ["  <!--a", "b-->"]);
    code.toggle_comment(&markdown);
    assert_eq!(code.lines(), ["  a", "b"]);
}

#[test]
fn a_whole_line_is_cut() {
    let mut code = text("a\nb\nc");
    code.move_to(at(1, 0), false);
    assert_eq!(code.line_with_break(), "b\n");
    let (cut, _) = code.cut_line();
    assert_eq!(cut, "b\n");
    assert_eq!(code.lines(), ["a", "c"]);
    assert_eq!(code.cursor(), at(1, 0));

    // The last line goes with its text only.
    let mut code = text("a\nb");
    code.move_to(at(1, 1), false);
    let (cut, _) = code.cut_line();
    assert_eq!(cut, "b");
    assert_eq!(code.lines(), ["a", ""]);
}

/// A browser has no paste in the editor.
#[cfg(not_wasm)]
#[test]
fn a_whole_line_is_pasted_above_the_cursor() {
    let mut code = text("a\nc");
    code.move_to(at(1, 1), false);
    code.paste_line("b\n");
    assert_eq!(code.lines(), ["a", "b", "c"]);
    assert_eq!(code.cursor(), at(2, 1));
}

#[test]
fn ctrl_t_swaps_the_chars_around_the_cursor() {
    let mut code = text("abc");
    code.move_to(at(0, 1), false);
    code.transpose();
    assert_eq!(code.lines(), ["bac"]);
    assert_eq!(code.cursor(), at(0, 2));
    code.move_to(at(0, 3), false);
    code.transpose();
    assert_eq!(code.lines(), ["bca"]);
    code.move_to(at(0, 0), false);
    assert_eq!(code.transpose(), None);
}

#[test]
fn a_line_indent_needs_no_selection() {
    let mut code = text("a");
    code.indent_lines();
    assert_eq!(code.lines(), ["    a"]);
    assert_eq!(code.cursor(), at(0, 4));
}
