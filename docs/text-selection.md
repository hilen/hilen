# Text selection

Text that selects and copies like in a text editor. Off by default, turned on per view.
The code is `hilen/src/ui/text_selection/`, the label part is
`views/basic/label_selection.rs`, the drawing is `ui/selection_drawer.rs`.

## The calls

- `Label::set_selectable(true)` and `MarkdownView::set_selectable(true)` make the text
  of that view selectable. Such a view takes the touches over it.
- `TableView::set_text_selectable(true)` makes one selection go over the cells of the
  table. Inside a cell the selectable text is every `Label` and `MarkdownView` with
  `set_selectable(true)`. A label of a cell that is not marked, a row number or a time,
  is left out.
- `TextSelection` from `hilen::ui` is the selection of the app: `text()`, `is_empty()`,
  `copy()`, `select_all()` and `clear()`.

```rust
// a cell, once in its Setup
self.line.set_selectable(true);
self.body.set_selectable(true);

// the page, once in its Setup
self.table.set_text_selectable(true);
```

## What the user gets

- A drag selects. A double click takes a word, a triple click a line, and a drag after
  them goes on by words or by lines. A line is the text between 2 line breaks, also
  when it wraps on screen.
- Shift with a click extends the selection.
- Cmd or Ctrl with C copies. Cmd or Ctrl with A takes all the text of the view the
  last click went to, in a table the text of every row. A text field in edit keeps
  these keys for itself.
- A right click opens a menu with Copy and Select All.
- A drag that reaches the top or the bottom edge of a table scrolls it. The zone is 24
  points, the speed grows with the distance.
- A press outside the view drops the selection. A press on a menu or a dialog over it
  does not.
- A click on selectable text of a cell that selects nothing is still a tap on the row,
  `cell_selected` gets it. A click on a link of a markdown view still opens it, the
  release of a drag does not.

Where a drag scrolls, a drag never selects. That is `UIManager::drag_scrolling`, on by
default on a phone and in a browser. There a long press selects the word under the
finger, the same finger then moves the end of the selection word by word, and the
release opens the menu. A tap drops the selection. An app that wants the mouse drag
in a desktop browser calls `UIManager::set_drag_scrolling(false)` there.

## What a copy holds

The text as it is drawn, with no markdown marks, and 1 line break between 2 views.
Inside a markdown view: an empty line between 2 blocks, 1 line break between 2 list
items and between 2 table rows, a tab between 2 table cells. A list marker is copied
as it is drawn, a task box copies nothing. A label that draws its text cut with an
ellipsis copies the full text. A label marked with `set_secret` is never selectable.

## Inside

There is 1 selection in the app at a time. Its 2 ends are a `Position`: the row in the
data of a table, the piece, and the byte. A piece is 1 selectable text of a cell in
reading order. No end is a view, so a recycled cell cannot break the selection, and it
stays when its rows scroll out and come back.

The scope of a selection is the table around the pressed text when that table has
selectable text, else the markdown view, else the label.

No view subscribes to an event for it, `touch().began` has room for 1 subscriber and
that one belongs to the app. The input code calls the module: `check_touch` on a press
and on a right click, `LongPress::fire` on a hold, `Input::process_touch_event` for
every move and release, `Input::on_char` for the keys. `TextSelection::tick` runs once
a frame from `UIDrawer::update`, follows the pointer and scrolls the table. The pointer
can rest at the edge with no event, so the scroll cannot depend on events.

The highlight is pulled. When a label is drawn it asks which part of it is selected,
so a layout, a scroll or a recycled cell never leaves an old highlight. The map from a
label to its row and piece is built once a frame and only while something is selected.

The rects of the highlight sit at the depth of the label and are queued before its
background. Of 2 rects at one depth the first one wins, so they show in place of the
background, in the selection color already mixed over the background color. A depth
of their own does not exist: the glyphs are less than 2 steps of a 24 bit depth buffer
in front of the label. A label with a gradient background shows the highlight over
what is behind the label.

A copy and Select All read the rows that are not on screen: the table calls
`setup_cell` for each, reads the texts and gives the cell back to the registry. So a
copy of a very long table costs 1 `setup_cell` per selected row. The text of a
markdown view is read from its parsed blocks, `flat_texts` in `markdown/selection.rs`,
and needs no layout. That walk and the layout have to visit the texts in the same
order, the labels are noted in `note_text`.

`TextField` keeps its own caret and selection. It shares the selection color and the
word rule, `word_range`.

## Limits

- No selection over a plain `ScrollView` or `Container`, only a table, a markdown view
  or a label.
- No drag handles on a touch screen and no text cursor on hover.
- The selection color is fixed and the menu texts are English.

## Tests

`Label selection`, `Markdown selection`, `Table text selection` and
`Text selection hold`. They point at a byte of a drawn text through
`ui-test-suite/src/text_points.rs`, not at a measured pixel.
