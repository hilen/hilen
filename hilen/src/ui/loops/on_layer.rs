//! The layer path of a loop: where a layer can show it, its pictures, and
//! the switch between the layer and the frame path.

use std::{
    ops::Deref,
    sync::{
        OnceLock,
        atomic::{AtomicU8, Ordering},
    },
};

use anyhow::{Result, ensure};
use log::{debug, warn};

#[cfg(feature = "ui-tests")]
use super::LOOPS;
use super::{Entry, fit, flipbook, key, layer::Layer};
#[cfg(feature = "ui-tests")]
use crate::ui::WeakView;
use crate::{
    frame_time::refresh_period,
    gm::{
        LossyConvert,
        flat::{Rect, Size},
    },
    ui::{UIDrawer, UIManager, ViewData, ViewFrame},
    window::{Window, frame_control},
};

/// Frames in a row with the same content before the pictures are drawn.
/// A content that changes in every frame, like a fade of the view, stays
/// on the frame path until it settles, and is drawn once after that.
const STABLE_FRAMES: u32 = 3;

/// Frames in a row with a new content at the same phase before the
/// warning that the pose is not one of the phase alone.
const RESTLESS_FRAMES: u32 = 600;

/// Most pictures of 1 loop with no change times. A long loop gets fewer
/// pictures than the screen has refreshes in it.
const MAX_PICTURES: f64 = 240.0;

/// The pixels of a picture, rows from the top, no gap between them.
pub(crate) struct RawFrames {
    pub(crate) pixels: Vec<Vec<u8>>,
    pub(crate) size:   Size<u32>,
    pub(crate) order:  PixelOrder,
}

/// How the bytes of a pixel are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PixelOrder {
    /// Blue, green, red, alpha, the colors multiplied by the alpha. What a
    /// frame of the engine is on Apple systems.
    BgraPremultiplied,
    /// Red, green, blue, alpha as a decoder gives it.
    Rgba,
}

/// Pictures a view already has, in place of pictures the engine draws.
pub(crate) struct OwnPictures {
    /// Where the pictures go, in the points of the window. None when they
    /// cannot show the view as it is now, the engine then draws the
    /// pictures itself.
    pub(crate) area:   Box<dyn Fn() -> Option<Rect> + Send>,
    /// The pictures, 1 for each change time of the loop.
    pub(crate) frames: Box<dyn Fn() -> Result<RawFrames> + Send>,
}

/// Where a layer shows a loop.
#[derive(Debug, Clone, Copy)]
pub(super) struct Place {
    /// The pixels of the window the pictures cover, whole ones.
    pixels: Rect<u32>,
    /// The rect of the layer in the pixels of the window.
    layer:  Rect,
    /// The pictures are the own ones of the view.
    own:    bool,
}

pub(super) struct LayerPath {
    own:      Option<OwnPictures>,
    layer:    Option<Layer>,
    /// The content the pictures in the layer were made of.
    built:    Option<u64>,
    /// The content in the frame before, and how many frames in a row had it.
    seen:     Option<u64>,
    stable:   u32,
    /// Pictures of this content could not be made.
    failed:   Option<u64>,
    restless: u32,
    warned:   bool,
    /// The layer shows the loop now.
    shown:    bool,
    /// How many times the pictures were made.
    #[cfg(feature = "ui-tests")]
    builds:   u32,
}

impl LayerPath {
    pub(super) fn new(own: Option<OwnPictures>) -> Self {
        Self {
            own,
            layer: None,
            built: None,
            seen: None,
            stable: 0,
            failed: None,
            restless: 0,
            warned: false,
            shown: false,
            #[cfg(feature = "ui-tests")]
            builds: 0,
        }
    }
}

impl LayerPath {
    #[cfg(hot)]
    pub(super) fn into_layer(self) -> Option<Layer> {
        self.layer
    }
}

static TEST_LAYERS: AtomicU8 = AtomicU8::new(TEST_LAYERS_APP);
const TEST_LAYERS_APP: u8 = 0;
const TEST_LAYERS_OFF: u8 = 1;
const TEST_LAYERS_ON: u8 = 2;

