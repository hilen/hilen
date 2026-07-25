# Engine gaps

Missing engine features, found by porting a real app. Each entry lists the current state
in code, what is needed, and what it blocks. When one of these lands it needs UI tests
like any other engine change.

The driver app is skaityk-te at `~/dev/apps/skaityk-te`, github.com/gebling-studio/skaityk-te.
It is a rewrite of skaityk, a Tauri and Vue reader app for Lithuanian learners at
`~/dev/apps/skaityk`. Full visual and functional parity with the original is the
acceptance bar. The news feed part is done with the engine as is. The reader and the
app-wide look need the features below.

Already proven sufficient during the port, for reference: `TableView` with columns and
`bottom_reached` paging, `ImageMode::AspectFill`, `Image::download`, SVG in textures,
`size_changed`, `ModalView`, `OnDisk` storage, `NavigationView` push and pop,
`Window::set_title`.

Landed from this list already, each with UI tests: font wiring, an app-wide default via
`Font::set_default` plus per-label `Label::set_font`, named keys in `Keymap`, arrows,
enter and friends via `NamedKey`, and label content measurement, `Label::content_size`
and `size_for_width` on top of `Font::measure`, with fit-to-text placer rules
`fit_text_width`, `fit_text_height` and `fit_text` that compose with anchors and
min and max clamps. Flow-wrap layout landed as the `all_wrap` placer tiling rule.
Subviews flow left to right in declaration order and wrap into rows, children keep
their own sizes including fit-to-text, hidden children take no space, and the
container height follows the content. Row and item gaps come from `all(margin)`.
This unblocked the skaityk reader word grid. Runtime theming landed as `DynamicColor`
light and dark pairs accepted by every color setter. `Theme` picks the effective look,
`ThemeMode` follows the OS or forces one. A switch re-resolves bound colors on the
live view tree in one walk and fires `UIEvents::theme_changed`, the draw path keeps
reading plain resolved colors. The OS theme arrives through winit `ThemeChanged` and
is read once at startup. This unblocked dark mode. Hover events landed as opt-in
`enable_hover` plus a `hovered` event that fires true on enter and false on exit. Only
the topmost hover enabled view under the cursor is hovered, desktop only, and modal
layers block hover like they block touches. Hover follows mouse moves and wheel scroll,
and clears when the cursor leaves the window. Everything runs on input events, nothing
per frame. This unblocked card lift and button hover colors. Drop shadows landed as an
opt-in `Shadow { offset, radius, color }` set through `set_shadow`. A dedicated shadow
pipeline draws a blurred rounded rect under the view, following its corner radii. The
view masks the shadow inside its own shape and hidden views cast nothing. Per-corner
radius landed as `CornerRadii` with `set_corner_radii`, honored by the rect, image and
gradient pipelines, while `set_corner_radius` keeps the uniform shortcut. Together
these unblocked card elevation and top-only rounded card images. TableView cell
spacing landed as `set_cell_spacing`, gaps between rows and columns, no gap after the
last row. Gaps are purely visual for touch, the table maps a tap to the nearest cell
index instead of per cell touch areas. The modal backdrop landed as an opt-in
`modal_scrim_color` override on `ModalView`, transparent by default. The scrim is a
dedicated `ScrimView` drawn after every other pipeline including text, so it dims
images, gradients and glyphs behind it, while the modal above keeps the depth buffer
and stays bright. A plain translucent rect could not do this, it erased later
pipelines through the depth test instead of dimming them. These unblocked the reader
card grid spacing and the dialog dim. Backdrop blur landed as `BlurView` with
`set_blur_radius`, its color acting as a tint over the blur. It shows a blurred copy
of everything drawn before it in tree order inside its frame and corner radii, while
its subviews stay crisp on top. The frame splits into several render passes at the
blur view, the scene downsamples to quarter resolution, gets a separable gaussian
blur, and composites back through a dedicated backdrop pipeline. A frame without a
blur view keeps the old single pass path. This unblocked the frosted sticky header.
The modal scrim blur landed as an opt-in `modal_blur` override on `ModalView`, zero
by default. With a radius the modal wrapper is a `BlurView` tinted by
`modal_scrim_color` instead of a plain scrim, so the whole scene behind the dialog
blurs and dims while the dialog stays crisp. This closed the last visual parity gap
of the skaityk port.

## Text stack rework

Found by the FontZoo emoji page. Parked until a real need.

