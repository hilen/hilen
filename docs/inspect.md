# Inspect

Remote UI inspector.

The whole inspect module sits behind the `inspect` cargo feature, off by default
(`test-engine/src/inspect/mod.rs`). An app opts in on its `test-engine` dependency:
`features = ["inspect"]`. Without the feature there is no server, no listener and
nothing to discover, so `te-inspect apps` finds nothing no matter how fresh the CLI is.
An app whose shipped artifact is a browser dist keeps the feature off the wasm target by
enabling it only through a target-conditional dependency section, see `beekeeper/web-te`
in the `local` repo for the pattern.

With the feature on, the app starts an inspect server at launch
(`test-engine/src/inspect/`): a TCP listener on an OS-assigned port, advertised over mDNS
as `_te-inspect._tcp.local.` with the app instance id in the TXT record. No config, no
fixed ports, any number of apps per machine.

Two clients exist:

- `inspector` — the GUI. Browses mDNS continuously, lists running apps in a dropdown,
  filters out its own advertisement.
- `te-inspect` — the CLI, also the interface for AI agents. Install once with
  `cargo install --path te-inspect`, reinstall after protocol changes. A serde error like
  `unknown field 'fit_text'` from any command means the installed CLI is older than the
  app's protocol, reinstall and retry. Commands: `apps`,
  `tree`, `view`, `ui`, `screenshot`, `tap`, `edit-rule`, `set-text`, `set-color`, `set-scale`,
  `edits`, `play-sound`, `run-tests`, `build-time`. The last discovery is cached in the temp dir, so repeat calls
  connect instantly and fall back to a fresh mDNS browse when the cached address is dead.
  The agent workflow lives in the maintainer's skill files outside this repo.

`te-inspect tap` takes a query, not only an id: exact view id, exact visible text, label
substring, then text substring, all case insensitive, first rung with a match decides.
One match taps, several list the candidates and error. Hidden views and their subtrees
never match a query, only an exact id reaches them, and the app refuses to tap a hidden
view. Dropdowns work like a human drives them: tap the dropdown to open it, the reply
tree already contains the open cells, then tap the wanted cell by its text.

## Protocol

Lives in `test-engine/src/inspect/protocol/`. Length-prefixed JSON frames over TCP
(`transport.rs`), request in, response out:

- `GetUI` — returns scale and the whole view tree as `ViewRepr`: labels, ids, frames,
  colors, texts, hidden flags and placer rules.
- `SetScale(f32)` — applies the scale on the main thread.
- `Tap { view_id }` — injects a touch began plus ended at the view's center through the
  real input pipeline, exactly like a click. Refuses hidden views. Replies with a fresh
  tree one frame later, so a page swap or a modal the tap triggered is already in it.
- `EditRule { view_id, rule_index, offset, enabled }` — edits a placer rule of the live
  view. Offset applies to Side and Anchor rules and edits the ratio of Relative rules.
- `SetText { view_id, text }` — sets the text of a live `Label`, `Button` or `TextField`.
- `SetColor { view_id, color }` — sets the background color of a live view.
- `Screenshot` — returns the current frame as base64 PNG. Works headless too.
- `ListEdits` — returns every edit applied in this session.
- `GetBuildTime` — unix seconds of when `test-engine` was compiled, stamped by
  `test-engine/build.rs`. `te-inspect build-time` compares it to the newest source here and
  combines it with `GetStartTime`, the unix seconds when the app process started. Source
  newer than the process is definitely stale. Source older than the process but newer than
  the engine build is reported as inconclusive: it can be a current app-only rebuild or a
  stale reused Rust library. The stamp has to live in the Rust code: an iOS
  build relinks the `.app` every time while reusing a stale `libtest_game.a`, so the
  bundle's own timestamp, md5 and install all report fresh while old code runs.

  It stamps **`test-engine`**, not the app, so it answers "when was the engine compiled",
  not "is this app current". Change only `ui-test-suite` or `test-game` and cargo rightly
  leaves `test-engine` alone, so a correctly rebuilt app reports stale. That is a false
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
With the `te_inspect` query flag set, the app dials out to the server that
served the page, same origin, at `/te-inspect`, in `web_transport.rs`. One
WebSocket text message carries one JSON frame, request in, response out, the
same protocol types as TCP. Requests are processed on one dedicated worker
thread, since commands block on `from_main` and `RunTests` runs the whole
suite. Responses serialize on that worker and their `Own` pointers drop on the
main thread, like the TCP transport. The test autorun also pushes its report
over the socket as an unsolicited frame, which is how the browser test lane
reads results without console access, see [ui-tests.md](ui-tests.md). Panics
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
`te-inspect` build in release like any other crate, `te-inspect` is excluded from default
workspace members.

## Local hook

Unrelated to the remote inspector: pressing `i` in any app calls the per-view
`Setup::inspect()` hook recursively. Default is empty, override it for ad hoc debugging.
