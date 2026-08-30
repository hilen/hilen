use std::{
    collections::HashMap,
    env::{current_dir, temp_dir},
    fs::{read_dir, read_to_string, write},
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    process::exit,
    time::{Duration, UNIX_EPOCH},
};

use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::{Parser, Subcommand};
use hilen::{
    gm::color::Color,
    inspect::protocol::{
        AppCommand, Client, InspectorCommand, Key, SERVICE_TYPE, UIRequest, UIResponse, ui::ViewRepr,
    },
    refs::{Own, hreads::set_current_thread_as_main},
    ui::{ModifiersState, NamedKey},
};
use mdns_sd::{ScopedIp, ServiceDaemon, ServiceEvent};
use serde_json::{Value, from_str, from_value, json, to_string, to_string_pretty, to_value};
use tokio::time::{Instant, timeout, timeout_at};

const NO_APPS: &str = "No running apps discovered. The app must be built with the `inspect` feature and running on the same network.";

#[derive(Parser)]
#[command(name = "hilen-inspect", about = "Inspect and edit UI of running hilen apps")]
struct Cli {
    /// App id from `apps`. Needed only when several apps run.
    #[arg(long, global = true)]
    app: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
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

// Responses hold Own pointers which must drop on the main thread. The
// current_thread runtime keeps everything on this thread.
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    set_current_thread_as_main();

    let cli = Cli::parse();

    if let Command::Apps = cli.command {
        let apps = discover().await?;
        save_cache(&apps)?;
        if apps.is_empty() {
            bail!(NO_APPS);
        }
        for (id, addr) in &apps {
            println!("{id} at {addr}");
        }
        return Ok(());
    }

    let client = connect(cli.app).await?;
    run(&client, cli.command).await
}

async fn run(client: &Client, command: Command) -> Result<()> {
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
        Command::Find { query, all } => find(client, &query, all).await?,
        Command::Wait { query, timeout } => wait(client, &query, timeout).await?,
        Command::Scroll { dy, at } => scroll(client, dy, at).await?,
        Command::ScrollTo { query } => scroll_to(client, &query).await?,
        Command::Resize { width, height } => {
            send(client, UIRequest::Resize { width, height }.into()).await?;
            println!("ok");
        }
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

/// A view with its window space origin and effective visibility. Frames in
/// the tree are local to the parent, locating walks them down.
struct Located<'tree> {
    view:   &'tree ViewRepr,
    x:      f32,
    y:      f32,
    hidden: bool,
}

impl Located<'_> {
    fn status(&self, window: (f32, f32)) -> &'static str {
        if self.hidden {
            return "hidden";
        }
        let size = self.view.frame.size;
        if self.x + size.width < 0.0 || self.y + size.height < 0.0 || self.x > window.0 || self.y > window.1 {
            return "offscreen";
        }
        "visible"
    }

    fn line(&self, window: (f32, f32)) -> String {
        format!(
            "{}{}  [{}, {}] {}x{}  {}  {}",
            self.view.label,
            shortened_text(self.view),
            self.x,
            self.y,
            self.view.frame.size.width,
            self.view.frame.size.height,
            self.status(window),
            self.view.id,
        )
    }
}

fn locate<'tree>(view: &'tree ViewRepr, x: f32, y: f32, hidden: bool, out: &mut Vec<Located<'tree>>) {
    let x = x + view.frame.origin.x;
    let y = y + view.frame.origin.y + view.content_offset;
    let hidden = hidden || view.hidden;
    out.push(Located { view, x, y, hidden });
    for sub in &view.subviews {
        locate(sub, x, y, hidden, out);
    }
}

fn located_tree(root: &ViewRepr) -> ((f32, f32), Vec<Located<'_>>) {
    let window = (root.frame.size.width, root.frame.size.height);
    let mut located = vec![];
    locate(root, 0.0, 0.0, false, &mut located);
    (window, located)
}

