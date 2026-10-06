# Text rendering

How labels turn into pixels, and the knobs that exist to match other renderers,
added while making the skaityk port pixel identical to its WebKit original.

## Pipeline

`draw_label` in `ui_drawer.rs` builds a `Section` per label and queues it on the
label's `Font`. Each `Font` owns a `wgpu_text::TextBrush` for rasterization and a
`rustybuzz::Face` for shaping. A brush owns only its glyph atlas and vertex buffer, the
shader module and the pipelines are built once in the `wgpu_text` fork and shared by
every brush, so a font costs no pipeline build of its own. Glyphs are positioned by `ShapedLayout`
(`hilen/src/window/text/shaped_layout.rs`), a custom `GlyphPositioner` that shapes
every line with rustybuzz and hands pre-positioned glyphs to glyph_brush.

Clip boundaries, and a translucent background in front of all queued text, flush
text several times in one frame. The second keeps text behind a dim layer visible,
since a rect writes depth and would hide text flushed after it. A translucent rect
behind queued text, like a selection, does not flush, text writes depth over whole
glyph boxes and would cut holes in it.

Every flush is its own batch of the font's brush, and all batches share the glyph
atlas. A full atlas gives a later batch the space of glyphs it does not use, and
all texture writes of a frame run before the render pass, so an earlier batch
would draw unrelated glyph fragments. The `wgpu_text` fork prevents it: when a
write of a later batch lands on a glyph an earlier batch of the frame still draws,
the writes go into a copy of the atlas, the earlier draw keeps the old one, and
the next frame starts with an atlas of double size. The copy is written from the
pixels the fork keeps in memory, not copied on the GPU, a texture to texture copy
made an LG TV drop its WebGL context, see [webos.md](webos.md). `Clipped text batches` and
`Text corruption` pin it. The engine used to queue every visible label once before
drawing to keep the atlas stable, that preload pass is gone.

Shaping through rustybuzz exists because ab_glyph reads only the legacy `kern`
table. Modern fonts, Roboto included, keep kerning in `GPOS`, so the builtin
glyph_brush layout renders them with no kerning at all. rustybuzz applies GPOS,
GSUB and variation aware kerning like CoreText and browsers do.

## Shape cache

Each `Font` owns a `ShapeCache` that stores rustybuzz output per line of text,
keyed by the line, the pixel scale and the tracking. glyph_brush has its own
shaped section cache, but every `process_queued` call drops the entries absent
from that batch, and clip boundaries process several batches per frame, so
nothing in it survives a frame and every label used to reshape every frame.
Shaping was almost the entire cost of a text heavy frame in debug. The cache
also serves `Font::measure`, which shapes through the same path. Entries unused
for 10 seconds are dropped by a sweep that runs about once a second from
`Font::process_queued`.

Each `Font` also owns a `MeasureCache` that stores whole `Font::measure`
results, keyed by the text, size, wrap width, tracking, line height and runs.
The shape cache removes the shaping cost, but glyph_brush still walks every
glyph's bounds through `ttf_parser` on every measure, and a table rebuilding
hundreds of measured spans spent seconds there. Same 10 second sweep from
`Font::process_queued`.

## Glyph positions snap to the atlas grid

The glyph atlas of `glyph_brush_draw_cache` keeps one raster per glyph and
step of sub pixel position, 0.1 pixel by default. It rasterizes at the exact
position of whichever glyph asks first and every later glyph in that step
reuses it. So the same text got slightly different edges depending on which
label drew first, and that changed from run to run: the same frame differed by
up to 27 levels per channel, which is how a probe on a glyph edge failed in
one run and passed in the next. `snap_to_cache` in `shaped_layout.rs` rounds
every glyph position to that step before the brush sees it, so a step has one
possible raster. A glyph moves by at most 0.05 pixels. Measured after it, the
same frame differs by 1 level between runs. If the brush is ever built with
another position tolerance, `CACHE_STEPS` follows it.

## Sizes are pixels per em

