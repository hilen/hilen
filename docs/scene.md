# 3D scenes

The `scene` module is the 3D twin of `level`, behind the `scene` cargo feature. A
`#[scene]` struct gets a `SceneBase` injected, `SceneManager` runs the one active
scene, nodes are `Own<dyn Node>` the way sprites are, and `SceneDrawer` draws them
with `MainLock` pipelines on `VecBuffer` and `UniformBind`. Physics is `rapier3d`,
math is glam re-exported from `gm::volume`. The scene draws every frame while one
is loaded, like a level.

## Writing a scene

```rust
#[scene]
#[derive(Default)]
struct Playground {}

impl SceneSetup for Playground {
    fn needs_physics(&self) -> bool { true }

    fn setup(&mut self) {
        self.camera = Camera { position: Vec3::new(0.0, 6.0, 10.0), ..Camera::default() };
        self.make_node::<Wall>(Shape3::Plane(20.0), Vec3::ZERO);
        self.make_node::<Body>(Shape3::Ball(0.5), Vec3::new(0.0, 5.0, 0.0))
            .set_color(Color::hex("#3498db"));
    }
}

SceneManager::set_scene(Playground::default());
```

`Shape3` is `Box`, `Ball` or `Plane`, and it gives both the unit mesh and the
collider, so what is drawn is what collides. `Body` is dynamic, `Wall` a fixed and
bouncy collider, `Prop` is only drawn. `NodeTemplates` carries `set_color`,
`set_position`, `set_rotation`, `set_friction` and `set_restitution`, and a `Body`
has `set_velocity`, `add_impulse` and `set_damping`. Rapier has no rolling
resistance, so a ball on a plane rolls forever without damping.

The world is right handed with y up, the glTF and Blender convention. A plane is
drawn at its origin facing up and its collider slab hangs below it, so a body rests
on the drawn surface. `Camera::orbit` turns the camera around its target, the demo
page drives it from a drag.

## How it draws

The scene draws in the main render pass, before the level and the UI, into the
viewport depth band 0.6 to 1.0. The UI draws at 0.5 and closer, a level sprite at
0.85, so the UI stays in front of any scene and a level can share the frame. The
band costs about one and a half bits of depth precision, the camera near plane
matters far more, keep it at 0.1 or above.

`MeshPipeline` does one instanced indexed draw per unit mesh. The instance carries
the model matrix, the inverse transpose for normals and the color, see
`MeshInstance`. Four float components cross the vertex to fragment boundary, the
normal and a flat instance index, and the fragment reads the color from a storage
binding by that index. An A7 draws nothing above eight, see [ios.md](ios.md). Base
vertex stays zero for the same device, and indices are 16 bit so every lane draws
them. Back faces are culled, so every primitive is wound counter clockwise seen from
outside, pinned by unit tests on the geometry.

Colors are encoded sRGB like the whole frame, see [colors.md](colors.md). The
shader decodes, lights in linear, and encodes at the end of the fragment. The first
delivery has one fixed sun and a flat ambient, Lambert only.

## Tests

A scene test is a `#[scene]` with `impl SceneTest`, registered by a ctor into
`hilen::SCENE_TESTS` behind the `scene-tests` feature and run by the `scene-test`
crate, which also holds the corpus. Same flags as `level-test`, `make scene` runs
the suite. `Primitives` orbits the camera a full turn around every shape and pins
probes at the half turn and the end. `Drop balls` rolls two dozen balls in a box
until they rest and pins where they stopped. Rapier is deterministic on one
machine, a lane that disagrees needs its `enhanced-determinism` feature.

```bash
cargo run -p scene-test -- --list
cargo run -p scene-test -- --headless --test-name Primitives
cargo run -p scene-test -- --test-name DropBalls --human
```

## What is next

The remaining deliveries are in [roadmap.md](roadmap.md): the Filament mobile PBR
material with a light list, glTF models, shadows and picking.
