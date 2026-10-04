//! One playing video in a browser. The page plays the source in a `<video>`
//! element that sits behind the canvas, and the view erases the frame over
//! it, so the picture never passes through the engine. The browser decodes
//! in hardware straight to the screen, which is the only way a TV plays 4K
//! and HDR. The element is asked for its state once per frame, no listener
//! is installed.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};

use log::{debug, info, warn};
use parking_lot::Mutex;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    HtmlVideoElement,
    js_sys::Reflect,
    wasm_bindgen::{JsCast, JsValue},
};

use crate::{
    deps::refs::{Weak, main_lock::MainLock},
    gm::{LossyConvert, flat::Rect},
    ui::{ImageMode, UIManager},
    video::{
        AudioTrack, PlayerEvent, SubtitleTrack, VideoSource, VideoState, VideoStats,
        cues::{self, TextCue},
    },
    window::image::Image,
};

/// `HTMLMediaElement.readyState`: the frame at the position is there.
const HAVE_CURRENT_DATA: u16 = 2;
/// And enough after it to play on.
const HAVE_FUTURE_DATA: u16 = 3;

/// A view that is hidden, or sits in a hidden view, gets no `update`, so it
/// cannot take its own element off the page. Every placed element is listed
/// here with the frame it was last placed in, and `hide_unplaced` hides the
/// ones the frame passed by.
struct Shown {
    element: HtmlVideoElement,
    mark:    Arc<Mark>,
}

#[derive(Default)]
struct Mark {
    /// The frame count of the last `place` with a frame.
    placed_at: AtomicU64,
    /// `hide_unplaced` hid the element, the next `place` shows it again.
    swept:     AtomicBool,
}

static SHOWN: MainLock<Vec<Shown>> = MainLock::new();
static FRAME: AtomicU64 = AtomicU64::new(1);

/// Once per frame, after every view had its update.
pub(crate) fn hide_unplaced() {
    let frame = FRAME.fetch_add(1, Ordering::Relaxed);
    let shown = SHOWN.get_mut();

    // The player of a dropped view holds the other half of the mark.
    shown.retain(|entry| Arc::strong_count(&entry.mark) > 1);

    for entry in shown.iter() {
        if entry.mark.placed_at.load(Ordering::Relaxed) < frame
            && !entry.mark.swept.swap(true, Ordering::Relaxed)
        {
            set_style(&entry.element, "display", "none");
        }
    }
}

/// What an async part of the player left for the next frame.
#[derive(Default)]
struct Landed {
    /// The browser refused `play`, with its reason.
    refused: Option<String>,
    /// A subtitle file that finished loading, with the count of the load
    /// that asked for it.
    cues:    Option<(u32, Vec<TextCue>)>,
}

/// How far the source got.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// No frame yet.
    Loading,
    /// The element once had a frame, so a seek is buffering, not loading.
    Ready,
    /// The end was reached and reported, until the next play or seek.
    Finished,
    /// The element reported an error, nothing more comes.
    Failed,
}

pub(crate) struct Player {
    /// A DOM object lives on the main thread only, and so does a view.
    element:  MainLock<HtmlVideoElement>,
    location: String,

    wants_play: bool,
    looping:    bool,
    stage:      Stage,

    landed:          Arc<Mutex<Landed>>,
    /// Counts the subtitle loads, the answer of an older one is dropped.
    cue_load:        u32,
    cues:            Vec<TextCue>,
    subtitle:        Option<String>,
    /// The CSS box and the fit the element has now, to write the style
    /// only when it changes.
    placed:          Option<(Rect, ImageMode)>,
    mark:            Arc<Mark>,
    /// Empty, see `audio_tracks`.
    audio_tracks:    Vec<AudioTrack>,
    subtitle_tracks: Vec<SubtitleTrack>,
    stats_at:        f64,
    stats_had:       f64,
    rate:            f64,
}

