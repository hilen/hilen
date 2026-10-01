# Level

The 2D `level` feature: sprites, rapier physics, tile maps, dug terrain and lights
under a camera.
The tests are in `level-test`, see [ui-tests.md](ui-tests.md).

## Time

`LevelManager::update` runs the level in fixed steps on the real clock, `Clock` time.
The default step is `LevelManager::DEFAULT_STEP`, a 120th of a second, so a 120 Hz
screen gets a new pose every frame too. `set_step` changes it. One frame runs at most
`MAX_STEPS_PER_FRAME` steps. After a longer stall the rest is dropped, so the level
slows down instead of running a burst. `LevelManager::time` is the simulated seconds.

The level used to run one step of `1 / refresh rate` per frame. The headless loop
draws frames far faster than a screen, and there it ran 68 seconds in 0.6. The
`Level time` test pins both the real clock and the stepped clock.

A physics level splits each step into `PHYSICS_SUBSTEPS`, 2, so a kinematic wall
moves in hops of a 480th of a second. `LevelSetup::update` gets the substep `dt`.

A test on the stepped clock, `Clock::enter_stepped` and `step_frames`, gets exactly
2 steps per frame. With free running frames, 2 `wait_for_next_frame` calls can run
no step at all.

## Screen and level points

A level unit is 10 render pixels times the level scale, around the center of
`UIManager::render_area`. `level_point` and `screen_point` convert between that and
UI points, `convert_touch` takes window pixels. `visible_rect` is the level rect on
screen. `LevelBase::cursor_position` is the cursor in level units, written every
frame. `Mouse::held(MouseButton)` is the raw button state next to `Keys::held`, a
finger counts as the left button.

## Sprites

`SpriteData` has `flip` for mirrored images and `tint`, multiplied into every texel,
white by default. `Image::set_filter(ImageFilter::Nearest)` keeps pixel art sharp,
once per image, for every sprite and view that draws it. `image_scale` draws the
picture bigger or smaller than the body, around the sprite position. It used to
scale the distance to the camera too, `Sprite image scale` pins the place with the
camera away from the sprite.

The image sampler repeats, and under MSAA a pixel that a quad edge only partly covers
runs its fragment at the pixel center, outside the quad. The uv there is past the
image and wrapped to the opposite edge, a thin line of the bottom row along the top.
`sprite_textured.wgsl` and `ui_image.wgsl` clamp the uv half a texel inside the image,
pinned by `Sprite edges` and the UI test `Image edges`.

The textured pipeline draws one batch per image. The batches go farthest first, by
the farthest sprite of each, see `sort_back_to_front`. In the order the images were
first seen, a soft cutout edge drawn before the batch behind it blended with the
clear color and hid that batch there by depth, a halo that depended on load order.

## Tile maps

`LevelBase::add_tile_map` takes a `TileMap`, a grid of `TileKind`s, each with an image
and a `TileCollision`: `None` for decor, `Solid`, or `Platform`. A layer draws behind
every sprite and behind the layers added before it, and only the cells on screen are
drawn. `box_hits_solid`, `move_box` and `stands_on_solid` let an actor walk and land on
the grid without rapier. `move_box` goes in hops under half a cell, so a fast box never
skips a thin wall.

A platform is a jump through ledge. It stops a box only when the box falls onto its
top from above, never moving up or sideways, and `stands_on_solid` counts its top as
ground. `move_box` with `drop_through` falls through it, for a held Down key.
`is_solid` and `box_hits_solid` see only solid cells.

`walk_box` moves a box like `move_box`, and a box standing on the ground that walks into
a ledge at most `step_up` high climbs onto it, the way a Terraria player walks up one
block without a jump. A ledge with no room above it for the box stays a wall.

A kind can be framed like Terraria frames its blocks. `TileKind::with_frames` takes
`TileFrames`, a list of image variations for each of the 16 `TileSides` shapes, the set
of the 4 neighbors that join a cell. A kind joins itself, `TileMap::join` makes 2 kinds
join, like dirt and stone, and outside the grid joins when `outside_solid` is on. The
shape is read when the cell is drawn, so a `set` reframes the neighbors on the next
frame with nothing to update. The variation comes from a hash of the cell position, so
a cell keeps it. A shape with no frames draws the kind's `image`. `sides` and `image_at`
answer what a cell looks like. The `Tile framing` level test shows it in pixel art mode.

## Terrain

`LevelBase::add_terrain` takes a `Terrain`, ground made of polygons with holes in level
units that can be dug into, like Worms. `add_polygon` adds ground of one material,
newer ground wins where two overlap. `carve_circle` and `carve_polygon` cut a shape
out of one material only, the others keep theirs. `material_at` reads the material at
a point. There is no way to add ground back after a carve.

`add_material` registers a `TerrainMaterial`, a `fill` image that repeats in level
space every `fill_size` units, and these optional looks:

- `with_surface` hangs a strip, like grass, straight down from every edge a box can
  walk on and that is open to the sky, no ground straight above it. A pit dug from the
  top grows grass on its floor, a tunnel stays bare. The strip is clipped to the
  ground of its material in its chunk and the 8 around it, so it never shows in the air
  and hangs on across a chunk border.
