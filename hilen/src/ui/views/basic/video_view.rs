use std::sync::atomic::{AtomicU64, Ordering};

use log::error;
use ui_proc::view;

use crate::{
    deps::{
        refs::{Weak, weak_from_ref},
        vents::Event,
    },
    ui::{ImageMode, ImageView, Setup, View, ViewCallbacks, ViewData, ViewFrame},
    video::{AudioTrack, Player, PlayerEvent, SubtitleTrack, VideoSource, VideoState, VideoStats},
};
#[cfg(ffmpeg)]
use crate::{video::VideoPiece, window::image::NoImage};

/// Unique per view, so two videos never share a frame texture.
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// Plays a video file or url. On desktop the decode runs on its own thread,
/// the sound through the engine's audio and the picture lands in the inner
/// `ImageView`, so aspect mode, corner radii and the rest work like on any
/// image. In a browser the page plays it in a `<video>` element under the
/// canvas and the view erases the frame over it, see docs/video.md.
#[view]
pub struct VideoView {
    player: Option<Player>,

    looping: bool,
    volume:  f32,
    speed:   f64,

    state: VideoState,

    /// Fires once when playback reaches the end and the video does not loop.
    pub on_finish:   Event<()>,
    /// Fires when the source cannot be opened or decoded, with the reason.
    pub on_error:    Event<String>,
    /// Fires every time `state` changes, with the new state.
    pub on_state:    Event<VideoState>,
    /// Fires with the subtitle line to show, and with none when it ends. The
    /// app draws the text, the view only says what and when.
    pub on_subtitle: Event<Option<String>>,

    #[init]
    image_view: ImageView,
}

impl VideoView {
    /// Opens a file path or an http or https url and shows its first frame.
    /// Replaces whatever was playing. A `VideoSource` carries request headers
    /// for a stream behind a login.
    pub fn set_source(&self, source: impl Into<VideoSource>) -> &Self {
        let mut this = weak_from_ref(self);
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let mut player = Player::open(source.into(), format!("video-{id}"));
        player.set_volume(this.volume);
        player.set_loop(this.looping);
        player.set_speed(this.speed);
        this.player = Some(player);
        // In a browser the picture is the page under the canvas.
        #[cfg(wasm)]
        {
            this.__base_view().page_hole = true;
        }
        this.keep_frames_coming();
        this.report_state();
        self
    }

    /// Plays a list of pieces as 1 video with 1 position and 1 length. Each
    /// piece is a source with the seconds of it that play. The step from a
    /// piece to the next loses no frame and has no gap in the sound, also
    /// when both are parts of 1 file. `seek_to`, `position` and `duration`
    /// count along the whole list. A piece with no sound track is silent.
    ///
    /// On a view that already plays a list the new list takes its place:
    /// the position stays, cut to the new length, and a list that played
    /// plays on. That is the call after a trim or a reorder. A list with no
    /// pieces empties the view.
    #[cfg(ffmpeg)]
    pub fn set_pieces(&self, pieces: impl IntoIterator<Item = VideoPiece>) -> &Self {
        let mut this = weak_from_ref(self);
        let source = VideoSource::from_pieces(pieces);
        if source.piece_list().is_none_or(|list| list.pieces().is_empty()) {
            this.player = None;
            this.image_view.set_image(NoImage);
            this.report_state();
            return self;
        }
        let was = this.player.as_ref().filter(|player| player.plays_pieces());
        let playing = was.is_some_and(Player::is_playing);
        let mut player = if let Some(was) = was {
            was.replaced_by(source)
        } else {
            let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
            Player::open(source, format!("video-{id}"))
        };
        player.set_volume(this.volume);
        player.set_loop(this.looping);
        player.set_speed(this.speed);
        if playing {
            player.play();
        }
        this.player = Some(player);
        this.keep_frames_coming();
        this.report_state();
        self
    }

    /// Loading, playing, buffering and the rest, see `VideoState`.
    pub fn state(&self) -> VideoState {
        self.player.as_ref().map_or(VideoState::Empty, Player::state)
    }

    fn report_state(mut self: Weak<Self>) {
        let state = self.state();
        if state != self.state {
            self.state = state;
            self.on_state.trigger(state);
        }
    }

    /// Plays from where the video is. On a failed video it opens the source
    /// again first and goes on from the position it failed at.
    pub fn play(&self) -> &Self {
        let mut this = weak_from_ref(self);
        if let Some(player) = this.player.as_mut() {
            player.play();
        }
        this.keep_frames_coming();
        this.report_state();
        self
    }

