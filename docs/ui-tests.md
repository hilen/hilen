# UI tests

Real-window tests. They open the app, inject touches and scrolls, then check labels, colors and
state. The corpus lives in `ui-test-suite/`, plus some in `hilen` and `demo`.
`ui-test/` is only the runner.

## Run

```bash
cargo run -p ui-test -- --list                # every registered test and the total, runs nothing
cargo run -p ui-test                          # full suite, all tests, 2 cycles
UI_TEST_CYCLES=5 cargo run -p ui-test         # more cycles
cargo run -p ui-test -- --test-name "Rest request"  # one test
cargo run -p ui-test -- --headless            # offscreen, much faster, for CI and agents
cargo run -p ui-test -- --test-name "Font zoo" --screenshot /tmp/font-zoo.png  # one offscreen capture
cargo run -p ui-test -- --test-name "Font zoo" --shots /tmp/shots        # every checked state as a clean PNG, headless
make uui                                      # full suite, headless, release mode
make smoke                                    # curated subset, desktop, debug, headless, the pre-commit check
cargo run -p ui-test -- --test-name "Font zoo" --human            # watch one test, ctrl to advance
cargo run -p ui-test -- --test-name "Font zoo" --present          # presentation mode, the view over the whole window, nothing injected, play with it and close
cargo run -p ui-test -- --record-colors --headless --test-name "Font zoo"  # print check_colors blocks
```

**A test answers to the name of its view.** `FontZoo` registers as `Font zoo`, through
`ui_test::spaced_test_name`, the one rule `get_test_name` and `--test-name` both call. So
`--test-name` takes either spelling, `"Font zoo"` or `FontZoo`, and a tool reading
`impl ViewTest for FontZoo` off the source can pass what it sees without deriving anything.
Deriving that name twice is what once made the generated `#[test]` hand the runner a name it
rejected. A name that matches nothing exits 1 and points at `--list`, so a typo never looks
like a pass.

Counting is not done by reading the log. Every test is registered by a `ctor` before `main`,
so `--list` knows the whole suite without running anything. An empty registry is a hard
error, never `0 tests passed`, because a suite that runs nothing otherwise reports success.

An app runs the same suite from inside itself, which is how tests run on a device.
`ui_test::run_all_tests` reaches every registered test with no help from the app, and
`hilen-inspect run-tests` triggers it over the network. `demo` also has a "Run UI tests"
button in its dev menu. See [inspect.md](inspect.md).

Set `HILEN_RUN_TESTS` and the app runs the whole suite once it is ready, prints
`HILEN_TEST_RESULT <n> tests, <m> failed` and exits with a matching code. It waits on
`UIManager::on_app_ready`, since a mid load teardown frees views the load task still
touches, so an app with a loading screen marks itself not ready until assets land. No
inspector and no mDNS, so it runs while the desktop lane runs. `make ui` uses it: on macOS
it runs the desktop suite and the iOS simulator suite in parallel, then prints one report.
Every lane's full output is kept in `target/ui-test/<lane>.log` and each failure report
in `target/ui-test/failures/<lane>/<test>.txt`, listed at the end of the report, so a
failed run is read back from disk, never run again to find out what broke. The dir is
wiped at the start of every `make ui`.
The simulator lane is `build/ios/sim-test.rs`, an iPhone 8 on iOS 16.4, the oldest device
this toolchain can boot. `make ui-ios` runs only that lane. It needs the base iOS platform
and the iOS 16.4 simulator runtime installed through `xcodebuild -downloadPlatform iOS`, else
the storyboard build fails and the whole lane reports `0 passed 0 failed`. See [ios.md](ios.md).

`make ui-ios` streams the suite live, the same `Started`/`OK` lines the desktop lane prints, so
a hang names the test it stuck on. The app logs through NSLog, which tags every console line
with a timestamp and process name, and the lane strips that prefix so the stream reads like
desktop. Under `make ui` the lane sets `HILEN_IOS_QUIET` and goes back to buffering, printing only
`[ios]` milestones, so three parallel lanes do not mangle each other's output.

