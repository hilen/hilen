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

## Screen keyboard, the proof in banda and on Android

Found by banda at `~/dev/apps/banda`, whose compose box sits at the bottom of the
screen. The keyboard frame, the field that stays in view, `b_keyboard` and the focus
by code that opens no keyboard landed, see [screen-keyboard.md](screen-keyboard.md).

- Current: proven by 4 UI tests on desktop, with a stand in for the keyboard, and on
  the iOS simulator lane with the real one. banda has the pin `da1b9180` and its
  compose box is placed with `b_keyboard`. Its chat was tried on the iPhone 14
  simulator, iOS 16.4, with the engine that makes the app views the safe area: the
  compose box sits above the home bar, a tap on its field opens the keyboard, and the
  box rides on the keyboard with its 16 points of gap while the list above gets
  shorter. No real iPhone ran it, and the pin of banda is still older than the safe
  area fix. The move runs on an ease curve close to the one of iOS, not measured
  against the real keyboard. Android opens no screen keyboard at all, a tap on a
  `TextField` there brings nothing up, so there is nothing to follow yet.
- Needed: the pin of banda moved to an engine with the safe area fix, and the chat
  tried on a real iPhone, with the 3 `compose.focus()` calls of
  `src/ui/session_page/mod.rs` left as they are. A screen keyboard on Android, opened
  by an edited field, with its frame passed to `ScreenKeyboard::moves_to`.
- Blocks: calling the chat of banda done on a phone, and every text field on Android.

## The wheel delta does not tell which way the user turned the wheel

Found by flixen at `~/dev/apps/flixen`. Its player turns the volume with the wheel, up
is louder.

- Current: `UIEvents::on_scroll` gives the delta the system reports, `scroll_pixels` in
  `hilen/src/window/app_handler.rs`. On a Mac with natural scrolling on, the default,
  that delta is already turned around, so a wheel turned up gives a negative `y`. That
  is right for content that scrolls and wrong for a value like volume or zoom. The
  window code of `hilen-winit` reads `scrollingDeltaY` in `scroll_wheel` of
  `platform_impl/macos/view.rs` and never reads `isDirectionInvertedFromDevice`, so no
  app can tell. flixen turns the sign around on every Mac, which is wrong again with
  natural scrolling switched off.
- Needed: the wheel event also carries the turn of the device itself, or a flag that
  the system turned it around. On a Mac from `isDirectionInvertedFromDevice`, on the
  other platforms from their own setting where the system reports the turned value. A
  test for the sign with the flag on and off.
- Blocks: the right volume direction in the flixen player on a Mac with natural
  scrolling off, and the `cfg!(target_os = "macos")` sign turn in `wheel_volume` of
  `crates/flixen/src/ui/player_screen.rs`, which goes away then.

## A paragraph of only bold text before a list is not drawn by MarkdownView

Found by banda at `~/dev/apps/banda`. Its chat draws what Claude wrote with
`MarkdownView::set_text`, and Claude often writes a bold line as a small header right
above a list.

- Current: a text like `**Done**` on 1 line with a list on the next line, `- first`,
  shows the list and an empty gap where the bold line should be. The text is there: a
  selection over that place copies `Done`, so the block is parsed and laid out, and
  only its drawing is missing. A bold word inside a longer paragraph is drawn. The
  cause is not found yet, prove it before a fix. Seen on hilen `dc3d50d5`, and before
  the text selection landed, so the selection did not bring it.
- Needed: the bold paragraph is drawn like any other paragraph. A UI test with a bold
  only paragraph right above a list, above a paragraph and at the end of a text.
- Blocks: the small headers of an answer in the banda chat.

## A key combo with Alt, and a combo that wins over the focus ring and a text field

Found by banda at `~/dev/apps/banda`. It jumps between its sessions with Cmd and an
arrow on a Mac, and has to do the same with Alt and an arrow on Windows and Linux.

