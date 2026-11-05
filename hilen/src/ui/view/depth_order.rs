//! The depth of a view comes from the tree order. A later sibling and all it
//! holds is in front of an earlier sibling and all it holds, at any depth of
//! the tree. A screen whose last child is an overlay so has it on top with no
//! number to pick. The walk runs in a frame after the tree changed.
//!
//! - A view with `set_z_position` starts a layer at that z, like a modal or a
//!   menu, and its views are ordered inside it.
//! - `ViewBase::raise` orders siblings only, it never lifts a view out of its
//!   superview.
//! - `ViewBase::floats` lifts a view and all it holds in front of everything
//!   else of its layer.
//! - `ViewBase::same_depth` puts a view and all it holds at the depth of its
//!   superview.

use std::{
    collections::VecDeque,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    deps::refs::Own,
    ui::{View, ViewFrame, ViewSubviews, WeakView},
};

/// How much nearer each view is than the one before it in tree order. Text
/// draws 1e-7 in front of its label, see `label_drawer`, so the next view
/// sits clearly in front of that. The depth buffer has 24 bits, a step is
/// about 6 of its units, and the root tree has room for 250000 views before
/// the modal layer.
pub(crate) const DEPTH_STEP: f32 = 0.000_000_4;

static GENERATION: AtomicU64 = AtomicU64::new(1);

/// Something changed the order, the next frame walks the tree again.
pub(crate) fn tree_changed() {
    GENERATION.fetch_add(1, Ordering::Relaxed);
}

pub(crate) fn generation() -> u64 {
    GENERATION.load(Ordering::Relaxed)
}

/// Puts `view` among its siblings by `raise`, a higher one is in front.
/// Siblings with the same raise keep the tree order.
pub(crate) fn set_raise(view: &(impl View + ?Sized), raise: i8) {
    if view.__base_view().raise != raise {
        view.__base_view().raise = raise;
        tree_changed();
    }
}

/// `view` and all it holds draw at the depth of its superview.
pub(crate) fn set_same_depth(view: &(impl View + ?Sized)) {
    if !view.__base_view().same_depth {
        view.__base_view().same_depth = true;
        tree_changed();
    }
}

/// `view` and all it holds come in front of everything else of its layer.
pub(crate) fn set_floats(view: &(impl View + ?Sized), floats: bool) {
    if view.__base_view().floats != floats {
        view.__base_view().floats = floats;
        tree_changed();
    }
}

/// Sets the depth of every view under `root` again.
pub(crate) fn order(root: WeakView) {
    layer(root);
}

fn layer(root: WeakView) {
    let mut walk = Walk {
        cursor: root.z_position() - DEPTH_STEP,
        floats: VecDeque::new(),
    };
    walk.children(root);
    while let Some(view) = walk.floats.pop_front() {
        walk.place(view);
    }
}

fn share(view: WeakView, z: f32) {
    view.__base_view().z_position = z;
    for sub in view.subviews() {
        let sub = sub.weak();
        if sub.__base_view().z_position_custom {
            layer(sub);
        } else {
            share(sub, z);
        }
    }
}

struct Walk {
    cursor: f32,
    floats: VecDeque<WeakView>,
}

impl Walk {
    fn place(&mut self, view: WeakView) {
        view.__base_view().z_position = self.cursor;
        self.cursor -= DEPTH_STEP;
        self.children(view);
    }

    fn children(&mut self, view: WeakView) {
        let mut subviews: Vec<WeakView> = view.subviews().iter().map(Own::weak).collect();
        if subviews.iter().any(|sub| sub.__base_view().raise != 0) {
            subviews.sort_by_key(|sub| sub.__base_view().raise);
        }
        let z = view.z_position();
        for sub in subviews {
            let base = sub.__base_view();
            if base.z_position_custom {
                layer(sub);
            } else if base.floats {
                self.floats.push_back(sub);
            } else if base.same_depth {
                share(sub, z);
            } else {
                self.place(sub);
            }
        }
    }
}
