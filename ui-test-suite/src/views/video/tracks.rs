use std::{
    env::temp_dir,
    fs::write,
    thread::sleep,
    time::{Duration, Instant},
};

use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{ImageMode, Label, Setup, VideoView, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
    video::VideoState,
};

/// Four seconds of the ffmpeg `testsrc2` pattern in an mkv, with 2 sound
/// tracks, a 440 Hz tone tagged `eng` at stream 1 and a 1760 Hz tone tagged
/// `ger` at stream 2, and 2 subrip tracks, `eng` at stream 3 and `ger` at
/// stream 4. Each has a line from 0.5 to 1.5 seconds and one from 2 to 3.
const VIDEO: &[u8] = include_bytes!("tracks.mkv");
/// One line from 0.5 to 3.5 seconds.
const SUBTITLES: &[u8] = include_bytes!("tracks.srt");

const ENGLISH_SOUND: usize = 1;
const GERMAN_SOUND: usize = 2;
const ENGLISH_LINES: usize = 3;
const GERMAN_LINES: usize = 4;

/// Polls the main thread until `done` holds.
fn wait_until(what: &str, done: impl Fn() -> bool + Send + Copy + 'static) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !from_main(done) {
        if Instant::now() > deadline {
            bail!("timed out waiting for {what}");
        }
        sleep(Duration::from_millis(10));
    }
    Ok(())
}

/// Waits until the subtitle line on screen is `text`, none for no line.
fn wait_for_line(view: Weak<VideoTracks>, text: Option<&'static str>) -> Result<()> {
    wait_until(text.unwrap_or("the line to clear"), move || {
        view.video.subtitle().as_deref() == text
    })
}

/// Plays the fixture and walks its tracks. Proves the sound and subtitle
/// tracks are listed with their language, a subtitle track reports its lines
/// in step with the clock and clears them, a switch of the sound track keeps
/// the position, a switch of the subtitle track and a seek bring the right
/// lines, a subtitle file from outside the source works the same way, and
/// none turns the lines off. Real time, the sound is the clock. The sound is
/// muted, `the_chosen_sound_track_plays` in the engine pins which track is
/// heard.
#[view]
struct VideoTracks {
    #[init]
    title: Label,
    video: VideoView,
    line:  Label,
    state: Label,
}

impl VideoTracks {
    fn describe(self: Weak<Self>) {
        let sound = self
            .video
            .audio_track()
            .map_or_else(|| "none".to_string(), |index| index.to_string());
        self.state.set_text(format!("sound track: {sound}"));
    }
}

impl Setup for VideoTracks {
    fn setup(self: Weak<Self>) {
        let path = temp_dir().join("hilen-video-tracks.mkv");
        write(&path, VIDEO).expect("the fixture video is writable to the temp dir");
        write(temp_dir().join("hilen-video-tracks.srt"), SUBTITLES)
            .expect("the fixture subtitles are writable to the temp dir");

        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("sound tracks and subtitles");
        self.video.set_frame((20, 100, 560, 350));
        self.line.set_frame((20, 470, 560, 40));
        self.line.set_text("subtitle: none");
        self.state.set_frame((20, 520, 560, 40));

        self.video.on_subtitle.val(move |text| {
            self.line.set_text(format!("subtitle: {}", text.as_deref().unwrap_or("none")));
        });
        self.video
            .set_mode(ImageMode::Fill)
            .set_volume(0.0)
            .set_source(path.to_string_lossy());
    }
}

impl ViewTest for VideoTracks {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_until("the first frame", move || {
            view.video.state() == VideoState::Paused
        })?;

        let sound = from_main(move || view.video.audio_tracks());
        let seen: Vec<_> = sound.iter().map(|track| (track.index, track.language.as_str())).collect();
        ensure!(
            seen == [(ENGLISH_SOUND, "eng"), (GERMAN_SOUND, "ger")],
            "the sound tracks are {sound:?}"
        );
        let lines = from_main(move || view.video.subtitle_tracks());
        let seen: Vec<_> = lines
            .iter()
            .map(|track| (track.index, track.language.as_str(), track.text))
            .collect();
        ensure!(
            seen == [(ENGLISH_LINES, "eng", true), (GERMAN_LINES, "ger", true)],
            "the subtitle tracks are {lines:?}"
        );
        ensure!(
            from_main(move || view.video.audio_track()) == Some(ENGLISH_SOUND),
            "the first sound track plays by default"
        );
        from_main(move || view.describe());

        // The lines of a track come in step with the clock and clear again.
        from_main(move || {
            view.video.set_subtitle_track(Some(ENGLISH_LINES)).play();
        });
        wait_for_line(view, Some("first line"))?;
        let at = from_main(move || view.video.position());
        ensure!(
            (0.5..1.5).contains(&at),
            "the first line shows inside its time, at {at}"
        );
        checkpoint("English subtitles, the first line")?;
        wait_for_line(view, None)?;
        let at = from_main(move || view.video.position());
        ensure!((1.5..2.0).contains(&at), "the line clears at its end, at {at}");

        // A new sound track keeps the position and goes on playing.
        let before = from_main(move || view.video.position());
        from_main(move || {
            view.video.set_audio_track(GERMAN_SOUND);
            view.describe();
        });
        let after = from_main(move || view.video.position());
        ensure!(
            (after - before).abs() < 0.3,
            "the position holds over a sound track switch, {before} to {after}"
        );
        ensure!(
            from_main(move || view.video.audio_track()) == Some(GERMAN_SOUND),
            "the second sound track plays after the switch"
        );
        wait_for_line(view, Some("second line"))?;
        checkpoint("sound track 2, styling dropped from the second line")?;

        // A switch of the subtitle track shows the line of the new one, the
        // one that is running now.
        from_main(move || {
            view.video.set_subtitle_track(Some(GERMAN_LINES));
        });
        wait_for_line(view, Some("zweite Zeile"))?;
        checkpoint("German subtitles, the same moment")?;

        // A seek back brings the earlier line of the track again.
        from_main(move || {
            view.video.seek_to(0.4);
        });
        wait_for_line(view, Some("erste Zeile"))?;
        checkpoint("after a seek back, the first German line")?;

        // A file from outside the source, paused so the line stays.
        from_main(move || {
            view.video.pause();
            view.video.seek_to(1.0);
            view.video
                .set_subtitle_file(temp_dir().join("hilen-video-tracks.srt").to_string_lossy());
        });
        wait_for_line(view, Some("line from a file"))?;
        checkpoint("paused, the line of the subtitle file")?;

        from_main(move || {
            view.video.set_subtitle_track(None);
        });
        wait_for_line(view, None)?;
        checkpoint("subtitles off")?;
        Ok(())
    }
}
