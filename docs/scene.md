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

`Shape3` is `Box`, `Ball` or `Plane`, and it gives both the unit mesh and the
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

`sky` is a cube map, `Sky::gradient` for a smooth one and `Sky::from_faces` for six
images. The skybox draws it behind everything and every surface reflects it: the
diffuse through nine spherical harmonics and the specular through one mip per
roughness step, both prefiltered on the CPU in `hilen-pixels` when the sky is
made, the split sum with the Karis fit in place of a lookup table. Without a sky the
flat `ambient` color stands in. Highlights roll off through the Khronos PBR Neutral
compression without its black offset, so a color under the knee lands on screen as
the hex it was written as.

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
balls from both sides, `Drop balls` a physics rest, and `Player walk` a player
shoving a crate into a wall and jumping. Rapier is deterministic on one machine, a
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

The remaining deliveries are in [roadmap.md](roadmap.md): glTF models, shadows and
picking.
