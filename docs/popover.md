# Popover

`Popover` is a panel with any view in it, tied to the view that opened it. The code is
`hilen/src/ui/views/controls/popover.rs`.

```rust
let content = MyCard::new();
content.set_size(260, 120);
Popover::show(content, button);
```

- The content comes sized. A popover keeps the size of the view. A sheet keeps its
  height and gives it the width of the window, so its children are laid out with `lr()`
  and anchors.
- On a medium or wide window it opens under the anchor, with an arrow at the middle of
  the anchor. With no room under the anchor it opens over it. Across, it slides back
  inside the window and the arrow still points at the anchor, kept clear of the round
  corners.
- On a narrow window, `UIManager::width_class()` below 600 points, it is a sheet: as
  wide as the window, at its bottom, round only at its top corners, over a scrim, with a
  grab bar at the top. It slides up in 0.25 seconds. A drag down moves it with the
  finger, a drag past a third of its height closes it, a shorter one puts it back. It
  stays clear of the bottom safe area of a phone and of the top 40 points of the window.
- A press outside, Escape or `Popover::dismiss_open()` closes it. A change of the width
  class closes it too, a sheet and a popover lay out differently. `closed` fires once on
  every way out. Only 1 is open, a new one closes the old one.
- `PopoverStyle` holds the look: background, border, corner radius, arrow, shadow, the
  scrim and the grab bar of a sheet. `PopoverStyle { background, ..default() }.show(...)`
  opens with an app's colors.
- It has its own layer, `POPOVER_Z_OFFSET` 0.37, over a modal and under a context menu,
  so a menu opened from inside it shows in front of it, see [depth.md](depth.md).

## Tests

`Popover anchor` opens it over a button in a bar at the bottom and under a button at the
top, checks the arrow side, a tap inside, a tap outside and Escape. `Popover sheet` runs
on a narrow canvas of 400 by 700 and checks the slide, a short drag, a long drag, a tap
on the scrim and Escape.
