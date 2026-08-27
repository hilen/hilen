# Hilen

Cross platform game engine and UI framework in Rust. Rendering on WGPU.
Supports: Windows, Linux, Mac, iOS, Android and WebAssembly.

The engine is one library crate, `hilen`, with modules like `gm`, `ui`, `window`,
`render`, `level` under `hilen/src/`. The foundational crates `hreads`, `refs`, `vents`
and `netrun` are modules under `hilen/src/deps/`, not separate crates, so a published
`hilen` is one self contained library. `deps/` holds only the proc macro crates plus
`plat`, which stays its own crate because three build scripts call its `platforms()`
to set the cfg aliases and a crate cannot use its own code in its build script.
Apps and test binaries are separate crates on top. Internals are `pub(crate)`, the
app-facing API is `pub` — keep new items `pub(crate)` unless apps need them, so the
`dead_code` lint stays meaningful.

`hilen-server` is the backend base crate for app backends, config, error type,
base routes and helpers over axum, sqlx and redis. It never links the `hilen`
UI crate, a backend and a client only share the wire.

The UI test corpus is its own crate, `ui-test-suite`, so `demo` can link it and carry
every test onto a device. It must never depend on `demo`, that is a cycle, since the
`ui-test` runner links both.

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
- [docs/windows.md](docs/windows.md) — why Windows renders through DX12, the silent Intel
  Vulkan crash it avoids, and how to read a `0xc0000005` from the event log. Read before
  changing backend selection or when an app dies on Windows with no message.
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

Docs should be concise.

## Commands

```bash
cargo run -p ui-test -- --list                                               # every registered test and the total
cargo run -p ui-test -- --headless                                           # full UI test suite
cargo run -p ui-test -- --headless --test-name <name>                        # single test, the name it prints
cargo run -p ui-test -- --test-name <name> --screenshot <path>               # capture one test offscreen
cargo run -p ui-test -- --test-name <name> --human                           # watchable run, ctrl to advance
cargo run -p ui-test -- --headless --test-name <name> --record-colors        # print check_colors blocks
cargo run -p render-test                                                     # render tests
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

Always run `make ci` and `make smoke` before every commit. The full lanes,
`make ui`, `make ui-ios` and `make ui-web`, are not part of the routine pre-commit check.
Run them only when asked, or when a change reworks rendering or another
engine-wide path where a smoke miss is likely.