impl Player {
    pub(crate) fn open(source: VideoSource, key: String) -> Self {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("a page has a document");

        let element: HtmlVideoElement = document
            .create_element("video")
            .expect("a page can make a video element")
            .unchecked_into();

        element.set_id(&key);
        // A phone browser would open its own fullscreen player without it.
        set_attribute(&element, "playsinline", "");
        set_attribute(&element, "preload", "auto");
        // Under the canvas, out of the page flow. The page keeps its own
        // background off `body` when `html` has one, or it would cover
        // the element, see docs/video.md.
        set_attribute(
            &element,
            "style",
            "position:fixed;left:0;top:0;width:0;height:0;z-index:-1;background:#000;display:none;",
        );

        if !source.headers().is_empty() {
            warn!(
                "video: {} has request headers, a browser video element sends none, the url has to carry the access",
                source.location()
            );
        }

        match crate::web::canvas().and_then(|canvas| canvas.parent_node().map(|parent| (parent, canvas))) {
            Some((parent, canvas)) => {
                if let Err(error) = parent.insert_before(&element, Some(&canvas)) {
                    warn!("video: the element did not go into the page: {error:?}");
                }
            }
            None => warn!("video: the page has no canvas to put the element under"),
        }

        info!("video: opening {} in a video element", source.location());
        element.set_src(source.location());

        let mark = Arc::<Mark>::default();
        SHOWN.get_mut().push(Shown {
            element: element.clone(),
            mark:    Arc::clone(&mark),
        });

        let holder: MainLock<HtmlVideoElement> = MainLock::new();
        holder.set(element);

        Self {
            element: holder,
            location: source.location().to_string(),
            wants_play: false,
            looping: false,
            stage: Stage::Loading,
            landed: Arc::default(),
            cue_load: 0,
            cues: Vec::new(),
            subtitle: None,
            placed: None,
            mark,
            audio_tracks: Vec::new(),
            subtitle_tracks: Vec::new(),
            stats_at: 0.0,
            stats_had: 0.0,
            rate: 0.0,
        }
    }

    fn element(&self) -> &HtmlVideoElement {
        self.element.try_get().expect("the player holds its element")
    }

    pub(crate) fn is_loaded(&self) -> bool {
        matches!(self.stage, Stage::Ready | Stage::Finished)
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.wants_play
    }

    pub(crate) fn state(&self) -> VideoState {
        let element = self.element();
        match self.stage {
            Stage::Failed => return VideoState::Failed,
            Stage::Loading => return VideoState::Loading,
            Stage::Ready | Stage::Finished => {}
        }
        if element.ended() && !self.looping {
            return VideoState::Finished;
        }
        if !self.wants_play {
            return VideoState::Paused;
        }
        if element.ready_state() >= HAVE_FUTURE_DATA && !element.seeking() {
            VideoState::Playing
        } else {
            VideoState::Buffering
        }
    }

    pub(crate) fn duration(&self) -> f64 {
        let duration = self.element().duration();
        if duration.is_finite() { duration } else { 0.0 }
    }

    /// The page draws the picture, the engine only has to keep asking the
    /// element where it is, for the state, the time and the subtitles.
    pub(crate) fn needs_frames(&self) -> bool {
        self.stage != Stage::Failed
            && (self.wants_play || self.stage == Stage::Loading || self.element().seeking())
    }

    pub(crate) fn position(&self) -> f64 {
        self.element().current_time()
    }

    pub(crate) fn play(&mut self) {
        self.wants_play = true;
        self.leave_the_end();

        let promise = match self.element().play() {
            Ok(promise) => promise,
            Err(error) => {
                self.landed.lock().refused = Some(js_text(&error));
                return;
            }
        };

        let landed = Arc::clone(&self.landed);
        spawn_local(async move {
            if let Err(error) = JsFuture::from(promise).await {
                // A pause or a new source right after the play ends it
                // with an abort, which is no failure.
                if js_name(&error) == "AbortError" {
                    debug!("video: a play was cut short by a pause or a new source");
                } else {
                    landed.lock().refused = Some(js_text(&error));
                }
            }
        });
    }

    pub(crate) fn pause(&mut self) {
        self.wants_play = false;
        if let Err(error) = self.element().pause() {
            warn!("video: pause failed: {error:?}");
        }
    }

    /// A play or a seek after the end makes the end reportable again.
    fn leave_the_end(&mut self) {
        if self.stage == Stage::Finished {
            self.stage = Stage::Ready;
        }
    }

    pub(crate) fn seek_to(&mut self, seconds: f64) {
        self.leave_the_end();
        self.element().set_current_time(seconds.max(0.0));
    }