- Current: `KeyCombo` in `hilen/src/ui/input/keymap/key_combo.rs` has `cmd` and
  `shift` only, and `cmd` is Cmd or Ctrl, `command_held` in `keymap.rs` takes both. No
  binding can ask for Alt. `Input::on_key` in `hilen/src/ui/input/input.rs` hands an
  arrow to `Focus::on_key` first, which takes it with any modifier held while the ring
  is active, so a combo with an arrow reaches the keymap only while a text field has
  the keys. That field then gets the same press through `keyboard_key` and also moves
  its caret.
- Needed: `KeyCombo::alt(key)`, and a way to bind 1 action to Cmd on a Mac and Alt
  elsewhere. A press that matches a bound combo with a modifier goes to that binding
  only, not to the focus ring and not to the focused text field. A plain arrow stays
  with the ring and the field. A UI test for both.
- Blocks: the session switch keys of banda on Windows and Linux.

## An image from the clipboard on Android, in a browser and on a real phone

Found by banda at `~/dev/apps/banda`. Its compose box takes a screenshot from the
clipboard and sends it with a prompt. `Clipboard::get_image` and the `image_pasted`
event of `TextField` landed, see [clipboard.md](clipboard.md).

- Current: proven on macOS with a real screenshot in banda, and by unit tests and the
  UI test `Text field image paste` on the store of the process. Windows and Linux use
  the same `arboard` code and were not run, Linux only under X11. iOS reads
  `UIPasteboard.image`, it compiles and never ran, and a paste there does not fire
  the event, the system text field pastes by itself. Android and the browser answer
  none.
- Needed: a picture from the clipboard on Android, through a `ContentResolver`, and
  in a browser, where the read answers later after a permission prompt. A paste on a
  phone that fires `image_pasted`. A run on Windows, on Linux and on an iPhone.
- Blocks: pictures in the prompts of banda on a phone and in a browser.

## File browser, the proof in an app and on the other lanes

Found by banda at `~/dev/apps/banda`. Its New session page picks the folder a session
starts in, on another machine. `FileBrowser`, `FilePicker` and the `FileSource` trait
landed, see [file-browser.md](file-browser.md).

- Current: proven by 14 UI tests on desktop, on a tree held in memory, and by unit
  tests of `LocalFiles` on a temp folder. banda uses `FilePicker` with its own source
  for the folders of another machine, run on macOS against a Linux machine. The tests did not
  run on the iOS simulator lane or the browser lane, 3 of them type through
  `system_input`. `LocalFiles` was not run on Windows, where the roots are the drive
  letters and a hidden file is an attribute, nor on a phone. The view has no drag and
  drop, no copy and paste of files, no preview, and no style call for an app with its
  own palette. A delete on `LocalFiles` is for good, nothing goes to a trash.
- Needed: the 14 tests green on `make ui-ios` and `make ui-web`. A run of a
  `LocalFiles` browser on Windows. A style call, so the picker takes the palette of
  the app, the same for `ContextMenu`, whose colors are fixed in
  `hilen/src/ui/views/controls/context_menu.rs`.
- Blocks: a folder picker and a right click menu in the colors of banda.

## Google access, the proof on a phone, in a browser and with a linked account

Found by lendar at `~/dev/apps/lendar`, a calendar that reads and writes Google
Calendar for several Google accounts at once. The feature landed, see
[google-access.md](google-access.md).

- Current: proven on a Mac debug build against the live backend of lendar. The main
  account signed in, the Drive app folder was read and the calendars synced. No
  second account was linked against the real Google yet, the 2 device tests run
  against a stand in for the Drive folder. The secure store ran only on a Mac, for
  Windows, Linux and Android `hilen-session` was only compiled. Nothing ran on iOS,
  on Android or in a browser, and no signed release build read the Keychain yet.
- Needed: a real link of a second account and its return on a second device. The
  sign in on an iPhone, on Android and in a browser. A signed Mac release that
  reads the Keychain with no dialog, and a run of the store on Windows and Linux.
