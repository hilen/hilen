# Engine gaps

Missing engine features, found by porting a real app. Each entry lists the current state
in code, what is needed, and what it blocks. When one of these lands it needs UI tests
like any other engine change.

The driver app is skaityk-te at `~/dev/apps/skaityk-te`, github.com/hilen/skaityk-te.
It is a rewrite of skaityk, a Tauri and Vue reader app for Lithuanian learners at
`~/dev/apps/skaityk`. Full visual and functional parity with the original is the
acceptance bar. The news feed part is done with the engine as is. The reader and the
app-wide look need the features below.

A second driver is web-te, the beekeeper homelab UI port in the `local` repo at
`beekeeper/web-te`, see its PORT.md. It drove the networking and system layer, all
landed: `netrun::ws::WebSocket`, one API over tokio-tungstenite with rustls on native
and web-sys in the browser, with native and browser tests. netrun REST verbs Patch and
Delete with simple post, patch and delete helpers, every 2xx accepted and an empty
response body parsed as JSON null so `()` outputs work. One shared rustls client config
with a named provider, without it two crypto providers in the graph leave no default
and connects panic. `system::Clipboard` and `system::open_url` on desktop, wasm, iOS
and Android, clipboard covered by the `Clipboard test` UI test. The `App::log_targets`
hook so apps see their own debug logs. The assets root prefers a cwd with an `assets`
folder, so a crate nested in a monorepo keeps its own assets. Wasm asset urls are
relative and resolve against the document base, so a dist hosted under a path prefix
like `/te/` fetches from its own mount point through a `<base data-trunk-public-url />`
tag, and `Assets` is exported so an app can await the boot group before building views.

A third driver is kukareker-hilen at github.com/hilen/kukareker-hilen, a rewrite of
kukareker, the Tauri and Vue git client at `~/dev/apps/kukareker`. Desktop only, the
app shells out to system git and ssh. Pixel parity with the original is the acceptance
bar. The port copies the Tauri app's Rust logic modules as they are and rebuilds the
Vue UI on the engine. Its assessment produced the entries below marked kukareker.

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
is read once at startup. This unblocked dark mode. The screen background followed.
`UIManager::set_clear_color` takes the same pair and re-resolves it on a switch, so an
app paints its background with the clear color and leaves the root view transparent.
That keeps the iOS safe areas the same color as the screen, where a colored root view
stops at the safe area edge. `NavigationView::push` and `present` no longer force the
new screen white for the same reason. Hover events landed as opt-in
`enable_hover` plus a `hovered` event that fires true on enter and false on exit. Only
the topmost hover enabled view under the cursor is hovered, on desktop and in the
browser since a touch screen has no pointer, and modal layers block hover like they
block touches. Hover follows mouse moves and wheel scroll, and clears when the cursor
leaves the window. Everything runs on input events, nothing per frame. This unblocked
card lift and button hover colors. Drop shadows landed as an
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
of the skaityk port. URL routing for wasm apps landed as `system::Router`. The engine
owns the history API: `current_path` reads the path the page loaded on relative to
the document base, `push` and `replace` write history entries without a reload, and
browser back and forward surface through the `on_pop` event. Outside the browser
every call is a no-op, so app navigation code carries no platform cfg. Covered by
the wasm-only `Router test`. This unblocked bookmarkable pages and deep links for
the beekeeper port.

Form input landed. `RadioGroup<T>` is a vertical list of options where exactly one is
selected, a ring that fills with a dot, mirroring the `DropDown` API down to
`set_value` not firing `changed` so restoring a saved pick is never mistaken for a
user action. `Button` gained `set_enabled`, which swaps in disabled colors and stops
`on_tap` firing while still taking the touch, so a dead button does not become a hole
that whatever sits behind it starts catching. Restoring the original colors needed
`ViewData::ui_color` and `Label::ui_text_color`, which give back the color as it was
set rather than what a theme pair resolved to. Without them a disabled and re-enabled
button kept a flattened plain color and stopped following theme switches.

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

## Multiline TextField

Found by the bug report dialog. A bug description needs a large textarea like
the karkas dialog has, not a single line.

- Landed: opt in `TextField::set_multiline`. Enter inserts `\n` instead of
  ending editing, the field got a caret with editing state in
  `text_field/editing.rs`, content height, a scroll offset that follows the
  caret, and text selection. `Label` gained an opt in Top vertical alignment
  threaded through `ShapedParams` into `ShapedLayout`, default stays Center so
  recorded text expectations hold. Covered by the `Multiline text field` and
  `Label vertical alignment` tests, and human mode now advances on ctrl so
  typing into a held field no longer collides with the advance key.

## TextField theme colors

Found by a port with a text field on several of its screens.

- Landed: `TextField::set_text_color` and `set_selected_color` widened to
  `impl Into<UIColor>`, so a field holds theme pairs like every other view.
  The selection color juggle saves the original pair through
  `ViewData::ui_color` instead of the resolved plain color, so a field keeps
  following theme switches after editing. Covered by the `Text field theme`
  test. `is_placeholding` landed alongside, `text()` returns the placeholder
  while empty and a reader needs to tell the hint from entered text.

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
- Landed: browser matched text blending. The UI pipeline blends encoded sRGB
  values on plain Unorm targets, see [colors.md](colors.md), so glyph coverage
  composites exactly like browser text and ports use nominal font weights on
  both polarities. The wgpu_text coverage remap remains in the fork but only
  activates on sRGB targets, which the engine no longer uses.

