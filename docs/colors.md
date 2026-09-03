# Color space

The engine works in encoded sRGB end to end, the same convention browsers,
CSS and design tools use. This is the UI industry standard, chosen so a
designer's hex plus alpha from Figma or a stylesheet lands on screen with the
same numbers. Game engines render scenes in linear light, but they too
composite UI in encoded space, Unreal draws Slate after the tonemapper. The
3D scene draws in the same encoded frame, its shader decodes, lights in linear,
rolls the highlights off and encodes at the end of the fragment, see
[scene.md](scene.md). A linear offscreen pass with real HDR, composited into the
UI frame, is the upgrade if a scene ever needs it.

## The convention

- `Color` floats hold encoded sRGB values, the exact 0..1 fractions a CSS
  value means. `Color::hex("#22c55e")` is the canonical constructor, const,
  usable in a `pub const`. `with_alpha` matches CSS `rgba()`.
- Render targets are plain Unorm, `Bgra8Unorm` on desktop, `Rgba8Unorm` on
  android, the browser's preferred canvas format on wasm. Never an sRGB
  format. Fixed function blending then operates on the encoded bytes, which
  is exactly the CSS compositing and gradient math.
- Image textures are also plain Unorm. Their bytes are already encoded and
  sampling must return them unchanged.
- `Color` to `U8Color` and back is plain scaling by 255 with rounding, no
  transfer function anywhere. Alpha is never gamma converted.
- Screenshot readback returns the target bytes as they are, so a recorded
  expectation is the same hex a browser DevTools color picker would show.

## What this buys

A solid fill drawn from a hex string screenshots back as exactly that hex on
every platform. A translucent color composites to the same result CSS
produces, `rgba(10, 10, 10, 0.55)` over white is `#787878`, never the
lighter linear blend. Gradients interpolate like CSS gradients. Text
antialiasing blends like browser text with no shader compensation. The
`Css colors` UI test pins all of this with hand computed expectations.

## Gradients

Every view has them, `Label` and `Button` included, since `set_gradient` and
`apply_gradient` live on `ViewData`. They fill the view box behind its content
and follow its corner radii.

- `set_gradient(start, end)` is the vertical ramp, CSS `linear-gradient(180deg,
  start, end)`.
- `apply_gradient` takes a `Gradient` for everything else. `Gradient::linear`
  takes a CSS angle, `Gradient::radial` and `radial_at` take a center as a
  fraction of the box, and `with_end_stop` is the CSS stop position such as
  `transparent 60%`.
- The ending shape of a radial is the CSS `ellipse`, the one through the
  farthest corner, so it follows the box aspect. A square box gives a round
  one. The CSS `circle` shape does not exist yet.
- Stops interpolate premultiplied, like CSS, so a ramp into `transparent` fades
  without sliding through black.
- A border draws on top of the ramp, the same band `UIRectInstance` draws over a
  flat fill. The gradient pipeline used to carry no border at all, so setting
  one on a gradient view silently did nothing.

Glyphs are separate. A gradient on a `Label` paints its box, not its text. For
CSS `background-clip: text`, `Label::set_text_gradient(start, end)` fades the
glyphs themselves from the top of the label frame to its bottom, see
[text.md](text.md). The `Gradient` UI test covers all of these.

## The surface colorspace

Correct bytes in the framebuffer are not the end of the story, the OS
still decides how the window surface is interpreted on the panel. The
metal backend used to leave the `CAMetalLayer` colorspace nil, which
disables color matching entirely, so on a wide gamut display every
saturated color oversaturated, `#f59e0b` displayed as `#ff9900` while
grays matched exactly. Since `hilen-wgpu-hal` 30.0.1 the layer is tagged
with an explicit sRGB colorspace, so the OS matches the content into the
display space the same way it does for browser output. Framebuffer
screenshots and UI test readback never see this layer, which is why the
kukareker port passed framebuffer comparison for eight waves while
looking wrong on screen. On screen colors are checked with
[pixdiff.md](pixdiff.md), whose captures go through the display
pipeline. If grays match but hues do not, suspect this layer first.

## History

The pipeline used to render into sRGB targets, which made the hardware
decode, blend in linear and encode back. Every translucent and antialiased
pixel came out lighter than the design, and colors defined as raw sRGB
fractions were encoded twice, the beekeeper port measured `#0a0a0a` text as
`#383838` on screen. The named constants in `color.rs` keep their old
on-screen appearance, their definitions were rewritten as the hex those
constants actually displayed.
