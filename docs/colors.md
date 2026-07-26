# Color space

The engine works in encoded sRGB end to end, the same convention browsers,
CSS and design tools use. This is the UI industry standard, chosen so a
designer's hex plus alpha from Figma or a stylesheet lands on screen with the
same numbers. Game engines render scenes in linear light, but they too
composite UI in encoded space, Unreal draws Slate after the tonemapper. A
future 3D scene pass should render linear offscreen and hand a finished image
to the UI layer.

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

## History

The pipeline used to render into sRGB targets, which made the hardware
decode, blend in linear and encode back. Every translucent and antialiased
pixel came out lighter than the design, and colors defined as raw sRGB
fractions were encoded twice, the beekeeper port measured `#0a0a0a` text as
`#383838` on screen. The named constants in `color.rs` keep their old
on-screen appearance, their definitions were rewritten as the hex those
constants actually displayed.
