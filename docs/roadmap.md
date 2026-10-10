# Engine gaps

The entries are in priority order. The first entry is the top priority, work on it
before any other. A new gap goes in as the first entry.

Open engine features still missing, found by porting real apps. Each entry lists the
current state in code, what is needed, and what it blocks. When one lands it needs UI
tests like any other engine change, then it moves out of this file. This roadmap holds
only open work, not a change log of what already shipped.

Engine capabilities only. Every entry must be a general feature of the `hilen` crate
that any app can use. Never an app-specific task, never a single app's dialog content,
screen, wiring or shipped asset. If an item only matters to one app, it belongs in that
app's own docs, not here. A gap an app found still qualifies only when the fix is a
reusable engine capability.

The driver apps are skaityk at `~/dev/apps/skaityk` (a reader for Lithuanian
learners), the beekeeper web UI in the `local` repo at `beekeeper/web`, and kukareker at
github.com/hilen/kukareker (a git client). Full visual and functional parity with each
original is the acceptance bar. Their ports drove the gaps below.

## tvOS, the run on an Apple TV

Found by the tvOS display bring-up and by flixen, `~/dev/apps/flixen`, a player on an
Apple TV. The remote, the safe area, stored settings, the media command of the remote
and video landed on 2026-10-10 and run in the Apple TV simulator, see
[tvos.md](tvos.md).

- Current: the engine never ran on a real Apple TV. A tvOS build of the demo is in
  TestFlight, version 1.0 build 1, sent by `build/tvos/flight.rs`. `make fly` never
  ran as 1 command for both systems. In the simulator the remote drives the demo and
  its shell keeps inside `UIManager::safe_area()`. The hilen skill has no tvOS
  chapter. There is no tvOS lane of the UI suite, single tests run by hand with
  `HILEN_TEST_ONLY`. `Key focus long press` did not run there.
- Needed: the demo from TestFlight on a real Apple TV, driven with its remote, and a
  film played there with sound. Check 4K and HDR output, the frame rate, hardware
  decode named in the first frame log line, and the latent `statusBarFrame` trap of
  [tvos.md](tvos.md). 1 run of `make fly` that sends the iOS and the tvOS build. A
  tvOS lane for the UI tests, like `build/ios/sim-test.rs`. A tvOS chapter in the
  hilen skill.
- Blocks: a tvOS release of any app.

## tvOS, the session, the Keychain and the Now Playing panel

Found by flixen on 2026-10-10.

- Current: on tvOS the sealed session of `SessionStore` is an entry of the user
  defaults, and the `keychain` feature of `hilen-session` builds with the protected
  store of iOS, see Stored data in [tvos.md](tvos.md). Both compile and never ran.
  `MediaSession` builds and links on tvOS. Its panel was never seen, no test played
  something long enough, and no command of a phone or of Siri was sent.
- Needed: a login on tvOS that is still there after a restart, in an app with the
  `login` feature. A secret saved and read through the Keychain there. A film that
  shows in the Now Playing panel of tvOS, paused from the panel.
- Blocks: a login and the system playback controls of a tvOS app.

## Sound clock and MIDI input, the proof on real devices

Found on 2026-10-09 when `SoundClock` and the `midi` feature landed, see
[sound-clock.md](sound-clock.md) and [midi.md](midi.md). The driver is blast,
`~/dev/apps/blast`, a drum practice app.

- Current: both are proven by unit tests only, on macOS and on Linux. The clock tests
  play the audio thread by hand and the MIDI tests use a fake device. No sound of a
  clock went through a real output device, and no real MIDI device was opened, on any
  platform. The engine compiles for iOS and for the browser with both features on.
  A build for Windows and for Android with the `midi` feature was never made. The
  test `a_message_of_a_core_midi_source_arrives` in `hilen/src/midi/midi_test.rs`
  sends through a virtual CoreMIDI source. It is ignored, and it never passed: on the
  build machine kotik midir gives `InitError` when it makes its CoreMIDI client. The
  clock gives the time the audio thread is at. A speaker is later than that by the
  delay of the output device, and the engine does not know that delay. In a browser
  the time of a MIDI message is taken on the main thread, so a busy frame makes it
  late.
- Needed: play a pattern of 16th notes at 280 BPM on a `SoundClock` on a Mac, an iPad
  and an iPhone and hear that it is even. Open a real MIDI device on each of them,
  plug it in and pull it out, and compare the time of a hit with the clock. Run the
  ignored CoreMIDI test on a Mac with a login session. Build with `midi` for Windows
  and Android and try a device there, and in a browser with Web MIDI. Decide whether
  the clock should know the delay of the output device, and whether a MIDI message
  should take its time from the system where the system gives one, like the packet
  time of CoreMIDI and the event time of Web MIDI.
- Blocks: nothing in the engine. blast finds out on its first run with a drum module.

## A tinted icon asked for before its download stays the placeholder for good

Found on 2026-10-09 in the beekeeper web UI, `beekeeper/web` of the `beekeeper` repo,
in a browser.

