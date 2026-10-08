# Video playback

`VideoView` plays a file or an http or https url, behind the `video` cargo feature. On
desktop and on iOS ffmpeg decodes, proven on macOS, on Windows x64 and on the iOS
simulator, see On iOS. In a browser the page plays the source in a `<video>` element,
see In a browser. The other lanes are in [roadmap.md](roadmap.md). `demo` and `ui-test`
turn the feature on through a target table for those systems, so the Android and tvOS
builds carry none of it, and the feature fails to compile with a clear message anywhere
else. The cfg alias `ffmpeg` of `deps/plat` names the targets ffmpeg decodes on.

## How it plays

- ffmpeg demuxes and decodes on its own thread, `hilen/src/video/decoder.rs`, a
  few frames ahead into a bounded queue. The codec context gets the platform
  device before it opens, VideoToolbox on macOS and iOS, VAAPI on Linux, D3D11VA on
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
  The old track plays on and stays the one `audio_track` names until the
  new one has opened, a track that fails to open changes nothing.
  Every open of a sound decoder after the first runs on a thread of its own,
  and until it is there the video follows the engine clock. kira runs the
  first seek of a decoder inside `play`, on the main thread, so that seek
  reads nothing and the first decode on kira's thread goes to the place.
  A slow network never holds the frame loop this way.
- Text subtitles of a track inside the source are decoded on the video
  thread as their packets pass by, `hilen/src/video/subtitles.rs`, and sent
  as cues with the seek generation, like frames. Picking a track decodes
  again from the current position with the sound left alone, the demuxer
  runs ahead of the picture and the cues had already gone by. A seek
  restarts the picture at the keyframe before the target, and the packet of
  a line that began before that keyframe lies behind it. So every seek with
  a track on also starts a read back on its own thread, a demuxer of its own
  that reads the subtitle packets of the 10 seconds before the target and
  hands over the line that is on screen there. When it finds none, it seeks
  on the subtitle stream itself, to the last line that began before the
  target however far back, so a line longer than 10 seconds shows too. That
  read back is only the fast answer. Picking a track also reads all its
  lines once on a thread of its own, `hilen/src/video/subtitles/track_read.rs`,
  and keeps them over every seek. With an index of the subtitle packets,
  mp4 always and matroska when its cues list them, it jumps from line to
  line. Without one it reads the source through with every other stream
  thrown away. Once that is done a seek reads nothing back, and a long line
  under a shorter one shows again when the shorter one ends. Of the lines
  on screen at once the one that began last shows. A subtitle
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
  full again or the stream ends. The sound is the clock, and on a stalled
  stream its own read can run dry before the picture is late. So a clock that
  has stood still for 0.25 seconds with an empty queue holds playback the same
  way. A play before the source opened waits the
  same way, so the clock never runs ahead of the first frame. Stepped time
  never holds, a stepped test waits for the decoder instead.
- Each demuxer opens with its own `Interrupt`, `hilen/src/video/source.rs`. It
  ends a read that waits on the network when the player drops, so the thread
  does not outlive the view, and when a seek must not wait behind it.
- A seek while the stream is stalled, `hilen/src/video/player/stall.rs`. The
  picture thread takes a seek only between packets and kira only between
  reads. For 5 seconds after a seek the player watches both demuxers, and a
  read that has lasted more than half a second is taken as stalled. The
  picture thread then gets its read broken, opens the source again and takes
  the seek. Some demuxers report a broken read as the end of the file, so
  the break is told by the flag and not by the error. The sound cannot be
  handled that way, its decoder lives on kira's thread and kira calls a
  decoder that fails again with no pause. So the stalled sound is dropped,
  its decoder ends in silence, and a fresh one opens on a thread and plays
  from where the video is. Until then the video follows the engine clock.
