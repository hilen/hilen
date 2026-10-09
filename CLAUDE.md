# Hilen

Cross platform game engine and UI framework in Rust. Rendering on WGPU.
Supports: Windows, Linux, Mac, iOS, Android and WebAssembly.

The engine is one library crate, `hilen`, with modules like `gm`, `ui`, `window`,
`render`, `level` under `hilen/src/`. The foundational crates `hreads`, `refs`, `vents`
and `netrun` are modules under `hilen/src/deps/`, not separate crates, so a published
`hilen` is one self contained library. `deps/` holds only the proc macro crates,
`project-proc` among them, which reads `project_name` from `hilen.toml` at build time
for `hilen` and `hilen-server`,
`ui-proc-test`, the compile check for the `view` macro, `hilen-pixels`, the per pixel
loops kept optimized in dev builds by a profile override, plus `plat`, which stays its
own crate because three build scripts call its `platforms()` to set the cfg aliases and
a crate cannot use its own code in its build script.
Apps and test binaries are separate crates on top. Internals are `pub(crate)`, the
app-facing API is `pub` — keep new items `pub(crate)` unless apps need them, so the
`dead_code` lint stays meaningful.

`hilen-server` is the backend base crate for app backends, config, error type,
base routes and helpers over axum, sqlx and redis, on Postgres or on SQLite
through its `Db` type, Redis optional. It also carries the standard
way to serve an app's trunk-built wasm dist, `web_mount` in `src/web.rs`, the
dist embeds via rust-embed with SPA fallback and `HILEN_WEB_DEV_PROXY` points
page requests at a running `trunk serve` for the dev loop. Its `auth` module is
the server half of the Google and Apple login, see [docs/login.md](docs/login.md). Its
`prometheus` module serves Prometheus metrics: `install_metrics` once at startup,
the re-exported `metrics` macros to record, and `metrics_mount` for a `/metrics`
route behind a Bearer token, since a stack with a public host has only one port
to share. It never
links the `hilen` UI crate, a backend and a client only share the wire.
Its own queries use the plain `query` functions of sqlx, because the login code runs
the same query on both databases through `Db` and a checked macro can check against
only 1. A backend built on it has 1 database and writes its queries with the checked
macros, `query!`, `query_as!` and `query_scalar!`, with the saved answers in a `.sqlx`
folder in git.

`hilen-updater` is the self update with no UI dependency, the check, the signed
download, the swap and the relaunch. `hilen` builds `system::Updater` on it and
re-exports it as `hilen::updater`, and a binary with no window uses it directly,
see [docs/updater.md](docs/updater.md).

The UI tests are their own crate, `ui-test-suite`, so `demo` can link it and carry
every test onto a device. It must never depend on `demo`, that is a cycle, since the
`ui-test` runner links both. Level tests, a `#[level]` with `impl LevelTest`, register
into `hilen::LEVEL_TESTS` the same way and run through the `level-test` crate, which
also holds the tests. Scene tests, a `#[scene]` with `impl SceneTest`, register into
`hilen::SCENE_TESTS`, live in `scene-test-suite` like the UI tests, and run through
`scene-test` on desktop and through `demo` on a device, after the UI tests in one
report. `render-test` is only for the render pipelines drawn directly.