- Current: in a browser `Tinted::to_image` in
  `hilen/src/window/image/tinted_image.rs` finds no bytes for an SVG whose asset group
  is not downloaded yet. It then makes the default picture and stores it under the
  real name of the icon and tint, `Image::from_file_data(DEFAULT_IMAGE_DATA,
  &tinted_name(..))`. Every later call finds that name with `Image::weak_with_name`
  and returns the default picture, also after the download. So 1 early call breaks
  that icon in that color until the page is loaded again. On desktop the file is read
  from disk at once, so it never shows there. The beekeeper shell painted its tab
  icons in `setup`, before `Assets::await_boot`, and 3 of 4 tabs showed the default
  picture on the public page.
- Needed: an icon asked for too early shows the right picture once its group is
  there. The default picture is not stored under the real name, or the stored entry
  is dropped when the group lands, and a view that got the default picture is drawn
  again then. The same for a plain `Image::get` of a file that is not there yet. A
  test that asks for a tinted icon before its bytes are stored, stores them, asks
  again and gets the real picture.
- Blocks: nothing now. Beekeeper waits for the boot group before it paints an icon,
  the guard is `icons_ready` in `ui/shell.rs`. Any other app that sets an icon in the
  `setup` of its root view has the same broken icons in a browser.

## A touch on a button ends the editing of a text field

Found on 2026-10-09 in banda, `~/dev/apps/banda`, on an iPhone 16 Pro Max.

- Current: every touch that begins on a view selects that view, and a touch no view
  takes unselects, `check_touch` in `hilen/src/ui/view/view_touch.rs`,
  `UIManager::set_selected(weak_from_ref(view), true)` and `UIManager::unselect_view()`.
  So a tap on a send button next to an edited `TextField` ends the editing and the
  screen keyboard closes before the message is sent. The log of the phone shows it:
  `on_selection_changed(false)` from `check_touch`, then `screen keyboard moves to None`
  40 ms later, then the send. A view has no way to say that a touch on it leaves the
  selection where it is. `TextField::focus()` cannot put the keyboard back, it does
  nothing on a device with a screen keyboard, and `focus_with_keyboard()` after the
  tap would close the keyboard and open it again.
- Needed: a view can be marked so that a touch on it does not change the selected
  view, like a button in a web page that prevents the default of its mouse down. The
  edited field stays edited and the keyboard stays up, the tap still fires. A UI test
  on the real keyboard: a field is edited, a tap on such a button, the field is still
  edited and the keyboard reported no move.
- Blocks: the send button of banda on a phone, the keyboard closes at every message.

## Video pictures, pieces and export, Windows and iOS

Found on 2026-10-08 while building kutreel, a video editor at `~/dev/apps/kutreel`.

- Current: `VideoFrames`, `VideoView::set_pieces` and `VideoExport` landed, see
  [video.md](video.md). Their unit tests and UI tests pass on macOS only. The Windows
  ffmpeg archive of release `ffmpeg-9.0-4` has no h264 encoder, so an export fails on
  Windows. `build/ffmpeg-win/build.sh` now turns Media Foundation on, `h264_mf` and
  `aac_mf`, but no archive built that way is published. Nothing of the 3 ran on iOS.
  The pictures and the export do not tone map an HDR source, the export cuts 10 bit
  to 8 bit.
- Needed: prove that a Windows build links the new archive, publish it per
  [video.md](video.md) and pin it. On a real Windows machine run
  `cargo test -p hilen --features video --lib -- video::` and the video UI tests, and
  play an exported file in another player. Run the same tests on the iOS simulator.
  Tone map an HDR source in the pictures and in the export, the way the view does.
- Blocks: the export of kutreel on Windows.

## A key binding with Alt

Found on 2026-10-08 in banda, `~/dev/apps/banda`, at `dev` commit `7d3dbd66`.

- Current: a `KeyCombo` has only `cmd` and `shift`, `hilen/src/ui/input/keymap/key_combo.rs`.
  `Keymap::check` in `keymap.rs` reads the Control, the Super and the Shift key of
  `Input::modifiers()` and never Alt, so a binding cannot ask for Alt, and Alt held
  changes nothing for a plain binding. `KeyCombo::cmd('n')` is Cmd+N on a Mac and
  Ctrl+N on Windows and Linux. An app that wants Cmd+N on a Mac and Alt+N on the other
  systems cannot bind the second one.
- Needed: a combo with Alt, like `KeyCombo::alt('n')` and one with Alt and Shift. It
  fires only while Alt is held, and a plain binding and a command binding of the same
  key do not fire then. On a Mac Option changes the char that arrives, Option+N is a
  dead key, so the match there goes by the key and not by the char. The `keys --alt`
  call of `hilen-inspect` must reach such a binding. A unit test next to the ones in
  `keymap.rs`, and a UI test that presses the key with and without Alt.
- Blocks: Alt+N for a new session in banda on Windows and Linux.

## A table keeps a scroll place that is set before its first layout

Found on 2026-10-08 in banda at `dev` commit `fcd1bda8`. A chat page is made new on
every switch and puts the reader back where they were. The chat stood at its top for
a frame and then jumped.

- Current: `layout_cells` in `views/containers/table_view/table_view.rs` returns
  while the table has no height, and a view gets its size only in the layout of the
  frame after it was added. So `reload_data` on a new table lays out no row, the
  content has no height, and `set_content_offset` and `scroll_to_bottom` cut the
  place to 0. Banda gives its table the old size by hand before the reload,
  `restore` in `src/ui/session_page/rows.rs`.