- Blocks: lendar on iOS, Android and in the browser.

## Canvas and pinch, the proof on real input

Found by Lan Atlas, a network map app with a canvas that pans and zooms. `CanvasView`,
the scale of a subtree and the pinch event landed, see [canvas.md](canvas.md).

- Current: proven by the UI tests `Canvas zoom` and `Canvas pinch` on desktop, with
  injected touches only. No real trackpad and no real finger made a pinch yet. The
  browser path, a wheel turn with Ctrl held, was never built or run, and the 2 tests
  did not run on the iOS simulator lane or the browser lane. Windows and Linux get no
  trackpad pinch, winit reports none there. A view that draws by itself, a
  `VideoView`, does not scale inside a canvas. A `ScrollView` inside a zoomed canvas
  drags at the speed of the screen, not of its content. After a pinch ends with 1
  finger still down, that finger pans only after it lifts and touches again. The
  touch recorder prints finger touches and no trackpad pinch.
- Needed: the Lan Atlas map on `CanvasView`, a pinch on a Mac trackpad, on a phone and
  in a browser, and the 2 tests green on `make ui-ios` and `make ui-web`. Then the
  small points above, each with its test.
- Blocks: calling the map screen of Lan Atlas done on a phone and in a browser.

## Video on iOS, the proof on a phone

Found by flixen at `~/dev/apps/flixen`, a self hosted media server and player.
The `video` feature builds and plays on iOS now, see [video.md](video.md), and
its UI tests pass on the simulator. No real iPhone ran it yet.

- Current: proven on the iOS 16.4 simulator only, where h264 decodes in
  software and HEVC in hardware. The audio session, `Window::set_orientations`,
  the phone side of `Window::set_fullscreen` and `MediaSession` on iOS are
  written and compile for a device. On Android `set_orientations` only keeps
  its value. A film stops when the phone locks, an app has no background
  audio mode.
- Needed: a film played on a tethered iPhone, with hardware decode named in
  the first frame log line, sound with the silent switch on, the screen turned
  by a player, and play, pause and seek from the control center and from
  headphones. `UIBackgroundModes` with `audio` in the generated `Info.plist`,
  as a knob of `hilen.toml`, so the sound goes on behind the lock screen and
  the lock screen shows the film. The orientation call on Android, through
  `setRequestedOrientation`.
- Blocks: calling flixen on an iPhone and an iPad done.

## Flixen, the media player gaps

Found by flixen at `~/dev/apps/flixen`, a self hosted media server and player.
Its first version is macOS only. The gaps landed in 8 batches: tracks and
subtitles, HDR, fullscreen, pointer hide and screen awake, SQLite in
`hilen-server`, view opacity with the group fade, the line limit and text
outline, image downloads, playback speed, Now Playing and the media keys, zlib
and dav1d, the whole track subtitle read. See
[video.md](video.md), [text.md](text.md) and [login.md](login.md). What is left:

### What the flixen batches left open

- AV1 always decodes in software through dav1d, also on a Mac whose hardware
  has an AV1 decoder, ffmpeg lists dav1d first. Picking the hardware decoder
  where VideoToolbox has one needs such a Mac to prove it, an M3 or newer.
- Now Playing and the media keys are macOS and iOS only. Windows needs the System
  Media Transport Controls. Raise this point only when the roadmap is read
  on Windows.
- Linux needs MPRIS for the same. Raise this point only when the roadmap is
  read on Linux.
- The outline and the soft shadow of a label are images made on the CPU
  now, see [text.md](text.md). The effect entry points of the `wgpu_text`
  fork, `vs_effect` and `fs_effect`, are unused and can leave the fork. They
  were never run on an A7 device.
- A film with no index of its subtitle packets is read through once when a
  subtitle track is picked, over http that downloads the whole file. Until
  that read is done a seek finds only a line that began in the 10 seconds
  before its target.
- The data folder on Android is made and set by the engine, the code
  compiles in the docker build and has not run on a device or an emulator.
