use std::{
    cell::RefCell,
    collections::{BTreeMap, HashSet},
    f32::consts::{PI, TAU},
    sync::{Arc, Mutex},
};

use log::warn;
use wgpu::{Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, MapMode};

use crate::{
    deps::refs::{Own, Weak, main_lock::MainLock},
    gm::{
        LossyConvert, checked_usize_to_u32,
        flat::Point,
        volume::{Shape3, SkinVertex, Vec3, Vertex3D},
    },
    render::DeviceHelper,
    window::Window,
};

/// The unit meshes live for the whole process, keyed by shape kind, and
/// every node of that kind draws the same one at its own size.
static MESHES: MainLock<BTreeMap<&'static str, Own<Mesh>>> = MainLock::new();

const BALL_RINGS: usize = 16;
const BALL_SEGMENTS: usize = 32;

/// Geometry on the GPU. Indices are 16 bit, which every lane draws, so a
/// mesh holds at most 65535 vertices.
#[derive(Debug)]
pub struct Mesh {
    pub(crate) vertex_buffer: Buffer,
    /// The joints and weights of a skinned mesh, one per vertex, the
    /// second vertex buffer of the skinned pipelines.
    pub(crate) skin_buffer:   Option<Buffer>,
    pub(crate) index_buffer:  Buffer,
    pub(crate) index_count:   u32,
    /// Whether the color fades across a triangle, see
    /// `Model::from_smooth_mesh`.
    pub(crate) smooth_colors: bool,
    /// Whether its texture slides along a flow, see
    /// `Model::from_flowing_mesh`.
    pub(crate) flowing:       bool,
    /// Every edge of the triangles once, as pairs of vertex indices, and
    /// their count, what `SceneBase::show_wireframe` draws as lines. Made
    /// on the first wireframe draw, at every upload it took a third of the
    /// main thread of a game that streams its ground. The indices are read
    /// back from the GPU for it, a copy kept on the CPU for every mesh was
    /// memory only a debug view used.
    edges:                    RefCell<Edges>,
}

/// How far the edges of a mesh are, see `Mesh::edges`.
#[derive(Debug, Default)]
enum Edges {
    /// No wireframe asked for them yet.
    #[default]
    Unasked,
    /// The indices are on their way back from the GPU.
    Reading(Arc<Mutex<Option<Vec<u16>>>>),
    Ready(Buffer, u32),
}

type Geometry = (Vec<Vertex3D>, Vec<u16>);

impl Mesh {
    /// The unit mesh of a shape, a 1 by 1 by 1 box, a ball of diameter 1
    /// or a 1 by 1 plane. `Shape3::mesh_scale` sizes it per instance. A
    /// model carries its own meshes and has no unit one.
    pub fn of_shape(shape: Shape3) -> Option<Weak<Mesh>> {
        match shape {
            Shape3::Box(_) => Some(Self::named("box", box_geometry)),
            Shape3::Ball(_) => Some(Self::named("ball", ball_geometry)),
            Shape3::Plane(_) => Some(Self::named("plane", plane_geometry)),
            Shape3::Model(_) => None,
        }
    }

    fn named(name: &'static str, geometry: fn() -> Geometry) -> Weak<Mesh> {
        let meshes = MESHES.get_mut();

        if let Some(mesh) = meshes.get(name) {
            return mesh.weak();
        }

        let (vertices, indices) = geometry();
        let mesh = Own::new(Self::upload(&vertices, &indices));
        let weak = mesh.weak();
        meshes.insert(name, mesh);
        weak
    }

    pub(crate) fn upload(vertices: &[Vertex3D], indices: &[u16]) -> Self {
        let device = Window::device();
        Self {
            vertex_buffer: device.buffer(vertices, BufferUsages::VERTEX),
            skin_buffer:   None,
            index_buffer:  device.buffer(indices, BufferUsages::INDEX | BufferUsages::COPY_SRC),
            index_count:   checked_usize_to_u32(indices.len()),
            smooth_colors: false,
            flowing:       false,
            edges:         RefCell::default(),
        }
    }

    /// A mesh whose color fades across a triangle, see
    /// `Model::from_smooth_mesh`.
    pub(crate) fn upload_smooth(vertices: &[Vertex3D], indices: &[u16]) -> Self {
        Self {
            smooth_colors: true,
            ..Self::upload(vertices, indices)
        }
    }

    /// A mesh whose texture slides along a flow, see
    /// `Model::from_flowing_mesh`.
    pub(crate) fn upload_flowing(vertices: &[Vertex3D], indices: &[u16]) -> Self {
        Self {
            flowing: true,
            ..Self::upload(vertices, indices)
        }
    }