- Needed: a table with no size yet keeps a scroll place that was asked for, an offset
  or the end, and goes there in the first layout it has a size in, before that frame
  is drawn. `scroll_to_row` already does this, its doc says "a table with no size yet
  scrolls once it gets one".
- Blocks: banda can drop the size it sets by hand.

## Work that runs after this turn of the main loop

Found on 2026-10-08 while the markdown view was made to lay out once.

- Current: `on_main` in `deps/hreads/dispatch.rs` runs its closure at once when it is
  already on the main thread. `after(0.0)` goes through a tokio task, so when it runs
  against the next frame is not fixed. The markdown view now lays out in its own
  `update`, since nothing else waits for the end of the turn.
- Needed: a call that runs a closure on the main thread after the code of this turn
  and before the next frame is drawn. A view that gets several changes in a row can
  then do its work once, also a view that is hidden and gets no `update`.
- Blocks: nothing. Banda uses `after(0.0)` in several places where it means this.

## The login poll waits 2 seconds

Found on 2026-10-08 in the investigation of delays in banda.

- Current: `wait_for_login` in `hilen/src/login/client.rs` asks the server every
  `POLL_SECONDS`, 2 seconds. A fresh login shows in the app up to 2 seconds after the
  browser finished it.
- Needed: a shorter wait at the start that grows, or a long poll on the server side,
  so the app moves on within a few 100 ms of the login.
- Blocks: nothing, it happens once per device.

## The browser websocket does not say why it failed to open

Found on 2026-10-08 in banda.

- Current: the native client reports a failed upgrade with its HTTP status, like
  `HTTP error: 404 Not Found`. The browser client in `deps/netrun/ws/web.rs` gives
  only `WebSocket failed`, a browser hides the status of a failed handshake.
- Needed: a way for an app to tell a server that has no such route from a lost link
  on every platform, like a plain request to the same path first, or a field on
  `WsEvent::Error` that says when the status is not known.
- Blocks: banda in the browser asks an old backend for its socket every 15 seconds
  and not every 60, it cannot tell the 2 cases apart.

## The Postgres twin of the token login test never runs

Found on 2026-10-08 with `user_of_token`.

- Current: `a_token_alone` in `hilen-server/src/auth/store_test.rs` passes on SQLite.
  Its Postgres twin is ignored, it needs a server.
- Needed: a lane that starts a Postgres and runs the ignored tests of `hilen-server`,
  like `sqlx.sh test` of a backend does.
- Blocks: nothing. The Postgres path of the login is proven only by the apps on it.

## A warning in every build of every app

Seen on 2026-10-08 with the nightly toolchain of the workspace.

- Current: `#![feature(generic_const_exprs)]` in `hilen/src/lib.rs` line 9 makes the
  compiler print "is not supported with the next-generation trait solver" for the
  `hilen` crate in every build, also in the build of an app. An app cannot reach a
  build with 0 warnings.
- Needed: the engine builds with no warning, by dropping the feature or by what the
  compiler note asks for.
- Blocks: nothing.

## Frame record and frame step, the proof in banda and on the other lanes

Found on 2026-10-07 in banda at `dev` commit `9439493e`. A popup opened with 3 wrong
frames: the pills of a row sat left of the popup until their anchors settled. The
user saw a flicker and had no way to show it or to look at it.

- Current: `hilen-inspect` saves every frame after an input, `tap <query> --frames 30
  --out <dir>` and `record`, and walks a paused app with `pause`, `step [n]` and
  `resume`, see [inspect.md](inspect.md). The `Inspect frames` UI test passes on the
  desktop lane, and both were driven by hand on the demo window on macOS.
- Needed: bump the pin of banda, open the popup with `tap --frames`, and see the 3
  wrong frames in the pictures. Run `Inspect frames` on the iOS simulator and the
  browser lanes. Start a `record` with a real click, the test covers only the record
  that no input starts.
- Blocks: nothing. This entry is deleted once the popup of banda is seen in the frames.

## 17 UI tests fail on the iOS simulator

Found on 2026-10-07 while the iOS lane was moved to the build machine kotik. The lane
`make ui-ios` ran at `dev` commit `9439493e`, on the iPhone 8 simulator with iOS 16.4
and Xcode 26.3.

- Current: 254 tests pass and 17 fail. The same 17 fail on 2 machines, on kotik in an
  account with no desktop session and on a mac with a desktop session and the Simulator
  window open, so the machine is not the cause. The `iOS UI Tests` job of CI was also
  red on its last runs, on 2026-09-25, so there is no green run to compare with. The
  input helper is alive in these runs, its log shows taps on keys of the screen
  keyboard. The 17 tests and what each one prints:
  - `File browser narrow`: `file_browser_narrow.rs:55`, the chosen files are
    `["dog.jpg"]` and the test wants `["cat.png", "dog.jpg"]`.
  - `File browser pick`: `file_browser_pick.rs:90`, the label is `Disk/readme.md` and
    the test wants `2 files`.
  - `File browser path`: stuck, `HILEN_TEST_STUCK` after 30 seconds.
  - `File browser search`: fails with no message after `opened Disk`.
  - `Label selection`: `label_selection.rs:724`, the selection is the whole sentence
    and the test wants `fox`.
  - `Markdown selection`: `markdown_selection.rs:819`, the selection is `link to a` and
    the test wants `k to a`.
  - `Multiline field grows`: stuck, `HILEN_TEST_STUCK` after 30 seconds.
  - `Screen keyboard compose`, `input`, `look`, `multiline`, `scroll`, `secure`,
    `shift`, `submit` and `taken return`: each fails after about 20 seconds with no
    message.
  - `Text field focus by code`: fails after about 10 seconds with no message.
