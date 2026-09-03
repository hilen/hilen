use std::{borrow::Cow, num::NonZeroU64, ops::Range, sync::OnceLock};

use bytemuck::Pod;
#[cfg(feature = "level")]
use wgpu::BindGroup;
use wgpu::{
    BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry,
    BindingResource, BindingType, BufferBinding, BufferBindingType, RenderPass, ShaderStages,
};

#[cfg(feature = "level")]
use crate::render::device_helper::DeviceHelper;
use crate::{
    render::vec_buffer::VecBuffer,
    window::{BufferUsages, Window},
};

#[cfg(feature = "level")]
pub(crate) fn make_bind<T: Pod>(data: &T, layout: &BindGroupLayout) -> BindGroup {
    let device = Window::device();
    let buffer = device.buffer(data, BufferUsages::UNIFORM);
    device.bind(&buffer, layout)
}

pub(crate) fn make_uniform_layout(name: &str, shader: ShaderStages) -> BindGroupLayout {
    Window::device().create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   name.into(),
        entries: &[BindGroupLayoutEntry {
            binding:    0,
            visibility: shader,
            ty:         BindingType::Buffer {
                ty:                 BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size:   None,
            },
            count:      None,
        }],
    })
}

/// How a UI pipeline hands instance data to the fragment stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InstanceBinding {
    /// A read only storage buffer over the whole flush.
    Storage,
    /// A uniform array of one chunk, drawn chunk by chunk, see
    /// `InstanceChunks`. WebGL2 has no storage buffers at all.
    Uniform,
}

impl InstanceBinding {
    /// What this device gets. Storage where it has storage buffers, its
    /// device reports zero per stage on WebGL2. `HILEN_UNIFORM_INSTANCES=1`,
    /// or `hilen_uniform_instances` in the page query, forces the uniform
    /// path on a device that has storage, so the UI suite can cover it
    /// anywhere.
    pub(crate) fn device() -> Self {
        static BINDING: OnceLock<InstanceBinding> = OnceLock::new();
        *BINDING.get_or_init(|| {
            #[cfg(wasm)]
            let forced = crate::web::query_flag("hilen_uniform_instances");
            #[cfg(not_wasm)]
            let forced = std::env::var("HILEN_UNIFORM_INSTANCES").is_ok();

            if forced || Window::device().limits().max_storage_buffers_per_shader_stage == 0 {
                Self::Uniform
            } else {
                Self::Storage
            }
        })
    }

    /// The bind group layout of the instance data.
    pub(crate) fn layout(self, name: &str, shader: ShaderStages) -> BindGroupLayout {
        match self {
            Self::Storage => make_storage_layout(name, shader),
            Self::Uniform => make_uniform_layout(name, shader),
        }
    }

    /// What an instance buffer is created with. A uniform device gets no
    /// storage usage, WebGL2 has no buffer target for it.
    pub(crate) fn buffer_usages(self) -> BufferUsages {
        let binding = match self {
            Self::Storage => BufferUsages::STORAGE,
            Self::Uniform => BufferUsages::UNIFORM,
        };
        BufferUsages::VERTEX | BufferUsages::COPY_DST | binding
    }

    /// The alignment an instance flush has to land at to bind.
    pub(crate) fn alignment(self) -> u64 {
        let limits = Window::device().limits();
        match self {
            Self::Storage => limits.min_storage_buffer_offset_alignment.into(),
            Self::Uniform => limits.min_uniform_buffer_offset_alignment.into(),
        }
    }

    /// Bytes past the written instances the buffer has to keep. A uniform
    /// chunk binds a whole window even when the last instances fill only
    /// part of it, and a binding past the buffer end is a validation error.
    pub(crate) fn tail_padding(self) -> u64 {
        match self {
            Self::Storage => 0,
            Self::Uniform => CHUNK_BYTES_CAP,
        }
    }
}

/// The largest uniform binding a chunk of instances takes. WebGL2 caps a
/// uniform binding at 16 KiB, and a D3D constant buffer at 64 KiB, so the
/// uniform path uses the same chunk size on every device.
const CHUNK_BYTES_CAP: u64 = 16 << 10;

/// How a flush of instances splits into uniform bindings. A chunk holds the
/// most instances that fit under the binding cap while its byte size stays
/// a multiple of the offset alignment, so every chunk after the first still
/// starts where a binding may. Each chunk binds its own window of the buffer
/// and draws its instances from index zero, which keeps `instance_index` the
/// index into the bound window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InstanceChunks {
    stride:    u64,
    per_chunk: u32,
}

impl InstanceChunks {
    pub(crate) fn new(stride: u64) -> Self {
        let limits = Window::device().limits();
        Self::with_limits(
            stride,
            limits.max_uniform_buffer_binding_size.min(CHUNK_BYTES_CAP),
            limits.min_uniform_buffer_offset_alignment.into(),
        )
    }

    fn with_limits(stride: u64, max_binding: u64, alignment: u64) -> Self {
        // The chunk byte size is a multiple of the alignment exactly when
        // the instance count is a multiple of this.
        let step = alignment / gcd(stride, alignment);
        let per_chunk = max_binding / stride / step * step;

        assert!(
            per_chunk > 0,
            "an instance of {stride} bytes cannot bind as a uniform under {max_binding} bytes"
        );

        Self {
            stride,
            per_chunk: per_chunk.try_into().expect("chunk fits u32"),
        }
    }

