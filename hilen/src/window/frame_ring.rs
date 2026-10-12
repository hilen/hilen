//! A record that runs from a start to a stop and keeps its last 10
//! seconds, for the inspector. In a window on screen it takes the frames
//! the app draws and asks for none. Each frame is scaled to 1 pixel per
//! point and packed without loss on worker threads.
//!
//! No frame is lost. When the packers fall behind, the app waits for them
//! and the frame is marked as stalled, so the record shows its own cost.

#[cfg(not_wasm)]
use std::thread::{Builder, available_parallelism, sleep};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        mpsc::{Receiver, Sender, channel},
    },
    time::Duration,
};

use anyhow::{Context, Result};
use image::{ExtendedColorType, ImageEncoder, RgbImage, codecs::qoi::QoiEncoder, imageops::thumbnail};
use log::{debug, error, info, warn};
use parking_lot::{Condvar, Mutex};
use web_time::Instant;

#[cfg(wasm)]
use crate::deps::hreads::spawn_thread;
#[cfg(not_wasm)]
use crate::window::{Window, occluded};
use crate::{
    deps::refs::main_lock::MainLock,
    gm::{LossyConvert, flat::Size},
    ui::UIManager,
    window::{Screenshot, frame_control::FrameReader, frame_step, request_frame},
};

/// A frame older than this, counted from the newest one, is dropped.
const KEPT_MS: f32 = 10_000.0;

/// A frame waits unpacked at full size, 21 MB for a window of 2880 by
/// 1872. With this many in the line the app waits for the packers.
const MAX_WAITING: u32 = 16;

/// How long the app waits for the packers before it gives a frame up. It
/// is the only way a frame is lost, and it means the packers are dead.
const STALL_WAIT: Duration = Duration::from_secs(2);

/// How long a reader of a stopped record waits for its last frames.
const PACK_WAIT: Duration = Duration::from_secs(10);

/// A frame of 60 a second.
#[cfg(not_wasm)]
const FRAME: Duration = Duration::from_micros(16_667);

pub(crate) struct PackedFrame {
    /// The count of the frame from the start of the record, from 0.
    pub index:   u32,
    /// Milliseconds from the start of the record to the draw of the frame.
    pub ms:      f32,
    pub size:    Size<u32>,
    /// The picture in the QOI format, RGB with no loss.
    pub qoi:     Vec<u8>,
    /// The app waited for the packers before it drew this frame.
    pub stalled: bool,
}

/// What a stopped or a running record holds.
pub(crate) struct Kept {
    pub running:  bool,
    /// Frames the app drew since the start.
    pub drawn:    u32,
    /// Frames packed since the start, the kept ones and the ones older
    /// than 10 seconds.
    pub recorded: u32,
    /// Frames given up, see `STALL_WAIT`.
    pub skipped:  u32,
    /// Frames the app waited for the packers at.
    pub stalled:  u32,
    pub frames:   Vec<Arc<PackedFrame>>,
    /// See `Store::shown`.
    pub shown:    BTreeMap<u32, f32>,
}

struct RawFrame {
    record:  u64,
    index:   u32,
    ms:      f32,
    /// The size of the packed picture.
    size:    Size<u32>,
    stalled: bool,
    shot:    Screenshot,
}

/// The main thread side of a running record.
struct Live {
    record:  u64,
    started: Instant,
    taken:   u32,
    /// When the next frame of a window with nothing to pace it is due.
    #[cfg(not_wasm)]
    due:     Instant,
    /// The frame that is drawn and not on its way to the screen yet.
    #[cfg(apple)]
    drawing: Option<u32>,
    raw:     Sender<RawFrame>,
}

static LIVE: MainLock<Option<Live>> = MainLock::new();

struct Store {
    /// The id of the newest record. A frame of an older one is dropped.
    record:   u64,
    running:  bool,
    /// Frames read back and not packed yet.
    waiting:  u32,
    drawn:    u32,
    recorded: u32,
    skipped:  u32,
    stalled:  u32,
    frames:   BTreeMap<u32, Arc<PackedFrame>>,
    /// Milliseconds from the start of the record to the moment a frame
    /// reached the screen, by its index. Only for the frames the screen
    /// told it for.
    shown:    BTreeMap<u32, f32>,
}