- Needed: find the cause of each group and fix it, in the engine or in the test. The 2
  file browser tests that choose 2 files get only the second one, which looks like a
  second tap that replaces the choice where the test wants it added. The 2 selection
  tests get a wider selection than the drag asks for. The keyboard tests need their
  failure printed first, a test that fails with no message cannot be read from a log.
  Run one test alone with `HILEN_TEST_ONLY="Screen keyboard input" make ui-ios`.
- Blocks: a green iOS lane, on a work mac, on kotik through far and in CI.

## Keys the owner of an edited text field takes for itself

Found by banda at `~/dev/apps/banda`. Its compose box shows a list of slash commands
above the field while the text starts with `/`. Up and Down have to move the marked
row, Tab and Enter have to fill the marked command in, Escape has to close the list,
and the field has to stay in edit through all of it.

- Current: `Input::on_key` in `hilen/src/ui/input/input.rs` calls
  `UIManager::keymap().check(key)` and then always
  `UIEvents::keyboard_key().trigger(key)`. The edited `TextField` handles that event in
  `on_key` of `hilen/src/ui/views/basic/text_field/editing.rs`: Tab selects the next
  field, Enter submits or adds a line, Escape ends the edit with
  `UIManager::unselect_view`, an up or down arrow moves the caret. A keymap binding
  for 1 of these keys runs too, but nothing can stop the field from acting on the
  same press. `TextField` has the events `changed`, `editing_ended`, `submitted` and
  `image_pasted`, none for a key.
- Needed: a way for the owner of a field to take named keys while the field is
  edited, switched on and off at run time, like a list of keys with 1 callback. A
  press of a taken key goes to the owner only. The field does not move its caret,
  does not submit, does not lose the edit and does not jump to the next field. A key
  that is not taken works as now. It must work for a single line and a multiline
  field, on every platform with a keyboard.
- Blocks: the keyboard side of the command list in banda, and any completion list,
  mention list or search drop down that opens under a field that is being typed in.

## Screen keyboard, the proof in banda and on Android

Found by banda at `~/dev/apps/banda`, whose compose box sits at the bottom of the
screen. The keyboard frame, the field that stays in view, `b_keyboard` and the focus
by code that opens no keyboard landed, see [screen-keyboard.md](screen-keyboard.md).

- Current: proven by 4 UI tests on desktop, with a stand in for the keyboard, and on
  the iOS simulator lane with the real one. banda has the pin `da1b9180` and its
  compose box is placed with `b_keyboard`. Its chat was tried on the iPhone 14
  simulator, iOS 16.4, with the engine that makes the app views the safe area: the
  compose box sits above the home bar, a tap on its field opens the keyboard, and the
  box rides on the keyboard with its 16 points of gap while the list above gets
  shorter. No real iPhone ran it, and the pin of banda is still older than the safe
  area fix. The move runs on an ease curve close to the one of iOS, not measured
  against the real keyboard. Android opens no screen keyboard at all, a tap on a
  `TextField` there brings nothing up, so there is nothing to follow yet.
- Needed: the pin of banda moved to an engine with the safe area fix, and the chat
  tried on a real iPhone, with the 3 `compose.focus()` calls of
  `src/ui/session_page/mod.rs` left as they are. A screen keyboard on Android, opened
  by an edited field, with its frame passed to `ScreenKeyboard::moves_to`.
- Blocks: calling the chat of banda done on a phone, and every text field on Android.

## The wheel delta does not tell which way the user turned the wheel

Found by flixen at `~/dev/apps/flixen`. Its player turns the volume with the wheel, up
is louder.

- Current: `UIEvents::on_scroll` gives the delta the system reports, `scroll_pixels` in
  `hilen/src/window/app_handler.rs`. On a Mac with natural scrolling on, the default,
  that delta is already turned around, so a wheel turned up gives a negative `y`. That
  is right for content that scrolls and wrong for a value like volume or zoom. The
  window code of `hilen-winit` reads `scrollingDeltaY` in `scroll_wheel` of
  `platform_impl/macos/view.rs` and never reads `isDirectionInvertedFromDevice`, so no
  app can tell. flixen turns the sign around on every Mac, which is wrong again with
  natural scrolling switched off.
- Needed: the wheel event also carries the turn of the device itself, or a flag that
  the system turned it around. On a Mac from `isDirectionInvertedFromDevice`, on the
  other platforms from their own setting where the system reports the turned value. A
  test for the sign with the flag on and off.
- Blocks: the right volume direction in the flixen player on a Mac with natural
  scrolling off, and the `cfg!(target_os = "macos")` sign turn in `wheel_volume` of
  `crates/flixen/src/ui/player_screen.rs`, which goes away then.

## A paragraph of only bold text before a list is not drawn by MarkdownView

Found by banda at `~/dev/apps/banda`. Its chat draws what Claude wrote with
`MarkdownView::set_text`, and Claude often writes a bold line as a small header right
above a list.

