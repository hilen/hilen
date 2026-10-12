use std::{
    mem::{replace, take},
    net::SocketAddr,
    sync::Arc,
};

use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use hilen::{
    dispatch::{from_main, log_spawn, on_main},
    gm::Animation,
    inspect::protocol::{AppCommand, Client, InspectorCommand, KeptFrameRepr, RecordRepr},
    refs::{Weak, manage::DataManager},
    ui::{
        Anchor::{Bot, Left, Top},
        Button, Container, Focus, ImageMode, ImageView, Label, NamedKey, ScrollView, Setup, Switch,
        TextAlignment, TextDropDown, UIAnimation, UIManager, ViewCallbacks, ViewData, ViewFrame,
        ViewSubviews, ViewTouch, view,
    },
    window::image::Image,
};
use log::{info, warn};
use tokio::task::spawn_blocking;

use crate::ui::{
    MainScreen,
    frames::{
        colors::{Mark, SELECTED},
        pictures::{Pixels, decode, paint_changed, thumb},
        thumb_view::ThumbView,
    },
};

/// How many pictures 1 answer of the app holds.
const PIECE: u32 = 30;

/// A frame of 60 a second.
const FRAME_MS: f32 = 16.667;

const THUMB_HEIGHT: f32 = 78.0;
/// A small picture is drawn on a screen with scale 2.
const THUMB_PIXELS: u32 = 156;
const THUMB_GAP: f32 = 4.0;
const TIME_LINE_HEIGHT: f32 = 30.0;

const SPEEDS: [(&str, f32); 4] = [("1x", 1.0), ("0.5x", 0.5), ("0.25x", 0.25), ("0.1x", 0.1)];

struct Frame {
    index:   u32,
    /// The time of the frame in the record, in milliseconds: when it
    /// reached the screen, or when it was drawn if the app does not know
    /// the screen time.
    ms:      f32,
    /// When the app drew the frame.
    drawn:   f32,
    /// `ms` is a real screen time.
    shown:   bool,
    /// Milliseconds the picture stayed on screen, until the next frame.
    /// The last frame of a record has 1 frame of 60 a second.
    hold:    f32,
    /// The app waited for the record before it drew this frame.
    waited:  bool,
    mark:    Mark,
    qoi:     Arc<Vec<u8>>,
    thumb:   Weak<ThumbView>,
    /// Its piece of the time line.
    segment: Weak<Container>,
    /// The name of the small picture in the image store.
    name:    String,
}

/// A frame as it comes from the app, with its small picture made.
struct Loaded {
    index:  u32,
    drawn:  f32,
    shown:  Option<f32>,
    waited: bool,
    qoi:    Arc<Vec<u8>>,
    thumb:  Pixels,
}

/// What the big picture shows: the record, the frame and the switch.
type Shown = (u64, usize, bool);

/// The frames of the last record of an app. A time line with 1 piece per
/// frame, as wide as the time the frame stayed on screen, a strip of
/// small pictures, the selected frame in big with the pixels that changed
/// in it, and a playback at the speed of the record or slower.
#[view]
pub struct FramesView {
    client:   Option<Arc<Client>>,
    running:  bool,
    /// What the app says about the record, see `RecordRepr`.
    drawn:    u32,
    recorded: u32,
    skipped:  u32,
    stalled:  u32,
    /// Counts the loads, a picture of an older load is dropped.
    record:   u64,
    frames:   Vec<Frame>,
    selected: usize,
    /// The next free place in the strip.
    strip_x:  f32,
    /// The name of the big picture in the image store, empty with none.
    big:      String,
    /// The width the time line was laid out for.
    laid_out: f32,
    playing:  bool,
    /// Counts the playbacks, the animation of an older one does nothing.
    play:     u64,

    #[init]
    back:          Button,
    record_button: Button,
    reload:        Button,
    play_button:   Button,
    speed:         TextDropDown,
    changed:       Switch,
    changed_label: Label,
    status:        Label,
    picture:       ImageView,
    info:          Label,
    time_line:     Container,
    strip:         ScrollView,
}

