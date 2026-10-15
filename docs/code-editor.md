# Code editor

`CodeEditor` from `hilen::ui` edits code: a gutter with line numbers, a mono font, a
caret and a selection, the keys of a text editor, copy and paste, undo and redo, and
Tab and Shift Tab that indent with spaces. The app colors the code, marks a line with
a message and lights a range of lines. It is not a `TextField`, it keeps the text as
lines. The code is `hilen/src/ui/views/complex/code_editor/`.

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

## Limits

- A phone has no keys for it yet: its screen keyboard comes up only for a
  `TextField`. Touch, the selection, the menu with copy and paste work there. See A
  phone has no keys for a view that reads the keyboard itself in
  [roadmap.md](roadmap.md).
- A browser has no paste, a page reads the clipboard only through a call that answers
  later, the same as in a `TextField`.
- No line wrap, a long line scrolls sideways. No scroll while a drag reaches an edge.
  No find and replace, no folding.

## Tests

`Code editor look`, `Code editor typing`, `Code editor selection`, `Code editor
clipboard` and `Code editor long file` in `ui-test-suite/src/views/code_editor/`. The
keys are injected key events, which the editor reads on every platform, so all run
everywhere but `Code editor clipboard`, which needs `Clipboard::set_in_process` of a
desktop.