`HILEN_TEST_ONLY` narrows a `HILEN_RUN_TESTS` run to a comma separated list of test names. It is for
isolating one case on a device or simulator, where the whole suite is slow to reach it. The
simulator lane forwards it, `simctl` only passes `SIMCTL_CHILD_` prefixed variables into the
app and the lane adds that prefix itself. `make ui-ios-human` is the watchable simulator run,
see Human mode below.

The browser lane is `make ui-web`. Its driver, `build/web/drive.ts` run by Bun, builds the
atomics wasm, serves it with the isolation headers and opens a real installed browser, Chrome
by default, `BROWSER=firefox` switches. A page has no env vars, so the autorun fires from the
`hilen_run_tests` query flag and `hilen_test_only` narrows it the way `HILEN_TEST_ONLY` does natively.
The report arrives over the inspect WebSocket instead of the console, and on failure the
driver saves an app screenshot to `target/web-test/ui-web-failure.png` over the same socket.
See [inspect.md](inspect.md).

A wasm panic aborts the whole instance, there is no unwinding to catch it like the native
runner does. The panic beacon names the running test, the driver records it as failed,
relaunches the browser with the dead tests in `hilen_test_skip`, and merges them into the
final report, so one panicking test cannot hide the rest of the suite.

## Level tests

A level is not a view, so a level check is not a UI test. It is a `#[level]` struct with
`impl LevelTest`, registered by the same kind of ctor into `hilen::LEVEL_TESTS`, and run
by the `level-test` crate, which also holds the corpus. The runner installs an empty root
on the test canvas, starts the level at scale 1 so a retina window does not move the
probes, hands the level to `perform_test` and stops it afterwards. The flags match
`ui-test`: `--list`, `--test-name`, `--headless`, `--record-colors`, `--human`,
`--screenshot` and `--present`. `make level` runs the suite. `render-test` stays what it
is, the pipelines drawn directly with no level and no view tree.

```bash
cargo run -p level-test -- --list
cargo run -p level-test -- --headless --test-name SpriteCutout
cargo run -p level-test -- --test-name SpriteCutout --human
```

## Run from the editor

A patched rust-analyzer puts a run button on every `impl ViewTest for X` line. Stock
rust-analyzer sees nothing runnable in that impl. The button offers three modes of
`cargo run -p ui-test`:

- `run ui-test X` passes `--test-name X --human`, watchable, ctrl to advance.
- `run ui-test X headed` passes `--test-name X`, windowed, runs by itself.
- `run ui-test X headless` passes `--headless --test-name X`, no window.