static STORE: Mutex<Store> = Mutex::new(Store {
    record:   0,
    running:  false,
    waiting:  0,
    drawn:    0,
    recorded: 0,
    skipped:  0,
    stalled:  0,
    frames:   BTreeMap::new(),
    shown:    BTreeMap::new(),
});

static PACKED: Condvar = Condvar::new();

/// A frame that was asked from the GPU and is not with a packer yet. A
/// reader that is dropped unread gives its place in the line back, so a
/// stopped record never waits for a frame that does not come.
struct Asked {
    record: u64,
    handed: bool,
}

impl Asked {
    fn hand_over(&mut self) {
        self.handed = true;
    }
}

impl Drop for Asked {
    fn drop(&mut self) {
        if !self.handed {
            packed(self.record, None);
        }
    }
}

/// Starts a record and drops the one kept before it.
pub(crate) fn start() -> Result<(), String> {
    let live = LIVE.get_mut();
    if live.is_some() {
        return Err("A record already runs".into());
    }

    let record = {
        let mut store = STORE.lock();
        store.record += 1;
        store.running = true;
        store.waiting = 0;
        store.drawn = 0;
        store.recorded = 0;
        store.skipped = 0;
        store.stalled = 0;
        store.frames.clear();
        store.shown.clear();
        store.record
    };
    // A packer of the old record that waits for room goes on.
    PACKED.notify_all();

    let (raw, frames) = channel();
    let frames = Arc::new(Mutex::new(frames));
    let packers = packers();
    for number in 0..packers {
        let frames = frames.clone();
        spawn_packer(number, move || pack_all(&frames));
    }

    let now = Instant::now();
    *live = Some(Live {
        record,
        started: now,
        taken: 0,
        #[cfg(not_wasm)]
        due: now,
        #[cfg(apple)]
        drawing: None,
        raw,
    });
    info!("Frame record {record} started, {packers} packers");
    request_frame();
    Ok(())
}

/// Stops the running record. Its frames stay until the next start.
pub(crate) fn stop() -> Result<(), String> {
    let Some(live) = LIVE.get_mut().take() else {
        return Err("No record runs".into());
    };
    let mut store = STORE.lock();
    store.running = false;
    info!(
        "Frame record {} stopped after {:.1} s: {} frames drawn, {} skipped, the app waited at {}",
        live.record,
        live.started.elapsed().as_secs_f32(),
        store.drawn,
        store.skipped,
        store.stalled
    );
    Ok(())
}

/// Only the native loop asks, a browser has no covered window to draw for.
#[cfg(not_wasm)]
pub(crate) fn running() -> bool {
    LIVE.get_mut().is_some()
}

/// The frames of the last record, the oldest first. For a stopped record
/// it waits until the last frame is packed. Not for the main thread.
pub(crate) fn kept() -> Result<Kept, String> {
    let mut store = STORE.lock();
    if !store.running && store.waiting > 0 {
        let timed_out = PACKED
            .wait_while_for(&mut store, |store| store.waiting > 0, PACK_WAIT)
            .timed_out();
        if timed_out {
            return Err(format!(
                "{} frames of the record are still not packed",
                store.waiting
            ));
        }
    }
    Ok(Kept {
        running:  store.running,
        drawn:    store.drawn,
        recorded: store.recorded,
        skipped:  store.skipped,
        stalled:  store.stalled,
        frames:   store.frames.values().cloned().collect(),
        shown:    store.shown.clone(),
    })
}

/// The handler for the screen time of the frame that is drawn now, when a
/// record took that frame.
#[cfg(apple)]
pub(crate) fn presented_handler() -> Option<Box<dyn FnOnce(Option<f64>) + Send>> {
    let live = LIVE.get_mut().as_mut()?;
    let index = live.drawing.take()?;
    let record = live.record;
    let started = live.started;
    Some(Box::new(move |ago: Option<f64>| {
        // The screen dropped the frame, it has no time there.
        let Some(ago) = ago else {
            debug!("Frame {index} of record {record} never reached the screen");
            return;
        };
        let ms: f32 = ((started.elapsed().as_secs_f64() - ago) * 1000.0).lossy_convert();
        let mut store = STORE.lock();
        if store.record == record {
            store.shown.insert(index, ms);
        }
    }))
}