- Current: a text like `**Done**` on 1 line with a list on the next line, `- first`,
  shows the list and an empty gap where the bold line should be. The text is there: a
  selection over that place copies `Done`, so the block is parsed and laid out, and
  only its drawing is missing. A bold word inside a longer paragraph is drawn. The
  cause is not found yet, prove it before a fix. Seen on hilen `dc3d50d5`, and before
  the text selection landed, so the selection did not bring it.
- Needed: the bold paragraph is drawn like any other paragraph. A UI test with a bold
  only paragraph right above a list, above a paragraph and at the end of a text.
- Blocks: the small headers of an answer in the banda chat.

## A key combo with Alt, and a combo that wins over the focus ring and a text field

Found by banda at `~/dev/apps/banda`. It jumps between its sessions with Cmd and an
arrow on a Mac, and has to do the same with Alt and an arrow on Windows and Linux.

- Current: `KeyCombo` in `hilen/src/ui/input/keymap/key_combo.rs` has `cmd` and
  `shift` only, and `cmd` is Cmd or Ctrl, `command_held` in `keymap.rs` takes both. No
  binding can ask for Alt. `Input::on_key` in `hilen/src/ui/input/input.rs` hands an
  arrow to `Focus::on_key` first, which takes it with any modifier held while the ring
  is active, so a combo with an arrow reaches the keymap only while a text field has
  the keys. That field then gets the same press through `keyboard_key` and also moves
  its caret.
- Needed: `KeyCombo::alt(key)`, and a way to bind 1 action to Cmd on a Mac and Alt
  elsewhere. A press that matches a bound combo with a modifier goes to that binding
  only, not to the focus ring and not to the focused text field. A plain arrow stays
  with the ring and the field. A UI test for both.
- Blocks: the session switch keys of banda on Windows and Linux.

## An image from the clipboard on Android, in a browser and on a real phone

Found by banda at `~/dev/apps/banda`. Its compose box takes a screenshot from the
clipboard and sends it with a prompt. `Clipboard::get_image` and the `image_pasted`
event of `TextField` landed, see [clipboard.md](clipboard.md).

- Current: proven on macOS with a real screenshot in banda, and by unit tests and the
  UI test `Text field image paste` on the store of the process. Windows and Linux use
  the same `arboard` code and were not run, Linux only under X11. iOS reads
  `UIPasteboard.image`, it compiles and never ran, and a paste there does not fire
  the event, the system text field pastes by itself. Android and the browser answer
  none.
- Needed: a picture from the clipboard on Android, through a `ContentResolver`, and
  in a browser, where the read answers later after a permission prompt. A paste on a
  phone that fires `image_pasted`. A run on Windows, on Linux and on an iPhone.
- Blocks: pictures in the prompts of banda on a phone and in a browser.

## File browser, the proof in an app and on the other lanes

Found by banda at `~/dev/apps/banda`. Its New session page picks the folder a session
starts in, on another machine. `FileBrowser`, `FilePicker` and the `FileSource` trait
landed, see [file-browser.md](file-browser.md).

- Current: proven by 14 UI tests on desktop, on a tree held in memory, and by unit
  tests of `LocalFiles` on a temp folder. banda uses `FilePicker` with its own source
  for the folders of another machine, run on macOS against a Linux machine. The tests did not
  run on the iOS simulator lane or the browser lane, 3 of them type through
  `system_input`. `LocalFiles` was not run on Windows, where the roots are the drive
  letters and a hidden file is an attribute, nor on a phone. The view has no drag and
  drop, no copy and paste of files, no preview, and no style call for an app with its
  own palette. A delete on `LocalFiles` is for good, nothing goes to a trash.
- Needed: the 14 tests green on `make ui-ios` and `make ui-web`. A run of a
  `LocalFiles` browser on Windows. A style call, so the picker takes the palette of
  the app, the same for `ContextMenu`, whose colors are fixed in
  `hilen/src/ui/views/controls/context_menu.rs`.
- Blocks: a folder picker and a right click menu in the colors of banda.

## Google access, the proof on a phone, in a browser and with a linked account

Found by lendar at `~/dev/apps/lendar`, a calendar that reads and writes Google
Calendar for several Google accounts at once. The feature landed, see
[google-access.md](google-access.md).

- Current: proven on a Mac debug build against the live backend of lendar. The main
  account signed in, the Drive app folder was read and the calendars synced. No
  second account was linked against the real Google yet, the 2 device tests run
  against a stand in for the Drive folder. The secure store ran only on a Mac, for
  Windows, Linux and Android `hilen-session` was only compiled. Nothing ran on iOS,
  on Android or in a browser, and no signed release build read the Keychain yet.
- Needed: a real link of a second account and its return on a second device. The
  sign in on an iPhone, on Android and in a browser. A signed Mac release that
  reads the Keychain with no dialog, and a run of the store on Windows and Linux.
- Blocks: lendar on iOS, Android and in the browser.

## Canvas and pinch, the proof on real input

Found by Lan Atlas, a network map app with a canvas that pans and zooms. `CanvasView`,
the scale of a subtree and the pinch event landed, see [canvas.md](canvas.md).

