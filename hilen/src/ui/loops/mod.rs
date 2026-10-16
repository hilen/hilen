//! Declared looping animations. A loop is a period and a pose of the
//! phase, 0 to 1, that only sets the state of the view and its subviews.
//!
//! Where a layer of the system can show the view, on macOS, iOS and tvOS
//! through Core Animation, the engine calls the pose at fixed phases, draws
//! each result once into a picture and hands the pictures to a layer over
//! the view. The layer shows them in a loop and the app draws no frame for
//! it. Everywhere else, and wherever a layer cannot show the view right,
//! the pose runs once per frame with the phase of `time::frame_seconds`.
//! The inside is in `docs/loops.md`.

#[cfg(apple)]
mod fit;
#[cfg(apple)]
mod flipbook;
#[cfg(apple)]
mod key;
#[cfg(apple)]
mod layer;
#[cfg(apple)]
mod on_layer;
#[cfg(feature = "ui-tests")]
mod test;

use std::{
    mem::take,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

use log::warn;

#[cfg(apple)]
pub(crate) use self::on_layer::{OwnPictures, PixelOrder, RawFrames, set_test_layers};
use crate::{
    deps::refs::main_lock::MainLock,
    frame_time::frame_seconds,
    gm::{LossyConvert, ToF32},
    ui::{UIDrawer, ViewData, ViewFrame, WeakView},
    window::request_frame,
};

/// What a pose gets, the phase of the loop from 0 to 1.
pub(crate) type Pose = Box<dyn FnMut(f32) + Send>;

/// A phase this close below a change time already counts as the change.
/// Frame stepped time adds 1/60 s in steps, and 6 of them land a hair
/// under 0.1 s.
const CHANGE_SLACK: f32 = 1e-5;

/// A declared looping animation, for anything that repeats or runs
/// without end, like a spinner. Run it with `ViewData::run_loop`.
///
/// The pose gets the phase, 0 to 1, and only sets the state of the view
/// and its subviews: frames, colors, paths, images. It touches nothing
/// else and keeps no state of its own between calls, the engine calls it
/// at any phase in any order, also many times in 1 frame. A pose at one
/// phase must give the same picture every time.
///
/// ```ignore
/// self.dot.run_loop(LoopAnimation::new(2.0, move |phase| {
///     self.dot.set_x(phase * 100.0);
/// }));
/// ```
pub struct LoopAnimation {
    period:  f64,
    start:   f32,
    changes: Option<Vec<f32>>,
    pose:    Pose,
    #[cfg(apple)]
    own:     Option<OwnPictures>,
}

impl LoopAnimation {
    /// A loop of `period` seconds. Without `changes_at` the picture can
    /// change on every refresh of the screen.
    pub fn new(period: impl ToF32, pose: impl FnMut(f32) + Send + 'static) -> Self {
        Self {
            period:            f64::from(period.to_f32()),
            start:             0.0,
            changes:           None,
            pose:              Box::new(pose),
            #[cfg(apple)]
            own:               None,
        }
    }

    /// The phases where the picture changes, each from 0 to 1. The pose
    /// is then called only at them, and each picture shows until the next
    /// one, like the frames of a gif with their own times. 0 is always one
    /// of them.
    pub fn changes_at(mut self, phases: impl IntoIterator<Item = f32>) -> Self {
        let mut phases: Vec<f32> = phases.into_iter().filter(|phase| (0.0..1.0).contains(phase)).collect();
        phases.push(0.0);
        phases.sort_by(f32::total_cmp);
        phases.dedup();
        self.changes = Some(phases);
        self
    }

    /// The phase the loop is at when it starts, 0 by default.
    pub fn starting_at(mut self, phase: impl ToF32) -> Self {
        self.start = phase.to_f32().rem_euclid(1.0);
        self
    }

    /// Pictures the view already has, 1 for each change time, in place of
    /// pictures the engine draws. An `AnimatedImage` hands its frames so.
    #[cfg(apple)]
    pub(crate) fn own_pictures(mut self, own: OwnPictures) -> Self {
        self.own = Some(own);
        self
    }
}

/// A loop on its view.
struct Entry {
    id:      u64,
    view:    WeakView,
    #[cfg(apple)]
    period:  f64,
    changes: Option<Vec<f32>>,
    pose:    Pose,
    #[cfg(apple)]
    layer:   on_layer::LayerPath,
}

impl Entry {
    fn alive(&self) -> bool {
        self.view.is_ok() && self.view.__base_view().loop_id == self.id
    }

    /// The phase the pose gets for `phase`, the change time it is in when
    /// the loop has them.
    fn sample(&self, phase: f32) -> f32 {
        let Some(changes) = &self.changes else {
            return phase;
        };
        changes
            .iter()
            .rev()
            .find(|change| **change <= phase + CHANGE_SLACK)
            .copied()
            .unwrap_or(0.0)
    }

    /// Poses the view at `phase` and lays it out again.
    fn pose_at(&mut self, phase: f32) {
        let sample = self.sample(phase);
        (self.pose)(sample);
        UIDrawer::update_subtree(self.view);
    }
}

static LOOPS: MainLock<Vec<Entry>> = MainLock::new();
static NEXT_ID: AtomicU64 = AtomicU64::new(1);
/// A loop on the frame path is on the screen, so the loop of the app
/// draws on every refresh.
static WANTS_FRAMES: AtomicBool = AtomicBool::new(false);

pub(crate) fn start(view: WeakView, animation: LoopAnimation) {
    let LoopAnimation {
        period,
        start,
        changes,
        pose,
        #[cfg(apple)]
        own,
    } = animation;

    if !(period.is_finite() && period > 0.0) {
        warn!(
            "{} asks for a loop of {period} s, a loop needs a period above 0",
            view.view_label()
        );
        stop(view);
        return;
    }

    let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let base = view.__base_view();
    base.loop_id = id;
    base.loop_clock = (frame_seconds() - f64::from(start) * period, period);

    LOOPS.get_mut().push(Entry {
        id,
        view,
        #[cfg(apple)]
        period,
        changes,
        pose,
        #[cfg(apple)]
        layer: on_layer::LayerPath::new(own),
    });
    request_frame();
}

/// The entry goes at the next frame, with its layer.
pub(crate) fn stop(view: WeakView) {
    view.__base_view().loop_id = 0;
    request_frame();
}

pub(crate) fn phase(view: WeakView) -> Option<f32> {
    let base = view.__base_view();
    (base.loop_id != 0).then(|| phase_at(base.loop_clock, frame_seconds()))
}

fn phase_at((zero, period): (f64, f64), time: f64) -> f32 {
    let phase: f32 = ((time - zero) / period).rem_euclid(1.0).lossy_convert();
    // An f64 just under 1 rounds to 1 as an f32.
    if phase >= 1.0 { 0.0 } else { phase }
}

/// Read by `continuous_render_active`.
pub(crate) fn wants_frames() -> bool {
    WANTS_FRAMES.load(Ordering::Relaxed)
}

/// Once a frame, after the layout. Puts every loop on a layer or on the
/// frame path and poses the ones on the frame path for this frame.
pub(crate) fn step() {
    let mut entries = take(LOOPS.get_mut());
    if entries.is_empty() {
        WANTS_FRAMES.store(false, Ordering::Relaxed);
        return;
    }

    entries.retain(|entry| {
        let alive = entry.alive();
        // A stopped or replaced loop leaves the view to the frame.
        if !alive && entry.view.is_ok() && entry.view.__base_view().loop_id == 0 {
            entry.view.__base_view().loop_layer = false;
        }
        alive
    });

    let now = frame_seconds();
    #[cfg(apple)]
    let mut places = on_layer::places(&entries).into_iter();
    let mut wants = false;

    for entry in &mut entries {
        #[cfg(apple)]
        let place = places.next().flatten();
        let phase = phase_at(entry.view.__base_view().loop_clock, now);

        if !entry.view.is_visible_on_screen() {
            #[cfg(apple)]
            entry.leave_layer();
            continue;
        }

        #[cfg(apple)]
        if entry.try_layer(place, phase) {
            continue;
        }

        entry.pose_at(phase);
        wants = true;
    }

    // A pose or a setup may have started loops while this ran.
    entries.append(LOOPS.get_mut());
    *LOOPS.get_mut() = entries;

    WANTS_FRAMES.store(wants, Ordering::Relaxed);
    if wants {
        request_frame();
    }
}

/// Every loop goes, at the stop of a hot build. Its layer stays on the
/// window view and goes with it, see `Layer::leave_with_view`.
#[cfg(hot)]
pub(crate) fn stop_all() {
    for entry in take(LOOPS.get_mut()) {
        if let Some(layer) = entry.layer.into_layer() {
            layer.leave_with_view();
        }
    }
    WANTS_FRAMES.store(false, Ordering::Relaxed);
}
