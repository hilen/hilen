# tvOS

The engine runs in the Apple TV simulator: it draws, the remote drives the UI, video
plays, and settings stay over a restart. It has never run on a real Apple TV.

Proven on 2026-10-10, toolchain nightly-2026-10-01, tvOS SDK 26.2, simulator runtime
tvOS 26.2 on an Apple TV 4K at 1080p.

## The cfg aliases and the forks

- `plat` is vendored at `deps/plat` and is a workspace member. It stayed its own crate
  when the other foundational crates were folded into `hilen`, because `hilen`, `demo`
  and `ui-test-suite` all call its `platforms()` from their build scripts, and a crate
  cannot use its own code in its build script. `ios` means ios or tvos, a separate
  `tvos` alias marks the real differences, `mobile` and `apple` include tvos, `ffmpeg`
  includes tvos, and `Platform` has a `TVOS` const.
- `winit` is forked, see [forks.md](forks.md). The fork adds tvos to the cfg aliases,
  guards five `UIViewController` calls the SDK marks unavailable on tvOS, and forwards
  the presses of the remote. The home indicator call aborts the app at window creation
  otherwise. winit's own capability guard cannot catch it, it checks OS versions, not
  platforms.
- `mach2` needed no fork, kira pulls a version that allows tvOS.

## Building and running

```bash
make tvos        # both libs, the Xcode project, a simulator build
make tvos-lib    # only the libs
make fly         # uploads the tvOS build after the iOS one when hilen.toml has `tvos = true`
```

tvOS is a tier 3 target with no prebuilt std, so `build/tvos/build-lib.rs` builds with
`-Z build-std=std,panic_abort`. There is no universal lib, the device and the simulator
are both arm64, `aarch64-apple-tvos` and `aarch64-apple-tvos-sim`, and the project
picks the lib by its sdk. `tvos_minimum_version` in `hilen.toml` sets the oldest tvOS,
15.0 when it is not there. On a machine with no cargo the libs are built through far
and come back, like the iOS ones.

The project is `mobile/tvOS/` from the `hilen-mobile` template. It has the app target
and `SystemInput`, an XCUITest that presses the buttons of the remote for a test, one
`remote up|down|left|right|select|menu|playpause` line per press over a local socket.
It links the lib with `-l<name>` and names every framework in its linker flags, a
static lib cannot ask for one by itself. Its asset catalog has the layered app icon and
the top shelf images tvOS wants. `make tvos` makes the whole `mobile` folder again, so
nothing in it is kept.

To run the app in the simulator. The runtime comes from `xcodebuild -downloadPlatform
tvOS`, a 3.6 GB download:

```bash
xcrun simctl create te-AppleTV-26.2 \
  com.apple.CoreSimulator.SimDeviceType.Apple-TV-4K-3rd-generation-1080p \
  com.apple.CoreSimulator.SimRuntime.tvOS-26-2
xcrun simctl boot <udid> && xcrun simctl bootstatus <udid>
xcrun simctl install <udid> mobile/tvOS/build/Build/Products/Debug-appletvsimulator/Demo.app
xcrun simctl launch --console <udid> vladas.test-engine
xcrun simctl io <udid> screenshot <path>.png
```

`SIMCTL_CHILD_HILEN_RUN_TESTS=1` with `SIMCTL_CHILD_HILEN_TEST_ONLY="<names>"` in
front of the launch runs UI tests in the app, like on the iOS simulator. There is no
tvOS lane of the whole suite yet.

`build/tvos/flight.rs` archives with no signing and lets the export sign for the store.
A signed archive wants a development profile, and Apple makes none for a team with no
Apple TV registered. The upload needs the tvOS platform added to the app in App Store
Connect first, altool otherwise says it cannot find the app for `TV_OS`.

## The remote

The buttons arrive as `UIPress`, no touch and no key input carries them. The winit fork
turns them into key events in `pressesBegan` and its 2 siblings of `WinitView`: the
arrows, Select as Enter, Menu or Back as Escape, Play/Pause as `MediaPlayPause`. A
press with no key, like the volume, goes on to the system. The key focus then drives
the UI with no code in an app, see [focus.md](focus.md).

Play/Pause does not reach the keymap. `key_event` in `app_runner.rs` sends it as
`MediaCommand::Toggle` through `MediaSession::on_command`, the way a Mac sends the
media key of a keyboard, so an app reads it the same on both.

The key focus is off while a level or a scene runs that takes the keys, see
[focus.md](focus.md). A level that is only the backdrop of a screen answers false in
`takes_keys`, like the home level of the demo, or the remote cannot leave that screen.

## The safe area

A TV may cut the edges of its picture, and tvOS reports the part that is always seen as
the safe area, 80 points in from the sides and 60 from the top and the bottom at 1080p.
On a phone the app views are the safe area, see [ios.md](ios.md). On tvOS they cover
the whole screen, so a background and a video reach the edges. `UIManager::safe_area()`
gives the safe part in the points of the app views, an app keeps its controls and its
text inside it. Everywhere else it is the whole of the app views. The code is
`resize_root` and `safe_area` in `hilen/src/ui/views/root_view.rs`, the test is `Focus
ring place`.

## Stored data

tvOS gives an app no folder that keeps its files. It has a caches folder the system
may empty while the app does not run, and the user defaults.

- `Paths::storage` is a folder in `Library/Caches`. Only what can be made again
  belongs there, the log and downloaded pictures.
- An `OnDisk` value is 1 entry of the user defaults, `hilen-store:<path under the
  root>`. A value stayed over 2 restarts and a wiped caches folder. tvOS ends an app
  whose defaults pass 1 MB, a store of over 100 KB is logged.
- The sealed session of `SessionStore` and a `SecretStore` with no Keychain are
  entries of the user defaults too, `hilen-<file>`. The `keychain` feature of
  `hilen-session` builds for tvOS with the protected store of iOS. Neither has run.

## Video

The `video` feature builds for tvOS like for iOS, see [video.md](video.md): ffmpeg from
a prebuilt archive for each of the 2 targets, VideoToolbox, the audio session. The 12
video UI tests pass in the simulator, with h264 decoded in software there.

## What still stands

- **No device run.** See [roadmap.md](roadmap.md).
- **One latent trap.** winit's `safe_area_screen_space` falls back to
  `UIApplication.statusBarFrame`, which does not exist on tvOS. It is only reached when
  the OS reports below iOS 11, so tvOS stays clear, but it is a trap if that version
  check ever moves. The five guarded selectors are the same failure shape.
