mod batches;
mod builders;

use std::{array::from_fn, ops::Range};

use indexmap::IndexMap;
use plat::Platform;
use wgpu::{
    BindGroupLayout, Buffer, BufferUsages, CommandEncoder, IndexFormat, PipelineLayout,
    PipelineLayoutDescriptor, RenderPass, RenderPipeline, Sampler, ShaderModule, ShaderStages, TextureView,
};

use self::builders::{
    SmoothPipelines, depth_fill_pipelines, grass_pipeline, line_pipelines, mesh_pipelines, shader,
};
use super::{
    air_textures::AirTextures,
    mesh_binds::{
        instances_bind, instances_layout, opaque_bind, plain_texture, textures_bind, textures_layout,
        view_bind, view_layout,
    },
};
use crate::{
    deps::refs::Weak,
    gm::{
        color::Color,
        volume::{Mat4, SkinVertex, Vec3, Vertex3D},
    },
    render::{
        SHADOW_CASCADES, SceneUniform,
        bind_cache::{CachedBind, StorageKey, cached},
        buffer_helper::BufferHelper,
        data::{GrassBlade, LineVertex, MeshInstance, MeshLight, WideLine},
        device_helper::{DeviceHelper, MeshDraw},
        frustum::{self, Frustum},
        pipelines::shadow_pass::{ShadowBatch, ShadowPass},
        uniform::make_storage_layout,
        vec_buffer::VecBuffer,
        vertex_layout::VertexLayout,
    },
    scene::{Mesh, Sky},
    window::{
        Window,
        image::{Image, Texture},
    },
};

/// What the mesh and the sky shaders share, prepended to both.
const COMMON: &str = include_str!("shaders/scene_common.wgsl");
const MESH: &str = include_str!("shaders/mesh.wgsl");
const SKY: &str = include_str!("shaders/sky.wgsl");
const SHADOW: &str = include_str!("shaders/shadow.wgsl");
/// The dither of a fading node, prepended to the mesh and the shadow
/// shaders.
const FADE: &str = include_str!("shaders/fade.wgsl");
const LINES: &str = include_str!("shaders/lines.wgsl");
const WIDE_LINES: &str = include_str!("shaders/wide_lines.wgsl");
const ATMOSPHERE: &str = include_str!("shaders/atmosphere.wgsl");

/// The vertex buffers of a static draw and of a skinned one, which adds
/// the joints and weights per vertex.
pub(super) const STATIC_LAYOUTS: &[wgpu::VertexBufferLayout] =
    &[Vertex3D::VERTEX_LAYOUT, MeshInstance::VERTEX_LAYOUT];
pub(super) const SKINNED_LAYOUTS: &[wgpu::VertexBufferLayout] = &[
    Vertex3D::VERTEX_LAYOUT,
    MeshInstance::VERTEX_LAYOUT,
    SkinVertex::VERTEX_LAYOUT,
];
/// One patch of grass this frame, see `scene::Grass`: its blades drawn
/// as instances of a mesh part, the instance its fragments read the
/// material and the lights from, and the matrix `v_grass` reads.
pub(crate) struct GrassDraw {
    pub key:      MeshKey,
    pub blades:   Buffer,
    /// The first blades of the buffer that are drawn, the far ones are
    /// gone, see `v_grass`.
    pub count:    u32,
    pub slot:     u32,
    pub instance: MeshInstance,
    pub patch:    Mat4,
    /// The turn of the field's axes, see `Grass::turn`.
    pub turn:     Mat4,
}

/// What one instanced draw shares: the mesh, the four textures and the
/// pipeline, a cut out node draws through its own.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MeshKey {
    pub mesh:                   Weak<Mesh>,
    pub texture:                Option<Weak<Image>>,
    pub normal_map:             Option<Weak<Image>>,
    pub metallic_roughness_map: Option<Weak<Image>>,
    pub occlusion_map:          Option<Weak<Image>>,
    /// Whether the material has a cutout or a fade below 1. The threshold
    /// and the fade are the instance's own.
    pub cutout:                 bool,
}

