# Key focus

A TV remote and a keyboard drive the UI with the arrow keys. The code is
`hilen/src/ui/focus.rs`. An app writes nothing for it.

- An arrow key moves a ring to the nearest view in that direction. Enter taps the view
  under the ring, with a began and an ended touch at its middle, so every view that
  takes a tap works.
- The views that can hold the ring are the touch views of the top touch layer. A
  `TableView` takes the taps of its cells itself, so each visible cell is a place of
  its own, known by its index, the cell views recycle.
- The first key only shows the ring. A pointer that moves or presses hides it.
- The gap along the move counts once, the gap across it 4 times, so a view straight
  ahead wins over a nearer one that sits diagonally.
- A move with nothing ahead scrolls the scroll view the ring is in and looks again for
  4 frames, a table makes its new rows after the scroll. A view the ring lands on is
  scrolled fully into its scroll view.
- A modal or a pushed screen is a touch layer. The ring moves into it by itself, and
  goes back to the view it was on when the layer closes. A removed view keeps the link
  to its parent for one frame, so the check asks every parent whether it still holds
  the view.
- The focus is off while a text field is edited, while the mouse is captured, and
  while a level or a scene runs. `Focus::set_enabled(false)` turns it off for a screen
  that uses the arrows itself, a video player that seeks.
- `view.set_key_focus(false)` keeps the ring off a view, a backdrop that takes
  touches. `view.set_focus_neighbor(direction, other)` names the next view by hand
  where the nearest one is wrong. `Focus::set(view)` puts the ring somewhere.

Back is Escape. `back_as_escape` in `app_runner.rs` turns the `GoBack` and
`BrowserBack` keys into Escape, and the Back key of an LG remote, key code 461, is read
in `web.rs` before winit sees it. Escape closes a modal that has a cancel result and
pops a pushed `NavigationView` screen. A screen an app swaps in with
`UIManager::set_view` binds Escape itself. A key action may swap the screen, the keys
the new screen binds in its `setup` join the keymap after the press, they never see
the press that added them. `Keymap::check` in `input/keymap/keymap.rs` keeps the key
list unborrowed while actions run for that.

The tests are `Key focus`, `Key focus table`, `Key focus modal` and `Navigation
escape`. The scoring has unit tests in `focus.rs`. The remote of a real TV was not
tried yet, see [roadmap.md](roadmap.md).
