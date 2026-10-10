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

## The picker glue is a dex

Android hands an activity result, like a picked photo or file, to Java only, and the
game activity of the template does not pass it on. So the engine ships one
small Java class, `hilen/android/NativeHandler.java`, built into
`hilen/android/native_handler.dex` and loaded at run time with
`InMemoryDexClassLoader`, API 26 and up. It lets Rust stand in for a Java
interface through `java.lang.reflect.Proxy`, which `Paths::pick_image` and
`Paths::pick_file_bytes` use for the result callback. After a change to the Java
file run `make android-dex`, it builds in the android docker image, and commit the
dex with it. A rotation while the picker is open recreates the activity and loses
the result, the pick then never finishes.

Both calls share `hilen/src/filesystem/picker/android.rs`, so 1 pick waits at a
time, a second one ends the first with `None`. An image goes to the photo picker of
Android 13 and later. A file goes to `ACTION_GET_CONTENT` with the type `*/*`. Its
extensions become MIME types by `mime_guess`: 1 type is the type of the intent,
more go into `EXTRA_MIME_TYPES`. When 1 extension has no known type there is no
filter at all, a file of that kind would be greyed out otherwise. A provider that
gives a file another type than `mime_guess` does still greys it out.