Optional engine parts sit behind cargo features, all off by default. `level` is the physics
levels, the no physics game scene, rapier and the sprite, polygon and background pipelines
with their shaders. `audio` is sound playback through kira and its decoders, silent instead of a panic on a machine with no output device. `video` is video
playback, on desktop and iOS through a prebuilt static ffmpeg and kira, proven on macOS, Windows x64 and the iOS simulator, in a browser
through a `<video>` element under the canvas. Where ffmpeg decodes it also gives pictures of a file with no view, a list of pieces played as 1 video and the export to an mp4 file, see [docs/video.md](docs/video.md). `inspect` is
the remote inspector. `hot` is hot reload in the iOS simulator and on a real iPhone, the app as a dynamic library that a loader app swaps while it runs, see [docs/hot-reload.md](docs/hot-reload.md). `scene` is the 3D twin of `level`, physics on rapier3d and glam, its own
`#[scene]` macro, `scene-test` crate and `SCENE_TESTS` registry, see [docs/scene.md](docs/scene.md).
`login` is the Google and Apple login client, the `GoogleLoginButton` and `AppleLoginButton` views, the login from a phone for a device with no keyboard with its `QrCodeView`, and the sealed `SessionStore`,
see [docs/login.md](docs/login.md). `google-access` is Google API access for an app that keeps its Google tokens on the device, `GoogleAccess`, `GoogleAccounts` and `SecretStore`, see [docs/google-access.md](docs/google-access.md). `ui-tests` and `level-tests` register tests. A GUI only app depends
on `hilen` with none of them and the wasm drops rapier, kira and the codecs entirely.
`demo` turns `audio`, `inspect`, `level` and `ui-tests` on, `video` on macOS, Windows, iOS and wasm, and `scene`
through its own default `scene` feature, so `--no-default-features` builds it with no 3D.

No proof, no merge. A performance claim needs an A/B per [docs/benchmark.md](docs/benchmark.md)
acceptance criteria, a correctness claim needs a reproduced failure. Unproved ideas go to
[docs/guesses.md](docs/guesses.md), not into the code.

Every new UI feature or bugfix must land together with a new UI test that covers it. No exceptions.
That test must run on every supported UI-test platform where the production feature exists.
A large canvas, desktop scale, fixture layout or easier reproduction is never a reason to gate it
to desktop; adapt the test while reusing the real production view and behavior.
See [docs/ui-tests.md](docs/ui-tests.md) for how UI tests work.

## Docs

Do not read these upfront. Read the matching file only when the task touches that area:

- [docs/colors.md](docs/colors.md) — the encoded sRGB convention, `Color::hex`, why targets
  are plain Unorm. Read before touching color types, surface formats, or blending.
- [docs/refs.md](docs/refs.md) — `Own`/`Weak` smart pointers, the memory model. Read before
  working with view lifetimes, pointers, or anything from the `refs` crate.
- [docs/dispatch.md](docs/dispatch.md) — main thread rules, `on_main`/`from_main`, frame loop.
  Read before touching threading, async, or dispatch code.
- [docs/ui-tests.md](docs/ui-tests.md) — how UI tests work and how to run a single one.
  Also `system_input`, real taps and screen keyboard keys through the XCUITest helper.
  Read before writing or debugging UI tests.
- [docs/inspect.md](docs/inspect.md) — the remote UI inspector, its protocol and the
  off-by-default `inspect` feature gate. Read before touching `hilen/src/inspect`,
  the `inspector` app, or the `hilen-inspect` CLI.
- [docs/benchmark.md](docs/benchmark.md) — the UI benchmark, its consistency guard, and the
  results history in `bench/`. Read before touching the benchmark or measuring performance.
- [docs/guesses.md](docs/guesses.md) — parked changes that lacked proof. Read before
  proposing an optimization or a speculative fix; add new unproved ideas there, not to code.
- [docs/text.md](docs/text.md) — the text pipeline: the bundled fonts, `MarkdownView` and its style, rustybuzz shaping, em sizing, variable
  font instances, letter spacing, line handling. Read before touching label rendering,
  fonts, or `hilen/src/window/text`.
- [docs/text-selection.md](docs/text-selection.md) — text that selects and copies in a `Label`, a
  `MarkdownView` and across the cells of a `TableView`: the calls, what a copy holds, positions kept
  in the data, the hooks in the input code and how the highlight is drawn. Read before touching
  `hilen/src/ui/text_selection`, `selection_drawer.rs` or the touch code in `view_touch.rs`.
- [docs/roadmap.md](docs/roadmap.md) — missing engine features found by porting a real app,
  with current state, design notes, and order. Read before planning or starting a new
  engine capability, and update it when one lands.