/// The sky cube, its sampler and the shadow map a view bind holds.
pub(super) type ViewKey = (TextureView, Sampler, TextureView, TextureView, TextureView);
/// The instance range and the joint range an instances bind holds.
pub(super) type InstancesKey = (StorageKey, StorageKey);
/// Whether the base texture, the normal map, the metallic roughness map
/// and the occlusion map of a key were alive when its bind was made, a
/// dead one is bound as its plain stand in.
pub(super) type TexturesKey = (bool, bool, bool, bool);

/// The passes an opaque instance draws in, the camera's first, then
/// every cascade of the sun.
const PASSES: usize = 1 + SHADOW_CASCADES;

/// Where a node is, the sphere around it in world space. A node with
/// none is drawn in every pass.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Reach {
    pub center: Vec3,
    pub radius: f32,
}

/// One instanced draw: the instances of a mesh and texture pair. They
/// wait on the CPU until `prepare` copies every batch into the one opaque
/// buffer, one upload per frame, where a buffer per batch cost a staging
/// buffer per batch. `drawn` is where they landed in it, a range per
/// pass with only the instances that pass can see. Every pass drew every
/// node before, and a streamed world sent its whole ring of distant
/// trees through the camera and 3 cascades, most of them out of all 4.
#[derive(Default)]
struct Batch {
    pending: Vec<(MeshInstance, Option<Reach>)>,
    drawn:   [Range<u32>; PASSES],
    frame:   u64,
}

impl Batch {
    /// The instances `prepare` loaded on `frame` for `pass`, none when it
    /// loaded none.
    fn drawn_in(&self, frame: u64, pass: usize) -> Option<Range<u32>> {
        let range = &self.drawn[pass];
        (self.frame == frame && !range.is_empty()).then(|| range.clone())
    }

    /// Whether `prepare` loaded any instance of this batch on `frame`.
    fn loaded(&self, frame: u64) -> bool {
        (0..PASSES).any(|pass| self.drawn_in(frame, pass).is_some())
    }
}

/// Draws the sky and every node of a frame, one instanced indexed draw
/// per mesh and texture pair for the opaque nodes, then one draw per
/// translucent node, back to front. `prepare` loads the frame's buffers
/// and draws the shadow map before the frame's pass opens, `draw` draws
/// into it. A skinned mesh draws through the skinned twin of each
/// pipeline, the same shader from its other vertex entry, which blends
/// the joint matrices its instance points at.
pub struct MeshPipeline {
    /// Each a pipeline and its skinned twin.
    opaque:       [RenderPipeline; 2],
    cutout:       [RenderPipeline; 2],
    cutout_depth: [RenderPipeline; 2],
    translucent:  [RenderPipeline; 2],
    sky:          RenderPipeline,
    /// Fill the depth of the viewport with the far end of the scene's
    /// reversed depth before the scene, and with 1 after it, see
    /// `fill_depth`.
    depth_far:    RenderPipeline,
    depth_near:   RenderPipeline,
    /// The grass, `f_grass` from the `v_grass` vertex entry.
    grass:        RenderPipeline,

    /// The opaque and the cut out pipelines of a depth photo, made at the
    /// first one from the layout and the shader of the nodes, so an app
    /// that takes none does not wait for them at the start.
    depth:               Option<[[RenderPipeline; 2]; 2]>,
    /// The pipelines of a mesh whose colors fade, made at the first such
    /// mesh. A cut out one takes the last in a depth photo, made with the
    /// depth ones when there is such a mesh.
    smooth:              Option<SmoothPipelines>,
    smooth_depth_cutout: Option<RenderPipeline>,
    mesh_layout:         PipelineLayout,
    mesh_shader:         ShaderModule,

    /// Whether this frame draws the edges of every node over it, see
    /// `SceneBase::show_wireframe`. The pipelines are made at the first
    /// such frame, like the depth ones.
    pub(crate) wireframe: bool,
    wireframe_pipelines:  Option<[RenderPipeline; 2]>,

    /// Debug lines in world space, the collider wireframes, drawn after
    /// every node and depth tested against them.
    lines_pipeline: RenderPipeline,
    lines:          VecBuffer<LineVertex>,

    /// The lines of the scene, a quad per line of its own width. The
    /// tested ones hide behind the nodes, the ones on top draw last over
    /// everything.
    wide_tested:  RenderPipeline,
    wide_on_top:  RenderPipeline,
    tested_lines: VecBuffer<WideLine>,
    on_top_lines: VecBuffer<WideLine>,

