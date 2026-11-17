use bytemuck::{Pod, Zeroable};
use wgpu::{BufferAddress, VertexBufferLayout, VertexStepMode};

use crate::render::{data::UIRectInstance, vertex_layout::VertexLayout};

/// A rect instance for `ui_backdrop.wgsl`, with a triangle on its top or
/// bottom edge. The rect fields come first, so the vertex stage reads them
/// at the same offsets as a plain [`UIRectInstance`].
#[repr(C)]
#[derive(Debug, Copy, Clone, Zeroable, Pod)]
pub struct UIBackdropInstance {
    pub rect: UIRectInstance,
    /// The middle of the triangle from the left edge, half its width, its
    /// height, and its edge: 1 for the bottom, -1 for the top, 0 for none.
    /// All in points, like the rect.
    pub tail: [f32; 4],
}

impl VertexLayout for UIBackdropInstance {
    const ATTRIBS: &'static [wgpu::VertexAttribute] = UIRectInstance::ATTRIBS;
    const VERTEX_LAYOUT: VertexBufferLayout<'static> = VertexBufferLayout {
        array_stride: size_of::<Self>() as BufferAddress,
        step_mode:    VertexStepMode::Instance,
        attributes:   Self::ATTRIBS,
    };
}

#[cfg(test)]
mod test {
    use std::mem::offset_of;

    use super::UIBackdropInstance;

    /// `ui_backdrop.wgsl` names these offsets in its `std430` storage struct.
    #[test]
    fn std430_layout() {
        assert_eq!(offset_of!(UIBackdropInstance, rect), 0);
        assert_eq!(offset_of!(UIBackdropInstance, tail), 80);
        assert_eq!(size_of::<UIBackdropInstance>(), 96);
    }
}
