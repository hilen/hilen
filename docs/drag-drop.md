# Drag and drop

`Drag` in `hilen/src/ui/drag_drop.rs` moves something from 1 view to another, like a
card of a board to another column.

- `Drag::source(view, start)` makes a view one that can be picked up. `start` runs at
  the lift and gives a `DragStart` with the payload and a picture, or `None` for no
  drag. The picture is a new view, filled before it is added, so a view with `#[init]`
  views fills them from a data field in its `setup`. `on_end` hears whether it was
  dropped.
- `Drag::target::<T>(view, on)` takes the drags whose payload is a `T`. `on` gets
  `DropEvent::Enter`, `Over`, `Leave` and `Drop`, with the point in the points of the
  target. Of 2 targets under the pointer the one in front wins.
- The lift: a mouse lifts after a move of 10 points, a finger after a hold of 0.4
  seconds with no move, so a quick finger drag still scrolls. `Touch::source` tells them
  apart, `TouchSource::Mouse` and `Finger` from the window events, also in a browser,
  where winit sends a pointer of type `touch` as a touch. A touch the engine or a test
  makes is `Unknown`, and goes by `UIManager::drag_scrolling` as before.
- The lift takes the touch from the views and the scrolls, so the release is no tap.
  The picture rides in its own layer at z `0.335`, in front of a modal.
- Escape, a screen that goes away and a swipe back put the drag back, with `Leave` and
  `on_end(false)`.
- A scroll view around the target scrolls while the pointer is within 48 points of its
  edge, 1 step a frame from 1 timer, so the speed does not follow the pointer events.
- `Input` offers every touch to `Drag::swallow` right after `RowDrag`, the row move of a
  table, which keeps its own rule.

`Drag drop test` pins it: a quick press stays a tap, a mouse lift, a target of another
type, a drop on nothing, Escape, a finger that moves before its hold, a finger hold with
the scroll at the edge, and a mouse lift where a drag scrolls.