    /// The frame's view, the sky cube it lights and reflects, and the
    /// sun's shadow map.
    view_layout:    BindGroupLayout,
    view_buffer:    Buffer,
    /// Bound when the scene has no sky.
    black_sky:      Sky,
    /// The view bind over the sky and the map last drawn with.
    view_bind:      Option<CachedBind<ViewKey>>,
    /// The tables of the scene's air, see `scene::Atmosphere`.
    pub(crate) air: AirTextures,

    /// The sun's depth passes, which cannot bind the map they draw, so
    /// they carry the light's matrices on their own.
    shadow: ShadowPass,

    /// Binds the instance buffer for the fragment stage and the joint
    /// buffer for the vertex stage.
    instances_layout: BindGroupLayout,

    // A unit mesh lives for the whole process, a model's mesh dies with
    // the model and takes its key out at the next draw. An image can die
    // too, then the draw falls back to the plain textures.
    instances: IndexMap<MeshKey, Batch>,

    /// The instances of every opaque batch of the frame, one bind over
    /// all of them.
    opaque_instances: VecBuffer<MeshInstance>,
    opaque_bind:      Option<CachedBind<InstancesKey>>,

    /// The translucent nodes in draw order, the key of each and the
    /// bind over them.
    transparent:      VecBuffer<MeshInstance>,
    transparent_keys: Vec<MeshKey>,
    transparent_bind: Option<CachedBind<InstancesKey>>,

    /// The joint matrices of every skinned node of the frame, one run
    /// per node and skin, indexed by the instance's `joint_base`.
    joints: VecBuffer<Mat4>,

    /// The grass patches of the frame waiting for `prepare`, then the
    /// ones it loaded, with 1 instance and 1 matrix per patch slot in the
    /// buffers of the grass bind.
    grass_pending:   Vec<GrassDraw>,
    grass_drawn:     Vec<GrassDraw>,
    grass_instances: VecBuffer<MeshInstance>,
    grass_patches:   VecBuffer<Mat4>,
    grass_bind:      Option<CachedBind<InstancesKey>>,

    /// The point and spot lights of the frame, indexed by every
    /// instance's light list.
    lights_layout: BindGroupLayout,
    lights:        VecBuffer<MeshLight>,
    lights_bind:   Option<CachedBind<StorageKey>>,

    /// The base color texture, the normal map, the metallic roughness
    /// map and the occlusion map of a draw, with the plain ones a
    /// material without them binds, white for all but the normal map,
    /// and the bind of every key drawn, opaque or translucent.
    textures_layout: BindGroupLayout,
    white:           Texture,
    flat_normal:     Texture,
    textures:        IndexMap<MeshKey, Option<CachedBind<TexturesKey>>>,
}

impl MeshPipeline {
    /// Texels along each side of every cascade of the shadow map. A
    /// phone and a browser get a quarter of the desktop's area.
    pub(crate) const SHADOW_MAP_SIZE: u32 = if Platform::MOBILE || Platform::WASM {
        1024
    } else {
        2048
    };
}

