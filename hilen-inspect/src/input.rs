//! The commands that send an input to the app. Each can also save the
//! frames the app draws after it.

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Args;
use hilen::{
    inspect::protocol::{AppCommand, Client, Key, UIRequest, UIResponse},
    ui::ModifiersState,
};

use super::{
    frames::record_frames, get_ui, parse_named_key, quoted_text, resolve_near, resolve_target, send,
};

#[derive(Args)]
pub(super) struct RecordArgs {
    /// Save this many frames the app draws after the input, 1 PNG each.
    /// The first is the frame that holds what the input did.
    #[arg(long, requires = "out", value_parser = clap::value_parser!(u32).range(1..=120))]
    frames: Option<u32>,
    /// Folder for the frames of --frames
    #[arg(long, requires = "frames")]
    out:    Option<PathBuf>,
}

/// Sends an input, as a frame record when asked for one, and answers with
/// the note of the input.
async fn send_input(client: &Client, request: UIRequest, record: &RecordArgs) -> Result<Option<String>> {
    if let Some((frames, out)) = record.frames.zip(record.out.as_ref()) {
        return record_frames(client, frames, Some(request), 0, out).await;
    }
    let AppCommand::UI(UIResponse::SendUI { note, .. }) = send(client, request.into()).await? else {
        bail!("Unexpected response to an input");
    };
    Ok(note)
}

pub(super) async fn tap(
    client: &Client,
    query: Option<String>,
    fuzzy: bool,
    (near, near_type): (Option<String>, Option<String>),
    [cmd, shift, alt]: [bool; 3],
    [right, force]: [bool; 2],
    record: &RecordArgs,
) -> Result<()> {
    let (_, root) = get_ui(client).await?;

    let target_id = match (&query, &near) {
        (Some(query), None) => {
            let target = resolve_target(&root, query, fuzzy)?;
            println!("tapping {} {} {}", target.label, quoted_text(target), target.id);
            target.id.clone()
        }
        (None, Some(near)) => {
            let target = resolve_near(&root, near, near_type.as_deref().unwrap_or("Button"))?;
            println!(
                "tapping near {near}: {} {} {}",
                target.label,
                quoted_text(target),
                target.id
            );
            target.id.clone()
        }
        _ => unreachable!("clap requires exactly one of query and --near"),
    };

    let mut modifiers = ModifiersState::empty();
    modifiers.set(ModifiersState::SUPER, cmd);
    modifiers.set(ModifiersState::SHIFT, shift);
    modifiers.set(ModifiersState::ALT, alt);

    // The tapped view is often gone from the fresh tree, a tab swaps the
    // page and a modal button closes the modal, so no lookup afterwards.
    let request = UIRequest::Tap {
        view_id: target_id,
        modifiers,
        right,
        force,
    };
    let note = send_input(client, request, record).await?;
    println!("tapped");
    if let Some(note) = note {
        println!("warning: {note}");
    }

    Ok(())
}

pub(super) async fn hover(
    client: &Client,
    query: Option<String>,
    fuzzy: bool,
    wait_ms: u32,
    record: &RecordArgs,
) -> Result<()> {
    let view_id = if let Some(query) = query {
        let (_, root) = get_ui(client).await?;
        Some(resolve_target(&root, &query, fuzzy)?.id.clone())
    } else {
        None
    };
    let note = send_input(client, UIRequest::Hover { view_id, wait_ms }, record).await?;
    if let Some(note) = note {
        println!("{note}");
    }
    Ok(())
}

pub(super) async fn keys(
    client: &Client,
    text: Option<String>,
    key: Option<String>,
    [cmd, shift, alt]: [bool; 3],
    record: &RecordArgs,
) -> Result<()> {
    let mut modifiers = ModifiersState::empty();
    modifiers.set(ModifiersState::SUPER, cmd);
    modifiers.set(ModifiersState::SHIFT, shift);
    modifiers.set(ModifiersState::ALT, alt);

    let keys = match (text, key) {
        (Some(text), None) => text.chars().map(Key::Char).collect(),
        (None, Some(key)) => vec![Key::Named(parse_named_key(&key)?)],
        _ => unreachable!("clap requires exactly one of text and --key"),
    };

    send_input(client, UIRequest::Keys { keys, modifiers }, record).await?;
    println!("ok");

    Ok(())
}

pub(super) async fn scroll(client: &Client, dy: f32, at: Option<String>, record: &RecordArgs) -> Result<()> {
    let view_id = match at {
        Some(query) => {
            let (_, root) = get_ui(client).await?;
            Some(resolve_target(&root, &query, false)?.id.clone())
        }
        None => None,
    };
    send_input(client, UIRequest::Scroll { view_id, dx: 0.0, dy }, record).await?;
    println!("ok");
    Ok(())
}
