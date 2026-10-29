# 3D scenes

The `scene` module is the 3D twin of `level`, behind the `scene` cargo feature. A
`#[scene]` struct gets a `SceneBase` injected, `SceneManager` runs the one active
scene, nodes are `Own<dyn Node>` the way sprites are, and `SceneDrawer` draws them
with `MainLock` pipelines on `VecBuffer`. Physics is `rapier3d`, math is glam
re-exported from `gm::volume`. The scene draws every frame while one is loaded,
like a level.

1 world unit is 1 meter. Gravity is 9.81 down, a speed is meters per second, a
light's range and a camera's `near` and `far` are meters, and a depth photo reads
meters. A model made in other units is fitted with `set_scale` on its node.

## Writing a scene

```rust
#[scene]
#[derive(Default)]
struct Playground {}

impl SceneSetup for Playground {
    fn needs_physics(&self) -> bool { true }

    fn setup(&mut self) {
        self.camera = Camera { position: Vec3::new(0.0, 6.0, 10.0), ..Camera::default() };
        self.sky = Some(Sky::gradient(Color::hex("#3a7bd5"), Color::hex("#d9e4f0"), Color::hex("#5a4a3a")));
        self.lights.push(Light::point(Vec3::new(4.0, 3.0, 4.0)).color(Color::hex("#ffb060")).intensity(6.0));
        self.make_node::<Wall>(Shape3::Plane(20.0), Vec3::ZERO);
        self.make_node::<Body>(Shape3::Ball(0.5), Vec3::new(0.0, 5.0, 0.0))
            .set_color(Color::hex("#3498db"))
            .set_metallic(1.0)
            .set_roughness(0.2);
    }
}

SceneManager::set_scene(Playground::default());
```

`Shape3` is `Box`, `Ball`, `Plane` or `Model`, and it gives both the mesh and the
collider, so what is drawn is what collides. `Body` is dynamic, `Wall` a fixed and
bouncy collider, `Prop` is only drawn. `NodeTemplates` carries `set_color`,
`set_material`, `set_metallic`, `set_roughness`, `set_position`, `set_rotation`,
`set_scale`, `set_friction` and `set_restitution`, and a `Body` has `set_velocity`,
`add_impulse` and `set_damping`. Rapier has no rolling resistance, so a ball on a
plane rolls forever without damping.

The world is right handed with y up, the glTF and Blender convention. A plane is
drawn at its origin facing up and its collider slab hangs below it, so a body rests
on the drawn surface. `Camera::orbit` turns the camera around its target and
`Camera::zoom` moves it along the line of sight, the demo page and presentation
mode drive them from a drag and the wheel.

## A scene inside a view

`SceneView` draws the running scene inside its own frame, so a scene can sit next
to other views, like a 3D preview next to a code editor:

```rust
let preview = self.add_view::<SceneView>();
preview.place().t(0).b(0).r(0).w(500);
let scene = preview.set_scene(Part::default());
```

The view runs the one scene of `SceneManager`. `set_scene` on it replaces a scene
that ran over the whole window, and `SceneManager::set_scene` takes the scene out
of the view again. `SceneManager::host` gives the view a scene draws in. The scene
stops when its view is gone.

The scene draws in `prepare`, before the frame's pass opens, into an image of the
view's size in pixels, through the same targets and copy pass a picture uses, see
Pictures below. The camera takes the aspect of that image. The view shows the image
with an `ImageView` inside it, so it clips, sits under the views over it and next to
the views beside it like any view. The main pass draws no scene then, and the frame
is not drawn whole for it. The view asks for every frame with
`keep_frames_exempt` while it holds the scene, so only its rect is drawn again, see
[damage.md](damage.md). `set_background` is what shows where the scene draws
nothing, clear by default. A sky or fog covers it.

A drag on the view turns the camera around its target, or the head of a player,
the wheel and a pinch zoom, and a tap picks a node the way a touch on a scene over
the whole window does, in the pixels of the view. `set_camera_controls(false)`
leaves the drag and the zoom to the app. A drag that begins on the scene's
`TransformHandle` goes to the handle. `ray`, `node_at` and `view_point` on the scene
measure in the frame of the view, and `view_point` gives points of the root view
inside that frame. A touch outside the view never reaches the scene. Only 1 scene
runs, so only 1 view shows a scene at a time.

## Orthographic camera

`Camera::projection` is `Projection::Perspective`, the default, or
`Projection::Orthographic { height }`, where a thing has the same size on the screen
at any distance and `height` is the world height the view shows, the front, top and
side views of a part. `Camera::orthographic(position, target, height)` makes one.
The camera stands outside what it shows, it sees from its near plane on.

`ray` follows the projection. An orthographic ray starts on the near plane at its
pixel and goes along the line of sight, so every ray of the view is parallel.
`screen_point` reads the depth along the line of sight, since w stays 1. `zoom`
scales the height and leaves the camera where it is. `view_height(depth)` and
`units_per_pixel(world, area)` give the size of the view and of 1 pixel at a point,
the same at every depth for an orthographic camera.

The shader gets the line of sight and the height in the `eye` vector of
`SceneUniform`. Every pixel then looks along that line, for the specular light and
the size of a pixel the shadow cascades pick a map by. A sky needs nothing, its
direction comes from the near and the far point of the pixel. An orthographic
slice of the view is a box, so its cascade sphere sits in the middle of it, see
`slice_sphere`.

## Lines

`Line3` is a straight line in a scene, `width` pixels wide at any distance, in its
own color. The lines in `SceneBase::lines` are in the world, the ones in a node's
`lines` are in its own space and move, turn and grow with it. `Line3::grid` and
`Line3::axes` make a floor grid and the 3 axes in `AXIS_COLORS`.
`MeshData::edges(crease_degrees)` gives the edges of a mesh worth a line, where 2
triangles meet at more than that angle and where a triangle has no neighbor, with
the corners of flat faces welded:

```rust
let edges = mesh.edges(30.0).into_iter().map(|[a, b]| Line3::new(a, b, dark).width(2));
let mut part = self.make_node::<Prop>(Shape3::Model(Model::from_mesh("part", mesh)), Vec3::ZERO);
part.lines = edges.collect();
self.lines = Line3::grid(20.0, 1.0, gray);
```

