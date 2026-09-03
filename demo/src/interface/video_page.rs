use hilen::{
    dispatch::{on_main, spawn},
    filesystem::Paths,
    gm::LossyConvert,
    refs::Weak,
    ui::{
        BLACK, Button, ImageMode, Label, Setup, Slider, TextAlignment, TextField, VideoView, ViewCallbacks,
        ViewData, WHITE, view,
    },
};

use crate::interface::{
    palette::{ACCENT, SURFACE_ALT, TEXT_DIM},
    scenes::{HEADER_HEIGHT, add_title},
};

/// A file picked from disk plays in a `VideoView`, with the counters the
/// roadmap's acceptance line asks for: the stream's frame rate against the
/// presented one, drops, and whether the hardware decoder is on.
#[view]
pub struct VideoPage {
    /// The progress slider is being written from playback, not dragged.
    updating: bool,

    #[init]
    video:     VideoView,
    path:      TextField,
    play_path: Button,
    open:      Button,
    play:      Button,
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
        self.video.place().t(HEADER_HEIGHT).lr(0).b(150);
        self.video.on_error.val(move |message| {
            self.stats.set_text(format!("error: {message}"));
        });

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
            if !source.is_empty() {
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
    fn pick(self: Weak<Self>) {
        spawn(async move {
            let picked = Paths::pick_file("Video", &["mp4", "mkv", "mov", "webm", "avi", "m4v"]).await;
            on_main(move || {
                if let Some(path) = picked
                    && self.is_ok()
                {
                    self.video.set_source(path.to_string_lossy()).play();
                }
            });
        });
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

        let decoder = if stats.hardware { "hardware" } else { "software" };
        self.stats.set_text(format!(
            "{}x{} {} {decoder}   stream {:.1} fps, presented {:.1} fps   decoded {} presented {} dropped {}   {:.1} / {:.1} s",
            stats.width,
            stats.height,
            stats.decoder,
            stats.frame_rate,
            stats.presented_per_second,
            stats.decoded,
            stats.presented,
            stats.dropped,
            position,
            duration
        ));
    }
}
