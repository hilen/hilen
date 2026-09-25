# Level

The 2D `level` feature: sprites, rapier physics, tile maps and lights under a camera.
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
once per image, for every sprite and view that draws it.

The textured pipeline draws one batch per image. The batches go farthest first, by
the farthest sprite of each, see `sort_back_to_front`. In the order the images were
first seen, a soft cutout edge drawn before the batch behind it blended with the
clear color and hid that batch there by depth, a halo that depended on load order.

## Tile maps

`LevelBase::add_tile_map` takes a `TileMap`, a grid of `TileKind`s, each with an image
and a solid flag. A layer draws behind every sprite and behind the layers added
before it, and only the cells on screen are drawn. `box_hits_solid`, `move_box` and
`stands_on_solid` let an actor walk and land on the grid without rapier. `move_box`
goes in hops under half a cell, so a fast box never skips a thin wall.

## Light

`LevelBase::ambient_light` lights everything, white draws the level unlit.
`add_light` adds a point light with a color, an intensity, a radius and a falloff,
the returned handle moves it. The three level shaders share `sprite_view.wgsl`, which
adds up the ambient and every light per pixel and caps the sum at white. The
uniform takes `MAX_SPRITE_LIGHTS`, 64, and with more the lights nearest the screen
win. The level `background` image is not lit, a sprite sent `to_background` is.
The textured shader carries 8 float components to the fragment stage, the A7 limit
in [ios.md](ios.md), so it has no room for another varying.