A line hides behind the nodes in front of it, or draws over everything with
`on_top`, the outline of a selected part or a handle. Each line is 1 instance of a
quad of 6 vertices that `wide_lines.wgsl` spreads across the screen from its 2
ends, with square ends so a polyline has no gaps. A line that crosses the near plane
is cut there. The tested lines draw after the opaque nodes and before the
translucent ones, the ones on top last. A line sits a little nearer to the eye
along its line of sight, so it keeps its pixel and a face it lies on does not cut
it: a thousandth of its distance, plus 2.5 pixels of the world for each pixel from
its middle to its edge. That holds a floor seen down to about 22 degrees. The cost
is a hidden edge that shows for a few pixels where it leaves a corner of a part.

## Triangle hits

`hit_triangle(ray)` on a scene gives the nearest triangle of a model that the ray
meets on its drawn side, as a `TriangleHit` with the node, the distance, the point,
the normal and the index of the triangle. `triangle_at(pixel)` does it for the ray
of a pixel. Only nodes that draw a model take part, at rest, and the box around
each node is tried first. A model keeps its triangles in model space for this,
`Model::triangle_count` and `Model::triangle(index)`. For a mesh built in code the
index is the triangle of its `MeshData`, 3 indices each, since the split into 16 bit
parts keeps their order. So an app reads its own data per triangle, like the part a
CSG library made it from. `node_at` and a touch still use the box around a model.

## Transform handle

`TransformHandle` in `SceneBase::handle` draws 3 arrows to move along x, y and z, 3
rings to turn about them and 3 boxes on the arrows to scale along them, over
everything, `size` pixels long at any distance. It stands on a node with
`TransformHandle::on(node)`, or at its own `position`. The axes are the world's, or
the node's with `local`. `modes` picks which parts show.

A `SceneView` hands it every drag that begins on a part, a box before an arrow
before a ring, in place of turning the camera. The part held draws yellow. The
handle changes nothing by itself, `on_drag` gets a `HandleDrag` with the part, the
axis and the amount from the start of the drag, on every move and once more with
`ended`. The amount is world units along the axis for a move, radians about it by
the right hand for a turn, and a factor for a scale. `snap` rounds each to a step.

```rust
let mut handle = TransformHandle::on(part.weak_node());
handle.snap.turns = Some(PI / 12.0);
handle.on_drag.val(move |drag| match drag.part.mode {
    HandleMode::Move => part.set_position(start + drag.axis * drag.amount),
    HandleMode::Turn => part.set_rotation(Quat::from_axis_angle(drag.axis, drag.amount) * turned),
    HandleMode::Scale => part.set_scale(scale * drag.amount),
});
self.handle = Some(handle);
```

A move takes the point of the axis nearest to the ray of the pointer. A scale is
the ratio of that point's distance from the center to the one at the press. A turn
is the angle the pointer went round the center on the screen, the other way when
the axis points away from the eye. A node has 1 scale for all axes, so a scale
along 1 axis is for the app to apply, a CAD app writes it into its source.
`press`, `drag_to` and `release` drive the handle from code of the app.

## Models

`Model` is a `.glb` on the GPU, a managed resource like an `Image`. It loads from
`assets/models` through `filesystem::read_bytes`, so the APK and the browser
manifest serve it like any asset, and `Model::get("tree.glb")` returns the one
copy. `Shape3::Model(model)` puts it on a node. The meshes, the metallic roughness
materials, the embedded base color, normal, metallic roughness and occlusion
textures and the node tree load, the
tree flattened into parts with their placements, so a node draws every mesh of the
file at its own size and with its own materials. `set_scale` sizes a node uniformly
on top of that, so a model in other units fits the scene, the collider and picking
follow. A primitive without a material
takes the node's `Material`. A material with `alphaMode` `MASK` loads as a cutout
with its `alphaCutoff`, `BLEND` as translucent. A material marked
`KHR_materials_unlit` loads as `unlit`, `unlit_quad.glb` is the fixture and
`deps/models/make_unlit_quad.ts` makes it with Bun. `doubleSided` is not read, back
faces are always culled. The collider is the box around the model's `bounds`,
placed where the bounds are, so a model whose origin sits at its feet still rests
on the floor. A primitive over 65535 vertices is split into parts so every lane
draws 16 bit indices, and one without normals shades flat as glTF asks. Only a
`.glb` with embedded buffers and images loads, triangles only, no morph targets.

## Skins and animations

A file with a skin or an animation keeps its node tree as a `Rig`: every node's
rest transform and parent, the skins with their joints and inverse bind matrices,
and the animation clips, `Model::clips` by name with step, linear and cubic spline
keys. A skinned vertex carries its four joints and weights in a second vertex
buffer, so a static mesh pays nothing. Each frame the drawer poses the tree, one
matrix per joint into a storage buffer shared by the whole frame, and the skinned
twins of the mesh and shadow pipelines, the same shader from its `v_skinned` entry,
blend the four matrices per vertex. An unskinned part under an animated node moves
with it, a windmill hub turns its blades. Four bind groups is every lane's limit,
so the joints share the instance group. At rest a skinned model draws through its
rest joints without walking the tree.

A node plays a clip with `play`, looped, or `play_once`, which holds the last
frame, then `set_animation_speed`, `set_animation_time` to seek, `stop_animation`
back to rest, and `animation_time` and `is_animating` to ask. The time moves with
the scene's update steps, the same clock as the physics. The bounds, and with
them the collider and picking, are the rest pose. `Fox.glb` is the Khronos glTF
sample fox with its Survey, Walk and Run clips, see `Fox.license.md` next to it.

The Blender sources sit next to the exports in `assets/models`. One file exports
with `blender --background --python deps/models/export_glb.py -- assets/models/tree.blend`,
skins and actions included, and the script also mends what the old files miss.
`deps/models/make_tree.py` builds the tree fixture from scratch, a trunk and four
cone tiers and nothing else, since a model's collider is the box around its bounds
and a ground plane in the file would make it a slab. `deps/models/make_leaf_card.py`
builds the leaf card, 2 crossed quads with a masked leaf texture. Only the `.glb` files reach
the browser manifest.

## Materials and lights

