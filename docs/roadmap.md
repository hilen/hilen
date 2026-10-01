# Engine gaps

Open engine features still missing, found by porting real apps. Each entry lists the
current state in code, what is needed, and what it blocks. When one lands it needs UI
tests like any other engine change, then it moves out of this file. This roadmap holds
only open work, not a change log of what already shipped.

Engine capabilities only. Every entry must be a general feature of the `hilen` crate
that any app can use. Never an app-specific task, never a single app's dialog content,
screen, wiring or shipped asset. If an item only matters to one app, it belongs in that
app's own docs, not here. A gap an app found still qualifies only when the fix is a
reusable engine capability.

The driver apps are skaityk at `~/dev/apps/skaityk` (a reader for Lithuanian
learners), the beekeeper web UI in the `local` repo at `beekeeper/web`, and kukareker at
github.com/hilen/kukareker (a git client). Full visual and functional parity with each
original is the acceptance bar. Their ports drove the gaps below.

## Flixen, the media player gaps

Found by flixen at `~/dev/apps/flixen`, a self hosted media server and player.
Its first version is macOS only. These entries are the top priority, in the
order below. The first 10 block the first version, the rest block the full
product.

### Text subtitles in VideoView

- Current: the decode loop drops every packet that is not the video stream,
  `packet.stream() != decoding.stream` in `hilen/src/video/decoder.rs`. The
  view has no subtitle API. The prebuilt ffmpeg has the `srt` and `ass`
  decoders.
- Needed: the list of subtitle tracks of a source with language and title, a
  call to pick one or none, and an event that carries the text of the cue to
  show and clears it when it ends, in step with the video clock and with a
  seek. A way to load a subtitle file from outside the source, an `.srt` path
  or url. ASS styling is dropped, only the text goes out. Picture subtitles
  like PGS come later on the same track list.
- Blocks: any film watched with subtitles.

### Audio track choice in VideoView

- Current: the sound always opens `input.streams().best(media::Type::Audio)`,
  `hilen/src/video/audio.rs`. There is no track list and no way to switch.
- Needed: the list of sound tracks with language, title, codec and channel
  count, and a call to switch that keeps the position.
- Blocks: a film with more than 1 language.

### HDR and 10 bit video

- Current: a 10 bit frame comes back as P010 and goes through swscale to 8 bit
  NV12 with no tone mapping, `Decoding::scale` in `decoder.rs`. `nv12.wgsl`
  knows BT.709 and BT.601 only. The speed of this path is not measured,
  [video.md](video.md) measured h264 only.
- Needed: PQ and HLG with BT.2020 read from the stream and tone mapped to SDR
  in the convert pass, a P010 upload with no swscale, and a measured 4K HEVC
  10 bit file at full rate.
- Blocks: correct colors for every HDR film. They play washed out today.

### Video over https and with request headers

- Current: the prebuilt archive is configured with `--disable-autodetect` in
  `build/ffmpeg.rs`, so it has the `http` protocol and no `tls` or `https`
  one. `set_source` takes a path or url only, no ffmpeg options reach
  `format::input`, so a request carries no header.
- Needed: TLS in the archive, SecureTransport on macOS, and a source that
  takes request headers, so a session token does not go into the url.
- Blocks: a stream from a server behind https, and a stream behind a login.

### The video feature in an app outside this repo

- Current: `FFMPEG_DIR` comes from this repo's `.cargo/config.toml`, an app
  elsewhere must point its own at the engine checkout, see
  [video.md](video.md). Already named in the video lanes entry below.
- Needed: an app that turns `video` on builds with no extra setup, on a
  machine with no engine checkout.
- Blocks: the first build of any video app that is not `demo`.

### Buffering state in VideoView

- Current: the events are `on_finish` and `on_error`. Real time never waits
  for the decoder, so a stalled network read shows a frozen picture while the
  clock runs on.
- Needed: a state the app can read and an event for it, loading, playing,
  buffering, and the clock held while the queue is empty and the stream has
  not ended.
- Blocks: a spinner over a stalled stream, and sound that stays in step after
  a stall.

### Fullscreen window

- Current: `Window` has no fullscreen call, only the system button on the
  title bar.
- Needed: enter, leave and read fullscreen, with an event when it changes.
- Blocks: a player's fullscreen key and button.

### Hide the mouse pointer

- Current: `set_cursor_visible` is called only from `Cursor::capture`, which
  also locks the pointer, `hilen/src/ui/input/cursor.rs`.