`Label::text_size` means pixels per em, the CSS convention. ab_glyph `PxScale`
means ascent minus descent, a different unit. `Font::em_scale()` converts, both
the drawer and `Font::measure` multiply by it. For fonts whose ascent minus
descent equals their units per em, Special Elite, nothing changes. For the
default font Roboto the difference is 17 percent.

## Variable fonts

`Font::with_variations(name, data, &[(*b"wght", 550.0), (*b"opsz", 17.0)])` loads
a variable font instance with axes pinned. Each combination is its own managed
instance, cache under a name that includes the values. Axis values apply to both
the raster font and the shaping face. A missing axis is an error.

## Letter spacing

`Label::set_letter_spacing(points)` adds tracking between glyphs, applied by
`ShapedLayout` after kerning, mirrored in `Font::measure`. `Button` forwards it,
and `set_font`, to its internal label. Needed to match platforms that apply the
font's `trak` curve automatically, macOS does for the system font.

## Line handling

`\n` always breaks lines, single line labels included. Multiline labels
additionally wrap greedily at the Unicode line break opportunities of the
text, UAX 14 through the `unicode-linebreak` crate, so Latin breaks after
spaces and after a `/` in a path, Japanese and Chinese between characters.
Spaces before a break are dropped. Only the first glyph of a cluster may start
a line, a combining mark stays on its base. A Latin word wider than the bound
overflows, the `UIKit` word wrapping behavior. Thai, Lao, Khmer and Myanmar
have no opportunity inside a word without a dictionary, so a piece of those
wider than the bound breaks at the cluster that overflows. `Script wrap`
walks ten widths over Japanese, Thai, Korean and a mixed label. Vertical alignment
defaults to center, `Label::set_vertical_alignment` opts a label into Top,
which the multiline `TextField` uses so a tall field starts its text at the
top.

`Label::set_line_height(points)` replaces the font's own line pitch with a
CSS line box: baselines advance by the box, glyphs center in each box with
half the leading above and below, and a multiline measure returns boxes
times the box. Without it the engine pitch is the font's ascent minus
descent plus line gap, which for a wrapped text-sm label is around 16.5
where CSS puts 20, and the difference compounds down a block.

A tab has no glyph in most fonts, so the shaper returns notdef and the
line would show a box. `expand_tabs` in `ShapedLayout` swaps every tab
for the space glyph of its font and stretches its advance to the next
tab stop, 4 space widths counted from the line start, so columns align
like in an editor. It runs after shaping and before wrapping, on the
copy the shape cache returns, so measure, wrap and drawing agree. A tab
at the very end of a text still measures as one space, `glyph_brush`
bounds the last glyph by its own advance. `Label tab` pins the behavior.

## Line limit, outline and shadow

`set_max_lines(n)` on a multiline label cuts the text to `n` lines and ends the
last one in an ellipsis, the CSS `line-clamp`. The cut is the longest start of
the text that still wraps into the limit with the ellipsis after it, found by
binary search over the layout and cached per width like the single line
ellipsis. `size_for_width` then measures what is drawn.

`set_text_outline(color, width)` and `set_text_shadow(color, offset)` draw the
same text again in 1 color behind the glyphs. `set_text_shadow_blur(radius)`
makes the shadow soft. A hard shadow is a plain copy of the text through the
brush, `hilen/src/ui/label_drawer.rs`.

The outline and a soft shadow are images, `hilen/src/ui/label_effect.rs`. The
glyphs of the label, of every font it uses, raster on the CPU into one
coverage map, the same outlines at the same places the brush draws. The map
is spread in 2 passes, along the rows and then down the columns, in
`deps/pixels/src/spread.rs`: the outline moves every edge out by its width,
round at the corners, the blur is a gaussian whose sigma is half the radius,
like a CSS text shadow. So the cost grows with the spread and not with its
square, and no spread is cut. The image is drawn pixel for pixel as 1 quad
of the image pipeline and kept by the hash of the text, its layout, its
place against the whole pixel origin of the label and the effect. It is made
again only when one of them changes, and freed 120 frames after its last
draw.

