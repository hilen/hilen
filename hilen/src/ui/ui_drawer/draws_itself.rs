//! Views that draw into the render pass by themselves, what the engine
//! cannot draw for them. The view trait names no GPU type, such a view
//! registers here, and the drawer asks only the views that did.

use std::collections::HashMap;

use wgpu::RenderPass;

use super::UIDrawer;
use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    ui::View,
};

/// A view that draws on the GPU itself. It calls `UIDrawer::draws_itself`
/// in its setup. A frame with such a view on screen is drawn whole, see
/// `ui::damage`, since the engine cannot see what it draws.
pub trait DrawsItself {
    fn draw(&self, pass: &mut RenderPass);
}

/// By the address of the view.
static VIEWS: MainLock<HashMap<usize, Weak<dyn DrawsItself>>> = MainLock::new();

impl UIDrawer {
    /// Marks `view` as one that draws into the pass by itself.
    pub fn draws_itself<V: DrawsItself + View + 'static>(view: Weak<V>) {
        VIEWS.get_mut().insert(view.addr(), view);
    }

    /// Draws `view` when it registered, and tells whether it did. A view
    /// that is gone leaves the list, its address can come back as another
    /// view.
    pub(super) fn draw_itself(view: &dyn View, pass: &mut RenderPass) -> bool {
        let addr = view.weak_view().addr();
        let views = VIEWS.get_mut();
        let Some(custom) = views.get(&addr) else {
            return false;
        };
        if !custom.is_ok() {
            views.remove(&addr);
            return false;
        }
        custom.draw(pass);
        true
    }
}
