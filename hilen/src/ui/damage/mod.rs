//! Which pixels of the window changed since the frame before, so a frame
//! draws only those again and keeps the rest. Without this a small
//! animation, a spinner of 20 points, draws every text and every rect of
//! the window again on each refresh. The design and its limits are in
//! `docs/damage.md`.

mod key;
mod regions;
mod walk;

use std::mem::swap;

use parking_lot::Mutex;

pub(crate) use self::key::PaintKey;
use self::walk::{Paints, Walk};
use crate::{
    deps::refs::main_lock::MainLock,
    gm::{
        LossyConvert,
        color::Color,
        flat::{Rect, Size},
    },
    ui::{DrawingView, Label, View, ViewData},
};

/// Pixels a view can paint past its frame, the soft edge of a rect and
/// the rounding of a frame to whole pixels.
const EDGE: f32 = 2.0;

/// What a frame has to draw.
#[derive(Debug, Clone, PartialEq)]
pub enum Damage {
    /// The whole window, with the reason.
    Whole(&'static str),
    /// Only these rects, in pixels. None at all when nothing changed.
    Regions(Vec<Rect<u32>>),
}

/// What the drawer does with the frame it is given.
#[derive(Debug, Default, Clone)]
pub(crate) struct Plan {
    /// `None` for a whole frame.
    pub regions: Option<Vec<Rect<u32>>>,
    /// The clear color of the window, a region starts from it like a
    /// whole frame does.
    pub fill:    Color,
}

/// How the frames so far were drawn, for a test.
#[cfg(feature = "ui-tests")]
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub(crate) struct Frames {
    pub whole:   u64,
    pub partial: u64,
    /// The pixels all frames drew again, the whole window for each whole
    /// frame.
    pub pixels:  u64,
    /// Why the last frame was whole, empty when it was partial.
    pub why:     &'static str,
}

/// The reasons for a whole frame that say the platform or the user rules
/// partial frames out, so a test of them has nothing to check there.
pub(crate) const WHY_OFF: &str = "HILEN_DAMAGE=0";
pub(crate) const WHY_NO_KEPT_FRAME: &str = "a surface with 1 sample keeps no frame";

static LAST: MainLock<Paints> = MainLock::new();
static SEEN: MainLock<Paints> = MainLock::new();
static PLAN: MainLock<Plan> = MainLock::new();
#[cfg(feature = "ui-tests")]
static FRAMES: MainLock<Frames> = MainLock::new();
/// Behind a mutex and not a `MainLock`, a font setting can change on any
/// thread.
static WHOLE_NEXT: Mutex<Option<&'static str>> = Mutex::new(None);

/// `HILEN_DAMAGE=0` draws every frame whole, the A/B lever for a
/// measurement and the way around a view that shows stale pixels.
pub(crate) fn enabled() -> bool {
    #[cfg(not_wasm)]
    {
        use std::sync::OnceLock;

        static ENABLED: OnceLock<bool> = OnceLock::new();
        *ENABLED.get_or_init(|| !std::env::var("HILEN_DAMAGE").is_ok_and(|value| value == "0"))
    }
    #[cfg(wasm)]
    {
        true
    }
}

/// What changed on the screen since the last call. Called once for every
/// frame that is drawn, also for a whole one, so the next call compares
/// against what is on the screen.
pub(crate) fn find() -> Damage {
    let last = LAST.get_mut();
    let seen = SEEN.get_mut();
    seen.clear();

    let (regions, whole) = Walk::run(last, seen);
    swap(last, seen);

    if let Some(why) = WHOLE_NEXT.lock().take() {
        return Damage::Whole(why);
    }

    if let Some(why) = whole {
        return Damage::Whole(why);
    }

    let resolution = crate::ui::UIManager::window_resolution();
    let window = Size::new(pixels(resolution.width), pixels(resolution.height));
    if regions.most_of(window) {
        return Damage::Whole("most of the window changed");
    }

    Damage::Regions(regions.rects().to_vec())
}

/// Hands the drawer what the frame draws, after the window had its say.
pub(crate) fn set_plan(damage: &Damage, fill: Color) {
    let plan = PLAN.get_mut();
    plan.fill = fill;

    plan.regions = match damage {
        Damage::Whole(_) => None,
        Damage::Regions(rects) => Some(rects.clone()),
    };

    #[cfg(feature = "ui-tests")]
    {
        let frames = FRAMES.get_mut();
        frames.why = match damage {
            Damage::Whole(why) => why,
            Damage::Regions(_) => "",
        };
        match &plan.regions {
            None => {
                let window = crate::ui::UIManager::window_resolution();
                frames.whole += 1;
                frames.pixels += u64::from(pixels(window.width)) * u64::from(pixels(window.height));
            }
            Some(rects) => {
                frames.partial += 1;
                frames.pixels += rects
                    .iter()
                    .map(|rect| u64::from(rect.width()) * u64::from(rect.height()))
                    .sum::<u64>();
            }
        }
    }
}

pub(crate) fn plan() -> &'static Plan {
    PLAN.get_mut()
}