Text writes depth over whole glyph boxes and sits only 2 depth steps in front
of its view, so each layer is `f32::EPSILON` nearer than the one before and the
text itself comes forward by the number of layers.

## Matching other renderers

Browsers composite text in sRGB space and so does the engine: render targets
are plain Unorm and color values are encoded sRGB end to end, see
[colors.md](colors.md). Glyph coverage therefore blends exactly like browser
text with no compensation, and ports use nominal font weights on both
polarities. Measuring workflow, scripts and the trak table details live in the
hilen skill's migration chapter, next to this repo's users.

One real difference remains: `CoreText` applies stem darkening when it
rasterizes, so browser text on macOS carries around 10 percent more ink
mass than the plain outline at UI sizes. `Font::with_variations_darkened`
opts a font into an approximation: the wgpu_text fork's darkening entry
point takes the maximum of five coverage taps a fraction of a pixel
apart, which moves every glyph edge outward by that fraction, and the
glyph quads are inflated to give the widened edge room. Keep the
strength at or under 0.5, the taps reach half a texel further through
bilinear filtering and the atlas pads glyphs by one pixel. 0.5 landed
the kukareker port within a few percent of WebKit mass at 12 to 14
point text. Off by default, engine text is untouched.

## Color runs

`Label::set_color_runs(ranges)` paints byte ranges of the text in their own
colors, the rest keeps the text color. The drawer emits one glyph_brush text
per run and per gap between runs, all slices of the same string. `ShapedLayout`
joins them back, shapes the whole string once, and maps every glyph to the
text its cluster came from, so a run boundary never breaks kerning or letter
spacing. Theme pairs in a run re-resolve on a switch like the text color does.
`set_text` clears the runs.

## Font runs

`Label::set_font_runs(ranges)` draws byte ranges in their own font, underlined,
or both, through `RunStyle`. `ShapedLayout` shapes every line per segment, one
per run it crosses and one per gap, each with its own face and shape cache, so
a wider run font moves the wraps and `Font::measure` follows. The line height
and baseline stay the label font's. Every `Font` owns one brush, so the drawer
queues a mixed label on every brush it touches, and each layout copy lays out
the whole text and emits only the glyphs of the font that brush draws. Kerning
stops at a run boundary, the two sides are different fonts. An underline is
not a glyph, `UIDrawer::draw_underlines` puts a rect under every line piece of
the run from the base font's underline metrics, in the color the text has
there, between the label background and the glyphs. `set_text` clears the runs.

## Glyph fallback

`Font::set_fallbacks(fonts)` registers fonts consulted in order for chars the
label font has no glyph for, `reset_fallbacks` clears them. `runs_with_fallbacks`
in `window/text/fallback.rs` runs at `Label::shaping_runs`, walks the text once,
asks `Font::has_glyph` per char with the effective font, the explicit run font or
the base, and gives each missing char a synthesized run with the first fallback
that covers it, merged with its neighbors. So a fallback rides on the font runs
machinery above and wrapping, measuring and the shape cache follow with no extra
path. Whitespace and control chars stay with their font, shapers handle them
without a glyph. The runner resets the fallbacks between tests, `GlyphFallback`
covers the plain label and the split inside an explicit run.

After the registered fallbacks come the fonts the OS has installed, so text from
the network in any script draws. `window/text/system_fallback.rs` walks the
system font folders once through fontdb, on a thread started right after
`before_launch`, and asks every face for the char in a fixed order: a short list
of UI families per platform first, then upright regular faces, then the family
name. fontdb knows no folder on iOS and Android, those two are added by hand.
macOS `LastResort` is skipped, it has a placeholder for every code point. Files
are memory mapped for good like the system emoji font, a face in a `.ttc` loads
by its index, and the answer per char is cached, a miss too. A char no font
covers draws notdef. On by default, `Font::set_system_fallback(false)` turns it
off. The browser cannot read system fonts, there it does nothing. The system
fonts differ per machine, so the UI test runner turns it off for every test and
hands the app its own setting back after the run. `System font fallback` turns
it on and checks where the IPA chars of a Jotunn description go.