## Shape edge anti-aliasing

Found by jagged rounded corners in the corner radius test.

- Landed: analytic one pixel coverage anti-aliasing on the four rounded box SDF
  pipelines, rect, image, gradient and backdrop. Each fragment turns its signed
  distance into an alpha ramp via `fwidth`, so the ramp stays one pixel wide at any
  scale, and blends over what is already drawn. Border to fill boundaries ramp the
  same way. The shadow pipeline was already soft through its own `smoothstep`. Covered
  by the re-recorded `CornerRadius`, `Gradient` and `Outline` tests.
- Landed: whole pass MSAA 4x. The frame renders into a multisampled color target
  with a matching depth buffer and resolves into the presentable texture at every
  pass end, so blur sampling and readback see resolved pixels. Every pipeline in
  the pass and the text brush share `msaa_sample_count()`, `HILEN_MSAA=1` switches it
  off as the benchmark A/B lever. 4 is the ceiling everywhere: the WebGPU spec
  guarantees only 1 and 4, and this Mac's GPU rejects 8 outright. Cost measured
  unguarded at the 297 panel stop scene: CPU identical, capacity identical, GPU
  2.1 ms to 3.4 ms, far under budget.
- Current: `sprite_textured` still hard discards on zero texture alpha, so sprite
  cutout edges stay aliased when a sprite scales.

## Vector path rendering reconnect

Found by the beekeeper node card sparklines, blank on every platform.

- Landed: lyon backed vector paths drawn in the UI pass. `VectorPath` builds
  polylines, polygons, circles and bezier curves through a Canvas style builder,
  `DrawingView::add_stroke` takes width, caps, joins and miter limit,
  `add_fill` takes a fill rule, and a second circle sub path cuts a hole under
  EvenOdd. `PathData` is an indexed mesh with a compare and write placement
  uniform, so paths follow scrolling and cost nothing per frame when static.
  Fully transparent paths are skipped, they would only write invisible depth.
  `CircleView` renders for the first time, and its `set_color` became
  `set_circle_color`, the `&self` trait method always shadowed the `&mut self`
  inherent one. Pinned by the `Drawing paths` test, 320 recorded probes passing
  identically on desktop, the iOS simulator and Chrome WebGPU.

## Web lane scroll injection determinism

Found by `Table view 2 test` failing only in the browser lane once MSAA 4x made
frames heavier.

- Current: the test injects 100 wheel scrolls and taps the rows it expects at the
  bottom. On desktop and the iOS simulator the landed offset is stable, in Chrome
  the run lands at a different offset every time, 50 rows, 33 rows, zero rows, so
  how many deltas take effect depends on frame pacing. With `HILEN_MSAA=1` the lane
  passes by timing luck. Rendering is not involved, `Drawing paths` pins the same
  pixels in all three lanes.
- Needed: make injected scrolls on wasm land deterministically, one applied delta
  per injection regardless of frame rate, likely together with the stepped time
  source below since scroll inertia samples real elapsed time.
- Blocks: a green `make ui-web` at MSAA 4x. Accepted as a known flake for now.

## Tooltips

Found porting the beekeeper UI, which hangs real data on hover titles, full
shas, container statuses, deploy errors, button hints. kukareker needs them
too, full file paths, ref chip sources, full dates, and a richer hover card
on commit avatars showing author name, email and the commit date.

- Current: no tooltip view exists, ports drop every `title` attribute. The one
  load bearing tooltip, the deploy error, got a tap fallback in the port.
- Needed: a tooltip layer, show a floating label after a hover delay near the
  cursor, dismiss on move out or tap, touch platforms substitute long press or
  skip. Needs the hover machinery already in `src/ui/hover.rs`. The floating
  content must accept a small view, not only a text label, so the kukareker
  avatar card with several lines can ride the same layer.
- Blocks: hover parity for web ports, kukareker hover parity.

## Right-click and context menus

Found by the kukareker port assessment. The app hangs actions on right-click
menus everywhere, branch rows, file rows, commit rows, stash rows.

- Landed: a `secondary` touch event on every touch enabled view. A right
  button press fires it on desktop and in the browser, with the position in
  the view's own space. It never captures the view, so a right release cannot
  end a left capture, every mouse event is finger 1. A long press fires the
  same event on every platform, 0.5 s held within 10 points, and consumes the
  hold so the release is not a tap. That is how a touch screen opens a menu.
  `ContextMenu::show_at_cursor(items)` and `ContextMenu::show(items, at)`
  float a one column list at a point and slide it back inside the screen.
  `MenuItem::new(title, action)`, `MenuItem::separator()`, `disabled()` and
  `danger()`. The menu is its own touch layer on a clear backdrop, dismissed
  by a tap outside, Escape, or picking an item, and only one is open at a
  time. Themed light and dark, hover highlight where a pointer exists. No
  submenus. Covered by `Secondary click` and `Context menu test`, plus the
  `inject_right_click` and `inject_long_press` test helpers.
