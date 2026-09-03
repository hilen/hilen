# Video playback

`VideoView` plays a file or an http url, behind the `video` cargo feature. Desktop
only for now and proven on macOS, the other lanes are in [roadmap.md](roadmap.md).
`demo` and `ui-test` turn it on through a macOS target table, so the iOS, Android
and wasm builds carry none of it, and the feature fails to compile with a clear
message anywhere but desktop.

## How it plays

- ffmpeg demuxes and decodes on its own thread, `hilen/src/video/decoder.rs`, a
  few frames ahead into a bounded queue. The codec context gets the platform
  device before it opens, VideoToolbox on macOS, VAAPI on Linux, D3D11VA on
  Windows, and a `get_format` callback that picks the device's pixel format, so
  a codec the device supports decodes on the GPU and the rest fall back to
  software on their own. A decoded frame is copied back to system memory as
  NV12. Software output that is not NV12, planar YUV or 10 bit P010, goes
  through swscale once.
- The two NV12 planes upload as they are, an `R8Unorm` and an `Rg8Unorm`
  texture with the decoder's row stride, and one fullscreen pass converts them
  into an RGBA image, `hilen/src/video/nv12.rs` and `nv12.wgsl`. BT.709 or
  BT.601, limited or full range, from the stream when it says. The image is a
  managed `Image` whose texture is also a render target, so the inner
  `ImageView` draws it like any picture and aspect modes, corner radii and
  flips apply.
- Sound goes through kira, the `audio` feature. The sound track is a kira
  streaming `Decoder` with its own ffmpeg demuxer over the same source,
  `hilen/src/video/audio.rs`, resampled to packed stereo floats. Its playback
  position is the clock the picture follows. A video with no sound track
  follows the engine `Clock`, so a stepped test drives it frame by frame.
  Sound effects play on the main track at minus 20 dB, a video track lifts
  its own sound back to unity.
- A frame shows once the clock passes its timestamp minus half a frame
  interval. When more than one frame is due the newest shows and the rest
  count as dropped. A seek bumps a generation, frames from before it are
  dropped as they arrive, and decoding restarts at the keyframe before the
  target with the frames up to it decoded and skipped.
- Render on demand keeps the loop awake through an empty animation while a
  video plays, the way `AnimatedImage` does, so a paused video costs nothing.

## The API

`set_source(path or url)`, `play`, `pause`, `is_playing`, `is_loaded`,
`seek_to(seconds)`, `duration`, `position`, `set_volume(0..1)`, `set_loop`,
`set_mode(ImageMode)`, `stats() -> VideoStats` and the events `on_finish` and
`on_error`. A broken file reports through `on_error` and the log, never a
panic. The demo has a Video page with a file picker, a path field, a progress
slider and the stats line.

## The prebuilt ffmpeg

The bindings, `ffmpeg-next` with its `static` feature, link static archives
from `FFMPEG_DIR`, which `.cargo/config.toml` points at `hilen/ffmpeg`. The
headers under `hilen/ffmpeg/include` are in git, bindgen needs them before
any build script of ours runs. The libraries under `hilen/ffmpeg/lib` are not,
`hilen/build.rs` downloads the archive for the target named in
`hilen/ffmpeg/prebuilt.txt`, a release asset of github.com/hilen/build,
checks its sha256 and unpacks it once. A download that fails is a cargo
warning, `cargo check` and clippy link nothing, and the link of a binary then
fails on the missing archives.

To build a new archive, on the host it is for:

```bash
rust build/ffmpeg.rs                      # clones FFmpeg, configures, builds, dist/ffmpeg-<v>-<triple>.tar.gz
gh release create ffmpeg-<v>-<n> -R hilen/build dist/ffmpeg-*.tar.gz dist/ffmpeg-*.sha256
```

then add or update the line in `prebuilt.txt`. The script passes the flags the
`build` feature of `ffmpeg-sys-next` would, minus debug info, with the
platform's hardware decoder on and avdevice and avfilter off, so a downloaded
archive links the same way a source build did. Only `aarch64-apple-darwin`
exists so far. An app outside this repo needs its own `[env] FFMPEG_DIR` in
its `.cargo/config.toml` pointing at the engine checkout, that wiring is on
the roadmap.

The bindings link the `QTKit` framework on macOS, which the arm64 SDK no
longer ships, so every link prints one `ld: ignoring file` warning. It is
harmless and belongs upstream in `rust-ffmpeg-sys`.

## Measured

On an Apple silicon Mac, `testsrc2` files with an AAC track, read off the
demo's stats line after 14 seconds of playback:

```
╭──────────────┬──────────┬───────────────┬─────────╮
│ file         │ decoder  │ presented fps │ dropped │
├──────────────┼──────────┼───────────────┼─────────┤
│ 1080p h264 60│ hardware │ 60.0          │ 2       │
├──────────────┼──────────┼───────────────┼─────────┤
│ 4K h264 30   │ hardware │ 30.5          │ 3       │
╰──────────────┴──────────┴───────────────┴─────────╯
```

The drops are the first frames after the file opens.

## The test

`Video playback` in `ui-test-suite/src/views/video` plays `colors.mp4`, four
solid frames at one per second, every frame a keyframe, no sound. Under
stepped time it pins each frame's color, the exact frame count between them,
`on_finish` after the last frame, a seek while paused landing on its frame
and a loop wrapping to the first. Under stepped time the player waits for the
decoder before a frame renders, real time never does.
