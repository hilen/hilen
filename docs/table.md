# TableView

A recycling table over `TableData`. The code is in
`hilen/src/ui/views/containers/table_view`. How an app uses it is in the
`scrolling.md` chapter of the hilen skill. This file holds what the engine does
inside.

## What a layout keeps

- A table with `set_variable_heights(true)` keeps `row_offsets`, the top of every
  row plus the end. A table with 1 height for all rows keeps nothing per row.
- `sticky_rows` is the list of cells the data marks sticky.
- `laid_cells` is the number of cells both were built for. It is none before the
  first full layout with a size, and for a table with no cells.

`layout_cells` has 3 modes. `Full` builds the offsets and the sticky rows again,
recycles every cell and sets up the ones on screen. `Resize` keeps the cells on
screen and gives each its frame again. `Scroll` keeps them and their frames, and
sets up only the rows that came into view.

## A change of a part

`reload_data()` is the `Full` mode: 1 `cell_height` call per row of a variable
table and 1 `setup_cell` call per cell on screen. A chat whose last row grows 20
times a second must not pay that, so 3 calls change a part of the layout. The code
is in `cell_reload.rs`.

- `reload_cell(index)`: the cell `index` changed, its content, its height or both.
  A variable table asks `cell_height` for the row of that cell only and moves the
  offsets of the rows under it. The cell goes to the registry and comes back
  through `setup_cell` when it is on screen, so `cell_removed` and `cell_added`
  fire for it like in a reload. Then a `Resize` layout gives every cell on screen
  its frame. No other row is asked and no other cell is set up. A cell that is not
  on screen is not set up at all.
- `load_new_cells()`: the data got more cells at its end. A variable table asks
  `cell_height` for the new rows only, `is_sticky` is read for the new cells only,
  and a `Scroll` layout sets up the new cells that are on screen.
- `drop_first_cells(count)`: the first cells left the data, a log that drops its
  oldest lines. No `cell_height` is asked. The offsets of the rows that are left
  move up by the height that is gone, the cells on screen get their new index and
  keep their content, and the scroll offset moves by the same height, so every row
  stays at its place on the screen. A text selection moves with its rows through
  `TextSelection::rows_dropped`, an end on a row that is gone goes to the start of
  the first row. A table with sticky rows, a count that does not fit the last
  layout and a count that is not whole rows reload everything.

`reload_cell` and `load_new_cells` keep the scroll offset, the caller calls
`scroll_to_bottom()` to follow the end. A table with `set_keeps_end(true)` follows it
by itself, see Keeping the end. Both fall back to `reload_data()` when the layout does not fit the data:
`laid_cells` is none or is not the count the call expects. So a wrong call costs
time and never draws a wrong table. `reload_cell` does not read `is_sticky` again.

A changed row costs 1 pass over the offsets of the rows under it, plain float
additions, and nothing for the last row.

`Table reload cell` pins it: it counts the `cell_height` and `setup_cell` calls of
every step and compares the result with a full reload of the same data.

`Table drop first cells` pins the third call the same way.

`row_top(index)` gives where a row starts in the content, for a view that keeps a
row at its place while the heights of other rows change, `LogView` does that.

## A sticky header view

`add_sticky_header_view` adds a header view whose frame the table sets itself, so it
takes no `place()` rules. `set_sticky_header_place(top, height)` says where it rides
in the header. `place_sticky_header` in `layout.rs` runs at the end of every layout,
all 3 modes and the empty table too, and puts it at `max(top, -offset)` in the
content, as wide as the table. A sticky row in a grid is only 1 column wide, which is
why this is a header view and not a row.

It is raised once by `sticky_raise` when it is added, so it draws in front of the
cells and its later subviews inherit that depth. Touches go to the view registered
last, and a cell set up by a scroll registers after it, so while it is stuck every
layout moves its subtree to the front of the touches again. A tap on it never
selects a cell under it, `select_at` and `index_at` both check
`sticky_header_covers`.

`Table sticky header` pins it: a grid of 3 columns, a plain header over the sticky
one, a real wheel scroll that pins it over a row, a tap on it, a tap on its button
and a scroll back to its place.

## Keeping the end

`set_keeps_end(true)` is a rule of `ScrollView`, the table passes it on to the scroll
view inside it. It is off by default. With it a view that shows the end of its content
goes on showing it when its own height or the height of its content changes. A view
scrolled away from the end keeps its offset. `is_at_end()` tells where the view is now.
The code is in `scrolling/scroll_view.rs`.

- The scroll view keeps 1 flag, `at_end`. It is set when the offset is set: by a
  wheel, a drag, a bar, `set_content_offset`, `scroll_to_bottom` and `scroll_to_row`.
  It is true when the offset is within 1 point of the end, and when all of the content
  fits the view.
- A new height does not set the flag first. It reads it, in `settle_end`: a view that
  was at its end goes to the new end, any other view is only cut to the new range. So
  the flag tells what the view showed before the change. Then the flag is set again,
  a view that grew until its end came into view is at its end from then on.
- The content height changes inside `set_content_height`, which every layout of the
  table calls before it reads the offset. So `reload_data`, `reload_cell` and
  `load_new_cells` put the rows at the new end in the same call, with no
  `scroll_to_bottom` after them. A call with the same height does nothing. Without
  that a slow scroll away from the end, which is made of steps smaller than 1 point,
  would be pulled back on every step.
- The height of the view changes in the layout of a frame. The views are laid out
  from the outside in: the table gets its frame and lays out its rows, then the scroll
  view inside it gets its frame, then the content view. `ScrollView::resized` runs
  when the scroll view has its new frame, holds the end against that height and fires
  `on_scroll`, and the table lays out the rows that came into view. All of this is
  before the frame is drawn, so no frame shows the end out of view. This also holds
  for a view that gets another height on every frame, a table above a `b_keyboard`
  box while the screen keyboard moves, see [screen-keyboard.md](screen-keyboard.md).
- Empty content fits the view, so a view that is filled later starts at its end, a
  chat opens on its last message. Turn the rule on in `Setup`. `set_content_offset(0)`
  after the first rows starts such a view at its top.
- A `ScrollView` with the automatic content height learns that height one frame after
  its views have their frames. New content is followed one frame later there. A new
  height of the view is followed in the same frame.
- With the rule off nothing changed: a view keeps its offset and is only cut to the
  range, and `resized` still measures against the height the content view had one
  frame before.

`LogView` does not use the rule, it has its own `keep_end`. It follows the end only
after a scroll of the user, it shows a button while it does not follow, and it gives
rows their exact heights in the same step, which moves the end again.

`Table keeps end` and `Scroll keeps end` pin it. Each has a view with the rule and a
plain one side by side, and a view that counts the frames drawn off the end.