Every node has a `Material`: `color`, `metallic`, `roughness`, an optional base
color `texture` that multiplies the color per texel, an optional `normal_map` in
tangent space with green up, the glTF convention, and `normal_scale` for its
depth. `emissive` makes a node glow by its own color: its base color
times the value is added to what the lights give it, so 1 shows the full color
even at night. It lights nothing around it, a point light does that, the flames
of a campfire are both. `unlit`, or `set_unlit(true)` on a node, draws the base color
times the texture as it is, with no light and no shadow on it, only the fog over it.
A photo scan carries its light in its texture already. The `Unlit` scene test
keeps a red box red under a blue sun. No tangents are stored, the shader builds the frame from the screen
derivatives of the position and the uv. A `color` with alpha below one makes the
node translucent, see below.

`metallic_roughness_map` holds metallic in its blue channel and roughness in its
green, as glTF packs them. The shader multiplies `metallic` and `roughness` by
them per pixel, so a circuit board with a factor of 1 gets shiny pads and a dull
solder mask. `occlusion_map` holds ambient occlusion in its red channel, and
`occlusion_strength`, 1 by default, scales it as 1 + strength times red minus 1.
It darkens only the sky or flat ambient light, never the sun or a lamp, as glTF
asks. Both maps are often 1 image. A material without them binds the plain white
texture, which draws exactly as before. The loader is
`hilen/src/scene/model/material_source.rs`, and the scene test `Metal roughness`
checks a quad whose 1 texture has a metal half, a rough half and an occluded half.

`cutout` cuts a node out by its alpha, leaves on a quad: `set_cutout(threshold)`,
`Material::CUTOUT` is the usual 0.5. A fragment whose alpha, the color's times the
vertex color's times the texel's, is under the threshold is not drawn. The rest is
solid: depth written, batched with the opaque nodes, lit and shadowed the same, even
with a color alpha below one. The edge is hard, a pixel is in or out.

`fade` draws a share of a node, 1 all of it and 0 none, for a level of detail or a
new tree that should not pop: `set_fade(0.4)`, or `fade_to(1.0, 0.3)` to run it over
0.3 s of scene time, stepped with the clips in `SceneBase::update_physics`. Below 1
the node goes through the cutout pipelines, and `faded_out` of `fade.wgsl`, put in
front of the mesh and the shadow shaders, drops every fragment whose 4 by 4 ordered
dither threshold, in pixels of the target, is over the fade. The rest is solid with
depth like a cut out node, and the shadow pass drops the same share in texels of
the map, so the shadow is as thin as the node. A fade of 0 is not drawn at all, 1
draws as a node that never faded. `set_fade_flip(true)` keeps the other share of the
pattern: a node fading in and a flipped one fading out at the same place, with fades
that add up to 1, cover every pixel once, a cross fade with no hole. The fade and the
flip fill the 2 words of padding `MeshInstance` had, its size stays 192 bytes. Each
comparison in `faded_out` sits in its own parentheses, a bare `a < b, c > d` in WGSL
parses as a template list.

The shading is the Filament mobile model: Lambert diffuse, GGX, the fast height
correlated Smith visibility and the Schlick Fresnel. A scene has one `sun`, a
directional light, plus any number of point, spot and directional `lights`.
`Light::directional(direction)` is a second sun, the moon at night, and
`angular_radius` gives it a disc for the occluders, see Planets and space. Every
node is drawn with the nearest eight lights whose range reaches it, picked on the
CPU each frame, a directional light first since it reaches everything, and packed
into the instance as indices into one storage buffer of lights. A
light's intensity is the brightness of a white matte surface facing it, one unit
away for a point or spot, so the Lambert term carries no `1 / pi` and the specular
one a `pi`.

`sun.shadows` makes the sun cast, off by default since it draws every opaque node
once more per cascade. The shadows are cascaded: the camera's view is cut into three
depth slices, `SHADOW_CASCADES`, and each slice gets its own orthographic map of the
sun fit around the sphere of the slice, so the near slice gets fine texels and the
far one coarse ones, see `scene::shadow`. The maps are the layers of one
`Depth32Float` array texture, `sun.shadow_map_size` texels a side, 2048 on desktop
and 1024 on a phone or in the browser, changeable at runtime. The range the slices
share runs from where the view enters the scene to where it leaves it or to
`sun.shadow_distance`, infinite by default. A big level wants a finite one, the
whole level in three maps is coarse everywhere and the biases, which scale with the
texel, then detach every shadow from its caster. Both ends snap outward to a
geometric ladder and every map's origin snaps to a texel, so a walking camera does
not shimmer the shadow edges. The passes draw before the frame's pass opens, through
the `prepare` hook of `WindowEvents`, so the frame can read the maps.

The fragment picks the nearest cascade whose map holds the point, unless that map's
texels are finer than half of what the pixel covers on its surface, then it moves on
to a coarser map that holds the point, the way a mip is picked, since a map finer
than the screen aliases a thin shadow into dots. A slice's map holds only its own
slice and a little around it, so a pixel too coarse for every map that holds it
keeps the coarsest of those, falling through to the last map instead left bands of
floor unshadowed. In the outer tenth of a map the answer blends into the next map
that holds the point, so the seam between two maps is not a line. The lookup reads
four texels around where the point lands by `textureLoad`, compares each and blends
them by distance, a depth texture cannot be filtered and a comparison sampler does
not work on iOS 12. Acne is held off by a slope scaled bias in the pass, a depth
bias of one texel and a push of up to a texel and a half along the normal that
fades as the surface turns to face the sun, so a floor under a high sun keeps its
shadows against its posts.

The maps are built around the camera, not the world's origin. A cascade's view
projection takes a point minus the camera's position, and both passes work out that
offset with small numbers, see the next section. With world numbers, a few
kilometers out the 32 bit floats lost about a millimeter, the bias of the fine map,
and the ground shadowed itself in dots that grew the farther the camera went.
`Terrain shadows` and the unit tests in `scene/shadow.rs` hold this. Translucent
nodes receive but do not cast. A cut out
node casts the shadow of what is drawn: its batches go through the cutout twins of
the shadow pipelines, the only ones with a fragment stage, which read the same
instance, vertex color and texel alpha and drop the same fragments.

`fog` is distance fog, `Fog::new(color, start, end)`: every surface blends towards
the color with its distance from the camera, untouched up to `start` and wholly fog
from `end` on, in linear light before the tonemap. The sky is fog colored at the
horizon and clears with height, gone where the view rises past `height`, 0.4 by
default, and without a sky the fog color fills the background, so fogged ground
always meets fog above it.

