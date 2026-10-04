use std::time::Duration;

use anyhow::{Result, anyhow, ensure};
use hilen::{
    dispatch::{from_main, wait_for_next_frame},
    refs::Weak,
    time::Instant,
    ui::{
        BLACK, Container, ImageMode, Label, Setup, UIManager, VideoView, ViewData, ViewFrame, ViewTest,
        WHITE, view,
    },
    ui_test::{check_colors, checkpoint},
    video::VideoState,
};
use web_sys::{
    Blob, BlobPropertyBag, Url,
    js_sys::{Array, Uint8Array},
};

/// Four solid frames at one per second, red, green, blue, yellow, h264 in
/// an mp4. The same file the desktop playback test plays.
const VIDEO: &[u8] = include_bytes!("colors.mp4");

const SUBTITLES: &str =
    "1\n00:00:00,000 --> 00:00:01,900\nfirst line\n\n2\n00:00:02,000 --> 00:00:03,900\nsecond line\n";

/// The view frame in the 600 by 600 canvas.
const FRAME: (f32, f32, f32, f32) = (100.0, 80.0, 400.0, 240.0);

/// In a browser the page plays the video in a `<video>` element under the
/// canvas and the view erases the frame over it. The element sits exactly
/// under the view, plays, seeks, ends, and reports subtitle lines of a
/// file. The bar is drawn over the video by the engine.
#[view]
struct WebVideo {
    line:     String,
    finished: bool,
    failed:   String,

    #[init]
    title:    Label,
    backdrop: Container,
    video:    VideoView,
    bar:      Label,
    subtitle: Label,
    state:    Label,
}

impl Setup for WebVideo {
    fn setup(mut self: Weak<Self>) {
        self.title.set_text("a video element under the canvas").set_text_size(20);
        self.title.place().lrt(10).h(40);

        // An opaque view behind the video, the hole has to cut it too.
        self.backdrop.set_color("#cfd8e3");
        self.backdrop.set_frame((60, 60, 480, 290));

        self.video.set_frame(FRAME);
        self.video.set_mode(ImageMode::Fill);
        self.video.on_subtitle.val(move |line: Option<String>| {
            self.line = line.unwrap_or_default();
            self.subtitle.set_text(self.line.clone());
        });
        self.video.on_finish.sub(move || {
            self.finished = true;
        });
        self.video.on_error.val(move |error: String| {
            self.failed = error;
        });

        self.bar.set_text("engine bar over the video").set_text_size(18);
        self.bar.set_text_color(WHITE).set_color(BLACK.with_alpha(0.5));
        self.bar.set_frame((100, 280, 400, 40));

        self.subtitle.set_text_size(22).set_text_color(WHITE);
        self.subtitle.set_frame((100, 230, 400, 40));

        self.state.set_text_size(20);
        self.state.place().lrb(10).h(40);
    }
}

fn blob_url(bytes: &[u8], kind: &str) -> Result<String> {
    let parts = Array::new();
    parts.push(&Uint8Array::from(bytes).buffer());
    let options = BlobPropertyBag::new();
    options.set_type(kind);
    let blob = Blob::new_with_buffer_source_sequence_and_options(&parts, &options)
        .map_err(|error| anyhow!("no blob: {error:?}"))?;
    Url::create_object_url_with_blob(&blob).map_err(|error| anyhow!("no blob url: {error:?}"))
}

/// The CSS box of the first video element of the page.
fn element_box() -> Result<(f64, f64, f64, f64)> {
    let element = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.query_selector("video").ok().flatten())
        .ok_or_else(|| anyhow!("the page has no video element"))?;
    let rect = element.get_bounding_client_rect();
    Ok((rect.left(), rect.top(), rect.width(), rect.height()))
}

impl WebVideo {
    fn show(self: Weak<Self>) -> (VideoState, f64) {
        from_main(move || {
            let state = self.video.state();
            let position = self.video.position();
            self.state.set_text(format!("{state:?} at {position:.1} s"));
            (state, position)
        })
    }

    fn wait(self: Weak<Self>, what: &str, mut done: impl FnMut(VideoState, f64) -> bool) -> Result<()> {
        let mut waited = Duration::ZERO;
        loop {
            let (state, position) = self.show();
            let failed = from_main(move || self.failed.clone());
            ensure!(failed.is_empty(), "the video failed: {failed}");
            if done(state, position) {
                return Ok(());
            }
            ensure!(
                waited < Duration::from_secs(20),
                "{what} never happened, the video is {state:?} at {position:.2} s"
            );
            let frame = Instant::now();
            wait_for_next_frame();
            waited += frame.elapsed().min(Duration::from_millis(100));
        }
    }
}

impl ViewTest for WebVideo {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        let video = from_main(|| blob_url(VIDEO, "video/mp4"))?;
        let subtitles = from_main(|| blob_url(SUBTITLES.as_bytes(), "text/plain"))?;

        // A browser starts a silent video with no click on the page.
        from_main(move || {
            view.video.set_volume(0.0);
            view.video.set_source(video);
            view.video.set_subtitle_file(subtitles);
        });

        view.wait("the load", |state, _| state == VideoState::Paused)?;
        let duration = from_main(move || view.video.duration());
        ensure!(
            (duration - 4.0).abs() < 0.2,
            "the video is {duration} s long, not 4"
        );

        // The element sits exactly under the view.
        let (left, top, width, height) = from_main(element_box)?;
        let css = from_main(|| {
            f64::from(UIManager::scale())
                / web_sys::window().map_or(1.0, |window| window.device_pixel_ratio())
        });
        let expected = [FRAME.0, FRAME.1, FRAME.2, FRAME.3].map(|points| f64::from(points) * css);
        for (got, expected) in [left, top, width, height].into_iter().zip(expected) {
            ensure!(
                (got - expected).abs() < 1.0,
                "the element is at {left} {top} {width} {height}, the view at {expected:?}"
            );
        }
        check_colors(PAUSED)?;

        from_main(move || {
            view.video.play();
        });
        view.wait("the playback", |state, position| {
            state == VideoState::Playing && position > 0.3
        })?;
        ensure!(
            from_main(move || view.line.clone()) == "first line",
            "no first subtitle line while the first seconds play"
        );
        checkpoint("playing, the first subtitle line shows")?;

        from_main(move || {
            view.video.pause();
            view.video.seek_to(2.5);
        });
        view.wait("the seek", |state, position| {
            state == VideoState::Paused && (position - 2.5).abs() < 0.1
        })?;
        wait_for_next_frame();
        ensure!(
            from_main(move || view.line.clone()) == "second line",
            "no second subtitle line after the seek to 2.5 s"
        );
        checkpoint("paused at 2.5 s on the blue frame, the second line shows")?;

        from_main(move || {
            view.video.play();
        });
        view.wait("the end", |state, _| state == VideoState::Finished)?;
        ensure!(
            from_main(move || view.finished),
            "on_finish did not fire at the end"
        );
        checkpoint("finished on the last frame")?;

        // A hidden view takes its element off the page.
        from_main(move || {
            view.video.set_hidden(true);
        });
        wait_for_next_frame();
        wait_for_next_frame();
        let (_, _, width, height) = from_main(element_box)?;
        ensure!(
            width == 0.0 && height == 0.0,
            "the element of a hidden view still shows, {width} by {height}"
        );

        Ok(())
    }
}

const PAUSED: &str = "";