    pub(crate) fn per_chunk(self) -> u32 {
        self.per_chunk
    }

    pub(crate) fn bytes(self) -> u64 {
        u64::from(self.per_chunk) * self.stride
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// The instance declaration of a UI shader, rewritten for the uniform path.
/// `var<storage, read> instances: array<T>;` becomes a uniform array of one
/// chunk, which is what the chunked draw binds. The storage binding gets the
/// source back untouched.
pub(crate) fn instances_shader(source: &str, stride: u64, binding: InstanceBinding) -> Cow<'_, str> {
    const STORAGE: &str = "var<storage, read> instances: array<";

    if binding == InstanceBinding::Storage {
        return source.into();
    }

    let start = source
        .find(STORAGE)
        .expect("a UI shader declares its instances as a storage array");
    let type_start = start + STORAGE.len();
    let type_end = type_start + source[type_start..].find(">;").expect("instance array declaration ends");
    let per_chunk = InstanceChunks::new(stride).per_chunk();

    format!(
        "{}var<uniform> instances: array<{}, {per_chunk}>;{}",
        &source[..start],
        &source[type_start..type_end],
        &source[type_end + 2..]
    )
    .into()
}

/// Binds the loaded instances at `group` and draws `vertices` over them,
/// with the instance vertex buffer at `vertex_slot`. The storage binding
/// takes one draw over the flush. The uniform binding goes chunk by chunk,
/// each chunk binding its own window of the buffer as bind group and
/// vertex slice and drawing from instance zero.
pub(crate) fn draw_instances<T: Pod>(
    pass: &mut RenderPass,
    layout: &BindGroupLayout,
    label: &str,
    group: u32,
    vertex_slot: u32,
    vertices: Range<u32>,
    instances: &VecBuffer<T>,
) {
    let range = instances.range();
    let bind = |offset: u64, size: u64| {
        Window::device().create_bind_group(&BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[BindGroupEntry {
                binding:  0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: instances.buffer(),
                    offset,
                    size: NonZeroU64::new(size),
                }),
            }],
        })
    };

    if instances.binding() == InstanceBinding::Storage {
        pass.set_bind_group(group, &bind(range.start, range.end - range.start), &[]);
        pass.set_vertex_buffer(vertex_slot, instances.slice());
        pass.draw(vertices, 0..instances.len());
        return;
    }

    let chunks = InstanceChunks::new(size_of::<T>() as u64);
    let mut offset = range.start;
    let mut left = instances.len();

    while left > 0 {
        let count = left.min(chunks.per_chunk());
        pass.set_bind_group(group, &bind(offset, chunks.bytes()), &[]);
        pass.set_vertex_buffer(vertex_slot, instances.buffer().slice(offset..range.end));
        pass.draw(vertices.clone(), 0..count);
        offset += chunks.bytes();
        left -= count;
    }
}

/// Read only instance data for the fragment stage. Everything a rect shader
/// needs per instance is constant across the shape, so carrying it as an inter
/// stage varying costs a payload that an A7 GPU silently refuses to draw, see
/// `docs/ios.md`. The fragment reads it from here instead and only `uv` and the
/// instance index cross the stage boundary.
pub(crate) fn make_storage_layout(name: &str, shader: ShaderStages) -> BindGroupLayout {
    Window::device().create_bind_group_layout(&BindGroupLayoutDescriptor {
        label:   name.into(),
        entries: &[BindGroupLayoutEntry {
            binding:    0,
            visibility: shader,
            ty:         BindingType::Buffer {
                ty:                 BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size:   None,
            },
            count:      None,
        }],
    })
}

#[cfg(test)]
mod test {
    use super::InstanceChunks;

    /// The WebGL2 limits, a 16 KiB binding at 256 byte offsets.
    fn webgl2(stride: u64) -> InstanceChunks {
        InstanceChunks::with_limits(stride, 16 << 10, 256)
    }

    #[test]
    fn chunk_fills_the_binding_when_the_stride_divides_it() {
        assert_eq!(webgl2(64).per_chunk(), 256);
        assert_eq!(webgl2(128).per_chunk(), 128);
        assert_eq!(webgl2(16).bytes(), 16 << 10);
    }

    #[test]
    fn chunk_bytes_stay_aligned_for_an_odd_stride() {
        // 204 rects of 80 bytes fit, but 192 is the largest count whose
        // byte size lands on a 256 byte boundary.
        let rects = webgl2(80);
        assert_eq!(rects.per_chunk(), 192);
        assert_eq!(rects.bytes() % 256, 0);

        let shadows = webgl2(48);
        assert_eq!(shadows.per_chunk(), 336);
        assert_eq!(shadows.bytes() % 256, 0);
    }

    #[test]
    #[should_panic(expected = "cannot bind as a uniform")]
    fn an_instance_over_the_binding_cap_is_refused() {
        InstanceChunks::with_limits(32 << 10, 16 << 10, 256);
    }
}