- A sound that dies opens again, `hilen/src/video/player/faults.rs`. kira
  stores the error of a decoder, for example a read of the stream that
  failed, and stops the sound for good. A pause and a play do not bring a
  stopped sound back. So a sound that stops more than a second before the
  end gets a fresh decoder on a thread, at most once every 2 seconds, and
  plays on from where the video is. The log names the error of the decoder,
  the place the sound stopped at, and each time it opens again.
- A network stream that breaks opens again,
  `hilen/src/video/decoder/reconnect.rs`. A server closes a connection
  nobody reads from, nginx after 60 seconds, so a long pause ends with a read
  that fails, or with an end of the file before its last byte. The picture
  thread then opens the source again and decodes on from the place it stood
  at. The first try runs at once, the next ones after 0.25 seconds, doubling
  up to 2. The player gets no frames in that time, so the state is
  `Buffering`. After 30 seconds without a packet it gives up, and the video
  is `Failed` with the last error. A path or a `file:` url fails at once.
  The log has a warning with the url, the seconds, the byte and the error
  when the stream breaks, a line for every try and an error when it gives
  up. The http options of ffmpeg, `reconnect` and the others, are left off:
  ffmpeg would write its tries only to stderr, and 2 layers of tries would
  add up their times.
- A cut can hide. The matroska demuxer answers a read that failed in the
  middle of a block by going back to the start of that block. That place is
  often out of its buffer, so ffmpeg asks the server again on a new
  connection, and the demuxer skips to the next cluster and reports nothing.
  The packets up to that cluster are lost. For the sound that is worse than
  a gap: kira counts samples, so all the sound after it plays too early. The
  connection keeps its error on record though, and `transport_error` in
  `source/open.rs` reads it. `VideoSource::open` clears the record, and every
  reader asks after every packet. A reader that finds an error throws its
  demuxer away, whatever it still reads.
- Each reader has its own connection and its own way back, and logs a cut
  with its name, the seconds and the byte it had read to. The picture opens
  again as above. The sound decoder fails for good, the player opens a fresh
  one at once, at the position the old sound still gives, and it starts on
  the exact sample. The read of a whole subtitle track opens again after a
  second and goes on after the last packet it took, until it got no further
  for 30 seconds. The read back of a seek and a subtitle file only report
  the error.
- A video that failed plays again, `hilen/src/video/player/restart.rs`. The
  decode thread ends when it gives up. `play` on a failed video starts a new
  one at the position the video stood at, with the tracks and the speed it
  had, and a fresh sound. The state is `Buffering` until frames come, or
  `Loading` when none was ever shown. A source that fails again makes the
  video `Failed` again, with a new `on_error`.
- Late pictures that were dropped are logged as 1 line per 5 seconds, with
  how many that window lost, never a line per picture.
- After a seek the position is the seek target until the sound has reached
  it. kira takes the seek on its own thread, and before that its position
  is the old place, which would run the picture through its frames.
- Render on demand keeps the loop awake through an empty animation while a
  video plays, the way `AnimatedImage` does, so a paused video costs nothing.

## On iOS

The same player as on desktop, with 3 things a phone adds.

- The audio session. With the session iOS gives an app the silent switch mutes all
  sound. The first video that starts a sound moves the session to playback, the last
  such video that goes puts the default back, `hilen/src/video/audio_session.rs`.
- The first sound of an app opens the audio device, about a second on the simulator.
  The decode thread opens it before the video reports loaded, so no play holds the
  main thread for it. `Video slow sound` pins that.
- The app has to link VideoToolbox, CoreMedia, CoreVideo, MediaPlayer and zlib. A
  static library cannot ask for them, so every xcodebuild call of the build scripts
  passes them, `build/shared/src/ios.rs`, and the `hilen-mobile` template carries
  them too.

A player turns the screen with `Window::set_orientations` and hides the status bar
and the home indicator with `Window::set_fullscreen`. `MediaSession` reports the film
to the control center and takes play, pause and seek from it and from headphones.
On the simulator h264 decodes in software and HEVC in hardware. Nothing ran on a
real iPhone yet, see [roadmap.md](roadmap.md).