    pub(crate) fn set_volume(&mut self, volume: f32) {
        let volume = volume.clamp(0.0, 1.0);
        let element = self.element();
        element.set_volume(f64::from(volume));
        // A browser starts a silent video without a click, a sounding
        // one only after the user touched the page.
        element.set_muted(volume <= 0.0);
    }

    pub(crate) fn set_loop(&mut self, looping: bool) {
        self.looping = looping;
        self.element().set_loop(looping);
    }

    pub(crate) fn set_speed(&mut self, speed: f64) {
        self.element().set_playback_rate(speed.clamp(0.5, 4.0));
    }

    /// The tracks inside the file, always none here. A video element hands
    /// out neither the sound tracks nor the subtitle tracks of an mkv, the
    /// app gets the subtitles from its server as a file, see
    /// `set_subtitle_file`.
    pub(crate) fn audio_tracks(&self) -> &[AudioTrack] {
        &self.audio_tracks
    }

    pub(crate) fn subtitle_tracks(&self) -> &[SubtitleTrack] {
        &self.subtitle_tracks
    }

    pub(crate) fn audio_track(&self) -> Option<usize> {
        self.audio_tracks.first().map(|track| track.index)
    }

    pub(crate) fn set_audio_track(&mut self, index: usize) {
        warn!(
            "video: {} cannot switch to sound track {index}, a video element plays the first one",
            self.location
        );
    }

    pub(crate) fn set_subtitle_track(&mut self, index: Option<usize>) {
        if let Some(index) = index {
            warn!(
                "video: {} cannot read subtitle track {index} out of the file in a browser",
                self.location
            );
        }
        self.cue_load += 1;
        self.cues.clear();
    }

    /// Loads a subtitle file, `SubRip`, `WebVTT` or ASS, with the headers of
    /// the source. Unlike the video element this request can carry them.
    pub(crate) fn set_subtitle_file(&mut self, source: VideoSource) {
        self.cue_load += 1;
        self.cues.clear();
        let load = self.cue_load;
        let landed = Arc::clone(&self.landed);

        spawn_local(async move {
            let mut request = reqwest::Client::new().get(source.location());
            for (name, value) in source.headers() {
                request = request.header(name, value);
            }
            let text = match request.send().await.and_then(reqwest::Response::error_for_status) {
                Ok(response) => response.text().await,
                Err(error) => Err(error),
            };
            match text {
                Ok(text) => {
                    let cues = cues::parse(&text);
                    info!("video: {} subtitle lines from {}", cues.len(), source.location());
                    landed.lock().cues = Some((load, cues));
                }
                Err(error) => warn!(
                    "video: the subtitle file {} did not load: {error}",
                    source.location()
                ),
            }
        });
    }

    pub(crate) fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    pub(crate) fn stats(&mut self) -> VideoStats {
        let element = self.element();
        let quality = element.get_video_playback_quality();
        let total = f64::from(quality.total_video_frames());
        let dropped = f64::from(quality.dropped_video_frames());
        let presented = total - dropped;

        let now = web_sys::window()
            .and_then(|window| window.performance())
            .map_or(0.0, |performance| performance.now() / 1000.0);
        let (width, height) = (element.video_width(), element.video_height());

        if now - self.stats_at >= 1.0 {
            self.rate = (presented - self.stats_had) / (now - self.stats_at);
            self.stats_at = now;
            self.stats_had = presented;
        }

        VideoStats {
            decoded: total.lossy_convert(),
            presented: presented.lossy_convert(),
            dropped: dropped.lossy_convert(),
            presented_per_second: self.rate,
            // The browser does not tell which decoder it took.
            hardware: false,
            decoder: "browser".to_string(),
            width,
            height,
            frame_rate: 0.0,
        }
    }

