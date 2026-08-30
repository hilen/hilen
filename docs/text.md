# Text rendering

How labels turn into pixels, and the knobs that exist to match other renderers,
added while making the skaityk port pixel identical to its WebKit original.

## Pipeline

`draw_label` in `ui_drawer.rs` builds a `Section` per label and queues it on the
label's `Font`. Each `Font` owns a `wgpu_text::TextBrush` for rasterization and a
`rustybuzz::Face` for shaping. Glyphs are positioned by `ShapedLayout`
(`hilen/src/window/text/shaped_layout.rs`), a custom `GlyphPositioner` that shapes
every line with rustybuzz and hands pre-positioned glyphs to glyph_brush.

Before drawing, the UI tree queues every visible label once and processes each font's
glyph atlas to its final layout for the frame. Clip boundaries can flush text several
times in one frame. Without the preload, a later flush can grow and reorder the shared
atlas after earlier vertices already captured its texture coordinates, which renders
unrelated glyph fragments in those earlier labels. The draw pass then queues the same
sections for their actual clipped batches against the stable atlas.

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

## Matching other renderers

Browsers composite text in sRGB space and so does the engine: render targets
are plain Unorm and color values are encoded sRGB end to end, see
[colors.md](colors.md). Glyph coverage therefore blends exactly like browser
text with no compensation, and ports use nominal font weights on both
polarities. The wgpu_text fork still carries a coverage remap entry point,
but it activates only on sRGB targets, which the engine no longer uses.
Measuring workflow, scripts and the trak table details live in the
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