impl Setup for FramesView {
    fn setup(mut self: Weak<Self>) {
        self.back.set_text("Back").place().tl(10).size(90, 40);
        self.back.on_tap(move || {
            Focus::set_enabled(true);
            UIManager::set_view(MainScreen::new());
        });

        self.record_button.set_text("Record");
        self.record_button.place().t(10).anchor(Left, self.back, 10).size(110, 40);
        self.record_button.on_tap(move || self.record_tapped());

        self.reload.set_text("Reload");
        self.reload.place().t(10).anchor(Left, self.record_button, 10).size(110, 40);
        self.reload.on_tap(move || self.refresh());

        self.play_button.set_text("Play");
        self.play_button.place().t(10).anchor(Left, self.reload, 30).size(110, 40);
        self.play_button.on_tap(move || self.play_tapped());

        self.speed.set_values(SPEEDS.map(|(text, _)| text));
        self.speed.place().t(10).anchor(Left, self.play_button, 10).size(110, 40);

        self.changed.place().t(15).anchor(Left, self.speed, 30).size(52, 30);
        self.changed.on_change(move |_| self.select(self.selected));
        self.changed_label.set_text("Changed pixels").set_alignment(TextAlignment::Left);
        self.changed_label.place().t(10).anchor(Left, self.changed, 4).size(170, 40);

        self.status.set_alignment(TextAlignment::Left);
        self.status.place().lr(10).anchor(Top, self.back, 6).h(26);

        self.strip.place().lr(10).b(10).h(THUMB_HEIGHT + 16.0);
        self.time_line.place().lr(10).anchor(Bot, self.strip, 6).h(TIME_LINE_HEIGHT);
        self.time_line.enable_touch();
        self.time_line.touch().all.val(move |touch| self.scrub(touch.position.x));
        self.info.place().lr(10).anchor(Bot, self.time_line, 4).h(28);

        self.picture.mode = ImageMode::AspectFit;
        self.picture
            .place()
            .lr(10)
            .anchor(Top, self.status, 6)
            .anchor(Bot, self.info, 4);

        // The arrows step through the frames here.
        Focus::set_enabled(false);
        UIManager::keymap().add(self, NamedKey::ArrowLeft, move || self.step(false));
        UIManager::keymap().add(self, NamedKey::ArrowRight, move || self.step(true));
        UIManager::keymap().add(self, NamedKey::Space, move || self.play_tapped());

        self.status.set_text("Connecting");
    }
}

impl ViewCallbacks for FramesView {
    fn update(&mut self) {
        // The pieces of the time line are placed by hand, so a window of
        // another width needs them placed again.
        if (self.time_line.width() - self.laid_out).abs() > 0.5 {
            self.weak().lay_out_time_line();
        }
    }
}

impl FramesView {
    pub fn open(self: Weak<Self>, addr: SocketAddr) {
        log_spawn(async move {
            let client = match Client::connect(addr).await {
                Ok(client) => Arc::new(client),
                Err(error) => {
                    let text = format!("No connection to {addr}: {error:#}");
                    on_main(move || {
                        self.status.set_text(text);
                    });
                    return Err(error);
                }
            };
            info!("Frames screen connected to {addr}");
            on_main(move || {
                let mut view = self;
                view.client = Some(client);
                view.refresh();
            });
            Ok(())
        });
    }

    /// Asks the app what it keeps and loads it.
    fn refresh(self: Weak<Self>) {
        self.ask(InspectorCommand::RecordInfo);
    }

    fn record_tapped(self: Weak<Self>) {
        self.ask(if self.running {
            InspectorCommand::RecordStop
        } else {
            InspectorCommand::RecordStart
        });
    }

    fn ask(self: Weak<Self>, command: InspectorCommand) {
        let Some(client) = self.client.clone() else {
            warn!("No app is connected yet");
            return;
        };
        log_spawn(async move {
            let result = self.load(&client, command).await;
            if let Err(error) = &result {
                let text = format!("{error:#}");
                on_main(move || {
                    self.status.set_text(text);
                });
            }
            result
        });
    }

