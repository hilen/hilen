# Clipboard

`Clipboard` from `hilen::system`, the code is `hilen/src/system/clipboard/`. Desktop goes
through the `arboard` crate, iOS through `UIPasteboard`, Android through the
`ClipboardManager` service over JNI, the browser through `navigator.clipboard`.

## The calls

- `Clipboard::set_text(text)` puts a text on the clipboard, a `Result`.
- `Clipboard::get_text()` reads it back. Not in the browser, a page reads the clipboard
  only through a call that answers later, after a permission prompt.
- `Clipboard::set_secret(text)` and `Clipboard::clear_if_holds(text)` copy a secret and
  take it back, see their doc comments for what each platform does.
- `Clipboard::get_image()` gives the picture in the clipboard as `Option<ClipboardImage>`,
  `None` when the clipboard holds no picture.
- `Clipboard::set_image(&image)` puts a picture on the clipboard. Desktop only, an error
  everywhere else.
- `ClipboardImage` has `png`, the bytes of a png file, and `width` and `height` in
  pixels. `ClipboardImage::from_rgba(rgba, width, height)` makes one from RGBA pixels.
  The bytes go as they are into a file, an upload or `Image::load`.

## A picture

A desktop clipboard holds a picture as raw pixels. `arboard` gives them as RGBA, its
`image-data` feature is on by default, and the engine makes the png file with the `image`
crate. What each platform does:

- macOS, Windows and Linux: the picture a screenshot or a copy in an image editor or a
  browser left. On Linux only under X11 or XWayland, the workspace does not turn on the
  `wayland-data-control` feature of `arboard`.
- iOS: `UIPasteboard.image` as the png file the system makes of it. The size is read
  from the header of that file. iOS can ask the user to allow the paste. This path
  compiles, it was not run on a device or a simulator yet.
- Android: always `None`. A picture is there a link to the app that holds it, to be
  read through a `ContentResolver`, and the engine does not do that yet.
- Browser: always `None`, for the same reason `get_text` does not exist there.

Making the png file is the slow part. Measured on an M series Mac for 3000 by 2000
pixels with the test `encoding_time_of_a_big_picture` in `clipboard/image.rs`:

- Default compression, the one in use: 52 ms and 137 KB for a picture like a window,
  518 ms and 23 MB for noise, which is the worst case. A debug build takes 94 and 621 ms.
- Fast compression: 27 ms for the window, but 11 MB, 80 times bigger.

So `get_image` must not run on the main thread, half a second there is 30 lost frames.
Call it on another thread and go back with `on_main`. The lock of the clipboard is
given back before the encoding starts.

## A paste into a TextField

The paste shortcut, Cmd or Ctrl and V, is `paste` in
`hilen/src/ui/views/basic/text_field/editing.rs`. It reads `get_text` first. A text is
inserted the way it always was. With no text it starts a thread that calls `get_image`,
and a picture found there fires `TextField::image_pasted` on the main thread with the
`ClipboardImage`, some frames after the keys. The field takes nothing from the picture,
the owner keeps it. A clipboard with a text and a picture pastes the text and fires
nothing.

The shortcut is the only paste route of the engine, a field has no right click menu.
So the event fires on desktop only:

- iOS: the system text field takes the keys and pastes by itself, see [ios.md](ios.md).
  It offers no paste for a picture, so nothing fires.
- Android: `get_image` answers `None`.
- Browser: there is no paste shortcut at all, the page cannot read the clipboard
  at once.

## A copy of selected text

Selectable text, see [text-selection.md](text-selection.md), copies with Cmd or Ctrl
and C and with Copy of its right click menu. Both go through `Clipboard::set_text`,
so they work on every platform, in a browser too.

## Tests and the clipboard of the user

A headless run has no display server, so every call goes to `HeadlessStore`, 1 text or
1 picture kept inside the process. A test that runs in a window, like a `--human` run,
would read and replace what the user copied. `Clipboard::set_in_process(true)` sends
every call to the same store for such a test, and the test runner turns it off again
before every test. `Text field image paste` uses it.
