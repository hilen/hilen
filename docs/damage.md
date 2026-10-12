# Partial frames and the sample count

How a frame draws only what changed, and how an app picks the samples per pixel. Both
are about the cost of a frame that changes little, like a spinner of 20 points in a
full window. The code is `hilen/src/ui/damage/`, `hilen/src/ui/ui_drawer/frame.rs`,
`hilen/src/window/state/kept.rs` and `hilen/src/window/msaa.rs`.

"Damage" is the usual word in graphics for the part of the screen that is out of date
and has to be drawn again.

## What a frame does

1. `damage::find` walks every view that is drawn, in the order `UIDrawer::draw_view`
   visits them. For each one it makes a key, a hash of everything the drawer reads to
   draw that view, and the rect of pixels the view can touch.
2. A view whose key or rect differs from the frame before adds its old and its new
   rect to the damage. A view that is gone adds its old rect.
3. Rects closer than 24 pixels become 1 region. More than 12 regions become 1 rect
   around all of them.
4. The pass keeps the color of the frame before and clears depth and stencil. For each
   region the drawer sets the scissor to it, paints the clear color over it, and draws
   every view that has pixels in it.

The result is the same pixels as a whole frame. The test `Partial frames` compares
them pixel for pixel after each of 10 small changes.

## When the whole frame is drawn

- A view that was never drawn is on the screen. Its first draw tells whether it has a
  `before_render` of its own.
- A view with a `before_render` of its own is on the screen. It draws what the engine
  cannot see.
- A blur, a scrim, a group fade, or a hole in the page of a browser is on the screen.
- A scene or a level runs, and the frame after it ends.
- The default font or the fallback fonts changed, `damage::whole_next` asks for it.
- Debug frames are on.
- More than half of the window changed.
- The size of the window or the clear color changed, or the clear color is not solid.
- `HILEN_DAMAGE=0` is in the env. This is the A/B lever, and the way around a view
  that shows stale pixels.

## Where the frame before is kept

- With several samples it is in the multisampled texture, which is the same in every
  frame. It resolves whole into the target.
- With 1 sample the frame is drawn into the scene texture and copied to the surface,
  see `keeps_through_scene`. A browser does not do this, there 1 sample is the path
  of weak devices and every frame is whole.

## Rules for new code

- A value that `draw_view` or one of its helpers reads has to be in the key, in
  `Walk::key`, `Label::feed_paint` or `PathData::feed_paint`. A value that is left out
  keeps its old pixels until the next whole frame.
- A view that paints outside its frame has to say so in `damage::paint_rect`, like a
  shadow and a text outline do. A `DrawingView` counts as the whole clip it is in.
- A setting that no view holds and many views draw with calls `damage::whole_next`.
- A view that changes its pixels with no change of what it holds, like a video, uses
  `keep_frames_while`. Such a view and its subviews count as changed in every frame.
- A view that is not drawn in a region still takes part in the flushes: its text and
  its pictures are noted, and a clip outside the region flushes like one inside. The
  order of the draws is so the same as in a whole frame, and with it every pixel.

## The sample count

`set_msaa_sample_count(1, 2 or 4)` sets the samples per pixel, the smoothing of edges
that a game calls MSAA. Before the window is up it is the count everything is first
built for. In a running app it drops every pipeline, the brush of every font and the
textures of the frame, and each is made again for the new count. The test
`Sample count switch` goes from 4 to 1 and back.

- Rects, text and images are smoothed in the shader or in the glyph picture. They
  look the same with 1 sample.
- Vector paths, polygons and sprite cutouts get rougher edges with 1 sample.
- A pipeline of an app's own, and a `ShaderLab` view, are not made again by a switch.

`HILEN_MSAA` in the env is only the count a run starts with, for tests and benchmarks.

## What was measured

On an M1 Pro, in the banda app with a mark of 18 points moving at 60 frames per
second, on 2026-10-12:

- Main thread busy share: 17.8% with whole frames, 9.3% with partial frames.
- GPU load of the mac above idle: about 15 points with 4 samples, about 7 points with
  1 sample. Partial frames do not change it. Half of the GPU cost of a frame is the
  4 samples of the whole window, the other half is showing a new window image.

So partial frames save CPU work, and the sample count saves GPU work.

A frame that changes little still pays for 3 walks over all views, the update, the
compare and the draw, and for the fixed work of wgpu. What is left to do is in
[roadmap.md](roadmap.md).