impl Default for MeshPipeline {
    fn default() -> Self {
        let device = Window::device();

        // The common part reads the cascade count, one source for both.
        let common = format!("const SHADOW_CASCADES: i32 = {SHADOW_CASCADES};\n{COMMON}\n{ATMOSPHERE}");
        let mesh_shader = shader(device, "mesh_shader", format!("{common}\n{FADE}\n{MESH}"));
        let sky_shader = shader(device, "sky_shader", format!("{common}\n{SKY}"));
        let lines_shader = shader(device, "lines_shader", format!("{common}\n{LINES}"));
        let wide_shader = shader(device, "wide_lines_shader", format!("{common}\n{WIDE_LINES}"));
        let shadow_shader = shader(device, "shadow_shader", format!("{FADE}\n{SHADOW}"));

        let view_layout = view_layout(device);
        let instances_layout = instances_layout(device);
        let lights_layout = make_storage_layout("mesh_lights_layout", ShaderStages::FRAGMENT);
        let textures_layout = textures_layout(device);

        let mesh_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "mesh_pipeline_layout".into(),
            bind_group_layouts: &[
                Some(&view_layout),
                Some(&instances_layout),
                Some(&lights_layout),
                Some(&textures_layout),
            ],
            immediate_size:     0,
        });
        let sky_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label:              "sky_pipeline_layout".into(),
            bind_group_layouts: &[Some(&view_layout)],
            immediate_size:     0,
        });

        let draws =
            |label: &str, draw: MeshDraw| mesh_pipelines(device, label, &mesh_layout, &mesh_shader, draw);
        let opaque = draws("mesh", MeshDraw::Opaque);
        let cutout = draws("mesh_cutout", MeshDraw::Cutout);
        let cutout_depth = draws("mesh_cutout_depth", MeshDraw::CutoutDepth);
        let translucent = draws("mesh_translucent", MeshDraw::Translucent);
        let sky = device.sky_pipeline("sky_pipeline", &sky_layout, &sky_shader);
        let [depth_far, depth_near] = depth_fill_pipelines(device);
        let grass = grass_pipeline(device, &mesh_layout, &mesh_shader);
        // The lines bind only the view, the sky's layout.
        let [lines_pipeline, wide_tested, wide_on_top] =
            line_pipelines(device, &sky_layout, &lines_shader, &wide_shader);

        let view_buffer = device.buffer(
            &SceneUniform::default(),
            BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        );

        Self {
            opaque,
            cutout,
            cutout_depth,
            translucent,
            sky,
            grass,
            depth: None,
            smooth: None,
            smooth_depth_cutout: None,
            mesh_layout,
            mesh_shader,
            wireframe: false,
            wireframe_pipelines: None,
            lines_pipeline,
            lines: VecBuffer::default(),
            wide_tested,
            wide_on_top,
            tested_lines: VecBuffer::default(),
            on_top_lines: VecBuffer::default(),
            view_layout,
            view_buffer,
            black_sky: Sky::black(),
            depth_far,
            depth_near,
            view_bind: None,
            air: AirTextures::new(),
            shadow: ShadowPass::new(
                device,
                &shadow_shader,
                Self::SHADOW_MAP_SIZE,
                &instances_layout,
                &textures_layout,
            ),
            instances_layout,
            instances: IndexMap::default(),
            opaque_instances: VecBuffer::default(),
            opaque_bind: None,
            transparent: VecBuffer::default(),
            transparent_keys: vec![],
            transparent_bind: None,
            joints: VecBuffer::default(),
            grass_pending: vec![],
            grass_drawn: vec![],
            grass_instances: VecBuffer::default(),
            grass_patches: VecBuffer::default(),
            grass_bind: None,
            lights_layout,
            lights: VecBuffer::default(),
            lights_bind: None,
            textures_layout,
            white: plain_texture([255, 255, 255, 255], "mesh_white"),
            flat_normal: plain_texture([128, 128, 255, 255], "mesh_flat_normal"),
            textures: IndexMap::default(),
        }
    }
}

impl MeshPipeline {
    /// An opaque instance, left out of the passes whose view misses
    /// `reach`, drawn in all of them with none.
    pub(crate) fn add(&mut self, key: MeshKey, instance: MeshInstance, reach: Option<Reach>) {
        self.fit_smooth(&key);
        self.instances.entry(key).or_default().pending.push((instance, reach));
    }

    /// Translucent nodes draw in the order they are added, so add them
    /// back to front.
    pub(crate) fn add_transparent(&mut self, key: MeshKey, mut instance: MeshInstance) {
        self.fit_smooth(&key);
        instance.index = self.transparent.pending();
        self.transparent.push(instance);
        self.transparent_keys.push(key);
    }

    /// Makes the pipelines of a mesh whose colors fade the first time
    /// `key` draws one.
    fn fit_smooth(&mut self, key: &MeshKey) {
        if key.mesh.smooth_colors && self.smooth.is_none() {
            let device = Window::device();
            self.smooth = Some(SmoothPipelines::new(device, &self.mesh_layout, &self.mesh_shader));
            self.shadow.make_smooth(device);
        }
    }

    /// A grass patch, drawn in the camera's pass only. Grass casts no
    /// shadow, it is too thin to show in the maps and doubles the cost.
    pub(crate) fn add_grass(&mut self, draw: GrassDraw) {
        self.grass_pending.push(draw);
    }

    pub(crate) fn add_light(&mut self, light: MeshLight) {
        self.lights.push(light);
    }

