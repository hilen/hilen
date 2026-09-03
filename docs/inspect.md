# Inspect

Remote UI inspector.

The whole inspect module sits behind the `inspect` cargo feature, off by default
(`hilen/src/inspect/mod.rs`). An app opts in on its `hilen` dependency:
`features = ["inspect"]`. Without the feature there is no server, no listener and
nothing to discover, so `hilen-inspect apps` finds nothing no matter how fresh the CLI is.
An app whose shipped artifact is a browser dist keeps the feature off the wasm target by
enabling it only through a target-conditional dependency section, see `beekeeper/web`
in the `local` repo for the pattern.

With the feature on, the app starts an inspect server at launch
(`hilen/src/inspect/`): a TCP listener on an OS-assigned port, advertised over mDNS
as `_hilen-inspect._tcp.local.` with the app instance id in the TXT record. No config, no
fixed ports, any number of apps per machine.

Two clients exist:

- `inspector` — the GUI. Browses mDNS continuously, lists running apps in a dropdown,
  filters out its own advertisement.
- `hilen-inspect` — the CLI, also the interface for AI agents. Install once with
  `cargo install --path hilen-inspect`, reinstall after protocol changes. A serde error like
  `unknown field 'fit_text'` from any command means the installed CLI is older than the
  app's protocol, reinstall and retry. Commands: `apps`,
  `tree`, `view`, `find`, `wait`, `ui`, `screenshot`, `tap`, `keys`, `drag`, `scroll`, `scroll-to`,
  `resize`, `edit-rule`, `set-text`, `set-color`, `set-scale`,
  `edits`, `play-sound`, `run-tests`, `build-time`. The last discovery is cached in the temp dir, so repeat calls
  connect instantly and fall back to a fresh mDNS browse when the cached address is dead.
  The agent workflow lives in the maintainer's skill files outside this repo.

`hilen-inspect tap` takes a query, not only an id, and matches exactly by default: exact
view id, exact visible text, then exact label field name like `save_button` or
`BackupPane.save_button`, all case insensitive, first rung with a match decides. Label
and text substring rungs run only with `--fuzzy`, so a short query cannot land on an
unrelated view. One match taps, several list the candidates and error. Hidden views and
their subtrees never match a query, only an exact id reaches them, and the app refuses to
tap a hidden view. The app also refuses a view whose center is outside the window instead
of pretending the tap landed, and the reply carries a warning when another view sits over
the tap point. `tap --near <anchor> [--type Button]` taps the view of that type nearest
to the anchor's row, which reaches unnamed controls like the textless open button on a
list card. The anchor is an exact text or a view id. Dropdowns work like a human drives
them: tap the dropdown to open it, the reply tree already contains the open cells, then
tap the wanted cell by its text.

