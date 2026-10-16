# LG webOS TV

An engine app runs on an LG TV as a web page, the wasm build on WebGL. The demo and the
whole UI test suite run in the web browser of the TV. `make webos` builds the app for
the TV and packs it as a hosted app, see The build. What was never tried on a TV is in
What still stands.

Proven on 2026-10-04 on an LG OLED55C11LB, webOS 6.0. Its web engine is Chromium 79, it
names itself `Chrome/87.0.3945.79`, the build number 3945 is the Chromium 79 one. The GPU
is a Mali-G51. The engine of a TV never updates, a firmware update keeps the same Chromium.

## The build

`make webos-dist` builds the dist of the app for the TV, `make webos` also packs the
app. Both are `build/web/webos.rs` and read the `[webos]` table of `hilen.toml`:

```toml
[webos]
url = "https://myapp.example.com"   # where the TV loads the app from
title = "My App"                    # the name on the TV, project_name when left out
icon = "assets/icon.png"            # a square png, this path when left out
features = "webgl"                  # cargo features of the build, this when left out
default_features = true             # false leaves the default features out
packaged = false                    # true packs the whole dist, url is then not used
```

Chromium 79 knows only part of the wasm features a default Rust build turns on. It has no
multivalue, no reference types and no BigInt at the JS border. So the build drops to the
`mvp` cpu and turns the 4 known features back on, and the standard library is rebuilt
with the same flags. This is what the script runs in the app crate:

```bash
RUSTFLAGS="--cfg tokio_unstable -Ctarget-cpu=mvp -Ctarget-feature=+mutable-globals,+sign-ext,+nontrapping-fptoint,+bulk-memory" \
CARGO_UNSTABLE_BUILD_STD=std,panic_abort \
CARGO_PROFILE_DEPLOY_WASM_OPT_LEVEL=s \
trunk build --release --cargo-profile deploy-wasm --no-sri --features webgl
```

Without Liftoff, see below, the TV compiles wasm with its slow optimizing compiler only,
about 4.7 s a MB, and that compile is the whole start of the app. So the TV build alone is built for size, on the
shipping profile with fat LTO. Measured with flixen on 2026-10-15: opt-level 3 gave 11.9
MB and Rust started 37.4 s after the page, `"s"` gave 8.7 MB and 24.6 s, `"z"` 7.7 MB and
20.2 s. The frame times were the same in all 3, `"z"` took 6 points more CPU while idle.
A `wasm-opt -Oz` pass on top saved 1 to 2 %.

V8 on the TV has Liftoff, its fast first compiler for wasm, but it is off there. The
build writes `"v8ExtraFlags": "--liftoff"` into `appinfo.json`, and the web engine of
webOS hands it to V8. Measured with flixen on 2026-10-16, 5.98 MB of wasm: Rust started
after 17.7 s without it and after 10 s with it, the first screen was ready after 19.5 s
and 11.6 s. Other flags were measured too. `--wasm-lazy-compilation` was slower, 20.9 s
alone and 16.5 s with Liftoff, it compiles each function on the main thread at its first
call. `--no-wasm-tier-up` and `--wasm-num-compilation-tasks=4` changed nothing, the
compile already runs on 3 threads. With Liftoff the start grows by about 2.3 s per MB of
code, an estimate. The data of the file costs no compile time.

The TV reads `v8ExtraFlags` only when it installs the app. An edit of the installed
`appinfo.json` does nothing, `getAppInfo` of `com.webos.applicationManager` shows what
the TV took.

The compiled code is never kept between 2 starts. Chromium keeps the code of a wasm
only for a file from its http cache, and the cache of the TV is about 10 MB, too small
to keep a file over about 1/8 of it. Tried on 2026-10-16, from the `file://` app and
from an http page: every start fetched the whole file again and compiled it again.

The package also holds `splash.png`, the app icon in the middle of a black picture of
1920 by 1080, named in `splashBackground`. webOS shows it from the launch to the first
frame, so the compile is not a black screen. It was not seen on the TV yet.

