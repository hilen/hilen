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
takes the node's `Material`. The collider is the box around the model's `bounds`,
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
with `blender --background --python build/export_glb.py -- assets/models/tree.blend`,
skins and actions included, and the script also mends what the old files miss. Only the `.glb` files
reach the browser manifest.

## Materials and lights

Every node has a `Material`: `color`, `metallic`, `roughness`, an optional base
color `texture` that multiplies the color per texel, an optional `normal_map` in
tangent space with green up, the glTF convention, and `normal_scale` for its
depth. No tangents are stored, the shader builds the frame from the screen
derivatives of the position and the uv. A `color` with alpha below one makes the
node translucent, see below.

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
shadows against its posts. Translucent nodes receive but do not cast.

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

## Picking

A touch that no view takes falls through to the scene, the way a level gets one.
`Camera::ray` turns the pixel into a `Ray` from the camera, `Shape3::hit` tests it
against a node's solid, a ball by its surface and everything else by its collider
box, so a model is hit on its bounds. The nearest node gets `on_touch` with the
world point hit, then the scene's `on_tap` fires with the ray, hit or not.
`node_at` and `ray` on the scene answer the same question without a touch.

## Player

`add_player` puts a first person `Player` in a scene with physics: a capsule on
rapier's kinematic character controller, with gravity, small steps, a jump on
space and a push on the bodies it walks into. `w` `a` `s` `d` or the arrows walk it
while held, read through `Keys::held`, and `look` turns it, the demo page calls
that from a drag. While a player exists the camera looks out of its eyes.

## How it draws

The scene draws in the main render pass, before the level and the UI, into the
viewport depth band 0.6 to 1.0. The UI draws at 0.5 and closer, a level sprite at
0.85, so the UI stays in front of any scene and a level can share the frame. The
band costs about one and a half bits of depth precision, the camera near plane
matters far more, keep it at 0.1 or above.

The sky draws first, one triangle with no depth test. `MeshPipeline` then does one
instanced indexed draw per unit mesh and texture pair for the opaque nodes, and one
draw per translucent node after them, back to front, blended and without a depth
write. The instance carries the model matrix, the inverse transpose for normals and
its own index in the buffer, see `MeshInstance`, so a translucent node drawn alone
from the middle of the buffer needs no base instance, which an A7 cannot draw. The
fragment reads the material and the light list from a storage binding at that
index. Six float components cross the vertex to fragment boundary, the uv, the
normal and the flat index, and the world position is rebuilt from the depth. An A7
draws nothing above eight, see [ios.md](ios.md). Indices are 16 bit so every lane
draws them. Back faces are culled, so every primitive is wound counter clockwise
seen from outside, pinned by unit tests on the geometry.

Colors are encoded sRGB like the whole frame, see [colors.md](colors.md). The
shader decodes, lights in linear, tonemaps and encodes at the end of the fragment.
Textures and the sky cube hold encoded bytes too and are decoded after the sample.

## Tests

A scene test is a `#[scene]` with `impl SceneTest`, registered by a ctor into
`hilen::SCENE_TESTS` behind the `scene-tests` feature and run by the `scene-test`
crate, which also holds the corpus. Same flags as `level-test`, `make scene` runs
the suite. `Primitives` orbits the camera around every shape, `Materials` is the
metallic by roughness chart, `Lights` a point and a spot light, `Textures` a
texture and a normal map, `Skybox` chrome under a sky, `Transparency` blended
balls from both sides, `Drop balls` a physics rest, `Models` the monkey, the tree
and the textured cube from `assets/models` with the monkey dropped onto its
bounds, `Shadows` a post, a ball, a floating crate and the monkey under a low sun,
`Picking` taps landing on the nearest node and on the sky, `Player walk` a
player shoving a crate into a wall and jumping, `Animations` a skinned bar
bending, the fox running and a windmill spinning, checked at rest, frozen mid
clip and held after a single run, `Cascades` a thin pole and a row of posts down
a field hundreds of units long with the camera walking it, its middle check on
the spot where a floor pixel too coarse for the maps holding it once fell through
to the sun, and `FogTest` posts fading into fog under a sky fogged at the horizon,
the fog then pushed back and taken away. The loop runs free, so the frames between two
waits vary by one. A check of a pose in flight freezes the clip at a chosen time
through `set_animation_speed(0)` and `set_animation_time` first. A human hold
pauses the scene's time, so the probes sit on a still picture. Rapier is deterministic on one machine, a
lane that disagrees needs its `enhanced-determinism` feature. `hold_key` and
`release_key` drive the player from a test.

```bash
cargo run -p scene-test -- --list
cargo run -p scene-test -- --headless --test-name Primitives
cargo run -p scene-test -- --test-name DropBalls --human
cargo run -p scene-test -- --test-name Materials --present
```

`--present` hands one scene over on the test's own canvas at scale 1, the frame a
test sees, with a drag to orbit, the wheel to zoom and `w` `s` `a` `d` or the arrows
walking the camera level, or with a player in the scene the drag turns its head and
the keys walk it.

## What is next

The remaining deliveries are in [roadmap.md](roadmap.md): an embeddable scene
view, and culling nodes outside a cascade's box on the CPU so a short shadow
distance also cuts the shadow passes on a big level.