- [docs/pixdiff.md](docs/pixdiff.md) — the `hilen-pixdiff` pixel parity tool: capture app
  windows from the screen, resize both apps to one size, diff the captures into ranked
  regions. Read before comparing a port against its original or touching `hilen-pixdiff`.
- [docs/updater.md](docs/updater.md) — `system::Updater` and the `hilen-updater` crate under it, the manifest schema, the ed25519
  signing contract and what the in place swap means for packaging. Read before wiring
  self update into an app or a daemon, or touching `hilen/src/system/updater.rs` or `hilen-updater`.
- [docs/windows.md](docs/windows.md) — why Windows renders through DX12, the silent Intel
  Vulkan crash it avoids, and how to read a `0xc0000005` from the event log. Read before
  changing backend selection or when an app dies on Windows with no message.
- [docs/wsl.md](docs/wsl.md) — running on Windows through WSL: the packages `make setup`
  installs, why the engine forces X11, takes the scale and follows the Windows theme
  through WSLg, and what to check when no window appears. Read before touching
  `window/wsl/` or when an app shows nothing under WSL.
- [docs/android.md](docs/android.md) — the docker build and the emulator lane, the
  Vulkan-only backend, APK asset loading, the register requirement in the shell crate,
  and the generated-project fixes the template still misses. Read before touching
  android builds or when an APK dies at startup.
- [docs/ios.md](docs/ios.md) — what keeps iOS 12 and the A7 working: `NSLog` output, the
  ObjC exception preprocessor, the two version settings that look alike, the weak linked
  CoreGraphics and the wgpu fork. Also the system text field an edited `TextField` becomes
  on an iPhone, and its native file inside the engine crate. Read before touching anything iOS, the `wgpu` pin, the
  iOS deployment target, or when an app dies on a device with no message.
- [docs/screen-keyboard.md](docs/screen-keyboard.md) — the screen keyboard of a phone: the
  edited field that stays in view, `b_keyboard` for a view that rides on the keyboard,
  `ScreenKeyboard`, why `TextField::focus` does nothing on a phone, and the several
  moves iOS reports for 1. Read before touching `hilen/src/ui/screen_keyboard.rs`, the
  keyboard hook in `hilen_text.m` or `TextField::focus`.
- [docs/tvos.md](docs/tvos.md) — tvOS builds and renders in the Apple TV simulator,
  display only, no key of the remote reaches the engine yet. The vendored plat, the winit fork pin, the hand made
  simulator shell and how to run it. Read before touching platform cfg aliases, the
  winit pin, or anything tvOS.
- [docs/canvas.md](docs/canvas.md) — `CanvasView`, how a subtree is drawn at a scale,
  the flat layer that lets a view over a canvas cover it, and the pinch from 2 fingers
  and from a trackpad. Read before touching `views/containers/canvas`, `input/pinch.rs`,
  `calculate_absolute_frame`, or the depth order of hover and touches.
- [docs/clipboard.md](docs/clipboard.md) — `Clipboard`: text, a secret, and a picture as a
  png file with `get_image`, what each platform does, why the png is made off the main
  thread, the `image_pasted` event of `TextField`, and how a test stays away from the
  clipboard of the user. Read before touching `hilen/src/system/clipboard` or the paste
  of a text field.
- [docs/file-browser.md](docs/file-browser.md) — `FileBrowser` and `FilePicker`, the
  `FileSource` trait with `LocalFiles` and `MemoryFiles`, the pick modes, the touch
  screen taps, the keys and the tests. Read before touching
  `views/complex/file_browser` or `filesystem/source`, or putting a file browser into
  an app.
- [docs/focus.md](docs/focus.md) — the key focus for a TV remote and a keyboard: how the
  ring picks the next view, tables, modals, scrolling, Back as Escape, and the calls an
  app has. Read before touching `hilen/src/ui/focus.rs` or key handling in `input.rs`.
