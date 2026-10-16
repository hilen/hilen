use super::{
    edit::LinesChanged,
    find::{FindOptions, FindState, Finder},
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

fn lines(text: &str) -> Vec<String> {
    text.split('\n').map(str::to_string).collect()
}

fn found(text: &str, query: &str, options: FindOptions) -> Vec<(CodePos, CodePos)> {
    let finder = Finder::new(query, options).expect("a valid query").expect("a query");
    finder.matches(&lines(text), 0)
}

fn span(line: usize, from: usize, to: usize) -> (CodePos, CodePos) {
    (at(line, from), at(line, to))
}

/// A find bar state with `query` searched in `text`.
fn searched(text: &[String], query: &str, options: FindOptions) -> FindState {
    let mut find = FindState::default();
    find.query = query.to_string();
    find.options = options;
    find.search(text);
    find
}

#[test]
fn find_ignores_the_case_until_asked() {
    let text = "Size = size;\nSIZE";
    assert_eq!(
        found(text, "size", FindOptions::default()),
        [span(0, 0, 4), span(0, 7, 11), span(1, 0, 4)]
    );
    let case = FindOptions {
        case_sensitive: true,
        ..FindOptions::default()
    };
    assert_eq!(found(text, "size", case), [span(0, 7, 11)]);
    // A query is plain text, its regex chars mean themselves.
    assert_eq!(found("a.b axb", "a.b", FindOptions::default()), [span(0, 0, 3)]);
    assert!(Finder::new("", FindOptions::default()).unwrap().is_none());
}

#[test]
fn whole_word_needs_no_word_char_around() {
    let words = FindOptions {
        whole_word: true,
        ..FindOptions::default()
    };
    assert_eq!(
        found("cube cubes my_cube cube(1) .cube", "cube", words),
        [span(0, 0, 4), span(0, 19, 23), span(0, 28, 32)]
    );
    // A match that starts with a char of no word may follow a word, and
    // still may not end inside one.
    assert_eq!(found("a.b a.bc", ".b", words), [span(0, 1, 3)]);
}

#[test]
fn a_regex_query_and_its_groups() {
    let regex = FindOptions {
        regex: true,
        ..FindOptions::default()
    };
    assert_eq!(found("x1 y22 z", r"\d+", regex), [span(0, 1, 2), span(0, 4, 6)]);
    // A match of nothing is left out.
    assert_eq!(found("abc", "x*", regex), []);
    assert!(Finder::new("(", regex).is_err());

    let finder = Finder::new(r"(\w+)\((\d+)\)", regex).unwrap().unwrap();
    let line = "cube(10); sphere(2);";
    assert_eq!(finder.replacement(line, 10..19, "$2$1"), "2sphere");
    // Plain text keeps a `$` as it is.
    let plain = Finder::new("cube", FindOptions::default()).unwrap().unwrap();
    assert_eq!(plain.replacement(line, 0..4, "$1"), "$1");
}

#[test]
fn next_and_back_go_round_and_the_bar_counts() {
    let text = lines("a b\na\nb a");
    let mut find = searched(&text, "a", FindOptions::default());
    assert_eq!(find.status(), "3 found");
    find.settle(at(1, 1));
    assert_eq!(find.current(), Some(2));
    assert_eq!(find.status(), "3 of 3");
    assert_eq!(find.step(true), Some(span(0, 0, 1)));
    assert_eq!(find.step(false), Some(span(2, 2, 3)));
    assert_eq!(find.on_line(2), [(2..3, true)]);
    assert_eq!(find.on_line(0), [(0..1, false)]);

    // Past the last match the cursor gives the first one.
    find.settle(at(2, 3));
    assert_eq!(find.current(), Some(0));

    find.query = "q".to_string();
    find.search(&text);
    assert_eq!(find.status(), "No results");
    find.options.regex = true;
    find.query = "[".to_string();
    find.search(&text);
    assert_eq!(find.status(), "Bad regex");
    assert!(find.error().is_some());
}

#[test]
fn matches_follow_the_edits() {
    let mut code = text("x\nab x\nx x\nx");
    let mut find = searched(code.lines(), "x", FindOptions::default());
    assert_eq!(find.matches().len(), 5);

    // A new line above the matches moves them down by 1.
    code.move_to(at(0, 1), false);
    let changed = code.new_line();
    find.lines_changed(code.lines(), changed);
    assert_eq!(
        find.matches(),
        [
            span(0, 0, 1),
            span(2, 3, 4),
            span(3, 0, 1),
            span(3, 2, 3),
            span(4, 0, 1)
        ]
    );

    // A change of 1 line finds again on that line only.
    code.move_to(at(3, 1), false);
    let changed = code.delete_backward().unwrap();
    find.lines_changed(code.lines(), changed);
    assert_eq!(
        find.matches(),
        [span(0, 0, 1), span(2, 3, 4), span(3, 1, 2), span(4, 0, 1)]
    );

    // Lines taken out take their matches with them.
    code.select(at(2, 0), at(4, 0));
    let changed = code.delete_selection().unwrap();
    find.lines_changed(code.lines(), changed);
    assert_eq!(code.lines(), ["x", "", "x"]);
    assert_eq!(find.matches(), [span(0, 0, 1), span(2, 0, 1)]);

    // A closed bar follows nothing.
    find.close();
    let changed = code.insert("x");
    find.lines_changed(code.lines(), changed);
    assert_eq!(find.matches(), []);
}

#[test]
fn replace_all_is_1_undo_step() {
    let mut code = text("cube(1);\nsphere(2);\ncube(3); cube(4);");
    let find = searched(code.lines(), "cube", FindOptions::default());
    let edits = find.replacements(code.lines(), "box");
    let changed = code.replace_all(&edits).unwrap();
    assert_eq!(code.lines(), ["box(1);", "sphere(2);", "box(3); box(4);"]);
    assert_eq!(
        changed,
        LinesChanged {
            start:    0,
            removed:  3,
            inserted: 3,
        }
    );
    code.undo();
    assert_eq!(code.lines(), ["cube(1);", "sphere(2);", "cube(3); cube(4);"]);
    assert!(!code.can_undo());
    assert_eq!(code.replace_all(&[]), None);

    // A regex replacement fills in the groups, and 1 match is its own step.
    let regex = FindOptions {
        regex: true,
        ..FindOptions::default()
    };
    let mut find = searched(code.lines(), r"cube\((\d)\)", regex);
    find.settle(at(1, 0));
    let (from, to, with) = find.current_replacement(code.lines(), "box($1, $1)").unwrap();
    code.replace_range(from, to, &with);
    assert_eq!(code.lines()[2], "box(3, 3); cube(4);");
    assert_eq!(code.cursor(), at(2, 9));
}
