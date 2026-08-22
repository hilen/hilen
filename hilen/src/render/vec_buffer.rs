use std::ops::Range;

use bytemuck::{Pod, cast_slice};
use wgpu::{Buffer, BufferDescriptor, BufferSlice, BufferUsages};

use crate::window::Window;

/// Instances are bound as a storage buffer as well as a vertex buffer, and a
/// storage binding may only start at a multiple of this.
fn storage_alignment() -> u64 {
    Window::device().limits().min_storage_buffer_offset_alignment.into()
}

/// CPU-side instance list backed by a persistent GPU buffer.
///
/// `load()` can be called several times per frame (once per pipeline flush).
/// All queued `write_buffer` calls execute together at submit, before the
/// render pass runs, so every flush must land at its own offset — the buffer
/// is bump-allocated through the frame and the cursor resets when
/// `Window::render_frame()` changes.
#[derive(Debug)]
pub(crate) struct VecBuffer<T> {
    len:    u32,
    data:   Vec<T>,
    buffer: Buffer,
    range:  Range<u64>,
    offset: u64,
    frame:  u64,
}

impl<T> VecBuffer<T> {
    pub(crate) fn push(&mut self, val: T) {
        self.data.push(val);
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    pub(crate) fn slice(&self) -> BufferSlice<'_> {
        self.buffer.slice(self.range.clone())
    }

    pub(crate) fn buffer(&self) -> &Buffer {
        &self.buffer
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

        if self.offset + size > self.buffer.size() {
            // Earlier flushes of this frame keep the old buffer alive through
            // their recorded draws, so replacing it mid-frame is safe.
            self.buffer = Window::device().create_buffer(&BufferDescriptor {
                label:              Some("VecBuffer"),
                size:               size.max(self.buffer.size() * 2).max(4096),
                usage:              BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            self.offset = 0;
        }

        Window::queue().write_buffer(&self.buffer, self.offset, bytes);

        self.range = self.offset..self.offset + size;
        self.offset = self.range.end.next_multiple_of(storage_alignment());
        self.len = self.data.len().try_into().unwrap();
        self.data.clear();
    }
}

impl<T> Default for VecBuffer<T> {
    fn default() -> Self {
        Self {
            len:    0,
            data:   vec![],
            buffer: Window::device().create_buffer(&BufferDescriptor {
                label:              Some("VecBuffer"),
                size:               0,
                usage:              BufferUsages::VERTEX | BufferUsages::STORAGE | BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            range:  0..0,
            offset: 0,
            frame:  0,
        }
    }
}