- Landed since: shaping through rustybuzz via `ShapedLayout`, so GPOS kerning,
  GSUB and variable font axes apply like in browsers. Em based text sizing,
  per label letter spacing, variable font instances via `Font::with_variations`.
  See [text.md](text.md).
- Current: rasterization still goes through wgpu_text, glyph_brush and ab_glyph —
  outline glyphs only, a single channel atlas tinted by the text color. Color emoji
  tables, CBDT, sbix and COLR, are ignored, so emoji render monochrome via the
  bundled `NotoEmoji.ttf`. No font fallback chains.
- Needed for the rest: migrate label rendering to cosmic-text with swash. Brings
  font fallback chains and color emoji in every format. Large: replaces the glyph
  atlas and `draw_label`, and invalidates every recorded text expectation in the UI
  tests.
- Blocks: colorful emoji. Nothing in the driver app today.
- Landed: gamma aware text blending in the wgpu_text pipeline. On sRGB targets
  the shader remaps coverage so the linear blend lands on the sRGB space result
  browsers produce, so ports use nominal font weights on both polarities.

## Shape edge anti-aliasing

Found by jagged rounded corners in the corner radius test.

- Landed: analytic one pixel coverage anti-aliasing on the four rounded box SDF
  pipelines, rect, image, gradient and backdrop. Each fragment turns its signed
  distance into an alpha ramp via `fwidth`, so the ramp stays one pixel wide at any
  scale, and blends over what is already drawn. Border to fill boundaries ramp the
  same way. The shadow pipeline was already soft through its own `smoothstep`. Covered
  by the re-recorded `CornerRadius`, `Gradient` and `Outline` tests.
- Current: `ui_path` and `polygon` fill arbitrary triangulated geometry, so there is
  no distance field to ramp and their edges stay hard. `sprite_textured` hard discards
  on zero texture alpha, so sprite cutout edges are aliased too.
- Needed: MSAA on the render pass. All pipelines in one pass must share the sample
  count, so this is count 4 for the whole UI pass plus a multisampled color target and
  a resolve step, or a separate multisampled pass just for the geometry pipelines. It
  has a real per frame cost, so it needs an A/B per [benchmark.md](benchmark.md) before
  it lands.
- Blocks: smooth vector path and polygon edges. Nothing in the driver app today.

## Frame stepped animation testing

Found by an animation problem seen during the PresentRich human review, not yet
diagnosed.

- Current: `Animation` samples real elapsed time, so mid-animation pixels depend on
  the wall clock and machine speed. Tests can only wait for `on_finish` and check the
  settled state, the present and navigation tests poll for completion with generous
  timeouts. Human mode pauses on injected events and checks, never inside an
  animation.
- Needed: one engine time source that all animation code reads. Normal runs keep real
  time, a stepped mode advances it by a fixed delta per rendered frame, 16.666 ms for
  a 60 fps timeline. The 0.4 s present slide becomes exactly 24 deterministic frames,
  push and pop become 30. A `step_frames(n)` helper next to `wait_for_next_frame`
  renders and advances n frames, so `check_colors` and `--record-colors` pin
  mid-flight frames through the existing flow, and `on_finish` lands on an exact
  frame count, no timeout waits. In human mode an active animation holds the run and
  a dedicated key advances one frame per press, the window title showing the frame
  number, the total and the virtual time, space keeps its current meaning. Stepped
  mode is opt in per test, defaults untouched, so the corpus stays green. The
  animation drives frames regression test must never run stepped, it proves free
  running animations request their own frames. Anything else reading real time
  inside the frame moves to the same source or the timeline drifts. Touches stay
  locked during present and push, a stepped test must not expect taps to land mid
  animation. The exact end value is currently never written, the last commit lands
  just before expiry, and with a fixed step clamping the finishing frame to the end
  value becomes possible, but that changes visible behavior and recorded
  expectations, so it is its own gated decision.
- Blocks: diagnosing the PresentRich animation problem, and any regression test
  that pins an animation mid flight.

## Browser UI tests

The suite in a real browser, in flight.