## Color glyphs

The emoji of a color font come as COLR layers, v0 or v1, or as PNG strikes,
CBDT and sbix, and none of them has a coverage mask for the text atlas.
`color_glyph.rs` rasterizes such a glyph with ttf-parser and tiny-skia, both
already in the tree. A COLR paint graph paints into a pixmap the size of its
box, gradients, clips, transforms and composite layers included, a PNG strike
decodes as is and scales on draw. The raster is a managed `Image`, cached per
font, glyph and pixel size for the life of the font, and a unit test walks
every emoji of the test fonts at three sizes.

Routing is per glyph. `ShapedLayout` places every glyph the same way, then
the brush skips the color ones and `UIDrawer::draw_color_glyphs` draws them
through the image pipeline at the same origin, snapped to whole pixels. Plain
glyphs of a color font stay on the brush, so shaping, measuring, wrapping,
runs and fallback know nothing about color, and a color font registered with
`Font::set_fallbacks` gives every label color emoji. A color glyph carries its
own colors, so the text color, its alpha and a text gradient do not apply to
it. A COLR palette entry that asks for the text color paints black.

`Font::system_emoji` maps the platform's own emoji font from disk. Apple
Color Emoji, an sbix font, ships with every Mac, iPhone and Apple TV and is
licensed for that hardware only, so it is never bundled and the file is
memory mapped, 190 MB that cost nothing until a glyph is read. Off Apple it
answers `None` and the app keeps a bundled font:

```rust
Font::set_fallbacks([Font::system_emoji().unwrap_or_else(|| Font::get("TwemojiColr0.ttf"))]);
```

`Color emoji` pins the three formats on every lane with 8 glyph subsets of
Twemoji and Noto Color Emoji in `assets/fonts`. `System emoji` pins the
system font on Apple, so a macOS update that redraws an emoji re-records it.

## Gradient text

`Label::set_text_gradient(start, end)` fades the glyphs from the top of the
label frame to its bottom, the CSS `background-clip: text` case. A gradient set
with `apply_gradient` paints the label box instead, see [colors.md](colors.md).
Both ends accept a `DynamicColor`, so a themed title resolves on a theme change
like a plain text color does.

The section extra in the wgpu_text fork carries a second color, and `to_vertex`
packs it into the glyph vertex along with the section box, which glyph_brush
already hands over as `bounds`. The ramp is applied in the vertex stage, so the
corner colors interpolate across the glyph quad, no value crosses between the
stages and the fragment shader is untouched. Flat text sets both ends to the
same color, which costs one `mix` and no branch. Per glyph this is 12 bytes,
`Vertex` went from 52 to 64.

`set_text_color` clears a gradient, so the two cannot both be live on one label.

## Secret text

`Label::set_secret(true)` marks the text of a label as a secret, like a recovery
phrase. Call it before `set_text`. The label then writes zeros over its text before
the memory is freed, when the text is replaced and when the label is dropped. The
text stays out of the shape cache and the measure cache, so such a label is shaped
again on every frame that draws it. Keep it to the few labels that need it.

A `TextField` holds a secret from its first `set_secure(true)`, also while
`set_secure(false)` shows the text. Every edit builds the new text in one buffer of
the exact size, `joined` in `hilen/src/wipe.rs`, and the old one is wiped. Its
`changed`, `editing_ended` and `submitted` events carry one bullet per character, the
real text is read with `text()`. A copy of shown text goes through `Clipboard::set_secret`.

What cannot be wiped: a `String` passed to `set_text` by value is the copy of the
caller. Single typed characters pass through winit and the event queue as they are.
The shaper, the glyph brush and the GPU buffers keep the glyph numbers and positions
of drawn text until other text replaces them, and the glyph atlas keeps the picture
of every drawn glyph. On iOS the system text field a field is edited in holds the text while it
is edited. It is cleared when the editing ends, the memory it used is not wiped. With the `inspect` feature the inspector still gets the real text.
