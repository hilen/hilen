use std::ops::Range;

use indexmap::IndexMap;
use plat::Platform;
use wgpu::{
    BindGroupLayout, Buffer, BufferUsages, CommandEncoder, CompareFunction, IndexFormat, PipelineLayout,
    PipelineLayoutDescriptor, PrimitiveTopology, RenderPass, RenderPipeline, Sampler, ShaderModule,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, TextureView,
};

use super::mesh_binds::{
    instances_bind, instances_layout, opaque_bind, plain_texture, storage_bind, textures_bind,
    textures_layout, view_bind, view_layout,
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
        data::{LineVertex, MeshInstance, MeshLight, WideLine},
        device_helper::{DeviceHelper, MeshDraw},
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
const LINES: &str = include_str!("shaders/lines.wgsl");
const WIDE_LINES: &str = include_str!("shaders/wide_lines.wgsl");

const LINE_LAYOUTS: &[wgpu::VertexBufferLayout] = &[LineVertex::VERTEX_LAYOUT];
const WIDE_LINE_LAYOUTS: &[wgpu::VertexBufferLayout] = &[WideLine::VERTEX_LAYOUT];

/// The vertex buffers of a static draw and of a skinned one, which adds
/// the joints and weights per vertex.
pub(super) const STATIC_LAYOUTS: &[wgpu::VertexBufferLayout] =
    &[Vertex3D::VERTEX_LAYOUT, MeshInstance::VERTEX_LAYOUT];
pub(super) const SKINNED_LAYOUTS: &[wgpu::VertexBufferLayout] = &[
    Vertex3D::VERTEX_LAYOUT,
    MeshInstance::VERTEX_LAYOUT,
    SkinVertex::VERTEX_LAYOUT,
];

/// What one instanced draw shares: the mesh, the four textures and the
/// pipeline, a cut out node draws through its own.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct MeshKey {
    pub mesh:                   Weak<Mesh>,
    pub texture:                Option<Weak<Image>>,
    pub normal_map:             Option<Weak<Image>>,
    pub metallic_roughness_map: Option<Weak<Image>>,
    pub occlusion_map:          Option<Weak<Image>>,
    /// Whether the material has a cutout. The threshold is the
    /// instance's own.
    pub cutout:                 bool,
}

/// The sky cube, its sampler and the shadow map a view bind holds.
pub(super) type ViewKey = (TextureView, Sampler, TextureView);
/// The instance range and the joint range an instances bind holds.
pub(super) type InstancesKey = (StorageKey, StorageKey);
/// Whether the base texture, the normal map, the metallic roughness map
/// and the occlusion map of a key were alive when its bind was made, a
/// dead one is bound as its plain stand in.
pub(super) type TexturesKey = (bool, bool, bool, bool);

/// One instanced draw: the instances of a mesh and texture pair. They
/// wait on the CPU until `prepare` copies every batch into the one opaque
/// buffer, one upload per frame, where a buffer per batch cost a staging
/// buffer per batch. `drawn` is where they landed in it.
#[derive(Default)]
struct Batch {
    pending: Vec<MeshInstance>,
    drawn:   Range<u32>,
    frame:   u64,
}

