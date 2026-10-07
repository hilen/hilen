# Screen keyboard

What the engine does when the screen keyboard of a phone comes up. The code is
`hilen/src/ui/screen_keyboard.rs`. Only iOS has a screen keyboard in the engine today,
Android opens none, see [roadmap.md](roadmap.md).

## What an app gets

- The edited `TextField` stays in view with no code in the app. The nearest `ScrollView`
  around the field scrolls as far as it can. What is still covered moves the whole
  screen up, and it comes back down with the keyboard. 8 points stay between the field
  and the keyboard.
- `place().b_keyboard(offset)` is `b(offset)` that also stays on top of the keyboard. A
  compose box rides on the keyboard this way, and a list anchored above it,
  `anchor(Anchor::Bot, compose, gap)`, gets shorter. A field inside such a view is in
  view already, the screen does not move for it.
- `ScreenKeyboard::top()` is the top edge now, part way during the move, `None` with no
  keyboard. `cover(view)` is how much of the bottom of a view it covers. `is_moving()`
  and `shift()` tell the rest. Lengths are in the points `absolute_frame` counts in.
- `UIEvents::screen_keyboard()` fires when a move starts, with the edge it ends at and
  the seconds it takes.
- `TextField::focus()` does nothing on a device with a screen keyboard. A focus set by
  code, at the start of a screen or on a switch of a chat, would bring the keyboard up
  with no word from the user. A tap opens it there. `focus_with_keyboard()` opens it
  everywhere, for a screen the user opened to type, the rename prompt of the file
  browser uses it. `ScreenKeyboard::on_this_device()` tells which kind of device this is.

## How it works

- `hilen_text.m` listens to `UIKeyboardWillChangeFrameNotification` and passes the top
  edge the keyboard moves to, in window pixels, and the seconds of the move.
  `system_field.rs` asks for that at the first edited field.
- iOS says several things in a row for 1 move. A keyboard that comes up reports its
  short form, then that it is gone in 0 seconds, then its full height. So `moves_to`
  only keeps what was said last, and `update` starts that move on the next frame. Without
  this the screen jumped down and came up again. The event of the app fires there too,
  not inside the change of the selected view the system spoke in, which holds a lock.
- The engine moves its own top edge over the same time on an ease curve, close to the
  one of iOS and not the same. Layout runs every frame, so `b_keyboard` just reads the
  edge.
- `ScreenKeyboard::update` runs every frame before the layout. While the keyboard moves
  it arranges the edited field every frame. A keyboard that stands still arranges a
  field once, after that the user may scroll it out of view.
- The move of the screen is `RootView::set_keyboard_shift`. It moves the container of
  the app views, the root background stays.

## Tests

`Screen keyboard shift`, `Screen keyboard scroll` and `Screen keyboard compose` cover
the 3 cases, `Text field focus by code` the focus. They run everywhere:
`system_input::screen_keyboard(up, top)` waits for the real keyboard on a phone and
tells the engine about a stand in with its top edge at `top` everywhere else.
`KeyboardMarker` of the test suite draws that stand in. The real keyboard of the lane
phone ends at 814 pixels, the tests use the same number. A tap at the very top of a
phone screen hits the status bar and never reaches the app.