/// A test reads the pixels of the frame, which hold nothing a layer of the
/// system shows. So a test run keeps every loop on the frame path,
/// `Some(false)`, the runner sets it before each test. A test of the
/// layers turns them on with `Some(true)`. `None` hands the choice back to
/// the app after the run.
pub(crate) fn set_test_layers(layers: Option<bool>) {
    let value = match layers {
        None => TEST_LAYERS_APP,
        Some(false) => TEST_LAYERS_OFF,
        Some(true) => TEST_LAYERS_ON,
    };
    TEST_LAYERS.store(value, Ordering::Relaxed);
}

/// `HILEN_LOOP_LAYERS=0` keeps every loop on the frame path, the lever of
/// an A/B and the way around a layer that shows a view wrong.
fn layers_off_by_env() -> bool {
    static OFF: OnceLock<bool> = OnceLock::new();
    *OFF.get_or_init(|| std::env::var("HILEN_LOOP_LAYERS").is_ok_and(|value| value == "0"))
}

/// Whether a layer may show a loop in this frame at all. A frame that is
/// read back, a screenshot, a step or a record of the inspector, has the
/// loops in the frame.
fn layers_allowed() -> bool {
    if TEST_LAYERS.load(Ordering::Relaxed) == TEST_LAYERS_OFF {
        return false;
    }
    !layers_off_by_env()
        && !Window::headless()
        && Window::winit_window().is_some()
        && !Window::current().state.screenshot_pending()
        && !frame_control::offscreen_wanted()
        && !UIManager::should_draw_debug_frames()
}

/// Where a layer can show each loop in this frame, none for a loop that
/// stays on the frame path.
pub(super) fn places(entries: &[Entry]) -> Vec<Option<Place>> {
    if !layers_allowed() {
        return vec![None; entries.len()];
    }

    let mut places: Vec<Option<Place>> = entries.iter().map(Entry::place).collect();

    let candidates: Vec<fit::Candidate> = entries
        .iter()
        .zip(&places)
        .filter_map(|(entry, place)| {
            place.map(|place| fit::Candidate {
                view:   entry.view,
                pixels: place.pixels,
            })
        })
        .collect();
    let mut covered = fit::covered(&candidates).into_iter();

    for place in &mut places {
        if place.is_some() && covered.next().unwrap_or(true) {
            *place = None;
        }
    }
    places
}

/// The screen refreshes a second, the number of pictures a second of a
/// loop with no change times gets.
fn refresh_rate() -> f64 {
    let monitor = Window::winit_window()
        .and_then(winit::window::Window::current_monitor)
        .and_then(|monitor| monitor.refresh_rate_millihertz());
    if let Some(millihertz) = monitor
        && millihertz > 0
    {
        return f64::from(millihertz) / 1000.0;
    }
    match refresh_period() {
        Some(period) if period > 0.0 => 1.0 / period,
        _ => 60.0,
    }
}

impl Entry {
    fn place(&self) -> Option<Place> {
        let view = self.view.deref();
        if !view.is_visible_on_screen() {
            return None;
        }
        let scale = UIManager::scale();

        let own_area = self.layer.own.as_ref().and_then(|own| (own.area)());
        if let Some(area) = own_area {
            let layer = area * scale;
            let pixels = fit::pixels(view, Some(layer))?;
            return Some(Place {
                pixels,
                layer,
                own: true,
            });
        }

        let pixels = fit::pixels(view, None)?;
        Some(Place {
            pixels,
            layer: pixels.lossy_convert(),
            own: false,
        })
    }

    /// The phases the pictures are made at.
    fn samples(&self) -> Vec<f32> {
        if let Some(changes) = &self.changes {
            return changes.clone();
        }
        let count: f64 = (self.period * refresh_rate()).ceil().clamp(1.0, MAX_PICTURES);
        let count: u32 = count.lossy_convert();
        let whole: f32 = count.lossy_convert();
        (0..count)
            .map(|index| {
                let index: f32 = index.lossy_convert();
                index / whole
            })
            .collect()
    }