`sky` is a cube map, `Sky::gradient` for a smooth one and `Sky::from_faces` for six
images. The skybox draws it behind everything and every surface reflects it: the
diffuse through nine spherical harmonics and the specular through one mip per
roughness step, both prefiltered on the CPU in `hilen-pixels` when the sky is
made, the split sum with the Karis fit in place of a lookup table. Without a sky the
flat `ambient` color stands in. Highlights roll off through the Khronos PBR Neutral
compression without its black offset, so a color under the knee lands on screen as
the hex it was written as.

`day_night_sky` is a sky the shader draws every frame, so a game can move it with its
clock, see `DayNightSky`: a blue gradient that darkens with `night`, an orange glow on
the horizon towards a low sun by `sunset`, the sun disc and its glow, and at night
stars from `star_seed` twinkling by `twinkle` and the moon disc with its glow. It is
drawn in place of the cube `sky`, which still lights the scene when both are set, and
fog blends it at the horizon the same way. It lights nothing itself, the game moves
`sun` and `ambient` with it. Its values reach the sky shader as three vectors of
`SceneUniform`.

## Planets and space

A planet is a sphere in the scene, and the engine keeps it there with its own
math in 32 bit floats around the camera. A game keeps its true places in 64 bit
and moves the scene to them, the bubble of geblings, see its `docs/planet.md`.

- `move_world(WorldMove { turn, shift })` moves and turns everything of the scene
  by 1 rigid move in 1 call: every node with its speed and spin, the player, the
  camera, the gravity and its wells, the sun, the lights, the occluders, the turn
  of the stars, the lines, the wind, the handle and the frames of the grass, in
  `scene/scene/scene_move.rs`. A game calls it when the player gets far from the
  origin, so the floats stay fine. An offset collider turns with its wall. The
  scene test `Moved world` checks a scene before and after a move.
- `set_wells` puts `GravityWell`s, point masses with a `gm` and a `radius`, in
  place of the 1 uniform gravity, `scene/gravity.rs`. `gravity_at` sums their pull
  at a point. Every awake dynamic body gets that pull as an impulse each substep.
- The player's up follows the pull. `Player::frame` is the least turn that puts
  its y against the pull, and `up`, `forward` and `right` come from it, so a walk,
  a jump, a fall, a flight, the eyes and the third person pivot work on any side of
  a planet. `Planet walk` walks a player round a small planet.
- `occluders` holds up to 4 `Occluder` balls, a center, a radius and a `transmit`
  color, black by default. Each pixel, and each step of the air, works out what
  share of a light's disc each ball hides, `light_through` in `atmosphere.wgsl`.
  That 1 test gives the night side with its soft edge, a solar eclipse under the
  moon, a lunar eclipse in the planet's shadow and the shadow of a planet on its
  air. A ball is skipped by a cheap cosine bound where the disc lies wholly past its
  edge or wholly behind it, where the full test gives exactly 0 or 1. `Night and
  eclipse` and `Moonlight` check them.
- `space_sky` with a `SpaceSky` draws the sky of a world in space in place of `sky`
  and `day_night_sky`: the stars, turned by `stars`, the sun's disc of
  `Sun::angular_radius`, and with an `Atmosphere` the air, from the ground and from
  space. `Atmosphere::earth` is Earth's air, `Atmosphere::scaled` shrinks it to a
  small planet with the same colors from the ground. The air is Hillaire 2020: 2
  tables built on the CPU in `scene/atmosphere.rs` when the shape of the air
  changes, the transmittance and the multiple scattering, uploaded as half floats
  by `render/pipelines/air_textures.rs`. The sky marches 24 steps a pixel, the
  haze over a surface 8, in place of the fog, and the ambient light comes from the
  air on the CPU. A step over 8.75 heights of both hazes and over the ozone is
  skipped, under 1 color step, and a step in full shadow only dims what lies behind
  it. `Planet sky` shows the sky from the ground at noon, at sunset, at night and
  from 60 km out.
- `Material::dust`, `set_dust`, reflects by the Lommel-Seeliger law of moon dust, so
  a full moon is flat and bright to its edge. `Moon dust` checks it.
- `Grass::origin` and `Grass::turn` put a field on the side of a planet, the field
  works in its own axes, `to_field` and `to_world`. `Grass on a planet side` grows
  one on a slope of a ball.

A planet from space in a window of 4096 by 2304 took 39 to 48 ms of GPU work a
frame on an M1 Pro, 90 percent of it the air. Drawing the sky after the opaque nodes
and the 3 skips above brought it to 9.6 ms with no pixel moved by more than 1
color step, measured with Metal System Trace in geblings on 2026-10-29.

## Picking

A touch that no view takes falls through to the scene, the way a level gets one.
`Camera::ray` turns the pixel into a `Ray` from the camera, `Shape3::hit` tests it
against a node's solid, a ball by its surface and everything else by its collider
box, so a model is hit on its bounds. The nearest node gets `on_touch` with the
world point hit, then the scene's `on_tap` fires with the ray, hit or not.
`node_at` and `ray` on the scene answer the same question without a touch.

The other way round, `Camera::screen_point` gives the pixel a world point lands on,
and `view_point` on the scene the same place in points of the root view, where a view
centered sits over it, a damage number or a name over a node. Both are `None` for a
point behind the camera, so the view can hide. A scene test puts such views over its
scene in `SceneTest::overlay`, which runs in the test and in presentation.

A scene takes the arrow keys and Enter, so the key focus is off while it runs. A scene
that only draws behind the views answers false in `SceneSetup::takes_keys`, see
[focus.md](focus.md).

## Player

`add_player` puts a first person `Player` in a scene with physics: a capsule on
rapier's kinematic character controller, with gravity, small steps, a jump on
space and a push on the bodies it walks into. `w` `a` `s` `d` or the arrows walk it
while held, read through `Keys::held`, and `look` turns it, the demo page calls
that from a drag. While a player exists the camera looks out of its eyes.
`add_player` makes a capsule 1.8 tall and 0.7 wide, `add_player_sized` takes the
radius and the height for a smaller or bigger hero. `walk_keys` set to `WalkKeys::WasdOnly`
leaves the arrow keys to the game. `teleport` puts the player
somewhere at once with no fall left, the way a game respawns it. A rising player is
never grounded and never snapped down, so a jump is not ended in its first step while
the capsule is still within the controller's offset of the ground.

