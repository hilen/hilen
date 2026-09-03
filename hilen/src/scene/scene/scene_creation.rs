use crate::{
    deps::refs::{Own, Weak},
    gm::volume::{Shape3, Vec3},
    scene::{Node, Scene},
};

pub trait SceneCreation {
    fn add_node<N: 'static + Node>(&mut self, node: Own<N>) -> Weak<N>;
    fn make_node<N: 'static + Node>(&mut self, _: Shape3, _: impl Into<Vec3>) -> Weak<N>;
}

impl<T: ?Sized + Scene> SceneCreation for T {
    fn add_node<N: 'static + Node>(&mut self, node: Own<N>) -> Weak<N> {
        let weak = node.weak();
        self.nodes.push(node);
        weak
    }

    fn make_node<N: 'static + Node>(&mut self, shape: Shape3, position: impl Into<Vec3>) -> Weak<N> {
        self.add_node(N::make(shape, position.into()))
    }
}
