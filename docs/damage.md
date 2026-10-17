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
- A scene or a level runs over the whole root area, and the frame after it ends. A
  scene in a `SceneView` draws into the image of the view, so only the rect of the
  view is drawn again, the view keeps its frames coming while it holds the scene.
- The default font or the fallback fonts changed, `damage::whole_next` asks for it.
- Debug frames are on.
- More than half of the window changed.
- The size of the window or the clear color changed, or the clear color is not solid.
- `HILEN_DAMAGE=0` is in the env. This is the A/B lever, and the way around a view
  that shows stale pixels.
- An iPhone draws with more than 1 sample. It loses the frame kept in the
  multisampled texture, see the roadmap. iOS draws 1 sample by default, so this
  happens only when an app sets more.

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
- A view whose loop a layer of the system shows, `ViewBase::loop_layer`, is not in
  the frame. The walk skips it like a view that is gone, see [loops.md](loops.md).
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

The default is 4, and 1 on iOS and in a browser on WebGL. An iPhone screen is 3x, so
the rougher edges are hard to see there, and the frame before is kept only with 1
sample, see above.

## The simulator is no proof

A change of how a frame is drawn, kept or shown is done only after it ran on a real
iPhone. The sample count, the damage, the surface, the scene texture and the passes are
such changes. The simulator draws with the GPU and the driver of the Mac, not those of
the phone, so it shows nothing about them.

This rule comes from 2026-10-17. Partial frames came in on 2026-10-12, proven on the Mac
and in the simulator, and went to App Review in skaityk 3.0.13. On an iPhone 16 Pro Max
with iOS 27.0.1 the app flashed black and showed old screens. With the same code, 4
samples and the old rule, the test `Frames over a flow` failed on the phone and passed
in the simulator. That the env reached the simulator app was checked: `HILEN_MSAA=3`
stopped it.

The bug did not come every time. In the morning the test failed in 6 of 6 runs that
reported, in the afternoon it passed 20 runs in a row with the same settings. The
failing runs were in a loader process that had lived for hours with several apps
swapped into it. Memory is the lead, it is not proven. So a pass of the phone lane
shows that nothing is broken in that run, not that the bug is gone. The default of 1
sample does not depend on it, that path kept the frame in every run.

The phone lane is in [ui-tests.md](ui-tests.md), under "On a phone".

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
