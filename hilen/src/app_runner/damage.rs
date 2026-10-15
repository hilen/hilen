//! What a frame of the app has to draw again: what `ui::damage` finds in
//! the view tree, unless something that is no view draws into the frame.

use std::mem::replace;

use crate::{
    deps::refs::main_lock::MainLock,
    ui::damage::{Damage, find},
};

/// The frame before had a scene or a level in it.
static OUTSIDE_VIEWS: MainLock<bool> = MainLock::new();

pub(super) fn of_frame() -> Damage {
    // Walked also when a scene or a level makes the frame whole, so the
    // frame after it compares against this one.
    let damage = find();

    // A scene or a level draws what no view holds. The frame after its
    // last one still has its pixels, so that frame is whole too.
    let outside = draws_outside_views();
    let was_outside = replace(OUTSIDE_VIEWS.get_mut(), outside);

    if outside {
        Damage::Whole("a scene or a level")
    } else if was_outside {
        Damage::Whole("a scene or a level ended")
    } else {
        damage
    }
}

/// Whether something that is no view draws into the frame.
fn draws_outside_views() -> bool {
    // A scene in a `SceneView` draws into the image of that view.
    #[cfg(feature = "scene")]
    if !crate::scene::SceneManager::no_scene() && crate::scene::SceneManager::host().is_none() {
        return true;
    }
    #[cfg(feature = "level")]
    if !crate::level::LevelManager::no_level() {
        return true;
    }
    false
}
