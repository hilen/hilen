//! A record of every frame after an input, for the inspector. All of it
//! runs on the main thread.

use std::sync::mpsc::{Receiver, Sender, channel};
#[cfg(not_wasm)]
use std::{thread::sleep, time::Duration};

use log::debug;
use web_time::Instant;

#[cfg(not_wasm)]
use crate::window::{Window, occluded};
use crate::{
    deps::refs::main_lock::MainLock,
    window::{Screenshot, frame_control::FrameReader, frame_step, request_frame},
};

/// A frame of 60 a second.
#[cfg(not_wasm)]
const FRAME: Duration = Duration::from_micros(16_667);

pub(crate) struct RecordedFrame {
    /// The count of the frame from the input, the frame of the input is 0.
    pub index: u32,
    /// Milliseconds from the input to the draw of the frame.
    pub ms:    f32,
    pub shot:  Screenshot,
}

struct Record {
    id:      u64,
    left:    u32,
    /// Frames taken so far.
    taken:   u32,
    /// `None` until the input comes.
    started: Option<Instant>,
    sender:  Sender<RecordedFrame>,
}

#[derive(Default)]
struct Recorder {
    record:  Option<Record>,
    last_id: u64,
}

static RECORDER: MainLock<Recorder> = MainLock::new();

/// Arms a record of `frames` frames. It starts at the next input, see
/// `start`. The id is for `cancel_waiting`.
pub(crate) fn arm(frames: u32) -> Result<(u64, Receiver<RecordedFrame>), String> {
    let recorder = RECORDER.get_mut();
    if recorder.record.is_some() {
        return Err("Another frame record is armed or running".into());
    }
    recorder.last_id += 1;
    let (sender, receiver) = channel();
    recorder.record = Some(Record {
        id: recorder.last_id,
        left: frames,
        taken: 0,
        started: None,
        sender,
    });
    Ok((recorder.last_id, receiver))
}

/// Starts an armed record. The frame that is drawn next, the one that
/// holds what the input did, is its first picture.
pub(crate) fn start() {
    if let Some(record) = &mut RECORDER.get_mut().record
        && record.started.is_none()
    {
        record.started = Some(Instant::now());
        request_frame();
    }
}

/// Drops a record that no input has started. A running one finishes.
pub(crate) fn cancel_waiting(id: u64) {
    let recorder = RECORDER.get_mut();
    if recorder
        .record
        .as_ref()
        .is_some_and(|record| record.id == id && record.started.is_none())
    {
        recorder.record = None;
    }
}

/// Only the native loop asks, a browser has no covered window to draw for.
#[cfg(not_wasm)]
pub(crate) fn running() -> bool {
    RECORDER
        .get_mut()
        .record
        .as_ref()
        .is_some_and(|record| record.started.is_some())
}

/// Holds the next frame of a running record to the step of a screen, 60
/// frames a second from the input. A covered window and a headless run
/// draw with nothing to pace them, about 1 frame a millisecond, and 30
/// frames then show only the first 30 ms of an animation.
#[cfg(not_wasm)]
pub(crate) fn pace() {
    if !(occluded() || Window::headless()) {
        return;
    }
    let Some(record) = &RECORDER.get_mut().record else {
        return;
    };
    let Some(started) = record.started else {
        return;
    };
    if let Some(wait) = (FRAME * record.taken).checked_sub(started.elapsed()) {
        sleep(wait);
    }
}

/// Called once per drawn frame. A running record takes the frame and asks
/// for the next one, so an app that went idle still fills the record.
pub(crate) fn take_frame() -> Option<FrameReader> {
    // A paused app draws a frame again for a screenshot, that is no new
    // frame.
    if frame_step::holding() {
        return None;
    }

    let recorder = RECORDER.get_mut();
    let record = recorder.record.as_mut()?;
    let started = record.started?;

    let index = record.taken;
    let ms = started.elapsed().as_secs_f32() * 1000.0;
    let sender = record.sender.clone();

    record.taken += 1;
    record.left -= 1;
    if record.left == 0 {
        recorder.record = None;
    } else {
        request_frame();
    }

    Some(Box::new(move |shot| {
        if sender.send(RecordedFrame { index, ms, shot }).is_err() {
            debug!("Frame {index} of a record has no receiver");
        }
    }))
}
