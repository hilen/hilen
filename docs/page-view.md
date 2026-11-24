# Page view

`PageView` is a row of pages that a finger turns sideways, like `UIPageViewController`
with the scroll style. The code is `hilen/src/ui/views/containers/page_view/`, the tests
are `ui-test-suite/src/views/containers/page_view/`, and the `Pages` screen of the demo
shows it.

## The calls

An app gives it a `PageSource`, keyed by a string per page:

- `page_beside(key, forward)` is the key after or before `key`, `None` at an end.
- `make_page(key)` makes the view of a page.
- `page_made(key, view)` comes after the view is added and set up, so it takes its
  data here. `page_dropped(key, view)` comes right before a view goes, the last moment
  to keep what it holds. Both only log when the app does not write them.

`set_page(key)` opens a page with no move, and keeps the page that is already open.
`turn_to(key, forward)` opens one with the move of a turn. `clear()` drops every page.
`rename_page(key)` gives the open page a new key and keeps its view, for a page that
got its key late. `offset()` is how far right of its place the open page stands, and
`set_spacing` is the gap between 2 pages. `will_turn` tells the key of a page the pages
move toward, and `turned` tells the open page once the pages rest, with `completed`
false when they went back to the page they started from.

## 1 strip

The pages lie in 1 strip, and the page nearest the middle is the open one at every
moment, also while a finger drags or the pages glide. When they pass half a page the
page beside becomes the open one and its next neighbor is made. So a swipe on pages
that still glide goes on to the page after the next one at once. Before 2026-11-24 the
open page changed only at the end of a glide, and a fast second swipe pulled into
empty space beside the last made page.

A page is made when the pages first move toward it. Nothing is dropped while the pages
move, so a finger that goes back and forth over the middle makes no page twice. Once
they rest, the pages beside the open one go.

## Measured on UIKit

Every move was measured on a real `UIPageViewController` in the iOS 26.3 simulator on
2026-11-24, with the touches of `make sim` and the app in `uikit-reference/`, which
logs every touch and every frame of the pages:

- The pages follow the finger 1 to 1 after it moved 10 points sideways, with no jump.
  UIKit started its follow at 10 to 22.67 points in different gestures.
- Past the first and the last page they pull like the rubber band of iOS,
  `(1 - 1 / (past * 0.55 / width + 1)) * width`, which fit within 0.12 points.
- A release turns the page past half the width, or with a flick of 300 points a
  second or more. Every flick that turned measured 386 or more, every one that did not
  240 or less. The speed is taken over the moves of the last 50 ms before the last
  move, and is 0 when the finger rested longer before it lifted.
- The speed only picks the page. The move into place is always a spring from rest with
  a stiffness of 16.5 and a damping of 0.9, starting 1 frame after the release, done
  when the pages are half a point from their place. It fit every measured release
  within 0.85 points rms.

On the demo all 16 measured gestures ended on the same page as UIKit. A return of 140
points, recorded frame by frame with `hilen-inspect record`, stayed within 1.38 points
of UIKit. Not measured: a finger on gliding pages, since XCTest waits for the app to
stand still before a gesture. The pages stop under it the way a scroll view of iOS
stops. A turn by code uses the same spring, not compared with UIKit.

## Input

A finger turns the pages through `SideSwipe`, where `UIManager::drag_scrolling` is on.
`SideSwipe::set_held` tells the page view when a finger comes down on it, so it stops
gliding pages, and when that finger goes with no swipe. A sideways wheel or trackpad
scroll moves the pages like a finger everywhere, and they go into place 0.12 seconds
after the last turn of the wheel, since the end of a trackpad gesture never reaches a
view.