    /// A debug line between two world points, drawn this frame.
    pub(crate) fn add_line(&mut self, from: Vec3, to: Vec3, color: Color) {
        self.lines.push(LineVertex::new(from, color));
        self.lines.push(LineVertex::new(to, color));
    }

    /// A line of the scene, `width` pixels wide, hidden behind the nodes
    /// or drawn over them all.
    pub(crate) fn add_wide_line(&mut self, line: WideLine, on_top: bool) {
        if on_top {
            self.on_top_lines.push(line);
        } else {
            self.tested_lines.push(line);
        }
    }

    /// Queues the joint matrices of one skinned node and returns where
    /// they start, the `joint_base` of its instances.
    pub(crate) fn add_joints(&mut self, matrices: &[Mat4]) -> u32 {
        let base = self.joints.pending();
        for matrix in matrices {
            self.joints.push(*matrix);
        }
        base
    }

    /// Loads the frame's buffers and, when the sun casts, draws the
    /// shadow map's cascades, `map_size` texels a side. Before the
    /// frame's pass opens, the pass reads the map.
    pub(crate) fn prepare(
        &mut self,
        encoder: &mut CommandEncoder,
        view: &SceneUniform,
        shadows: bool,
        map_size: u32,
    ) {
        self.instances.retain(|key, _| key.mesh.is_ok());
        self.textures.retain(|key, _| key.mesh.is_ok());

        self.view_buffer.update(*view);

        // A storage binding cannot be empty, and no instance indexes the
        // placeholder since every light count is zero then.
        if self.lights.is_empty() {
            self.lights.push(MeshLight::default());
        }
        self.lights.load();

        // The same for the joints, only a skinned draw reads them.
        if self.joints.is_empty() {
            self.joints.push(Mat4::IDENTITY);
        }
        self.joints.load();

        let frame = Window::render_frame();
        let camera = view.camera_pos.truncate();
        let views: [Frustum; PASSES] = from_fn(|pass| match pass {
            0 => Frustum::of(view.view_proj),
            cascade => Frustum::of(view.sun_view_proj[cascade - 1]),
        });
        let passes = if shadows { PASSES } else { 1 };
        let cull = frustum::enabled();
        for batch in self.instances.values_mut() {
            // A batch nothing added to draws nothing. Its range can still be
            // from this same frame number, a picture is drawn between two
            // frames and the scene on screen loaded the frame before it.
            batch.drawn = Default::default();
            if batch.pending.is_empty() {
                continue;
            }
            for (pass, frustum) in views.iter().enumerate().take(passes) {
                let start = self.opaque_instances.pending();
                for (instance, reach) in &batch.pending {
                    // The view projections take a point relative to the
                    // camera, see `SceneUniform`.
                    if cull && reach.is_some_and(|reach| !frustum.holds(reach.center - camera, reach.radius))
                    {
                        continue;
                    }
                    let mut instance = *instance;
                    instance.index = self.opaque_instances.pending();
                    self.opaque_instances.push(instance);
                }
                batch.drawn[pass] = start..self.opaque_instances.pending();
            }
            batch.pending.clear();
            batch.frame = frame;
        }
        if !self.opaque_instances.is_empty() {
            self.opaque_instances.load();
        }
        self.load_grass();
        if self.transparent.is_empty() {
            self.transparent.forget();
        } else {
            self.transparent.load();
        }
        if self.lines.is_empty() {
            self.lines.forget();
        } else {
            self.lines.load();
        }
        for lines in [&mut self.tested_lines, &mut self.on_top_lines] {
            if lines.is_empty() {
                lines.forget();
            } else {
                lines.load();
            }
        }

        if shadows && self.instances.values().any(|batch| batch.loaded(frame)) {
            self.shadow.fit(Window::device(), map_size);
            let mut batches = vec![];
            for (key, batch) in self.instances.iter().filter(|(_, batch)| batch.loaded(frame)) {
                // A cut out batch reads its texture's alpha in the pass.
                let textures = key.cutout.then(|| {
                    textures_bind(
                        &mut self.textures,
                        &self.textures_layout,
                        key,
                        &self.white,
                        &self.flat_normal,
                    )
                    .clone()
                });
                batches.push(ShadowBatch {
                    key,
                    ranges: from_fn(|cascade| batch.drawn[1 + cascade].clone()),
                    textures,
                });
            }
            let instances_bind = opaque_bind(
                &mut self.opaque_bind,
                &self.instances_layout,
                &self.opaque_instances,
                &self.joints,
            );
            self.shadow.draw(
                encoder,
                &view.sun_view_proj,
                view.camera_pos,
                &self.opaque_instances,
                &batches,
                instances_bind,
            );
        } else if shadows {
            self.shadow.fit(Window::device(), map_size);
            self.shadow.clear(encoder);
        }
    }