    pub fn pause(&self) -> &Self {
        let mut this = weak_from_ref(self);
        if let Some(player) = this.player.as_mut() {
            player.pause();
        }
        this.report_state();
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
        this.report_state();
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

    /// How fast the video plays, 1 by default.
    pub fn speed(&self) -> f64 {
        self.speed
    }

    /// Plays at this speed from where it is, 0.5 to 4. The sound keeps its
    /// pitch.
    pub fn set_speed(&self, speed: f64) -> &Self {
        let mut this = weak_from_ref(self);
        this.speed = speed.clamp(0.5, 4.0);
        if let Some(player) = this.player.as_mut() {
            player.set_speed(speed);
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

    /// The sound tracks of the source, empty until it is loaded.
    pub fn audio_tracks(&self) -> Vec<AudioTrack> {
        self.player.as_ref().map_or_default(|player| player.audio_tracks().to_vec())
    }

    /// The `index` of the sound track that plays.
    pub fn audio_track(&self) -> Option<usize> {
        self.player.as_ref().and_then(Player::audio_track)
    }

    /// Switches the sound to the track with this `index` and keeps the
    /// position.
    pub fn set_audio_track(&self, index: usize) -> &Self {
        if let Some(player) = weak_from_ref(self).player.as_mut() {
            player.set_audio_track(index);
        }
        self
    }

    /// The subtitle tracks of the source, empty until it is loaded.
    pub fn subtitle_tracks(&self) -> Vec<SubtitleTrack> {
        self.player.as_ref().map_or_default(|player| player.subtitle_tracks().to_vec())
    }

    /// Reports the lines of the track with this `index` through
    /// `on_subtitle`, none turns subtitles off.
    pub fn set_subtitle_track(&self, index: Option<usize>) -> &Self {
        let mut this = weak_from_ref(self);
        if let Some(player) = this.player.as_mut() {
            player.set_subtitle_track(index);
        }
        this.keep_frames_coming();
        self
    }

    /// Reports the lines of a subtitle file from outside the source, an
    /// `.srt` path or url, through `on_subtitle`.
    pub fn set_subtitle_file(&self, source: impl Into<VideoSource>) -> &Self {
        let mut this = weak_from_ref(self);
        if let Some(player) = this.player.as_mut() {
            player.set_subtitle_file(source.into());
        }
        this.keep_frames_coming();
        self
    }

    /// The subtitle line on screen now.
    pub fn subtitle(&self) -> Option<String> {
        self.player.as_ref().and_then(Player::subtitle).map(ToString::to_string)
    }

    pub fn stats(&self) -> VideoStats {
        weak_from_ref(self).player.as_mut().map_or_default(Player::stats)
    }

    /// Render on demand sleeps the loop unless something asks for frames. A
    /// playing video asks while frames are wanted and stops when it pauses,
    /// hides or dies.
    fn keep_frames_coming(self: Weak<Self>) {
        if self.__base_view().keeps_frames {
            return;
        }
        self.keep_frames_while(move || {
            self.is_visible_on_screen() && self.player.as_ref().is_some_and(Player::needs_frames)
        });
    }
}

#[cfg(wasm)]
impl VideoView {
    /// The element follows the view, and goes when the view is hidden or
    /// scrolled away.
    fn place_element(&mut self) {
        let frame = self.is_visible_on_screen().then(|| *self.absolute_frame());
        let mode = self.image_view.mode;
        if let Some(player) = self.player.as_mut() {
            player.place(frame, mode);
        }
    }
}

impl Setup for VideoView {
    // A flat layer needs a view that clips, and the picture never leaves
    // the view.
    fn clips_to_bounds(&self) -> bool {
        true
    }

    fn setup(mut self: Weak<Self>) {
        // The picture is drawn by a child, and a child is nearer than a
        // later sibling of its parent. As a flat layer the video counts as
        // 1 view at its own depth, so a title or a subtitle line an app
        // puts over it after the video covers the picture.
        self.__base_view().flat_depth = true;
        self.image_view.place().back();
        self.volume = 1.0;
        self.speed = 1.0;
    }
}

impl ViewCallbacks for VideoView {
    fn update(&mut self) {
        #[cfg(wasm)]
        self.place_element();

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
        self.weak().report_state();
        for event in events {
            match event {
                PlayerEvent::Finished => self.on_finish.trigger(()),
                PlayerEvent::Subtitle(text) => self.on_subtitle.trigger(text),
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
