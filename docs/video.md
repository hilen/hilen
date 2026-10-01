# Video playback

`VideoView` plays a file or an http or https url, behind the `video` cargo feature. Desktop
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
  NV12, or as P010 for 10 bit content. Planar YUV from a software decoder
  goes through swscale once, into P010 when it has more than 8 bits.
- The two planes upload as they are, an `R8Unorm` and an `Rg8Unorm` texture
  with the decoder's row stride, and one fullscreen pass converts them into an
  RGBA image, `hilen/src/video/nv12.rs` and `nv12.wgsl`. BT.709, BT.601 or
  BT.2020, limited or full range, from the stream when it says. P010 planes
  go up as `Rg8Unorm` and `Rgba8Unorm`, 2 bytes a sample, and the shader joins
  the bytes, so no 16 bit texture format is needed. A PQ or HLG frame is tone
  mapped to SDR in that pass: the BT.2390 curve in PQ space for a 1000 nit
  source and a 203 nit SDR white, on the brightest channel so the hue stays,
  then BT.2020 to BT.709 and the BT.1886 encode. The image is a
  managed `Image` whose texture is also a render target, so the inner
  `ImageView` draws it like any picture and aspect modes, corner radii and
  flips apply.
- Sound goes through kira, the `audio` feature. The sound track is a kira
  streaming `Decoder` with its own ffmpeg demuxer over the same source,
  `hilen/src/video/audio.rs`, resampled to packed stereo floats. Its playback
  position is the clock the picture follows. At a speed other than 1 the
  floats go through ffmpeg's `atempo` filter, which keeps the pitch, and kira
  hears a sound that is the media length divided by the speed, so its
  position times the speed is the position in the stream. A speed change or
  a sound track switch opens a fresh sound decoder at the current position.
- Text subtitles of a track inside the source are decoded on the video
  thread as their packets pass by, `hilen/src/video/subtitles.rs`, and sent
  as cues with the seek generation, like frames. Picking a track decodes
  again from the current position with the sound left alone, the demuxer
  runs ahead of the picture and the cues had already gone by. A subtitle
  file from outside the source is read whole on its own thread. ffmpeg hands
  every text format over as an ASS line, the styling is dropped. The player
  shows the cue the clock is in. A video with no sound track
  follows the engine `Clock`, so a stepped test drives it frame by frame.
  Sound effects play on their own track, 20 dB down until the app calls
  `Sound::set_volume`. A video track is a separate track at its own volume.
- A frame shows once the clock passes its timestamp minus half a frame
  interval. When more than one frame is due the newest shows and the rest
  count as dropped. A seek bumps a generation, frames from before it are
  dropped as they arrive, and decoding restarts at the keyframe before the
  target with the frames up to it decoded and skipped.
- A stream that stalls holds playback. When the next frame is 0.25 seconds
  late, the queue is empty and the stream has not ended, the clock and the
  sound are held and the state is `Buffering`. Both go on once the queue is
  full again or the stream ends. A play before the source opened waits the
  same way, so the clock never runs ahead of the first frame. Stepped time
  never holds, a stepped test waits for the decoder instead.
- Each demuxer opens with an interrupt callback over a stop flag the player
  sets when it drops, so a read that waits on the network does not keep its
  thread after the view is gone.
- Render on demand keeps the loop awake through an empty animation while a
  video plays, the way `AnimatedImage` does, so a paused video costs nothing.

## The API

`set_source(path, url or VideoSource)`, `play`, `pause`, `is_playing`, `is_loaded`,
`seek_to(seconds)`, `duration`, `position`, `set_volume(0..1)`, `set_loop`,
`set_mode(ImageMode)`, `stats() -> VideoStats`, `state() -> VideoState` and the
events `on_finish`, `on_error` and `on_state`. `VideoState` is `Empty`,
`Loading`, `Paused`, `Playing`, `Buffering`, `Finished` or `Failed`, and
`on_state` fires on every change. `is_playing` stays true while buffering.
A `VideoSource` carries request headers for an http or https source,
`VideoSource::new(url).header("Authorization", token)`, so a session token
stays out of the url. The picture and the sound demuxer both send them. The
headers never reach a log line.