fn matches_loosely(view: &ViewRepr, query: &str) -> bool {
    let lowercase = query.to_lowercase();
    view.id == query
        || view.label.to_lowercase().contains(&lowercase)
        || view.text.as_ref().is_some_and(|text| text.to_lowercase().contains(&lowercase))
}

async fn find(client: &Client, query: &str, all: bool) -> Result<()> {
    let (_, root) = get_ui(client).await?;
    let (window, located) = located_tree(&root);

    let mut shown = 0;
    for item in &located {
        if !matches_loosely(item.view, query) {
            continue;
        }
        if !all && item.status(window) != "visible" {
            continue;
        }
        shown += 1;
        println!("{}", item.line(window));
    }

    if shown == 0 {
        bail!(
            "No view matches: {query}{}",
            if all {
                ""
            } else {
                ". Add --all to include hidden and offscreen views."
            }
        );
    }
    Ok(())
}

async fn wait(client: &Client, query: &str, wait_seconds: f32) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs_f32(wait_seconds);

    loop {
        let (_, root) = get_ui(client).await?;
        let (window, located) = located_tree(&root);

        if let Some(item) = located
            .iter()
            .find(|item| matches_loosely(item.view, query) && item.status(window) == "visible")
        {
            println!("{}", item.line(window));
            return Ok(());
        }

        if Instant::now() > deadline {
            bail!("No visible view matched {query} within {wait_seconds}s");
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

async fn scroll(client: &Client, dy: f32, at: Option<String>) -> Result<()> {
    let view_id = match at {
        Some(query) => {
            let (_, root) = get_ui(client).await?;
            Some(resolve_target(&root, &query, false)?.id.clone())
        }
        None => None,
    };
    send(client, UIRequest::Scroll { view_id, dx: 0.0, dy }.into()).await?;
    println!("ok");
    Ok(())
}

/// Repeats window sized scroll steps until the view's center is inside
/// the window. Fuzzy matching, the target is often known only by text.
async fn scroll_to(client: &Client, query: &str) -> Result<()> {
    for _ in 0..16 {
        let (_, root) = get_ui(client).await?;
        let (window, located) = located_tree(&root);
        // Loose matching and the first hit, an ambiguous query is fine
        // here, any of the matches leads the scroll to the same place.
        let Some(item) = located.iter().find(|item| !item.hidden && matches_loosely(item.view, query)) else {
            bail!("No view matches: {query}");
        };

        let center = item.y + item.view.frame.size.height / 2.0;
        if center > 0.0 && center < window.1 && !item.hidden {
            println!("{}", item.line(window));
            return Ok(());
        }

        send(
            client,
            UIRequest::Scroll {
                view_id: None,
                dx:      0.0,
                dy:      window.1 / 2.0 - center,
            }
            .into(),
        )
        .await?;
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
    bail!("Could not scroll {query} into view");
}

/// The type name is the last path segment of the label, `Button` from
/// `DeploymentCard.open: full::path::Button`.
fn type_name(view: &ViewRepr) -> &str {
    view.label
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .rsplit(':')
        .next()
        .unwrap_or_default()
        .trim()
}

/// The visible view of `wanted_type` nearest to the view with `anchor`
/// exact text, preferring the same row. Reaches controls with no text of
/// their own, like the open button on a list card.
fn resolve_near<'tree>(root: &'tree ViewRepr, anchor: &str, wanted_type: &str) -> Result<&'tree ViewRepr> {
    let (window, located) = located_tree(root);
    let lowercase = anchor.to_lowercase();

    // An exact id works as the anchor too, it is the way out when several
    // views carry the same text.
    let anchors: Vec<&Located> = located
        .iter()
        .filter(|item| {
            item.view.id == anchor
                || (item.status(window) == "visible"
                    && item.view.text.as_ref().is_some_and(|text| text.to_lowercase() == lowercase))
        })
        .collect();

    let anchor_item = match anchors.as_slice() {
        [] => bail!("No visible view has the exact text: {anchor}"),
        [only] => only,
        candidates => {
            let listed: Vec<String> = candidates.iter().map(|item| item.line(window)).collect();
            bail!("Ambiguous anchor: {anchor}\n{}", listed.join("\n"));
        }
    };

    let anchor_center = (
        anchor_item.x + anchor_item.view.frame.size.width / 2.0,
        anchor_item.y + anchor_item.view.frame.size.height / 2.0,
    );

    let nearest = located
        .iter()
        .filter(|item| {
            item.status(window) == "visible"
                && item.view.id != anchor_item.view.id
                && type_name(item.view).eq_ignore_ascii_case(wanted_type)
        })
        .min_by(|a, b| {
            let distance = |item: &Located| {
                let x = item.x + item.view.frame.size.width / 2.0 - anchor_center.0;
                let y = item.y + item.view.frame.size.height / 2.0 - anchor_center.1;
                // Off row candidates lose to same row ones, a column
                // neighbor is almost never the wanted control.
                x.hypot(y * 4.0)
            };
            distance(a).total_cmp(&distance(b))
        });

    match nearest {
        Some(item) => Ok(item.view),
        None => bail!("No visible {wanted_type} found near {anchor}"),
    }
}

async fn screenshot(client: &Client, out: Option<PathBuf>) -> Result<()> {
    let AppCommand::Screenshot {
        width,
        height,
        png_base64,
    } = send(client, InspectorCommand::Screenshot).await?
    else {
        bail!("Unexpected response to screenshot");
    };
    let path = out.unwrap_or_else(|| temp_dir().join("te-screenshot.png"));
    write(&path, STANDARD.decode(png_base64)?)?;
    println!("{width}x{height} saved to {}", path.display());
    Ok(())
}

async fn keys(
    client: &Client,
    text: Option<String>,
    key: Option<String>,
    [cmd, shift, alt]: [bool; 3],
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

    send(client, UIRequest::Keys { keys, modifiers }.into()).await?;
    println!("ok");

    Ok(())
}

/// `NamedKey` has no `FromStr`, its serde form is the plain variant name,
/// so the name round trips through a JSON string.
fn parse_named_key(name: &str) -> Result<NamedKey> {
    match from_value(Value::String(name.to_string())) {
        Ok(key) => Ok(key),
        Err(_) => {
            bail!("Unknown key: {name}. Use a winit NamedKey name like Enter, Escape, Tab or ArrowDown")
        }
    }
}

async fn run_tests(client: &Client) -> Result<()> {
    let AppCommand::TestResults { total, failures } = send(client, InspectorCommand::RunTests).await? else {
        bail!("Unexpected response to run-tests");
    };

    println!("{total} tests, {} failed", failures.len());

    for failure in &failures {
        println!("\n===== {} =====\n{}", failure.name, failure.detail);
    }

    if !failures.is_empty() {
        exit(1);
    }

    Ok(())
}

async fn send(client: &Client, command: InspectorCommand) -> Result<AppCommand> {
    match client.send(command).await? {
        AppCommand::Error(err) => bail!("{err}"),
        response => Ok(response),
    }
}

async fn get_ui(client: &Client) -> Result<(f32, Own<ViewRepr>)> {
    let AppCommand::UI(UIResponse::SendUI { scale, root, .. }) =
        send(client, UIRequest::GetUI.into()).await?
    else {
        bail!("Unexpected response to get ui");
    };
    Ok((scale, root))
}

/// Prints the fresh post-layout state of the edited view. The whole tree
/// would flood the output, `ui` prints it when needed.
async fn print_edited(client: &Client, request: UIRequest, view_id: &str) -> Result<()> {
    let AppCommand::UI(UIResponse::SendUI { root, .. }) = send(client, request.into()).await? else {
        bail!("Unexpected response to an edit");
    };

    let Some(view) = find_by_id(&root, view_id) else {
        bail!("Edited view {view_id} is gone from the fresh tree");
    };

    println!("{}", to_string_pretty(&view_json(view)?)?);

    Ok(())
}

fn print_tree(view: &ViewRepr, depth: usize) {
    let frame = &view.frame;
    println!(
        "{}{}{}  [{}, {}] {}x{}  {}{}",
        "  ".repeat(depth),
        view.label,
        shortened_text(view),
        frame.origin.x,
        frame.origin.y,
        frame.size.width,
        frame.size.height,
        view.id,
        if view.hidden { "  hidden" } else { "" },
    );
    for sub in &view.subviews {
        print_tree(sub, depth + 1);
    }
}

fn shortened_text(view: &ViewRepr) -> String {
    let Some(text) = &view.text else {
        return String::new();
    };

    let chars: Vec<char> = text.chars().collect();
    if chars.len() > 30 {
        let short: String = chars.into_iter().take(30).collect();
        format!(" \"{short}...\"")
    } else {
        format!(" \"{text}\"")
    }
}

fn quoted_text(view: &ViewRepr) -> String {
    view.text.as_ref().map_or_else(|| "-".to_string(), |text| format!("\"{text}\""))
}

/// Exact id, exact text, then exact label field name, all case
/// insensitive. Substring rungs run only with `fuzzy`, so a short query
/// can never land on an unrelated view, `back` once matched
/// `BackupPane.save_button` and pressed save. The first rung with any
/// match decides: one match wins, more than one errors listing the
/// candidates. Hidden views and everything under them never match a
/// query, a hidden view is only reachable by exact id.
fn resolve_target<'a>(root: &'a ViewRepr, query: &str, fuzzy: bool) -> Result<&'a ViewRepr> {
    if let Some(view) = find_by_id(root, query) {
        return Ok(view);
    }

    let query = query.to_lowercase();

    let exact_text = |view: &ViewRepr| view.text.as_ref().is_some_and(|text| text.to_lowercase() == query);
    // The label is `Owner.field: full::type::Path`, the query matches the
    // owner dot field part or the bare field name.
    let exact_field = |view: &ViewRepr| {
        let name = view.label.split(':').next().unwrap_or_default().to_lowercase();
        name == query || name.rsplit('.').next().unwrap_or_default() == query
    };
    let label_substring = |view: &ViewRepr| view.label.to_lowercase().contains(&query);
    let text_substring =
        |view: &ViewRepr| view.text.as_ref().is_some_and(|text| text.to_lowercase().contains(&query));

    let mut rungs: Vec<&dyn Fn(&ViewRepr) -> bool> = vec![&exact_text, &exact_field];
    if fuzzy {
        rungs.push(&label_substring);
        rungs.push(&text_substring);
    }

    for matches_query in rungs {
        let mut found = vec![];
        collect_visible(root, matches_query, &mut found);

        match found.as_slice() {
            [] => {}
            [only] => return Ok(only),
            candidates => {
                let listed: Vec<String> = candidates
                    .iter()
                    .map(|view| format!("  {} {} {}", view.label, quoted_text(view), view.id))
                    .collect();
                bail!("Ambiguous query: {query}\n{}", listed.join("\n"));
            }
        }
    }

    if fuzzy {
        bail!("No view matches: {query}");
    }
    bail!("No view matches: {query}. Exact matching only, add --fuzzy for substrings.");
}