    /// Puts the element under the view, `frame` in points, or hides it.
    pub(crate) fn place(&mut self, frame: Option<Rect>, mode: ImageMode) {
        let Some(frame) = frame else {
            if self.placed.take().is_some() {
                set_style(self.element(), "display", "none");
            }
            return;
        };

        self.mark.placed_at.store(FRAME.load(Ordering::Relaxed), Ordering::Relaxed);
        let swept = self.mark.swept.swap(false, Ordering::Relaxed);

        if !swept && self.placed == Some((frame, mode)) {
            return;
        }
        self.placed = Some((frame, mode));

        let (left, top, ratio) = crate::web::canvas_css_origin();
        let css = f64::from(UIManager::scale()) / ratio;
        let px = |points: f32| format!("{:.2}px", f64::from(points) * css);

        let element = self.element();
        set_style(element, "display", "block");
        set_style(
            element,
            "left",
            &format!("{:.2}px", left + f64::from(frame.x()) * css),
        );
        set_style(
            element,
            "top",
            &format!("{:.2}px", top + f64::from(frame.y()) * css),
        );
        set_style(element, "width", &px(frame.width()));
        set_style(element, "height", &px(frame.height()));
        set_style(
            element,
            "object-fit",
            match mode {
                ImageMode::Fill => "fill",
                ImageMode::AspectFit => "contain",
                ImageMode::AspectFill => "cover",
            },
        );
    }

    /// No frame image ever comes, the page draws the picture. The events
    /// are read off the element.
    pub(crate) fn update(&mut self) -> (Option<Weak<Image>>, Vec<PlayerEvent>) {
        let mut events = Vec::new();

        let (refused, cues) = {
            let mut landed = self.landed.lock();
            (landed.refused.take(), landed.cues.take())
        };

        if let Some(reason) = refused {
            self.wants_play = false;
            events.push(PlayerEvent::Error(format!(
                "the browser did not start {}: {reason}",
                self.location
            )));
        }

        if let Some((load, cues)) = cues
            && load == self.cue_load
        {
            self.cues = cues;
        }

        // A cheap handle to the same element, so the fields of the
        // player stay free to change while it is read.
        let element = self.element().clone();

        if self.stage != Stage::Failed
            && let Some(error) = element.error()
        {
            self.stage = Stage::Failed;
            self.wants_play = false;
            events.push(PlayerEvent::Error(format!(
                "{} did not play: {}",
                self.location,
                media_error(error.code(), &error.message())
            )));
        }

        if self.stage == Stage::Loading && element.ready_state() >= HAVE_CURRENT_DATA {
            self.stage = Stage::Ready;
            info!(
                "video: {} loaded, {}x{}, {:.1} s",
                self.location,
                element.video_width(),
                element.video_height(),
                self.duration()
            );
        }

        if element.ended() && !self.looping && self.stage == Stage::Ready {
            self.stage = Stage::Finished;
            self.wants_play = false;
            events.push(PlayerEvent::Finished);
        }

        let line = cues::at(&self.cues, element.current_time()).map(ToString::to_string);
        if line != self.subtitle {
            self.subtitle.clone_from(&line);
            events.push(PlayerEvent::Subtitle(line));
        }

        (None, events)
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        let Some(element) = self.element.try_get() else {
            return;
        };
        if let Err(error) = element.pause() {
            debug!("video: pause on close failed: {error:?}");
        }
        // An element that keeps its source keeps the connection and the
        // decoder, a TV has one hardware decoder.
        if let Err(error) = element.remove_attribute("src") {
            debug!("video: the source did not come off: {error:?}");
        }
        element.load();
        element.remove();
    }
}

fn set_attribute(element: &HtmlVideoElement, name: &str, value: &str) {
    if let Err(error) = element.set_attribute(name, value) {
        warn!("video: the element did not take {name}: {error:?}");
    }
}

fn set_style(element: &HtmlVideoElement, name: &str, value: &str) {
    if let Err(error) = element.style().set_property(name, value) {
        warn!("video: the element did not take the style {name}: {error:?}");
    }
}

fn js_name(error: &JsValue) -> String {
    Reflect::get(error, &"name".into())
        .ok()
        .and_then(|name| name.as_string())
        .unwrap_or_default()
}

fn js_text(error: &JsValue) -> String {
    let message = Reflect::get(error, &"message".into())
        .ok()
        .and_then(|message| message.as_string());
    match message {
        Some(message) => format!("{}: {message}", js_name(error)),
        None => format!("{error:?}"),
    }
}

/// `MediaError.code` in words. 4 is what a TV answers for a codec or a
/// container it has no decoder for.
fn media_error(code: u16, message: &str) -> String {
    let what = match code {
        1 => "the load was aborted",
        2 => "the network failed",
        3 => "the decoder failed",
        4 => "this device cannot play the format",
        _ => "unknown error",
    };
    if message.is_empty() {
        what.to_string()
    } else {
        format!("{what}, {message}")
    }
}