A scene steps by the real time of its frames, measured on `frame_seconds`, at most
1/15 s a frame, each frame in `PHYSICS_SUBSTEPS` parts. A fixed step of 1/60 s a frame
ran the whole scene in slow motion under 60 frames a second. A stepped scene test keeps
the fixed 1/60 s, so its frames stay exact.

`motion` set to `Motion::Fly` makes the player fly, a jet or a free camera: no gravity, no jump, no
snap to the ground and no step up ledges. `w` and `s` move it along its whole
`direction`, the pitch included, `a` and `d` to the side, space straight up and ctrl
straight down. It still slides along what it hits, also at 300 m/s into a hill.
Back to `Motion::Walk`, it falls from where it is. The unit tests in `scene/player.rs` fly a
player up, hold it still, stop it at a hill and drop it.

`steer` set to a direction moves the player from code in place of the keys, for an
autopilot or a player a test drives. It moves the same way a held key does, at `speed`
along the direction and with every collision. A walking player takes it level, a flying
one takes it whole, so a climb or a sink is in the direction. `None` gives the keys back.
`teleport` every frame is no way to drive a player, the next physics step puts it back.

`Cursor::capture()` takes the mouse the way a game does: hidden, held inside the
window, and reported as raw motion, which the player reads every step through
`Cursor::take_motion` and turns by `look_speed` radians per unit. Escape frees the
mouse and nothing else sees that Escape, so a page can bind its own Escape to
capture again, a lost window focus frees it too, and `Cursor::on_capture` tells a
page when to hide or show its controls. Windows and X11 cannot lock the cursor in
place, there it is confined to the window, which looks the same once hidden. The
browser needs a click before it locks and lets go on its own Escape, both reach
`Cursor` through the pointer lock events. A phone has no mouse and `capture` does
nothing there.

`third_person` on the player puts the camera over its shoulder instead of in its
eyes, see `ThirdPerson`: it turns around a pivot above the capsule with the yaw
and pitch, sits `distance` behind and `shoulder` to the right, and looks where the
player looks. Every step a ray from the pivot to that spot pulls the camera to
`margin` in front of the first collider on the way, never closer than
`min_distance`. Nodes in `skip` are looked through, like a swing's hitbox. The
player is no node, so a game draws its body itself and moves it to
`Player::position` in the scene's `update`.

## Meshes from code

`Model::from_mesh(name, MeshData)` uploads geometry built in code, a terrain or a
procedural prop, and stores it under `name` like a loaded file, so `Model::get`
returns it too. The indices are `u32` and a mesh of any size is split into 16 bit
parts, `model/split.rs`, the same split a big glTF primitive gets. It has no
material, the node's own draws it, and it collides and picks as the box around
its vertices unless a collider is put on.

`Vertex3D::color` multiplies the node's material color, white by default, so a
mesh built from parts of different colors is one mesh. A triangle takes one
color: the shader passes it packed and flat, the seventh of the eight values an
A7 lets cross from the vertex to the fragment, so `from_mesh` panics when the
three corners of a triangle differ. A full channel passes as exactly one, the
sRGB decode of 255 lands a hair under it in f32 and moved pixels of every mesh.

## Colliders, queries and parents

A node collides as its drawn shape until `set_collider(ColliderShape, offset)`
puts on a box, ball, capsule, cylinder, heightfield, triangle mesh or voxels.
`Heightfield` is a grid of
heights, row by row along +x, rows stepping along +z, centered on the node, and
`Heightfield::mesh` builds the matching ground cut along the same diagonal rapier
cuts each cell, so what is drawn is what bodies rest on. `cut_hole(row, column)`
takes the ground out of 1 cell, in the collider and in the mesh, so a cave mesh can
take its place. `TriMesh` collides as the triangles of a `MeshData`, with shared
corners merged and the inner edges fixed so a body does not catch on them. 2
triangle mesh shapes are equal only when they share 1 mesh. `Voxels` are cubes of
1 size at integer cells. `show_colliders` draws every kind, a heightfield as a
coarse grid, a triangle mesh as its edges and voxels as their cubes.

`cast_ray`, `cast_shape` and `overlapping` on the scene answer what a ray or a
swept or placed shape meets, as a `QueryHit` with the node, the distance, the
point and the normal. Nodes in `skip` and the scene's player are left out.

`attach_to(parent)` hangs a node on another: its position, rotation and scale
become the parent's local ones, down any chain, and `detach` keeps it where and
as big as it is in the world. `world_scale` is the scale times every parent's. A
child's collider moves and grows along every step. A `Body` cannot hang on a
parent and a loop of parents panics.

## Pictures

`SceneManager::picture(scene, name, size, background)` draws a scene once into a
managed `Image` of `size` pixels, a picture of a model for a grid or a list. The
scene is set up, drawn with its own camera, sun, lights, sky and nodes, and
dropped. The running scene and the frame on screen are not touched, and none has
to run. `background` shows where nothing is drawn, `CLEAR` leaves the picture see
through, a sky or fog covers it. The image is keyed by the name and the size, the
same pair draws into the same image again, so a view over it shows the new
picture with nothing set on it. A picture has no time, so its scene has no
physics, it is made of `Prop` nodes, anything else panics.

`SceneDrawer::picture` gathers the scene into the one shared `MeshPipeline` with
its own encoder and submits it at once, then `forget_frame` empties the
pipeline, so the frame's own draw on the same frame number does not draw the
picture's nodes again. For the same reason `MeshPipeline::prepare` zeroes the
range of every batch nothing was added to. The pass draws into targets of the
picture's size in the surface format with the frame's sample count, the
pipelines are the frame's own, and resolves. `PicturePipeline` then copies the
resolved frame into the image's RGBA texture with the color divided by its
alpha: over a clear background the frame holds color times coverage, from the
multisample resolve and from every blended node, and an image draws with
straight alpha, so without the divide every outline gets a dark rim. The
targets are kept for the next picture of the same size. 50 pictures of 256
pixels take about 18 ms of CPU time in a debug build and 12 ms in release on an
M series Mac, models and textures are managed and load once.

A picture has 1 level, so drawn much smaller than its size it picks scattered
texels and flickers. `SceneManager::mipped_picture(scene, name, size, background,
cutout)` draws the scene through the photo path, reads the pixels back and builds
every level on the CPU with `hilen_pixels::mip_chain`, an atlas of far trees for
example. A native GPU is waited for in the call and the image made on the calling
thread, call it on the main one. A browser gives the pixels after a frame of the
loop, so the result is a future to `spawn`. With a `cutout` threshold,
`keep_coverage` scales the alpha of every smaller level so the share of texels at or
over the threshold stays that of the full size, found by halving a scale in steps of
1/256 and taking the side of a step nearer to it. A plain box filter thins far leaves to
nothing. The image is stored under `name`, a second call replaces it. A 1024 pixel
atlas of 64 trees takes about 1.4 s in release, most of it the read back and the
levels.

