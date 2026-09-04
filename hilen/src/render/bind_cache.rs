use std::ops::Range;

use wgpu::{BindGroup, Buffer};

use crate::render::vec_buffer::VecBuffer;

/// A bind group kept with what it was made from, so a frame that binds
/// the same resources again reuses it instead of making a new one. wgpu
/// resources compare by identity, so a buffer that was reallocated or a
/// map that was remade misses on its own.
pub(crate) struct CachedBind<K> {
    key:  K,
    bind: BindGroup,
}

/// The bind group for `key` out of `slot`, made by `make` when the slot
/// is empty or holds one for other resources.
pub(crate) fn cached<K: PartialEq>(
    slot: &mut Option<CachedBind<K>>,
    key: K,
    make: impl FnOnce() -> BindGroup,
) -> &BindGroup {
    if slot.as_ref().is_some_and(|cached| cached.key != key) {
        *slot = None;
    }
    &slot.get_or_insert_with(|| CachedBind { key, bind: make() }).bind
}

/// What a storage binding over a `VecBuffer` names: the buffer and the
/// range its last load landed in. The range moves when the count
/// changes and the buffer when it grows, so a key that still matches
/// means the shader reads the same elements the draw uses.
#[derive(PartialEq)]
pub(crate) struct StorageKey {
    buffer: Buffer,
    range:  Range<u64>,
}

impl StorageKey {
    pub(crate) fn of<T>(buffer: &VecBuffer<T>) -> Self {
        Self {
            buffer: buffer.buffer().clone(),
            range:  buffer.range().clone(),
        }
    }
}