- [docs/webos.md](docs/webos.md) — an app on an LG TV as a web page: `make webos`, the
  `[webos]` table of `hilen.toml`, the Chromium 79 build flags, the start script rewrite, what the TV needed from the engine, the plain
  canvas format, no texture copy, 1 sample, lazy pictures, shared text pipelines, and
  how to read a page log and run the UI suite on the TV. Read before touching the wasm
  build for an old browser, `window/msaa.rs`, `window/image/pending.rs`, or anything
  webOS.
- [docs/level.md](docs/level.md) — the 2D `level` module: the fixed step on the real
  clock, level and screen points, the mouse, sprite flip, tint and pixel art filter,
  tile maps with their collision, step up and neighbor framing, the dug polygon
  terrain, and the lights. Read before touching `hilen/src/level`, `level_drawer.rs`,
  the sprite shaders or `terrain.wgsl`.
- [docs/scene.md](docs/scene.md) — the 3D `scene` module: the level shaped architecture, glb
  models with skins and clips, the sun's shadow map, touch picking, the depth band it draws in, the A7 varying
  budget of the mesh shader, scene tests and what is still to come. Read before touching
  `hilen/src/scene`, `scene_drawer.rs`, the mesh pipeline or a scene test.
- [docs/video.md](docs/video.md) — the `video` feature: `VideoView`, the ffmpeg decode thread and
  hardware devices, the NV12 pass, kira as the clock, the prebuilt static ffmpeg archives and
  how to build one, request headers, the buffering state, tracks and subtitles, HDR tone
  mapping, playback speed, what was measured, the iOS side with its audio session, and the browser side, a `<video>` element
  under a hole in the frame. Read before touching `hilen/src/video` or the archive script.
- [docs/google-access.md](docs/google-access.md) — Google API access with the tokens on the device:
  the 1 Web client on the backend, the sealed hand over between `hilen::google_access` and
  `hilen_server::google_access`, the refresh that stores nothing, `GoogleAccounts` with a main
  account and linked accounts in the Drive app folder, and `SecretStore`. Read before touching
  those modules, `hilen-session/src/secret_store.rs` or `device_key.rs`.
- [docs/table.md](docs/table.md) — `TableView` inside: what a layout keeps, the 3 layout
  modes, `reload_cell`, `load_new_cells` and `drop_first_cells`. Read before touching
  `views/containers/table_view`.
- [docs/log-view.md](docs/log-view.md) — `LogView`, the ready view for a log: the
  `LogData` trait, ANSI colors, the prefix column, follow the end, and why a row out of
  view gets its height from arithmetic and not from shaped text. Read before touching
  `views/complex/log_view` or putting a log into an app.
- [docs/login.md](docs/login.md) — the Google and Apple login: the poll flow between `hilen::login` and
  `hilen_server::auth`, the two copies of the wire, the masked `HILEN_SESSION_KEY` and the
  `HILEN_RELEASE` mark, the short code and QR login of a device with no keyboard, and how
  the button test stays away from a real browser. Read before
  touching `hilen/src/login`, `hilen-session`, `hilen-server/src/auth` or the
  session key part of `hilen-session/build.rs`.
- [docs/hot-reload.md](docs/hot-reload.md) — hot reload in the iOS simulator and on a real iPhone, what iOS lets a build load and the `phone` word of `make swap`: `make hot`,
  the loader app and the app as 1 dynamic library, `make swap` for several apps in 1
  loader, the own heap of a build, what a stop leaves, the stop of a generation, why a hot
  build has no rayon, the winit attach, the Objective-C classes, and the `make hot-test`
  lane. Read before touching `hilen/src/hot.rs`, `hot_loader.m`, the `hot-test` crate, or
  anything a hot build has to give back at its stop.
- [docs/forks.md](docs/forks.md) — the 5 forked crates, what each fork branch carries
  against upstream, which commits are upstream candidates, and the recipe for sending a
  fork fix upstream as a PR. Read before touching `~/dev/forks`, bumping a fork, or
  opening an upstream PR.

