use std::ops::Range;

use bytemuck::{Pod, cast_slice};
use wgpu::{Buffer, BufferDescriptor, BufferSlice};

use crate::{render::uniform::InstanceBinding, window::Window};

/// CPU-side instance list backed by a persistent GPU buffer.
///
/// `load()` can be called several times per frame (once per pipeline flush).
/// All queued `write_buffer` calls execute together at submit, before the
/// render pass runs, so every flush must land at its own offset — the buffer
/// is bump-allocated through the frame and the cursor resets when
/// `Window::render_frame()` changes.
#[derive(Debug)]
pub(crate) struct VecBuffer<T> {
    len:     u32,
    data:    Vec<T>,
    buffer:  Buffer,
    range:   Range<u64>,
    offset:  u64,
    frame:   u64,
    binding: InstanceBinding,
}

impl<T> VecBuffer<T> {
    /// A buffer bound the given way, whatever the device would pick.
    pub(crate) fn with_binding(binding: InstanceBinding) -> Self {
        Self {
            len: 0,
            data: vec![],
            buffer: Self::make_buffer(0, binding),
            range: 0..0,
            offset: 0,
            frame: 0,
            binding,
        }
    }

    fn make_buffer(size: u64, binding: InstanceBinding) -> Buffer {
        Window::device().create_buffer(&BufferDescriptor {
            label: Some("VecBuffer"),
            size,
            usage: binding.buffer_usages(),
            mapped_at_creation: false,
        })
    }

    pub(crate) fn binding(&self) -> InstanceBinding {
        self.binding
    }

    pub(crate) fn push(&mut self, val: T) {
        self.data.push(val);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Elements pushed since the last `load()`, the index the next push
    /// lands at.
    pub(crate) fn pending(&self) -> u32 {
        self.data.len().try_into().unwrap()
    }

    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    /// Whether the last `load()` landed any element. `is_empty` asks
    /// about the pushes since, not about what the frame draws.
    pub(crate) fn has_loaded(&self) -> bool {
        self.len > 0
    }

    pub(crate) fn slice(&self) -> BufferSlice<'_> {
        self.buffer.slice(self.range.clone())
    }

    pub(crate) fn buffer(&self) -> &Buffer {
        &self.buffer
    }

    /// The last frame `load()` ran on.
    pub(crate) fn frame(&self) -> u64 {
        self.frame
    }

    /// Bytes the last `load()` landed at. A storage binding needs this to point
    /// the shader at the same instances the vertex stage draws.
    pub(crate) fn range(&self) -> &Range<u64> {
        &self.range
    }
}

impl<T: Pod> VecBuffer<T> {
    pub(crate) fn load(&mut self) {
        let frame = Window::render_frame();

        if self.frame != frame {
            self.frame = frame;
            self.offset = 0;
        }

        let bytes: &[u8] = cast_slice(self.data.as_slice());
        let size: u64 = bytes.len().try_into().unwrap();

        let padding = self.binding.tail_padding();

        if self.offset + size + padding > self.buffer.size() {
            // Earlier flushes of this frame keep the old buffer alive through
            // their recorded draws, so replacing it mid-frame is safe.
            self.buffer = Self::make_buffer(
                (size + padding).max(self.buffer.size() * 2).max(4096),
                self.binding,
            );
            self.offset = 0;
        }

        Window::queue().write_buffer(&self.buffer, self.offset, bytes);

        self.range = self.offset..self.offset + size;
        self.offset = self.range.end.next_multiple_of(self.binding.alignment());
        self.len = self.data.len().try_into().unwrap();
        self.data.clear();
    }
}

impl<T> Default for VecBuffer<T> {
    fn default() -> Self {
        Self::with_binding(InstanceBinding::device())
    }
}
