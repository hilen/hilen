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
            return Damage::Whole(damage::why_off());
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
        // Proven on an iPhone 16 Pro Max with iOS 27: a partial frame over
        // the kept multisampled texture shows black or an older screen,
        // the simulator keeps it. See the roadmap. A page in a browser of an
        // iPhone does the same over WebGPU: on 2026-11-05 the icons of a
        // page blinked 20 times a second in Brave on iOS 18, and stopped with
        // partial frames off.
        if (cfg!(target_os = "ios") || on_apple_touch_page()) && msaa_sample_count() > 1 {
            return Damage::Whole(damage::WHY_IPHONE_SAMPLES);
        }
        // A region is painted over with the clear color first. A color
        // that is not solid would mix with the old pixels there.
        if self.clear_color.a < 1.0 {
            return Damage::Whole("the clear color is not solid");
        }

        damage
    }
}

/// Whether this is a page on an iPhone or an iPad, read once from the
/// browser.
#[cfg(wasm)]
fn on_apple_touch_page() -> bool {
    use std::sync::OnceLock;

    static APPLE_TOUCH: OnceLock<bool> = OnceLock::new();
    *APPLE_TOUCH.get_or_init(|| {
        let Some(navigator) = web_sys::window().map(|window| window.navigator()) else {
            return false;
        };
        let agent = navigator.user_agent().unwrap_or_default();
        apple_touch(&agent, navigator.max_touch_points())
    })
}

#[cfg(not_wasm)]
fn on_apple_touch_page() -> bool {
    false
}

/// An iPhone, an iPod, or an iPad, which since iPadOS 13 names itself a
/// Mac and tells it by its touch points. Every browser there runs on `WebKit`.
#[cfg(any(wasm, test))]
fn apple_touch(user_agent: &str, touch_points: i32) -> bool {
    ["iPhone", "iPad", "iPod"].iter().any(|name| user_agent.contains(name))
        || (user_agent.contains("Macintosh") && touch_points > 1)
}

#[cfg(test)]
mod test {
    use super::apple_touch;

    #[test]
    fn a_page_on_an_iphone_or_an_ipad_is_found() {
        let brave = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like \
                     Gecko) Version/27.0.1 Mobile/15E148 Safari/604.1 Brave";
        assert!(apple_touch(brave, 5));
        let ipad = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) \
                    Version/18.0 Safari/605.1.15";
        assert!(apple_touch(ipad, 5));
        assert!(!apple_touch(ipad, 0), "a Mac has no touch points");
        let android = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) \
                       Chrome/130.0 Mobile Safari/537.36";
        assert!(!apple_touch(android, 5));
    }
}
