use wgpu::{
    CommandEncoder, LoadOp, Operations, RenderPass, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, RenderPassDescriptor, RenderPassTimestampWrites, StoreOp, TextureView,
};

use crate::gm::color::Color;

/// One frame's render encoding. Owns the encoder and the render pass
/// together, which `forget_lifetime` makes possible, so the frame can
/// be split into several passes to sample the already drawn scene.
/// wgpu checks the pass and encoder use order at runtime.
///
/// The scene view is what the passes render into. When the frame has
/// to be sampled mid frame the scene is an intermediate texture and
/// `present_view` holds the real surface, which gets one full screen
/// copy at the end, encoded by the drawer through `present_pass`.
pub struct RenderFrame {
    encoder: CommandEncoder,
    pass:    Option<RenderPass<'static>>,

    scene_view:   TextureView,
    depth_view:   TextureView,
    /// The multisampled color target. When present the passes render
    /// into it and resolve to `scene_view` at every pass end, so the
    /// scene view always holds the resolved frame for sampling and
    /// readback.
    msaa_view:    Option<TextureView>,
    present_view: Option<TextureView>,
}

impl RenderFrame {
    pub fn new(
        mut encoder: CommandEncoder,
        scene_view: TextureView,
        present_view: Option<TextureView>,
        depth_view: TextureView,
        msaa_view: Option<TextureView>,
        clear_color: Color,
        timestamp_writes: Option<RenderPassTimestampWrites>,
    ) -> Self {
        // The bench GPU timer measures the first pass only. The
        // benchmark never blurs, so its frame is exactly one pass.
        let pass = begin_pass(
            &mut encoder,
            msaa_view.as_ref().unwrap_or(&scene_view),
            msaa_view.as_ref().map(|_| &scene_view),
            &depth_view,
            LoadOp::Clear(wgpu::Color {
                r: f64::from(clear_color.r),
                g: f64::from(clear_color.g),
                b: f64::from(clear_color.b),
                a: f64::from(clear_color.a),
            }),
            true,
            timestamp_writes,
        );

        Self {
            encoder,
            pass: Some(pass),
            scene_view,
            depth_view,
            msaa_view,
            present_view,
        }
    }

    /// The current pass. After a split a new pass is opened that keeps
    /// the color and depth drawn so far.
    pub(crate) fn pass(&mut self) -> &mut RenderPass<'static> {
        if self.pass.is_none() {
            self.pass = Some(begin_pass(
                &mut self.encoder,
                self.msaa_view.as_ref().unwrap_or(&self.scene_view),
                self.msaa_view.as_ref().map(|_| &self.scene_view),
                &self.depth_view,
                LoadOp::Load,
                false,
                None,
            ));
        }

        self.pass.as_mut().expect("pass just ensured")
    }

    /// Ends the current pass so the scene texture can be read. Returns
    /// the encoder for the reading passes and the scene view to read.
    /// The next `pass` call continues drawing on top.
    pub(crate) fn split(&mut self) -> (&mut CommandEncoder, &TextureView) {
        self.pass = None;
        (&mut self.encoder, &self.scene_view)
    }

    pub(crate) fn scene_view(&self) -> &TextureView {
        &self.scene_view
    }

    /// When the frame rendered into an intermediate texture, ends the
    /// scene pass and opens the pass on the real surface. The caller
    /// draws the full screen copy into it. `None` when the frame
    /// rendered straight to the target.
    pub(crate) fn present_pass(&mut self) -> Option<&mut RenderPass<'static>> {
        let present_view = self.present_view.as_ref()?;

        self.pass = None;
        self.pass = Some(
            self.encoder
                .begin_render_pass(&RenderPassDescriptor {
                    label:                    Some("Present Pass"),
                    color_attachments:        &[Some(RenderPassColorAttachment {
                        view:           present_view,
                        depth_slice:    None,
                        resolve_target: None,
                        ops:            Operations {
                            load:  LoadOp::Clear(wgpu::Color::BLACK),
                            store: StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set:      None,
                    timestamp_writes:         None,
                    multiview_mask:           None,
                })
                .forget_lifetime(),
        );

        self.pass.as_mut()
    }

    pub fn finish(mut self) -> CommandEncoder {
        self.pass = None;
        self.encoder
    }
}

fn begin_pass(
    encoder: &mut CommandEncoder,
    color: &TextureView,
    resolve_target: Option<&TextureView>,
    depth: &TextureView,
    color_load: LoadOp<wgpu::Color>,
    // The first pass of a frame clears depth and stencil together, a
    // reopened pass keeps both.
    clear_depth_stencil: bool,
    timestamp_writes: Option<RenderPassTimestampWrites>,
) -> RenderPass<'static> {
    encoder
        .begin_render_pass(&RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: color,
                depth_slice: None,
                resolve_target,
                // The multisampled attachment is stored, not discarded,
                // so a reopened pass after a blur split can load it and
                // keep drawing on the samples.
                ops: Operations {
                    load:  color_load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view:        depth,
                depth_ops:   Some(Operations {
                    load:  if clear_depth_stencil {
                        LoadOp::Clear(1.0)
                    } else {
                        LoadOp::Load
                    },
                    store: StoreOp::Store,
                }),
                stencil_ops: Some(Operations {
                    load:  if clear_depth_stencil {
                        LoadOp::Clear(0)
                    } else {
                        LoadOp::Load
                    },
                    store: StoreOp::Store,
                }),
            }),
            occlusion_query_set: None,
            timestamp_writes,
            multiview_mask: None,
        })
        .forget_lifetime()
}