- A group fade draws into an image of the frame size with its own depth and
  multisample target, made on first use and kept. A target of the size of
  the group would cost less memory.

## LG webOS TV, the run on a TV

`make webos`, the key focus, browser storage, video in a browser and the login from a
phone landed, see [webos.md](webos.md). None of it ran on a TV yet.

- Current: proven on desktop and in Chrome only. The Back key code 461 is taken from
  the LG documents. The guard after a lost WebGL context has no reproduced failure.
  The pack step of `make webos` waits on a RustScript fix, see webos.md.
- Needed: an app driven with the remote on an LG C1, a film played there in a
  `VideoView`, a login with the code, and the UI suite on the TV with the Clipboard
  test no longer skipped. A lost context made on purpose with `WEBGL_lose_context`
  and a resize after it.
- Blocks: calling the TV a supported target.

## Password autofill on iOS

- Current: an edited field on an iPhone is a system text field, see
  `hilen/src/ui/views/basic/text_field/system_field.rs`. It gets the secure flag, and
  nothing that says what the field holds. `TextField` has no call for that. iOS offers
  a saved login only by guessing from the secure flag.
- Needed: a content type on `TextField`, user name, password, new password, email and
  one time code, passed to the system field as its `textContentType`. For the
  passwords of the app's own site also an associated domain in the entitlements of
  the mobile project. The simulator has no saved logins, so the proof needs a phone.
- Blocks: a login form that fills itself from the iOS keychain, and a code from an
  SMS offered over the keyboard.

## Text entry on a TV

Found by flixen, whose admin screens have text fields and stay off the TV for it.

- Current: a text field needs a keyboard. A TV has none, and the keyboard of the TV
  itself opens only for an HTML input, not for a canvas.
- Needed: an on screen keyboard view driven by the key focus, or a hidden HTML input
  that raises the keyboard of the TV and feeds the engine text field.
- Blocks: any screen with a text field on a TV.

## Sound tracks of a video in a browser

- Current: `VideoView` in a browser lists no sound tracks and plays the first one of
  the file. `set_audio_track` logs a warning.
- Needed: the `audioTracks` list of the video element where the browser has it. Not
  checked: whether Chromium 79 on webOS has it for an mkv.
- Blocks: a film whose first sound track the device cannot play, DTS on an LG C1.

## Packaged app for LG webOS

`make webos` packs a hosted app, a small `.ipk` whose page sends the TV to a served
wasm dist.

- Current: the app needs its server for its own files.
- Needed: the same target also packs the whole dist into the `.ipk`, so the app starts
  with no server. Not verified: a packaged app loads from a local origin on the TV, and
  the engine fetches the wasm and the asset groups relative to the document base.
- Blocks: a webOS app that works with no network and no server.

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
  fails in `create_bind_group` for `mesh_lights_bind`. The web lane runs the scene
  tests on WebGPU alone, so nothing pins it. The sprite pipelines of `level` no
  longer panic there, `instances_shader` leaves a source with no storage untouched,
  but no lane runs the level tests on the uniform path.
- Needed: `level-test` on `HILEN_UNIFORM_INSTANCES=1` in `make ci`. The scene follows
  the UI: the lights become a fixed uniform array, the opaque batches draw chunk by
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
  the lead. `Colliders` is off the browser lane too,
  SwiftShader draws the green collider lines in another shade, `#00ff60` where
  a GPU blends `#1fd153`, the lines are not antialiased there. `Collider
  shapes` and `Player walk` are off the browser lane as well. Both no longer
  check a picture, see [scene.md](scene.md), so the reason for their gates is
  gone, and neither was run on SwiftShader since. `Mouse look` is
  desktop only. `Cursor::capture` does nothing on a phone, and a browser grants
  pointer lock only after a real click, which no test can inject, so the
  capture is released at once. The UI test `Cursor capture` is desktop only for
  the same reason. Chromium on SwiftShader runs on a Mac in docker: an image
  with `chromium`, `xvfb`, `xauth` and `socat`, the driver started with
  `--browser none`, `socat` forwarding the container's port 44810 to the host
  so the page stays on `localhost`, and the Linux flags of `launchChrome`.
  `Cutout shadows` passed there on arm64, the CI runner is x86_64, so its gate
  stays until CI ran it.