## In a browser

A browser cannot link ffmpeg. `video/web_player.rs` is the `Player` there, with the same
calls, so `VideoView` is one view on both.

- The picture never passes through the engine. A `<video>` element sits behind the
  canvas, `position: fixed` with `z-index: -1`, under the frame of the view. The view
  sets `page_hole` on itself and the drawer erases the frame there: everything queued so
  far is flushed, then `UIHolePipeline` draws the rect shader with a blend that writes
  zeros, color and alpha, `hole_barrier` in `ui_drawer.rs`. What comes later in the tree
  draws over the hole as usual. This is how a TV plays 4K and HDR, it decodes in
  hardware straight to the screen. Copying each frame into a texture was not tried.
- The canvas needs alpha. A WebGL canvas has it by default. A WebGPU canvas is asked
  for `CompositeAlphaMode::PreMultiplied`, `web_formats::resolve` in `window/state.rs`.
- The page must not cover the element. With a background on `html` the one of `body` is
  drawn over an element with a negative `z-index`. Give only `body` a background.
- What the hole costs: the video has no opacity, no rounded corners of its own content
  and nothing drawn under it, and a UI test cannot read its pixels.
- The element is asked for its state once per frame, no listener is installed. A view
  that is hidden gets no `update`, so `hide_unplaced` runs after the update pass and
  hides every element whose view was not placed in that frame.
- A stream that breaks is opened again by the browser, not by the engine. An element
  that reports an error keeps it for good, so `play` on a failed video calls `load` on
  the element, which starts at 0, and sets the old position once the length is known.
  No test covers this yet.
- A video element sends no request headers. A `VideoSource` with headers logs a warning
  and plays without them, the url has to carry the access.
- The tracks inside a file are not listed, a video element hands out neither the sound
  tracks nor the subtitle tracks of an mkv, and it plays the first sound track. The app
  gets subtitles from its server as a file: `set_subtitle_file` loads it with the
  headers of its source and `video/cues.rs` reads `SubRip`, `WebVTT` and ASS.
- A browser starts a silent video with no click on the page, a sounding one only after
  the user touched the page. Volume 0 mutes the element.
- `VideoStats` come from `getVideoPlaybackQuality`. The decoder name is `browser`.
- A browser build with `video` also links kira, the feature turns `audio` on.

`Web video` is the test, wasm only, in the browser lane: the element sits exactly under
the view, plays, seeks, ends, reports subtitle lines and leaves the page with a hidden
view. The cue parser has unit tests.

## The API

`set_source(path, url or VideoSource)`, `play`, `pause`, `is_playing`, `is_loaded`,
`seek_to(seconds)`, `duration`, `position`, `set_volume(0..1)`, `set_loop`,
`set_mode(ImageMode)`, `stats() -> VideoStats`, `state() -> VideoState` and the
events `on_finish`, `on_error` and `on_state`. `VideoState` is `Empty`,
`Loading`, `Paused`, `Playing`, `Buffering`, `Finished` or `Failed`, and
`on_state` fires on every change. `is_playing` stays true while buffering.
`play` on a `Failed` video opens the source again and goes on from the
position it failed at, on desktop, on iOS and in a browser.
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

## Pictures, pieces and export

3 things an editor needs, on the systems ffmpeg decodes on. A browser has none
of them. All 3 read a file through `Reader`, `hilen/src/video/decoder/reader.rs`:
one file, picture by picture, no queue and no clock, a seek exact to the frame
by the rule a seek of the player has.

