# Declared loops

A thing that repeats or runs without end, like a spinner, is a declared loop:
a period and a pose of the phase. The engine then knows the loop and can hand
it to the system. The code is `hilen/src/ui/loops/`.

## Why

At 120 Hz on an M1 Pro an app with 1 turning `RingSpinner` used about 44% of a
core in idle, a Tauri copy 0.1%. WebKit hands a CSS rotation to Core Animation
and draws no frame. A Core Animation layer that shows 120 pictures of a ring
in a loop, with a discrete keyframe animation on `contents`, costs its app
0.03% while its main thread sleeps, `tools/layer` in `~/dev/ui-compare`. A
closure animation is code the engine cannot look into, a declared loop is not.

With the spinner as a declared loop the same app used 0.1% of a core at 60 Hz,
24.5% before and Tauri 0.2%, 5 idle runs of 20 s each in 1 session,
`results/2026-10-14T16-10.json` in ui-compare.

## The API

```rust
self.run_loop(LoopAnimation::new(1.0, move |phase| self.pose(phase)));
self.run_loop(LoopAnimation::new(0.6, pose).changes_at([0.25, 0.5]).starting_at(0.5));
self.stop_loop();
self.loop_phase(); // Some(0 to 1) while a loop runs
```

- The pose gets the phase, 0 to 1, and only sets the state of the view and
  its subviews. It is called at any phase, in any order, also more than once
  in 1 frame. A pose at one phase gives the same picture every time.
- Without `changes_at` the picture may change on every refresh of the
  screen. With it the pose is called only at those phases, on both paths,
  and each picture shows until the next change time.
- A new `run_loop` replaces the loop of the view. The phase is counted from
  the time of the frame the loop started in, on `time::frame_seconds`.

## 2 paths

Each frame `loops::step` runs after the layout and puts every loop on 1 of 2
paths.

- The frame path, on every platform. The pose runs once a frame with the
  phase of `frame_seconds`, the view is laid out again, and the loop of the
  app draws on every refresh while the view is on the screen,
  `continuous_render_active`. A hidden or scrolled out view poses nothing.
- The layer path, only on macOS, iOS and tvOS, through Core Animation. The
  engine calls the pose at fixed phases, draws each result once with its own
  pipelines into a picture, and hands the pictures to a `CALayer` over the
  place of the view in the window view. A `CAKeyframeAnimation` on
  `contents` with the discrete mode and the change times as `keyTimes` shows
  them in a loop. The frame then draws nothing of the view and its subviews,
  `ViewBase::loop_layer`, and the loop of the app sleeps.

A switch between the paths keeps the phase: both read it from 1 zero time,
`ViewBase::loop_clock`, and the animation of a layer starts with a
`timeOffset` of the phase at that frame.

## When a layer shows a loop

All of these, checked again in every frame, `fit.rs`:

- A window, no headless run, no `HILEN_LOOP_LAYERS=0` in the env, no
  screenshot waiting, no step or record of the inspector, no debug frames.
- The view is on the screen, at scale 1 and full opacity, in no group fade.
- Every view above it that clips leaves its whole picture, a rounded clip
  only inside its corners, and the picture is inside the window.
- No view outside the loop that is nearer to the viewer paints over its
  picture. A blur over it counts as such a view.
- Its subtree has no blur, scrim, hole in the page, group fade, view that
  draws itself, view that keeps frames like a video, other loop, and no view
  that was never drawn.
- Its content held still for 3 frames in a row, see below.

A picture is the frame of the view grown by the shadows and text effects of
its subtree and 2 pixels of soft edge. A pose keeps what it moves inside the
frame of the view, and a path of a `DrawingView` inside its frame, else the
picture cuts it.

Else the loop is on the frame path. The region of a layer takes no part in
the damage of a frame, `damage/walk.rs` skips the view like a view that is
gone, so its old pixels are drawn over. When it falls back, it is in the walk
again and the frame draws it at the current phase.

## The pictures

`flipbook.rs` draws them: per phase the pose, a layout of the subtree, and a
draw of that subtree alone, `UIDrawer::draw_subtree`, into a clear texture of
the window size with the sample count of the frame. One submit per picture,
then 1 copy out of the GPU for all of them. The pixels are BGRA with
premultiplied alpha in encoded sRGB, the same bytes a frame has, and become
`CGImage`s in the sRGB space, the space of the surface. The compositor blends
the edge pixels, so they can differ from a whole frame by 1 to 2 color
levels, every other view keeps exact pixels.