- Current: proven by the UI tests `Canvas zoom` and `Canvas pinch` on desktop, with
  injected touches only. No real trackpad and no real finger made a pinch yet. The
  browser path, a wheel turn with Ctrl held, was never built or run, and the 2 tests
  did not run on the iOS simulator lane or the browser lane. Windows and Linux get no
  trackpad pinch, winit reports none there. A view that draws by itself, a
  `VideoView`, does not scale inside a canvas. A `ScrollView` inside a zoomed canvas
  drags at the speed of the screen, not of its content. After a pinch ends with 1
  finger still down, that finger pans only after it lifts and touches again. The
  touch recorder prints finger touches and no trackpad pinch.
- Needed: the Lan Atlas map on `CanvasView`, a pinch on a Mac trackpad, on a phone and
  in a browser, and the 2 tests green on `make ui-ios` and `make ui-web`. Then the
  small points above, each with its test.
- Blocks: calling the map screen of Lan Atlas done on a phone and in a browser.

## Video on iOS, the proof on a phone

Found by flixen at `~/dev/apps/flixen`, a self hosted media server and player.
The `video` feature builds and plays on iOS now, see [video.md](video.md), and
its UI tests pass on the simulator. No real iPhone ran it yet.

- Current: proven on the iOS 16.4 simulator only, where h264 decodes in
  software and HEVC in hardware. The audio session, `Window::set_orientations`,
  the phone side of `Window::set_fullscreen` and `MediaSession` on iOS are
  written and compile for a device. On Android `set_orientations` only keeps
  its value. A film stops when the phone locks, an app has no background
  audio mode.
- Needed: a film played on a tethered iPhone, with hardware decode named in
  the first frame log line, sound with the silent switch on, the screen turned
  by a player, and play, pause and seek from the control center and from
  headphones. `UIBackgroundModes` with `audio` in the generated `Info.plist`,
  as a knob of `hilen.toml`, so the sound goes on behind the lock screen and
  the lock screen shows the film. The orientation call on Android, through
  `setRequestedOrientation`.
- Blocks: calling flixen on an iPhone and an iPad done.

## Flixen, the media player gaps

Found by flixen at `~/dev/apps/flixen`, a self hosted media server and player.
Its first version is macOS only. The gaps landed in 8 batches: tracks and
subtitles, HDR, fullscreen, pointer hide and screen awake, SQLite in
`hilen-server`, view opacity with the group fade, the line limit and text
outline, image downloads, playback speed, Now Playing and the media keys, zlib
and dav1d, the whole track subtitle read. See
[video.md](video.md), [text.md](text.md) and [login.md](login.md). What is left:

### What the flixen batches left open

- AV1 always decodes in software through dav1d, also on a Mac whose hardware
  has an AV1 decoder, ffmpeg lists dav1d first. Picking the hardware decoder
  where VideoToolbox has one needs such a Mac to prove it, an M3 or newer.
- Now Playing and the media keys are macOS and iOS only. Windows needs the System
  Media Transport Controls. Raise this point only when the roadmap is read
  on Windows.
- Linux needs MPRIS for the same. Raise this point only when the roadmap is
  read on Linux.
- The outline and the soft shadow of a label are images made on the CPU
  now, see [text.md](text.md). The effect entry points of the `wgpu_text`
  fork, `vs_effect` and `fs_effect`, are unused and can leave the fork. They
  were never run on an A7 device.
- A film with no index of its subtitle packets is read through once when a
  subtitle track is picked, over http that downloads the whole file. Until
  that read is done a seek finds only a line that began in the 10 seconds
  before its target.
- The data folder on Android is made and set by the engine, the code
  compiles in the docker build and has not run on a device or an emulator.
- A group fade draws into an image of the frame size with its own depth and
  multisample target, made on first use and kept. A target of the size of
  the group would cost less memory.

## LG webOS TV, the run on a TV

`make webos`, the key focus, browser storage, video in a browser and the login from a
phone landed, see [webos.md](webos.md). None of it ran on a TV yet.

- Current: proven on desktop and in Chrome only. The Back key code 461 is taken from
  the LG documents. The guard after a lost WebGL context has no reproduced failure.
  The pack step of `make webos` waits on a RustScript fix, see webos.md.
- Needed: an app driven with the remote on an LG C1, a film played there in a
  `VideoView`, a login with the code, and the UI suite on the TV with the Clipboard
  test no longer skipped. A lost context made on purpose with `WEBGL_lose_context`
  and a resize after it.
- Blocks: calling the TV a supported target.

## Password autofill on iOS

- Current: an edited field on an iPhone is a system text field, see
  `hilen/src/ui/views/basic/text_field/system_field.rs`. It gets the secure flag, and
  nothing that says what the field holds. `TextField` has no call for that. iOS offers
  a saved login only by guessing from the secure flag.
- Needed: a content type on `TextField`, user name, password, new password, email and
  one time code, passed to the system field as its `textContentType`. For the
  passwords of the app's own site also an associated domain in the entitlements of
  the mobile project. The simulator has no saved logins, so the proof needs a phone.
- Blocks: a login form that fills itself from the iOS keychain, and a code from an
  SMS offered over the keyboard.

## Text entry on a TV

Found by flixen, whose admin screens have text fields and stay off the TV for it.

- Current: a text field needs a keyboard. A TV has none, and the keyboard of the TV
  itself opens only for an HTML input, not for a canvas.
- Needed: an on screen keyboard view driven by the key focus, or a hidden HTML input
  that raises the keyboard of the TV and feeds the engine text field.
