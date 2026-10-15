# Inspect

Remote UI inspector.

The whole inspect module sits behind the `inspect` cargo feature, off by default
(`hilen/src/inspect/mod.rs`). An app opts in on its `hilen` dependency:
`features = ["inspect"]`. Without the feature there is no server, no listener and
nothing to discover, so `hilen-inspect apps` finds nothing no matter how fresh the CLI is.
An app whose shipped artifact is a browser dist keeps the feature off the wasm target by
enabling it only through a target-conditional dependency section, see `beekeeper/web`
in the `local` repo for the pattern.

With the feature on, the app starts an inspect server at launch, a release build only with
`HILEN_INSPECT=1`, see [Release builds](#release-builds)
(`hilen/src/inspect/`): a TCP listener, advertised over mDNS
as `_hilen-inspect._tcp.local.` with the app instance id as `app_id` and the name of
the exe as `name` in the TXT record. No config,
any number of apps per machine. The first app on a device takes the port 7435,
`FIXED_PORT` in `protocol/mod.rs`, every other app takes a free port of the system. So
a tool that can not search the network still finds 1 app by the address of the device,
see `--addr` below.

On iOS the Bonjour service of the system announces, `hilen/src/inspect/bonjour.rs`,
not the `mdns-sd` crate. A real iPhone lets an app send multicast packets of its
own only with an entitlement Apple gives on request, so the crate stays silent
there with no error. The system announces for an app whose `Info.plist` has
`NSLocalNetworkUsageDescription` and names `_hilen-inspect._tcp` in
`NSBonjourServices`. `build/ios/build-project.rs` writes both keys when the built
library carries the server, and `hot_loader.plist` has them.

On a Mac the system announces and finds apps too. The `mdns-sd` crate shares the
mDNS port with the system there and often hears no answer, in a 12 second test it
got none while the system had every app, so it is on no Apple target. `discover` in
`protocol/discovery.rs` is the one search, a `ServiceBrowser` of the `discovery`
feature, see [discovery.md](discovery.md). Linux and Windows keep `mdns-sd`. A
phone on a cable answers with the address of the cable too, an address of the
real network wins.

`Client::connect` starts from the address this machine has in the network of the
app. A VPN can hold a route for the whole local network and take the packets
for a phone, a socket bound to the own address of that network goes out there.

Two clients exist:

- `inspector` — the GUI. Browses mDNS continuously, lists running apps in a dropdown
  by name and id, like `blast EENIUK`, filters out its own advertisement, and connects
  to the app picked in the list. An app on an engine from before 2026-10-12 tells no
  name and shows its id alone. `Frames` opens the frames screen for the picked app,
  see [The frames screen](#the-frames-screen).
- `hilen-inspect` — the CLI, also the interface for AI agents. Install once with
  `cargo install --path hilen-inspect`, reinstall after protocol changes. A serde error like
  `unknown field 'fit_text'` from any command means the installed CLI is older than the
  app's protocol, reinstall and retry. Commands: `apps`,
  `tree`, `view`, `find`, `wait`, `ui`, `screenshot`, `tap`, `hover`, `keys`, `hold`, `drag`, `scroll`, `scroll-to`,
  `resize`, `record`, `record-start`, `record-stop`, `frames`, `pause`, `step`, `resume`, `edit-rule`, `set-text`, `set-color`, `set-scale`,
  `edits`, `play-sound`, `run-tests`, `build-time`, `quit`, `devices`, and `hot-status` and `hot-send`
  for the loader on a device, see [hot-reload.md](hot-reload.md). The last discovery is cached in the temp dir, so repeat calls
  connect instantly and fall back to a fresh mDNS browse when the cached address is dead.
  `--addr <host>` or the variable `HILEN_INSPECT_ADDR` names the device, a name or an ip,
  with a port or with none for 7435. The tool then makes no search and uses no cache,
  for every command. That is the way to an app a search does not reach, like one on a
  phone over Tailscale, a VPN carries no mDNS. The code is `hilen-inspect/src/reach.rs`.
  `--device <word>` reaches the app on a paired iPhone, iPad or Apple TV with no search:
  the word is the id of the device from `hilen-inspect devices`, or a part of its name
  or kind, like `tv` or `ipad`. It asks the device over the tunnel of Xcode, so a
  command never lands on another device. The code is `hilen-inspect/src/device.rs`.
  `devices` lists the paired devices with the system and its version and the address
  an app answers at. After every connection the tool first asks the app for its start
  time, with a limit of 5 seconds. An app that is not in front on a phone or a TV is
  suspended: the system still takes the connection and no answer ever comes, and a
  command with no limit of its own hung for good. The tool now ends with `The app took
  the connection and gave no answer`. `hot-send` needs `--device`, `--addr` or `--app`,
  see [hot-reload.md](hot-reload.md).
  The agent workflow lives in the maintainer's skill files outside this repo.

`hilen-inspect tap` takes a query, not only an id, and matches exactly by default: exact
view id, exact visible text, then exact label field name like `save_button` or
`BackupPane.save_button`, all case insensitive, first rung with a match decides. Label
and text substring rungs run only with `--fuzzy`, so a short query cannot land on an
unrelated view. One match taps, several list the candidates and error. Hidden views and
their subtrees never match a query, only an exact id reaches them, and the app refuses to
tap a hidden view. The app also refuses a view whose center is outside the window or cut
off by a scroll view instead of pretending the tap landed. It refuses a tap whose point
another view sits over too and names that view, `tap --force` sends the touch anyway and
the reply then carries the warning. `tap --near <anchor> [--type Button]` taps the view of that type nearest
to the anchor's row, which reaches unnamed controls like the textless open button on a
list card. The anchor is an exact text or a view id. Dropdowns work like a human drives
them: tap the dropdown to open it, the reply tree already contains the open cells, then
tap the wanted cell by its text.

`hover <query> [--fuzzy]` uses the same exact matching as `tap` and moves the pointer
through the input pipeline without pressing a button. The reply names the view that
actually receives hover, or says `hovered none`. `hover <query> --wait 600` waits 600 milliseconds
before replying, long enough for the tooltip delay; the maximum wait is 60000 milliseconds.
`hover --clear` moves the pointer outside the window and clears hover and its tooltip,
while respecting an active drag's hover lock. Hidden and offscreen targets are refused.
Hover works on desktop and web; touch-only platforms return an error.

`find <query>` prints one line per match, id, label and text substrings, with window
space coordinates and a `visible`, `hidden` or `offscreen` status, `--all` includes the
last two. A view outside every parent that clips its subviews, a row scrolled out of a
list, is `offscreen` even inside the window. `wait <query> [--timeout]` polls until a visible view matches. `drag <from_x>
<from_y> <to_x> <to_y> [--steps N]` holds the left button from one window point to
another through the real input pipeline, for drag driven behavior like selecting text.
`scroll <dy>
[--at view]` injects a wheel scroll at the window center or a view's center, `scroll-to
<query>` scrolls the list that clips the target, or the window with none, until the
target is inside it. `resize <w> <h>`
resizes the window in points, desktop only. A size the OS does not give, like one
above the screen, comes back as an error with the size the window has instead. Frames in the tree are local to the parent
and do not include scrolling, absolute positions add `content_offset` down the tree,
which `find`, `wait` and `scroll-to` already do.

`hilen-inspect keys` drives the keyboard. `keys "text"` types every char in order,
`keys --key Enter` presses one winit `NamedKey` by name, and `--cmd`, `--shift` and
`--alt` hold modifiers for that one call only, so `keys p --cmd` opens a Cmd+P palette
and the next call types into it with nothing held. Keys go where a real keyboard sends
them, the focused text field and the app keymap.

`hold <keys>... [--ms 500]` holds physical keys down for a while, then releases them, for
input read every frame through `Keys::held` like a walking player. `keys` presses and
releases in one frame, so `Keys::held` never sees it. A letter or digit names its key,
anything else is a winit `KeyCode` name like `Space` or `ArrowUp`.

## Frame by frame

A screenshot is 1 frame, the one the app drew last, some frames after the input. A
layout that settles over 3 frames or a flash of 1 frame is never in it. 3 things show
every frame: a record after an input, a record from a start to a stop, and a frame
step.

A record saves every frame after an input. `tap <query> --frames 30 --out <dir>` sends
the tap and saves the next 30 frames the app draws as `000.png` to `029.png`. `000.png`
is the frame of the tap, the first one that holds what the tap did, and no frame is
skipped. The printed line of each frame has its time since the tap. `hover`, `keys` and
`scroll` take the same 2 flags. `record --frames 30 --out <dir> [--timeout 60]` sends
no input, the next press, key or wheel turn of the user starts it, so a glitch is
shown by doing it once. A record takes up to 120 frames and 1 runs at a time.

A record from a start to a stop needs no input and no frame count. `record-start`
starts it, `record-stop` stops it, and the app keeps the frames of its last 10
seconds until the next start. `frames --out <dir>` saves them as `00000.png` and so
on, named by the count of the frame from the start, and prints for each frame its
time, the gap to the frame before and its draw time. It is the record for a stretch
with no input, like a playback, and for a glitch that comes when it wants: start it,
use the app, stop it right after the glitch.

- No frame is left out. Every frame the app draws is taken, the answer says how many
  the app drew and how many the record took. When the app cannot pack the frames
  fast enough it waits for its packers, and the frame it drew after the wait is
  marked `stalled`, so the record shows its own cost. A frame is lost only when the
  packers do not answer for 2 seconds, `skipped` counts those and is 0 in a healthy
  app.
- A frame is kept at 1 pixel per point, so half the size on a screen with scale 2,
  with no loss, in the QOI format.
- A frame has 2 times. `ms` is when the engine drew it. `shown_ms` is when it
  reached the screen, told by Metal, so only on a Mac and an iPhone, and not for a
  covered window. The draw times of even frames jump by several ms, the screen
  times are exactly 1 refresh apart. On Blast the screen time was 65 ms after the
  draw time.
- In a window on screen the record asks for no frame, it takes what the app draws.
  An app that draws nothing gives no frames, that is the truth about that app and
  no fault of the record. A covered window and a headless run draw with nothing to
  pace them, there the record asks for the frames itself, 60 a second.

A frame step walks a running app. `pause` freezes the engine clock and the drawing,
`step [n]` draws the next n frames, each moves the clock by one frame of 60 a second,
and `resume` lets the app run again. An input sent while the app is paused is taken at
once but shows only in the next stepped frame, so `pause`, `tap`, `step`, `screenshot`,
`step`, `screenshot` walks through an opening screen. The window title shows the frames
stepped since the pause. A screenshot of a paused app draws what the app has again and
runs no layout, so take it after a step, right after a tap a new view has no frame yet.
The tree in the reply to a tap is from before the layout too. Only what reads the
engine clock stands still, `Animation`, `time::frame_seconds` and what is built on them. A timer of `after` and a
level on the real clock go on.

How it works:

- The code is `hilen/src/window/frame_record.rs` and `frame_step.rs`, behind the
  `inspect` feature. `frame_control.rs` is what the frame loop calls, without the
  feature every answer is the normal run.
- A record is armed in a main thread trip of its own and starts at the next input:
  a press in `Input::process_touch_event`, a key, a wheel turn, or the move of an
  inspector hover. The frame of that same loop turn is frame 0. The record counts its
  frames itself, the count is `FrameRepr.index`.
- While a record runs, every drawn frame is read back like a screenshot, and the record
  asks for the next frame, so an app that went idle still fills it.
- A covered window and a headless run draw the frames of a record and of a step
  offscreen, like a screenshot of one. Nothing paces those frames, so a record holds
  each one to the step of a screen, 60 a second from the input. Without that 30 frames
  came 1 ms apart and showed the first 30 ms of an animation.
- A paused app runs only the queued callbacks in its loop turn, so the inspector still
  gets its answers. `Clock::enter_stepped` holds the clock, a step calls
  `Clock::advance_frame` before the update.
- The record from a start to a stop is `hilen/src/window/frame_ring.rs`. The main
  thread hands each read back frame to a line of at most 16, and worker threads, 1
  per 2 cores, scale it down and pack it. With a full line the main thread waits,
  the only place where the engine blocks it on purpose. The packed frames sit in a
  map by index, a frame older than 10 seconds than the newest one is dropped.
- The screen time comes from the wgpu fork, `wgpu::hal::metal::on_next_presented`,
  see [forks.md](forks.md). `State` asks for it right before it presents a frame a
  record took. Metal calls back on a thread of its own when the frame is on screen.

The `Inspect frames` UI test proves the record after an input and the frame step on
an animation, a card that slides and changes its color in every frame. The `Inspect
record` UI test proves the record from a start to a stop on a card that slides for 2
seconds: 127 frames in 2.1 seconds, as many taken as drawn, none lost, in order, at
1 pixel per point. A headless run presents nothing, so no test proves the screen
time.

### The frames screen

The `inspector` app shows the kept record of an app. `Record` starts and stops a
record, `Reload` loads the record again, like one made with the CLI.

- A time line with 1 piece per frame, as wide as the time its picture stayed on
  screen. 2 grays take turns so the frames can be told apart. A piece has a color
  in 1 case only, orange where the app waited for the record. A picture that stays
  long has no color, on a screen where nothing moves that is normal. A click or a
  drag on the line selects the frame that was on screen at that time.
- A strip of small pictures, the selected frame in big, and under it the screen
  time, how long it stayed and the draw time.
- The left and right arrow keys step. `Play` or Space plays the record at its real
  speed, or at 0.5, 0.25 and 0.1 of it.
- `Changed pixels` paints the pixels that differ from the frame before in magenta
  and counts them.

The code is `inspector/src/ui/frames/`.

## Protocol

Lives in `hilen/src/inspect/protocol/`. Length-prefixed JSON frames over TCP
(`transport.rs`), request in, response out:

- `GetUI` — returns scale and the whole view tree as `ViewRepr`: labels, ids, frames, scroll content offsets,
    colors, texts, hidden flags, whether a view clips its subviews and placer rules.
- `SetScale(f32)` — applies the scale on the main thread.
- `Tap { view_id }` — injects a touch began plus ended at the view's center through the
  real input pipeline, exactly like a click. Refuses hidden views and views whose center
  is outside the window or cut off by a scroll view, a tap there lands nowhere while looking like a success. Replies
  with a fresh tree one frame later, so a page swap or a modal the tap triggered is
  already in it. Refuses a tap point another view sits over, by frame containment,
  transparent empty overlays excluded, a touch there once pressed a reset button. With
  `force` the touch is sent and the reply has a `note` naming that view. A covered
  window draws no frame and layout runs in a frame, so before every UI command the
  service draws 1 frame for a covered window through the screenshot path. Without it a
  screen made by the last command has every view at 0, 0.
- `Hover { view_id, wait_ms }` — injects a cursor move at the view center, or clears
  hover as a cursor leave when `view_id` is absent. The optional `wait_ms` defaults to
  zero and waits on the inspector worker, leaving the UI free to show a tooltip.
  Replies with a fresh tree and a note naming the actual hovered view and its id.
- `Scroll { view_id, dx, dy }` — moves the cursor to the view's center, or the window
  center with no view, then injects a wheel scroll, so it lands on the deepest scroll
  view under that point like a real wheel.
- `Resize { width, height }` — resizes the window, points, desktop only.
- `Keys { keys, modifiers }` — plays a list of `Key::Char` and `Key::Named(NamedKey)`
  through `Input::on_char` and `Input::on_key` in one main thread trip, the same entry
  points the winit key handler uses, so a named key also fires its text char like a real
  key. The modifiers hold for this request only and reset to empty after it, a stuck Cmd
  can never leak into later input. Replies with a fresh tree one frame later.
- `Hold { keys, ms }` — sets the `KeyCode`s held in `Keys`, waits `ms` on the engine
  timer so frames keep running, then releases them. Replies with a fresh tree.
- `EditRule { view_id, rule_index, offset, enabled }` — edits a placer rule of the live
  view. Offset applies to Side and Anchor rules and edits the ratio of Relative rules.
- `SetText { view_id, text }` — sets the text of a live `Label`, `Button` or `TextField`.
- `SetColor { view_id, color }` — sets the background color of a live view.
- `Screenshot` — returns the current frame as base64 PNG. Works headless too. An idle app renders a frame for it on demand, and an occluded or hidden window answers from the offscreen scene path, so the command never waits for the window to become visible.
- `Record { frames, input, wait_ms }` — arms a record, plays `input` when there is one,
  a `Tap`, `Keys`, `Hover`, `Scroll` or `Drag`, and replies with `Frames`, every frame
  as base64 PNG with its `index` from the input and its `ms`. With no input it waits
  up to `wait_ms` for an input of the user and replies with an error when none came.
- `RecordStart`, `RecordStop`, `RecordInfo` — the record from a start to a stop. Each
  replies with `Record`, a `RecordRepr`: whether it runs, the frames drawn, taken,
  lost and waited at, and a `FrameInfoRepr` per kept frame with no picture.
- `RecordFrames { from, count }` — replies with `KeptFrames`, the pictures of at most
  `count` kept frames from the index `from` on, each as base64 QOI with its
  `FrameInfoRepr`. At most 120 in 1 answer, a client asks in pieces.
- `Pause`, `Step { frames }`, `Resume` — the frame step. `Pause` and `Step` reply with
  `Paused { frame }`, the frames drawn since the pause. `Step` on an app that is not
  paused is an error.
- `ListEdits` — returns every edit applied in this session.
- `GetBuildTime` — unix seconds of when the code of the app was compiled, the answer of
  `hilen_app_build_time` in `hilen/src/app.rs`. `hilen-inspect build-time` compares it to
  the newest source of the folder it is run from, which it prints, and combines it with
  `GetStartTime`, the unix seconds when the app process started. Source newer than the
  process is stale. Source older than the process but newer than the stamp is stale too:
  the binary was started after the edit and its code is from before it. The stamp has to
  live in the Rust code: an iOS build relinks the `.app` every time while reusing a stale
  `libdemo.a`, so the bundle's own timestamp, md5 and install all report fresh while old
  code runs.

  The stamp is made by `register_app!`, which calls the proc macro `build_time!()` of
  `project-proc` in the final crate of the app. Cargo compiles that crate again after any
  change below it, the engine included, so the number moves with every real build of the
  app. Before this the stamp came from `hilen/build.rs` alone and answered "when was the
  engine compiled". Change only `ui-test-suite` or `demo` and cargo rightly left `hilen`
  alone, so a correctly rebuilt app reported stale. An app with no `register_app!` still
  answers with that engine stamp, through the weak `hilen_app_build_time` of the engine.

  2 things can still make a current app read as stale. A source file in the folder that
  is not part of the app, like a backend crate in the same repo, is newer and a build
  compiles nothing. And the stamp is the clock of the machine that compiles, so a build
  on another machine whose clock is behind looks older than a local edit.
- `GetStartTime` — unix seconds of when the current process started, recorded before the
  app runner launches. Used with `GetBuildTime` to distinguish a source edit made after
  launch from an app-only source edit already present when the process started.
- `Speed { to_app, ms }` — a run of the speed test, see "Speed test" below. Its bytes
  are not all JSON.
- `HotBytes { file, offset, len }` — a piece of a file for a hot build. Its `len` bytes
  follow the command on the connection as they are, see [hot-reload.md](hot-reload.md).
- `Quit` — closes the app the way closing its window does, through `AppRunner::stop`,
  and replies before it exits. Desktop only, elsewhere it replies with an error. The
  flag is read where the event loop is about to wait and the stop wakes the loop, so an
  idle or covered window quits too. `hilen-inspect quit [--timeout 10]` then waits until
  the app closes the connection, which it does only when its process ends.
- `PlaySound` — plays a sound in the app, for finding which instance is which.
- `RunTests` — runs the app's whole UI test suite in the app and returns the total and
  every failure. Needs nothing from the app: every test registers into the engine's own
  `UI_TESTS`, so `ui_test::run_all_tests` reaches whatever the app links. The run happens
  on a tokio task, never the main thread, because the tests drive the main thread through
  `from_main`. The runner forces the harness preconditions the tests expect, scale 1 and 32
  point text, and takes the app's global styles away for the duration, or an app style such
  as a themed `Button` colour would fail every color check. It puts all of that back
  afterwards and rebuilds the app's root view, see [ui-tests.md](ui-tests.md).

Edits reply with a fresh tree snapshotted one frame later, after layout ran, so the client
never sees stale frames. Failures (unknown view id, bad rule index, view without text)
reply with `Error(String)` instead of being silently ignored. All UI access happens on the
main thread via `from_main`. Responses hold `Own` pointers, so the transport hands them to
the main thread for dropping (see [refs.md](refs.md)).

## Speed test

`hilen-inspect speed [--seconds 3]` measures the link between the tool and an app, with
`--device` or `--addr` like every command. It prints the time of a round trip, the least
and the average of 10 small questions, then the MB per second to the app and from the
app, with a line per second.

- It measures like iperf: raw bytes on the connection for a fixed time, counted on the
  side that gets them. No JSON, no base64 and no wait per piece are in the number.
- A run starts with the `Speed` command. The bytes follow on the same connection in
  pieces of 256 KB, each behind its length as 4 bytes, and a length of 0 ends the run.
  Then the app answers `Speed { bytes, ms }`. `handle_connection` in `transport.rs` runs
  it on the connection, the handler of the service never sees it, so the number waits
  on no frame of the app.
- To the app the run ends when the app said how many bytes it got. Bytes that still
  wait in a buffer do not count as fast.
- The bytes have no pattern, so nothing on the way can pack them.
- An app sends for 30 seconds at most, whatever the client asks.
- An app built before 2026-10-13 does not know the command and closes the connection.
  The tool says so.
- A browser page has no TCP connection, the test is for native apps only.
- Measured on 2026-10-13 from an M1 mac to an iPhone 16 Pro Max on Wi-Fi: 14 ms for a
  round trip, 44 to 50 MB a second in both directions. The hot send of the 45 MB
  library of banda in the same minute made 12.7 MB a second, `times:` in its output.
  The link is not what limits that send.
- The first run right after a hot swap ended once with a broken pipe after 2 seconds.
  5 runs after it passed, the cause is not known.

## Browser transport

A page cannot listen on TCP and has no mDNS, so on wasm the direction inverts.
With the `hilen_inspect` query flag set, the app dials out to the server that
served the page, same origin, at `/hilen-inspect`, in `web_transport.rs`. One
WebSocket text message carries one JSON frame, request in, response out, the
same protocol types as TCP. Requests are processed on one dedicated worker
thread, since commands block on `from_main` and `RunTests` runs the whole
suite. Responses serialize on that worker and their `Own` pointers drop on the
main thread, like the TCP transport. The test autorun also pushes its report
over the socket as an unsolicited frame, which is how the browser test lane
reads results without console access, see [ui-tests.md](ui-tests.md). A failing
test pushes a `FailureScreenshot` frame the same way, its frame as PNG, which the
driver writes under `target/web-test/failures/`. Every log line at info and above
goes out as a `Log` frame while the socket is open, from the logger in
`web_log.rs`, so the driver sees the app's progress live. A `Log` frame that fails
to send says nothing, a log line about it would forward itself without end. Panics
POST to `/te-panic` on the page origin from the panic hook through a sync XHR,
which delivers before the wasm instance dies and works from workers too.

## Edit log

Every applied edit (`edit_log.rs`) is kept in memory for `ListEdits` and appended as a JSON
line to `inspect-edits.jsonl` in the `target` folder the running exe sits in: timestamp,
view label and id, what changed, old and new values. The file survives app restarts. An exe
outside a `target` folder, an installed app or a device build, keeps only the in-memory list.

## Release builds

The server lets anyone on the network read and drive the app, so a shipped app must never
carry it. Kukareker shipped it in every release up to v0.5.16 because the cargo feature was
the only gate. Now four independent checks stand in the way, so one mistake is not enough.

- **Build block.** `hilen/build.rs` fails a build that has the feature while `HILEN_RELEASE`
  is set, with no override. The logic is in `hilen/src/inspect/release_guard.rs`, shared with
  its unit tests. `build/release/with-secrets.sh` sets the mark, and the release scripts
  `mac.rs`, `linux.rs`, `win.rs` and `docker.rs` set it again on their own.
- **Runtime gate.** A build without debug assertions starts the listener only when
  `HILEN_INSPECT=1` is set at launch, and then logs a warning. A hot build is a release
  build that listens always, see "A release build" in [hot-reload.md](hot-reload.md). A release binary built by hand,
  outside the scripts, starts with the server off. Lanes that build release with the feature,
  `make uui` and `make bench`, do not use the listener. An app whose `make run` is a release
  build sets the variable there, like kukareker. 1 app is the exception: an iOS
  app with the bundle id `vladas.test-engine`, the engine's own demo, starts the
  listener in every build, `ALWAYS_ON_BUNDLE_ID`. It ships only through
  TestFlight, to be inspected on a phone. No other id may be added.
- **File scan.** Every build with the module carries the `MARKER` bytes of
  `hilen/src/inspect/mod.rs`, the start functions keep them alive. `build/shared/src/inspect.rs`
  searches the built binary for them, and the release scripts stop before anything is signed,
  packed or copied to `dist`. The iOS and Android build paths run the same scan when
  `HILEN_RELEASE` is set, since their test builds carry the server on purpose.
- **Web dist scan.** In a release build, `web_mount` of `hilen-server` searches the embedded
  wasm for the marker and panics at start, so a deploy of a dist with the feature fails. The
  browser transport only dials the page's own origin, but it still must not ship.

The marker has three copies, in the engine, in `hilen-server` and in `build/shared`. The last
two hold it in two pieces, so a scan of their own binaries stays clean. A `hilen-server` test
checks its copy against the engine.

An app turns the feature on only for its dev loop. A desktop app puts it behind its own
feature, like `dev = ["hilen/inspect"]` in kukareker, which `make run` passes through
`RUN_FEATURES` of `build/run.sh`. An app whose shipped artifact is a browser dist enables it
only through a target-conditional dependency, see above. The host-side tools `inspector` and
`hilen-inspect` build in release like any other crate, `hilen-inspect` is excluded from
default workspace members.

The request frame of the TCP transport is capped at 64 MiB, a bigger length prefix closes the
connection before any buffer is allocated.

## Local hook

Unrelated to the remote inspector: pressing `i` in any app calls the per-view
`Setup::inspect()` hook recursively. Default is empty, override it for ad hoc debugging.