fn collect_visible<'a>(
    view: &'a ViewRepr,
    matches_query: &dyn Fn(&ViewRepr) -> bool,
    found: &mut Vec<&'a ViewRepr>,
) {
    if view.hidden {
        return;
    }
    if matches_query(view) {
        found.push(view);
    }
    for sub in &view.subviews {
        collect_visible(sub, matches_query, found);
    }
}

fn find_by_id<'a>(view: &'a ViewRepr, id: &str) -> Option<&'a ViewRepr> {
    if view.id == id {
        return Some(view);
    }
    view.subviews.iter().find_map(|sub| find_by_id(sub, id))
}

fn find_matches(view: &ViewRepr, query: &str, found: &mut Vec<Value>) -> Result<()> {
    if view.id == query || view.label.to_lowercase().contains(&query.to_lowercase()) {
        found.push(view_json(view)?);
    }
    for sub in &view.subviews {
        find_matches(sub, query, found)?;
    }
    Ok(())
}

/// Full view JSON with the subview subtrees replaced by their labels,
/// so printing a container does not dump everything under it.
fn view_json(view: &ViewRepr) -> Result<Value> {
    let mut value = to_value(view)?;
    let labels: Vec<&str> = view.subviews.iter().map(|sub| sub.label.as_str()).collect();
    value["subviews"] = json!(labels);
    Ok(value)
}

