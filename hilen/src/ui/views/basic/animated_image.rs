#[cfg(apple)]
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(apple)]
use anyhow::Context;
use anyhow::Result;
use ui_proc::view;

use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{Clock, LossyConvert},
    ui::{
        ImageMode, ImageView, LoopAnimation, Setup, View, ViewCallbacks, ViewData, ViewFrame,
        keep_frames_exempt,
    },
    window::image::{Image, decode_gif},
};
#[cfg(apple)]
use crate::{
    gm::flat::{CornerRadii, Rect},
    ui::loops::{OwnPictures, PixelOrder, RawFrames},
};

/// Unique per loaded gif, so one gif's frame textures never collide with
/// another's in the image store.
static NEXT_GIF_ID: AtomicU64 = AtomicU64::new(0);

/// A phase this close below the start of a frame already shows that frame,
/// the slack of the change times of a loop.
const START_SLACK: f32 = 1e-5;

/// Plays a gif as a frame sequence. Decoded once into one texture per frame.
/// The inner `ImageView` does the drawing, so aspect mode, corner radii and
/// the rest work like any image.
///
/// A gif that loops without end runs as a declared loop with the times of
/// its frames, see `LoopAnimation`. On Apple systems a layer of the system
/// then shows its frames and the app draws no frame for it. A gif with a
/// loop count ends, it swaps its frames on a timer that reads the engine
/// clock, real time in a normal run and stepped time in a test.
#[view]
pub struct AnimatedImage {
    frames: Vec<Weak<Image>>,
    /// Seconds each frame shows, same length as `frames`.
    delays: Vec<f32>,
    /// Where each frame starts as a phase of the whole gif, 0 to 1.
    starts: Vec<f32>,
    /// The seconds of all frames.
    period: f32,
    /// The file, decoded again for a layer of the system. The frames of
    /// the decoder are not kept, so a gif costs no memory twice.
    #[cfg(apple)]
    gif:    Option<Arc<[u8]>>,

    current: usize,
    /// Seconds accumulated toward the current frame's delay.
    accum:   f32,
    /// Engine clock milliseconds at the last update, `None` when paused so a
    /// resume does not count the paused time as one huge step.
    last_ms: Option<f64>,

    playing:    bool,
    /// Zero means loop forever.
    loop_count: u32,
    loops_done: u32,

    /// Fires when a finite loop count is reached and playback stops on the last
    /// frame. An infinite gif never fires it.
    pub on_finish: Event<()>,

    #[init]
    image_view: ImageView,
}

impl AnimatedImage {
    /// Decode a gif and start playing it from the first frame. Replaces any gif
    /// already loaded.
    pub fn set_gif(&self, data: &[u8]) -> Result<&Self> {
        let id = NEXT_GIF_ID.fetch_add(1, Ordering::Relaxed);
        self.set_gif_keyed(data, &id.to_string())
    }

    /// `set_gif` with caller chosen texture names. Frame textures are managed
    /// and never freed, so a gif loaded repeatedly, like a dialog opened many
    /// times, reuses one texture set under a stable key instead of keeping a
    /// new set per load.
    pub fn set_gif_keyed(&self, data: &[u8], key: &str) -> Result<&Self> {
        let decoded = decode_gif(data)?;
        let id = key;

        let mut this = weak_from_ref(self);
        this.frames = decoded
            .frames
            .iter()
            .enumerate()
            .map(|(i, frame)| {
                Image::from_raw_data(frame.pixels.clone(), format!("gif-{id}-{i}"), decoded.size, 4)
            })
            .collect();
        this.delays = decoded.frames.iter().map(|frame| frame.delay).collect();
        this.period = this.delays.iter().sum();
        let period = this.period;
        let mut start = 0.0;
        this.starts = this
            .delays
            .iter()
            .map(|delay| {
                let phase = if period > 0.0 { start / period } else { 0.0 };
                start += delay;
                phase
            })
            .collect();
        #[cfg(apple)]
        {
            this.gif = Some(Arc::from(data));
        }

        this.current = 0;
        this.accum = 0.0;
        this.last_ms = None;
        this.loops_done = 0;
        this.playing = true;

        if let Some(first) = this.frames.first().copied() {
            this.image_view.set_image(first);
        }

        this.start_playing();
        Ok(self)
    }

    pub fn set_mode(&self, mode: ImageMode) -> &Self {
        weak_from_ref(self).image_view.mode = mode;
        self
    }

    /// Zero loops forever, the default. A positive count stops on the last
    /// frame after that many loops and fires `on_finish`.
    pub fn set_loop(&self, count: u32) -> &Self {
        let mut this = weak_from_ref(self);
        this.hold_frame();
        this.loop_count = count;
        if this.playing {
            this.start_playing();
        }
        self
    }

    pub fn play(&self) -> &Self {
        let mut this = weak_from_ref(self);
        if !this.playing && !this.frames.is_empty() {
            this.playing = true;
            this.start_playing();
        }
        self
    }

