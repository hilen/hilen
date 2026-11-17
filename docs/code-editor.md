# Code editor

`CodeEditor` from `hilen::ui` edits code: a gutter with line numbers, a mono font, a
caret and a selection, the keys of a text editor, copy and paste, undo and redo,
Tab and Shift Tab that indent with spaces, and a find bar with replace. The app
colors the code, marks a line with a message and lights a range of lines. It is not
a `TextField`, it keeps the text as lines. The code is
`hilen/src/ui/views/complex/code_editor/`.

## The calls

```rust
self.editor.set_text(&source);
self.editor.changed.val(move |first_line| self.highlight_from(first_line));
self.editor.set_color_runs(line, runs);           // byte ranges of that line
self.editor.set_mark(12, "unknown module part");  // an error on line 13
self.editor.set_highlight(Some(6..8));            // lines 7 and 8
self.editor.place().back();
```

- `set_text` puts a new text in with no colors, no marks and no undo history. It
  fires nothing. `text()` gives the whole text, `lines()` the lines.
- `changed` fires on every change the user makes, with the first line that changed.
  The colors of the changed lines are gone, the app sets them again in the handler.
  The handler runs before the next frame, so nothing flashes. `CodeHighlighter`
  gives the runs, see Syntax colors in the hilen skill.
- `set_color_runs(line, runs)` and `color_runs(line)`. Runs are set per line and drawn
  on the next frame, so an app can set 10 000 lines in a row.
- `set_mark(line, message)`, `mark(line)`, `clear_mark(line)` and `clear_marks()`.
  A marked line has its number and its message in the mark color and its own
  background, the message stands 3 chars after the code. A mark moves with its line
  when lines are added or taken out above it.
- `set_highlight(Some(range))` lights whole lines, like the lines of a part picked in
  a 3D view. `scroll_to_line(line)` brings a line to the top.
- `cursor()`, `set_cursor(pos)`, `select(from, to)`, `select_all()`, `selection()`,
  `selected_text()`. A `CodePos` is a line and a byte of that line, both from 0.
  `cursor_moved` fires when the cursor moves.
- `insert(text)`, `undo()`, `redo()`, `can_undo()`, `can_redo()`, `copy()`, `cut()`
  and `paste()` for the buttons and the menu of an app.
- `set_tab_width(spaces)`, 4 by default. `set_style(CodeEditorStyle { .. })` sets the
  text size, the font, the spacing and the colors. Every default color is a light and
  dark pair. `focus()` gives the editor the keys, `is_editing()` tells if it has them.
- `point_of(pos)` is the place of a byte on the screen, for a test.

## Find and replace

Every editor has a find bar, the app writes nothing for it. The calls are for an app
that wants its own menu entries or buttons:

```rust
let matches = self.editor.find("plate", FindOptions::default())?; // Vec<(CodePos, CodePos)>
self.editor.show_find("plate");   // opens the bar with this query
self.editor.find_next();          // and find_previous()
self.editor.close_find();
```

- `find(query, options)` gives the start and the end of every match, in the order of
  the text, and needs no open bar. `FindOptions` has `case_sensitive`, `whole_word`
  and `regex`, all off by default. A regular expression is one of the `regex` crate,
  one that does not parse is a `FindError`. An empty query finds nothing. A match
  never goes over a line break, and a match of nothing, like `a*`, is left out.
- A whole word does not start or end inside a word: a letter, digit or `_` at the
  edge of the match has none of them next to it outside.
- `show_find(query)` opens the bar with that query and gives its field the keys.
  `close_find()`, `is_find_open()`, `find_matches()`, `current_match()`, the index
  of the current match, `find_options()` and `set_find_options(options)`, which the
  toggles show.
- `find_next()` and `find_previous()` make the next or the match before current and
  scroll to it, after the last comes the first. While the editor has the keys the
  match is also selected.
- The colors are light and dark pairs in `CodeEditorStyle`: `find_match`,
  `find_current`, `find_bar`, `find_field`, `find_text` and `find_toggle`.

## What the user gets

- A press puts the caret, a drag selects, a double click takes a name, a triple
  click the line, Shift with a press makes the selection reach it.
- The arrows, Home and End, Page Up and Page Down. Home goes to the first char that
  is no space, then to the line start. Up and Down keep the column of the first move.
  Alt with Left and Right jumps by words on a Mac, Ctrl elsewhere. Cmd with Left and
  Right goes to the line start and end on a Mac. Cmd or Ctrl with Up, Down, Home and
  End goes to the start and the end of the text. Shift with any of them selects.
- Enter keeps the indent of the line, 1 level more after `{`, `(` or `[`. Tab with no
  selection puts spaces up to the next tab stop. Tab with a selection indents every
  line of it, Shift Tab takes 1 level out. Backspace in an indent goes back to the tab
  stop before.
