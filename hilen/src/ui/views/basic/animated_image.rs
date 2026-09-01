use std::sync::atomic::{AtomicU64, Ordering};

use anyhow::Result;
use ui_proc::view;

use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    gm::{Clock, LossyConvert},
    ui::{ImageMode, ImageView, Setup, UIAnimation, ViewCallbacks, ViewData, ViewFrame},
    window::image::{Image, decode_gif},
};

/// Unique per loaded gif, so one gif's frame textures never collide with
/// another's in the image store.
static NEXT_GIF_ID: AtomicU64 = AtomicU64::new(0);

/// Plays a gif as a frame sequence. Decoded once into one texture per frame,
/// swapped on a timer that reads the engine clock, so it advances on real time
/// in a normal run and on stepped time in a test. The inner `ImageView` does
/// the drawing, so aspect mode, corner radii and the rest work like any image.
#[view]
pub struct AnimatedImage {
    frames: Vec<Weak<Image>>,
    /// Seconds each frame shows, same length as `frames`.
    delays: Vec<f32>,

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

    keeping_alive: bool,

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
        let decoded = decode_gif(data)?;
        let id = NEXT_GIF_ID.fetch_add(1, Ordering::Relaxed);

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

        this.current = 0;
        this.accum = 0.0;
        this.last_ms = None;
        this.loops_done = 0;
        this.playing = true;

        if let Some(first) = this.frames.first().copied() {
            this.image_view.set_image(first);
        }

        this.keep_frames_coming();
        Ok(self)
    }

    pub fn set_mode(&self, mode: ImageMode) -> &Self {
        weak_from_ref(self).image_view.mode = mode;
        self
    }

    /// Zero loops forever, the default. A positive count stops on the last
    /// frame after that many loops and fires `on_finish`.
    pub fn set_loop(&self, count: u32) -> &Self {
        weak_from_ref(self).loop_count = count;
        self
    }

    pub fn play(&self) -> &Self {
        let mut this = weak_from_ref(self);
        if !this.playing && !this.frames.is_empty() {
            this.playing = true;
            this.last_ms = None;
            this.keep_frames_coming();
        }
        self
    }

    pub fn pause(&self) -> &Self {
        weak_from_ref(self).playing = false;
        self
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Show a specific frame. Also usable while paused to drive the gif by
    /// hand. Not `set_frame`, which is the `ViewFrame` rect setter.
    pub fn show_frame(&self, index: usize) -> &Self {
        if let Some(image) = self.frames.get(index).copied() {
            let mut this = weak_from_ref(self);
            this.current = index;
            this.accum = 0.0;
            this.image_view.set_image(image);
        }
        self
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    pub fn current_frame(&self) -> usize {
        self.current
    }

    /// Render on demand sleeps the loop unless continuous work is live. A
    /// playing gif is that work, so an empty animation runs while it plays and
    /// shows on screen, and ends when it stops, hides or dies.
    fn keep_frames_coming(mut self: Weak<Self>) {
        if self.keeping_alive || !self.playing {
            return;
        }
        self.keeping_alive = true;
        let anim = UIAnimation::new(|_, _| {})
            .finish_condition(move || self.is_null() || !self.is_visible_on_screen() || !self.playing);
        anim.on_finish.sub(move || {
            if self.is_ok() {
                self.keeping_alive = false;
                self.last_ms = None;
            }
        });
        self.add_animation(anim);
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
        if self.frames.is_empty() || !self.playing || !self.is_visible_on_screen() {
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
