# hilen-pixdiff

The pixel parity tool for ports. It captures app windows from the actual
screen and diffs two captures into ranked difference regions, so a port
is compared against its original by a machine instead of a hand picked
checklist. Both captures pass through the display pipeline, which also
catches what framebuffer comparison cannot see, like a surface
colorspace bug. The dot color drift the kukareker port shipped with for
eight waves was exactly this class, framebuffer bytes correct, on
screen colors wrong.

Install from the repo:

```bash
cargo install --path ~/dev/hilen/hilen-pixdiff
```

## The one command flow

```bash
hilen-pixdiff run <original> <port> --size 1200x800 --crop-top 56 -o /tmp/diff
```

`run` resolves both windows, parks them at the top left corner, resizes
both to the same size over the Accessibility API, waits for relayout,
captures both, restores each window's original size and position, then
diffs and writes the report plus a heatmap. The parking step matters,
macOS clamps a window frame to the screen, so a window near the right
edge silently gets less width than asked.

The queries are the owner app name, matched case insensitively, or a
bare window id. When a name matches several windows the tool lists the
candidates with ids and errors, rerun with an id.

Before a run, put both apps in the same state, same repo, same tab,
same selection. Content differences are honestly reported as
differences, and a differing document drowns the style signal.

## Subcommands

- `capture <query> -o out.png` — capture one window from the screen at
  its retina backing resolution. The window only needs to be on screen,
  not frontmost.
- `resize <query> <WxH>` — resize one window, frame size in points.
- `diff a.png b.png -o heatmap.png` — compare two same size captures.
- `run <a> <b> --size WxH -o dir` — the whole flow above.

## How the diff works

Two passes over a cell grid, default 8 px cells:

- The mean pass compares per cell average colors. It forgives the
  subpixel text anti aliasing that differs between any two renderers
  while catching layout shifts, wrong sizes and color drift.
- The pixel pass marks a cell when a large fraction of its pixels
  differ beyond a per pixel tolerance, which catches flat fills.

Marked cells cluster into connected regions, reported largest first
with the bounding box in pixels and points and the mean hex of both
sides. The heatmap is the second image with every region outlined in
red. The exit code is nonzero when differences remain, so a script can
gate on it. Known state regions, live badges or timestamps, are skipped
with repeatable `--ignore x,y,w,h` rects.

## Permissions

Two one time grants for the terminal running the tool, both in System
Settings, Privacy & Security:

- Screen Recording, for the window captures.
- Accessibility, for the resize and move in `run`.

The tool prints which one is missing when a call fails.

## Platform

Capture and resize are macOS only, the diff core is platform
independent, so on other platforms `diff` still works on captures made
by hand. The window id to accessibility element bridge uses the private
`_AXUIElementGetWindow`, long stable and relied on by window managers
like yabai.