    /// Draws the frame. `background` draws the sky pass first, for a
    /// sky or for fog filling in behind everything without one.
    /// A scene shares the frame's depth buffer with the UI, which tests
    /// plain depth from a clear of 1. Before the scene the viewport's depth
    /// becomes 0, the far end of the scene's reversed depth, and after it
    /// 1 again, so the UI over the scene draws as before.
    pub(crate) fn fill_depth(&self, render_pass: &mut RenderPass, far: bool) {
        render_pass.set_pipeline(if far { &self.depth_far } else { &self.depth_near });
        render_pass.draw(0..3, 0..1);
    }

    pub(crate) fn draw(&mut self, render_pass: &mut RenderPass, sky: Option<&Sky>, background: bool) {
        let frame = Window::render_frame();
        let translucent = self.transparent.frame() == frame && self.transparent.has_loaded();
        let lines = self.lines.frame() == frame && self.lines.has_loaded();
        let tested = self.tested_lines.frame() == frame && self.tested_lines.has_loaded();
        let on_top = self.on_top_lines.frame() == frame && self.on_top_lines.has_loaded();
        let no_nodes = !self.instances.values().any(|batch| batch.drawn_in(frame, 0).is_some())
            && self.grass_drawn.is_empty()
            && !translucent
            && !lines
            && !tested
            && !on_top;
        if no_nodes && !background {
            self.transparent_keys.clear();
            return;
        }

        let sky = sky.unwrap_or(&self.black_sky);
        let shadow_map = self.shadow.map();
        let view_bind = cached(
            &mut self.view_bind,
            (
                sky.view.clone(),
                sky.sampler.clone(),
                shadow_map.clone(),
                self.air.transmittance.clone(),
                self.air.multi.clone(),
            ),
            || view_bind(&self.view_layout, &self.view_buffer, sky, shadow_map, &self.air),
        );
        render_pass.set_bind_group(0, view_bind, &[]);

        if no_nodes {
            if background {
                self.draw_sky(render_pass);
            }
            self.transparent_keys.clear();
            return;
        }

        if self.wireframe && self.wireframe_pipelines.is_none() {
            self.wireframe_pipelines = Some(mesh_pipelines(
                Window::device(),
                "mesh_wireframe",
                &self.mesh_layout,
                &self.mesh_shader,
                MeshDraw::Wireframe,
            ));
        }

        self.draw_batches(render_pass, frame, false);
        self.draw_grass(render_pass);

        // After the opaque nodes, so the sky shades only the pixels they
        // left at the far depth. Its march through the air is the dearest
        // shader of a frame, a planet seen from space hides half the sky.
        if background {
            self.draw_sky(render_pass);
        }

        // Before the translucent nodes, which blend over a line behind
        // them.
        if tested {
            render_pass.set_pipeline(&self.wide_tested);
            render_pass.set_vertex_buffer(0, self.tested_lines.slice());
            render_pass.draw(0..6, 0..self.tested_lines.len());
        }

        if translucent {
            self.draw_translucent(render_pass);
        } else {
            self.transparent_keys.clear();
        }

        if lines {
            render_pass.set_pipeline(&self.lines_pipeline);
            render_pass.set_vertex_buffer(0, self.lines.slice());
            render_pass.draw(0..self.lines.len(), 0..1);
        }

        if on_top {
            render_pass.set_pipeline(&self.wide_on_top);
            render_pass.set_vertex_buffer(0, self.on_top_lines.slice());
            render_pass.draw(0..6, 0..self.on_top_lines.len());
        }
    }

    fn draw_sky(&self, render_pass: &mut RenderPass) {
        render_pass.set_pipeline(&self.sky);
        render_pass.draw(0..3, 0..1);
    }

