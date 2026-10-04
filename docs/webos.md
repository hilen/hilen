# LG webOS TV

An engine app runs on an LG TV as a web page, the wasm build on WebGL. The demo and the
whole UI test suite run in the web browser of the TV. There is no `make webos` target and
no `.ipk` yet, see [roadmap.md](roadmap.md).

Proven on 2026-10-04 on an LG OLED55C11LB, webOS 6.0. Its web engine is Chromium 79, it
names itself `Chrome/87.0.3945.79`, the build number 3945 is the Chromium 79 one. The GPU
is a Mali-G51. The engine of a TV never updates, a firmware update keeps the same Chromium.

## The build

Chromium 79 knows only part of the wasm features a default Rust build turns on. It has no
multivalue, no reference types and no BigInt at the JS border. So the build drops to the
`mvp` cpu and turns the 4 known features back on, and the standard library is rebuilt
with the same flags:

```bash
cd demo
RUSTFLAGS="--cfg tokio_unstable -Ctarget-cpu=mvp -Ctarget-feature=+mutable-globals,+sign-ext,+nontrapping-fptoint,+bulk-memory" \
CARGO_UNSTABLE_BUILD_STD=std,panic_abort \
trunk build --release --no-default-features --features webgl
```

`RUSTFLAGS` in the environment replaces the `rustflags` of `.cargo/config.toml`, so
`--cfg tokio_unstable` has to be repeated. `--no-default-features` leaves the 3D `scene`
feature of the demo out, the TV is too weak for it.

The start script trunk writes into `index.html` uses a top level `await`, which came in
Chrome 89. Chromium 79 stops with `Unexpected reserved word`. Until a webOS target
exists the script is patched by hand, `const wasm = await init(...)` becomes
`init(...).then(function (wasm) { ... })`, and the `integrity` attributes go.

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

203 tests run there. The Clipboard test has to be skipped with
`hilen_test_skip=Clipboard%20test`, the TV browser has no clipboard API and the test
hangs.

A recorded probe has to hold with 1 sample and with 4, see [ui-tests.md](ui-tests.md).
The Mali also draws some soft grays up to 49 off the desktop value, over the limit of
45, so a probe in a blurred or translucent area can fail there alone.

## What still stands

- No `make webos`, no `.ipk`, no fixed start script.
- No focus model. The pointer of the Magic Remote works as a mouse, arrow keys, OK and
  Back do nothing.
- No browser storage for `OnDisk`.
- The Scrolling page of the demo runs at 21 to 24 frames a second, the cause is not
  measured.
- One path still panics after a lost context, `window/state.rs` in the frame setup.