- Blocks: nothing. This unblocked the kukareker core wave.

## TableView variable row heights

Found by the kukareker commit graph, a row is 30 px without ref chips and
42 px with them.

- Current: `cell_height(index)` takes an index but only `cell_height(0)` is
  read and every cell shares that height. Offsets, recycling and tap mapping
  all assume one height.
- Needed: honor per-index heights, cumulative offsets for scroll position and
  tap mapping, recycling, `set_cell_spacing`, `scroll_to_bottom` and
  `bottom_reached` keep working. A table with uniform heights keeps the
  current fast path.
- Blocks: the kukareker commit graph in its core wave.

## Colored text runs in a label

Found by the kukareker diff view, a syntax highlighted code line needs
per-token colors.

- Current: a `Label` has one text color for the whole string. A multi color
  line means one label per token run placed by hand, measured and drifting.
- Needed: per-range color runs on one label, threaded through `ShapedParams`
  into `ShapedLayout` so shaping, kerning and letter spacing stay intact.
  Highlighting itself stays in the app, syntect produces the ranges.
- Blocks: kukareker syntax highlighted diffs. A later wave, the port ships
  plain add and remove row tinting first, so this is not on the core path.

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

Landed. The suite runs green in real installed Chrome and Firefox, 100 tests,
0 failed, through `make ui-web` and the `web-ui-tests` CI job.

- The lane: `build/web/drive.ts`, run by Bun, builds the atomics wasm, serves
  `dist` with the COOP and COEP headers, launches a real installed browser
  with a throwaway profile and reads the report over the inspect WebSocket,
  see [inspect.md](inspect.md). No automation protocol and no console
  scraping, so any browser works. Chrome is the default, `BROWSER=firefox`
  switches. On a failure or a timeout the driver asks the app for a
  screenshot over the same socket and saves it to
  `target/web-test/ui-web-failure.png`, CI uploads it as an artifact. A panic
  reaches the driver through the `/te-panic` beacon and fails the run.
- The atomics build the driver owns: target features
  `+atomics,+bulk-memory,+mutable-globals`, `build-std=std,panic_abort`,
  and link args `--shared-memory`, `--max-memory`, `--import-memory` plus exports
  `__heap_base`, `__data_end`, `__wasm_init_tls`, `__tls_size`, `__tls_align`,
  `__tls_base`. rustc adds none of them itself. The `hilen_run_tests` autorun
  fires, the suite worker runs every test, the scene texture readback feeds
  `check_colors` and failures report without a filesystem.
- Landed, rendering: every platform renders into a plain Unorm target with
  encoded sRGB values, see [colors.md](colors.md), so wasm, android and native
  produce the same bytes and readbacks return what recorded expectations
  compare against. The canvas format is still the browser's preferred one,
  resolved at runtime from the surface capabilities during window creation,
  because rendering the non preferred one makes Chrome convert every frame and
  makes Firefox present it with red and blue swapped. The readback swizzles by
  the actual format.
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
- Landed: assets in the browser. demo's build.rs writes `assets/assets.json`,
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
- Resolved, Firefox: the blocker was Playwright's patched Firefox Nightly, not
  Firefox. That build throttled requestAnimationFrame for the engine page down
  to an exponential backoff, one second to four seconds per frame, and failed
  the `diagonal.bmp` fetch with a body decode error. Real installed Firefox
  runs the full suite green with none of that, verified on a clean throwaway
  profile and on a normal user profile. Headless Firefox still has no working
  WebGPU, the lane always runs headed. The driver made this reachable, results
  ride the inspect socket, so no automation protocol is needed at all.
- Blocks: nothing. CI compiles the wasm target and runs the suite in Chrome.

## Siri Remote input for tvOS

Found by the tvOS display bring-up, see [tvos.md](tvos.md).

- Current: the engine builds for tvOS and renders in the Apple TV simulator, but the
  UI is touch driven through `WindowEvent::Touch` and Apple TV has no touch screen.
  The app draws and nothing can drive it.
- Needed: a remote input path. Siri Remote events arrive through the UIKit focus
  engine and `UIPress`, and winit's UIKit backend forwards direct touches only, so
  the winit fork needs press and focus forwarding, and the engine needs to map that
  onto its views, most likely a focus model over the existing key and touch events.
- Blocks: any interactive tvOS app, and the tvOS UI test lane, since what a test can
  assert depends on this path.

## Suggested order

Browser UI tests, the path rendering reconnect and whole pass MSAA have landed.
Right-click context menus landed. The kukareker core wave needs TableView
variable row heights next, then tooltips during the port, then colored label runs for
its later diff highlighting wave. Among the rest, frame stepped animation
testing goes next, it has two live needs, the unassessed animation problem in
the present test and the web lane scroll determinism flake it would also fix.
The text stack remainder waits for a real need for color emoji or font
fallback, and tvOS remote input waits for a real tvOS app need.