Docs should be concise.

## Logs

Every launch logs to stdout and to a file, `~/Library/Logs/<app>/` on mac,
`%LOCALAPPDATA%\<app>\logs\` on Windows, `~/.local/state/<app>/logs/` on Linux, named
`<app>-<date>_<time>.log` after the exe, newest 10 kept. An app sets another number with
`App::log_files_kept`. The first lines name the file.
`hilen::log_file_path()` returns it, `hilen::log_dir(app)` the folder. A GUI build on
Windows has no console and a dock launch on mac has no terminal, so the file is the only
log of a shipped app. Android has no file, its lines go to logcat through the same
dispatch.

A panic is logged too, `panic_log.rs` hooks it on desktop and Android and writes the
message, the file and line and a backtrace as an error line, then runs the earlier hook.
A crash at a user's machine is read from that log file. iOS and wasm have their own hooks.

A backend on `hilen-server` logs the same way. `tracing_init::init(name)` writes the stdout
lines without color codes to `<name>-<date>_<time>.log` in the same folders, with `name`
in place of the exe name, newest 10 kept, a panic included. The first line names the file.
`hilen_server::log_file_path()` returns it. `HILEN_LOG_DIR` sets another folder and
`HILEN_LOG_FILE=off` writes no file, for a backend in Docker that keeps its lines in
`docker logs`. The code is `hilen-server/src/log_file.rs`, a copy of the folder and trim
rules of `hilen/src/log_file.rs`, since the server crate never links `hilen`.

## Data folder

All data of an app lives in `~/.config/<project_name>` on desktop. The name is the
`project_name` of the `hilen.toml` above the app crate, `hilen::register_app!(MyApp)`
reads it at build time and an app without that line stops at start. The engine creates
the folder and sets it as the `OnDisk` root before `before_launch`. `Paths::storage()`
returns it. `HILEN_DATA_DIR=<folder>` in the env moves it for 1 run of a desktop app, for a
check on a copy of the data, a test run ignores it. On iOS and Android the folder is `.<exe name>` inside the documents dir
and inside the private files dir, made and set as the root the same way. A backend
gets the same folder from `hilen_server::data_dir!()`, so a Docker build of a backend
has to copy `hilen.toml` in. The test runners start the engine's own app, which has no
name, their storage is a folder in the temp dir.

Downloaded images can be kept on disk, `Image::set_download_cache_dir`.
`set_download_cache_limit(bytes)` deletes the files used longest ago over the bound, and
`set_download_cache_recheck_age(seconds)` asks the server about an older file again with
its `ETag`. The code is `hilen/src/window/image/disk_cache.rs`.

## Window placement

Every desktop app opens where it was closed, with no code in the app: the size, the
place, the maximized state and the display. `App::initial_size` is only the size of the
very first start. The engine keeps a `WindowPlacement` in `window_placement.json` in the
data folder. The code is `hilen/src/window/placement_store.rs`.

The 2 hooks of `App` decide it. The default `window_placement` returns
`WindowPlacement::remembered()` and the default `window_placement_changed` calls
`placement.remember()`. An app that keeps the placement somewhere else writes both
hooks, the engine then reads and writes nothing. An app that wants the same window on
every start returns `None` from the first and leaves the second empty. An app that
decides at run time calls the 2 `WindowPlacement` functions from its own hooks, `demo`
does that to keep a benchmark run at one size.

A resize or a move fires many times a second. `remember` only keeps the newest
placement in memory, on the main thread. A timer thread writes it 500 ms after the
first change of a burst, so a burst is 1 write, and a placement the file already has is
not written again. The write goes through a second file and a rename. The last
placement lands when the app ends: `AppHandler::exiting` writes what still waits, and
winit calls it on every way out of the event loop, Cmd and Q on macOS included. A
process that is killed loses at most the last 500 ms.

A headless run, a run of the test runners, which store in the temp dir, and an app
started with `HILEN_RUN_TESTS` or `HILEN_PRESENT` never open that memory, they read
nothing and write nothing. A test run started inside a live app through the inspector
pauses it until the app has its root view back.

## Assets folder

On desktop `assets_root.rs` finds the `assets` folder with file checks only, first walking
up from the current folder, then up from the exe. It never starts a program such as git,
an installed app runs on a machine with no dev tools. Finding nothing is normal, a shipped
app embeds its assets and ships one exe.

## Commands

```bash
cargo run -p ui-test -- --list                                               # every registered test and the total
cargo run -p ui-test -- --headless                                           # full UI test suite
cargo run -p ui-test -- --headless --test-name <name>                        # single test, the name it prints
cargo run -p ui-test -- --test-name <name> --screenshot <path>               # capture one test offscreen
cargo run -p ui-test -- --test-name <name> --shots <dir>                     # clean PNG of every checked state, headless
cargo run -p ui-test -- --test-name <name> --human                           # watchable run, ctrl to advance
cargo run -p ui-test -- --headless --test-name <name> --record-colors        # print check_colors blocks
cargo run -p render-test                                                     # render tests, the pipelines drawn directly
cargo run -p level-test -- --list                                            # every registered level test
cargo run -p level-test -- --headless                                        # level test suite, same flags as ui-test
cargo run -p scene-test -- --list                                            # every registered scene test
cargo run -p scene-test -- --headless                                        # scene test suite, same flags as ui-test
make ui                                                                      # desktop suite, plus the iOS simulator suite on macOS, one report
make uui                                                                     # desktop suite only, headless, release mode
make smoke                                                                   # curated subset, desktop only, debug, headless, the pre-commit check
make ui-ios                                                                  # iOS simulator suite only
make ui-ios-human                                                            # the same lane held for a human, a tap on the phone screen advances, HILEN_TEST_ONLY narrows it
make ui-web                                                                  # browser suite in a real installed browser, BROWSER=firefox switches
make hot                                                                     # hot reload of the app in the iOS simulator, a saved file reloads it
make hot-test                                                                # the hot reload lane, 2 builds of hot-test swapped in one process
make swap args="start demo ../apps/skaityk"                                  # 1 loader in the iOS simulator for several apps, then args="to skaityk", "status", "stop"
make android                                                                 # APKs with every ABI, docker only
make android-emu                                                             # arm64 debug APK for the emulator
make ci                                                                      # typos, formatting, lints, unused dependencies
make lint                                                                    # clippy, pedantic, zero warnings
cargo machete                                                                # unused dependencies, zero findings
make bench                                                                   # UI benchmark suite, saves bench/<date>-<commit>.json
UI_BENCHMARK=1 cargo run -p demo --release --features bench             # single benchmark run, prints and exits
```

`HILEN_HEADLESS=1` runs any app without a window.

The suite runs every test, prints every failure at the end, then exits 1 if any failed.
`--headless` runs without a window or a display — tests run many times faster. Always pass
it unless `--screenshot` already selects the offscreen runner.

After touching any `Cargo.toml` or removing code, run `cargo machete`. It must report
zero unused dependencies.

`build/` is a git submodule, github.com/hilen/build, and it holds the build scripts.
Before every commit in this repo pull its latest `main` first, `git -C build checkout
main && git -C build pull --rebase`. If anything in `build/` is new or changed, commit
it there and push it to `main` before the parent commit, so the parent never points at
a commit that exists only on this machine. Never leave the submodule dirty or on a
detached HEAD for the user to sort out.

Run `make ci` and `make smoke` before a commit, and only then, never while no commit is asked. The full lanes,
`make ui`, `make ui-ios` and `make ui-web`, are not part of the routine pre-commit check.
Run them only when asked, or when a change reworks rendering or another
engine-wide path where a smoke miss is likely.