- `with_surface_end` puts an end piece over each end of a run of strip, like the
  rounded, outlined end of a grass ledge. The image's left column lies on the end, a
  right end draws it mirrored. Its u runs along x, the piece hangs straight down like
  the strip. A run crossing a chunk border gets no end there.
- `with_edge` draws a flat color band along the whole outline, like the dark outline
  of pixel art ground. It is the ground minus the ground shrunk by its width with
  geo's `Buffer`, cut from the chunk and the 8 around it, so arcs and corners join and
  a chunk border gets no band. The strip draws over it, the strip art brings its own
  top line.
- `with_blend` lets the fill spill its width over every border with another
  material, clipped to that material, thinned out in whole texels by a 4 by 4 ordered
  dither, so dirt fades into stone instead of meeting it in a hard line.

A piece of outline finds its material by a probe just inside it, asked of the whole
terrain, so an edge a hair under a chunk border still gets its look.

The ground is split into square chunks of 8 units, and a carve rebuilds only the
chunks its shape touches. The boolean operations come from
[geo](https://github.com/georust/geo). After a carve, points of one outline closer than
0.04 units merge, so digging one spot many times keeps the point count bounded. A
chunk cuts the ground along its square, and where the chunk next door has ground on
the other side, that cut is left out of the outline, so a box never meets a chunk
border.

`move_box` and `stands_on_ground` follow the `TileMap` contract. An edge flatter than
`slope_limit`, 50 degrees by default, is ground. The middle of the box bottom rides it
up and down, and a box that stood on it and is not moving up sticks to it going
downhill. A steeper edge is a wall to the box sides and top. A box falling onto one
slides down it, and a box landing on a corner of the ground, like the rim of a crack
narrower than itself, stays there. Every move sweeps the box over the whole hop, so a
fast box never skips a thin edge.

`add_one_way` adds an invisible segment that works like `TileCollision::Platform`: it
stops a box only when the box falls onto it from above, passes it moving up or
sideways, and `drop_through` falls through it. It holds the box at any slope.

The terrain is drawn in front of the tile maps and behind every sprite, lit like the
sprites. lyon triangulates each chunk once per change and the pipeline keeps its
buffers until the next carve of that chunk or a chunk next to it, whose outline band
and strips reach across. Every part of a terrain lies at one depth with a `LessEqual`
test and draws in `PAINT_ORDER` over all visible chunks, fill, blend, edge band,
strip, strip end, so a later part wins, also a strip hanging into the next chunk.
Depth steps between the parts were a few float steps apart at the terrain depth and
fought, a 1 pixel column of an end piece came and went. `terrain.wgsl` takes no
instance storage buffer, so it runs on the WebGL2 path, and carries 4 float components
to the fragment stage, under the A7 limit. The strip and its end clamp their uv half a
texel inside the image for the MSAA reason in Sprites. The edge band draws the uniform
color and still binds the fill image, and the shader samples before it picks, since a
texture read needs uniform control flow. `Terrain ground` pins the fill, the strip and
its ends, the edge band, the blend, a pit open to the sky, a bare tunnel, and grass
across a chunk border.

## Pixel art

`LevelManager::set_pixel_art(Some(pixel_size))` draws the level as pixel art, each
`pixel_size` level units one art pixel. Smooth shapes, a terrain slope or a turned
sprite, would otherwise cut through the middle of texture pixels, a screen pixel half
grass and half sky, which pixel art never has. A new level starts smooth, set it in the
level setup.

`LevelDrawer::prepare` draws the level before the frame pass into a texture with one
texel per art pixel, one sample and no MSAA, so an edge covers a texel or not. The level
pipelines exist twice for that, the frame set with the frame MSAA and a pixel set with
one sample, both from `WithSamples::with_samples`. The frame pass then shows the texture
over the render area with `PixelBlitPipeline`, the nearest filter and alpha blending
over the clear color. The texture is even in both sides and 2 texels bigger than the
area, so a texel edge falls on the camera.

The scale snaps so an art pixel covers a whole number of screen pixels, `set_scale` and
`set_points_per_unit` round to it, an uneven count would draw some art pixels a screen
pixel wider than others. `view_camera` is the camera on a whole art pixel, the one the
level draws from and `level_point`, `screen_point` and `visible_rect` convert with, so a
touch lands where the drawing shows. `Grass across chunk border` pins the look, with
the unit test `pixel_art_snaps_the_scale_and_the_camera` for the snapping.

## Light

`LevelBase::ambient_light` lights everything, white draws the level unlit.
`add_light` adds a point light with a color, an intensity, a radius and a falloff,
the returned handle moves it. The three level shaders share `sprite_view.wgsl`, which
adds up the ambient and every light per pixel and caps the sum at white. The
uniform takes `MAX_SPRITE_LIGHTS`, 64, and with more the lights nearest the screen
win. The level `background` image is not lit, a sprite sent `to_background` is.
The textured shader carries 8 float components to the fragment stage, the A7 limit
in [ios.md](ios.md), so it has no room for another varying.