- Needed: for `Collider shapes` and `Player walk`, a run on SwiftShader and then
  their gates removed. For the shadows, a bias or a depth format that reads the same on
  SwiftShader. For the collider lines, probes off the lines or lines that cover
  the same pixels without multisampling. For the mouse, a real click from the
  driver, through the debug protocol of Chrome and its twin in Firefox, else
  those two tests stay desktop only.
- Blocks: the gated tests on `make ui-web`, and `Mouse look` on `make ui-ios`.

## Video playback, the other lanes

Desktop macOS, Windows x64 and iOS landed, see [video.md](video.md): `VideoView` behind
the `video` feature, ffmpeg from prebuilt static archives, VideoToolbox and
D3D11VA decode, kira for the sound and as the clock. On macOS a 1080p60 and a
4K30 file play at full rate with sound. The rest of the platforms are open.

- Current: the archive exists for `aarch64-apple-darwin` and for
  `x86_64-pc-windows-msvc`. `demo` and `ui-test` turn the feature on through
  a target table for macOS and Windows. On Windows the 8 video UI tests and
  the video unit tests pass on a real machine, run by hand, no CI lane runs
  them there. Sound on Windows was checked by the tests only, nobody
  listened, and no large file was measured for dropped frames. The engine
  declares VAAPI for Linux, which has no archive and no CI lane. A decoded 4K
  frame is copied through system memory, 12 MB a frame, and plays at rate.
  Each `VideoView` keeps one frame image per source size for good, the
  managed image store never frees.
- Needed: an archive for Linux and for the Intel Mac from `build/ffmpeg.rs`,
  and for Windows on ARM from `build/ffmpeg-win.rs`. Linux needs `libva-dev`
  and `nasm` in `build/setup.sh` and in the docker containers of the Linux CI
  job, and its TLS named in the script. Then the target tables widen to
  `desktop`. A person plays a real film on Windows, with sound, and reads
  the dropped frames off the stats line.
  Android needs archives cross built per ABI with MediaCodec, and the feature
  unlocked there. The browser plays through a
  video element under the canvas, see video.md. Zero copy on macOS, a `CVPixelBuffer` into a Metal texture
  through the wgpu hal, only after an A/B shows the copy costs frames.
- Blocks: video on Linux, the Intel Mac, Windows on ARM and Android.

## Leftovers inside landed features

Small remainders not worth their own entry.

- Sign in with Apple was never run against the real Apple, which refuses
  `localhost` as a return address. The exchange, the form post and the revoke are
  proven against a stand in server only, see [login.md](login.md). The first backend
  with a real service id proves them.
- Tab focus traversal does not scroll. Tab selects the next text field even when
  it sits scrolled out of view inside a `ScrollView`, so the editing session
  starts off screen. Needs a scroll-to-view step in `select_next_field`, the way
  a multiline field already follows its caret line while typing.
- Frame stepped time covers `Animation` and `AnimatedImage` only. `RingSpinner`,
  the text field caret blink, tooltip and long press delays still
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

## Siri Remote input for tvOS

Found by the tvOS display bring-up, see [tvos.md](tvos.md). Waits for a real
tvOS app need.

- Current: the engine builds for tvOS and renders in the Apple TV simulator. The key
  focus exists, see [focus.md](focus.md), arrow keys, Enter and Escape drive every
  view. But no key reaches the engine on tvOS: Siri Remote events arrive through the
  UIKit focus engine and `UIPress`, and winit's UIKit backend forwards direct touches
  only.
- Needed: press forwarding in the winit fork, as arrow, Enter and Escape key events.
- Blocks: any interactive tvOS app, and the tvOS UI test lane.