    pub fn pause(&self) -> &Self {
        let mut this = weak_from_ref(self);
        this.hold_frame();
        this.stop_loop();
        this.playing = false;
        self
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Show a specific frame. Also usable while paused to drive the gif by
    /// hand. Not `set_frame`, which is the `ViewFrame` rect setter. A gif
    /// that plays goes on from there.
    pub fn show_frame(&self, index: usize) -> &Self {
        if let Some(image) = self.frames.get(index).copied() {
            let mut this = weak_from_ref(self);
            this.current = index;
            this.accum = 0.0;
            this.image_view.set_image(image);
            if this.loop_phase().is_some() {
                this.start_playing();
            }
        }
        self
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn current_frame(&self) -> usize {
        match self.loop_phase() {
            Some(phase) => self.frame_at(phase),
            None => self.current,
        }
    }

    /// The frame a phase of the whole gif shows.
    fn frame_at(&self, phase: f32) -> usize {
        self.starts.iter().rposition(|start| *start <= phase + START_SLACK).unwrap_or(0)
    }

    /// Keeps the frame a running loop shows now as the current one, also
    /// in the image view, which a layer of the system left at frame 0.
    fn hold_frame(&mut self) {
        if let Some(phase) = self.loop_phase() {
            self.current = self.frame_at(phase);
            self.accum = 0.0;
            if let Some(image) = self.frames.get(self.current).copied() {
                self.image_view.set_image(image);
            }
        }
    }

    /// A gif that loops without end runs a declared loop from its current
    /// frame. One with a loop count ends, it keeps frames until it does.
    fn start_playing(mut self: Weak<Self>) {
        if self.loop_count == 0 && self.period > 0.0 && self.frames.len() > 1 {
            let start = self.starts[self.current];
            let animation = LoopAnimation::new(self.period, move |phase| self.pose(phase))
                .changes_at(self.starts.clone())
                .starting_at(start);
            #[cfg(apple)]
            let animation = animation.own_pictures(self.own_pictures());
            self.run_loop(animation);
            return;
        }
        self.stop_loop();
        // The time the gif stood must not count as one huge step.
        self.last_ms = None;
        self.keep_frames_coming();
    }

    /// The gif at a phase of all its frames.
    fn pose(mut self: Weak<Self>, phase: f32) {
        let index = self.frame_at(phase);
        if index != self.current || !self.image_view.image().is_ok() {
            self.current = index;
            if let Some(image) = self.frames.get(index).copied() {
                self.image_view.set_image(image);
            }
        }
    }

    /// The decoded frames of the gif for a layer of the system, where the
    /// layer can show them as the image view would.
    #[cfg(apple)]
    fn own_pictures(self: Weak<Self>) -> OwnPictures {
        let gif = self.gif.clone();
        OwnPictures {
            area:   Box::new(move || self.picture_area()),
            frames: Box::new(move || {
                let gif = gif.as_ref().context("no gif")?;
                let decoded = decode_gif(gif)?;
                Ok(RawFrames {
                    pixels: decoded.frames.into_iter().map(|frame| frame.pixels).collect(),
                    size:   decoded.size,
                    order:  PixelOrder::Rgba,
                })
            }),
        }
    }

    /// Where the frames go as they are, in the points of the window. None
    /// when the image view does more than show them stretched over a rect:
    /// a background, a border, round corners, a flip, a crop or a cut.
    #[cfg(apple)]
    fn picture_area(self: Weak<Self>) -> Option<Rect> {
        if !self.is_ok() {
            return None;
        }
        let image = self.image_view;
        let plain = |view: &dyn View| {
            view.color().a <= 0.0
                && (view.border_color().a <= 0.0 || view.border_width() <= 0.0)
                && view.shadow().is_none()
                && view.gradient().is_none()
                && view.corner_radii() == CornerRadii::default()
        };
        let shows_as_is = plain(&*self.weak_view())
            && plain(&*image.weak_view())
            && !image.flip_x
            && !image.flip_y
            && image.cut().is_none()
            && matches!(image.mode, ImageMode::Fill | ImageMode::AspectFit);
        shows_as_is.then(|| image.image_frame())
    }

    /// Render on demand sleeps the loop unless something asks for frames. A
    /// gif with a loop count asks while it plays and shows on screen, and
    /// stops when it stops, hides or dies. It ends, so it is no loop.
    fn keep_frames_coming(mut self: Weak<Self>) {
        if self.__base_view().keeps_frames || !self.playing {
            return;
        }
        // The time the gif stood must not count as one huge step.
        self.last_ms = None;
        keep_frames_exempt(self.weak_view(), move || {
            self.is_visible_on_screen() && self.playing && self.loop_phase().is_none()
        });
    }

    /// Move the current frame on by the time passed since the last update.
    /// Returns whether the shown frame changed.
    fn advance(&mut self, dt: f32) -> bool {
        let start = self.current;
        self.accum += dt;

        while self.playing && self.accum >= self.delays[self.current] {
            self.accum -= self.delays[self.current];
            self.current += 1;

            if self.current >= self.frames.len() {
                self.loops_done += 1;
                if self.loop_count != 0 && self.loops_done >= self.loop_count {
                    self.current = self.frames.len() - 1;
                    self.accum = 0.0;
                    self.playing = false;
                    self.on_finish.trigger(());
                    break;
                }
                self.current = 0;
            }
        }

        self.current != start
    }
}

impl Setup for AnimatedImage {
    fn setup(self: Weak<Self>) {
        self.image_view.place().back();
    }
}

impl ViewCallbacks for AnimatedImage {
    fn update(&mut self) {
        // A declared loop poses the gif itself.
        if self.frames.is_empty()
            || !self.playing
            || self.loop_phase().is_some()
            || !self.is_visible_on_screen()
        {
            return;
        }

        self.weak().keep_frames_coming();

        let now = Clock::now_ms();
        let dt: f32 = self.last_ms.map_or(0.0, |last| ((now - last) / 1000.0).lossy_convert());
        self.last_ms = Some(now);

        if self.advance(dt) {
            let image = self.frames[self.current];
            self.image_view.set_image(image);
        }
    }
}
