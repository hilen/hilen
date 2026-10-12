//! Whether a frame can draw over the frame before, see `ui::damage`.

use super::State;
use crate::{
    gm::{color::Color, flat::Size},
    ui::damage::{self, Damage},
    window::{app_handler::AppHandler, msaa::msaa_sample_count},
};

/// The frame before, as far as a later frame can draw over it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Kept {
    pub size:        Size<u32>,
    pub clear_color: Color,
    pub target:      Target,
}

/// What the passes of a frame draw into, or resolve into with several
/// samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Target {
    /// The offscreen texture of a headless run, the same in every frame.
    Offscreen,
    /// The scene texture, the same in every frame. It is copied to the
    /// surface at the end of the frame.
    Scene,
    /// An image of the surface, another one in every frame.
    Surface,
}

impl Target {
    /// Whether a frame drawn into it is still there for the next frame.
    fn keeps(self) -> bool {
        self != Self::Surface
    }
}

/// Whether a frame for a surface goes through the scene texture only so
/// that the next frame can draw over it. With 1 sample nothing else keeps
/// a frame, and the copy to the surface costs less than a whole frame of
/// views. Not in a browser, where 1 sample is the path of weak devices
/// and the copy is paid by every frame.
pub(super) fn keeps_through_scene() -> bool {
    cfg!(not_wasm) && damage::enabled() && msaa_sample_count() == 1
}

impl State {
    /// Plans the frame and hands the plan to the drawer. Gives what the
    /// frame is kept in, and the color its pass clears with, none for a
    /// partial frame, which draws over the frame before.
    pub(super) fn plan_frame(
        &self,
        size: Size<u32>,
        offscreen: bool,
        scene: bool,
    ) -> (Target, Option<Color>) {
        let target = if offscreen {
            Target::Offscreen
        } else if scene {
            Target::Scene
        } else {
            Target::Surface
        };

        let damage = self.frame_damage(size, target);
        damage::set_plan(&damage, self.clear_color);

        let clear = matches!(damage, Damage::Whole(_)).then_some(self.clear_color);
        (target, clear)
    }

    /// The frame that was just drawn is what the next one can draw over.
    pub(super) fn keep_frame(&mut self, size: Size<u32>, target: Target) {
        self.kept = Some(Kept {
            size,
            clear_color: self.clear_color,
            target,
        });
    }

    /// What this frame draws again. A partial frame needs the pixels of
    /// the frame before:
    ///
    /// - With several samples they are in the multisampled texture, which is
    ///   the same one in every frame. It resolves whole into the target, so the
    ///   target needs to keep nothing.
    /// - With 1 sample the frame is drawn straight into the target, so the
    ///   target has to be the same texture as in the frame before. A surface
    ///   hands out another image each time, see `keeps_through_scene`.
    fn frame_damage(&self, size: Size<u32>, target: Target) -> Damage {
        // Asked for every frame, so the next one compares against this one.
        let damage = AppHandler::current().te_window_events.damage();

        if matches!(damage, Damage::Whole(_)) {
            return damage;
        }
        if !damage::enabled() {
            return Damage::Whole(damage::WHY_OFF);
        }

        let Some(kept) = self.kept else {
            return Damage::Whole("no frame is kept");
        };
        if kept.size != size || kept.clear_color != self.clear_color {
            return Damage::Whole("the size or the clear color changed");
        }
        if msaa_sample_count() == 1 && !(target.keeps() && kept.target == target) {
            return Damage::Whole(damage::WHY_NO_KEPT_FRAME);
        }
        // A region is painted over with the clear color first. A color
        // that is not solid would mix with the old pixels there.
        if self.clear_color.a < 1.0 {
            return Damage::Whole("the clear color is not solid");
        }

        damage
    }
}