- Current: the full suite passes end to end in headed Chrome, 100 tests, 0
  failed, a clean `TE_TEST_RESULT` line. The `te_run_tests` autorun fires, the
  suite worker runs every test, the scene texture readback feeds `check_colors`
  and failures report without a filesystem. Needs the atomics build: target
  features `+atomics,+bulk-memory,+mutable-globals`, `build-std=std,panic_abort`,
  and link args `--shared-memory`, `--max-memory`, `--import-memory` plus exports
  `__heap_base`, `__data_end`, `__wasm_init_tls`, `__tls_size`, `__tls_align`,
  `__tls_base`. rustc adds none of them itself. The page needs COOP and COEP headers.
- Landed, rendering: wasm now renders through an sRGB view of the surface, so
  colors match native and readbacks return the bytes recorded expectations
  compare against. The canvas format is the browser's preferred one, resolved at
  runtime from the surface capabilities during window creation. Hardcoding
  `Rgba8Unorm` made Chrome convert every frame and made Firefox present it with
  red and blue swapped. The readback swizzles by the actual format. Android still
  hardcodes non sRGB `Rgba8Unorm` and stays suspect for the same darker colors.
- Landed, main thread rules: the frame loop uses `Wait` plus a rAF chain, never
  `Poll`, and the hot shared locks spin on wasm, both per
  [dispatch.md](dispatch.md). std `Instant` panics on wasm, `web_time` replaces
  it in the test corpus, and `recv_timeout` in the animation test polls with
  short sleeps on a worker, the only timed block wasm has. Tinted SVGs read
  their source from memory kept at download time, the browser cannot reread the
  file, and an invalid tinted source degrades to the default image.
- Landed, dev profile decode speed: boot spent ten seconds decoding the asset
  fixtures in a debug wasm build, `diagonal.bmp`, `svg_rendered.png` and
  `full_hd.jpg` cost multiple seconds each. The workspace now builds the decoder
  crates optimized in dev, boot dropped to 0.7 seconds. Boot time and slow
  decodes log at debug level, a regression shows in the console immediately.
- Landed: assets in the browser. test-game's build.rs writes `assets/assets.json`,
  every image, font and sound with a content hash and a load group, the group is
  the first folder under the kind folder, kind root files are `boot`. On wasm the
  engine downloads the boot group into the managed stores before anything runs,
  `Assets::load_progress` feeds the loading bar, `Assets::load_group` pulls a lazy
  group on demand, the game sprites download on the first Physics tap. Urls carry
  the content hash, so a host can serve `assets/` as immutable. A trunk post_build
  hook recopies the manifest, the asset pipeline runs in parallel with cargo and
  would ship it one build stale. Downloads now fail on HTTP error status, a 404
  page used to be stored as asset bytes, and a font that fails to parse degrades
  to the default font instead of killing the session.
- Landed: the last 3 Chrome failures. Game view rendered the level transparent
  black because Tint rejected `sprite_textured.wgsl`, a value returning fragment
  fn ended in an if else where one branch discards, and spec `discard` demotes
  and continues, so the function could fall off the end. naga accepts that shape,
  which is why native passed. Now it discards then returns unconditionally, like
  every other engine shader. Label image and Nine segment probed the default
  image because the nine segment textures live in the lazy `button` group and a
  sync `get` miss fell through to a filesystem read wasm does not have. The test
  autorun now downloads every manifest group before the suite starts, native
  equivalence, any file reachable on demand.
- Firefox is blocked by the browser, not the engine. Playwright's Firefox Nightly
  throttles requestAnimationFrame for the engine page down to an exponential
  backoff, one second to four seconds per frame, while a plain page from the same
  server with the same headers holds 60 fps. The engine paces itself off rAF, so
  everything main thread crawls to a stop. Firefox also reproducibly fails the
  `diagonal.bmp` fetch with a body decode error. Retest when the Playwright
  Firefox build updates. Headless Firefox has no working WebGPU at all.
- Needed: canvas sizing for the 600 by 600 fixtures and device pixel ratio
  handling, then a `build/web/` lane driver, Bun plus Playwright, that owns the
  build flags, serves dist with the isolation headers, runs Chromium and parses
  the `TE_TEST_RESULT` console line. Then `make ui-web` and a CI job.
- Blocks: browser regressions going unnoticed. CI only compiles the wasm target today.

## Suggested order

Browser UI tests are in flight already. Among the rest, frame stepped animation
testing goes first, it has a live need, the unassessed
animation problem in the present test. The text stack remainder waits for a real
need for color emoji or font fallback. Shape MSAA waits for a real need for smooth
path or polygon edges, since the SDF UI shapes people actually use are already
anti-aliased.
