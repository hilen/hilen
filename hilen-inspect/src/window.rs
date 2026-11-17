//! The window of the app a command works on, `--window`. Every command goes
//! through `send`, which wraps it in `InWindow` for a window other than the
//! main one.

use std::sync::OnceLock;

use anyhow::{Result, bail};
use hilen::inspect::protocol::{AppCommand, Client, InspectorCommand};

static WINDOW: OnceLock<usize> = OnceLock::new();

pub(super) fn choose(window: Option<usize>) {
    if let Some(window) = window {
        WINDOW.get_or_init(|| window);
    }
}

/// Sends `command` to the chosen window. With none, or with 0, it goes as
/// it is, so an app from before `InWindow` still answers it.
pub(super) async fn send(client: &Client, command: InspectorCommand) -> Result<AppCommand> {
    let command = match WINDOW.get().copied() {
        Some(window) if window != 0 => InspectorCommand::InWindow {
            window,
            command: Box::new(command),
        },
        _ => command,
    };
    match client.send(command).await? {
        AppCommand::Error(err) => bail!("{err}"),
        response => Ok(response),
    }
}

/// Prints 1 line per open window: its number, its size in points and its
/// title.
pub(super) async fn windows(client: &Client) -> Result<()> {
    let AppCommand::Windows(windows) = client.send(InspectorCommand::Windows).await? else {
        bail!("Unexpected response to windows, an app from before the windows command does not know it");
    };
    for window in windows {
        println!(
            "{}  {}x{}  {}",
            window.number, window.width, window.height, window.title
        );
    }
    Ok(())
}