`RUSTFLAGS` in the environment replaces the `rustflags` of `.cargo/config.toml`, so the
script says `--cfg tokio_unstable` again when the config has it. The demo sets
`default_features = false`, that leaves the 3D `scene` feature out, the TV is too weak
for it.

The start script trunk writes into `index.html` uses a top level `await`, which came in
Chrome 89. Chromium 79 stops with `Unexpected reserved word`. The script rewrites
`const wasm = await init(...)` into `init(...).then(function (wasm) { ... })` and fails
when trunk wrote another start script. `--no-sri` leaves the `integrity` attributes out,
their hash would not hold after the rewrite. A dist built this way also loads in every
new browser, so a server can serve the one dist to all.

The hosted app is a small `.ipk`: `appinfo.json` of type `web`, 2 icons and a splash made from the
app icon, and a page that sends the TV on to `url`. The app on the TV is so always the
one the server has. `disableBackHistoryAPI` makes the Back key reach the page. The
packer is `ares-package` of the LG CLI, `npm install -g @webos-tools/cli`, and
`ares-install` puts the `.ipk` on a TV in developer mode.

The packaged app, `packaged = true`, holds the whole dist and starts with no server. The
TV opens it from `file:///media/developer/apps/usr/palm/applications/<id>/index.html`,
and Chromium 79 there loads no module script, a file has no MIME type, and `fetch` of a
local file fails. An `XMLHttpRequest` of a local file works. So the build:

- builds from a copy of `index.html`, `webos.index.html` next to it, whose rust link has
  `data-bindgen-target="no-modules"`, so wasm-bindgen writes a classic script,
- builds with `--public-url ./`, a link from `/` would start at the root of the TV,
- puts a classic start script in place of the one of trunk, it reads the wasm by XHR and
  calls the global `wasm_bindgen`,
- leaves out the preload links, they fetch.

The engine reads a local file by XHR, `deps/refs/manage/file_xhr.rs`, so the asset
groups load from the package. The script fills `target/webos/<name>` and checks for
`ares-package` last, so a builder without the LG CLI leaves a whole folder another machine
can pack.

Every TV page gets a small script first in its head: the `WEBGL_compressed_texture_astc`
of Chromium 79 has no `getSupportedProfiles`, which came later, and wgpu calls it. The
script adds it with `["ldr"]`, what an extension without it means.

The remote debugger of the TV is the console there: `ares-inspect --device tv --app <id>`
opens the DevTools protocol on port 9998 of the TV, a page and its `console` are read
over its websocket. The inspector of hilen does not reach a packaged app, it dials the
origin of the page and a file has none.

## Size of a browser build

A browser build of hilen leaves out what a page does not run, the native builds keep all
of it. The `image` crate decodes only png, jpeg and gif, a browser decodes a downloaded
picture itself. `usvg` and `resvg` have no svg text, icons have none, that stack was
about 0.9 MB of wasm. `video` alone pulls no `audio` module, the `sound` cfg of
`hilen/build.rs` builds it only where ffmpeg plays the sound, a page plays it in the
video element. A downloaded picture other than an svg is decoded by the browser with
`createImageBitmap`, 2 at a time with 1 shared canvas, see
`window/image/browser_decode.rs`. `Image::set_download_max_side` shrinks it while it
is decoded, on every platform. The logger of a page lets only the engine and the
`log_targets` of the app through at debug, like the native one, the debug lines of naga
were about 13700 at the start of flixen on the TV.

A page opened over plain http on a LAN address works, WebGL needs no secure context.

## What the TV needs from the engine

Each of these was found on the TV and is fixed in the engine or its forks.

- **A plain canvas format.** WebGL lists `Rgba8UnormSrgb` first. `plain_format` in
  `window/state.rs` takes its plain twin, see [colors.md](colors.md). With the sRGB one
  every color got the sRGB curve twice, `#597c95` came out as `#9fb9c9`, in every WebGL
  browser.
- **No texture to texture copy.** `copyTexSubImage2D` from an `R8` texture makes the TV
  drop the whole WebGL context, with no GL error before it. The text crate did that copy
  when it forked its glyph cache, which happens when text is drawn in several batches,
  under a rounded clip for example. The cache now keeps its pixels in memory and writes
  the new texture from there, see [forks.md](forks.md).
