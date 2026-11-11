use anyhow::{Context, Result, bail, ensure};

use super::{MeshData, PreparedMesh, images, merge, parse::parse_glb};
use crate::gm::{
    flat::Point,
    volume::{Vec3, Vertex3D},
};

/// The 80 bytes of a binary STL before its triangle count.
const HEADER: usize = 80;
/// A triangle of a binary STL: the normal, 3 corners and 2 spare bytes.
const TRIANGLE: usize = 50;

impl MeshData {
    /// The triangles of an STL file, binary or ASCII, in the units and the
    /// axes of the file. Each triangle gets 3 vertices of its own with the
    /// normal of its face, worked out from its corners, since many files
    /// write a normal of 0. A corner order counter clockwise seen from
    /// outside is what STL asks for, the same as `MeshData`. Any thread. A
    /// file that is no STL, or one cut short, is an error.
    pub fn from_stl(data: &[u8]) -> Result<Self> {
        let triangles = if is_binary(data) {
            binary(data)
        } else {
            ascii(data)?
        };
        ensure!(!triangles.is_empty(), "the STL has no triangle");
        let mut mesh = MeshData {
            vertices: Vec::with_capacity(triangles.len() * 3),
            indices:  Vec::with_capacity(triangles.len() * 3),
        };
        for corners in triangles {
            let normal = face_normal(corners);
            for corner in corners {
                let index = u32::try_from(mesh.vertices.len()).context("the STL has too many triangles")?;
                mesh.vertices.push(Vertex3D::new(corner, normal, Point::default()));
                mesh.indices.push(index);
            }
        }
        Ok(mesh)
    }
}

impl PreparedMesh {
    /// A `.glb` from bytes, made ready to upload with
    /// `Model::from_prepared`. Unlike `Model::load`, a file that does not
    /// parse is an error, not a panic, for a model the app did not ship.
    /// Any thread.
    pub fn from_glb(data: &[u8], name: &str) -> Result<Self> {
        let mut source = merge::static_parts(parse_glb(data, name)?);
        images::decode(&mut source);
        Ok(Self(source))
    }
}

/// A binary STL says how many triangles it has, and its size is exactly
/// that. Many binary files start with `solid` like an ASCII one, so the
/// size decides.
fn is_binary(data: &[u8]) -> bool {
    count(data).is_some_and(|count| {
        count.checked_mul(TRIANGLE).and_then(|bytes| bytes.checked_add(HEADER + 4)) == Some(data.len())
    })
}

fn count(data: &[u8]) -> Option<usize> {
    let bytes = data.get(HEADER..HEADER + 4)?;
    usize::try_from(u32::from_le_bytes(bytes.try_into().ok()?)).ok()
}

fn binary(data: &[u8]) -> Vec<[Vec3; 3]> {
    data[HEADER + 4..]
        .as_chunks::<TRIANGLE>()
        .0
        .iter()
        .map(|triangle| {
            let float = |at: usize| {
                f32::from_le_bytes([triangle[at], triangle[at + 1], triangle[at + 2], triangle[at + 3]])
            };
            // The normal of the file takes the first 12 bytes.
            let corner = |at: usize| Vec3::new(float(at), float(at + 4), float(at + 8));
            [corner(12), corner(24), corner(36)]
        })
        .collect()
}

fn ascii(data: &[u8]) -> Result<Vec<[Vec3; 3]>> {
    let text = std::str::from_utf8(data).context("the file is no STL, not binary and not text")?;
    ensure!(
        text.trim_start().starts_with("solid"),
        "the file is no STL, it does not start with solid"
    );
    let mut triangles = Vec::new();
    let mut corners = Vec::with_capacity(3);
    for (number, line) in text.lines().enumerate() {
        let mut words = line.split_whitespace();
        match words.next() {
            Some("vertex") => {
                let mut axis = || -> Result<f32> {
                    words
                        .next()
                        .with_context(|| {
                            format!("line {} has a vertex with fewer than 3 numbers", number + 1)
                        })?
                        .parse()
                        .with_context(|| format!("line {} has a vertex that is no number", number + 1))
                };
                corners.push(Vec3::new(axis()?, axis()?, axis()?));
            }
            Some("endloop") => {
                let Ok(corners) = <[Vec3; 3]>::try_from(std::mem::take(&mut corners)) else {
                    bail!("the loop that ends on line {} has no 3 corners", number + 1);
                };
                triangles.push(corners);
            }
            _ => {}
        }
    }
    ensure!(corners.is_empty(), "the STL ends inside a triangle");
    Ok(triangles)
}