/// A covered window and a headless run draw with nothing to pace them.
/// A record there holds each frame to the step of a screen.
#[cfg(not_wasm)]
pub(crate) fn pace() {
    if !(occluded() || Window::headless()) {
        return;
    }
    let Some(live) = LIVE.get_mut() else {
        return;
    };
    // A fixed step from frame to frame. A wait counted from the last
    // frame added the work of each frame to its gap, 4 ms in a debug run.
    let now = Instant::now();
    if let Some(wait) = live.due.checked_duration_since(now) {
        sleep(wait);
        live.due += FRAME;
    } else {
        live.due = now + FRAME;
    }
}

/// Called once per drawn frame, the reader of a running record for it.
pub(crate) fn take_frame() -> Option<FrameReader> {
    // A paused app draws a frame again for a screenshot, that is no new
    // frame.
    if frame_step::holding() {
        return None;
    }

    let live = LIVE.get_mut().as_mut()?;
    let record = live.record;
    let index = live.taken;
    live.taken += 1;

    let stalled = wait_for_room(record, index)?;

    // After the wait, the frame is drawn now.
    let ms = live.started.elapsed().as_secs_f32() * 1000.0;
    #[cfg(apple)]
    {
        live.drawing = Some(index);
    }
    // A covered window draws a frame only when one is asked for, a sound
    // timer alone gave 10 frames a second there.
    #[cfg(not_wasm)]
    if occluded() || Window::headless() {
        request_frame();
    }

    let raw = live.raw.clone();
    let scale = UIManager::scale().max(1.0);
    let mut asked = Asked {
        record,
        handed: false,
    };

    Some(Box::new(move |shot| {
        let frame = RawFrame {
            record,
            index,
            ms,
            size: points(shot.size, scale),
            stalled,
            shot,
        };
        if raw.send(frame).is_ok() {
            asked.hand_over();
        } else {
            debug!("Frame {index} of record {record} has no packer");
        }
    }))
}

/// Takes a place in the line of the packers for a frame. With a full
/// line the app waits here, the engine rule against a blocked main thread
/// is broken on purpose: a record that loses frames tells nothing. The
/// answer is whether it waited, `None` when the frame is given up.
fn wait_for_room(record: u64, index: u32) -> Option<bool> {
    let mut store = STORE.lock();
    store.drawn += 1;
    let stalled = store.waiting >= MAX_WAITING;
    if stalled {
        store.stalled += 1;
        let timed_out = PACKED
            .wait_while_for(
                &mut store,
                |store| store.record == record && store.waiting >= MAX_WAITING,
                STALL_WAIT,
            )
            .timed_out();
        if timed_out {
            if store.skipped == 0 {
                error!("The frame packers do not answer, frame {index} is the first one lost");
            }
            store.skipped += 1;
            return None;
        }
    }
    store.waiting += 1;
    Some(stalled)
}

/// The size of a picture with 1 pixel per point.
fn points(pixels: Size<u32>, scale: f32) -> Size<u32> {
    let side = |pixels: u32| -> u32 {
        let pixels: f32 = pixels.lossy_convert();
        let points: u32 = (pixels / scale).round().lossy_convert();
        points.max(1)
    };
    Size {
        width:  side(pixels.width),
        height: side(pixels.height),
    }
}

/// A packer per 2 cores. The app keeps the others for its frames.
#[cfg(not_wasm)]
fn packers() -> usize {
    available_parallelism().map_or(2, |cores| cores.get() / 2).clamp(2, 8)
}

/// A page has few workers to give.
#[cfg(wasm)]
fn packers() -> usize {
    2
}

#[cfg(not_wasm)]
fn spawn_packer(number: usize, work: impl FnOnce() + Send + 'static) {
    if let Err(error) = Builder::new().name(format!("hilen-frame-packer-{number}")).spawn(work) {
        error!("Frame packer {number} did not start: {error}");
    }
}

#[cfg(wasm)]
fn spawn_packer(number: usize, work: impl FnOnce() + Send + 'static) {
    debug!("Frame packer {number} starts");
    spawn_thread(work);
}

/// Runs until the record is stopped and every reader of it is gone.
fn pack_all(frames: &Mutex<Receiver<RawFrame>>) {
    loop {
        let received = frames.lock().recv();
        let Ok(raw) = received else {
            return;
        };
        let record = raw.record;
        let index = raw.index;
        match pack(raw) {
            Ok(frame) => packed(record, Some(frame)),
            Err(error) => {
                error!("Frame {index} of record {record} was not packed: {error:#}");
                packed(record, None);
            }
        }
    }
}