## Photos

`SceneManager::photo(camera, size, background)` draws the running scene through
another `Camera` and gives its pixels back, a `ScenePhoto` with `size` and `rgba`,
4 bytes a pixel from the top row, straight alpha. It is for a virtual camera in
a scene, like the phone of a 3D scanner rig that takes 4032 x 3024 photos while
the rig shows live on screen. The scene's own camera does not move and the frame
on screen does not change. It works for the scene of the window and for one in a
`SceneView`.

```rust
let photo = SceneManager::photo(phone_camera, (4032, 3024), BLACK);
spawn(async move {
    match photo.await {
        Ok(photo) => save_jpeg(&photo.rgba, photo.size),
        Err(err) => log::error!("no photo of the rig: {err}"),
    }
});
```

The draw is sent in the call, so photos taken in a row each show the scene at
their own call. On native the future waits for the GPU and reads the bytes, in a
browser it ends after the next frame of the loop. No scene, a size of 0, a side
over the GPU limit, 8192 in a browser, or a buffer too big for the GPU is an
error.

`SceneDrawer::photo` draws through the same targets and `PicturePipeline` as a
picture, into an RGBA texture kept for the next photo of the same size, and in
the same encoder copies it into a buffer through `State::read_screen`, the path
of a screenshot. A 4032 x 3024 photo takes 9 to 16 ms in a debug build on an M
series Mac. Its targets with 4 samples take about 450 MB of GPU memory and stay
for the next photo, which is not measured on a phone yet. The scene tests
`Photo of scene` and `Photo of scene in view` check 6 known pixels of a big
photo, take 24 small ones on a full turn, and check that the screen is the same
before and after.

`ScenePhoto::save(path)` writes a photo to a png with its alpha, so a photo whose pixels
a depth photo cut away keeps them clear. `Screenshot::save` writes every pixel opaque.

`SceneManager::depth_photo(camera, size)` draws the same way and gives a
`SceneDepth`, the distance in meters from the camera to the surface under every
pixel along the line of sight, infinity where only the sky, a translucent node or
nothing is. The turnscan simulator blurs its phone photos by it, the way a lens out
of focus does. A multisampled depth buffer cannot be read on WebGL2, so the nodes
are drawn a second time through the fragment entries `f_depth` and
`f_depth_cutout` of `mesh.wgsl`, which write the distance as 24 bits in red, green
and blue on a log scale from 1 mm to 10 km, white for no surface. Inside a surface
it comes back to a millionth of itself. On an edge the resolve mixes the samples,
and the pixel lands within about 3 percent of their mix. The 4 pipelines are made
at the first depth photo, an app that takes none does not wait for them. The scene
test `Depth photo of scene` checks the distance of a cube face, 2 floor points and
the sky against the camera, then a plain photo and the screen after it.

## Grass

`SceneBase::grass` holds fields of grass that grow around the player, or
around the camera with no player, `scene/grass.rs`. `Grass::new(model,
ground)` takes the mesh of 1 clump and a function that answers what grows at
a point of the ground: a `GrassSpot` with the height, the ground's normal,
which the clump stands along, and a tint, or none for no grass there. The
field grows in square patches, `patch_size` 10 by default, the nearest
first, and asks `ground` at most `spots_per_frame` times a frame, so a
streamed world grows its grass over a few frames. A patch out of reach goes.
`regrow` and `regrow_around` let patches grow again after the ground or the
answer of `ground` changed, a chunk that loaded or a floor built on it. The
same seed and the same cell give the same blades, and every random draw is
taken for every spot, so a hole in a field leaves the blades around it in
place.

A patch is 1 vertex buffer of `GrassBlade`s, 32 bytes each: the root from
the patch's origin, the turn about up, the scale, the tint, the ground's
normal as 2 snorm16 and the patch's slot. Each frame the CPU culls whole
patches by their sphere and sends 1 `MeshInstance` and 1 matrix per patch,
in a slot the patch keeps while it lives, a blade names its slot since an
A7 cannot draw from a base instance. Each patch is 1 instanced draw of
`v_grass` and `f_grass` in mesh.wgsl, after the opaque nodes. The fragment
is lit, shadowed and fogged like a node through the same `lit`, cuts out by
the material's `cutout` and drops everything within 2 units of the camera,
so a blade never fills the view. Grass casts no shadow.

Up to `thin_from` every blade is drawn, to `distance` they go 1 by 1. The
blades of a patch lie in random order and blade `rank` is gone past
`thin_from + (1 - rank / count) * (distance - thin_from)`, shrinking away over
the last 15 percent of that, so the edge of a field has no line and no
blade pops. The CPU leaves out the blades already gone at a patch's nearest
point.

`Wind` on the scene, a direction on the ground and a strength from 0 to 1,
moves every field. A point bends by its height in the mesh over
`bend_height`, a straight line from 0 at the root. The wind leans it along
the wind by `0.4 * sway_distance * strength` times that bend, and a gust
swings it, a wave running along the wind times 3 sines across and along it.
`Grass::advance` runs the 4 phases on with the scene's time and keeps each
under a full turn, so the sway never jumps however long a scene runs. The
player pushes grass within 1 unit of the line from where its feet were a
moment ago to where they are, so a walk leaves a trail, out from the player
and down by `push_distance`, only above its feet. After the wind and the
push every point is put back at its distance from the root, so a blade
swings as if hinged there and never stretches.

`Grass::blade` and `Grass::tuft` build simple blade meshes. A clump of cards
cut out of a texture of thin blades in its alpha looks far better from a
distance: `Image::from_cutout_data` makes such a texture from RGBA bytes
with mip levels that keep the share of texels over the cutout, a box filter
thins far blades to nothing. Give the cards normals straight up, so a field
is lit like the ground under it.

The scene test `Grass field` grows a field of wide blades with a bare square
and a tinted half, leans it in the wind after an exact count of steps, and
grows it again 300 m away. The unit tests in `scene/grass.rs` hold the
planting, the ring order, the thinning count and the meshes.

