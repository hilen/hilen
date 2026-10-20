# Side swipe

A swipe left or right over any view, also over a scroll view that moves up and down,
like the pages of a native iOS app. banda opens the session before or after the open
one with it. The code is `hilen/src/ui/side_swipe.rs`, the test is
`side_swipe_test.rs` next to it.

## The call

`SideSwipe::set(view, on)`. `on` gets a `SideSwipeEvent`:

- `Moved(offset)`: the finger is `offset` points right of where it came down, less
  than 0 to the left. The engine moves nothing, the view moves its own content.
- `Ended(end)`: the finger is up. `end` is `Left` or `Right` for a release past half
  the width of the view or a last step of 8 points or more, a fling, and `Back` for
  any other release.

A second `set` on the same view replaces the first. The last view set under the finger
gets the swipe.

## The gesture

- A finger that comes down on the view is watched. The views get the touch as usual,
  so a tap still taps and a scroll still scrolls.
- A move of 10 points sideways, more sideways than down, starts the swipe. The views
  and the scroll that caught the finger let it go, `TouchStack::cancel_touch` and
  `release_scrolls`, so the release is no tap and no scroll. A move of 10 points down
  first ends the watch.
- A finger that comes down on a view that moves sideways by itself starts no swipe:
  a `ScrollView` with room to the side, a `CanvasView` and a `SceneView`. They answer
  `__moves_sideways` of `Scrollable`.
- `Input` asks `SwipeBack` first, so a finger at the left edge of a screen that moves
  right is the swipe back.
- It runs where `UIManager::drag_scrolling` is on, never on tvOS and not while a modal
  or another overlay is open.

## Not proven yet

The test runs on the desktop lane with drag scrolling on. On a real iPhone the gesture
was tried through banda on 2026-10-19, but not a swipe that changed the page: banda
slides the next page in from the `on_finish` of the slide out, and that froze every app
until 2026-10-20, `commit_animations` held the list of animations locked while it ran
`on_finish`. `SideSwipeSlide` swipes and slides like banda, `AnimationFromFinish`
starts an animation from `on_finish`. Both run in the smoke set and pass on the
desktop and the iOS simulator lane. A sideways `ScrollView` inside a `ScrollView` that
moves up and down did not scroll under a finger in a first draft of the test, which
is likely the outer scroll that takes the finger first. That case is not checked.