    async fn load(self: Weak<Self>, client: &Client, command: InspectorCommand) -> Result<()> {
        let record = match client.send(command).await? {
            AppCommand::Record(record) => record,
            AppCommand::Error(error) => bail!("{error}"),
            other => bail!("Unexpected answer to a record command: {other:?}"),
        };
        let (first, last) = match (record.frames.first(), record.frames.last()) {
            (Some(first), Some(last)) => (first.index, last.index),
            _ => (1, 0),
        };
        let running = record.running;
        let id = from_main(move || self.begin(&record));
        // A running record changes under the reader, it is loaded after
        // its stop.
        if running {
            return Ok(());
        }

        let mut from = first;
        while from <= last {
            let command = InspectorCommand::RecordFrames { from, count: PIECE };
            let frames = match client.send(command).await? {
                AppCommand::KeptFrames(frames) => frames,
                AppCommand::Error(error) => bail!("{error}"),
                other => bail!("Unexpected answer to a frames request: {other:?}"),
            };
            // A new record started in between, the old frames are gone.
            let Some(newest) = frames.last().map(|frame| frame.info.index) else {
                break;
            };
            let piece =
                spawn_blocking(move || frames.iter().map(loaded).collect::<Result<Vec<_>>>()).await??;
            on_main(move || self.add(id, piece));
            from = newest + 1;
        }
        Ok(())
    }

    /// Clears the screen for a new load and answers with its id.
    fn begin(mut self: Weak<Self>, record: &RecordRepr) -> u64 {
        self.stop_playing();
        self.record += 1;
        self.running = record.running;
        self.drawn = record.drawn;
        self.recorded = record.recorded;
        self.skipped = record.skipped;
        self.stalled = record.stalled;
        self.selected = 0;
        self.strip_x = THUMB_GAP;

        self.strip.remove_all_subviews();
        self.strip.set_content_width(0);
        self.time_line.remove_all_subviews();
        self.picture.set_image(Weak::<Image>::default());
        for frame in self.frames.drain(..) {
            Image::free_with_name(frame.name);
        }
        Image::free_with_name(take(&mut self.big));

        self.record_button.set_text(if record.running { "Stop" } else { "Record" });
        self.info.set_text("");
        if record.running {
            self.status.set_text("Recording, the app keeps the last 10 seconds");
        } else if record.frames.is_empty() {
            self.status.set_text("The app keeps no record");
        } else {
            self.status.set_text(format!("Loading {} frames", record.frames.len()));
        }
        info!(
            "Record of the app: running {}, {} frames kept, drawn {}, recorded {}, lost {}, waited at {}",
            record.running,
            record.frames.len(),
            record.drawn,
            record.recorded,
            record.skipped,
            record.stalled
        );
        self.record
    }

    fn add(mut self: Weak<Self>, record: u64, piece: Vec<Loaded>) {
        if !self.is_ok() || record != self.record {
            return;
        }
        let was_empty = self.frames.is_empty();

        for loaded in piece {
            let place = self.frames.len();
            // The screen time when the app knows it. A frame never
            // comes before the one it follows.
            let ms = loaded
                .shown
                .unwrap_or(loaded.drawn)
                .max(self.frames.last().map_or(f32::MIN, |before| before.ms));
            // The frame before stayed on screen until this one.
            if let Some(before) = self.frames.last_mut() {
                before.hold = ms - before.ms;
                before.mark = if loaded.waited { Mark::Waited } else { Mark::Plain };
                before.thumb.set_mark(before.mark);
                before.segment.set_color(before.mark.time_line(place - 1));
            }

            let name = format!("inspector-thumb-{record}-{}", loaded.index);
            let size = (loaded.thumb.width, loaded.thumb.height).into();
            let image = Image::from_raw_data(loaded.thumb.rgba, name.clone(), size, 4);

            let width = THUMB_HEIGHT * to_f32(loaded.thumb.width) / to_f32(loaded.thumb.height);
            let thumb = self.strip.add_view::<ThumbView>();
            thumb.show(image);
            thumb.place().l(self.strip_x).t(4).size(width, THUMB_HEIGHT);
            thumb.tapped.sub(move || self.select(place));
            self.strip_x += width + THUMB_GAP;

            let segment = self.time_line.add_view::<Container>();
            segment.set_color(Mark::Plain.time_line(place));

            self.frames.push(Frame {
                index: loaded.index,
                ms,
                drawn: loaded.drawn,
                shown: loaded.shown.is_some(),
                hold: FRAME_MS,
                waited: loaded.waited,
                mark: Mark::Plain,
                qoi: loaded.qoi,
                thumb,
                segment,
                name,
            });
        }
        let width = self.strip_x;
        self.strip.set_content_width(width);
        self.lay_out_time_line();
        self.show_status();

        if was_empty {
            self.select(0);
        }
    }