impl Batch {
    /// Whether `prepare` loaded this batch on `frame`.
    fn loaded(&self, frame: u64) -> bool {
        self.frame == frame && !self.drawn.is_empty()
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
    opaque:      [RenderPipeline; 2],
    cutout:      [RenderPipeline; 2],
    translucent: [RenderPipeline; 2],
    sky:         RenderPipeline,

    /// The opaque and the cut out pipelines of a depth photo, made at the
    /// first one from the layout and the shader of the nodes, so an app
    /// that takes none does not wait for them at the start.
    depth:       Option<[[RenderPipeline; 2]; 2]>,
    mesh_layout: PipelineLayout,
    mesh_shader: ShaderModule,

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
    view_layout: BindGroupLayout,
    view_buffer: Buffer,
    /// Bound when the scene has no sky.
    black_sky:   Sky,
    /// The view bind over the sky and the map last drawn with.
    view_bind:   Option<CachedBind<ViewKey>>,

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
        let common = format!("const SHADOW_CASCADES: i32 = {SHADOW_CASCADES};\n{COMMON}");
        let mesh_shader = shader(device, "mesh_shader", format!("{common}\n{MESH}"));
        let sky_shader = shader(device, "sky_shader", format!("{common}\n{SKY}"));
        let lines_shader = shader(device, "lines_shader", format!("{common}\n{LINES}"));
        let wide_shader = shader(device, "wide_lines_shader", format!("{common}\n{WIDE_LINES}"));
        let shadow_shader = shader(device, "shadow_shader", SHADOW.to_string());

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

        let opaque = mesh_pipelines(device, "mesh", &mesh_layout, &mesh_shader, MeshDraw::Opaque);
        let cutout = mesh_pipelines(
            device,
            "mesh_cutout",
            &mesh_layout,
            &mesh_shader,
            MeshDraw::Cutout,
        );
        let translucent = mesh_pipelines(
            device,
            "mesh_translucent",
            &mesh_layout,
            &mesh_shader,
            MeshDraw::Translucent,
        );
        let sky = device.sky_pipeline("sky_pipeline", &sky_layout, &sky_shader);
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
            translucent,
            sky,
            depth: None,
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
            view_bind: None,
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
    pub(crate) fn add(&mut self, key: MeshKey, instance: MeshInstance) {
        self.instances.entry(key).or_default().pending.push(instance);
    }

    /// Translucent nodes draw in the order they are added, so add them
    /// back to front.
    pub(crate) fn add_transparent(&mut self, key: MeshKey, mut instance: MeshInstance) {
        instance.index = self.transparent.pending();
        self.transparent.push(instance);
        self.transparent_keys.push(key);
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
        for batch in self.instances.values_mut() {
            // A batch nothing added to draws nothing. Its range can still be
            // from this same frame number, a picture is drawn between two
            // frames and the scene on screen loaded the frame before it.
            if batch.pending.is_empty() {
                batch.drawn = 0..0;
                continue;
            }
            let start = self.opaque_instances.pending();
            for mut instance in batch.pending.drain(..) {
                instance.index = self.opaque_instances.pending();
                self.opaque_instances.push(instance);
            }
            batch.drawn = start..self.opaque_instances.pending();
            batch.frame = frame;
        }
        if !self.opaque_instances.is_empty() {
            self.opaque_instances.load();
        }
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
                    range: batch.drawn.clone(),
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
    pub(crate) fn draw(&mut self, render_pass: &mut RenderPass, sky: Option<&Sky>, background: bool) {
        let frame = Window::render_frame();
        let translucent = self.transparent.frame() == frame && self.transparent.has_loaded();
        let lines = self.lines.frame() == frame && self.lines.has_loaded();
        let tested = self.tested_lines.frame() == frame && self.tested_lines.has_loaded();
        let on_top = self.on_top_lines.frame() == frame && self.on_top_lines.has_loaded();
        let no_nodes = !self.instances.values().any(|batch| batch.loaded(frame))
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
            (sky.view.clone(), sky.sampler.clone(), shadow_map.clone()),
            || view_bind(&self.view_layout, &self.view_buffer, sky, shadow_map),
        );
        render_pass.set_bind_group(0, view_bind, &[]);

        if background {
            render_pass.set_pipeline(&self.sky);
            render_pass.draw(0..3, 0..1);
        }

        if no_nodes {
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

    /// Draws the opaque and the cut out nodes of the frame with their
    /// distance from the camera as the color, see
    /// `SceneManager::depth_photo`. The sky, the translucent nodes and the
    /// lines are no surface to measure, so they stay out.
    pub(crate) fn draw_depth(&mut self, render_pass: &mut RenderPass, sky: Option<&Sky>) {
        let frame = Window::render_frame();
        self.transparent_keys.clear();
        if !self.instances.values().any(|batch| batch.loaded(frame)) {
            return;
        }
        if self.depth.is_none() {
            let device = Window::device();
            let pipelines = |name: &str, draw: MeshDraw| {
                mesh_pipelines(device, name, &self.mesh_layout, &self.mesh_shader, draw)
            };
            self.depth = Some([
                pipelines("mesh_depth", MeshDraw::Depth),
                pipelines("mesh_depth_cutout", MeshDraw::DepthCutout),
            ]);
        }

        // The sky of the frame, so the cached bind stays the frame's.
        let sky = sky.unwrap_or(&self.black_sky);
        let shadow_map = self.shadow.map();
        let view_bind = cached(
            &mut self.view_bind,
            (sky.view.clone(), sky.sampler.clone(), shadow_map.clone()),
            || view_bind(&self.view_layout, &self.view_buffer, sky, shadow_map),
        );
        render_pass.set_bind_group(0, view_bind, &[]);
        self.draw_batches(render_pass, frame, true);
    }

    /// Draws every opaque and cut out batch loaded this `frame`, through
    /// the depth pipelines when `depth` is set.
    fn draw_batches(&mut self, render_pass: &mut RenderPass, frame: u64, depth: bool) {
        let lights_bind = cached(&mut self.lights_bind, StorageKey::of(&self.lights), || {
            storage_bind("mesh_lights_bind", &self.lights_layout, &self.lights)
        });
        render_pass.set_bind_group(2, lights_bind, &[]);

        if self.instances.values().any(|batch| batch.loaded(frame)) {
            let instances_bind = opaque_bind(
                &mut self.opaque_bind,
                &self.instances_layout,
                &self.opaque_instances,
                &self.joints,
            );
            render_pass.set_bind_group(1, instances_bind, &[]);
        }

        for (key, batch) in self.instances.iter().filter(|(_, batch)| batch.loaded(frame)) {
            let textures_bind = textures_bind(
                &mut self.textures,
                &self.textures_layout,
                key,
                &self.white,
                &self.flat_normal,
            );

            render_pass.set_bind_group(3, textures_bind, &[]);
            let pipelines = match (&self.depth, depth) {
                (Some([plain, cutout]), true) => {
                    if key.cutout {
                        cutout
                    } else {
                        plain
                    }
                }
                _ if key.cutout => &self.cutout,
                _ => &self.opaque,
            };
            set_mesh(render_pass, &key.mesh, pipelines);
            // The slice starts at the batch, so the draw starts at instance
            // zero, an A7 cannot draw from a base instance. The `index`
            // attribute still names the real slot for the fragment stage.
            render_pass.set_vertex_buffer(1, self.opaque_instances.elements(batch.drawn.clone()));
            let count = batch.drawn.end - batch.drawn.start;
            render_pass.draw_indexed(0..key.mesh.index_count, 0, 0..count);
            if let (Some(wireframe), false) = (&self.wireframe_pipelines, depth)
                && self.wireframe
            {
                draw_edges(render_pass, &key.mesh, wireframe, count);
            }
        }
    }

    /// Lets go of everything loaded this frame. A picture is drawn and
    /// submitted on its own, and the frame's draw that follows on the same
    /// frame number must not draw the picture's nodes again.
    pub(crate) fn forget_frame(&mut self) {
        for batch in self.instances.values_mut() {
            batch.drawn = 0..0;
        }
        self.transparent.forget();
        self.transparent_keys.clear();
        self.lines.forget();
        self.tested_lines.forget();
        self.on_top_lines.forget();
    }

    fn draw_translucent(&mut self, render_pass: &mut RenderPass) {
        let instances_bind = cached(
            &mut self.transparent_bind,
            (StorageKey::of(&self.transparent), StorageKey::of(&self.joints)),
            || instances_bind(&self.instances_layout, &self.transparent, &self.joints),
        );
        render_pass.set_bind_group(1, instances_bind, &[]);

        let range = self.transparent.range().clone();
        let stride = MeshInstance::VERTEX_LAYOUT.array_stride;

        for (i, key) in self.transparent_keys.drain(..).enumerate() {
            let textures_bind = textures_bind(
                &mut self.textures,
                &self.textures_layout,
                &key,
                &self.white,
                &self.flat_normal,
            );
            render_pass.set_bind_group(3, textures_bind, &[]);
            set_mesh(render_pass, &key.mesh, &self.translucent);
            // The draw starts at instance zero of a slice that begins at
            // this node, its `index` attribute still names the real slot.
            let start = range.start + u64::try_from(i).expect("node count fits u64") * stride;
            render_pass.set_vertex_buffer(1, self.transparent.buffer().slice(start..range.end));
            render_pass.draw_indexed(0..key.mesh.index_count, 0, 0..1);
            if let Some(wireframe) = &self.wireframe_pipelines
                && self.wireframe
            {
                draw_edges(render_pass, &key.mesh, wireframe, 1);
            }
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

/// A mesh pipeline and its skinned twin, the same shader from its two
/// vertex entries.
/// The thin lines, then the wide lines that are depth tested and the wide
/// lines on top of everything.
fn line_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    lines: &wgpu::ShaderModule,
    wide: &wgpu::ShaderModule,
) -> [RenderPipeline; 3] {
    [
        device.pipeline(
            "lines_pipeline",
            layout,
            lines,
            CompareFunction::Less,
            PrimitiveTopology::LineList,
            LINE_LAYOUTS,
        ),
        device.pipeline(
            "wide_lines_tested",
            layout,
            wide,
            CompareFunction::Less,
            PrimitiveTopology::TriangleList,
            WIDE_LINE_LAYOUTS,
        ),
        device.pipeline(
            "wide_lines_on_top",
            layout,
            wide,
            CompareFunction::Always,
            PrimitiveTopology::TriangleList,
            WIDE_LINE_LAYOUTS,
        ),
    ]
}

fn mesh_pipelines(
    device: &wgpu::Device,
    name: &str,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    draw: MeshDraw,
) -> [RenderPipeline; 2] {
    [
        device.mesh_pipeline(
            &format!("{name}_pipeline"),
            layout,
            shader,
            STATIC_LAYOUTS,
            "v_main",
            draw,
        ),
        device.mesh_pipeline(
            &format!("{name}_skinned_pipeline"),
            layout,
            shader,
            SKINNED_LAYOUTS,
            "v_skinned",
            draw,
        ),
    ]
}

fn shader(device: &wgpu::Device, label: &str, source: String) -> wgpu::ShaderModule {
    device.create_shader_module(ShaderModuleDescriptor {
        label:  label.into(),
        source: ShaderSource::Wgsl(source.into()),
    })
}
