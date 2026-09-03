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
`set_friction` and `set_restitution`, and a `Body` has `set_velocity`,
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
file at its own size and with its own materials. A primitive without a material
takes the node's `Material`. The collider is the box around the model's `bounds`,
placed where the bounds are, so a model whose origin sits at its feet still rests
on the floor. A primitive over 65535 vertices is split into parts so every lane
draws 16 bit indices, and one without normals shades flat as glTF asks. Only a
`.glb` with embedded buffers and images loads, triangles only, no skins, morphs or
animations yet.

The Blender sources sit next to the exports in `assets/models`. One file exports
with `blender --background --python build/export_glb.py -- assets/models/tree.blend`,
which also mends what the old files miss, see the script. Only the `.glb` files
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

`sun.shadows` makes the sun cast, off by default since it draws every opaque node a
second time. One shadow map covers the whole scene, an orthographic view of the sun
over the sphere around every node, 2048 texels wide on desktop and 1024 on a phone
or in the browser, `Depth32Float`. It draws in its own pass before the frame's pass
opens, through the `prepare` hook of `WindowEvents`, so the frame can read it. The
fragment reads four texels around where it lands by `textureLoad`, compares each
and blends them by distance, a depth texture cannot be filtered and a comparison
sampler does not work on iOS 12. Acne is held off by a slope scaled bias in the
pass and a normal offset of a texel and a half in the receiver. Translucent nodes
receive but do not cast.

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
`Picking` taps landing on the nearest node and on the sky, and `Player walk` a
player shoving a crate into a wall and jumping. Rapier is deterministic on one machine, a
lane that disagrees needs its `enhanced-determinism` feature. `hold_key` and
`release_key` drive the player from a test.

```bash
cargo run -p scene-test -- --list
cargo run -p scene-test -- --headless --test-name Primitives
cargo run -p scene-test -- --test-name DropBalls --human
cargo run -p scene-test -- --test-name Materials --present
```

`--present` hands one scene over with a drag to orbit and the wheel to zoom, or
with a player in the scene the drag turns its head and the keys walk it.

## What is next

The remaining deliveries are in [roadmap.md](roadmap.md): skins and animations,
then cascaded shadows, fog and an embeddable scene view.
