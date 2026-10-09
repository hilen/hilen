# LogView

A ready view for a log. The code is in `hilen/src/ui/views/complex/log_view`.

## What it does

- 1 row per line, in a mono font. A long line wraps and its row is as tall as its text.
- ANSI codes of a line are drawn: the 16 colors, the table of 256 colors, a free color,
  bold, dim, underlined and struck text. Every other escape code is dropped. A
  background color is not drawn. Each line starts plain, a line does not go on with the
  look of the line before it.
- A line has a text and an optional prefix. The view does not know what the prefix
  means, an app puts a time, a name or a level there. The prefix column is as wide as
  the widest prefix, and the text wraps beside it.
- The text selects over many rows, see [text-selection.md](text-selection.md). A copy
  of a row is the prefix, 1 space and the text, with no color code. A row with no
  prefix copies only its text.
- While the view shows its end it follows new lines. After a scroll up it stays, and a
  round button with an arrow shows in the bottom right corner. A tap on it, or a
  scroll down to the end, follows again.

## The calls

The app holds the lines, the view keeps no copy of them.

```rust
impl LogData for MyPage {
    fn number_of_lines(&self) -> usize {
        self.lines.len()
    }

    fn line(&self, index: usize) -> LogLine {
        let entry = &self.lines[index];
        LogLine::new(&entry.text).with_prefix(&entry.time)
    }
}

// once, in Setup
self.log.set_data_source(self);

// after a change of the lines
self.log.lines_added();      // lines came to the end
self.log.lines_removed(n);   // the first n lines are gone
self.log.reload();           // anything else
```

- `line` is asked when a line is added and every time its row comes on screen, so it
  has to be cheap.
- The view drops nothing by itself. A log that grows without end is the concern of the
  app: it trims its own data and calls `lines_removed`.
- `is_following()` and `scroll_to_end()` are there for a button of the app.
- `set_style(LogStyle { .. })` sets the text size, the fonts, the padding and the
  colors. Every default color is a light and dark pair.

## Heights without shaping

A row is as tall as its wrapped text, and the wrap depends on the width. Shaping the
text of every line for that costs about 15 microseconds a line, 1.5 seconds for
100 000 lines, on every new width. So the view shapes only what is on screen.

- Per line it keeps the count of its chars, taken when the line is added.
- The height of a row that is not on screen is arithmetic: chars divided by the chars
  that fit 1 row of the text column, rounded up, times the height of 1 line. The width
  of 1 char comes from 2 measured samples of the font.
- A row gets its exact height from the shaped text when it comes on screen, `settle`
  in `view.rs`. It runs after every change and once a frame.
- A row that already has its exact height is the anchor: it keeps its place on the
  screen when rows near it get exact. On a new width the first row on screen is the
  anchor.

What it costs: the arithmetic is right for a line that fits 1 row. A line that wraps at
words can take 1 row more, so the scroll bar is a little off for a part of the log
nobody saw yet. The rows on screen are always exact.

The prefix column is handled the same way. A prefix is shaped only when it has more
chars than every prefix before it, or when it has a char outside ASCII, which may come
from another font.

## Removed lines

`lines_removed` calls `TableView::drop_first_cells`, see [table.md](table.md). No line
is asked, the rows on screen stay where they are, and a selection stays on its text.

## Tests

`Log view wrap`, `Log view colors`, `Log view selection`, `Log view follow`,
`Log view append`, `Log view trim`, `Log view resize` and `Log view big`.
`Log view resize` fails when a new width asks the data for more than the rows on
screen. `Log view big` is also the fixture to play with:

```bash
cargo run -p ui-test --release -- --test-name LogViewBig --present
```

The ANSI parser has unit tests in `ansi.rs`, `cargo test -p hilen --lib log_view`.
