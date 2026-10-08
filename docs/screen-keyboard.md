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
- A tap on any other view ends the editing and the keyboard leaves, since a touch
  selects the view it begins on. `view.set_keeps_selection(true)` takes a view out of
  that rule, see below. A send button next to a compose box carries it.

## The selected view

The engine has 1 selected view, the one that has the keys. `check_touch` in
`hilen/src/ui/view/view_touch.rs` selects the view a touch begins on, and a touch no
view takes selects none. A `TextField` is edited while it is the selected view, so
both end its editing, and on a phone the keyboard leaves about 40 ms later.

`set_keeps_selection(true)` of `ViewTouch` marks a view, a `Button` or any other one.
A touch that begins on it leaves the selected view as it is, like a button of a web
page that prevents the default of its mouse down. The view still gets every touch
event, `began`, `up_inside` and the tap of a button. It is never the selected view
itself by a touch. On a desktop the field keeps its caret and its keys the same way.
The mark counts for the view that takes the touch, not for the views inside it. A
touch beside the view still ends the editing. The touch views are asked one by one,
and each one that is not under the press unselects before the one that takes it is
reached. So `press_keeps_selection` finds the view that takes a press before the
press goes to the views, and `check_touch` gets the answer. The text selection of a `Label` is
another thing and does not follow the mark.

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
the 3 cases, `Text field focus by code` the focus. `Screen keyboard first letter` types
and erases a letter in a `b_keyboard` box and sends the resize an iPhone sends then, the
keyboard and the box must stay, see the system text field in [ios.md](ios.md).
`Screen keyboard kept by tap` taps a button and a plain view with
`set_keeps_selection` next to an edited text area, the field stays edited and the
keyboard reports no move, then a button without the mark ends the editing. They run
everywhere:
`system_input::screen_keyboard(up, top)` waits for the real keyboard on a phone and
tells the engine about a stand in with its top edge at `top` everywhere else.
`KeyboardMarker` of the test suite draws that stand in. The real keyboard of the lane
phone ends at 814 pixels, the tests use the same number. A tap at the very top of a
phone screen hits the status bar and never reaches the app.