`find <query>` prints one line per match, id, label and text substrings, with window
space coordinates and a `visible`, `hidden` or `offscreen` status, `--all` includes the
last two. `wait <query> [--timeout]` polls until a visible view matches. `drag <from_x>
<from_y> <to_x> <to_y> [--steps N]` holds the left button from one window point to
another through the real input pipeline, for drag driven behavior like selecting text.
`scroll <dy>
[--at view]` injects a wheel scroll at the window center or a view's center, `scroll-to
<query>` repeats window sized steps until the target is on screen. `resize <w> <h>`
resizes the window in points, desktop only. Frames in the tree are local to the parent
and do not include scrolling, absolute positions add `content_offset` down the tree,
which `find`, `wait` and `scroll-to` already do.

`hilen-inspect keys` drives the keyboard. `keys "text"` types every char in order,
`keys --key Enter` presses one winit `NamedKey` by name, and `--cmd`, `--shift` and
`--alt` hold modifiers for that one call only, so `keys p --cmd` opens a Cmd+P palette
and the next call types into it with nothing held. Keys go where a real keyboard sends
them, the focused text field and the app keymap.

## Protocol

Lives in `hilen/src/inspect/protocol/`. Length-prefixed JSON frames over TCP
(`transport.rs`), request in, response out:

- `GetUI` — returns scale and the whole view tree as `ViewRepr`: labels, ids, frames, scroll content offsets,
    colors, texts, hidden flags and placer rules.
- `SetScale(f32)` — applies the scale on the main thread.
- `Tap { view_id }` — injects a touch began plus ended at the view's center through the
  real input pipeline, exactly like a click. Refuses hidden views and views whose center
  is outside the window, a tap there lands nowhere while looking like a success. Replies
  with a fresh tree one frame later, so a page swap or a modal the tap triggered is
  already in it, plus an optional `note` naming the view sitting over the tap point when
  frame containment says the touch may land elsewhere, transparent empty overlays
  excluded.
- `Scroll { view_id, dx, dy }` — moves the cursor to the view's center, or the window
  center with no view, then injects a wheel scroll, so it lands on the deepest scroll
  view under that point like a real wheel.
- `Resize { width, height }` — resizes the window, points, desktop only.
- `Keys { keys, modifiers }` — plays a list of `Key::Char` and `Key::Named(NamedKey)`
  through `Input::on_char` and `Input::on_key` in one main thread trip, the same entry
  points the winit key handler uses, so a named key also fires its text char like a real
  key. The modifiers hold for this request only and reset to empty after it, a stuck Cmd
  can never leak into later input. Replies with a fresh tree one frame later.
- `EditRule { view_id, rule_index, offset, enabled }` — edits a placer rule of the live
  view. Offset applies to Side and Anchor rules and edits the ratio of Relative rules.
- `SetText { view_id, text }` — sets the text of a live `Label`, `Button` or `TextField`.
- `SetColor { view_id, color }` — sets the background color of a live view.
- `Screenshot` — returns the current frame as base64 PNG. Works headless too. An idle app renders a frame for it on demand, and an occluded or hidden window answers from the offscreen scene path, so the command never waits for the window to become visible.
- `ListEdits` — returns every edit applied in this session.
- `GetBuildTime` — unix seconds of when `hilen` was compiled, stamped by
  `hilen/build.rs`. `hilen-inspect build-time` compares it to the newest source here and
  combines it with `GetStartTime`, the unix seconds when the app process started. Source
  newer than the process is definitely stale. Source older than the process but newer than
  the engine build is reported as inconclusive: it can be a current app-only rebuild or a
  stale reused Rust library. The stamp has to live in the Rust code: an iOS
  build relinks the `.app` every time while reusing a stale `libdemo.a`, so the
  bundle's own timestamp, md5 and install all report fresh while old code runs.

  It stamps **`hilen`**, not the app, so it answers "when was the engine compiled",
  not "is this app current". Change only `ui-test-suite` or `demo` and cargo rightly
  leaves `hilen` alone, so a correctly rebuilt app reports stale. That is a false
  positive, and it has already happened. Treat a stale verdict as a reason to check, not as
  proof: something that only the new code produces, a test count or an `nm` symbol, settles
  it. A fresh verdict is still worth having, it catches the case that matters, a `.a` that
  never rebuilt.
- `GetStartTime` — unix seconds of when the current process started, recorded before the
  app runner launches. Used with `GetBuildTime` to distinguish a source edit made after
  launch from an app-only source edit already present when the process started.
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
driver writes under `target/web-test/failures/`. Panics
POST to `/te-panic` on the page origin from the panic hook through a sync XHR,
which delivers before the wasm instance dies and works from workers too.

## Edit log

Every applied edit (`edit_log.rs`) is kept in memory for `ListEdits` and appended as a JSON
line to `target/inspect-edits.jsonl` under the app's git root: timestamp, view label and
id, what changed, old and new values. The file survives app restarts. Outside a git repo,
on a device for example, only the in-memory list works.

## Release builds

The `inspect` cargo feature is the one and only gate, by design. With the feature on the
server works the same in debug and release builds, there is no `debug_assertions` gating.
The flip side: an app that enables the feature carries the server in its release builds
too, so a shipping app keeps the feature out of its shipped targets, which is what the
target-conditional dependency pattern above does. The host-side tools `inspector` and
`hilen-inspect` build in release like any other crate, `hilen-inspect` is excluded from default
workspace members.

## Local hook

Unrelated to the remote inspector: pressing `i` in any app calls the per-view
`Setup::inspect()` hook recursively. Default is empty, override it for ad hoc debugging.