- **1 sample.** With 4 samples at 1080p one label drew at 10 frames a second, with 1 at
  53. A browser on WebGL gets 1 sample, `window/msaa.rs`. `hilen_msaa=4` in the page
  query sets another count.
- **Pictures decoded on first use.** The TV takes the context away near 250 MB for the
  whole page. A group download keeps only the file bytes, `window/image/pending.rs`, and
  `Image::get` decodes a picture when a view first names it. The boot group of the demo
  went from 96 MB of textures and 109 MB of wasm memory to 13 and 16.
- **Shared text pipelines.** Every font built its own shader module and pipelines, about
  1.2 seconds each on the TV. The text crate builds them once. The boot of the demo
  went from 28 seconds to 3.5.
- **No atomic instructions without atomics.** Chromium 79 refuses a module with atomic
  instructions and no shared memory. `wasm_thread` is linked only in the atomics build,
  `deps/hreads/spawn.rs`.

A lost WebGL context is logged as a fatal line and the page stops drawing and shows its
fallback content, `install_context_lost_listener` in `web.rs`.

## Looking at a page on the TV

The TV has no console. Serve the dist from the Mac with a small server that puts a
script at the top of `<head>`, the script posts `console.*`, `error` and
`unhandledrejection` to a `/log` path of that server, see the phone section of the wasm
chapter of the hilen skill. Wrapping `WebGL2RenderingContext.prototype` in that script
is how the texture copy was found: the last GL calls before `webglcontextlost`, the
texture bytes and every `linkProgram` go to the same log.

The remote control socket of the TV opens a page with no typing. Pair once on
`ws://<tv>:3000`, the TV shows a prompt, then send `ssap://system.launcher/open` with
the url as `target`.

## The UI suite on the TV

The suite needs the atomics build for its worker thread. `SharedArrayBuffer` exists on
the TV with no isolation headers. Build with the flags above plus `+atomics` and the
link args of `build/web/drive.ts`, patch `demo/dist/index.html` as above, then:

```bash
bun build/web/drive.ts --no-build --browser none --timeout 6000 --stall 900
```

and open `http://<mac>:44810/?hilen_run_tests=1&hilen_inspect=1` on the TV. The results
and the failure screenshots come back over the inspect socket like in the browser lane.
A full run takes about 6 minutes. The first open of the page sometimes does not load,
open it again.

203 tests ran there on 2026-10-04. The Clipboard test had to be skipped with
`hilen_test_skip=Clipboard%20test`, the TV browser has no clipboard API and the test
hung on a call that threw. The call gives an error now and the test skips itself on it.
That was not run on the TV yet, keep the skip until it was.

A recorded probe has to hold with 1 sample and with 4, see [ui-tests.md](ui-tests.md).
The Mali also draws some soft grays up to 49 off the desktop value, over the limit of
45, so a probe in a blurred or translucent area can fail there alone.

## What the engine does for a TV

- **The remote.** Arrow keys, OK and Back drive the UI through the key focus, see
  [focus.md](focus.md).
- **Storage.** `OnDisk` keeps its files in `localStorage`, one entry per file, named
  `hilen-store:<path>`, `store/on_disk.rs`. The login session was there already.
- **Video.** `VideoView` plays in a `<video>` element under the canvas, the TV decodes
  in hardware straight to the screen, see [video.md](video.md).
- **Login.** A TV shows a QR code and a short code and the user logs in on a phone, see
  [login.md](login.md).
- **No clipboard.** `Clipboard::set_text` gives an error where the browser has no
  clipboard API, it used to throw.
- **A lost context.** `State::resize` and `State::render` stop once the context is
  gone, the resize event used to draw into the dead surface.

## What still stands

- Ran on the TV with flixen on 2026-10-15: the remote with the key focus, a menu walked
  with the arrows, video in a video element with a seek, the login of the server. The
  QR code login and the long press of OK were not tried there.
- The lost context guard has no reproduced failure behind it.
- The Scrolling page of the demo runs at 21 to 24 frames a second, the cause is not
  measured.
