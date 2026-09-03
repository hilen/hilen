use std::ops::{Deref, DerefMut};

use crate::{
    deps::refs::{Own, Weak, weak_from_ref},
    gm::volume::{Shape3, Vec3},
    scene::{Node, NodeData},
};

/// A node that is only drawn. No body, no collider, it sits where it is
/// put, the way a `Banner` does in a level.
pub struct Prop {
    node: NodeData,
}

impl Deref for Prop {
    type Target = NodeData;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl DerefMut for Prop {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl Node for Prop {
    fn make(shape: Shape3, position: Vec3) -> Own<Self>
    where Self: Sized {
        Own::new(Self {
            node: NodeData::make(shape, position),
        })
    }

    fn weak_node(&self) -> Weak<dyn Node> {
        weak_from_ref(self)
    }
}