#[cfg(feature = "ui-tests")]
pub(crate) fn frames() -> Frames {
    *FRAMES.get_mut()
}

/// The next frame is drawn whole. For a change of something that no
/// view holds and many views draw with, like the fallback fonts.
pub(crate) fn whole_next(why: &'static str) {
    *WHOLE_NEXT.lock() = Some(why);
}

/// The pixels a clipping view at `frame` leaves of `scissor`.
pub(crate) fn clip_rect(frame: &Rect, scale: f32, resolution: Size, scissor: Rect<u32>) -> Rect<u32> {
    let mut frame = *frame * scale;
    frame.origin.clip_positive();

    if frame.max_x() > resolution.width {
        frame.size.width -= frame.max_x() - resolution.width;
    }

    if frame.max_y() > resolution.height {
        frame.size.height -= frame.max_y() - resolution.height;
    }

    // A clip view fully past the right or bottom edge drives these
    // subtractions negative. Converting a negative Rect to Rect<u32>
    // panics, so clamp the clipped size to an empty rect instead.
    // max(0.0) also turns a NaN size into 0.
    frame.size.width = frame.size.width.max(0.0);
    frame.size.height = frame.size.height.max(0.0);

    let clip_rect: Rect<u32> = frame.lossy_convert();
    clip_rect.intersection(&scissor)
}

/// The pixels a view at `frame` can paint, itself and not its subviews,
/// cut at `scissor`. Never smaller than what it paints, a pixel left out
/// would keep its old color.
pub(crate) fn paint_rect(
    view: &dyn View,
    frame: &Rect,
    scale: f32,
    resolution: Size,
    scissor: Rect<u32>,
) -> Rect<u32> {
    // A path has its own points, nothing ties them to the frame.
    if view.as_any().downcast_ref::<DrawingView>().is_some() {
        return scissor;
    }

    let mut grow = 0.0;
    if let Some(shadow) = view.shadow() {
        grow = shadow.radius * 2.0 + shadow.offset.x.abs().max(shadow.offset.y.abs());
    }
    if let Some(label) = view.as_any().downcast_ref::<Label>() {
        if let Some(outline) = label.text_outline() {
            grow = f32::max(grow, outline.width * 2.0);
        }
        if let Some(shadow) = label.text_shadow() {
            grow = f32::max(
                grow,
                shadow.blur * 2.0 + shadow.offset.x.abs().max(shadow.offset.y.abs()),
            );
        }
    }
    let grow = grow * scale + EDGE;

    let frame = *frame * scale;
    let left = (frame.x() - grow).floor().max(0.0).min(resolution.width);
    let top = (frame.y() - grow).floor().max(0.0).min(resolution.height);
    let right = (frame.max_x() + grow).ceil().max(left).min(resolution.width);
    let bottom = (frame.max_y() + grow).ceil().max(top).min(resolution.height);

    Rect::new(
        pixels(left),
        pixels(top),
        pixels(right - left),
        pixels(bottom - top),
    )
    .intersection(&scissor)
}

/// A length of the window in whole pixels.
fn pixels(length: f32) -> u32 {
    length.max(0.0).lossy_convert()
}