    /// Shows the loop on a layer at `place` when it can. False leaves it
    /// to the frame path for this frame.
    pub(super) fn try_layer(&mut self, place: Option<Place>, phase: f32) -> bool {
        let Some(place) = place else {
            self.leave_layer();
            return false;
        };

        // The content at phase 0. A pose that keeps its paths when nothing
        // changed makes this cheap.
        let zero = self.sample(0.0);
        (self.pose)(zero);
        UIDrawer::update_subtree(self.view);
        let content = key::content(self.view.deref(), place.pixels, place.own);

        if self.layer.built == Some(content) && self.layer.layer.is_some() {
            return self.show_layer(place, phase);
        }

        let path = &mut self.layer;
        if path.seen == Some(content) {
            path.stable += 1;
        } else {
            path.seen = Some(content);
            path.stable = 0;
            path.restless += 1;
        }

        if path.restless >= RESTLESS_FRAMES && !path.warned {
            path.warned = true;
            warn!(
                "{}: the pose of its loop gives a new picture at the same phase in every frame, so the loop never goes to a layer. A pose only sets the state for the phase, see docs/loops.md",
                self.view.view_label()
            );
        }

        if path.stable < STABLE_FRAMES || path.failed == Some(content) {
            self.leave_layer();
            return false;
        }
        path.restless = 0;

        match self.build(place, content) {
            Ok(()) => self.show_layer(place, phase),
            Err(error) => {
                debug!(
                    "{}: its loop stays on the frame path: {error:#}",
                    self.view.view_label()
                );
                self.layer.failed = Some(content);
                self.leave_layer();
                false
            }
        }
    }

    /// Makes the pictures of the loop and puts them into its layer.
    fn build(&mut self, place: Place, content: u64) -> Result<()> {
        let samples = self.samples();

        let frames = match &self.layer.own {
            Some(own) if place.own => {
                let frames = (own.frames)()?;
                ensure!(
                    frames.pixels.len() == samples.len(),
                    "{} own pictures for {} change times",
                    frames.pixels.len(),
                    samples.len()
                );
                frames
            }
            _ => flipbook::draw(self.view, place.pixels, &samples, &mut self.pose)?,
        };

        let mut key_times: Vec<f64> = samples.iter().map(|phase| f64::from(*phase)).collect();
        key_times.push(1.0);

        let layer = match self.layer.layer.take() {
            Some(layer) => layer,
            None => Layer::new()?,
        };
        let layer = self.layer.layer.insert(layer);
        layer.set_pictures(&frames, key_times, self.period)?;

        debug!(
            "{}: its loop shows {} pictures of {:?} on a layer",
            self.view.view_label(),
            frames.pixels.len(),
            frames.size
        );

        self.layer.built = Some(content);
        self.layer.shown = false;
        #[cfg(feature = "ui-tests")]
        {
            self.layer.builds += 1;
        }
        Ok(())
    }

    fn show_layer(&mut self, place: Place, phase: f32) -> bool {
        let Some(layer) = &mut self.layer.layer else {
            return false;
        };
        layer.place(place.layer);
        if !self.layer.shown {
            layer.start(f64::from(phase) * self.period);
            self.layer.shown = true;
        }
        self.view.__base_view().loop_layer = true;
        true
    }

    /// The frame path shows the loop from this frame on.
    pub(super) fn leave_layer(&mut self) {
        if self.layer.shown
            && let Some(layer) = &self.layer.layer
        {
            layer.hide();
        }
        self.layer.shown = false;
        self.view.__base_view().loop_layer = false;
    }
}

/// What a test reads of the layer of a loop.
#[cfg(feature = "ui-tests")]
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayerState {
    /// The rect of the layer in the points of the window view.
    pub(crate) frame:    Rect,
    pub(crate) pictures: usize,
    /// The keyframe animation is on the layer.
    pub(crate) animated: bool,
    /// The layer would show its pictures upside down.
    pub(crate) flipped:  bool,
    /// The y axis of the layer under it goes down.
    pub(crate) y_down:   bool,
    /// How many times the pictures of the loop were made.
    pub(crate) builds:   u32,
}

/// The layer of the loop of `view` while it shows the loop.
#[cfg(feature = "ui-tests")]
pub(crate) fn layer_state(view: WeakView) -> Option<LayerState> {
    let entry = LOOPS.get_mut().iter().find(|entry| entry.view.addr() == view.addr())?;
    if !entry.layer.shown {
        return None;
    }
    let mut state = entry.layer.layer.as_ref().map(Layer::state)?;
    state.builds = entry.layer.builds;
    Some(state)
}
