# Hot reload on iOS

A saved file shows in the running app in the iOS simulator, with no install and
no restart of the process. Engine changes too. It is a dev mode, a release app
stays 1 static library linked into the Xcode shell.

```bash
make hot        # in an app folder: build, start in the simulator, watch the sources
make hot-test   # in hilen: the lane that proves the swap
make swap args="start demo ../apps/skaityk"   # 1 loader for several apps, see below
```

Proven on 2026-10-07 in the simulator of an Apple Silicon Mac, on iOS 16.4. A
change in `demo` is on the screen about 5 seconds after the save, of which the
build is about 1 second for an app change. Not run on a real phone, see below.

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

Proven on 2026-10-08 with `demo`, `skaityk` and `lendar`: 124 swaps in a row
in 1 process, about 55 ms from the stop of one app to the start of the next.

The build of an app outside the hilen repo does not run through `make swap`
yet. `PathBuf::push` does nothing in RustScript, see the roadmap of
rustscript, so the path to the engine comes out empty. Until that is fixed
such a library is built with the same cargo line by hand.

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
the audio manager, the media command handlers, the system text field with its
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
  That covers the 3 of winit, `RawWindowMetalLayer` of raw-window-metal and
  `HilenImagePickerDelegate`. objc2 makes its classes that way.
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

- A real iPhone. An app loads a library from its bundle. From the data folder
  there is one report of a development build that loaded on iOS 17 with Xcode
  attached and failed on iOS 18 with `code signature invalid`, see
  [the Apple forum thread](https://developer.apple.com/forums/thread/773210),
  and Apple says code outside the bundle is not supported. The loader and the
  scripts are simulator only until a load test on a phone says more.
- Video and the open image picker during a reload never ran. The stop of the
  media commands is written and not proven, it needs a played video.
- A thread the app started itself cannot be ended from outside, it runs on in
  old code.
- Assets are copied into the loader app, a changed asset needs a new
  `make hot`.
- What each reload leaves is in "What a stop leaves". A real iPhone would end
  the app long before a Mac does.
- `hilen` asks for `hilen-winit` 0.30, and the `hot` feature needs 0.30.14. An
  app with an older lock file fails the hot build with a feature error until
  `cargo update hilen-winit`.
- The watch looks at the app repo only, not at a path dependency outside it.
