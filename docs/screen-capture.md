# Screen pictures

The `screen-capture` feature: a picture of every screen and the Screen Recording
permission of macOS, through [xcap](https://github.com/nashaofu/xcap). Windows and Linux
answer with an error, see the roadmap. The code is `hilen/src/screen_capture/`.

- `ScreenCapture::allowed()` and `ask()`. The system shows its dialog the first time
  only, a grant counts after the app starts again. macOS gives the grant to the app
  that started the process: a binary started from a terminal or from a holder asks for
  that one. Start the app as a bundle with `open`.
- `ScreenPicture::all()` takes every screen at its full pixel size, the main one first,
  some tens of ms each, off the main thread too. `display` is the screen in the points
  of `Display::all`.
- `image(name)` gives an `Image` to show, `png_of(rect)` the part under a rect in
  points as a png. Its unit tests use a made up picture.
