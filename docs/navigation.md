# Navigation

`NavigationView` is a stack of pages. The code is `hilen/src/ui/navigation_view.rs`,
the tests are `navigation_swipe_test.rs` and `navigation_shell_swipe_test.rs` next to
it.

## The rule

A page that has a way back is a pushed page, and the stack owns every way back of it.
So no page can go back by a tap and not by a swipe.

- `nav.push(page)` puts a page on top. The page below stays alive and hidden, so it
  keeps its scroll place and its state.
- `nav.pop()` drops the top page and shows the one below. It is the only way back.
- The swipe from the left edge, a tap on the `BackButton`, Escape, and the Back key
  of Android or a remote all call `pop`.
- A push and a pop are instant. Only the swipe moves the pages.
- `UIManager::set_view` stays for a switch with no way back, like the login to the
  main screen.

## When a page leaves and when it shows again

A page that is pushed keeps living under the next push, so it does not get a new
`setup` when it is on top again. 2 calls of `ViewController` tell a page about it:

- `self.on_pop(action)` runs `action` when the pushed page leaves its stack, by any of
  the 4 ways back. The page is still alive while it runs. The first page of a stack
  never leaves, the call panics there.
- `self.on_return(action)` runs `action` each time the page is on top again, after the
  page over it left. A page reloads here what the page over it may have changed.

Both run in `drop_top`, which every way back ends in. A swipe that slides back never
reaches it, so it runs neither. A switch with `UIManager::set_view` drops every page
with no pop, so it runs neither too. `NavigationEvents` in `navigation_events_test.rs`
counts both for Escape, the back button, a full swipe and a swipe let go before half.

## The back button

The stack makes a `BackButton` for each pushed page. A page gets it with
`self.back_button()` of `ViewController` and places it. The look is free: a color, a
corner radius, and any views inside it that take no touches, like an icon or a text.
A tap on it pops the page.

- A `BackButton` the stack did not make panics in its setup, with the name of the
  call to use.
- The first page of a stack has nothing to go back to, `back_button()` panics there.

## A stack in a shell

`TabShell` puts the page of each tab in a stack of its own, see
[tab-shell.md](tab-shell.md). A detail page is pushed on
`shell.navigation()`. The swipe moves only the pages of that stack, the tab bar stays.
A stack can sit in any view, so a sidebar app puts one in its page container.

## The gesture

- A finger that comes down within 24 points of the left edge of a stack with 2 pages
  or more is watched. The innermost stack under the finger takes it, and only a stack
  in the top touch layer, so not one under a modal. The views get the touch as usual,
  so a tap there still taps.
- A move of 10 points to the right, more right than down, starts the swipe. The views
  that caught the finger let it go, `TouchStack::cancel_touch` and `release_scrolls`,
  so the release is no tap and no scroll. A move of 10 points down first, or to the
  left, ends the watch.
- The page below shows at once, 30 percent of the width left of its place. The top
  page follows the finger, the one below slides in slower.
- A release past half the width finishes, and so does a last move of 8 points or more
  to the right, a fling. Any other release slides back. The slide holds `Touch::lock`.
- It runs where `UIManager::drag_scrolling` is on, never in the browser or on tvOS.
  `Input` asks the stack before `SideSwipe`.

## How the pages are drawn

Each page sits in a `PageHolder`. A later sibling draws behind the subviews of an
earlier one, so while a swipe shows the page below, its holder is a flat layer that
clips, see "A flat layer" in [canvas.md](canvas.md). The top page drawn after it covers
all of it.

- A page with no background of its own shows the clear color of the window. While a
  swipe runs, the holder of the top page paints that color, else the page below would
  show through it. After the swipe it is clear again, so a page over the background
  image of the root looks as before.
- A holder is placed with `back`, and a `custom` rule adds its `shift`, how far the
  swipe moved it. A rule with `back` alone put a held page back at 0 on every layout.
- A page ends exactly at 0, a page left a fraction of a point off draws its text
  blurred.

## Not proven yet

The tests run on the desktop lane with drag scrolling on. A real iPhone and the
Android system back, which the engine turns into Escape, are not checked.
