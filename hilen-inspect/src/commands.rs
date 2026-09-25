use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Subcommand;
use hilen::{
    gm::color::Color,
    inspect::protocol::{AppCommand, Client, InspectorCommand, UIRequest, UIResponse},
    ui::ModifiersState,
};
use serde_json::{json, to_string_pretty};

use super::{
    build_time, drag, find, find_matches, get_ui, hold, keys, print_edited, print_tree, quoted_text,
    resolve_near, resolve_target, run_tests, screenshot, scroll, scroll_to, send, wait,
};

#[derive(Subcommand)]
pub(super) enum Command {
    /// List running apps discovered on the local network
    Apps,
    /// Print a compact overview of the view tree: label, frame, id per line
    Tree,
    /// Print full JSON of every view whose label contains the query, or with
    /// this exact id
    View {
        /// Label substring, case insensitive, or an exact view id
        query: String,
    },
    /// Print the whole view tree as JSON
    Ui,
    /// Save a screenshot as PNG and print its path
    Screenshot {
        /// Output file. Defaults to te-screenshot.png in the temp dir.
        #[arg(short, long)]
        out: Option<PathBuf>,
    },
    /// Edit a layout rule: offset for Side and Anchor rules, ratio for Relative
    /// rules
    EditRule {
        /// View id from `tree` or `view`
        view_id:    String,
        /// Index into the view's placer rules from `view`
        rule_index: usize,
        offset:     f32,
        /// Disable the rule instead of keeping it applied
        #[arg(long)]
        disable:    bool,
    },
    /// Set the text of a `Label`, `Button` or `TextField`
    SetText { view_id: String, text: String },
    /// Set the background color of a view, components 0 to 1
    SetColor {
        view_id: String,
        r:       f32,
        g:       f32,
        b:       f32,
        #[arg(default_value_t = 1.0)]
        a:       f32,
    },
    /// Tap the center of a view: touch began plus ended, like a real click
    Tap {
        /// Exact view id, exact visible text or exact label field name
        /// like `save_button` or `BackupPane.save_button`. Substrings
        /// match only with `--fuzzy`, so a short query can never land on
        /// an unrelated view.
        #[arg(required_unless_present = "near")]
        query:  Option<String>,
        /// Also match label and text substrings, tried after the exact
        /// rungs. An ambiguous query still lists the candidates.
        #[arg(long)]
        fuzzy:  bool,
        /// Tap the view nearest to the view with this exact text, on the
        /// same row. Reaches the unnamed button next to a label.
        #[arg(long, conflicts_with = "query")]
        near:   Option<String>,
        /// With --near, only consider views of this type. Default Button.
        #[arg(long, requires = "near")]
        r#type: Option<String>,
        /// Hold the command modifier for this tap, Cmd on a Mac
        #[arg(long)]
        cmd:    bool,
        #[arg(long)]
        shift:  bool,
        #[arg(long)]
        alt:    bool,
        /// A right click instead of a left one, fires the secondary
        /// action such as a context menu
        #[arg(long)]
        right:  bool,
    },
    /// Move the pointer to a view without pressing a button
    Hover {
        /// Exact id, visible text or label field name, as with tap
        #[arg(required_unless_present = "clear", conflicts_with = "clear")]
        query: Option<String>,
        #[arg(long, requires = "query")]
        fuzzy: bool,
        /// Move the pointer outside the window and clear hover
        #[arg(long)]
        clear: bool,
        /// Milliseconds to wait before replying; use 600 for tooltips
        #[arg(long, default_value_t = 0, value_parser = clap::value_parser!(u32).range(0..=60_000))]
        wait:  u32,
    },
    /// One line per matching view: label, text, absolute frame, status,
    /// id. Matches id, label and text by substring, case insensitive.
    Find {
        query: String,
        /// Include hidden and offscreen views
        #[arg(long)]
        all:   bool,
    },
    /// Poll until a visible view matches the query, then print it
    Wait {
        query:   String,
        /// Seconds to wait before giving up
        #[arg(long, default_value_t = 10.0)]
        timeout: f32,
    },
    /// Drag with the left button held, window points, for drag driven
    /// behavior like selecting text
    Drag {
        from_x: f32,
        from_y: f32,
        to_x:   f32,
        to_y:   f32,
        /// Moved events between begin and end
        #[arg(long, default_value_t = 8)]
        steps:  usize,
    },
    /// Wheel scroll at the window center, or at a view with --at.
    /// Positive dy scrolls toward the top of the content.
    Scroll {
        dy: f32,
        /// Aim at this view instead of the window center
        #[arg(long)]
        at: Option<String>,
    },
    /// Scroll the page until the view is inside the window
    ScrollTo { query: String },
    /// Resize the window, in points
    Resize { width: f32, height: f32 },
    /// Hold physical keys down for a while, then release them, for input
    /// read every frame like walking. A single letter or digit names its
    /// key, anything else is a winit `KeyCode` name like Space or `ArrowUp`
    Hold {
        #[arg(required = true)]
        keys: Vec<String>,
        /// Milliseconds to hold
        #[arg(long, default_value_t = 500, value_parser = clap::value_parser!(u32).range(1..=60_000))]
        ms:   u32,
    },
    /// Type text or press one named key, with modifiers held only for that
    /// input. Keys go where a real keyboard would send them, the focused text
    /// field and the app keymap.
    Keys {
        /// Text to type, every char in order
        #[arg(required_unless_present = "key")]
        text:  Option<String>,
        /// A named key instead of text, a winit `NamedKey` name like Enter,
        /// Escape, Tab, Backspace or `ArrowDown`
        #[arg(long, conflicts_with = "text")]
        key:   Option<String>,
        /// Hold the command modifier, Cmd on a Mac and Ctrl elsewhere
        #[arg(long)]
        cmd:   bool,
        #[arg(long)]
        shift: bool,
        #[arg(long)]
        alt:   bool,
    },
    /// Set the UI scale of the app
    SetScale { scale: f32 },
    /// Play a sound in the app, to tell which instance is which
    PlaySound,
    /// List all edits applied to the app in this session
    Edits,
    /// Run the app's whole UI test suite in the app and report every failure
    RunTests,
    /// When the running app's Rust code was compiled, against the newest source
    /// file here. Tells a stale binary from a current one before anything is
    /// tested against it.
    BuildTime,
}

