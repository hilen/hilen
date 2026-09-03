# Android

## Build

`make android` builds APKs with every ABI inside docker, the same flow CI runs. `make
android-emu` builds an arm64 debug APK only, for the emulator on an Apple Silicon host.
No Android tooling on the host, docker is the only requirement.

The `HILEN_ANDROID_ABI` env var carries the single ABI choice into the container.
The generated gradle project always lists all four ABIs, `build.rs` trims the list after
every regeneration and runs `assembleDebug` instead of the full `build`.

Gradle on a docker bind mount misses changed inputs even with vfs watching off. It
reports the merge task executed while packing a stale `.so` into the APK, so `build.rs`
deletes the jni intermediates before every build, which forces the merge, strip and
package tasks to run every time.

## The backend is Vulkan alone

`Window::instance` asks for `Backends::VULKAN` on android. With GL and Vulkan both
enabled they race for the one `ANativeWindow`. The loser gets
`ERROR_NATIVE_WINDOW_IN_USE_KHR` and wgpu-hal panics instead of skipping that backend.
A native window holds one producer, which is also why `start_internal` drops the
adapter probe surface before `Surface::new` connects the real one.

GL is no fallback. GLES may report zero fragment stage storage buffers, the legal
minimum, and the UI pipelines bind one. The emulator's GLES does exactly that while its
Vulkan works.

## No host lane compiles the android code

Everything behind `#[cfg(android)]` is invisible to `make ci` and `make smoke`, so it can
break while every desktop lane stays green. The docker build is the only check it has,
so it runs with `RUSTFLAGS=-D warnings` and a warning fails it like clippy fails desktop.
That is how the jni bump from 0.21 to 0.22 left `Clipboard` and `open_url` calling
methods that no longer existed, and the break only showed up in CI.

Behind Docker Desktop's proxy on a mac, git inside the container can hang on the
template clone or fail every anonymous fetch. Forcing `http.version=HTTP/1.1` through
the `GIT_CONFIG_COUNT` environment variables on the container fixes it, CI runners
have no such proxy.

## Logging

Android goes through the same fern dispatch as every other platform, with
`android_logger` as its output, so logcat carries the lines and the bug report ring
is fed there too. There is no log file, `log_file::create` refuses on android.

## Assets come from the APK

Android assets live inside the APK, not on the filesystem. `filesystem::read_bytes`
reads them through the `AAssetManager` of the `AndroidApp` that `android_main` hands
over before the event loop consumes it. Asset paths are already relative on android,
`images/engine.png`, and match APK asset paths exactly. The gradle project packs the
repo `assets/` folder via `sourceSets`.

## The app must register from the shell crate

`register_app!` in an app lib behind `cfg(ios)` never reaches the android cdylib. The
weak `hilen_create_app` stub wins the link and panics at startup. The android
shell crate, `demo-android`, invokes the macro itself, the same way the desktop
binary does in `main.rs`.

## Pending template fixes

The generated project comes from the hilen-mobile template cloned at `main` on every
build. Four fixes still live only in the generated files, so a regeneration reverts
them until they land in the template:

- `androidx.games:games-activity` must be `4.4.0`, the version the `android-activity`
  crate bundles. With `2.0.2` `RegisterNatives` aborts the process at startup.
- `mergeDebugJniLibFolders` and `mergeReleaseJniLibFolders` must depend on
  `cargoBuild`, or packaging can run before the fresh `.so` lands.
- `org.gradle.vfs.watch=false` in `gradle.properties`, file watching cannot work in
  the container.
- The `assets/` source dir wiring described above.
