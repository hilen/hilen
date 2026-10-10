# Key focus

A TV remote and a keyboard drive the UI with the arrow keys. The code is
`hilen/src/ui/focus.rs`. An app writes nothing for it.

- An arrow key moves a ring to the nearest view in that direction. Enter taps the view
  under the ring, with a began touch at its middle when the key goes down and the ended
  one when it comes up, so every view that takes a tap works.
- Enter held for half a second is a long press, like a held finger. It fires
  `secondary` on the view under the ring, on a table with the position of the cell, and
  its release is no tap. The repeats of a held key are ignored. `Input::on_key_up`
  carries the release, and `hilen-inspect hold Enter --ms 800` takes the same path.
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
  while a level or a scene runs that takes the arrows itself, a game. A level or a
  scene that only draws behind the views answers false in `takes_keys` of its
  `LevelSetup` or `SceneSetup`, true by default, and the ring then works over it. The
  tests are `Level leaves keys` and `Scene leaves keys`. `Focus::set_enabled(false)`
  turns it off for a screen that uses the arrows itself, a video player that seeks.
- A view that walks its own rows with the arrows holds the keys with
  `Focus::hold_keys`, the focus is off until `Focus::release_keys`, or until that
  view is hidden or under another touch layer. `FileBrowser` does it after a press
  on its list, see [file-browser.md](file-browser.md).
- `view.set_key_focus(false)` keeps the ring off a view, a backdrop that takes
  touches. `view.set_focus_neighbor(direction, other)` names the next view by hand
  where the nearest one is wrong. `Focus::set(view)` puts the ring somewhere.

Back is Escape. `back_as_escape` in `app_runner.rs` turns the `GoBack` and
`BrowserBack` keys into Escape, and the Back key of an LG remote, key code 461, is read
in `web.rs` before winit sees it. Escape closes a modal that has a cancel result and
pops a pushed `NavigationView` screen. A screen an app swaps in with
`UIManager::set_view` binds Escape itself. While a modal, a menu or another touch layer
is open, a key binding answers only when its view is inside the top layer or holds it,
`TouchStack::key_reaches`, so the Escape of a screen does not leave it under an open
alert. A binding on the root view or on something that is not a view always answers. A key action may swap the screen, the keys
the new screen binds in its `setup` join the keymap after the press, they never see
the press that added them. `Keymap::check` in `input/keymap/keymap.rs` keeps the key
list unborrowed while actions run for that.

An edited `TextField` gets a named key before the keymap does when its owner took that
key with `take_keys`, and then nothing else sees the press, no binding and no
`keyboard_key` subscriber. `Input::on_key` asks `TextField::offer_key` for that, see
Keys the owner of a text field takes in [text.md](text.md).

The ring is one of the app views, its frame is counted from where they start, below
the status bar of a phone, the test is `Focus ring place`.

The tests are `Key focus`, `Key focus table`, `Key focus modal`, `Key focus long
press` and `Navigation escape`. The scoring has unit tests in `focus.rs`. The remote
of an Apple TV drives it in the simulator, see [tvos.md](tvos.md). The remote of a
real TV was not tried yet, see [roadmap.md](roadmap.md).