pub(super) async fn run(client: &Client, command: Command) -> Result<()> {
    match command {
        Command::Apps => unreachable!(),
        Command::Ui => {
            let (scale, root) = get_ui(client).await?;
            println!("{}", to_string_pretty(&json!({ "scale": scale, "root": root }))?);
        }
        Command::Tree => {
            let (_, root) = get_ui(client).await?;
            print_tree(&root, 0);
        }
        Command::View { query } => {
            let (_, root) = get_ui(client).await?;
            let mut found = vec![];
            find_matches(&root, &query, &mut found)?;
            if found.is_empty() {
                bail!("No view matches: {query}");
            }
            for view in found {
                println!("{}", to_string_pretty(&view)?);
            }
        }
        Command::Screenshot { out } => screenshot(client, out).await?,
        Command::PlaySound => {
            send(client, InspectorCommand::PlaySound).await?;
            println!("ok");
        }
        Command::RunTests => run_tests(client).await?,
        Command::BuildTime => build_time(client).await?,
        Command::Edits => {
            let AppCommand::Edits(edits) = send(client, InspectorCommand::ListEdits).await? else {
                bail!("Unexpected response to edits");
            };
            println!("{}", to_string_pretty(&edits)?);
        }
        Command::Tap {
            query,
            fuzzy,
            near,
            r#type,
            cmd,
            shift,
            alt,
            right,
        } => tap(client, query, fuzzy, near, r#type, [cmd, shift, alt], right).await?,
        Command::Hover {
            query,
            fuzzy,
            clear: _,
            wait,
        } => hover(client, query, fuzzy, wait).await?,
        Command::Find { query, all } => find(client, &query, all).await?,
        Command::Wait { query, timeout } => wait(client, &query, timeout).await?,
        Command::Drag {
            from_x,
            from_y,
            to_x,
            to_y,
            steps,
        } => drag(client, (from_x, from_y), (to_x, to_y), steps).await?,
        Command::Scroll { dy, at } => scroll(client, dy, at).await?,
        Command::ScrollTo { query } => scroll_to(client, &query).await?,
        Command::Resize { width, height } => {
            send(client, UIRequest::Resize { width, height }.into()).await?;
            println!("ok");
        }
        Command::Hold { keys, ms } => hold(client, &keys, ms).await?,
        Command::Keys {
            text,
            key,
            cmd,
            shift,
            alt,
        } => keys(client, text, key, [cmd, shift, alt]).await?,
        Command::SetScale { scale } => {
            send(client, UIRequest::SetScale(scale).into()).await?;
            println!("ok");
        }
        edit => run_edit(client, edit).await?,
    }

    Ok(())
}

/// The live edit commands, split out of `run` to keep it readable.
async fn run_edit(client: &Client, command: Command) -> Result<()> {
    match command {
        Command::EditRule {
            view_id,
            rule_index,
            offset,
            disable,
        } => {
            let request = UIRequest::EditRule {
                view_id: view_id.clone(),
                rule_index,
                offset,
                enabled: !disable,
            };
            print_edited(client, request, &view_id).await?;
        }
        Command::SetText { view_id, text } => {
            let request = UIRequest::SetText {
                view_id: view_id.clone(),
                text,
            };
            print_edited(client, request, &view_id).await?;
        }
        Command::SetColor { view_id, r, g, b, a } => {
            let request = UIRequest::SetColor {
                view_id: view_id.clone(),
                color:   Color::rgba(r, g, b, a),
            };
            print_edited(client, request, &view_id).await?;
        }
        _ => unreachable!("only the edit commands reach run_edit"),
    }

    Ok(())
}

async fn tap(
    client: &Client,
    query: Option<String>,
    fuzzy: bool,
    near: Option<String>,
    near_type: Option<String>,
    [cmd, shift, alt]: [bool; 3],
    right: bool,
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
    let AppCommand::UI(UIResponse::SendUI { note, .. }) = send(
        client,
        UIRequest::Tap {
            view_id: target_id,
            modifiers,
            right,
        }
        .into(),
    )
    .await?
    else {
        bail!("Unexpected response to tap");
    };
    println!("tapped");
    if let Some(note) = note {
        println!("warning: {note}");
    }

    Ok(())
}

async fn hover(client: &Client, query: Option<String>, fuzzy: bool, wait_ms: u32) -> Result<()> {
    let view_id = if let Some(query) = query {
        let (_, root) = get_ui(client).await?;
        Some(resolve_target(&root, &query, fuzzy)?.id.clone())
    } else {
        None
    };
    let AppCommand::UI(UIResponse::SendUI { note, .. }) =
        send(client, UIRequest::Hover { view_id, wait_ms }.into()).await?
    else {
        bail!("Unexpected response to hover");
    };
    if let Some(note) = note {
        println!("{note}");
    }
    Ok(())
}