## How it draws

A scene over the whole root area draws in the main render pass, before the level
and the UI. A scene in a `SceneView` draws into the image of the view instead, see
above. The depth is reversed, 1 on the near plane and 0 at `far`, where 32 bit
floats are densest, compared with `Greater`, and `Camera::far` may be
`f32::INFINITY` for no far plane, so a stone 1 m away and a moon 300 km away share
1 frame. The scene draws in the whole depth range, so `MeshPipeline::fill_depth`
writes 0 over the viewport before it and 1 after it, and the UI over it tests depth
as before, `depth_fill.wgsl`. `Near and far` checks both ends.

Everything draws relative to the camera, so a world kilometers big keeps its
millimeters far from its origin. `SceneUniform::view_proj` and `inv_view_proj` take
a point minus the camera's position, built in 64 bit on the CPU. A vertex turns by
its model first and adds the model's place minus the camera's after,
`model_offset` in scene_common.wgsl, both small near the camera. A fragment gets
its offset from the camera from `camera_offset`, and the world position is the
camera plus that offset. Lines subtract the camera before their view projection.

`MeshPipeline` does one instanced indexed draw per unit mesh and texture pair for
the opaque nodes, then the grass, then the sky, one triangle at the far depth of 0
with an `Equal` test, so it shades only the pixels no node covers. The air makes
the sky the dearest shader of a frame, drawn first it ran under the whole planet.
Then one draw per translucent node, back to front, blended and without a depth
write. A cut out node is an opaque batch of its own, `MeshKey::cutout`, drawn
through pipelines whose fragment entry `f_cutout` discards, so a plain opaque draw
carries no discard. An opaque node writes alpha 1 through `f_opaque`, the alpha of
its texture would leave a hole in a scene picture. Every opaque batch lands in one instance buffer with one upload per frame. A
buffer per batch cost a staging buffer per batch every frame. The instance carries the
model matrix, the inverse transpose for normals and its own index in the buffer, see
`MeshInstance`, so a batch or a translucent node drawn from a slice in the middle of
the buffer needs no base instance, which an A7 cannot draw. The
fragment reads the material and the light list from a storage binding at that
index. Seven float components cross the vertex to fragment boundary, the uv, the
normal, the flat index and the flat packed vertex color, and the world position is
rebuilt from the depth. An A7
draws nothing above eight, see [ios.md](ios.md). The cutout shadow pass carries
four, the uv, the index and the color. Every mesh buffer loads once per
frame, so the bind groups over the view, the lights, the instances and
each key's textures are kept from frame to frame and remade only when a buffer
grows, a count changes, the sky or the shadow map is replaced or an image dies,
see `render/bind_cache.rs`. Indices are 16 bit so every lane
draws them. Back faces are culled, so every primitive is wound counter clockwise
seen from outside, pinned by unit tests on the geometry.

Each pass leaves out the opaque nodes it cannot see. A node keeps the sphere around it,
its center and the length of its half extents, and `prepare` loads 1 range of instances
per pass into each batch: 1 for the camera and 1 for each cascade of the sun. A pass
draws only its own range and skips a batch whose range is empty. The test is against the
same view projection the GPU clips with, `render/frustum.rs`, so a node it leaves out
would have drawn no pixel, and the picture stays the same. A cascade tests its own box,
so a node behind the camera still casts its shadow into the view, the scene test
`Shadow out of view` pins that and fails when a cascade culls by the camera. A skinned
part and a part of a playing clip are never left out, their rest bounds do not hold the
pose. Translucent nodes are not culled. `HILEN_CULL=0` in the env draws every node in
every pass, the A/B for a measurement. In geblings, standing in a forest with the sun
held still, a headless frame went from 2.43 ms to 1.44 ms.

`GpuCapture::frames(count, path)` captures the GPU work of the next frames into a
`.gputrace` file that Xcode replays with the time of every pass and draw,
`window/gpu_capture.rs`. Metal only, and the app must start with
`METAL_CAPTURE_ENABLED=1`. Metal System Trace in Instruments times whole command
buffers only, and a headless run lowers the GPU clock to its free frame rate, so
only a run in a window in front, with the GPU full, measures a shader's cost.

Colors are encoded sRGB like the whole frame, see [colors.md](colors.md). The
shader decodes, lights in linear, tonemaps and encodes at the end of the fragment.
Textures and the sky cube hold encoded bytes too and are decoded after the sample.

`show_colliders` on the scene draws every collider as a green wireframe after the
nodes: the twelve edges of a box or of a model's bounds, three rings on a ball,
nothing for a plane, whose slab is the floor itself. The lines go through a small
line pipeline on the same view uniform, depth tested against the nodes, and sit a
little outside their solid so the faces do not cut them.

`show_wireframe` on the scene draws the edge of every triangle of every node as a
white line over the scene. A mesh keeps its triangle indices and builds its edge
index buffer the first time a wireframe draws it, each edge of the triangles once,
so the lines cost nothing on the CPU per frame. Built at every upload they took a
third of the main thread of a game that streams its ground. The wireframe pipelines are the mesh pipelines with a line list, made at
the first frame that shows a wireframe. They use the same vertex entries and
instances, so a model and a skinned mesh draw their edges where they stand. An edge
lies on its own faces, so the fragment moves its depth 0.2 % of its distance toward
the camera, depth bias is for triangles only in WebGPU. A hidden edge stays hidden,
the edges of a translucent node show through it.

## Tests

