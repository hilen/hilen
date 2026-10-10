# Hot reload on iOS and tvOS

A saved file shows in the running app in the iOS simulator, with no install and
no restart of the process. Engine changes too. It is a dev mode, a release app
stays 1 static library linked into the Xcode shell. On a real iPhone a loader
app swaps between apps the same way, on a command, see "A real iPhone". The
Apple TV simulator and a real Apple TV have the same, see "An Apple TV".

```bash
make hot        # in an app folder: build, start in the simulator, watch the sources
make hot-test   # in hilen: the lane that proves the swap
make swap args="start demo ../apps/skaityk"   # 1 loader for several apps, see below
make swap args="phone install"                # the loader on a real iPhone, see below
make hot args="tv"                            # the same in the Apple TV simulator
make swap args="tv device install"            # the loader on a real Apple TV, see below
```

Proven on 2026-10-07 in the simulator of an Apple Silicon Mac, on iOS 16.4. A
change in `demo` is on the screen about 5 seconds after the save, of which the
build is about 1 second for an app change. Proven on 2026-10-08 on an iPhone 16
Pro Max with iOS 27.0, a swap between `demo` and `skaityk` in 1 process.

## How it works

A loader app is installed once. The whole program, engine and app and `std`,
is 1 dynamic library, called a generation here. On a new build the loader stops
the old generation, leaves its image loaded, loads the new one and starts it
from scratch. The process and the `UIApplication` live on.

```
loader, installed once            library, built on every change
  main                              winit, wgpu, hilen, the app
  UIApplicationMain                 hilen_start_app
  dlopen, start, stop               hilen_stop, hilen_stopped
```

The old image is never closed with `dlclose`, so an address in old code stays
valid. The rule for it: it keeps its code and its plain memory, and it has no
work left that can touch the next generation. No thread, no task, no callback
the OS holds, no socket, no GPU object, no native object.

Every start is fresh, the app opens on its first screen again.

## The parts

- The loader, `hilen/native/ios/hot_loader.m` and `hot_loader.plist`. It owns
  `UIApplicationMain` and has no window and no event code. `HILEN_HOT_DIR` names
  a folder with the libraries and a file `current`. Its first line is the name
  of the library to run. A second line, when it is there, is the repo of that
  app, the loader sets it as `HILEN_HOT_ROOT` and the engine takes the `assets`
  from there and not from the loader app. The loader watches the folder, a
  watch on the file would not follow a file that replaced it. It loads the new
  library before it stops the old one, so a broken build leaves the old one
  running. It writes the name of the library it started into the file
  `started`. At its start it raises its limit of open files, see "What a stop
  leaves".
- The library is the app crate as a `cdylib`, built with
  `cargo rustc --crate-type cdylib`, so `Cargo.toml` of an app needs nothing.
- The `hot` feature of `hilen`. It turns `inspect` on, so the release guards of
  [inspect.md](inspect.md) keep it out of a shipped app. `cfg(hot)` in the
  engine is that feature on iOS, set by `hilen/build.rs`.
- `hilen/src/hot.rs`, the start that returns and the stop. A normal start never
  leaves the `block_on` of its tokio runtime. A hot start returns to the loader,
  so the runtime, its context on the main thread and the Sentry guard live in
  statics there.
- `hilen/native/ios/hilen_shell.m`, the 2 functions the engine takes from the
  header of the Xcode shell. Only a hot build compiles it, a normal build would
  have them twice.
- The scripts, `build/ios/hot.rs`, `build/ios/hot-test.rs` and
  `build/shared/src/hot.rs`. The builds run through `far` when it is installed,
  the simulator is the one of the Mac the script runs on.

## Several apps in 1 loader

`make swap` runs 1 loader that swaps between different apps in 1 process.
Nothing swaps by itself, every command returns, so a person and an agent use
it the same way. The state between the commands is in
`target/hot/swap/state.json`.

```bash
make swap args="start demo ../apps/skaityk ../apps/lendar"   # build all, start the first
make swap args="to skaityk"                                  # build it again and swap to it
make swap args="status"                                      # the app on the screen, threads, memory, open files
make swap args="stop"
```

A folder is the folder of the app crate, a name is its cargo package. Run from
the hilen repo, an app of another repo is built with the engine of this
folder, through a cargo patch on the command line, so every side of a swap
carries the same engine and an engine change needs no push. The lock file of
the app is put back after the build.

What 1 loader for several apps needs from the engine, both only in a hot
build: the data folder is `.<project_name>` and not `.<exe name>`, the exe is
the loader for all of them, and the assets come from `HILEN_HOT_ROOT`.

Proven on 2026-10-08 with `demo`, `skaityk` and `lendar`, all 3 built by the
tool: 124 swaps in a row in 1 process, about 55 ms from the stop of one app to the start of the next.

