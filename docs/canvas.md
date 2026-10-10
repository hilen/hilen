# Canvas, scale of a subtree and pinch

`CanvasView` pans and zooms what is inside it, the way `ScrollView` scrolls. Its subviews
are laid out as in any view, in its own points at zoom 1. The code is
`hilen/src/ui/views/containers/canvas/canvas_view.rs`.

## What an app calls

- `zoom()`, `offset()`, `set_zoom`, `set_offset`, `set_zoom_limits(min, max)`, 0.1 and 10
  at the start.
- `zoom_at(point, factor)` zooms and keeps what is under `point` in place. `point` is in
  the points of the canvas.
- `content_point(point)` gives the point of the subviews drawn at a point of the canvas.
- The event `on_change` fires when the zoom or the offset changes.

A drag on the empty area pans. The wheel zooms at the cursor. A pinch zooms at the
fingers and pans with them. The canvas clips its subviews to its frame.

## How a subtree is scaled

Nothing is stretched. A view has 3 numbers in `ViewBase`:

- `content_scale` and `content_shift`, how much bigger a view draws its subviews and where
  their zero point sits. Only a `CanvasView` sets them.
- `tree_scale`, the `content_scale` of every view above multiplied, set in
  `calculate_absolute_frame` on each update.

The absolute frame stays in screen points, so it is the frame times `tree_scale`. Hit
tests, clipping, hover and the focus ring read it as before.

Every drawer takes a rect in points and a scale, and draws at points times scale. So the
drawer hands a scaled view its frame in its own points, `ViewBase::draw_frame`, and the
screen scale times `tree_scale`. The corner radius, the border, the text size and the
stroke of a `DrawingView` path are given in points, so they all grow with the view and
are drawn at the new size. With `tree_scale` 1 the numbers are the same bits as before.

A touch reaches a view in its own points, `ViewBase::local_point` divides by `tree_scale`.

## A flat layer

A later sibling draws behind the subviews of an earlier one, the depth of a subview is
nearer than the depth of its parent's sibling. For a map with controls over it that is
wrong, the map would draw over the controls.

A view with `ViewBase::flat_depth` counts as one flat layer at its own depth.
`CanvasView` and `VideoView` set it, and it needs a view that clips.

- Drawing: after the subtree is drawn, `UIClipPipeline::flatten` writes the depth of the
  view over its clip shape, with no color. A view drawn later is nearer than that depth
  and covers all of it.
- Input: `depth_key` in `view_frame.rs` orders views by the depth of the outermost flat
  layer they are inside, then by their own. Hover, the wheel and the pinch pick the
  front view with it. A began touch skips a view inside a flat layer when another touch
  view lies over that layer at the point, `TouchStack::covered`.

## Pinch

`view.enable_pinch()` and `view.touch().pinch` give a `Pinch`: `scale`, the step to
multiply a zoom by, `center` and `shift`, how far the center moved, both in the points of
the view. The frontmost enabled view under the pinch gets it and keeps it until the
pinch ends. The code is `hilen/src/ui/input/pinch.rs`.

- 2 fingers: `PinchInput` follows every finger by touch id. When a second finger lands
  and a pinch view is under the middle, the pinch begins. Both fingers leave the views
  that captured them, so the release is no tap, and no view sees them move.
- A finger held by a view outside the pinch view is busy there. 2 thumbs on 2 buttons
  over a map stay 2 taps.
- A trackpad on macOS: winit's `PinchGesture`, at the cursor, `shift` is 0.
- A browser has no pinch event. It reports a wheel turn with Ctrl held,
  `Input::on_wheel` turns that into a pinch step on wasm only.
- iOS: the 2 fingers arrive as touches. The pinch recognizer of winit stays off, it
  would report the same pinch a second time.

In a UI test a pinch of 2 fingers is `inject_touches` with finger ids, and a trackpad
pinch is `inject_pinch(x, y, scale)`. `Canvas zoom` and `Canvas pinch` are the tests.

What is still open is in [roadmap.md](roadmap.md).