    /// The milliseconds the record covers, with the last frame on screen.
    fn span(&self) -> f32 {
        match (self.frames.first(), self.frames.last()) {
            (Some(first), Some(last)) => last.ms + last.hold - first.ms,
            _ => 0.0,
        }
    }

    /// Gives every frame its piece of the time line, as wide as the time
    /// its picture stayed on screen.
    fn lay_out_time_line(mut self: Weak<Self>) {
        let width = self.time_line.width();
        self.laid_out = width;
        let span = self.span();
        let Some(first) = self.frames.first().map(|frame| frame.ms) else {
            return;
        };
        if width <= 0.0 || span <= 0.0 {
            return;
        }
        for frame in &self.frames {
            let x = (frame.ms - first) / span * width;
            let piece = (frame.hold / span * width).max(1.0);
            frame.segment.set_frame((x, 0.0, piece, TIME_LINE_HEIGHT));
        }
    }

    /// The frame that was on screen at a point of the time line.
    fn scrub(self: Weak<Self>, x: f32) {
        let (Some(first), width) = (self.frames.first(), self.time_line.width()) else {
            return;
        };
        if width <= 0.0 {
            return;
        }
        let ms = first.ms + (x / width).clamp(0.0, 1.0) * self.span();
        self.stop_playing();
        self.select(self.place_at(ms));
    }

    /// The place of the frame that was on screen at `ms` of the record.
    fn place_at(&self, ms: f32) -> usize {
        self.frames.partition_point(|frame| frame.ms <= ms).saturating_sub(1)
    }

    fn show_status(self: Weak<Self>) {
        if self.frames.is_empty() {
            return;
        }
        let shown = self.frames.iter().filter(|frame| frame.shown).count();
        let times = if shown == 0 {
            "Times are draw times, the app told no screen time.".to_string()
        } else {
            format!("{shown} with the real screen time.")
        };
        self.status.set_text(format!(
            "{} frames over {:.1} s.   The app drew {}, the record took {}, lost {}, waited for the record at \
             {}.   {times}",
            self.frames.len(),
            self.span() / 1000.0,
            self.drawn,
            self.recorded,
            self.skipped,
            self.stalled
        ));
    }

    fn step(self: Weak<Self>, forward: bool) {
        self.stop_playing();
        let next = if forward {
            self.selected + 1
        } else {
            self.selected.saturating_sub(1)
        };
        self.select(next);
    }

    fn play_tapped(mut self: Weak<Self>) {
        if self.playing {
            self.stop_playing();
            return;
        }
        let Some(last) = self.frames.last() else {
            return;
        };
        // From the selected frame, or from the start when it is the last.
        let from = if self.selected + 1 >= self.frames.len() {
            0
        } else {
            self.selected
        };
        let (start, end) = (self.frames[from].ms, last.ms + last.hold);
        let speed = SPEEDS
            .iter()
            .find(|(text, _)| Some(*text) == self.speed.try_get_value())
            .map_or(1.0, |(_, speed)| *speed);

        self.play += 1;
        self.playing = true;
        self.play_button.set_text("Pause");
        self.select(from);

        let id = self.play;
        // In seconds of the record, an animation keeps its values times
        // 1000 inside.
        let run = UIAnimation::new(move |_, seconds| self.play_to(id, seconds * 1000.0)).animation(
            Animation::new(start / 1000.0, end / 1000.0, (end - start) / 1000.0 / speed),
        );
        run.on_finish.sub(move || {
            if self.is_ok() && id == self.play {
                self.stop_playing();
            }
        });
        self.picture.add_animation(run);
    }

    fn play_to(self: Weak<Self>, id: u64, ms: f32) {
        if !self.playing || id != self.play {
            return;
        }
        let place = self.place_at(ms);
        if place != self.selected {
            self.select(place);
        }
    }

    fn stop_playing(mut self: Weak<Self>) {
        if self.playing {
            self.playing = false;
            self.play += 1;
            self.play_button.set_text("Play");
        }
    }

