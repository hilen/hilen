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

## Web lane scroll injection determinism

Found by `Table view 2 test` failing only in the browser lane once MSAA 4x made
frames heavier. The frame stepped clock, `gm::Clock` with `ui_test::step_frames`,
now exists, so this is a small job on top of it.

- Current: the test injects 100 wheel scrolls and taps the rows it expects at the
  bottom. On desktop and the iOS simulator the landed offset is stable, in Chrome
  the run lands at a different offset every time, 50 rows, 33 rows, zero rows, so
  how many deltas take effect depends on frame pacing. With `HILEN_MSAA=1` the lane
  passes by timing luck. Rendering is not involved, `Drawing paths` pins the same
  pixels in all three lanes.
- Needed: scroll inertia reads `Clock` instead of real elapsed time, and the test
  runs stepped, so one injected wheel delta is one applied delta regardless of
  frame rate.
- Blocks: a green `make ui-web` at MSAA 4x. Accepted as a known flake for now.

## Pause continuous work while the window is not visible

- Current: a live animation or a loaded level keeps the loop polling even
  when the window is minimized, fully covered, or on another desktop. A
  browser tab throttles `requestAnimationFrame` on its own, native does not.
- Needed: listen to winit's occluded event in the app handler and hold the
  loop in `Wait` while the window is occluded, then request a frame when it
  shows again. Animations keep their clock, so a long hold still lands them
  at the end state on the first frame back.
- Blocks: nothing visible, only CPU and battery in a background window.

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

## Thai word breaking

Wrapping follows UAX 14, which has no boundaries inside Thai, Lao, Khmer
or Myanmar text, so a too wide piece of those breaks at the overflowing
character. Platforms wrap them at words through an ICU dictionary.

- Needs: a dictionary based word breaker for the complex scripts, fed into
  the wrapper as extra break opportunities.
- Blocks: nothing yet, the demo Fonts page shows the character breaks.

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

## Leftovers inside landed features

Small remainders not worth their own entry.

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
  straight alpha costs 3 to 5 times the raster itself. Uploading premultiplied and
  blending svg textures premultiplied in the image pipeline would remove that loop,
  it needs a flag per image instance and a blend change in the shader.
- DrawingView paths: texture fills for paths, more than 8 gradient stops, and soft
  edges on arbitrary path outlines, a radial alpha ramp only covers circular glows.