- Needed: hide and show the pointer with no capture.
- Blocks: a player that hides the pointer after a few still seconds.

### Screen awake on desktop

- Current: `ScreenAwake` keeps the display on for mobile only, its doc says
  other targets keep their normal policy, `hilen/src/system/screen_awake.rs`.
- Needed: the same guard on macOS, Windows and Linux.
- Blocks: a film longer than the display sleep time.

### SQLite in hilen-server

- Current: `build_db` returns a `PgPool` and sqlx has the `postgres` feature
  only. `Config::from_env` fails with no `DATABASE_URL` and no `REDIS_URL`.
  The `auth` module and its migrations take a `PgPool`.
- Needed: a SQLite pool next to the Postgres one, Redis optional in `Config`,
  and the Google login tables and queries on SQLite too.
- Blocks: a backend that ships as 1 binary with 1 data file and no Docker.

### Image downloads with headers, a memory bound and a disk cache

- Current: `Image::download` takes a name and a url, sends no header, keeps
  nothing on disk, and the image stays in the store until `free` is called
  by hand, `hilen/src/deps/refs/manage/data_manager.rs`.
- Needed: request headers for a download, a disk cache so a second launch
  does not fetch again, and a bound on the memory the downloaded images hold.
- Blocks: a grid of thousands of posters from a server behind a login.

### Multiline label with an ellipsis

- Current: `set_ellipsize` works on a single line label, its doc says
  multiline labels wrap and ignore it, `hilen/src/ui/views/basic/label.rs`.
- Needed: a line limit on a multiline label with the ellipsis on the last
  line.
- Blocks: a description cut to 3 lines.

### View opacity

- Current: a view has a color with alpha, there is no setter that fades a
  view together with its subviews.
- Needed: an opacity per view that covers its whole subtree, usable in a
  `UIAnimation`.
- Blocks: player controls that fade in and out.

### Text outline and shadow

- Current: `set_shadow` draws under the view's shape, a `Label` has no
  outline and no shadow on its glyphs.
- Needed: an outline or a soft shadow on label text.
- Blocks: subtitles that stay readable over a bright picture.

### Playback speed in VideoView

- Current: no API, the clock is the sound position at its own rate.
- Needed: a rate for picture and sound, with the pitch kept.
- Blocks: watching at 1.5 times.

### Media keys and Now Playing

- Current: nothing in the engine reads the play, pause and next keys or
  reports to the system's Now Playing panel.
- Needed: both, on macOS first.
- Blocks: the keyboard media keys and the control center panel.

### More in the ffmpeg archive

- Current: no zlib and no software AV1 decoder, AV1 decodes only where
  VideoToolbox has it. Sound is always resampled to stereo.
- Needed: zlib, dav1d, and the sound in its own channel layout when the
  output device has more than 2 channels.
- Blocks: files with compressed headers, AV1 on older Macs, surround sound.

## Siri Remote input for tvOS

Found by the tvOS display bring-up, see [tvos.md](tvos.md). Waits for a real
tvOS app need.

- Current: the engine builds for tvOS and renders in the Apple TV simulator, but the
  UI is touch driven through `WindowEvent::Touch` and Apple TV has no touch screen.
  The app draws and nothing can drive it.
- Needed: a remote input path. Siri Remote events arrive through the UIKit focus
  engine and `UIPress`, and winit's UIKit backend forwards direct touches only, so
  the winit fork needs press and focus forwarding, and the engine needs to map that
  onto its views, most likely a focus model over the existing key and touch events.
- Blocks: any interactive tvOS app, and the tvOS UI test lane, since what a test can
  assert depends on this path.

## 3D scene, remaining deliveries

The `scene` module landed with primitives, physics, the Filament mobile PBR
materials, a sun with point and spot lights, textures and normal maps, a sky
with image based lighting, transparency, a tonemap, a first person player,
`.glb` models with skins and animation clips, cascaded sun shadows with a shadow
distance, distance fog and touch picking, see [scene.md](scene.md). The rest was
planned with it.

- An embeddable `SceneView` that composites into any view frame instead of the
  root area, and the offscreen linear pass with real HDR that `colors.md`
  sketches, if a scene ever needs it.
- The shadow passes draw every opaque node into every cascade, the GPU clips
  what lies outside a map. Culling nodes against each cascade's box on the CPU
  would make a short shadow distance cut the passes' cost on a big level.

## 3D scene on WebGL2

