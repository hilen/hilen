//! The draws of the opaque, the cut out and the translucent nodes of
//! `MeshPipeline`.

use wgpu::RenderPass;

use super::{
    MeshKey, MeshPipeline,
    builders::{mesh_pipelines, smooth_pipeline},
    draw_edges, set_mesh,
};
use crate::{
    render::{
        bind_cache::{StorageKey, cached},
        data::MeshInstance,
        device_helper::MeshDraw,
        pipelines::mesh_binds::{instances_bind, opaque_bind, storage_bind, textures_bind, view_bind},
        vertex_layout::VertexLayout,
    },
    scene::Sky,
    window::Window,
};

/// The rounds of `draw_batches`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Round {
    /// Every node, with its distance as the color.
    Photo,
    /// Every node but the cut out ones of the next 2 rounds.
    Solid,
    CutoutDepth,
    CutoutColor,
}

impl Round {
    /// A mesh with fading colors draws its cut out node in 1 go,
    /// `f_smooth_cutout`.
    fn holds(self, key: &MeshKey) -> bool {
        let twice = key.cutout && !key.mesh.smooth_colors;
        match self {
            Self::Photo => true,
            Self::Solid => !twice,
            Self::CutoutDepth | Self::CutoutColor => twice,
        }
    }
}

impl MeshPipeline {
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
        if self.smooth.is_some() && self.smooth_depth_cutout.is_none() {
            self.smooth_depth_cutout = Some(smooth_pipeline(
                Window::device(),
                "mesh_smooth_depth_cutout_pipeline",
                &self.mesh_layout,
                &self.mesh_shader,
                MeshDraw::SmoothDepthCutout,
            ));
        }

        // The sky of the frame, so the cached bind stays the frame's.
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
        self.draw_batches(render_pass, frame, true);
    }

    /// Draws every opaque and cut out batch loaded this `frame`, through
    /// the depth pipelines when `depth` is set.
    pub(super) fn draw_batches(&mut self, render_pass: &mut RenderPass, frame: u64, depth: bool) {
        let lights_bind = cached(&mut self.lights_bind, StorageKey::of(&self.lights), || {
            storage_bind("mesh_lights_bind", &self.lights_layout, &self.lights)
        });
        render_pass.set_bind_group(2, lights_bind, &[]);

        if self.instances.values().any(|batch| batch.drawn_in(frame, 0).is_some()) {
            let instances_bind = opaque_bind(
                &mut self.opaque_bind,
                &self.instances_layout,
                &self.opaque_instances,
                &self.joints,
            );
            render_pass.set_bind_group(1, instances_bind, &[]);
        }

        // Solid first, its depth spares the cut out nodes behind it.
        let rounds: &[Round] = if depth {
            &[Round::Photo]
        } else {
            &[Round::Solid, Round::CutoutDepth, Round::CutoutColor]
        };
        for &round in rounds {
            for (key, drawn) in self
                .instances
                .iter()
                .filter(|(key, _)| round.holds(key))
                .filter_map(|(key, batch)| Some((key, batch.drawn_in(frame, 0)?)))
            {
                let textures_bind = textures_bind(
                    &mut self.textures,
                    &self.textures_layout,
                    key,
                    &self.white,
                    &self.flat_normal,
                );

                render_pass.set_bind_group(3, textures_bind, &[]);
                let pipelines = match round {
                    Round::Photo => {
                        let [plain, cutout] = self.depth.as_ref().expect("made before the photo draws");
                        if key.cutout { cutout } else { plain }
                    }
                    Round::Solid => &self.opaque,
                    Round::CutoutDepth => &self.cutout_depth,
                    Round::CutoutColor => &self.cutout,
                };
                set_mesh(render_pass, &key.mesh, pipelines);
                if key.mesh.smooth_colors {
                    let smooth = self.smooth.as_ref().expect("made when the node was added");
                    let pipeline = match (depth, key.cutout) {
                        (false, false) => Some(&smooth.opaque),
                        (false, true) => Some(&smooth.cutout),
                        // `f_depth` reads only the position, the plain pipeline
                        // set above draws this mesh the same.
                        (true, false) => None,
                        (true, true) => self.smooth_depth_cutout.as_ref(),
                    };
                    if let Some(pipeline) = pipeline {
                        render_pass.set_pipeline(pipeline);
                    }
                }
                // The slice starts at the batch, so the draw starts at instance
                // zero, an A7 cannot draw from a base instance. The `index`
                // attribute still names the real slot for the fragment stage.
                render_pass.set_vertex_buffer(1, self.opaque_instances.elements(drawn.clone()));
                let count = drawn.end - drawn.start;
                render_pass.draw_indexed(0..key.mesh.index_count, 0, 0..count);
                if let (Some(wireframe), false) = (&self.wireframe_pipelines, depth)
                    && self.wireframe
                    && round != Round::CutoutDepth
                {
                    draw_edges(render_pass, &key.mesh, wireframe, count);
                }
            }
        }
    }

    pub(super) fn draw_translucent(&mut self, render_pass: &mut RenderPass) {
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
            if key.mesh.smooth_colors {
                let smooth = self.smooth.as_ref().expect("made when the node was added");
                render_pass.set_pipeline(&smooth.translucent);
            }
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