`make swap` needs rustscript 0.6.56 or newer. In an older one `PathBuf::push`
did nothing, and the path to the engine came out empty.

## A real iPhone

1 loader app on the phone shows the demo and swaps to any hilen app that a Mac
sends it over the local network. Run from the hilen repo:

```bash
make swap args="phone install"            # build the loader with the app of this repo inside, install, start
make swap args="phone ../apps/skaityk"    # build the app, sign it, send it, swap the phone to it
make swap args="phone status"             # the library the loader runs
```

### What iOS allows

The load test of 2026-10-08, 1 tiny library written into the data folder of the
app and loaded with `dlopen`, on iOS 27.0:

| library signed with | development build | TestFlight build |
| --- | --- | --- |
| nothing | missing code signature | missing code signature |
| an ad hoc signature | code signature invalid | code signature invalid |
| the development certificate | loaded | file system sandbox blocked mmap() |

So a TestFlight or App Store build can never hot swap. Its sandbox lets it map
code only from the system and from its own bundle, whoever signed the file, and
Apple says the same for a library with a distribution signature, see
[the forum thread](https://developer.apple.com/forums/thread/711730). A
development build loads a library from its data folder when the library has the
signature of the same development certificate. Apple calls that a looser rule
for development and gives no promise for it.

### How it works

- The loader is a development build with the bundle id of `hilen.toml`,
  `vladas.test-engine`, so it takes the place of the TestFlight demo on the
  phone. `build_device_loader` in `build/shared/src/hot.rs` builds it with
  `clang`, with no Xcode project. It takes the development profile Xcode once
  made for that id, puts it into the app and signs with the development
  certificate of the Mac. With no such profile, build the app for a phone in
  Xcode once.
- On a phone nothing sets `HILEN_HOT_DIR`. The loader uses the folder `hot` in
  its own Application Support folder and sets the variable itself.
- The loader app carries the app of the repo as `Frameworks/default.dylib`. It
  runs when nothing was sent yet, and when a sent library does not start. A
  library that ends the process at its start leaves the file `starting`, the
  next launch sees it and runs the packed one.
- A phone cannot read the Mac. The app that runs takes the next library and
  the assets of its app over the inspect connection, `hilen/src/inspect/hot_swap.rs`,
  and writes them into the hot folder. Then it writes `current`, and the
  loader swaps as in the simulator. The commands are `HotInfo`, `HotFiles`,
  `HotChunk` and `HotSwap`, a file goes in pieces of 8 MB. An asset whose
  length and hash are there already is not sent again. Only the library that
  runs and the next one stay on the phone.
- A phone that is not on the network of the Mac is reached by its address, over a
  VPN like Tailscale: `HILEN_INSPECT_ADDR=<phone> make swap args="phone ../apps/skaityk"`.
  The loader takes the fixed inspect port, see [inspect.md](inspect.md), and takes it
  again after a swap, a hot build asks for it for 1 second. The sender then asks the
  same address until it names the new library. The loader has to be in front on an
  unlocked phone, and it has to be a build from 2026-10-10 or later. The install of
  the loader still needs a cable or the Wi-Fi of the Mac. Proven on 2026-10-10 on an
  iPhone 16 Pro Max by its Tailscale name, several swaps in a row, each on the fixed
  port again. The phone was on the Wi-Fi of the Mac then, so Tailscale sent the data
  straight. Not proven: a phone on a mobile network.
- `hilen-inspect hot-send <library> <name> --assets <folder>` is the sender, and
  `hilen-inspect hot-status` asks what runs. The loader is found by what it
  answers: the only app on the network that knows `HotInfo`, or `--app <id>`.
  A swap starts a new build with a new id and port, so the sender waits until
  a hot build at the address of the phone names the new library.
- No check of our own guards the send. iOS refuses every library that is not
  signed with the development certificate, that is the guard.
- The hot compiler flag goes where the app keeps its own. Cargo ignores
  `build.rustflags` when the repo has `rustflags` for the target, as `skaityk`
  has for `aarch64-apple-ios`, so `hot_flag` reads `.cargo/config.toml` of the
  app and picks the key.

### Measured

A swap from `demo` to `skaityk` and back in the process 54243: the stop of an
app took 3 to 5 ms, the next app had its UI about 150 ms after the stop began.
`make swap args="phone demo"` with nothing to compile took 31 seconds, nearly
all of it the send of the 145 MB library and 264 asset files over Wi-Fi.

### Limits

- The install ends with its development profile, 1 year, and only runs on
  phones registered in the developer account.
- TestFlight offers its own build of the same bundle id as an update, which
  removes the loader. Turn automatic updates off for it in the TestFlight app.
- The first install goes through `devicectl`, by cable or over Wi-Fi once the
  phone is paired. The phone has to be unlocked for the start.
- Not run yet: many swaps in a row and what each leaves in the memory of a
  phone, a repeat send of the same app, and the start guard.

## An Apple TV

The word `tv` in front asks for an Apple TV. Everything else is the same code
as for a phone.

```bash
make hot args="tv"                            # in an app folder, the watch of saved files
make hot-test args="tv"                       # in hilen: the lane, in the Apple TV simulator
make swap args="tv start demo ../apps/flixen/crates/flixen"
make swap args="tv to flixen"
make swap args="tv status"
make swap args="tv stop"
make swap args="tv device install"            # a real Apple TV: build the loader, install, start
make swap args="tv device ../apps/flixen/crates/flixen"   # build, sign, send, swap
make swap args="tv device status"
make load-test args="tv"                      # the load test on the paired Apple TV
```

Proven on 2026-10-10. In the simulator of tvOS 26.2: the lane with 6 swaps in
1 process, a swap between `demo` and flixen, and a reload 6.1 seconds after a
saved file. On an Apple TV 4K of the 3rd generation with tvOS 26.3: the load
test, the install of the loader, and swaps between `demo` and flixen in 1
process, about 55 ms from the stop of one app to the start of the next.

What differs from a phone:

- tvOS has no prebuilt standard library, so the library is built with
  `-Z build-std=std,panic_abort`. `cfg(hot)` is on for tvOS too.
- The crate `dispatch` 0.2 asks the linker for a library `dispatch` on every
  system but macOS and iOS, and tvOS has none. A normal tvOS build is a static
  library and links nothing. A hot library failed with `library 'dispatch' not
  found`. `hilen/native/tvos/dispatch_stub.c` is that library with nothing in
  it, `hilen/build.rs` compiles it for a tvOS hot build only.
- The loader has its own `Info.plist`, `hot_loader_tvos.plist`, with the device
  family of a TV.
- Everything of a tvOS hot build is under `target/hot/tv`, so both simulators
  run at the same time.
- The simulator is the first booted Apple TV, else the first one at 1080p. The
  lists of `simctl` are split by their `-- tvOS` lines, a name tells nothing.
- A TV has no system text field and no image picker, the stop skips both.

### A real Apple TV

The load test of 2026-10-10 on tvOS 26.3 gave the same answers as on a phone:
no signature and an ad hoc signature are refused, a library signed with the
development certificate loads. `make load-test` is that test as a script now,
`build/ios/load-test.rs` with the app `hilen/native/ios/load_test.m`.

- Pair the TV once. On the TV open Settings, Remotes and Devices, Remote App
  and Devices, and leave it open. On the Mac run `xcrun devicectl manage pair
  --device "<name of the TV>"` in a terminal and type the code the TV shows.
  The code belongs to that run of the command.
- The development profile for tvOS is made by 1 build of `mobile/tvOS` for the
  TV: `xcodebuild -project Demo.xcodeproj -scheme Demo -destination
  'platform=tvOS,id=<udid>' -allowProvisioningUpdates
  -allowProvisioningDeviceRegistration build`. It registers the TV in the
  developer account. An app has a profile for each system under the same id,
  `development_profile` picks by the system.
- tvOS lets an app write only into its caches folder, so the hot folder is
  `Library/Caches/hot`. The system may empty it while the app does not run, the
  loader then runs the packed library.
- `devicectl device process launch --console` shows the lines of the loader
  and of the app. When that command ends on the Mac, the app on the TV ends.
- The crash report of the loader comes off the TV with `devicectl device copy
  from --domain-type systemCrashLogs`, and the log file of the app with
  `--domain-type appDataContainer` from `Library/Caches/Logs`.

Not explained: the first swap after the install, from the packed `demo` to
flixen, did not start in 90 seconds. The old app had stopped and the process
lived. No console was attached. Every later swap worked.

## The heap of a build

A stopped build is never unloaded, so nothing runs the drop of what its
statics and caches still hold. That was about 10 MB per swap. In a hot build
every Rust allocation comes from a heap of its own, a malloc zone,
`hilen/src/hot/heap.rs`. The loader calls `hilen_free_heap` for a stopped
library at a later swap, not before 5 seconds after its stop, when its last
threads have ended.

The price: memory of a stopped build that anything still reads is a crash,
where it was a silent leftover before. So a hole in the stop shows now.

## What a stop leaves

Measured over swaps between 3 apps, 2026-10-08:

- About 3 MB of real memory per swap, 104 MB at swap 1 and 435 MB at swap 100.
  Nearly all of it is the constants of the library that the system patched
  at the load. Before the own heap it was about 11 MB per swap.
- 2 open files per swap, 1 pair of sockets. tokio makes it for its signal
  handling as soon as a runtime has IO, and keeps it in a private static. The
  usual limit of 256 open files ended the process at swap 31, so the loader
  raises its limit.
- About 390 file mappings per swap, the system fonts. They cost address space
  and no real memory.
- The log file and the kqueues of the runtime are given back. The runtime
  handle of the main thread is freed at the stop, a handle that lives on keeps
  the kqueue open.

## The stop

`hilen_stop` gives back, in this order: the level and the scene, every view,
the audio manager, after 100 ms for the sound of an open video to stop, with the feed timer of the sound clocks, the media command handlers, the system text field with its
observer and callbacks, the picker delegate, the mDNS thread, the queued main
thread work, the window with its surface, the event loop, Sentry and the tokio
runtime. `hilen_stopped` then answers 1 once the Objective-C classes are
deleted, the loader asks every 50 ms.

## No rayon

The global rayon pool has no shutdown, each generation would leave 10 sleeping
threads. So a hot build is compiled with `--cfg hilen_hot`, which `make hot`
passes. That flag takes the rayon lines of rapier and `image` out of
`hilen/Cargo.toml`, and the text brush is built with `with_multithread(false)`,
its glyph cache draws on rayon too. `hilen/build.rs` stops a build with the
`hot` feature and no flag. Physics, text and image decoding run on 1 thread in
a hot build.

## The winit fork

winit has no app delegate, it ties into iOS with the `UIApplicationMain` call,
run loop observers and notification observers. The feature `ios-attach` of
`hilen-winit` adds, see [forks.md](forks.md):

- `EventLoopExtIOSAttach::run_app_attached`. It skips `UIApplicationMain`, goes
  through the launch states by itself and returns. An app that is active
  already gets its `Resumed` from it, iOS says that only once. The launch
  observer skips an attached loop, or a first start from the launch delegate
  would launch twice.
- `detach`. It ends the loop, removes the 3 run loop observers, the timer and
  the notification observers. Not from inside an event callback.
- `release_classes`. A dropped window is hidden first, UIKit holds a visible
  one.

## Objective-C classes

A class name is taken once per process, and every generation brings the same
names.

- A class made at run time is deleted at the stop with
  `objc_disposeClassPair`, and the next generation registers the name again.
  That covers the 3 of winit, `RawWindowMetalLayer` of raw-window-metal,
  `HilenImagePickerDelegate` and `HilenDocumentPickerDelegate`. objc2 makes its
  classes that way. A new `define_class!` in the engine has to join `CLASSES` in
  `hot.rs`.
- A delete does not check for live objects, and UIKit holds a hidden window a
  moment longer. So winit keeps a weak reference to each window, view and view
  controller, and deletes only when all are gone. In the proof that took 0.1
  seconds.
- A class compiled from a `.m` file cannot be deleted. A second copy loads
  anyway, the runtime prints 1 warning, and the new image uses its own copy.
  That covers `HilenTextField` and `HilenTextBridge`.

## The arm64 simulator

The hot build targets `aarch64-apple-ios-sim`. The x86 build of `make ui-ios`
runs through Rosetta, which keeps its translation of every library a process
ever loaded, 37 MB per reload. Native it is about 6 MB. The ffmpeg archive of
this target is in [video.md](video.md).

## The lane

`make hot-test` builds the `hot-test` app 2 times, the second time with its
`second` feature, which stands for a saved change. It starts the first build,
swaps the 2 in one process 6 times, and checks after every swap that the new
code wrote its marker file, that the middle of the screen has its color, that
the process id is the same, and that the thread count does not grow. It is not
part of `make smoke`.

## Open

- `make hot`, the watch of a saved file, is simulator only. A phone swaps on a
  command, see "A real iPhone". One report says a development build failed to
  load from its data folder on iOS 18, see
  [the Apple forum thread](https://developer.apple.com/forums/thread/773210),
  on iOS 27.0 it loads.
- The send to a phone is slow for a big library, the file goes as it is, with
  its symbols and not packed.
- The stop of the feed timer of a sound clock was never compiled in a hot build.
- A swap after a played video crashed on a real Apple TV on 2026-10-10. Every
  closed video left its sound decode thread of kira running, and the thread
  read the memory of its build after a later swap freed it. The sound track of
  a video now stays until its sound has stopped, `persist_until_sounds_finish`
  in `video/player.rs`, and the stop waits for a video that is still open. The
  swap after a played video was not run again after the fix. The open image
  picker during a reload never ran.
- A thread the app started itself cannot be ended from outside, it runs on in
  old code.
- In the simulator assets are copied into the loader app, a changed asset
  needs a new `make hot`. A send to a phone brings the changed assets along.
- What each reload leaves is in "What a stop leaves". A real iPhone would end
  the app long before a Mac does.
- `hilen` asks for `hilen-winit` 0.30, and the `hot` feature needs 0.30.14. An
  app with an older lock file fails the hot build with a feature error until
  `cargo update hilen-winit`.
- The watch looks at the app repo only, not at a path dependency outside it.