Found by opening the demo with `?hilen_webgl` in the page query. The UI pipelines
read their instances from a uniform array on WebGL2, `InstanceBinding` in
`render/uniform`, the scene does not.

- Current: `MeshPipeline` binds the instances, the joint matrices and the lights
  as read only storage buffers with no uniform fallback, and WebGL2 has none. On
  the forced uniform path, `HILEN_UNIFORM_INSTANCES=1`, the first scene frame
  fails in `create_bind_group` for `mesh_lights_bind`. The demo never gets that
  far: the sprite pipelines of `level` go through `instances_shader`, whose
  rewrite expects a storage instance array that `sprite.wgsl` does not declare,
  so the demo panics at startup with `a UI shader declares its instances as a
  storage array`. The web lane runs the scene tests on WebGPU alone, so
  nothing pins either.
- Needed: `instances_shader` leaves a source without the declaration untouched,
  pinned by a level test on the forced uniform path. Then the scene follows the
  UI: the lights become a fixed uniform array, the opaque batches draw chunk by
  chunk through `InstanceChunks` with the `index` attribute chunk relative, and
  the joints bind a uniform window per batch, 256 matrices, split when the
  skinned nodes overflow it, shared with the shadow pass. Check first that naga's
  GLSL output takes `textureLoad` on the depth array shadow map, else that path
  needs a comparison sampler or a color render of the depth. Then `scene-test` on
  `HILEN_UNIFORM_INSTANCES=1` in `make ci`, and a `?hilen_webgl` run in the web
  lane.
- Blocks: any 3D page in a browser without WebGPU: an iPhone below iOS 26, a page
  on plain http over a LAN address, WebKit handing out no adapter, a blocklisted
  GPU.

## Scene tests on the browser lane

The scene tests live in `scene-test-suite`, `demo` links it, and the device
autorun runs them after the UI tests, so `make ui-ios` and `make ui-web` cover
them. Nine of them are gated off a lane they fail on, so the lanes run green
while the causes stay open. Each gate points here, and every fix removes its
gate.

- Current: `Animations`, `Cascades`, `Shadows` and `Cutout shadows` are off the
  browser lane. They pass on the iOS simulator and in a real Chrome, and under
  SwiftShader, the software WebGPU the CI browser lane renders with, their
  shadow edges land a few pixels off, the depth precision of the shadow map is
  the lead. `Colliders` and `Collider shapes` are off the browser lane too,
  SwiftShader draws the green collider lines in another shade, `#00ff60` where
  a GPU blends `#1fd153`, the lines are not antialiased there. `Drop balls`
  and `Player walk` are off the browser lane as well. Their physics lands on
  the same bits on desktop, in Chrome and on the simulator now, see
  [scene.md](scene.md), and both pass in a real Chrome. `Drop balls` still
  fails its picture on SwiftShader, the cause is not looked at, and `Player
  walk` has not finished a run there. `Mouse look` is
  desktop only. `Cursor::capture` does nothing on a phone, and a browser grants
  pointer lock only after a real click, which no test can inject, so the
  capture is released at once. The UI test `Cursor capture` is desktop only for
  the same reason. Chromium on SwiftShader runs on a Mac in docker: an image
  with `chromium`, `xvfb`, `xauth` and `socat`, the driver started with
  `--browser none`, `socat` forwarding the container's port 44810 to the host
  so the page stays on `localhost`, and the Linux flags of `launchChrome`.
  `Cutout shadows` passed there on arm64, the CI runner is x86_64, so its gate
  stays until CI ran it.
- Needed: for `Drop balls` and `Player walk`, a read of the SwiftShader failure
  picture against the desktop one. For the shadows, a bias or a depth format that reads the same on
  SwiftShader. For the collider lines, probes off the lines or lines that cover
  the same pixels without multisampling. For the mouse, a real click from the
  driver, through the debug protocol of Chrome and its twin in Firefox, else
  those two tests stay desktop only.
- Blocks: the gated tests on `make ui-web`, and `Mouse look` on `make ui-ios`.

## Video playback, the other lanes

Desktop macOS landed, see [video.md](video.md): `VideoView` behind the `video`
feature, ffmpeg from prebuilt static archives, VideoToolbox decode, kira for
the sound and as the clock, a 1080p60 and a 4K30 file play at full rate with
sound. The rest of the platforms and the packaging are open.