fn pack(raw: RawFrame) -> Result<PackedFrame> {
    let pixels = raw.shot.size;
    let mut bytes = Vec::with_capacity(raw.shot.data.len() * 3);
    for color in &raw.shot.data {
        bytes.extend_from_slice(&[color.r, color.g, color.b]);
    }
    let mut picture = RgbImage::from_raw(pixels.width, pixels.height, bytes)
        .context("The frame has fewer pixels than its size says")?;
    if raw.size != pixels {
        picture = thumbnail(&picture, raw.size.width, raw.size.height);
    }

    let mut qoi = Vec::new();
    QoiEncoder::new(&mut qoi).write_image(
        picture.as_raw(),
        raw.size.width,
        raw.size.height,
        ExtendedColorType::Rgb8,
    )?;

    Ok(PackedFrame {
        index: raw.index,
        ms: raw.ms,
        size: raw.size,
        qoi,
        stalled: raw.stalled,
    })
}

/// A frame left the line of the packers, kept or lost.
fn packed(record: u64, frame: Option<PackedFrame>) {
    let mut store = STORE.lock();
    if store.record != record {
        return;
    }
    store.waiting = store.waiting.saturating_sub(1);
    if let Some(frame) = frame {
        store.recorded += 1;
        store.frames.insert(frame.index, Arc::new(frame));
        drop_old(&mut store.frames);
        if let Some(oldest) = store.frames.keys().next().copied() {
            store.shown = store.shown.split_off(&oldest);
        }
    } else {
        if store.skipped == 0 {
            warn!("A frame of record {record} was lost on its way to a packer");
        }
        store.skipped += 1;
    }
    PACKED.notify_all();
}

fn drop_old(frames: &mut BTreeMap<u32, Arc<PackedFrame>>) {
    let Some(newest) = frames.values().next_back().map(|frame| frame.ms) else {
        return;
    };
    while let Some(oldest) = frames.first_entry()
        && oldest.get().ms < newest - KEPT_MS
    {
        oldest.remove();
    }
}

#[cfg(test)]
mod test {
    use std::{collections::BTreeMap, sync::Arc};

    use image::{ImageFormat, load_from_memory_with_format};

    use super::{PackedFrame, RawFrame, drop_old, pack, points};
    use crate::{
        gm::{color::U8Color, flat::Size},
        window::Screenshot,
    };

    fn frame(index: u32, ms: f32) -> Arc<PackedFrame> {
        Arc::new(PackedFrame {
            index,
            ms,
            size: Size { width: 1, height: 1 },
            qoi: vec![],
            stalled: false,
        })
    }

    #[test]
    fn keeps_the_last_10_seconds() {
        let mut frames = BTreeMap::new();
        for index in 0..=30 {
            frames.insert(
                index,
                frame(index, f32::from(u8::try_from(index).unwrap()) * 500.0),
            );
            drop_old(&mut frames);
        }
        // The newest is at 15 000 ms, so 5 000 ms is the oldest kept.
        assert_eq!(frames.keys().next(), Some(&10));
        assert_eq!(frames.len(), 21);
    }

    #[test]
    fn a_point_is_a_pixel() {
        let pixels = Size {
            width:  2880,
            height: 1872,
        };
        assert_eq!(
            points(pixels, 2.0),
            Size {
                width:  1440,
                height: 936,
            }
        );
        assert_eq!(points(pixels, 1.0), pixels);
        assert_eq!(
            points(Size { width: 1, height: 1 }, 3.0),
            Size { width: 1, height: 1 }
        );
    }

    #[test]
    fn packs_a_frame_at_its_point_size_with_no_loss() {
        let size = Size { width: 4, height: 2 };
        let color = U8Color {
            r: 201,
            g: 17,
            b: 99,
            a: 255,
        };
        let raw = RawFrame {
            record:  1,
            index:   7,
            ms:      12.0,
            size:    Size { width: 2, height: 1 },
            stalled: true,
            shot:    Screenshot::new(vec![color; 8], size),
        };
        let packed = pack(raw).unwrap();
        assert_eq!(packed.index, 7);
        assert!(packed.stalled);
        let picture = load_from_memory_with_format(&packed.qoi, ImageFormat::Qoi).unwrap().to_rgb8();
        assert_eq!(picture.dimensions(), (2, 1));
        assert_eq!(picture.get_pixel(1, 0).0, [201, 17, 99]);
    }
}
