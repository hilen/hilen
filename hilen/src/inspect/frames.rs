use base64::{Engine, engine::general_purpose::STANDARD};

use crate::{
    deps::hreads::{after, from_main, on_main},
    gm::LossyConvert,
    inspect::{
        AppCommand, InspectService,
        protocol::{FrameRepr, UIRequest, UIResponse},
        wait::MAX_WAIT_MS,
    },
    window::{
        frame_record::{self, RecordedFrame},
        frame_step,
    },
};

/// Every frame is a full picture of the window in the answer.
const MAX_FRAMES: u32 = 120;

/// How long a record with an input waits for that input to start it.
const INPUT_WAIT_MS: u32 = 5000;

impl InspectService {
    pub(super) fn record(frames: u32, input: Option<UIRequest>, wait_ms: u32) -> AppCommand {
        if frames == 0 || frames > MAX_FRAMES {
            return AppCommand::Error(format!("A record takes 1 to {MAX_FRAMES} frames"));
        }
        if wait_ms > MAX_WAIT_MS {
            return AppCommand::Error(format!("Record wait must be at most {MAX_WAIT_MS} milliseconds"));
        }
        if let Some(input) = &input
            && !starts_record(input)
        {
            return AppCommand::Error("A record takes a tap, keys, a hover, a scroll or a drag".into());
        }

        // Armed in a trip of its own. The input comes in a later frame and
        // starts the record there, so the first picture is the frame that
        // holds what the input did.
        let (id, recorded) = match from_main(move || frame_record::arm(frames)) {
            Ok(armed) => armed,
            Err(error) => return AppCommand::Error(error),
        };

        let (note, wait_ms) = match input {
            Some(input) => {
                Self::lay_out_covered();
                match input_note(Self::process_ui_command(input)) {
                    Ok(note) => (note, INPUT_WAIT_MS),
                    Err(error) => {
                        from_main(move || frame_record::cancel_waiting(id));
                        return AppCommand::Error(error);
                    }
                }
            }
            None => (None, wait_ms),
        };

        let seconds: f32 = wait_ms.lossy_convert();
        after(seconds / 1000.0, move || frame_record::cancel_waiting(id));

        // Ends when the record has its last frame or was dropped unstarted.
        let mut shots: Vec<RecordedFrame> = recorded.iter().collect();
        if shots.is_empty() {
            return AppCommand::Error(format!(
                "No input came within {wait_ms} milliseconds, nothing recorded"
            ));
        }
        // Each picture is read back on its own task.
        shots.sort_by_key(|shot| shot.index);

        let frames = shots.iter().map(|frame| {
            Ok(FrameRepr {
                index:      frame.index,
                ms:         frame.ms,
                width:      frame.shot.size.width,
                height:     frame.shot.size.height,
                png_base64: STANDARD.encode(Self::encode_png(&frame.shot)?),
            })
        });
        match frames.collect::<anyhow::Result<Vec<_>>>() {
            Ok(frames) => AppCommand::Frames { frames, note },
            Err(error) => AppCommand::Error(format!("Frame encoding failed: {error}")),
        }
    }

    pub(super) fn pause() -> AppCommand {
        AppCommand::Paused {
            frame: from_main(frame_step::pause),
        }
    }

    pub(super) fn step(frames: u32) -> AppCommand {
        let done = match from_main(move || frame_step::step(frames)) {
            Ok(done) => done,
            Err(error) => return AppCommand::Error(error),
        };
        match done.recv() {
            Ok(frame) => AppCommand::Paused { frame },
            Err(_) => AppCommand::Error("The app was resumed before the frames were drawn".into()),
        }
    }

    pub(super) fn resume() -> AppCommand {
        from_main(frame_step::resume);
        AppCommand::Ok
    }
}

/// The inputs that pass a start point of the record, a press, a key, a
/// wheel turn or the hover move.
fn starts_record(input: &UIRequest) -> bool {
    matches!(
        input,
        UIRequest::Tap { .. }
            | UIRequest::Keys { .. }
            | UIRequest::Hover { .. }
            | UIRequest::Scroll { .. }
            | UIRequest::Drag { .. }
    )
}

/// The note of an input answer, or its error. The tree in it is dropped on
/// the main thread, it holds `Own` pointers.
fn input_note(response: AppCommand) -> Result<Option<String>, String> {
    let result = match &response {
        AppCommand::Error(error) => Err(error.clone()),
        AppCommand::UI(UIResponse::SendUI { note, .. }) => Ok(note.clone()),
        _ => Ok(None),
    };
    on_main(move || drop(response));
    result
}