    fn select(mut self: Weak<Self>, place: usize) {
        let Some(frame) = self.frames.get(place) else {
            return;
        };
        if let Some(old) = self.frames.get(self.selected) {
            old.thumb.set_selected(false);
            old.segment.set_color(old.mark.time_line(self.selected));
        }
        frame.thumb.set_selected(true);
        frame.segment.set_color(SELECTED);
        self.info.set_text(frame.text(None));

        // Keeps the selected picture in the middle of the strip.
        let middle = frame.thumb.frame().x() + frame.thumb.width() / 2.0;
        let most = (self.strip_x - self.strip.width()).max(0.0);
        let offset = (middle - self.strip.width() / 2.0).clamp(0.0, most);

        let qoi = frame.qoi.clone();
        let with_changes = self.changed.on() && place > 0;
        let before = with_changes.then(|| self.frames[place - 1].qoi.clone());
        let shown: Shown = (self.record, place, with_changes);

        self.selected = place;
        self.strip.set_content_offset_x(-offset);

        log_spawn(async move {
            let result = spawn_blocking(move || big(&qoi, before.as_deref().map(Vec::as_slice))).await?;
            on_main(move || self.show(shown, result));
            Ok(())
        });
    }

    fn show(mut self: Weak<Self>, shown: Shown, result: Result<(Pixels, Option<usize>)>) {
        let (record, place, with_changes) = shown;
        let now: Shown = (self.record, self.selected, self.changed.on() && self.selected > 0);
        if !self.is_ok() || shown != now {
            return;
        }
        let (pixels, changed) = match result {
            Ok(done) => done,
            Err(error) => {
                warn!("Frame {place} was not shown: {error:#}");
                self.info.set_text(format!("{error:#}"));
                return;
            }
        };

        let name = format!("inspector-big-{record}-{place}-{with_changes}");
        let size = (pixels.width, pixels.height).into();
        let image = Image::from_raw_data(pixels.rgba, name.clone(), size, 4);
        self.picture.set_image(image);
        // Only 1 big picture is kept, a frame is megabytes as a texture.
        if self.big != name {
            Image::free_with_name(replace(&mut self.big, name));
        }

        if let (Some(changed), Some(frame)) = (changed, self.frames.get(place)) {
            self.info.set_text(frame.text(Some(changed)));
        }
    }
}

impl Frame {
    /// The line under the big picture.
    fn text(&self, changed: Option<usize>) -> String {
        let mut text = if self.shown {
            format!(
                "frame {}   on screen at {:.1} ms for {:.1} ms   drawn at {:.1} ms",
                self.index, self.ms, self.hold, self.drawn
            )
        } else {
            format!(
                "frame {}   drawn at {:.1} ms, the next one {:.1} ms later   no screen time",
                self.index, self.ms, self.hold
            )
        };
        if self.mark == Mark::Waited {
            text.push_str(", the app waited for the record");
        }
        if self.waited {
            text.push_str(", drawn after a wait for the record");
        }
        if let Some(changed) = changed {
            text.push_str(&format!("   {changed} pixels changed"));
        }
        text
    }
}

fn loaded(frame: &KeptFrameRepr) -> Result<Loaded> {
    let qoi = STANDARD.decode(&frame.qoi_base64)?;
    Ok(Loaded {
        index:  frame.info.index,
        drawn:  frame.info.ms,
        shown:  frame.info.shown_ms,
        waited: frame.info.stalled,
        thumb:  thumb(&qoi, THUMB_PIXELS)?,
        qoi:    Arc::new(qoi),
    })
}

/// The big picture of a frame, with the pixels that differ from the
/// frame before painted when that frame is given.
fn big(qoi: &[u8], before: Option<&[u8]>) -> Result<(Pixels, Option<usize>)> {
    let frame = decode(qoi)?;
    let Some(before) = before else {
        return Ok((frame, None));
    };
    let (painted, changed) = paint_changed(frame, &decode(before)?)?;
    Ok((painted, Some(changed)))
}

fn to_f32(pixels: u32) -> f32 {
    f32::from(u16::try_from(pixels).unwrap_or(u16::MAX))
}