/// Tries the address cached by the last discovery first and falls back to a
/// fresh mDNS browse, so repeat calls skip the discovery wait.
async fn connect(app: Option<String>) -> Result<Client> {
    if let Some(addr) = cached_addr(app.as_deref())
        && let Ok(Ok(client)) = timeout(Duration::from_secs(1), Client::connect(addr)).await
    {
        return Ok(client);
    }

    let apps = discover().await?;
    save_cache(&apps)?;
    let addr = resolve(&apps, app)?;

    Client::connect(addr).await
}

fn cache_path() -> PathBuf {
    temp_dir().join("hilen-inspect-apps.json")
}

fn cached_addr(app: Option<&str>) -> Option<SocketAddr> {
    let cache: HashMap<String, SocketAddr> = from_str(&read_to_string(cache_path()).ok()?).ok()?;
    match app {
        Some(id) => cache.get(id).copied(),
        // A single cached app can be trusted without a browse. With several,
        // discover every time, correctness over speed.
        None => {
            if cache.len() == 1 {
                cache.values().next().copied()
            } else {
                None
            }
        }
    }
}

fn save_cache(apps: &HashMap<String, SocketAddr>) -> Result<()> {
    write(cache_path(), to_string(apps)?)?;
    Ok(())
}

/// Browses mDNS until the deadline. Cuts the wait short when something is
/// found: waits a little longer after the first hit to catch the others,
/// then returns.
async fn discover() -> Result<HashMap<String, SocketAddr>> {
    let mdns = ServiceDaemon::new()?;
    let events = mdns.browse(SERVICE_TYPE)?;

    let mut apps = HashMap::new();

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut cutoff = deadline;

    loop {
        let until = deadline.min(cutoff);

        let Ok(Ok(event)) = timeout_at(until, events.recv_async()).await else {
            break;
        };

        let ServiceEvent::ServiceResolved(service) = event else {
            continue;
        };

        let Some(app_id) = service.txt_properties.get_property_val_str("app_id") else {
            continue;
        };

        let ip = service
            .addresses
            .iter()
            .map(ScopedIp::to_ip_addr)
            .find(IpAddr::is_ipv4)
            .or_else(|| service.addresses.iter().next().map(ScopedIp::to_ip_addr));

        let Some(ip) = ip else {
            continue;
        };

        apps.insert(app_id.to_string(), SocketAddr::new(ip, service.port));
        cutoff = Instant::now() + Duration::from_millis(500);
    }

    Ok(apps)
}

