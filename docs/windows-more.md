# More than 1 window

`AppWindow` opens a window next to the main one, on a desktop and in a headless run. A
phone and a page have 1 window and answer `AppWindow::open` with an error. The code is
`hilen/src/window/extra/` and `window/slots.rs`.

```rust
let window = AppWindow::open(
    WindowSpec {
        title:       "Picker".into(),
        place:       WindowPlace::Frame((100, 100, 400, 300).into()),
        level:       WindowLevel::Normal,
        decorations: true,
    },
    || PickerView::new(),
)?;
```

- The view `make_root` gives is the content. It is made once the window is up: on a
  desktop in the next turn of the loop, since winit makes a window only inside its
  handler, headless at once. Build its parts in `setup`, a view that turns on touch
  before it is in a window finds no touch stack.
- `WindowPlace::Cover(display)` covers 1 screen of `Display::all()`. `WindowLevel::AboveAll`
  puts it over every window, the menu bar and the dock.
- `close` frees the views at the next frame, so a button of the window can close it
  from its own action. `within(f)` runs `f` as an event of that window, so a modal lands
  there. `take_screenshot` reads its next frame.

## Window slots

The main window is slot 0, an extra window takes a free slot, 8 at most. The engine
works on 1 window at a time: an event of a window and its frame run with its slot
current, everything else with the main one.

- Each slot has its own root view, `Window` with its surface and frame state, touch
  stack, hover, focus ring, text selection, damage, open menus, cursor and scale. The
  state of 1 window is a `WindowLock<T>`, a `MainLock` per slot that derefs to the lock
  of the current slot. A slot gets a new generation each time it is given out, so a
  value of a closed window is made again for the next one.
- The GPU device, the pipelines, the fonts, the pictures, the theme and the keymap are
  shared. Each font sets the view matrix of its brush to the size of the window it
  draws in, `Font::begin_frame`.
- A view registers its touch in the stack of the window it is in, `TouchStack::in_window_of`,
  also when a timer of the main window adds it. A key binding answers only in the window
  that has the keys, so an Escape in an extra window does not reach the main one.
- The queued callbacks, the removed views and the animations run with the frame of
  the main window. While the main window is covered, a frame of an extra window runs
  them first.
- A window that closes in its own event hands the rest of that event to the main window,
  `slots::leave`.
- A scene and a level draw in the main window only. Each window steps the declared
  loops of its own views, see "Windows" in [loops.md](loops.md). The inspector reaches
  every window with `--window`, see [inspect.md](inspect.md).

## Over everything on a Mac

An `AboveAll` window is made hidden, its class is swapped for a subclass of `NSPanel`,
it gets the nonactivating style, the screen saver level and the behavior to join every
desktop and an app in full screen, then it is shown with `orderFrontRegardless` and
`makeKeyWindow`. The app is never activated: an app that activates leaves an app in
full screen for the desktop of its own windows, and the window showed there. The swap
works because `WinitWindow` adds no fields to `NSWindow`, the way tauri-nspanel does it.
The class goes back before winit frees the window, its hidden observer subclass is gone
otherwise and the app aborted. When the last such window closes, the app that was in
front gets the focus back. The code is `window/extra_mac.rs`.

## Tests

The UI test `Second window` opens one headless, taps and types into each window, reads
the picture of the second one and closes it. `window::slots` has unit tests for the
values of a reused slot. `Loop in second window` checks a loop there, `Inspect window`
the inspector commands there.