Tracks: `audio_tracks`, `audio_track` and `set_audio_track(index)`, which keeps
the position, `subtitle_tracks`, `set_subtitle_track(Some(index))`,
`set_subtitle_file(path or url)`, `subtitle()` and the event `on_subtitle`,
which carries the line to show and none when it ends. The view draws no
subtitles, the app puts the text in a `Label`, with `set_text_outline` it
reads over any picture. `set_speed(0.5 to 4)` plays faster or slower with the
pitch kept. A broken file reports through `on_error` and the log, never a
panic. The demo has a Video page with a file picker, a path field, a progress
slider and the stats line.

## The prebuilt ffmpeg

The bindings are `ffmpeg-next` with its `static` feature, from the hilen forks of
`rust-ffmpeg` and `rust-ffmpeg-sys`, see [forks.md](forks.md). Nothing of ffmpeg
lives in this repo. The build script of the forked `ffmpeg-sys-next` reads the
`prebuilt.txt` next to it, downloads the archive for the target, a release asset
of github.com/hilen/build with the headers and the libraries, checks its sha256
and unpacks it once into its `OUT_DIR`. So an app outside this repo turns
`video` on and builds with no setup and no engine checkout. The fetch lives in
that build script because cargo orders nothing between a consumer's build script
and the bindings crate, which needs the archive the moment it compiles. A
download that fails fails the build script with the reason. `FFMPEG_DIR`, when
set, wins and is used as it is, that is how a fresh archive is tried before it
is published, `FFMPEG_DIR=$PWD/target/ffmpeg-dist`.

To build a new archive, on the host it is for:

```bash
rust build/ffmpeg.rs                      # clones FFmpeg, configures, builds, dist/ffmpeg-<v>-<triple>.tar.gz
gh release create ffmpeg-<v>-<n> -R hilen/build dist/ffmpeg-*.tar.gz dist/ffmpeg-*.sha256
```

then update the line in `prebuilt.txt` of the sys fork, push it, pin its rev in
the `rust-ffmpeg` fork, push that, and pin its rev in the root `Cargo.toml`.
The script passes the flags the `build` feature of `ffmpeg-sys-next` would,
minus debug info, with the platform's hardware decoder on and avdevice and
avfilter built with only the `atempo` filter and the 2 ends of its graph.
Autodetect is off, so TLS is named too, SecureTransport on macOS, without it
the archive has no https protocol. zlib is on, and the script builds dav1d,
the software AV1 decoder, into the same prefix first, it needs `meson` and
`ninja`. `lib/link.txt` in the archive names what has to be linked besides the
ffmpeg libraries, one `<kind>=<name>` per line, and the sys fork links those.
Only `aarch64-apple-darwin` exists so far.

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

A 4K HEVC main 10 file with PQ, `testsrc2` again, no sound, measured the same
way with no window: 60.0 presented fps of 60 and 30.0 of 30, hardware decode,
0 dropped. With no window there is no vsync, so this is the decode, the copy,
the upload and the tone map pass keeping up, not the display.

## The tests

`Video stream` in `ui-test-suite/src/views/video` plays `stream.mp4`, 6 seconds
of the ffmpeg `testsrc2` pattern, from a local http server inside the test that
holds the file back from the frame at 3 seconds. In real time it pins the token
header on every request, the state order loading, paused, playing, buffering,
playing, finished, and the position held while buffering. The unit test
`https_protocol_is_linked` fails when the archive has no TLS.

`Video tracks` plays `tracks.mkv`, 2 tone tracks and 2 subrip tracks, muted,
and walks the track lists, the lines in step with the clock, a sound track
switch that keeps the position, a subtitle switch, a seek and a subtitle
file. `Video hdr` shows a PQ, an HLG and a 10 bit SDR fixture of flat bars,
HEVC main 10 coded lossless, whose colors were computed from the standards'
formulas outside the engine. `Video speed` pins the position at double and
half speed under stepped time. `Video av1` plays an AV1 copy of the color
fixture and pins that dav1d decodes it. The unit tests next to the code pin
which sound track is heard, 440 against 1760 Hz, the length and the kept
pitch at every speed, the cues of a track and of a file, and that dav1d and
zlib are in the archive.

`Video playback` in `ui-test-suite/src/views/video` plays `colors.mp4`, four
solid frames at one per second, every frame a keyframe, no sound. Under
stepped time it pins each frame's color, the exact frame count between them,
`on_finish` after the last frame, a seek while paused landing on its frame
and a loop wrapping to the first. Under stepped time the player waits for the
decoder before a frame renders, real time never does.