The patch lives in the fork at
[VladasZ/rust-analyzer](https://github.com/VladasZ/rust-analyzer), branch
`view-test-runnable`. It adds a `ViewTest` runnable kind. `ide` detects the impl,
`target_spec` maps it to the fixed `ui-test` invocation, and it is exposed through both the
code lens and `experimental/runnables`, so VS Code lenses and Zed gutter tasks both get it.

To use it, build the fork in release and make it the binary the editor runs. One gotcha for
Zed. When `rust-toolchain.toml` lists the `rust-analyzer` component, Zed asks rustup first
and checks PATH only after that, so a patched binary earlier on PATH is never found. Replace
the toolchain's own binary with a symlink to the patched build:

```bash
ln -sf <patched rust-analyzer> ~/.rustup/toolchains/<channel>/bin/rust-analyzer
```

A rustup reinstall of the toolchain restores the stock binary, then the symlink needs to be
redone. Zed also needs `"enable_lsp_tasks": true` on its rust-analyzer entry, since it drops
LSP runnables by default and the button rides on `experimental/runnables`. Zed fetches
runnables the moment a buffer opens and caches the answer until it changes, which on startup
is before the workspace is loaded. The patch holds those early requests until the server is
quiescent, so the button appears on its own once loading finishes.

## One registry

Every test, whatever crate it lives in, registers into a single map, `hilen::UI_TESTS`,
holding the name, the fn to run and the source file. The count is its length.

That map is a static of the engine, so a test in `hilen`, one in `ui-test-suite` and one
in `demo` all land in the same place. Nothing merges maps, nothing registers a runner, and
the engine can run the whole suite on its own.

Registration is by name, and a duplicate name aborts at startup rather than silently replacing
the other test. A test that quietly stops running looks exactly like a test that passes, which
is the failure this registry exists to prevent. The key is the type's own name with no path, so
two test views called the same thing collide even from different crates. That is loud, not
silent, so it needs no other rule.

**A test registers through a `ctor`, and nothing calls it by name.** A linker drops any object
nothing references, so a crate whose only content is tests is dropped whole and its tests
disappear without a word. Every consumer of a test-carrying crate has to name it:
`ui_test_suite::keep_linked()` in `demo` and in the runner. This is not theoretical, it is
how the device ran 24 tests while the desktop ran 100.

A failing test does not stop the run. Every test executes, each failure is collected, and
the whole report prints at the end, then the process exits 1 if anything failed. One run
therefore shows every broken test rather than only the first.

Always pass `--headless` when running from a script or agent, and always tee the output to
a temp file — with a plain pipe (`| tail`) you lose everything printed before a hang:

```bash
cargo run -p ui-test -- --headless 2>&1 | tee /tmp/ui-test.log | tail -12
```

Don't run the suite after every change. During development run only the single tests the
change affects. The routine pre-commit check is `make smoke`, a curated one-test-per-pillar
subset, desktop only, debug, headless. The list lives in the Makefile as `SMOKE_TESTS` and
rides on `--test-name` taking a comma separated list. The full suite and the platform
lanes run only when asked, or when a change reworks rendering or another engine-wide
path where a smoke miss is likely. Mechanical changes (renames, comments, docs) only
need `cargo build` and `make lint`.

On failure a report is printed: window resolution and scale, a path to a screenshot of
the actual screen, and the view tree with frames. For `check_colors` failures the failing
pixel also gets a highlight marker, visible in the screenshot. Read the screenshot and
the view tree first — they usually show the problem immediately.

Never edit test expectations (`check_colors` data, asserted values) to make a failing
test pass. The expectations are the spec: the UI must behave exactly like before. If a
test fails after a code change, the code is wrong. Expectations change only when the new
look or behavior is intended and explicitly approved.

Never change existing UI tests while implementing a new feature unless the user
explicitly allows it. Design the feature so old tests stay green: make new behavior
opt-in instead of changing defaults. If a new mechanism genuinely invalidates an old
assertion, stop and ask before touching it.

Temporary edits that are never committed are allowed — for example breaking one
expectation on purpose to verify the failure machinery. Say what you are doing first,
revert right after the run, and check that `git diff` is clean before committing.

`cargo test` does not run UI tests. `ui-test` is the only runner. The macro used to generate a
`#[test]` per test that shelled out to `ui-test` in a second target dir, which bought a second
entry point onto the same runner, cost a duplicate build, and was broken and skipped in CI for
long enough that nobody noticed.

Every test prints `Name: Started` and `Name: OK`. On a hang or failure the broken test is the
one with `Started` and no `OK` — usually the last line of the log.

Every test starts in `ThemeMode::System` with the system theme pinned to light. A headed
window follows the OS theme, so on a dark desktop every light color block failed while
headless passed. `run_test` resets it before each test, which also stops a test that
switched the theme from leaking into the next.

The test app disables vsync and raises max frame latency at startup (`Window::set_vsync(false)`,
`Window::set_max_frame_latency(3)`) so tests are not capped to the display refresh rate.

`--headless` goes further: the app starts with no window at all — no winit, no surface,
no display. Frames render to an offscreen texture in a plain loop, so the full suite runs
in a few seconds and works on machines without a display server (CI), given a GPU or a
software Vulkan driver. Screenshots and `check_colors` still work. Run headed when you
want to watch the UI. The network test (`Rest request`) checks `Window::headless()` at the top
of its `perform_test` and returns before the tap that sends the request, since a registry has
no place to hang that condition.

`--screenshot <path>` requires one `--test-name` and selects the headless runner by itself.
The runner saves the final tested frame when the test does not choose a capture point. A
test that needs an earlier exact state calls `capture_screenshot()` there; it still saves
only when the command requested an output path. Screenshot mode is for fast agent inspection,
not a substitute for the required `--human` user review.

`--shots <dir>` selects the headless runner too and takes any test subset or the whole
suite. It saves a clean frame, no probe markers, at every `check_colors` and every
`checkpoint`, as `<dir>/<test>-<NN>-<label>.png` with `NN` counting up per test, so the
files sort in run order and an agent sees every verified state from one run. A check
saves before it asserts, so a failing check still leaves its frame. `checkpoint` is the
way to name a state a test wants on disk that no check pins.

For profiling, pass `--fps-report` to print a report at the end of the run: frames, duration
and average fps per test. Per-test fps varies a lot between runs — macOS sometimes paces frames
at display rate anyway — so don't compare single runs.

## The canvas

Probes index screen pixels, so a test needs a fixed rectangle to draw in. Tests never
resize the window. Instead the harness pins the root view to a canvas at the frame origin,
600 by 600 by default, and the rest of the screen shows the clear color. A phone screen
cannot be resized, so this is what lets the same test and the same probes run on desktop
and on a device.

The canvas is counted in screen pixels, not points, and the harness divides the scale back
out. A scale change resizes the root, so the canvas keeps the same pixels either way.

Because the root itself is the canvas, anything laid out against the root lands inside it,
including modals, alerts and drop downs. Touch dispatch starts at the root, so injected
touches outside the canvas go nowhere.

Declare a different canvas when the default is too small. Two ceilings apply and the
lower one wins, so a canvas has to clear both.

Width and height must fit the smallest supported screen, 640 by 1136 on an iPhone 5S,
or the test cannot run on device. Height must also fit the desktop render surface,
which is `App::initial_size`, 1200 by 1000 by default and not overridden by `ui-test`.
So the real ceiling is 640 by 1000, and 1136 is unreachable on desktop.

Going over is silent, not loud. Nothing below 1000 renders, and the probe recorder
clips to the screenshot with `height.min(shot.size.height)`, so the rows past the
surface record no probes and read as tested. A canvas of 640 by 1136 leaves its bottom
three rows of labels dead with a green run.

```rust
impl ViewTest for LongTableTest {
    fn canvas() -> (u32, u32) { (640, 1000) }
    ...
}
```

`AppRunner::set_window_size` stays an app API. No test may call it, a window smaller than
the canvas clips it and every later test probes the clipped frame.

A game or a level fills the root rather than the window, see `UIManager::render_area`.
Anything else that renders from the whole frame, such as a blur, samples the clear color
around the canvas, so probes within a blur radius of a canvas edge pick that up. That is
consistent on every screen, since the canvas is always smaller than the frame.

Global state is reset per test: the root background, the clear color, the string state,
the drag scrolling platform default and any running level, since a level is not in the view tree and one left by a test
would draw under every test after it.
A test that fails part way never reaches its own cleanup, and without the reset every
later test would probe the leftovers.

## One shape

**`impl ViewTest for X` is the whole declaration of a test.** There is no attribute. A test is a
`#[view]` like any other, plus that impl.

The impl is what registers it. `#[view]` puts a ctor on every view which asks the type whether
it implements `ViewTest`, through a specialization probe, and registers it if so. So the text
you read and the thing that runs are the same text, and there is nothing to keep in step. Two
attributes used to answer that question instead, `#[view_test]` and `#[ui_test]`, and both could
be forgotten: `LoadingView` carried an `impl ViewTest` that no attribute ever registered, so it
never ran, and `RestRequest` carried both and registered twice, which defeated the headless
guard on one of the copies and sent every CI run at a live endpoint.

Only `perform_test` is required. The rest have defaults and exist because a real test needed
them, not in advance:

```rust
impl ViewTest for MyTest {
    fn perform_test(view: Weak<Self>) -> Result<()> { ... }   // required

    fn before_start() {}                                       // runs before the view is built
    fn canvas() -> (u32, u32) { (600, 600) }                   // screen pixels to draw in
    fn make_root(view: Own<Self>) -> Own<dyn View> { view }    // the root to install
}
```

- `before_start` is for anything a view reads *while being built*. A global `Style` is read in
  `setup`, so applying it from `perform_test` is too late and the test renders unstyled against
  styled expectations. `Global styles` and `Number view design` need this.
- `make_root` is for a view that only works inside a host. `Present test view` has to sit in a
  `NavigationView` before it can present, so it returns the stack. `perform_test` still gets the
  test view, not the host.

Nothing here is async. A UI test drives the main thread through `from_main` and never awaits. The
corpus was async for years and not one test awaited anything, which cost a second registry, a
boxed future type and a hand written call list.

### A generic view cannot be a test

A ctor names one concrete type, and a generic view has none until something instantiates it
somewhere the macro cannot see. So `#[view]` emits no ctor for a generic view, and nothing would
ever register the test. That is a compile error rather than a silent no-run: `#[view]` also emits
`impl Registrable for X` for non-generic views only, and `ViewTest: View + Registrable`. Wrap the
generic view in a plain one and put the impl on the wrapper.

### The feature

Registration lives behind `hilen/ui-tests`, off by default, so a shipped app carries no
ctors at all. The switch is on the proc macro crate, `ui-proc/ui-tests`, not on each consumer,
so there is exactly one of it and no crate can forget its own and silently lose its tests.
`ui-test`, `ui-test-suite` and `demo` turn it on.

Level tests have their own switch, `hilen/level-tests`, which turns on `level-proc/level-tests`
so `#[level]` emits its ctor. It depends on `ui-tests` for the shared runner, but `ui-tests`
never pulls level code in. Only `level-test` turns it on.

The engine's own test modules are gated behind the same feature with `#[cfg]`, so a
default-features build compiles none of them. Without the gate an app pulling the engine as a
path dependency sees their dead code warnings, since an unregistered test view is unused.

## Platform gating

Every UI test must run on every supported UI-test platform where the production behavior exists.
This includes renderer regressions first found on desktop, tests that set a desktop scale, and
fixtures whose original presentation is larger than the cross-platform canvas. A canvas size,
fixture layout, screenshot workflow or convenient reproduction is never a valid reason for
`#[cfg(desktop)]`. Keep the real production view and behavior, and arrange the test so it fits the
640 by 1000 cross-platform canvas.

Platform gating is allowed only when the production feature itself is compiled out or cannot
exist on that platform. Gate such a test where the feature is gated, not with a runtime skip.
`Hover::update` is `#[cfg(any(desktop, wasm))]`, so `hover.rs` is too. Typing goes through the
screen keyboard on a phone rather than injected key events, so the text field tests are desktop
only as well.
Test counts may differ between platforms only for these feature-availability gates. Never gate a
cross-platform rendering, layout or interaction regression merely to make the current fixture fit.
Gate the module in its `mod.rs`, with a comment saying which feature is missing:

```rust
/// Hover needs a pointer, and there is no such thing on a touch screen.
#[cfg(any(desktop, wasm))]
mod hover;
```

A crate that gates on `#[cfg(desktop)]` needs `plat::platforms()` in its `build.rs`, which is
what defines those cfgs.

**A test that asserts inside `from_main` and fails takes the whole run down**, because the
panic lands on the main thread, unwinds through the dispatch and aborts. On a device that means
the run dies with no report at all. The desktop runner installs a hook for this, the in-app
runner does not yet.

## Writing a test

```rust
#[view]
struct MyTest {
    #[init]
    button: Button,
}

impl Setup for MyTest {
    fn setup(self: Weak<Self>) { /* build UI */ }
}

impl ViewTest for MyTest {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        inject_touches("100 100 b\n100 100 e"); // x y begin/end
        assert_eq!(view.button.text(), "tapped");
        Ok(())
    }
}
```

That is the whole thing. The harness builds the view and hands it to `perform_test`, so a test
never calls `UITest::start` for its own view. It registers as `My test`.

To test an existing widget, give it a fixture view to live in and put the impl on the fixture.
The corpus does this throughout, and a fixture is usually what you want anyway, since its
`setup` arranges the scene the widget is tested in.

Test helpers: `inject_touches`, `inject_scroll`, `inject_right_click`, `inject_long_press`,
`wait_for_tooltip`, `check_colors` (asserts pixel colors at coordinates). To read UI state from test code use `from_main` (see [dispatch.md](dispatch.md)).

An animation samples real time, so its mid flight frames depend on machine speed and
a test can normally only check the settled state. A test that needs an exact frame
opts into frame stepped time: `Clock::enter_stepped()` on the main thread freezes
the engine clock, and `step_frames(n)` moves it by `n` frames of 16.666 ms, rendering
each one. `Animation` and `AnimatedImage` read that clock, so after `step_frames(15)`
a 0.5 s animation sits at exactly half and a gif with 100 ms frames is on frame 1
after `step_frames(6)`, on every platform. The runner leaves stepped mode before
every test, so a test never has to clean it up on the failure path. `Frame stepped
animation` and `Animated gif` are the examples. `Animation drives frames` must never
run stepped, it proves free running animations request their own frames.

## What a run takes from the app

A run is not read only. It overrides the UI scale to 1, sets the default label text size to 32,
takes the app's global styles away, paints its own clear color, pins the root to the test canvas
and tears the app's root view down. `run_test_map` snapshots all of it and hands it back at the
end, then asks the app for a new root view, so an app that runs its own suite lands back on its
main screen at its real scale.

The 32 points are only the default for unsized labels, `set_text_size` always wins. So an
unsized label renders with the app default in production and with 32 in a test. Size labels
explicitly in views under test.

Leave any of it behind and the app carries on wrong. On a phone that means half sized UI, since
the harness scale of 1 is not the screen's 2.

One test per file. Deliberate decision to keep files small.

A new UI test is not finished when it passes. Always show it to the user for approval.
The agent launches the `--human` run of every new or changed test itself, tells the user
the window is up and asks them to check it, then waits for their verdict. Never hand the
user the command to run. After the verdict, stop and wait. Do not mention, plan, or run
`make ci`, `make smoke`, or any commit step until the user brings up committing or
pushing.

## Presentation mode

`--present` builds one test's view over the whole window and hands it over. Nothing is
injected and `perform_test` never runs, the window is the user's until they close it. This
is the mode for looking at a view or playing with it, for example a `ViewGallery` of design
variants. Human mode is not that, it runs the test and only pauses between its steps, so a
view with no steps flashes by and a panicking `perform_test` takes the window with it.

## Human mode

`--human` makes a run watchable: vsync stays on, injected touches are drawn on screen, every
injected event pauses (`UI_TEST_HUMAN_DELAY` ms, default 50, moved touches an eighth of it),
and every screenshot pauses first so the verified state is visible. Every `check_colors`
outlines its checked pixels on screen, each with a swatch of the color that probe pins just
outside the outline's top right corner, so a probe sitting on the background next to a glyph
is telling apart from one sitting on the glyph. The window title names the check, with the
frame time after it the way an app title shows it, `Font zoo check 1 | 1.23ms`, refreshed once a
second while frames render and frozen while the loop sleeps, so a stalled loop is visible in the
title. Prompts go through `Window::set_title_prefix`, which keeps the frame time, unlike
`Window::set_title`, which replaces the whole title. The run holds until ctrl before asserting, ctrl and not space so a hold with a selected text
field does not type the advance key into it. After each test the title shows the result and
the run holds again. Works for one test or the whole suite. Rejected together with `--headless`.

The browser lane has the same mode. `bun build/web/drive.ts --human --only "Name"` puts
the `hilen_human` query flag on the page, the browser spelling of `--human`, and drops the
driver's report timeout, which would otherwise kill the held run it exists to protect.
Prompts land in the tab title, `Window::set_title` writes `document.title` on wasm since
winit's web backend only sets the canvas `alt` attribute, which nobody sees. Click the
page once so key events reach the canvas, then ctrl advances the same way.

The simulator lane has it too. `make ui-ios-human`, or `rust build/ios/sim-test.rs --human`,
passes `HILEN_HUMAN` into the app, the device spelling of `--human`, and always streams.
Combine it with `HILEN_TEST_ONLY` to watch one test, `HILEN_TEST_ONLY="Font zoo" make
ui-ios-human`. A phone has no window title and no ctrl key, so there every hold draws a
translucent bar with the prompt over the bottom 40 px of the canvas and a tap anywhere on
the screen advances. The overlay is its own touch layer, the tap never reaches the views
under review, and it is gone between holds. The run still ends like the plain lane, the
simulator shuts down after the last tap.

`checkpoint(label)?` marks a state worth looking at that no injection paces, like a
browser URL change that would otherwise flash by. In human mode it holds with `label` as
the prompt, in shots mode it saves a frame named after `label`, and otherwise it costs
nothing, so headless runs keep full speed. `Router test` steps through every history
change with it.

## Recording color probes

A test that pins how something looks carries recorded `check_colors` blocks. Put empty
`check_colors("")?` placeholders in from the first draft and record them with
`--record-colors` while iterating. Region comparisons are fine on top but they do not
replace the blocks. A test that only checks behavior, counters and state, carries no
blocks.

`check_colors` expectations are recorded, not written by hand. With `--record-colors` every
`check_colors` call prints a ready to paste block instead of asserting: it takes a
screenshot, picks probe pixels automatically, and prints them labeled with the test name and
check index. Write the test with empty `check_colors("")?` placeholders, run once with the
flag, paste each block over its placeholder, rerun normally to verify.

A probe line is `x y - #rrggbb`, coordinates right aligned to four columns, the color a
lowercase CSS hex. The parser accepts nothing else, and the recorder and the failure
report print the same shape, so pasted lines always stay aligned.

The picker is deterministic, the same screen always produces the same block. It is bounded
to the canvas, the frame around it is not part of the test and does not exist on a device.
It samples a 4px grid, keeps only pixels whose neighborhood is near uniform along at least
one axis — skipping antialiased corners, the pixels a sub pixel layout shift moves most —
clusters candidates by color so text ink is probed alongside backgrounds, gives small
enclosed features like letter holes their own probes first, and spreads the rest spatially.

One axis, not all of them. Uniform in every direction needs a stem 3 pixels wide, which
only a blocky font has at a normal text size, so every hairline or striped font used to
yield no candidates at all. Its labels recorded nothing but the background around the
glyphs and read as tested: `Label stress` pinned 26 of its 40 labels, and blanking the
other 14 kept the suite green. Uniform along a stem holds however thin the stem is.

Changing the picker does not invalidate existing blocks. `stable_color` is only reached
under `--record-colors`, a normal run just compares the pinned pixels, so old blocks keep
passing and a test only gains the better probes when someone re-records it.

Default is 32 probes per check. A test declares its own density with
`set_record_probe_count(n)` as the first statement of `perform_test`, since starting the test
resets it to the default. It is inert outside record runs. Keeping it in the test source means
it survives the next re-record.

`--record-colors --human` combined shows the freshly picked probes the same way normal
human runs show existing ones, to review what gets pinned before pasting.

Re-recording an existing block rewrites the spec. Approval of a code change is not
approval to re-record, and approval to record is not approval of the recorded result.
The gates, every one mandatory:

1. Inspect the failure screenshot and confirm the render is intentionally different.
2. Name the test, explain why its pixels moved, ask permission to record, and wait.
3. Record only that test with `--headless --test-name <name> --record-colors`.
4. Paste the block over the old one and compare the two. Keep every probe inside the
   declared canvas.
5. Show the recorded render and probe markers with `--test-name <name> --human
   --record-colors`.
6. Stop and wait for explicit acceptance. Run no other test, suite, check or commit while
   that review is pending. A passing rerun proves nothing, the expectation came from that
   same render.

A recorded block is large. Keep it in a `const` next to the test rather than inline, so
the function stays readable and within the line limit.
