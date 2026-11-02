# Text entry in a browser

A canvas takes no text from the system, only keys. So while a `TextField` is edited
in a browser, a real `<input>`, or a `<textarea>` for a multiline field, has the focus
and lies over it. The keyboard of a phone, a paste, undo, word guesses and composed text
all come from the browser. The code is
`hilen/src/ui/views/basic/text_field/browser_field.rs`, the iPhone does the same with a
system field, see The system text field in [ios.md](ios.md).

## How it works

- The element cannot be seen and takes no touches, `opacity: 0` and
  `pointer-events: none`. The engine still draws the text, the caret and the selection,
  so a field looks the same as on a desktop and the probes of the UI tests hold.
- Its text size is 16 px, a smaller one makes Safari on an iPhone zoom the page in.
- Every `input` event gives the whole text and the selection to the field,
  `system_text_changed` and `system_selection_changed`. A browser counts places in
  UTF-16 units and the engine in bytes, `utf16.rs` turns one into the other.
- Every change the engine makes, a typed key of a test, `set_text`, a constraint that
  drops a char or a drag that selects, goes back to the element in `sync`. It writes only
  what differs, a new value wipes the undo list of the element. It writes nothing while
  a text is composed, an input method breaks when its text changes under it.
- The keys a field handles itself, Enter, Escape, Tab, the arrows, Home and End, go to
  `Input::on_key` with the default of the browser stopped. A key taken with `take_keys`
  goes too, Backspace and Delete only then. A key of an input method, `isComposing` or
  the code 229, stays with the browser. Cmd or Ctrl with a letter other than the edit
  letters a, c, v, x, y and z goes to the key bindings of the screen.
- winit focuses the canvas on every press. A listener on the focus of the canvas gives
  the focus straight back in that event, and the end of a tap on the field does it
  again, `grab_focus`. A phone opens its keyboard only inside a user gesture.
- `ScreenKeyboard::on_this_device` asks `(pointer: coarse)` in a browser. On such a
  device a resize of the visual viewport is the keyboard, its bottom edge goes to
  `ScreenKeyboard::moves_to`. A loss under 120 CSS pixels is a bar of the browser.
- At the end of the editing every listener comes off, the element leaves the page and
  the canvas gets the focus back.

## Tests

`Text field browser` in `ui-test-suite/src/views/text/text_field_browser.rs`, on the
browser lane only. It puts text in with `document.execCommand("insertText")`, the way a
paste and a key of a phone do, with a real `input` event: Latin, Japanese and an emoji,
the arrows with and without Shift, a paste over a selection, undo, composed text with
an Enter of the input method, the focus that comes back from the canvas, `set_text`, Tab
into a multiline field and Escape. Firefox under a bare Xvfb gives the window no focus
of the system and then fires no focus event, there the step of the canvas is left out
with a warning. The unit tests of `utf16.rs` cover the places.

Passed on 2026-11-02 in Chromium and Firefox on the Linux builder. Not run: a real
phone, a real input method, a long press. The element takes no touches, so a long press
on a phone offers no Paste, only the bar of the keyboard can paste there.