A scene test is a `#[scene]` with `impl SceneTest`, registered by a ctor into
`hilen::SCENE_TESTS` behind the `scene-tests` feature and run by the `scene-test`
crate, the tests live in `scene-test-suite`. Same flags as `level-test`, `make scene` runs
the suite. `demo` links `scene-test-suite` too, so `make ui-ios` and `make ui-web` run
the scene tests after the UI tests. The three that cast shadows pin
`shadow_map_size`, the default is 1024 on a phone and in the browser, and the
lanes would draw a softer shadow edge than the desktop recorded.
`Primitives` orbits the camera around every shape, `Materials` is the
metallic by roughness chart, `Lights` a point and a spot light, `Textures` a
texture and a normal map, `Skybox` chrome under a sky, `Transparency` blended
balls from both sides, `Models` the monkey, the tree
and the textured cube from `assets/models` with the monkey dropped onto its
bounds, `Shadows` a post, a ball, a floating crate and the monkey under a low sun,
`Picking` taps landing on the nearest node and on the sky, `Player walk` a
player shoving a crate into a wall and jumping, `Animations` a skinned bar
bending, the fox running and a windmill spinning, checked at rest, frozen mid
clip and held after a single run, `Cascades` a thin pole and a row of posts down
a field hundreds of units long with the camera walking it, its middle check on
the spot where a floor pixel too coarse for the maps holding it once fell through
to the sun, `FogTest` posts fading into fog under a sky fogged at the horizon,
the fog then pushed back and taken away, `Colliders` the wireframes off and on
over a ball, a turned crate, the monkey and a prop without one, `Wireframe` the
edges of a ball, a crate, the monkey and a translucent ball off and on, and `Mouse look` a
player turned by a stream of captured mouse motion onto a crate, then left alone
by the same stream once Escape freed the mouse. `Code meshes` is a terrain split
into parts under a knot, a lathed vase, a twisted star and a rock, all built in
code, `Vertex colors` a camp of meshes colored face by face, `Collider shapes`
bodies of every collider shape resting on heightfield hills, `Scene queries`
rays, a sweep and a hitbox against static walls, `Node parenting` a knight whose
arm, sword and hitbox follow its swing, walk and growth, and `Third person
camera` a figure turning so a wall pulls the camera in. `Day and night` a field under
a `DayNightSky` at noon, in the afternoon, at sunset, at dusk and at night with the
moon, and `View points` labels over a circling ball and a post, the post's hidden once
it is behind the camera. `Scene picture` a tree in a
running scene next to a picture of it in an `ImageView`, the same image drawn again
with the monkey, then 50 more pictures with the frame unchanged, and `Scene picture
clear` one picture with a clear and with a given background over a light and a dark
view while no scene runs. `Glow` orange flames at night glowing by 0, 0.5 and 1, then the dark one
turned to full glow. `Cutout` quads of leaves over a wall, not cut, cut out, 2
crossed that hide each other by depth and the masked `leaf_card.glb`, then the
threshold lowered so the leaves grow, and `Cutout shadows` the same quads over a
floor, a square shadow from the one not cut and leaf shadows from the rest.
`Node fade` cubes over a wall at fades 0, 1/4, 1/2, 3/4 and 1, a blue and a flipped
red cube at a half each at the same place with no wall between, and a cube that runs
`fade_to` from 0 to 1 and stays whole. `Fade shadows` 3 cubes over a floor at 1, a
half and 0 under a sun that casts, a whole shadow, a dithered one and none. `Mipped
picture` the same picture of a tree of balls on cut out quads at full size, a
quarter and a 16th, plain on top and mipped below. `Moved world`, `Planet walk`,
`Near and far`, `Moonlight`, `Night and eclipse`, `Grass on a planet side`, `Moon
dust` and `Planet sky` are in Planets and space above.
`Terrain shadows` is a streaming world as heavy as a game: 5 by 5 chunks of steep
ground with 10 trees and 4 rocks each come and go around a flying jet, over a coarse
ground and a sea 2400 m wide, under a day sky with shadows and fog. The test flies the
jet to 4 places up to 19 km from the origin under 2 suns, and the shadows hold no dots
anywhere. In `--present` the jet flies with `w`, space, ctrl, shift and a drag.
`Scene in view` runs its scene in a tall `SceneView` next to a panel with a label
over it, picks the ball with a tap, turns and zooms the camera, narrows the view
and removes it. `Orthographic camera` shows 3 cubes at 3 depths as 3 squares of 1
size, the same camera in perspective, a view from above with shadows, a tap on the
far cube and a zoom. `Scene lines` has a grid, the axes, the edges of a part that
turn with it, a line hidden by a ball and one on top, a line through the near plane
and the same lines seen orthographic. `Triangle picking` taps the 2 parts of an L
built as 1 mesh and the empty corner of the L. `Transform handle test` drags the x
arrow, the y ring with a snap and the z box of a handle on a crate, pulls the camera
back and drags off the handle. A scene test leaves its scene on screen through the final
human hold, the next test or the end of the run stops it. The loop runs free, so the frames
between two waits vary by one. A check of a pose in flight freezes the clip at a
chosen time through `set_animation_speed(0)` and `set_animation_time` first. A
human hold pauses the scene's time, so the probes sit on a still picture.
`hold_key`, `release_key` and `inject_mouse_motion` drive the player from a test.

A test never checks a picture after simulated physics. Where a body comes to
rest is the result of rapier, and a new rapier release moves it by a few
pixels: the jump from 0.33 to 0.36 failed every recorded picture of that kind
while the scenes were right. Such a test reads values from the scene and checks
them with a tolerance, a body rests on the ground under it, the wall stopped
the player, the camera sits behind the figure, and it holds at each state with
`checkpoint(label)`, so a human run still stops there. `Collider shapes`,
`Player walk`, `Mouse look` and `Third person camera` work this way. A test
that only shows that bodies fall and come to rest tests rapier and not the
engine, so there is none.

A test that reads where physics ends up returns true from `SceneTest::stepped`.
Its scene stands still from the setup on and `step_scene(n)` moves it by exactly
`n` steps, so every lane takes the same number. Three things made a rest land
elsewhere per lane, each found by printing the positions as raw bits on desktop,
in Chrome and on the x86_64 simulator. The free running loop takes another
number of steps between two waits on every lane. glam's SIMD sums in another
order on arm64, x86_64 and wasm, so `scene-tests` turns on `glam/scalar-math`
next to rapier's `enhanced-determinism`. And `cos` and `sin` of the system
differ in the last bit, so a test writes a start velocity out as numbers. With
all three the player and the crate of `Player walk` end on identical bits on
the three lanes. `parallel` in rapier was not a cause, desktop gives the same
result with and without it.

```bash
cargo run -p scene-test -- --list
cargo run -p scene-test -- --headless --test-name Primitives
cargo run -p scene-test -- --test-name PlayerWalk --human
cargo run -p scene-test -- --test-name Materials --present
```

`--present` hands one scene over on the test's own canvas at scale 1, the frame a
test sees, with a drag to orbit, the wheel to zoom and `w` `s` `a` `d` or the arrows
walking the camera level, or with a player in the scene the drag turns its head and
the keys walk it.

## What is next

The remaining deliveries are in [roadmap.md](roadmap.md). A scene in a view is above,
and only 1 scene runs at a time.
