# Depth

Which view is in front comes from the tree order. A later sibling and all it holds is in
front of an earlier sibling and all it holds, at any depth of the tree. So the last
child of a screen, like the mark of a file held over the window, is in front of
everything on that screen, the cells of a table and its sticky header included, with no
number to pick. The code is `hilen/src/ui/view/depth_order.rs`.

## How the depth is set

Each view has a z from 0 to 1, smaller is nearer, and the drawer tests it against the
depth buffer. `depth_order::order` walks the tree in order and gives each view a z
`DEPTH_STEP`, 4e-7, nearer than the view before it. It runs after the update walk of a
frame, only in a frame after the tree changed: a view added or removed, `subviews_mut`,
a raise, a float or a `set_z_position`. `add_view` still gives a first z by the depth in
the tree, the walk sets the real one before the frame is drawn.

Text draws 1e-7 in front of its label, so the step between 2 views is bigger than that.
The depth buffer has 24 bits, a step is about 6 of its units.

## Layers

`set_z_position` starts a layer at that z, over the whole window. Its views are ordered
inside it the same way. The engine layers, nearest last:

- the root tree from 0.5, room for 250000 views,
- a modal at 0.4, each modal over it 0.005 nearer, room for 12500 views each,
- the context menu at 0.35, the toast and the tooltip at 0.34, the focus ring at 0.33,
- debug views at 0.3.

An app uses `set_z_position` only for a layer of its own over everything, like a toast.

## Inside the engine

`bump_z_position` and `draw_on_top` are gone, an app puts a view in front by adding it
later. The engine has 3 marks for its own controls, all in `depth_order`:

- `set_raise(view, n)` orders siblings only, a higher raise is in front of its siblings
  and all they hold, never in front of a view outside its superview. The scroll bars,
  the split handles, a pinned row, the sticky header and a held row use it. A negative
  raise goes behind the siblings, like the selection of a text field under the glyphs.
- `set_floats(view, true)` puts a view and all it holds in front of everything else of
  its layer, a modal over it stays in front. The open panel of a drop down uses it.
- `set_same_depth(view)` puts a view and all it holds at the depth of its superview,
  like the picture of a label under its glyphs.

Hover, the wheel and the pinch pick the view in front by its z, see `depth_key`. A touch
does not, it follows the order the views turned their touch on, see
[canvas.md](canvas.md).

A hover view gets no hover when a touch view that is not inside it lies in front of it
at the pointer, see `TouchStack::hover_view_at`. The rows of an open drop down take
taps and no hover, and the field under the list lit up through them. A button inside a
hoverable row is inside it, so the row keeps its hover. `Drop down hover under` tests it.

## Test

`Overlay over table` holds a dark overlay as the last child over a table that is
scrolled under its stuck header. The overlay has no raise and covers the header and
every cell.
