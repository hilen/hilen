# AccentCard

`AccentCard` in `hilen::ui` is a rounded block with a bar of color along 1 side, like an
event in a calendar, a quote or a note with a kind. Use it for every such block. Never
build the bar from a plain `Container` at the edge of a rounded view.

## Why it exists

A child does not take the corner radius of its parent. A bar placed with `tbl(0)` inside
a block with a radius draws its own square corners over the round ones, so the left side
of the block looks square and the right side round. Apps on hilen made this mistake again
and again, so the bar is now a view of the engine with its shape decided in 1 place.

## The shape

There is 1 shape, the one the web gets from a bar with `overflow: hidden`, an inset
`box-shadow` or a gradient fill on a round box:

- The bar lies on the edge of the card over its whole height, 4 points by default.
- The card clips the bar and all its content to its round outline, through
  `clips_to_bounds`. So the outer side of the bar follows the corners and its inner side
  is straight, and the card keeps 1 clean outline.
- A 1 point border, `set_border_color` and `set_border_width(1)`, is the outlined look.
  The bar draws over the border on its side and the border outlines the rest.

These looks are left out on purpose and the view cannot draw them: a bar that stands in
from the edge, a bar with round ends of its own, a gap above or below the bar, a square
side next to a round one, and a thick `border-left` on a round box, which bends the bar
into the corner.

## Use

```rust
let card = self.add_view::<AccentCard>();
card.set_color(fill);
card.set_bar_color(calendar_color);
card.set_corner_radius(6);
let title = card.add_view::<Label>();
title.place().t(3).l(card.content_left() - 8.0).r(4).h(15);
```

- `set_bar_color` takes a `Color` or a `DynamicColor`.
- `set_bar_width(w)` and `set_bar_side(BarSide::Left)` or `BarSide::Right` change the
  bar.
- `content_left()` is the bar width and a gap of 8. Content of a card with the bar on the
  left starts there. A `Label` draws its text 8 points in from its own left edge, so a
  label frame starts 8 points before that, or the label sets `set_text_inset(0)`.
- The fill and the corner radius are the usual `set_color`, `set_corner_radius` and
  `set_corner_radii`, 8 by default. Content that runs past the card is cut at its
  outline, so a long title needs no clip of its own.

## Test

`Accent card test` in `ui-test-suite` shows a bar on the left with a title cut at the
right edge, a bar on the right, a bar of 8 in a card of radius 16, a card 16 points tall,
a card with a 1 point outline and a square card, on a dark ground that shows any corner
the bar is not cut by.
