use std::{
    env::home_dir,
    time::{Duration, Instant},
};

use hilen::{
    Window,
    dispatch::{on_main, spawn},
    filesystem::Paths,
    gm::LossyConvert,
    refs::Weak,
    system::{MediaCommand, MediaSession, NowPlaying},
    ui::{
        BLACK, Button, Cursor, ImageMode, Label, Point, Setup, Slider, TextAlignment, TextDropDown,
        TextField, UIManager, VideoView, ViewCallbacks, ViewData, WHITE, view,
    },
    video::VideoState,
};

use crate::interface::{
    palette::{ACCENT, SURFACE_ALT, TEXT_DIM},
    scenes::{HEADER_HEIGHT, add_title},
};

/// The speeds of the speed drop down, as its rows and as numbers.
const SPEEDS: [(&str, f64); 5] = [
    ("0.5x", 0.5),
    ("1x", 1.0),
    ("1.5x", 1.5),
    ("2x", 2.0),
    ("4x", 4.0),
];

/// How long the pointer stays still over a fullscreen video before it
/// hides.
const POINTER_HIDES_AFTER: Duration = Duration::from_secs(3);

/// A file picked from disk plays in a `VideoView`, with the counters the
/// roadmap's acceptance line asks for: the stream's frame rate against the
/// presented one, drops, and whether the hardware decoder is on.
#[view]
pub struct VideoPage {
    /// The progress slider is being written from playback, not dragged.
    updating: bool,
    /// The rows of the sound track drop down, in the order of the source.
    tracks:   Vec<String>,
    /// Where the pointer was last frame and when it last moved.
    pointer:  Option<(Point, Instant)>,
    /// The last thing a media key or the Now Playing panel asked for.
    asked:    String,
    /// Why the source did not play, shown until the next one is set.
    failed:   String,

    #[init]
    video:     VideoView,
    path:      TextField,
    play_path: Button,
    open:      Button,
    play:      Button,
    track:     TextDropDown,
    speed:     TextDropDown,
    full:      Button,
    progress:  Slider,
    stats:     Label,
}

impl Setup for VideoPage {
    fn setup(mut self: Weak<Self>) {
        add_title(
            self,
            "Video",
            "ffmpeg on a thread, the hardware decoder when the codec allows it, kira for the sound.",
        );

        self.video.set_color(BLACK).set_mode(ImageMode::AspectFit);
        self.video.place().t(HEADER_HEIGHT).lr(0).b(196);
        self.video.on_error.val(move |message| {
            self.failed = message;
        });

        // The system's Now Playing panel and the media keys follow the
        // video, and what they ask for comes back here.
        self.video.on_state.val(move |state| self.report(state));
        MediaSession::on_command().val(self, move |command| self.obey(command));

        for (mut drop, left, width) in [(self.track, 28, 200), (self.speed, 240, 110)] {
            drop.set_text_color(TEXT_DIM);
            drop.set_color(SURFACE_ALT).set_corner_radius(10);
            drop.place().l(left).b(148).size(width, 36);
        }
        self.track.set_values(["No sound track"]);
        self.track.on_changed(move |row: String| self.pick_track(&row));
        self.speed.set_values(SPEEDS.map(|(row, _)| row));
        self.speed.set_value("1x");
        self.speed.on_changed(move |row: String| {
            if let Some((_, speed)) = SPEEDS.iter().find(|(text, _)| *text == row) {
                self.video.set_speed(*speed);
            }
        });

        self.full
            .set_text("Fullscreen")
            .set_color(SURFACE_ALT)
            .set_text_color(TEXT_DIM)
            .set_corner_radius(10);
        self.full.place().l(362).b(148).size(110, 36);
        self.full.on_tap(move || Window::set_fullscreen(!Window::is_fullscreen()));

        // A path or url typed in, the way a media client hands over a stream.
        self.path.set_placeholder("Path or url");
        self.path.place().l(28).r(150).b(104).h(32);
        self.play_path
            .set_text("Play path")
            .set_color(SURFACE_ALT)
            .set_text_color(TEXT_DIM)
            .set_corner_radius(10);
        self.play_path.place().r(28).b(102).size(110, 36);
        self.play_path.on_tap(move || {
            let source = self.path.text().trim().to_string();
            // A path typed the shell way, the home folder as a tilde.
            let source = match (source.strip_prefix("~/"), home_dir()) {
                (Some(rest), Some(home)) => home.join(rest).to_string_lossy().into_owned(),
                _ => source,
            };
            if !source.is_empty() {
                self.failed.clear();
                self.video.set_source(source).play();
            }
        });

        self.open
            .set_text("Open file")
            .set_color(ACCENT)
            .set_text_color(WHITE)
            .set_corner_radius(10);
        self.open.place().l(28).b(56).size(120, 36);
        self.open.on_tap(move || self.pick());

        self.play
            .set_text("Play")
            .set_color(SURFACE_ALT)
            .set_text_color(TEXT_DIM)
            .set_corner_radius(10);
        self.play.place().l(160).b(56).size(90, 36);
        self.play.on_tap(move || self.toggle());

        self.progress
            .set_horizontal()
            .set_track_color(SURFACE_ALT)
            .set_fill_color(ACCENT);
        self.progress.place().l(270).r(28).b(60).h(28);
        self.progress.on_change.val(move |value| {
            if !self.updating {
                let duration = self.video.duration();
                self.video.seek_to(f64::from(value) * duration);
            }
        });

        self.stats
            .set_text_color(TEXT_DIM)
            .set_text_size(13)
            .set_alignment(TextAlignment::Left);
        self.stats.place().l(28).r(28).b(16).h(24);
    }
}

