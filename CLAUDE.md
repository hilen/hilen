# Hilen

Cross platform game engine and UI framework in Rust. Rendering on WGPU.
Supports: Windows, Linux, Mac, iOS, Android and WebAssembly.

The engine is one library crate, `hilen`, with modules like `gm`, `ui`, `window`,
`render`, `level` under `hilen/src/`. The foundational crates `hreads`, `refs`, `vents`
and `netrun` are modules under `hilen/src/deps/`, not separate crates, so a published
`hilen` is one self contained library. `deps/` holds only the proc macro crates,
`ui-proc-test`, the compile check for the `view` macro, `hilen-pixels`, the per pixel
loops kept optimized in dev builds by a profile override, plus `plat`, which stays its
own crate because three build scripts call its `platforms()` to set the cfg aliases and
a crate cannot use its own code in its build script.
Apps and test binaries are separate crates on top. Internals are `pub(crate)`, the
app-facing API is `pub` — keep new items `pub(crate)` unless apps need them, so the
`dead_code` lint stays meaningful.

`hilen-server` is the backend base crate for app backends, config, error type,
base routes and helpers over axum, sqlx and redis. It also carries the standard
way to serve an app's trunk-built wasm dist, `web_mount` in `src/web.rs`, the
dist embeds via rust-embed with SPA fallback and `HILEN_WEB_DEV_PROXY` points
page requests at a running `trunk serve` for the dev loop. It never links the
`hilen` UI crate, a backend and a client only share the wire.

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
with their shaders. `audio` is sound playback through kira and its decoders. `video` is video
playback through a prebuilt static ffmpeg and kira, desktop only and proven on macOS, see
[docs/video.md](docs/video.md). `inspect` is
the remote inspector. `scene` is the 3D twin of `level`, physics on rapier3d and glam, its own
`#[scene]` macro, `scene-test` crate and `SCENE_TESTS` registry, see [docs/scene.md](docs/scene.md).
`ui-tests` and `level-tests` register tests. A GUI only app depends
on `hilen` with none of them and the wasm drops rapier, kira and the codecs entirely.
`demo` turns `audio`, `inspect`, `level`, `scene` and `ui-tests` on, and `video` on macOS.

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
  Read before writing or debugging UI tests.
- [docs/inspect.md](docs/inspect.md) — the remote UI inspector, its protocol and the
  off-by-default `inspect` feature gate. Read before touching `hilen/src/inspect`,
  the `inspector` app, or the `hilen-inspect` CLI.
- [docs/benchmark.md](docs/benchmark.md) — the UI benchmark, its consistency guard, and the
  results history in `bench/`. Read before touching the benchmark or measuring performance.
- [docs/guesses.md](docs/guesses.md) — parked changes that lacked proof. Read before
  proposing an optimization or a speculative fix; add new unproved ideas there, not to code.
- [docs/text.md](docs/text.md) — the text pipeline: rustybuzz shaping, em sizing, variable
  font instances, letter spacing, line handling. Read before touching label rendering,
  fonts, or `hilen/src/window/text`.
- [docs/roadmap.md](docs/roadmap.md) — missing engine features found by porting a real app,
  with current state, design notes, and order. Read before planning or starting a new
  engine capability, and update it when one lands.
- [docs/pixdiff.md](docs/pixdiff.md) — the `hilen-pixdiff` pixel parity tool: capture app
  windows from the screen, resize both apps to one size, diff the captures into ranked
  regions. Read before comparing a port against its original or touching `hilen-pixdiff`.
- [docs/updater.md](docs/updater.md) — `system::Updater`, the manifest schema, the ed25519
  signing contract and what the in place swap means for packaging. Read before wiring
  self update into an app or touching `hilen/src/system/updater.rs`.
- [docs/windows.md](docs/windows.md) — why Windows renders through DX12, the silent Intel
  Vulkan crash it avoids, and how to read a `0xc0000005` from the event log. Read before
  changing backend selection or when an app dies on Windows with no message.
- [docs/wsl.md](docs/wsl.md) — running on Windows through WSL: the packages `make setup`
  installs, why the engine forces X11 and takes the scale from WSLg, and what to check
  when no window appears. Read before touching `window/wsl.rs` or when an app shows
  nothing under WSL.
- [docs/android.md](docs/android.md) — the docker build and the emulator lane, the
  Vulkan-only backend, APK asset loading, the register requirement in the shell crate,
  and the generated-project fixes the template still misses. Read before touching
  android builds or when an APK dies at startup.
- [docs/ios.md](docs/ios.md) — what keeps iOS 12 and the A7 working: `NSLog` output, the
  ObjC exception preprocessor, the two version settings that look alike, the weak linked
  CoreGraphics and the wgpu fork. Read before touching anything iOS, the `wgpu` pin, the
  iOS deployment target, or when an app dies on a device with no message.
- [docs/tvos.md](docs/tvos.md) — tvOS builds and renders in the Apple TV simulator,
  display only, no input path yet. The vendored plat, the winit fork pin, the hand made
  simulator shell and how to run it. Read before touching platform cfg aliases, the
  winit pin, or anything tvOS.
- [docs/scene.md](docs/scene.md) — the 3D `scene` module: the level shaped architecture, glb
  models with skins and clips, the sun's shadow map, touch picking, the depth band it draws in, the A7 varying
  budget of the mesh shader, scene tests and what is still to come. Read before touching
  `hilen/src/scene`, `scene_drawer.rs`, the mesh pipeline or a scene test.
- [docs/video.md](docs/video.md) — the `video` feature: `VideoView`, the ffmpeg decode thread and
  hardware devices, the NV12 pass, kira as the clock, the prebuilt static ffmpeg archives and
  how to build one, and what was measured. Read before touching `hilen/src/video`, the
  archive script or `hilen/ffmpeg`.
- [docs/forks.md](docs/forks.md) — the 3 forked crates, what each fork branch carries
  against upstream, which commits are upstream candidates, and the recipe for sending a
  fork fix upstream as a PR. Read before touching `~/dev/forks`, bumping a fork, or
  opening an upstream PR.

Docs should be concise.

## Logs

Every launch logs to stdout and to a file, `~/Library/Logs/<app>/` on mac,
`%LOCALAPPDATA%\<app>\logs\` on Windows, `~/.local/state/<app>/logs/` on Linux, named
`<app>-<date>_<time>.log` after the exe, newest 10 kept. The first lines name the file.
`hilen::log_file_path()` returns it, `hilen::log_dir(app)` the folder. A GUI build on
Windows has no console and a dock launch on mac has no terminal, so the file is the only
log of a shipped app. Android has no file, its lines go to logcat through the same
dispatch.

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

Always run `make ci` and `make smoke` before every commit. The full lanes,
`make ui`, `make ui-ios` and `make ui-web`, are not part of the routine pre-commit check.
Run them only when asked, or when a change reworks rendering or another
engine-wide path where a smoke miss is likely.