/// An iOS build relinks the app bundle every time while happily reusing a
/// stale `libdemo.a`, so a fresh looking bundle can run code from an hour
/// ago. The only honest answer comes from the running app itself, and the only
/// useful form of it is a verdict against what is on disk here.
async fn build_time(client: &Client) -> Result<()> {
    let AppCommand::BuildTime(built) = send(client, InspectorCommand::GetBuildTime).await? else {
        bail!("Unexpected response to build-time");
    };
    let AppCommand::StartTime(started) = send(client, InspectorCommand::GetStartTime).await? else {
        bail!("Unexpected response to start-time");
    };

    println!("app code built: {built}");
    println!("app started:    {started}");

    let Some((newest, path)) = newest_source(&current_dir()?)? else {
        println!("No source files here, cannot tell whether the app is current");
        return Ok(());
    };

    println!("newest source:  {newest}  {}", path.display());

    match freshness(built, started, newest) {
        Freshness::Current => println!("verdict: app is up to date"),
        Freshness::ChangedSinceStart { seconds } => {
            let minutes = seconds / 60;
            bail!(
                "App is stale. Source changed {minutes} minutes after this app started. Rebuild and reinstall before testing anything against it."
            );
        }
        Freshness::EngineOlder { seconds } => {
            let minutes = seconds / 60;
            bail!(
                "Cannot prove the app is current. Source is {minutes} minutes newer than the engine build, but predates this app launch. This can be a current app-only rebuild or a stale reused Rust library. Verify app-only evidence before testing against it."
            );
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
enum Freshness {
    Current,
    ChangedSinceStart { seconds: u64 },
    EngineOlder { seconds: u64 },
}

fn freshness(built: u64, started: u64, newest: u64) -> Freshness {
    if newest > started {
        return Freshness::ChangedSinceStart {
            seconds: newest - started,
        };
    }
    if newest > built {
        return Freshness::EngineOlder {
            seconds: newest - built,
        };
    }
    Freshness::Current
}

/// Newest mtime among the files that end up compiled in, as unix seconds.
fn newest_source(dir: &Path) -> Result<Option<(u64, PathBuf)>> {
    let mut newest: Option<(u64, PathBuf)> = None;

    for entry in read_dir(dir)? {
        let path = entry?.path();

        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        // `target` holds build output, and its mtimes are always newer than the
        // sources they came from, which would make every app look stale.
        // `inspector` and `hilen-inspect` are host side clients that never link
        // into an app, so editing them cannot make one stale either. A check
        // that cries wolf is a check nobody runs.
        if name.starts_with('.')
            || name == "target"
            || name == "build"
            || name == "inspector"
            || name == "hilen-inspect"
        {
            continue;
        }

        if path.is_dir() {
            if let Some(found) = newest_source(&path)?
                && newest.as_ref().is_none_or(|(time, _)| found.0 > *time)
            {
                newest = Some(found);
            }
            continue;
        }

        let ext = path.extension().unwrap_or_default().to_string_lossy().to_string();

        if !matches!(ext.as_str(), "rs" | "wgsl" | "toml") {
            continue;
        }

        let modified = path.metadata()?.modified()?.duration_since(UNIX_EPOCH)?.as_secs();

        if newest.as_ref().is_none_or(|(time, _)| modified > *time) {
            newest = Some((modified, path));
        }
    }

    Ok(newest)
}

fn resolve(apps: &HashMap<String, SocketAddr>, app: Option<String>) -> Result<SocketAddr> {
    let ids = || apps.keys().cloned().collect::<Vec<_>>().join(", ");

    if let Some(id) = app {
        return match apps.get(&id) {
            Some(addr) => Ok(*addr),
            None => bail!("App {id} not found. Running apps: {}", ids()),
        };
    }

    match apps.len() {
        0 => bail!(NO_APPS),
        1 => Ok(*apps.values().next().unwrap()),
        _ => bail!("Multiple apps running, pass --app. Running apps: {}", ids()),
    }
}

#[cfg(test)]
mod tests {
    use super::{Freshness, freshness};

    #[test]
    fn current_when_engine_was_built_after_source() {
        assert_eq!(freshness(200, 300, 100), Freshness::Current);
    }

    #[test]
    fn stale_when_source_changed_after_app_started() {
        assert_eq!(
            freshness(100, 200, 230),
            Freshness::ChangedSinceStart { seconds: 30 }
        );
    }

    #[test]
    fn ambiguous_when_app_started_after_source_but_engine_is_older() {
        assert_eq!(freshness(100, 300, 220), Freshness::EngineOlder { seconds: 120 });
    }
}
