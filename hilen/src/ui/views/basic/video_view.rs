use std::sync::atomic::{AtomicU64, Ordering};

use log::error;
use ui_proc::view;

use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    ui::{ImageMode, ImageView, Setup, UIAnimation, ViewCallbacks, ViewData, ViewFrame},
    video::{Player, PlayerEvent, VideoStats},
};

/// Unique per view, so two videos never share a frame texture.
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// Plays a video file or url. The decode runs on its own thread, the sound
/// through the engine's audio and the picture lands in the inner `ImageView`,
/// so aspect mode, corner radii and the rest work like on any image. Desktop
/// only for now, see docs/video.md.
#[view]
pub struct VideoView {
    player: Option<Player>,

    looping: bool,
    volume:  f32,

    keeping_alive: bool,

    /// Fires once when playback reaches the end and the video does not loop.
    pub on_finish: Event<()>,
    /// Fires when the source cannot be opened or decoded, with the reason.
    pub on_error:  Event<String>,

    #[init]
    image_view: ImageView,
}

impl VideoView {
    /// Opens a file path or an http url and shows its first frame. Replaces
    /// whatever was playing.
    pub fn set_source(&self, source: impl AsRef<str>) -> &Self {
        let mut this = weak_from_ref(self);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let mut player = Player::open(source.as_ref(), format!("video-{id}"));
        player.set_volume(this.volume);
        player.set_loop(this.looping);
        this.player = Some(player);
        this.keep_frames_coming();
        self
    }

    pub fn play(&self) -> &Self {
        let mut this = weak_from_ref(self);
        if let Some(player) = this.player.as_mut() {
            player.play();
        }
        this.keep_frames_coming();
        self
    }

    pub fn pause(&self) -> &Self {
        if let Some(player) = weak_from_ref(self).player.as_mut() {
            player.pause();
        }
        self
    }

    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(Player::is_playing)
    }

    /// The source opened and its size and length are known.
    pub fn is_loaded(&self) -> bool {
        self.player.as_ref().is_some_and(Player::is_loaded)
    }

    pub fn seek_to(&self, seconds: f64) -> &Self {
        let mut this = weak_from_ref(self);
        if let Some(player) = this.player.as_mut() {
            player.seek_to(seconds);
        }
        this.keep_frames_coming();
        self
    }

    /// Seconds, zero until the source is loaded.
    pub fn duration(&self) -> f64 {
        self.player.as_ref().map_or(0.0, Player::duration)
    }

    /// Seconds into the video.
    pub fn position(&self) -> f64 {
        self.player.as_ref().map_or(0.0, Player::position)
    }

    /// Linear, 0 is silent and 1 is the file's own level.
    pub fn set_volume(&self, volume: f32) -> &Self {
        let mut this = weak_from_ref(self);
        this.volume = volume;
        if let Some(player) = this.player.as_mut() {
            player.set_volume(volume);
        }
        self
    }

    pub fn set_loop(&self, looping: bool) -> &Self {
        let mut this = weak_from_ref(self);
        this.looping = looping;
        if let Some(player) = this.player.as_mut() {
            player.set_loop(looping);
        }
        self
    }

    pub fn set_mode(&self, mode: ImageMode) -> &Self {
        weak_from_ref(self).image_view.mode = mode;
        self
    }

    pub fn stats(&self) -> VideoStats {
        weak_from_ref(self).player.as_mut().map(Player::stats).unwrap_or_default()
    }

    /// Render on demand sleeps the loop unless continuous work is live. A
    /// playing video is that work, so an empty animation runs while frames are
    /// wanted and ends when the video pauses, hides or dies.
    fn keep_frames_coming(mut self: Weak<Self>) {
        if self.keeping_alive {
            return;
        }
        self.keeping_alive = true;
        let anim = UIAnimation::new(|_, _| {}).finish_condition(move || {
            self.is_null()
                || !self.is_visible_on_screen()
                || !self.player.as_ref().is_some_and(Player::needs_frames)
        });
        anim.on_finish.sub(move || {
            if self.is_ok() {
                self.keeping_alive = false;
            }
        });
        self.add_animation(anim);
    }
}

impl Setup for VideoView {
    fn setup(mut self: Weak<Self>) {
        self.image_view.place().back();
        self.volume = 1.0;
    }
}

impl ViewCallbacks for VideoView {
    fn update(&mut self) {
        if !self.is_visible_on_screen() {
            return;
        }
        let Some(player) = self.player.as_mut() else {
            return;
        };
        let (image, events) = player.update();
        let keep = player.needs_frames();

        if let Some(image) = image {
            self.image_view.set_image(image);
        }
        for event in events {
            match event {
                PlayerEvent::Finished => self.on_finish.trigger(()),
                PlayerEvent::Error(message) => {
                    error!("video: {message}");
                    self.on_error.trigger(message);
                }
            }
        }
        if keep {
            self.weak().keep_frames_coming();
        }
    }
}
