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
`scroll_to_bottom()` to follow the end. Both fall back to `reload_data()` when the layout does not fit the data:
`laid_cells` is none or is not the count the call expects. So a wrong call costs
time and never draws a wrong table. `reload_cell` does not read `is_sticky` again.

A changed row costs 1 pass over the offsets of the rows under it, plain float
additions, and nothing for the last row.

`Table reload cell` pins it: it counts the `cell_height` and `setup_cell` calls of
every step and compares the result with a full reload of the same data.

`Table drop first cells` pins the third call the same way.

`row_top(index)` gives where a row starts in the content, for a view that keeps a
row at its place while the heights of other rows change, `LogView` does that.
