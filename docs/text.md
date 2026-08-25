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
additionally wrap greedily at spaces to the label width. A single word wider
than the bound overflows, same as the builtin layout did. Vertical alignment
defaults to center, `Label::set_vertical_alignment` opts a label into Top,
which the multiline `TextField` uses so a tall field starts its text at the
top.

## Matching other renderers

Browsers composite text in sRGB space and so does the engine: render targets
are plain Unorm and color values are encoded sRGB end to end, see
[colors.md](colors.md). Glyph coverage therefore blends exactly like browser
text with no compensation, and ports use nominal font weights on both
polarities. The wgpu_text fork still carries a coverage remap entry point,
but it activates only on sRGB targets, which the engine no longer uses.
Measuring workflow, scripts and the trak table details live in the
hilen skill's migration chapter, next to this repo's users.

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
