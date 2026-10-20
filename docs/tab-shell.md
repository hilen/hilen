# Tab shell

`TabShell` is a tab app like a native iOS one: the page of the open tab on top and a bar
of tabs at the bottom. The code is `hilen/src/ui/views/complex/tab_shell/`. How an app
uses it is in the views chapter of the hilen skill.

## The bar

- 49 points high above the home bar, `TAB_BAR_HEIGHT`, the height of an iOS tab bar.
  An icon of 25 points with a label of 10 points under it, 1 entry per tab, each as wide
  as the bar divided by the count.
- A `Tab` names an svg in `assets/images` drawn in black, `#000000`, and the bar tints
  it with `Tinted`. A theme change tints again through `ViewCallbacks::theme_changed`.
- `TabColors` holds 4 theme pairs: the bar, the line on top, the selected tab and the
  others. The default is the iOS system look.

## The strip under the home bar

On a phone the app views end above the home bar, see `RootView::resize_root`, so a
view placed at `b(0)` ends there too and the clear color shows under it. The
background of the bar is a subview of the bar placed with `custom`, after its other
rules, and it adds `UIManager::system_insets().bottom` to its height. It paints past
the bottom of the app views to the edge of the screen. `custom` runs in every layout, so
a turn of the screen gets the new inset. The app views do not clip, only a layer that
holds the screen under a swipe does.

`UIManager::system_insets` is the room between the app views and the screen edges, in
points, from the inner and the outer size the window reports. It is 0 on desktop, in a
browser, on a TV and on a test canvas.

## Pages

`set_tabs` takes the tabs and a builder that adds the page of a tab to the container it
gets. `select` builds the page of a tab, drops the page before, marks the tab and fires
`changed`. `show_page` puts a page the app built in `content()` in place of the open one,
the mark stays, like a detail page. A call from a view inside the open page waits 1
frame, that view dies in the swap.

## Tests

- `Tab shell test` in `ui-test-suite` taps tabs, checks the marks, the order of
  `changed` and a detail page, with recorded colors.
- `Tab shell screen edge` in `hilen/src/ui/tests/base` drops the canvas and checks the
  bottom row of the real screen and the middle of the home bar strip have the color of
  the bar. In the iPhone 8 lane there is no home bar, the strip is proven on a simulator
  with one through `make swap` and `hilen-inspect run-tests`.