- Blocks: any screen with a text field on a TV.

## Sound tracks of a video in a browser

- Current: `VideoView` in a browser lists no sound tracks and plays the first one of
  the file. `set_audio_track` logs a warning.
- Needed: the `audioTracks` list of the video element where the browser has it. Not
  checked: whether Chromium 79 on webOS has it for an mkv.
- Blocks: a film whose first sound track the device cannot play, DTS on an LG C1.

## Packaged app for LG webOS

`make webos` packs a hosted app, a small `.ipk` whose page sends the TV to a served
wasm dist.

- Current: the app needs its server for its own files.
- Needed: the same target also packs the whole dist into the `.ipk`, so the app starts
  with no server. Not verified: a packaged app loads from a local origin on the TV, and
  the engine fetches the wasm and the asset groups relative to the document base.
- Blocks: a webOS app that works with no network and no server.

## 3D scene, remaining deliveries

The `scene` module landed with primitives, physics, the Filament mobile PBR
materials, a sun with point and spot lights, textures and normal maps, a sky
with image based lighting, transparency, a tonemap, a first person player,
`.glb` models with skins and animation clips, cascaded sun shadows with a shadow
distance, distance fog and touch picking, see [scene.md](scene.md). The rest was
planned with it.

- An embeddable `SceneView` that composites into any view frame instead of the
  root area, and the offscreen linear pass with real HDR that `colors.md`
  sketches, if a scene ever needs it.
- The shadow passes draw every opaque node into every cascade, the GPU clips
  what lies outside a map. Culling nodes against each cascade's box on the CPU
  would make a short shadow distance cut the passes' cost on a big level.

## 3D scene on WebGL2

Found by opening the demo with `?hilen_webgl` in the page query. The UI pipelines
read their instances from a uniform array on WebGL2, `InstanceBinding` in
`render/uniform`, the scene does not.

- Current: `MeshPipeline` binds the instances, the joint matrices and the lights
  as read only storage buffers with no uniform fallback, and WebGL2 has none. On
  the forced uniform path, `HILEN_UNIFORM_INSTANCES=1`, the first scene frame
  fails in `create_bind_group` for `mesh_lights_bind`. The web lane runs the scene
  tests on WebGPU alone, so nothing pins it. The sprite pipelines of `level` no
  longer panic there, `instances_shader` leaves a source with no storage untouched,
  but no lane runs the level tests on the uniform path.
- Needed: `level-test` on `HILEN_UNIFORM_INSTANCES=1` in `make ci`. The scene follows
  the UI: the lights become a fixed uniform array, the opaque batches draw chunk by
  chunk through `InstanceChunks` with the `index` attribute chunk relative, and
  the joints bind a uniform window per batch, 256 matrices, split when the
  skinned nodes overflow it, shared with the shadow pass. Check first that naga's
  GLSL output takes `textureLoad` on the depth array shadow map, else that path
  needs a comparison sampler or a color render of the depth. Then `scene-test` on
  `HILEN_UNIFORM_INSTANCES=1` in `make ci`, and a `?hilen_webgl` run in the web
  lane.
- Blocks: any 3D page in a browser without WebGPU: an iPhone below iOS 26, a page
  on plain http over a LAN address, WebKit handing out no adapter, a blocklisted
  GPU.

## Scene tests on the browser lane

The scene tests live in `scene-test-suite`, `demo` links it, and the device
autorun runs them after the UI tests, so `make ui-ios` and `make ui-web` cover
them. Nine of them are gated off a lane they fail on, so the lanes run green
while the causes stay open. Each gate points here, and every fix removes its
gate.

- Current: `Animations`, `Cascades`, `Shadows` and `Cutout shadows` are off the
  browser lane. They pass on the iOS simulator and in a real Chrome, and under
  SwiftShader, the software WebGPU the CI browser lane renders with, their
  shadow edges land a few pixels off, the depth precision of the shadow map is
  the lead. `Colliders` is off the browser lane too,
  SwiftShader draws the green collider lines in another shade, `#00ff60` where
  a GPU blends `#1fd153`, the lines are not antialiased there. `Collider
  shapes` and `Player walk` are off the browser lane as well. Both no longer
  check a picture, see [scene.md](scene.md), so the reason for their gates is
  gone, and neither was run on SwiftShader since. `Mouse look` is
  desktop only. `Cursor::capture` does nothing on a phone, and a browser grants
  pointer lock only after a real click, which no test can inject, so the
  capture is released at once. The UI test `Cursor capture` is desktop only for
  the same reason. Chromium on SwiftShader runs on a Mac in docker: an image
  with `chromium`, `xvfb`, `xauth` and `socat`, the driver started with
  `--browser none`, `socat` forwarding the container's port 44810 to the host
  so the page stays on `localhost`, and the Linux flags of `launchChrome`.
  `Cutout shadows` passed there on arm64, the CI runner is x86_64, so its gate
  stays until CI ran it.
- Needed: for `Collider shapes` and `Player walk`, a run on SwiftShader and then
  their gates removed. For the shadows, a bias or a depth format that reads the same on
  SwiftShader. For the collider lines, probes off the lines or lines that cover
  the same pixels without multisampling. For the mouse, a real click from the
  driver, through the debug protocol of Chrome and its twin in Firefox, else
  those two tests stay desktop only.