    /// The edge buffer and its index count, none until the indices came
    /// back from the GPU. A native GPU is waited for at the first ask, so
    /// the edges draw that frame. A browser answers on its own time and the
    /// edges show from the frame they landed.
    pub(crate) fn edges(&self) -> Option<(Buffer, u32)> {
        let mut state = self.edges.borrow_mut();
        if let Edges::Unasked = *state {
            *state = Edges::Reading(self.read_indices());
        }
        if let Edges::Reading(slot) = &*state {
            let indices = slot.lock().expect("the map callback does not panic").take()?;
            let edges = edges(&indices);
            *state = Edges::Ready(
                Window::device().buffer(edges.as_slice(), BufferUsages::INDEX),
                checked_usize_to_u32(edges.len()),
            );
        }
        match &*state {
            Edges::Ready(buffer, count) => Some((buffer.clone(), *count)),
            _ => None,
        }
    }

    /// Copies the index buffer into one the CPU can map, and fills the slot
    /// with the indices once the map is done.
    fn read_indices(&self) -> Arc<Mutex<Option<Vec<u16>>>> {
        let device = Window::device();
        let size = self.index_buffer.size();
        let readable = device.create_buffer(&BufferDescriptor {
            label: Some("mesh_indices_read"),
            size,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("mesh_indices_read"),
        });
        encoder.copy_buffer_to_buffer(&self.index_buffer, 0, &readable, 0, size);
        Window::queue().submit([encoder.finish()]);

        let slot = Arc::new(Mutex::new(None));
        let (filled, mapped) = (slot.clone(), readable.clone());
        let count = self.index_count as usize;
        readable.slice(..).map_async(MapMode::Read, move |result| {
            if let Err(err) = result {
                warn!("the indices of a mesh did not come back for its wireframe: {err}");
                return;
            }
            let indices = match mapped.slice(..).get_mapped_range() {
                Ok(bytes) => bytemuck::cast_slice::<u8, u16>(&bytes)[..count].to_vec(),
                Err(err) => {
                    warn!("the indices of a mesh could not be read for its wireframe: {err}");
                    return;
                }
            };
            mapped.unmap();
            *filled.lock().expect("nothing else holds the slot") = Some(indices);
        });
        #[cfg(not_wasm)]
        if let Err(err) = device.poll(wgpu::PollType::wait_indefinitely()) {
            warn!("the GPU poll for the indices of a mesh failed: {err}");
        }
        slot
    }

    /// A mesh whose vertices follow the joints of a skin, `skin` is one
    /// entry per vertex.
    pub(crate) fn upload_skinned(vertices: &[Vertex3D], skin: &[SkinVertex], indices: &[u16]) -> Self {
        assert_eq!(vertices.len(), skin.len(), "one skin vertex per vertex");
        Self {
            skin_buffer: Some(Window::device().buffer(skin, BufferUsages::VERTEX)),
            ..Self::upload(vertices, indices)
        }
    }
}

/// The edges of the triangles of `indices` as a line list, an edge two
/// triangles share only once. A mesh that has no triangle still gets
/// one degenerate pair, a buffer of 0 bytes cannot be bound.
fn edges(indices: &[u16]) -> Vec<u16> {
    let mut seen = HashSet::new();
    let mut edges = vec![];
    for triangle in indices.as_chunks::<3>().0 {
        for (from, to) in [(0, 1), (1, 2), (2, 0)] {
            let (a, b) = (triangle[from], triangle[to]);
            if seen.insert((a.min(b), a.max(b))) {
                edges.extend([a, b]);
            }
        }
    }
    if edges.is_empty() {
        edges.extend([0, 0]);
    }
    edges
}

fn index(i: usize) -> u16 {
    u16::try_from(i).expect("a mesh holds at most 65535 vertices")
}

/// One quad of a box, counter clockwise seen from outside, so it faces
/// the culling. `u` cross `v` is the outward normal.
fn face(vertices: &mut Vec<Vertex3D>, indices: &mut Vec<u16>, normal: Vec3, u: Vec3, v: Vec3) {
    let first = index(vertices.len());
    let center = normal * 0.5;
    let corners = [
        (center - u * 0.5 - v * 0.5, Point::new(0.0, 1.0)),
        (center + u * 0.5 - v * 0.5, Point::new(1.0, 1.0)),
        (center + u * 0.5 + v * 0.5, Point::new(1.0, 0.0)),
        (center - u * 0.5 + v * 0.5, Point::new(0.0, 0.0)),
    ];
    for (pos, uv) in corners {
        vertices.push(Vertex3D::new(pos, normal, uv));
    }
    indices.extend([first, first + 1, first + 2, first, first + 2, first + 3]);
}

