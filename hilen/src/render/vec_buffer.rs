use std::{cell::RefCell, mem::take, ops::Range};

use bytemuck::{Pod, cast_slice};
use rustc_hash::FxHashMap;
use wgpu::{BindGroup, Buffer, BufferDescriptor, BufferSlice};

use crate::{deps::refs::main_lock::MainLock, render::uniform::InstanceBinding, window::Window};

/// More bind groups than this in 1 buffer are the windows of a frame that
/// keeps changing, not ones that come again, so the list starts over.
const KEPT_BINDS: usize = 256;

/// The bytes that `load()` calls put into 1 buffer since the last
/// `upload_pending`, sent to the GPU in 1 write.
struct Upload {
    buffer: Buffer,
    offset: u64,
    bytes:  Vec<u8>,
}

/// A frame flushes its pipelines many times, and a write of its own for
/// every flush made a staging buffer each time, most of the cost of a
/// flush. The writes of a frame run together at submit anyway, so they are
/// gathered here and sent once per buffer.
#[derive(Default)]
struct Uploads {
    waiting: Vec<Upload>,
    /// Goes up at every `upload_pending`, so a `VecBuffer` can tell that
    /// the place it wrote to last is gone.
    round:   u64,
    /// Emptied byte lists, kept for their memory.
    spare:   Vec<Vec<u8>>,
}

static UPLOADS: MainLock<Uploads> = MainLock::new();

/// Sends what the `load()` calls gathered. Called before every submit of
/// commands that draw from a `VecBuffer`.
pub(crate) fn upload_pending() {
    let uploads = UPLOADS.get_mut();
    uploads.round += 1;

    for mut upload in take(&mut uploads.waiting) {
        Window::queue().write_buffer(&upload.buffer, upload.offset, &upload.bytes);
        upload.bytes.clear();
        uploads.spare.push(upload.bytes);
    }
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
    len:     u32,
    data:    Vec<T>,
    buffer:  Buffer,
    range:   Range<u64>,
    offset:  u64,
    frame:   u64,
    binding: InstanceBinding,
    /// Where the last `load()` put its bytes, the round of `Uploads` and
    /// the place in its list.
    upload:  Option<(u64, usize)>,
    /// The bind groups made for windows of the buffer, by offset and
    /// size. A screen that does not change flushes the same windows in
    /// every frame, and making a bind group costs more than finding one.
    binds:   RefCell<FxHashMap<(u64, u64), BindGroup>>,
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
            upload: None,
            binds: RefCell::default(),
        }
    }

    /// The bind group of the window `offset..offset + size` of the buffer,
    /// made by `make` when this window was not bound before.
    pub(crate) fn with_bind(
        &self,
        offset: u64,
        size: u64,
        make: impl FnOnce() -> BindGroup,
        with: impl FnOnce(&BindGroup),
    ) {
        let mut binds = self.binds.borrow_mut();
        if binds.len() > KEPT_BINDS {
            binds.clear();
        }
        with(binds.entry((offset, size)).or_insert_with(make));
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

    /// The elements pushed since the last `load()`.
    pub(crate) fn queued(&self) -> &[T] {
        &self.data
    }

    /// Elements pushed since the last `load()`, the index the next push
    /// lands at.
    #[cfg(feature = "scene")]
    pub(crate) fn pending(&self) -> u32 {
        self.data.len().try_into().unwrap()
    }

    pub(crate) fn len(&self) -> u32 {
        self.len
    }

    /// Whether the last `load()` landed any element. `is_empty` asks
    /// about the pushes since, not about what the frame draws.
    #[cfg(feature = "scene")]
    pub(crate) fn has_loaded(&self) -> bool {
        self.len > 0
    }

    /// Drops what the last `load()` landed, so `has_loaded` is false until
    /// the next one. The buffer and its cursor stay.
    #[cfg(feature = "scene")]
    pub(crate) fn forget(&mut self) {
        self.len = 0;
    }

    pub(crate) fn slice(&self) -> BufferSlice<'_> {
        self.buffer.slice(self.range.clone())
    }

    /// The elements `range` of the last `load()`, counted from its first.
    #[cfg(feature = "scene")]
    pub(crate) fn elements(&self, range: Range<u32>) -> BufferSlice<'_> {
        let stride: u64 = size_of::<T>().try_into().unwrap();
        let start = self.range.start + u64::from(range.start) * stride;
        let end = self.range.start + u64::from(range.end) * stride;
        self.buffer.slice(start..end)
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
            self.upload = None;
            self.binds.borrow_mut().clear();
        }

        gather(&self.buffer, self.offset, &mut self.upload, bytes);

        self.range = self.offset..self.offset + size;
        self.offset = self.range.end.next_multiple_of(self.binding.alignment());
        self.len = self.data.len().try_into().unwrap();
        self.data.clear();
    }
}

/// Adds `bytes` at `offset` of `buffer` to what waits for the GPU. They
/// join the bytes of the load before, `last`, when those went into the same
/// buffer in this round, with zeros in the gap that the alignment leaves.
fn gather(buffer: &Buffer, offset: u64, last: &mut Option<(u64, usize)>, bytes: &[u8]) {
    let uploads = UPLOADS.get_mut();

    let joined = last.and_then(|(round, place)| {
        let upload = uploads.waiting.get(place)?;
        let end = upload.offset + u64::try_from(upload.bytes.len()).ok()?;
        (round == uploads.round && upload.buffer == *buffer && end <= offset).then_some(place)
    });

    let place = joined.unwrap_or_else(|| {
        uploads.waiting.push(Upload {
            buffer: buffer.clone(),
            offset,
            bytes: uploads.spare.pop().unwrap_or_default(),
        });
        uploads.waiting.len() - 1
    });

    let upload = &mut uploads.waiting[place];
    let start = usize::try_from(offset - upload.offset).expect("an instance buffer fits memory");
    upload.bytes.resize(start, 0);
    upload.bytes.extend_from_slice(bytes);
    *last = Some((uploads.round, place));
}

impl<T> Default for VecBuffer<T> {
    fn default() -> Self {
        Self::with_binding(InstanceBinding::device())
    }
}
