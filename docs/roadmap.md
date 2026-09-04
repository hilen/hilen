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

## Bug reporting and Sentry on wasm

Found by bringing karkas style bug reporting into the engine. Desktop, iOS and
Android landed, the browser did not.

- Current: `App::sentry_url` and `BugReport` are native only. The sentry crate
  does not run on wasm, so `setup_sentry` is gated `not_wasm` and `BugReport::open`
  is a browser no-op like `system::Router`.
- Needed: hand build the Sentry envelope, the event JSON plus attachment items,
  and POST it to the DSN's envelope endpoint through `netrun`. The report dialog
  and the rings are engine views and plain state, only the transport is missing.
- Blocks: bug reports and crash events from web apps.

## Pause continuous work while the window is not visible

- Current: a live animation or a loaded level keeps the loop polling even
  when the window is minimized, fully covered, or on another desktop. A
  browser tab throttles `requestAnimationFrame` on its own, native does not.
- Needed: listen to winit's occluded event in the app handler and hold the
  loop in `Wait` while the window is occluded, then request a frame when it
  shows again. Animations keep their clock, so a long hold still lands them
  at the end state on the first frame back.
- Blocks: nothing visible, only CPU and battery in a background window.

## Button cannot measure its title

Found by the studio port, the admin page filter tabs.

- Current: `Label::content_size` and `size_for_width` measure the text.
  `Button` has no text measurement, its `content_size` comes from
  `ViewCallbacks` and reflects the frame, near zero before layout, so a
  button cannot be sized to its title. The studio admin page works around
  it with touch enabled `Label`s styled as tabs.
- Needed: title measurement on `Button`, or one measuring API shared by
  every text carrying view, so `content_size` means the same thing on
  `Label` and `Button`.
- Blocks: any row of buttons sized by their titles, filter tabs, toolbars,
  segmented controls.

## Text stack rework

Found by the FontZoo emoji page. Parked until a real need for color emoji or
font fallback.

- Current: shaping goes through rustybuzz via `ShapedLayout`, so GPOS kerning,
  GSUB and variable font axes apply. But rasterization still goes through wgpu_text,
  glyph_brush and ab_glyph, outline glyphs only, a single channel atlas tinted by
  the text color. Color emoji tables, CBDT, sbix and COLR, are ignored, so emoji
  render monochrome via the bundled `NotoEmoji.ttf`. No font fallback chains.
  See [text.md](text.md).
- Needed: migrate label rendering to cosmic-text with swash. Brings font fallback
  chains and color emoji in every format. Large: replaces the glyph atlas and
  `draw_label`, and invalidates every recorded text expectation in the UI tests.
- Blocks: colorful emoji. Nothing in the driver apps today.

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

## Scene tests on the device lanes, work in progress

The scene tests live in `scene-test-suite`, `demo` links it, and the device
autorun runs them after the UI tests, so `make ui-ios` and `make ui-web` cover
them. Six of them are gated off the lanes they fail on, so the lanes run green
while the causes stay open. Each gate points here, and every fix removes its
gate.

- Current: `Mouse look` is desktop only. `Cursor::capture` does nothing on a
  phone, and a browser grants pointer lock only after a real click, which no
  test can inject, so the capture is released at once. The UI test
  `Cursor capture` is desktop only for the same reason. `Drop balls`,
  `Colliders` and `Player walk` are desktop only. They pin a physics rest that
  lands a little off on the x86_64 iPhone simulator and in the browser lanes.
  `Colliders` passes in a real Chrome on this Mac and fails under the
  SwiftShader frame pacing of CI, so the number of physics steps between two
  waits is the lead. rapier's `enhanced-determinism`, on for `scene-tests`,
  changed nothing, the failing probes read byte identical colors with and
  without it, so it is in without proof. `Animations`, `Cascades` and
  `Shadows` are off the browser lane only. They pass on the iOS simulator and
  in a real Chrome, and under SwiftShader, the software WebGPU the CI browser
  lane renders with, their shadow edges land a few pixels off, the depth
  precision of the shadow map is the lead.
- Needed: for the rests, an A/B of the two candidates: rapier's `parallel`
  feature ordering float sums by thread, and a slow lane pacing a different
  number of physics steps between two waits, then a fixed step count per wait
  or the stepped clock for the scene, and keep or drop `enhanced-determinism`
  on what the A/B shows. For the shadows, a bias or a depth format that reads
  the same on SwiftShader. For the mouse, a way to grant pointer lock to a
  test page, else those two tests stay desktop only.
- Blocks: the gated tests on `make ui-ios` and `make ui-web`.

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
- Human mode has no frame step key. A stepped test pauses only on checks, an
  animation in flight cannot be walked one frame per key press with the frame
  number in the window title.
- An animation never writes its exact end value, the last commit lands just before
  expiry. With a fixed step the finishing frame could clamp to the end value, but
  that changes visible behavior and recorded expectations, so it is its own gated
  decision.

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
