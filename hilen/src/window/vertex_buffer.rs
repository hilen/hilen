use crate::gm::flat::Point;

#[derive(Debug, Default)]
pub struct VertexBuffer {
    pub vertices: Vec<Point>,
    pub indices:  Option<Vec<u16>>,
}

impl VertexBuffer {
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices = None;
    }
}

impl From<Vec<Point>> for VertexBuffer {
    fn from(vertices: Vec<Point>) -> Self {
        Self {
            vertices,
            indices: None,
        }
    }
}