impl VideoPage {
    fn pick(mut self: Weak<Self>) {
        spawn(async move {
            let picked = Paths::pick_file("Video", &["mp4", "mkv", "mov", "webm", "avi", "m4v"]).await;
            on_main(move || {
                if let Some(path) = picked
                    && self.is_ok()
                {
                    self.failed.clear();
                    self.video.set_source(path.to_string_lossy()).play();
                }
            });
        });
    }

    /// The sound track of a row of the drop down.
    fn pick_track(self: Weak<Self>, row: &str) {
        let tracks = self.video.audio_tracks();
        if let Some(track) = self.tracks.iter().position(|text| text == row).and_then(|at| tracks.get(at)) {
            self.video.set_audio_track(track.index);
        }
    }

    /// The next sound track of the source, around to the first, for the
    /// next key of a keyboard.
    fn next_track(mut self: Weak<Self>) {
        let tracks = self.video.audio_tracks();
        let now = self.video.audio_track();
        let at = tracks.iter().position(|track| Some(track.index) == now).unwrap_or(0);
        let next = (at + 1) % tracks.len().max(1);
        if let (Some(track), Some(row)) = (tracks.get(next), self.tracks.get(next).cloned()) {
            self.video.set_audio_track(track.index);
            self.track.set_value(&row);
        }
    }

    /// Fills the sound track drop down once the source says what it has.
    fn list_tracks(mut self: Weak<Self>) {
        let rows: Vec<String> = self
            .video
            .audio_tracks()
            .iter()
            .enumerate()
            .map(|(at, track)| {
                format!(
                    "{} {} {} {}ch",
                    at + 1,
                    track.language,
                    track.codec,
                    track.channels
                )
            })
            .collect();
        if rows.is_empty() || rows == self.tracks {
            return;
        }
        self.track.set_values(rows.clone());
        self.tracks = rows;
    }

    /// Tells the system what plays and whether it plays.
    fn report(self: Weak<Self>, state: VideoState) {
        match state {
            VideoState::Empty | VideoState::Failed => MediaSession::set_now_playing(None),
            VideoState::Loading => {}
            VideoState::Paused | VideoState::Playing | VideoState::Buffering | VideoState::Finished => {
                self.list_tracks();
                MediaSession::set_now_playing(Some(&NowPlaying {
                    title: "hilen demo video".to_string(),
                    duration: self.video.duration(),
                    ..NowPlaying::default()
                }));
                MediaSession::set_playback(state == VideoState::Playing, self.video.position());
            }
        }
    }

    /// A media key or the Now Playing panel asked for something.
    fn obey(mut self: Weak<Self>, command: MediaCommand) {
        self.asked = format!("{command:?}");
        match command {
            MediaCommand::Play => {
                self.video.play();
            }
            MediaCommand::Pause | MediaCommand::Stop => {
                self.video.pause();
            }
            MediaCommand::Toggle => self.toggle(),
            MediaCommand::SeekTo(seconds) => {
                self.video.seek_to(seconds);
            }
            MediaCommand::Next => self.next_track(),
            MediaCommand::Previous => {
                self.video.seek_to(0.0);
            }
        }
    }

    /// The pointer hides over a fullscreen video that plays, once it has
    /// been still for a while, and shows again when it moves.
    fn hide_still_pointer(&mut self) {
        let now = Instant::now();
        let at = UIManager::cursor_position();
        let since = match self.pointer {
            Some((was, since)) if was == at => since,
            _ => now,
        };
        self.pointer = Some((at, since));
        let still = now.duration_since(since) > POINTER_HIDES_AFTER;
        if still && Window::is_fullscreen() && self.video.is_playing() {
            Cursor::hide();
        } else {
            Cursor::show();
        }
    }

    fn toggle(self: Weak<Self>) {
        if self.video.is_playing() {
            self.video.pause();
        } else {
            self.video.play();
        }
    }
}

impl ViewCallbacks for VideoPage {
    fn update(&mut self) {
        self.hide_still_pointer();
        self.full.set_text(if Window::is_fullscreen() {
            "Leave full"
        } else {
            "Fullscreen"
        });

        let stats = self.video.stats();
        let position = self.video.position();
        let duration = self.video.duration();

        self.play.set_text(if self.video.is_playing() { "Pause" } else { "Play" });

        if duration > 0.0 {
            self.updating = true;
            let fraction: f32 = (position / duration).lossy_convert();
            self.progress.set_value(fraction);
            self.updating = false;
        }

        if !self.failed.is_empty() {
            self.stats.set_text(format!("error: {}", self.failed));
            return;
        }

        let decoder = if stats.hardware { "hardware" } else { "software" };
        self.stats.set_text(format!(
            "{}x{} {} {decoder}   stream {:.1} fps, presented {:.1} fps   decoded {} presented {} dropped {}   {:.1} / {:.1} s   sound track {:?}   media key {}",
            stats.width,
            stats.height,
            stats.decoder,
            stats.frame_rate,
            stats.presented_per_second,
            stats.decoded,
            stats.presented,
            stats.dropped,
            position,
            duration,
            self.video.audio_track(),
            if self.asked.is_empty() { "none" } else { &self.asked }
        ));
    }
}