/// The normal of a face with its corners counter clockwise, up for a
/// triangle with no area.
fn face_normal([a, b, c]: [Vec3; 3]) -> Vec3 {
    (b - a).cross(c - a).try_normalize().unwrap_or(Vec3::Y)
}

#[cfg(test)]
mod test {
    use super::*;

    fn binary_stl(header: &[u8], triangles: &[[Vec3; 3]]) -> Vec<u8> {
        let mut file = vec![0; HEADER];
        file[..header.len()].copy_from_slice(header);
        file.extend(u32::try_from(triangles.len()).unwrap().to_le_bytes());
        for triangle in triangles {
            file.extend([0u8; 12]);
            for corner in triangle {
                for axis in corner.to_array() {
                    file.extend(axis.to_le_bytes());
                }
            }
            file.extend([0u8; 2]);
        }
        file
    }

    const FLOOR: [Vec3; 3] = [
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(1.0, 0.0, 0.0),
        Vec3::new(0.0, 1.0, 0.0),
    ];

    #[test]
    fn a_binary_stl_gives_its_triangles_with_the_normal_of_each_face() {
        let wall = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 2.0),
            Vec3::new(0.0, 3.0, 0.0),
        ];
        let mesh = MeshData::from_stl(&binary_stl(b"made by a slicer", &[FLOOR, wall])).unwrap();
        assert_eq!(mesh.indices, [0, 1, 2, 3, 4, 5]);
        let positions: Vec<Vec3> = mesh.vertices.iter().map(|vertex| vertex.pos).collect();
        assert_eq!(positions, [FLOOR, wall].concat());
        assert!(mesh.vertices[..3].iter().all(|vertex| vertex.normal == Vec3::Z));
        assert!(mesh.vertices[3..].iter().all(|vertex| vertex.normal == Vec3::NEG_X));
    }

    // Many slicers and CAD tools write `solid` at the start of a binary
    // file too, a reader that looks only at that word reads it as text.
    #[test]
    fn a_binary_stl_whose_header_starts_with_solid_is_read_as_binary() {
        let mesh = MeshData::from_stl(&binary_stl(b"solid part", &[FLOOR])).unwrap();
        assert_eq!(mesh.vertices.len(), 3);
        assert_eq!(mesh.vertices[1].pos, FLOOR[1]);
    }

    #[test]
    fn an_ascii_stl_gives_its_triangles() {
        let text = "solid floor\n  facet normal 0 0 0\n    outer loop\n      vertex 0 0 0\n      vertex 1 0 0\n      vertex 0 1 0\n    endloop\n  endfacet\nendsolid floor\n";
        let mesh = MeshData::from_stl(text.as_bytes()).unwrap();
        let positions: Vec<Vec3> = mesh.vertices.iter().map(|vertex| vertex.pos).collect();
        assert_eq!(positions, FLOOR);
        assert!(mesh.vertices.iter().all(|vertex| vertex.normal == Vec3::Z));
    }

    #[test]
    fn a_file_that_is_no_stl_is_an_error() {
        assert!(MeshData::from_stl(b"").is_err());
        assert!(MeshData::from_stl(b"not a model").is_err());
        assert!(MeshData::from_stl(b"solid empty\nendsolid empty\n").is_err());
        assert!(MeshData::from_stl(b"solid x\nouter loop\nvertex 0 0 0\nvertex 1 0\n").is_err());
        assert!(MeshData::from_stl(b"solid x\nouter loop\nvertex 0 0 0\nendloop\n").is_err());
        // A binary file cut short is no binary STL and no text either.
        let mut cut = binary_stl(b"", &[FLOOR]);
        cut.truncate(cut.len() - 10);
        cut[0] = 0xff;
        assert!(MeshData::from_stl(&cut).is_err());
    }

    #[test]
    fn a_triangle_with_no_area_faces_up() {
        let line = [Vec3::ZERO, Vec3::X, Vec3::X * 2.0];
        let mesh = MeshData::from_stl(&binary_stl(b"", &[line])).unwrap();
        assert_eq!(mesh.vertices[0].normal, Vec3::Y);
    }

    #[test]
    fn a_glb_that_does_not_parse_is_an_error_not_a_panic() {
        assert!(
            PreparedMesh::from_glb(include_bytes!("../../../../assets/models/Box.glb"), "Box.glb").is_ok()
        );
        assert!(PreparedMesh::from_glb(b"glTF but not really", "broken.glb").is_err());
        assert!(PreparedMesh::from_glb(&[], "empty.glb").is_err());
    }
}
