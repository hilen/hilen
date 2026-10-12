//! The frame record and the frame step of a running app.

use std::{
    fs::{create_dir_all, write},
    path::Path,
};

use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use hilen::inspect::protocol::{AppCommand, Client, InspectorCommand, RecordRepr, UIRequest};
use image::{ImageFormat, load_from_memory_with_format};

use super::send;

/// Sends a record, saves every frame into `out` and prints 1 line per
/// frame with its time since the input. The file name counts the frames
/// from the input, the frame of the input is 0. Answers with the note of
/// the input.
pub(super) async fn record_frames(
    client: &Client,
    frames: u32,
    input: Option<UIRequest>,
    wait_ms: u32,
    out: &Path,
) -> Result<Option<String>> {
    let command = InspectorCommand::Record {
        frames,
        input,
        wait_ms,
    };
    let AppCommand::Frames { frames, note } = send(client, command).await? else {
        bail!("Unexpected response to a record");
    };

    create_dir_all(out)?;
    for frame in &frames {
        let path = out.join(format!("{:03}.png", frame.index));
        write(&path, STANDARD.decode(&frame.png_base64)?)?;
        println!(
            "frame {}  +{:.1} ms  {}x{}  {}",
            frame.index,
            frame.ms,
            frame.width,
            frame.height,
            path.display()
        );
    }

    Ok(note)
}

pub(super) async fn record(client: &Client, frames: u32, out: &Path, timeout: u32) -> Result<()> {
    println!("waiting for a click, a key or a wheel turn in the app");
    record_frames(client, frames, None, timeout * 1000, out).await?;
    Ok(())
}

/// How many pictures 1 answer holds.
const PIECE: u32 = 30;

pub(super) async fn record_start(client: &Client) -> Result<()> {
    record_info(client, InspectorCommand::RecordStart).await?;
    println!("recording, the app keeps the last 10 seconds until record-stop");
    Ok(())
}

pub(super) async fn record_stop(client: &Client) -> Result<()> {
    let record = record_info(client, InspectorCommand::RecordStop).await?;
    print_kept(&record);
    Ok(())
}

/// Saves the pictures of the last record into `out`, named by the count
/// of the frame from the start of the record.
pub(super) async fn kept_frames(client: &Client, out: &Path) -> Result<()> {
    let record = record_info(client, InspectorCommand::RecordInfo).await?;
    let Some(last) = record.frames.last().map(|frame| frame.index) else {
        bail!("The app keeps no record, send record-start first");
    };

    create_dir_all(out)?;
    let mut from = record.frames[0].index;
    let mut before: Option<f32> = None;
    while from <= last {
        let command = InspectorCommand::RecordFrames { from, count: PIECE };
        let AppCommand::KeptFrames(frames) = send(client, command).await? else {
            bail!("Unexpected response to a frames request");
        };
        // A new record started in between, the old frames are gone.
        let Some(newest) = frames.last().map(|frame| frame.info.index) else {
            break;
        };
        for frame in &frames {
            let info = &frame.info;
            let path = out.join(format!("{:05}.png", info.index));
            let qoi = STANDARD.decode(&frame.qoi_base64)?;
            load_from_memory_with_format(&qoi, ImageFormat::Qoi)?
                .save_with_format(&path, ImageFormat::Png)?;
            // The screen time when the app knows it, the draw time if not.
            let (ms, clock) = match info.shown_ms {
                Some(shown) => (shown, "on screen"),
                None => (info.ms, "drawn"),
            };
            let gap = before.map_or(0.0, |before| ms - before);
            before = Some(ms);
            let stalled = if info.stalled {
                "  the app waited for the record"
            } else {
                ""
            };
            println!(
                "frame {}  {clock} {ms:.1} ms  gap {gap:.1} ms  drawn {:.1} ms  {}x{}  {}{stalled}",
                info.index,
                info.ms,
                info.width,
                info.height,
                path.display()
            );
        }
        from = newest + 1;
    }
    print_kept(&record);
    Ok(())
}

fn print_kept(record: &RecordRepr) {
    let state = if record.running { "running" } else { "stopped" };
    let seconds = match (record.frames.first(), record.frames.last()) {
        (Some(first), Some(last)) => (last.ms - first.ms) / 1000.0,
        _ => 0.0,
    };
    println!(
        "{state}, {} frames kept over {seconds:.1} s. The app drew {}, the record took {}, lost {}, the app waited at {}",
        record.frames.len(),
        record.drawn,
        record.recorded,
        record.skipped,
        record.stalled
    );
}

async fn record_info(client: &Client, command: InspectorCommand) -> Result<RecordRepr> {
    let AppCommand::Record(record) = send(client, command).await? else {
        bail!("Unexpected response to a record command");
    };
    Ok(record)
}

pub(super) async fn pause(client: &Client) -> Result<()> {
    let frame = paused_frame(client, InspectorCommand::Pause).await?;
    println!("paused at frame {frame}");
    Ok(())
}

pub(super) async fn step(client: &Client, frames: u32) -> Result<()> {
    let frame = paused_frame(client, InspectorCommand::Step { frames }).await?;
    println!("frame {frame}");
    Ok(())
}

pub(super) async fn resume(client: &Client) -> Result<()> {
    send(client, InspectorCommand::Resume).await?;
    println!("resumed");
    Ok(())
}

async fn paused_frame(client: &Client, command: InspectorCommand) -> Result<u64> {
    let AppCommand::Paused { frame } = send(client, command).await? else {
        bail!("Unexpected response to a frame step command");
    };
    Ok(frame)
}
