# tvOS

The engine builds for tvOS and renders in the Apple TV simulator. The demo menu
draws fully, wgpu on Metal, images, fonts and cards, and the process stays alive. It is
display only. There is no input path, so nothing can drive the UI, and it has never run
on real hardware.

Proven on 2026-07-30, toolchain nightly-2026-07-03, tvOS SDK 26.2, simulator runtime
tvOS 26.2 on an Apple TV 4K device at 1080p.

## The four cfg gaps, closed durably

The first probe proved these with throwaway patches that were later lost. They are now
permanent:

- `plat` is vendored at `deps/plat` as 0.10.0 and is a workspace member. It stayed its own
  crate when the other foundational crates were folded into `hilen`, because `hilen`,
  `demo` and `ui-test-suite` all call its `platforms()` from their build scripts, and a
  crate cannot use its own code in its build script. `ios` means ios or tvos, a separate
  `tvos` alias marks the real differences, `mobile` and `apple` include tvos, and
  `Platform` has a `TVOS` const. Every in-tree consumer points at the path, so one copy
  resolves everywhere.
- `winit` is forked. Branch `tvos-0.30` at github.com/VladasZ/winit sits on the exact
  0.30.13 release commit and is pinned by `rev` in the workspace `Cargo.toml`. It adds
  tvos to `ios_platform` and `apple` in `build.rs` plus five `[target.'cfg(...)']`
  sections in its `Cargo.toml`, and guards five `UIViewController` calls that the SDK
  marks unavailable on tvOS, both `setNeedsStatusBarAppearanceUpdate` call sites,
  `setNeedsUpdateOfHomeIndicatorAutoHidden`,
  `setNeedsUpdateOfScreenEdgesDeferringSystemGestures` and
  `attemptRotationToDeviceOrientation`. The home indicator one aborts the app at window
  creation otherwise. winit's own capability guard cannot catch it, it checks OS
  versions, not platforms.
- `mach2` needed no fork. kira 0.12.2 pulls cpal 0.18.1 which pulls mach2 0.6.0, and
  0.6.0 already allows tvOS. The mach2 0.4.3 that arrives via audio_thread_priority
  never enters the tvOS target graph.
- The three `target_os = "ios"` gates in `hilen/Cargo.toml` include tvos.

## Building

Tier 3 target, no prebuilt std, so `-Z build-std` is required. The repo is already on
nightly with `rust-src`.

```bash
cargo build -p demo --lib --target aarch64-apple-tvos     -Z build-std=std,panic_abort  # device
cargo build -p demo --lib --target aarch64-apple-tvos-sim -Z build-std=std,panic_abort  # simulator
```

The bin target cannot link and that is fine. The old `dispatch` 0.2 crate wants a
standalone libdispatch no Apple mobile SDK ships. The shell links the staticlib and
libSystem carries libdispatch, exactly as iOS already does. Build `--lib` and ignore
the bin.

## The simulator shell

`mobile/tvOS/` holds a hand made shell. It is gitignored with the rest of `mobile/`,
so everything needed to recreate it is recorded here. Its durable home should be the
hilen-mobile template, which today generates iOS projects only.

- `Demo/main.m` calls `hilen_start_app()`, same as iOS.
- `Demo/hilen.h` defines the five `hilen_ios_*` symbols. They are the
  iOS implementations as is, UIAlertController and UITextField exist on tvOS, except
  `hilen_ios_get_icloud_storage_path` returns NULL, tvOS has no iCloud document
  storage.
- `Demo/Info.plist` is minimal plus `UIDeviceFamily` = 3, Apple TV.

No Xcode project. Build the app bundle by hand, the assets folder is copied to the
bundle root where the ios asset path already looks, next to the executable:

```bash
mkdir -p mobile/tvOS/build/Demo.app
cp mobile/tvOS/Demo/Info.plist mobile/tvOS/build/Demo.app/
cp -R assets mobile/tvOS/build/Demo.app/assets
xcrun -sdk appletvsimulator clang -target arm64-apple-tvos26.2-simulator -fobjc-arc \
  mobile/tvOS/Demo/main.m target/aarch64-apple-tvos-sim/debug/libdemo.a \
  -framework UIKit -framework Foundation -framework Metal -framework QuartzCore \
  -framework CoreGraphics -framework AudioToolbox -framework AVFoundation \
  -framework Security -framework SystemConfiguration -lobjc \
  -o mobile/tvOS/build/Demo.app/Demo
codesign -s - --force mobile/tvOS/build/Demo.app
```

Run it. The runtime comes from `xcodebuild -downloadPlatform tvOS`, a 3.6 GB download:

```bash
xcrun simctl create te-AppleTV-26.2 \
  com.apple.CoreSimulator.SimDeviceType.Apple-TV-4K-1080p \
  com.apple.CoreSimulator.SimRuntime.tvOS-26-2
xcrun simctl boot <udid> && xcrun simctl bootstatus <udid>
xcrun simctl install <udid> mobile/tvOS/build/Demo.app
xcrun simctl launch <udid> vladas.hilen
xcrun simctl io <udid> screenshot <path>.png
```

The window comes up inset by the tvOS overscan safe area, a colored frame around the
UI. That is standard TV behavior, not a bug, and worth a decision once display grows
into a real target.

## What still stands

- **Input is the real work.** The UI is touch driven through `WindowEvent::Touch` and
  Apple TV has no touch screen. Siri Remote events arrive through the UIKit focus
  engine and `UIPress`, and winit forwards direct touches only. Tracked in
  [roadmap.md](roadmap.md), and it decides what a tvOS UI test could assert.
- **No device run.** Real hardware needs signing and a durable shell first.
- **One latent trap.** winit's `safe_area_screen_space` falls back to
  `UIApplication.statusBarFrame`, which does not exist on tvOS. It is only reached when
  the OS reports below iOS 11, so tvOS stays clear, but it is a trap if that version
  check ever moves. The five guarded selectors above are the same failure shape.
