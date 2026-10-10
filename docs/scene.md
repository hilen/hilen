# 3D scenes

The `scene` module is the 3D twin of `level`, behind the `scene` cargo feature. A
`#[scene]` struct gets a `SceneBase` injected, `SceneManager` runs the one active
scene, nodes are `Own<dyn Node>` the way sprites are, and `SceneDrawer` draws them
with `MainLock` pipelines on `VecBuffer`. Physics is `rapier3d`, math is glam
re-exported from `gm::volume`. The scene draws every frame while one is loaded,
like a level.

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

## Models

`Model` is a `.glb` on the GPU, a managed resource like an `Image`. It loads from
`assets/models` through `filesystem::read_bytes`, so the APK and the browser
manifest serve it like any asset, and `Model::get("tree.glb")` returns the one
copy. `Shape3::Model(model)` puts it on a node. The meshes, the metallic roughness
materials, the embedded base color and normal textures and the node tree load, the
tree flattened into parts with their placements, so a node draws every mesh of the
file at its own size and with its own materials. `set_scale` sizes a node uniformly
on top of that, so a model in other units fits the scene, the collider and picking
follow. A primitive without a material
takes the node's `Material`. A material with `alphaMode` `MASK` loads as a cutout
with its `alphaCutoff`, `BLEND` as translucent. `doubleSided` is not read, back
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
of a campfire are both. No tangents are stored, the shader builds the frame from the screen
derivatives of the position and the uv. A `color` with alpha below one makes the
node translucent, see below.

`cutout` cuts a node out by its alpha, leaves on a quad: `set_cutout(threshold)`,
`Material::CUTOUT` is the usual 0.5. A fragment whose alpha, the color's times the
vertex color's times the texel's, is under the threshold is not drawn. The rest is
solid: depth written, batched with the opaque nodes, lit and shadowed the same, even
with a color alpha below one. The edge is hard, a pixel is in or out.

The shading is the Filament mobile model: Lambert diffuse, GGX, the fast height
correlated Smith visibility and the Schlick Fresnel. A scene has one `sun`, a
directional light, plus any number of point and spot `lights`. Every node is drawn
with the nearest eight lights whose range reaches it, picked on the CPU each frame
and packed into the instance as indices into one storage buffer of lights. A
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
shadows against its posts. Translucent nodes receive but do not cast. A cut out
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
`SceneView`.

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
somewhere at once with no fall left, the way a game respawns it.

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
puts on a box, ball, capsule, cylinder or heightfield. `Heightfield` is a grid of
heights, row by row along +x, rows stepping along +z, centered on the node, and
`Heightfield::mesh` builds the matching ground cut along the same diagonal rapier
cuts each cell, so what is drawn is what bodies rest on. `show_colliders` draws
every kind, a heightfield as a coarse grid.

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

## How it draws

The scene draws in the main render pass, before the level and the UI, into the
viewport depth band 0.6 to 1.0. The UI draws at 0.5 and closer, a level sprite at
0.85, so the UI stays in front of any scene and a level can share the frame. The
band costs about one and a half bits of depth precision, the camera near plane
matters far more, keep it at 0.1 or above.

The sky draws first, one triangle with no depth test. `MeshPipeline` then does one
instanced indexed draw per unit mesh and texture pair for the opaque nodes, and one
draw per translucent node after them, back to front, blended and without a depth
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

Colors are encoded sRGB like the whole frame, see [colors.md](colors.md). The
shader decodes, lights in linear, tonemaps and encodes at the end of the fragment.
Textures and the sky cube hold encoded bytes too and are decoded after the sample.

`show_colliders` on the scene draws every collider as a green wireframe after the
nodes: the twelve edges of a box or of a model's bounds, three rings on a ball,
nothing for a plane, whose slab is the floor itself. The lines go through a small
line pipeline on the same view uniform, depth tested against the nodes, and sit a
little outside their solid so the faces do not cut them.

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
over a ball, a turned crate, the monkey and a prop without one, and `Mouse look` a
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
floor, a square shadow from the one not cut and leaf shadows from the rest. A scene test leaves its scene on screen through the final
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

The remaining deliveries are in [roadmap.md](roadmap.md): an embeddable live
scene view, a picture is the still one, and culling nodes outside a cascade's box on the CPU so a short shadow
distance also cuts the shadow passes on a big level.
