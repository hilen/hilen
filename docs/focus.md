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
- The first key only shows the ring. A pointer that presses hides it, and the next key
  only brings it back. A pointer that only moves hides it too, but the next arrow moves
  the ring on from where it was, a remote with a pointer moves it with a small move of
  the hand. The test is `Key focus pointer`.
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
- A `ContextMenu` panel takes touches for its padding and keeps the ring off itself, the
  ring walks its rows, test `Key focus menu`. Before, the arrow below a row went to the
  middle of the whole panel when a separator stood between the rows.
- `view.set_key_focus(false)` keeps the ring off a view, a backdrop that takes
  touches. `view.set_focus_neighbor(direction, other)` names the next view by hand
  where the nearest one is wrong. `Focus::set(view)` puts the ring somewhere.

Back is Escape. `back_as_escape` in `app_runner.rs` turns the `GoBack` and
`BrowserBack` keys into Escape, and the Back key of an LG remote, key code 461, is read
in `web.rs` before winit sees it. An open `DropDown` list takes Escape first and closes alone,
`DropDown::escape`, the ring goes back to its box, the test is `Drop down escape`. Then
Escape closes a modal that has a cancel result and
pops a pushed `NavigationView` screen. A screen an app swaps in with
`UIManager::set_view` binds Escape itself. While a modal, a menu or another touch layer
is open, a key binding answers only when its view is inside the top layer or holds it,
`TouchStack::key_reaches`, so the Escape of a screen does not leave it under an open
alert. A binding on the root view or on something that is not a view always answers. A key action may swap the screen, the keys
the new screen binds in its `setup` join the keymap after the press, they never see
the press that added them. `Keymap::check` in `input/keymap/keymap.rs` keeps the key
list unborrowed while actions run for that.

## A binding on any keyboard layout

A binding names a Latin key, like `KeyCombo::cmd('b')`, and fires on every layout. A
layout like Russian, Greek or Hebrew types no Latin letters, Cmd with B types `и` there.
So every typed character comes with a second one, `latin_candidate` in
`input/keymap/latin.rs`. The keymap checks the typed character first and the Latin
one only when no binding took the typed one, so a binding on `и` still wins. Qt
matches its shortcuts the same way.

- The Latin letter comes from the system, `latin_key` of the winit fork, see
  [forks.md](forks.md). On Windows it is the virtual key code, on macOS the key with
  Cmd held, on Linux the first other installed layout that types Latin, the walk of
  Qt with its check that no earlier layout types that letter on another key.
- With no answer from the system, on the web, iOS and Android, the place of the key
  on a US keyboard decides.
- A layout that types ASCII keeps its own letters, AZERTY gets no second one and Cmd
  with Z stays on the key labeled Z.
- The digit row always offers its digit, AZERTY types `&` and Lithuanian `ą` on the 1
  key.
- The Latin letter follows the case of the typed one, Shift with `я` gives `Z`.

Text gets only the typed character. Cmd with A, C, X and V of a `TextField` and the
copy of `TextSelection` read the Latin one through `Input::shortcut_char`. A view
that reads the keys itself through `UIEvents::keyboard_input` gets only the typed
character. The test is `Layout shortcuts`, `inject_layout_key` presses a key as a
layout types it.

An edited `TextField` gets a named key before the keymap does when its owner took that
key with `take_keys`, and then nothing else sees the press, no binding and no
`keyboard_key` subscriber. `Input::on_key` asks `TextField::offer_key` for that, see
Keys the owner of a text field takes in [text.md](text.md).

The ring is one of the app views, its frame is counted from where they start, below
the status bar of a phone, the test is `Focus ring place`.

## A view that reads the keys itself

A view like a terminal takes every key. It subscribes to `UIEvents::keyboard_input`
and `UIEvents::keyboard_key`, which fire for every press in the app, and reads the
held modifiers with `Keys::modifiers()`, to tell Ctrl with a letter from the letter.
The keys are meant for it only while it is the selected view: a press on a view with
touch on selects it, `ViewTouch::select` does the same with no press, when its page
opens, and a field that is focused or `UIManager::unselect_view` takes the selection
away. So it checks `is_selected` before it takes a key. `View select` is the test.
A named key the focus ring takes never reaches it, such an app turns the ring off
with `Focus::set_enabled(false)`. The keymap takes Ctrl for Cmd on every system, so a
binding that must not fire on Ctrl checks `Keys::modifiers().super_key()` itself.

The tests are `Key focus`, `Key focus table`, `Key focus modal`, `Key focus long
press`, `Key focus pointer`, `Drop down escape` and `Navigation escape`. The scoring has unit tests in `focus.rs`. The remote
of an Apple TV drives it in the simulator, see [tvos.md](tvos.md). The remote of a
real TV was not tried yet, see [roadmap.md](roadmap.md).