- Blocks: the gated tests on `make ui-web`, and `Mouse look` on `make ui-ios`.

## Video playback, the other lanes

Desktop macOS, Windows x64 and iOS landed, see [video.md](video.md): `VideoView` behind
the `video` feature, ffmpeg from prebuilt static archives, VideoToolbox and
D3D11VA decode, kira for the sound and as the clock. On macOS a 1080p60 and a
4K30 file play at full rate with sound. The rest of the platforms are open.

- Current: the archive exists for `aarch64-apple-darwin` and for
  `x86_64-pc-windows-msvc`. `demo` and `ui-test` turn the feature on through
  a target table for macOS and Windows. On Windows the 8 video UI tests and
  the video unit tests pass on a real machine, run by hand, no CI lane runs
  them there. Sound on Windows was checked by the tests only, nobody
  listened, and no large file was measured for dropped frames. The engine
  declares VAAPI for Linux, which has no archive and no CI lane. A decoded 4K
  frame is copied through system memory, 12 MB a frame, and plays at rate.
  Each `VideoView` keeps one frame image per source size for good, the
  managed image store never frees.
- Needed: an archive for Linux and for the Intel Mac from `build/ffmpeg.rs`,
  and for Windows on ARM from `build/ffmpeg-win.rs`. Linux needs `libva-dev`
  and `nasm` in `build/setup.sh` and in the docker containers of the Linux CI
  job, and its TLS named in the script. Then the target tables widen to
  `desktop`. A person plays a real film on Windows, with sound, and reads
  the dropped frames off the stats line.
  Android needs archives cross built per ABI with MediaCodec, and the feature
  unlocked there. The browser plays through a
  video element under the canvas, see video.md. Zero copy on macOS, a `CVPixelBuffer` into a Metal texture
  through the wgpu hal, only after an A/B shows the copy costs frames.
- Blocks: video on Linux, the Intel Mac, Windows on ARM and Android.

## Leftovers inside landed features

Small remainders not worth their own entry.

- Sign in with Apple was never run against the real Apple, which refuses
  `localhost` as a return address. The exchange, the form post and the revoke are
  proven against a stand in server only, see [login.md](login.md). The first backend
  with a real service id proves them.
- Tab focus traversal does not scroll. Tab selects the next text field even when
  it sits scrolled out of view inside a `ScrollView`, so the editing session
  starts off screen. Needs a scroll-to-view step in `select_next_field`, the way
  a multiline field already follows its caret line while typing.
- Frame stepped time covers `Animation` and `AnimatedImage` only. `RingSpinner`,
  the text field caret blink, tooltip and long press delays still
  read `Instant`, so they drift under stepped time and their mid flight frames
  cannot be pinned. Each is a one line move to `Clock::now_ms` plus a test.
- Sideways scrolling pins nothing in a plain `ScrollView`, its whole content
  moves. Only a `TableView` cell keeps parts in place, through
  `set_moves_sideways`.
- The same text frame still differs by 1 color level between 2 runs of the
  program after the glyph snap, see [text.md](text.md). The cause is not found.
- A scene picture, `SceneManager::picture`, is proven on desktop Metal only. The
  iOS simulator and the browser lanes have not run its 2 scene tests, and
  WebGL2 draws no scene at all yet.
- The alpha cutout of a scene material is proven on desktop Metal only. The iOS
  simulator and the browser lanes have not run `Cutout` and `Cutout shadows`. A
  `.glb` material's `doubleSided` is not read, back faces are always culled, so a
  leaf card is seen from one side. The cut edge is hard, alpha to coverage would
  smooth it under MSAA.
- Human mode has no frame step key. A stepped test pauses only on checks, an
  animation in flight cannot be walked one frame per key press with the frame
  number in the window title.
- An animation never writes its exact end value, the last commit lands just before
  expiry. With a fixed step the finishing frame could clamp to the end value, but
  that changes visible behavior and recorded expectations, so it is its own gated
  decision.

- A COLR sweep gradient draws through tiny-skia's sweep shader with the
  angles taken as degrees counter clockwise from the font's x axis, the
  COLR convention is not pinned by any test font. A color glyph ignores the
  text color's alpha, a translucent emoji needs an opacity in the image
  instance.
- SVG premultiplied upload: the convert from tiny-skia's premultiplied pixels to
  straight alpha is a per pixel loop, optimized in `hilen-pixels` but still work.
  Uploading premultiplied and blending svg textures premultiplied in the image
  pipeline would remove that loop, it needs a flag per image instance and a blend
  change in the shader.
- DrawingView paths: texture fills for paths, more than 8 gradient stops, and soft
  edges on arbitrary path outlines, a radial alpha ramp only covers circular glows.
- A rect edge on a fractional pixel row blends differently per GPU under MSAA
  4x. The SDF alpha and the hardware sample coverage combine, and the 4x
  sample pattern is the GPU's own, SwiftShader in the CI browser lane reads
  such a row 30 levels off a Mac. Text underlines snap to whole rows now,
  anything else laid out at a fraction still varies, so a probe on such a row
  holds on one GPU only. A sample mask of all ones in the SDF pipelines would
  make the alpha the only coverage, it moves every fractional edge on every
  platform, so it needs a re-record of the suite.