    /// Lets go of everything loaded this frame. A picture is drawn and
    /// submitted on its own, and the frame's draw that follows on the same
    /// frame number must not draw the picture's nodes again.
    pub(crate) fn forget_frame(&mut self) {
        for batch in self.instances.values_mut() {
            batch.drawn = Default::default();
        }
        self.grass_drawn.clear();
        self.transparent.forget();
        self.transparent_keys.clear();
        self.lines.forget();
        self.tested_lines.forget();
        self.on_top_lines.forget();
    }

    /// Fills the grass buffers from the patches added this frame, the
    /// instance and the 2 matrices of each at its slot, the patch's and
    /// its field's turn. The slots stay with a patch while it lives, its
    /// blades name theirs.
    fn load_grass(&mut self) {
        self.grass_drawn = std::mem::take(&mut self.grass_pending);
        let Some(last) = self.grass_drawn.iter().map(|draw| draw.slot).max() else {
            return;
        };
        let slots = last as usize + 1;
        let mut instances = vec![bytemuck::Zeroable::zeroed(); slots];
        let mut patches = vec![Mat4::ZERO; slots * 2];
        for draw in &self.grass_drawn {
            let slot = draw.slot as usize;
            instances[slot] = draw.instance;
            instances[slot].index = draw.slot;
            patches[slot * 2] = draw.patch;
            patches[slot * 2 + 1] = draw.turn;
        }
        for instance in instances {
            self.grass_instances.push(instance);
        }
        for patch in patches {
            self.grass_patches.push(patch);
        }
        self.grass_instances.load();
        self.grass_patches.load();
    }

    /// Draws the grass patches `prepare` loaded, after the opaque nodes,
    /// whose binds of group 0 and 2 they share.
    fn draw_grass(&mut self, render_pass: &mut RenderPass) {
        if self.grass_drawn.is_empty() {
            return;
        }
        let bind = cached(
            &mut self.grass_bind,
            (
                StorageKey::of(&self.grass_instances),
                StorageKey::of(&self.grass_patches),
            ),
            || instances_bind(&self.instances_layout, &self.grass_instances, &self.grass_patches),
        );
        render_pass.set_bind_group(1, bind, &[]);
        let stride = GrassBlade::VERTEX_LAYOUT.array_stride;
        for draw in self.grass_drawn.iter().filter(|draw| draw.key.mesh.is_ok()) {
            let textures_bind = textures_bind(
                &mut self.textures,
                &self.textures_layout,
                &draw.key,
                &self.white,
                &self.flat_normal,
            );
            render_pass.set_bind_group(3, textures_bind, &[]);
            render_pass.set_pipeline(&self.grass);
            render_pass.set_vertex_buffer(0, draw.key.mesh.vertex_buffer.slice(..));
            render_pass.set_index_buffer(draw.key.mesh.index_buffer.slice(..), IndexFormat::Uint16);
            render_pass.set_vertex_buffer(1, draw.blades.slice(..u64::from(draw.count) * stride));
            render_pass.draw_indexed(0..draw.key.mesh.index_count, 0, 0..draw.count);
        }
    }
}

/// Draws the edges of `count` instances of the mesh whose vertex and
/// instance buffers are set, see `MeshDraw::Wireframe`.
fn draw_edges(pass: &mut RenderPass, mesh: &Mesh, [plain, skinned]: &[RenderPipeline; 2], count: u32) {
    pass.set_pipeline(if mesh.skin_buffer.is_some() {
        skinned
    } else {
        plain
    });
    let (edges, edge_count) = mesh.edges();
    pass.set_index_buffer(edges.slice(..), IndexFormat::Uint16);
    pass.draw_indexed(0..*edge_count, 0, 0..count);
}

/// Picks the pipeline for the mesh out of a pipeline and its skinned
/// twin, and sets its vertex and index buffers, the skin buffer too
/// when it has one.
pub(super) fn set_mesh(pass: &mut RenderPass, mesh: &Mesh, [plain, skinned]: &[RenderPipeline; 2]) {
    match &mesh.skin_buffer {
        Some(skin) => {
            pass.set_pipeline(skinned);
            pass.set_vertex_buffer(2, skin.slice(..));
        }
        None => pass.set_pipeline(plain),
    }
    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    pass.set_index_buffer(mesh.index_buffer.slice(..), IndexFormat::Uint16);
}