Without change times a loop gets 1 picture per refresh of the screen, the
rate of the monitor, else the measured period of the frames, else 60, at
most 240 pictures. The pictures of 1 loop may take 64 MB, a loop that needs
more stays on the frame path.

The pictures are drawn again when the content changes. The content is a hash
of the subtree after a pose at phase 0, `key.rs`: the frames from the corner
of the view, colors, borders, radii, opacity, text, images and the shape of
each path, plus the size of the picture, the UI scale and the screen scale.
So a setter, a theme, a scale or a size makes new pictures, a move of the
view moves the layer. A path is known by a hash of its mesh, a pose makes a
new path with new buffers for the same shape. A content has to stay the same
for 3 frames before pictures are drawn, so a fade of the view stays on the
frame path and is drawn once after it. A pose that gives a new content at
the same phase in every frame never reaches a layer and gets 1 warning.

An `AnimatedImage` that loops without end declares the times of its frames
and hands its own frames, `OwnPictures`: the layer stretches the decoded gif
frames over the rect of the image, with no drawing. It keeps the gif file
and decodes it again for the layer, so the decoded frames are in memory only
while a layer shows them. A background, border, round corners, flip, crop
or cut of the image view makes the engine draw the pictures instead.

## Tests and screenshots

A screenshot, `check_colors` and the test runners read the wgpu texture,
which holds nothing a layer shows. So a frame that is read back puts every
loop on the frame path, and the test runner turns layers off before every
test, `set_test_layers(Some(false))`. A test of the layers turns them on.
`hilen-inspect screenshot` of an app with layers draws that 1 frame with the
loops in it.

The test is `Declared loop`, in `ui/loops/test.rs`. On every platform it pins
the pixels of the frame path at fixed phases on stepped time, and that a pose
with change times is called only at them. Where a window has layers, macOS
and the iOS and tvOS simulators, it turns them on and checks that each loop
is on a layer with its animation at the right place, that the app draws at
most 3 frames in 600 ms while only the loops run, that the screen shows the
ring through `screen_ink` where the iOS helper runs, that a move of a view
moves its layer with no new pictures, that a new color makes new pictures,
and that a view put over the ring sends it back to the frame, which draws it
at the right phase. A headless run has no window and checks only the frame
path. Last it checks the warning of a repeating closure animation.

## Warnings

A closure animation is for a short move that ends. Once per view the engine
writes a warning that names the view and `run_loop` when:

- `UIAnimation::repeat()` is added to it.
- A condition of `keep_frames_while` holds for 10 seconds in a row,
  `ENDLESS_SECONDS` in `view_data.rs`. Longer than any transition or glide.

Both keep running on the frame path as before. A playing `VideoView` and a
gif with a loop count end or are no loop, they keep frames with
`keep_frames_exempt` and write nothing.

## Platforms

- Only system classes, `CALayer` and `CAKeyframeAnimation`, so a hot build
  has no class of its own to delete at its stop. At the stop the layers stay
  on the window view and go with it, `Layer::leave_with_view`. With a
  removal there the hot reload lane crashed in about half of its runs, with
  no crash report, and with no removal 10 runs of 6 reloads passed. The
  likely cause is a layout of the view that the removal asks for and that
  runs after the stop in the deleted class of winit, no backtrace proves
  it. The app of `make hot-test` has a turning ring for that. Every call
  exists on iOS 12.
- The root is the layer of the winit view. The layer is added over the
  Metal layer of the surface, with a z of 1000 so a new surface layer stays
  under it. On a Mac the y axis of that layer is learned from
  `convertPointToLayer:` of the view, UIKit layers count from the top. A
  layer that would show its pictures upside down under flipped layers,
  `contentsAreFlipped`, flips itself back.
- Run on 2026-10-14: `Declared loop` in a window on macOS and in the iOS
  simulator lane, an iPhone 8 on iOS 16.4, where `screen_ink` read the ring
  of the layer off the screen in the right box and color. On a Mac the
  place and the flip state of the layer are checked, the pixels on the
  screen are not, the test has no capture of the screen there. tvOS builds
  the same code and was not run, an A7 was not run.

## What is not here yet

Keyframe animations of a position, an angle, a scale or an opacity, which
Core Animation could run with no pictures at all.