fn box_geometry() -> Geometry {
    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);
    face(&mut vertices, &mut indices, Vec3::X, Vec3::NEG_Z, Vec3::Y);
    face(&mut vertices, &mut indices, Vec3::NEG_X, Vec3::Z, Vec3::Y);
    face(&mut vertices, &mut indices, Vec3::Y, Vec3::X, Vec3::NEG_Z);
    face(&mut vertices, &mut indices, Vec3::NEG_Y, Vec3::X, Vec3::Z);
    face(&mut vertices, &mut indices, Vec3::Z, Vec3::X, Vec3::Y);
    face(&mut vertices, &mut indices, Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y);
    (vertices, indices)
}

fn plane_geometry() -> Geometry {
    let mut vertices = Vec::with_capacity(4);
    let mut indices = Vec::with_capacity(6);
    face(&mut vertices, &mut indices, Vec3::Y, Vec3::X, Vec3::NEG_Z);
    // The face sits half a unit up, a plane sits on its own origin.
    for vertex in &mut vertices {
        vertex.pos.y = 0.0;
    }
    (vertices, indices)
}

/// A uv sphere of diameter 1, the poles on the y axis.
fn ball_geometry() -> Geometry {
    let mut vertices = Vec::with_capacity((BALL_RINGS + 1) * (BALL_SEGMENTS + 1));
    let mut indices = Vec::with_capacity(BALL_RINGS * BALL_SEGMENTS * 6);

    for ring in 0..=BALL_RINGS {
        let phi = PI * ring.lossy_convert() / BALL_RINGS.lossy_convert();
        for segment in 0..=BALL_SEGMENTS {
            let theta = TAU * segment.lossy_convert() / BALL_SEGMENTS.lossy_convert();
            let normal = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
            vertices.push(Vertex3D::new(
                normal * 0.5,
                normal,
                Point::new(
                    segment.lossy_convert() / BALL_SEGMENTS.lossy_convert(),
                    ring.lossy_convert() / BALL_RINGS.lossy_convert(),
                ),
            ));
        }
    }

    // Seen from outside a growing segment moves left, so the quad goes
    // top right, top left, bottom left, bottom right to stay counter
    // clockwise.
    for ring in 0..BALL_RINGS {
        for segment in 0..BALL_SEGMENTS {
            let top_right = index(ring * (BALL_SEGMENTS + 1) + segment);
            let top_left = top_right + 1;
            let bottom_right = index((ring + 1) * (BALL_SEGMENTS + 1) + segment);
            let bottom_left = bottom_right + 1;
            indices.extend([
                top_right,
                top_left,
                bottom_left,
                top_right,
                bottom_left,
                bottom_right,
            ]);
        }
    }

    (vertices, indices)
}

#[cfg(test)]
mod test {
    use super::*;

    fn outward(vertices: &[Vertex3D], indices: &[u16]) -> bool {
        indices.as_chunks::<3>().0.iter().all(|triangle| {
            let a = vertices[usize::from(triangle[0])].pos;
            let b = vertices[usize::from(triangle[1])].pos;
            let c = vertices[usize::from(triangle[2])].pos;
            let center = (a + b + c) / 3.0;
            let normal = (b - a).cross(c - a);
            // The pole triangles of the ball are slivers of rounding
            // error, sin of pi is not zero in f32, and carry no face.
            normal.length() < 1e-6 || normal.dot(center) > 0.0
        })
    }

    // Back faces are culled, so a triangle wound the wrong way is a hole
    // in the mesh. Every triangle of a closed unit solid faces away from
    // its center.
    #[test]
    fn box_faces_outward() {
        let (vertices, indices) = box_geometry();
        assert_eq!(vertices.len(), 24);
        assert_eq!(indices.len(), 36);
        assert!(outward(&vertices, &indices));
    }

    #[test]
    fn ball_faces_outward() {
        let (vertices, indices) = ball_geometry();
        assert!(vertices.len() < usize::from(u16::MAX));
        assert!(outward(&vertices, &indices));
    }

    #[test]
    fn plane_faces_up_from_its_origin() {
        let (vertices, indices) = plane_geometry();
        assert!(vertices.iter().all(|vertex| vertex.pos.y == 0.0 && vertex.normal == Vec3::Y));
        let a = vertices[usize::from(indices[0])].pos;
        let b = vertices[usize::from(indices[1])].pos;
        let c = vertices[usize::from(indices[2])].pos;
        assert!((b - a).cross(c - a).y > 0.0);
    }
}
