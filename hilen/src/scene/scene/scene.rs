use std::ops::{Deref, DerefMut};

use super::SceneInternal;
use crate::{
    deps::refs::{AsAny, Own},
    scene::{Node, SceneBase},
};

pub trait Scene: AsAny + Deref<Target = SceneBase> + DerefMut + SceneInternal {
    fn nodes(&self) -> &[Own<dyn Node>] {
        &self.nodes
    }

    fn nodes_mut(&mut self) -> &mut [Own<dyn Node>] {
        &mut self.nodes
    }
}