- Cmd or Ctrl with A, C, X, V, Z, Shift Z and Y. Typed chars on 1 line are 1 undo step
  until the cursor moves.
- Escape drops the selection, a second Escape ends the editing.
- A right click, or a long press on a touch screen, opens a menu with Undo, Redo, Cut,
  Copy, Paste and Select All. A long press takes the word under the finger first.
- Where a drag scrolls, on a phone and in a browser, a drag scrolls and a tap puts
  the caret. See `UIManager::set_drag_scrolling`.
- Cmd F on a Mac, Ctrl F elsewhere, opens the find bar over the top of the editor,
  the rows move down under it. The selected text is the query when it is on 1 line,
  else the query of the last find stays. Every match gets a background, the current
  one a stronger one, and the bar says "3 of 12", "No results" or "Bad regex". The
  first match from the cursor is current.
- In the bar Enter and Cmd G go to the next match, Shift Enter and Cmd Shift G to the
  one before, and the editor scrolls to it. The toggles `Aa`, `W` and `.*` are the
  case, whole words and a regular expression. Escape or the x closes the bar, selects
  the current match and gives the editor the keys back. Escape in the editor while the
  bar is open closes the bar too.
- Cmd Alt F on a Mac, Ctrl H elsewhere, shows the replace row. Enter or Replace
  replaces the current match and goes to the next, Cmd Enter or All replaces every
  match as 1 undo step. With a regular expression a replacement can name the groups,
  `$1` or `${name}`.
- The matches follow the edits: a change finds again on the lines it gave, the
  matches after them move along, and the first match from the cursor is current.

## Inside

- `text.rs` and `edit.rs` hold the lines, the cursor and the history, with no view.
  Every change is 1 step: some lines from a start line gave way to new ones. A step
  keeps the old and the new lines, so the history costs the lines it touched, not
  copies of the file. It keeps 1000 steps. The unit tests are in `tests.rs`.
- The view is a `TableView` with 1 row per line, so only the lines on screen are
  views. A row is `CodeLine` in `line.rs`: the selection, the code label, the caret,
  the message, the gutter and the number, in that order. The code labels use the
  cell grid of `Label`, so columns line up. The table scrolls sideways with
  `set_content_width`, the gutter stays, the parts marked `set_moves_sideways` move.
- The widest line is counted in columns of the font, so the width of a long file
  costs no shaping.
- An edited editor counts in `UIManager::text_editing` like a text field. The key
  focus ring, an Escape of a modal and the copy of `TextSelection` leave its keys
  alone. It holds the arrows with `Focus::hold_keys` while it has the keys.
- Find is 3 files. `find.rs` is the matching with no view, `Finder` and `FindState`,
  with unit tests in `tests.rs`. `find_bar.rs` is the bar, it only sends a
  `FindCommand`. `finding.rs` has the calls and does the commands. The bar takes Enter
  and Escape of its fields with `TextField::take_keys`, and reads Cmd with a letter
  from `UIEvents::keyboard_input` while a field of it is edited.
- The editor makes the bar with `add_view` after its own touch is on. The view that
  turns its touch on last gets a press first, so the fields and buttons of the bar get
  theirs before the editor. A press on the bar between them is ignored by the editor.
  The buttons keep the selection, so a toggle leaves the keys with the field.
- A match is drawn as a background run of the code label of its row,
  `Label::set_background_runs`. Replace All is 1 step of the history over the lines
  from the first match to the last, `CodeText::replace_all`.

## Limits

- A phone has no keys for it yet: its screen keyboard comes up only for a
  `TextField`. Touch, the selection, the menu with copy and paste work there. See A
  phone has no keys for a view that reads the keyboard itself in
  [roadmap.md](roadmap.md).
- A browser has no paste, a page reads the clipboard only through a call that answers
  later, the same as in a `TextField`.
- No line wrap, a long line scrolls sideways. No scroll while a drag reaches an edge.
  No folding.
- Find has no search over several files and no history of queries. A match does not
  go over a line break. On a phone the bar opens only by `show_find` or by keys of a
  hardware keyboard, and its fields get the keys only from a tap.
- A selected match shows the match color, not the selection color, while the bar is
  open.

## Tests

`Code editor look`, `Code editor typing`, `Code editor selection`, `Code editor
clipboard`, `Code editor long file`, `Code editor find`, `Code editor find next` and
`Code editor replace` in `ui-test-suite/src/views/code_editor/`. The keys are
injected key events, which the editor reads on every platform, so all run everywhere
but `Code editor clipboard`, which needs `Clipboard::set_in_process` of a desktop.
The fields of the find bar are typed into through `system_input`, which types through
the screen keyboard of a phone. `Code editor find` pins the look of the bar and the
matches in both themes and the toggles, `Code editor find next` next and back with
the scroll, Escape and the matches that follow a typed change, `Code editor replace`
the replace row, Replace, Replace All and its undo as 1 step.