- Current: the archive exists for `aarch64-apple-darwin` only. `demo` and
  `ui-test` turn the feature on through a macOS target table. The engine
  declares VAAPI for Linux and D3D11VA for Windows, neither has an archive
  nor a CI lane. A decoded 4K
  frame is copied through system memory, 12 MB a frame, and plays at rate.
  Each `VideoView` keeps one frame image per source size for good, the
  managed image store never frees.
- Needed: an archive per desktop triple from `build/ffmpeg.rs`, Linux needs
  `libva-dev` and `nasm` in `build/setup.sh` and in the docker containers of
  the Linux CI job, Windows needs the ffmpeg configure under an MSYS2 shell,
  then the target tables widen to `desktop`. Apps outside this repo need an
  `[env] FFMPEG_DIR` recipe pointing at the engine checkout.
  iOS and Android need archives cross built per target with VideoToolbox and
  MediaCodec, and the feature unlocked there. The browser cannot link
  ffmpeg, it hands the stream to an HTML5 video element and imports each
  frame with `copyExternalImageToTexture`, the one place the `VideoView`
  backend differs. Zero copy on macOS, a `CVPixelBuffer` into a Metal texture
  through the wgpu hal, only after an A/B shows the copy costs frames.
  Subtitles and track switching sit on top of the decode thread.
- Blocks: video on any platform but macOS, and a shipped app on macOS until
  the packaging question of a static archive per triple is settled.

## Leftovers inside landed features

Small remainders not worth their own entry.

- Tab focus traversal does not scroll. Tab selects the next text field even when
  it sits scrolled out of view inside a `ScrollView`, so the editing session
  starts off screen. Needs a scroll-to-view step in `select_next_field`, the way
  a multiline field already follows its caret line while typing.
- Frame stepped time covers `Animation` and `AnimatedImage` only. `RingSpinner`,
  the text field caret blink and double click, tooltip and long press delays still
  read `Instant`, so they drift under stepped time and their mid flight frames
  cannot be pinned. Each is a one line move to `Clock::now_ms` plus a test.
- Sideways scrolling pins nothing in a plain `ScrollView`, its whole content
  moves. Only a `TableView` cell keeps parts in place, through
  `set_moves_sideways`.
- The same text frame still differs by 1 color level between 2 runs of the
  program after the glyph snap, see [text.md](text.md). The cause is not found.
- A scene picture, `SceneManager::picture`, is proven on desktop Metal only. The
  iOS simulator and the browser lanes have not run its 2 scene tests, and
  WebGL2 draws no scene at all yet.
- The alpha cutout of a scene material is proven on desktop Metal only. The iOS
  simulator and the browser lanes have not run `Cutout` and `Cutout shadows`. A
  `.glb` material's `doubleSided` is not read, back faces are always culled, so a
  leaf card is seen from one side. The cut edge is hard, alpha to coverage would
  smooth it under MSAA.
- Human mode has no frame step key. A stepped test pauses only on checks, an
  animation in flight cannot be walked one frame per key press with the frame
  number in the window title.
- An animation never writes its exact end value, the last commit lands just before
  expiry. With a fixed step the finishing frame could clamp to the end value, but
  that changes visible behavior and recorded expectations, so it is its own gated
  decision.

- A COLR sweep gradient draws through tiny-skia's sweep shader with the
  angles taken as degrees counter clockwise from the font's x axis, the
  COLR convention is not pinned by any test font. A color glyph ignores the
  text color's alpha, a translucent emoji needs an opacity in the image
  instance.
- SVG premultiplied upload: the convert from tiny-skia's premultiplied pixels to
  straight alpha is a per pixel loop, optimized in `hilen-pixels` but still work.
  Uploading premultiplied and blending svg textures premultiplied in the image
  pipeline would remove that loop, it needs a flag per image instance and a blend
  change in the shader.
- DrawingView paths: texture fills for paths, more than 8 gradient stops, and soft
  edges on arbitrary path outlines, a radial alpha ramp only covers circular glows.
- A rect edge on a fractional pixel row blends differently per GPU under MSAA
  4x. The SDF alpha and the hardware sample coverage combine, and the 4x
  sample pattern is the GPU's own, SwiftShader in the CI browser lane reads
  such a row 30 levels off a Mac. Text underlines snap to whole rows now,
  anything else laid out at a fraction still varies, so a probe on such a row
  holds on one GPU only. A sample mask of all ones in the SDF pipelines would
  make the alpha the only coverage, it moves every fractional edge on every
  platform, so it needs a re-record of the suite.
