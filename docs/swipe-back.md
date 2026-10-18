# Swipe back

A swipe from the left edge leaves a screen, like the back swipe of a native iOS app.
The code is `hilen/src/ui/swipe_back.rs`, the test is `swipe_back_test.rs` next to it.

## The calls

- `SwipeBack::set(screen, build)`: `build` makes the screen below, the same one the back
  arrow and Escape of `screen` set with `UIManager::set_view`. It only builds, it also
  runs for a swipe that goes back to its start.
- `SwipeBack::set_with_done(screen, build, done)`: `done` runs after the screen below
  became the app views, never for a swipe that went back. It is for what the back
  function does besides building, like `Focus::set_enabled(true)`.

Only a screen set with `UIManager::set_view` swipes, the one on top of the app views. A
page inside a shell with a sidebar does not. The screen below is new, it restores its own
scroll place.

## The gesture

- A finger that comes down within 24 points of the left edge is watched. The views get
  the touch as usual, so a tap there still taps.
- A move of 10 points to the right, more right than down, starts the swipe. The views
  that caught the finger let it go, `TouchStack::cancel_touch` and `release_scrolls`, so
  the release is no tap and no scroll. A move of 10 points down first, or to the left,
  ends the watch.
- The screen below is built at once. The app views follow the finger, the screen below
  starts 30 percent of the width left of its place and slides in slower.
- A release past half the width finishes, and so does a last move of 8 points or more to
  the right, a fling. Any other release slides back. The slide holds `Touch::lock`.
- It runs where `UIManager::drag_scrolling` is on, never in the browser or on tvOS, and
  not while a modal or another overlay is open.

## The 2 layers of the root

`RootView` holds the app views in `screen` and the screen under a swipe in `under`, both a
`ScreenLayer`. `under` is added first, so it is drawn first, and has the depth of the
background, 1 sibling step farther than `screen`, which keeps the depth the app views had
before. A later sibling draws behind the subviews of an earlier one, so while it holds a
screen `under` is a flat layer that clips, see "A flat layer" in [canvas.md](canvas.md).
Then the app views drawn after it cover all of it. At any other time it is empty, not
flat and does not clip, so nothing changes for an app outside a swipe.

A finished swipe swaps the 2 layers, their depths with `bump_z_position` and their order
in the subviews of the root. No view is added again, so no screen runs its setup twice.
`clear_root` ends a swipe that runs, a screen set during the slide is not touched by its
end.

## Not proven yet

The test runs on the desktop lane with drag scrolling on. A real iPhone and the Android
system back, which the engine turns into Escape, are not checked.
