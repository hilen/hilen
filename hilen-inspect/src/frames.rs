//! The frame record and the frame step of a running app.

use std::{
    fs::{create_dir_all, write},
    path::Path,
};

use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use hilen::inspect::protocol::{AppCommand, Client, InspectorCommand, UIRequest};

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