- Pictures with no view, `hilen/src/video/frames.rs`. `VideoFrames::load(source,
  times, max_size, each)` reads on a thread of its own and reports on the main
  thread: `Info` with the length, the size, the frame rate and whether there is
  sound, then a `Frame` with an `Image` per place in the order of time, then
  `Finished`, or `Failed`. With no times it gives only the `Info`. The handle
  it returns has `cancel`. 1 open decoder serves the whole batch, a place a
  little ahead is reached by decoding on. An image stays in memory under the
  name of its source, its place and the size asked for, so asking again decodes
  nothing. `VideoFrames::open`, `info`, `picture` and `pictures` are the same
  work as blocking calls with RGBA pixels, for a thread of the app. The colors
  are computed with the numbers of `nv12.wgsl`. HDR is not tone mapped there.
- A list of pieces as 1 video. `VideoView::set_pieces([VideoPiece::new(source,
  start, end), ..])` plays the pieces one after another, `seek_to`, `position`
  and `duration` count along the whole list. The frame at `end` is not part of
  a piece. The list travels inside a `VideoSource`, `source/pieces.rs`, so the
  player is the same. Its decode thread is `decoder/pieces.rs`: it gives every
  frame its time in the list, and it opens the next piece and decodes up to its
  first frame while the queue is full. A piece that goes on in the same file
  up to a second ahead is read on with no seek. The sound is 1 kira decoder
  over all pieces, `audio/pieces.rs`, at 48000 Hz, so a cut has no gap. A piece
  with no sound track is silence, a list with no sound track at all has no
  sound and follows the engine clock. `set_pieces` on a view that plays a list
  keeps the position and the picture, the state does not go through loading.
- Export, `hilen/src/video/export.rs`. `VideoExport::start(pieces, path,
  VideoExportSettings::new(width, height, frame_rate), each)` writes an mp4
  with h264 and AAC on a thread of its own and reports `Progress`, then
  `Finished`, `Cancelled` or `Failed` on the main thread. The handle has
  `progress` and `cancel`. A file that is not complete is deleted. It reads the
  list with the code of the player, so the file holds the frames and the
  samples of the preview. The file has its own frame rate, every frame of it
  takes the newest picture that is due. A piece of another shape is fitted in
  with black around it. The matrix and the range of the first picture are
  written into the file, the samples are not converted, and HDR is not tone
  mapped. The encoder is the first of `h264_videotoolbox`, `h264_mf`,
  `libx264` and `libopenh264` that the archive has and that opens. The macOS
  archive has `h264_videotoolbox`. The published Windows archive, revision 4,
  has no h264 encoder, so an export fails there until an archive built with
  `--enable-mediafoundation` is published, `build/ffmpeg-win/build.sh` has the
  flag.

The tests: the unit tests next to each file, on the fixtures `ramp.mp4`, 60
gray frames that get lighter with a sound that rises in a line, so a frame and
a sample tell their place, and `ramp_silent.mp4`, the same with no sound. The
UI tests `Video thumbnails`, `Video pieces`, which pins the frame on each side
of a cut under stepped time, `Video pieces sound` in real time and `Video
export to file`.

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

To build a new archive, on the host it is for, or on a Mac for iOS:

```bash
rust build/ffmpeg.rs                      # clones FFmpeg, configures, builds, dist/ffmpeg-<v>-<triple>.tar.gz
rust build/ffmpeg.rs aarch64-apple-ios    # cross build for an iPhone, x86_64-apple-ios for the simulator, aarch64-apple-ios-sim for the arm64 one
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
There is an archive for `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`,
`aarch64-apple-ios`, `x86_64-apple-ios` and `aarch64-apple-ios-sim`. The iOS ones are
cross built on a Mac against the iPhone SDKs, with the same set as the Mac archive, the
first 2 for iOS 12. The x86 simulator one has no assembly, it only runs the UI tests
under Rosetta and the x86 assembly would need nasm. The arm64 simulator one is for a hot
build, see [hot-reload.md](hot-reload.md), for iOS 14, no arm64 simulator is older. The
build needs `meson` and `ninja` for dav1d. Every target builds in a folder of its own,
`target/ffmpeg-build-<triple>`, and a cross build installs into
`target/ffmpeg-dist-<triple>`.

The Windows archive is the one that is not built on its own host.
`rust build/ffmpeg-win.rs` cross builds it in docker on any machine, with clang
in cl mode and the MSVC headers and libraries from
[xwin](https://github.com/Jake-Shadle/xwin). That is the same kind of build the
Windows release of an app is, so the archive links there, and it links with the
Microsoft linker on a real Windows too. It has D3D11VA, Schannel as its TLS,
zlib and dav1d, all with the static C runtime, and its `lib/link.txt` also
names the Windows libraries ffmpeg needs. The first build of the docker image
unpacks the SDK, which takes about 40 minutes.

On Windows the bindings need libclang at build time, `scoop install llvm` brings
it. On an Intel HD 630 h264 and HEVC, 10 bit included, decode on the GPU, AV1
decodes in dav1d. The first frame of every video logs its decoder and whether
it is the hardware one.

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

`Video stalled seek` plays `stalled.mkv`, the picture and 1 sound track of
`tracks.mkv` with the index at the front, from a server that stops sending
at 2.5 seconds, and seeks back while both demuxers wait. It pins that the
video plays again from the target. With the index at the end the first seek
of the sound reads the end of the file through the stall.

The unit tests in `player/restart.rs` play the same `stalled.mkv` from a
server that promises the whole file and closes the connection at 2.5
seconds, with no window, so the frames are not uploaded. They pin that a
stream cut once plays on to the end and is never `Failed`, that a server
that stays dead gives `Buffering` and then `Failed` after the limit, 1.5
seconds in the test, and that `play` after that fails again while the server
is dead and plays on from the same position once it is back. The server is
`video/test_server.rs`. It can also hold an answer in the middle of a block
and close it a little later, which is the cut the demuxer hides. 3 tests use
that: the player is paused, both connections are cut, and all 120 pictures
still come once and the sound gets a fresh decoder. The sound decoder alone
fails at the cut, it used to give 3.53 of 4 seconds and no error before the
end. The read through `no_index.mkv` gives all 3 lines, it used to lose the
one in the cluster of the cut.

`Video slow sound` plays `slow_sound.mkv` from a server that can stop
answering, and measures how long the first play after a seek, a speed
change, a sound track switch and a seek after the end keep the main thread
while it does. Each took the 2 seconds the test waits before the fix.

`Video tracks` plays `tracks.mkv`, 2 tone tracks and 2 subrip tracks, muted,
and walks the track lists, the lines in step with the clock, a sound track
switch that keeps the position, a subtitle switch, a seek, a seek into the
middle of a line that began before the keyframe, and a subtitle file. `Video hdr` shows a PQ, an HLG and a 10 bit SDR fixture of flat bars,
HEVC main 10 coded lossless, whose colors were computed from the standards'
formulas outside the engine. `Video speed` pins the position at double and
half speed under stepped time. `Video av1` plays an AV1 copy of the color
fixture and pins that dav1d decodes it. The unit tests next to the code pin
which sound track is heard, 440 against 1760 Hz, the length and the kept
pitch at every speed, the cues of a track and of a file, that the first seek
of a sound waits for the first decode and still starts on the right sample,
the 24 second line of `long_line.mkv` after a seek into it, the long line of
`overlap.mkv` under a shorter one, the whole track read through the index
and, for `no_index.mkv`, through the file, that a sound track that fails to
open leaves the old one named, and that dav1d and zlib are in the archive.

`Video playback` in `ui-test-suite/src/views/video` plays `colors.mp4`, four
solid frames at one per second, every frame a keyframe, no sound. Under
stepped time it pins each frame's color, the exact frame count between them,
`on_finish` after the last frame, a seek while paused landing on its frame
and a loop wrapping to the first. Under stepped time the player waits for the
decoder before a frame renders, real time never does.
