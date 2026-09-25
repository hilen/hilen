mod commands;

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
use clap::Parser;
use commands::{Command, run};
use hilen::{
    inspect::protocol::{
        AppCommand, Client, InspectorCommand, Key, SERVICE_TYPE, UIRequest, UIResponse, ui::ViewRepr,
    },
    refs::{Own, hreads::set_current_thread_as_main},
    ui::{ModifiersState, NamedKey},
    window::KeyCode,
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

/// The window space area a view can show in: the window cut down by every
/// ancestor that clips its subviews. `scroller` is the innermost of those
/// ancestors, the view a wheel scroll aims at to move this one into sight.
#[derive(Clone, Copy)]
struct Clip<'tree> {
    left:     f32,
    top:      f32,
    right:    f32,
    bottom:   f32,
    scroller: Option<&'tree str>,
}

impl Clip<'_> {
    fn center_y(&self) -> f32 {
        f32::midpoint(self.top, self.bottom)
    }
}

/// A view with its window space origin and effective visibility. Frames in
/// the tree are local to the parent, locating walks them down.
struct Located<'tree> {
    view:   &'tree ViewRepr,
    x:      f32,
    y:      f32,
    hidden: bool,
    clip:   Clip<'tree>,
}

impl Located<'_> {
    fn status(&self) -> &'static str {
        if self.hidden {
            return "hidden";
        }
        let size = self.view.frame.size;
        let clip = self.clip;
        if self.x + size.width <= clip.left
            || self.y + size.height <= clip.top
            || self.x >= clip.right
            || self.y >= clip.bottom
        {
            return "offscreen";
        }
        "visible"
    }

    fn line(&self) -> String {
        format!(
            "{}{}  [{}, {}] {}x{}  {}  {}",
            self.view.label,
            shortened_text(self.view),
            self.x,
            self.y,
            self.view.frame.size.width,
            self.view.frame.size.height,
            self.status(),
            self.view.id,
        )
    }
}

fn locate<'tree>(
    view: &'tree ViewRepr,
    (x, y): (f32, f32),
    hidden: bool,
    clip: Clip<'tree>,
    out: &mut Vec<Located<'tree>>,
) {
    let x = x + view.frame.origin.x;
    let y = y + view.frame.origin.y + view.content_offset;
    let hidden = hidden || view.hidden;
    out.push(Located {
        view,
        x,
        y,
        hidden,
        clip,
    });
    let inner = if view.clips {
        Clip {
            left:     clip.left.max(x),
            top:      clip.top.max(y),
            right:    clip.right.min(x + view.frame.size.width),
            bottom:   clip.bottom.min(y + view.frame.size.height),
            scroller: Some(&view.id),
        }
    } else {
        clip
    };
    for sub in &view.subviews {
        locate(sub, (x, y), hidden, inner, out);
    }
}

fn located_tree(root: &ViewRepr) -> Vec<Located<'_>> {
    let window = Clip {
        left:     0.0,
        top:      0.0,
        right:    root.frame.size.width,
        bottom:   root.frame.size.height,
        scroller: None,
    };
    let mut located = vec![];
    locate(root, (0.0, 0.0), false, window, &mut located);
    located
}

fn matches_loosely(view: &ViewRepr, query: &str) -> bool {
    let lowercase = query.to_lowercase();
    view.id == query
        || view.label.to_lowercase().contains(&lowercase)
        || view.text.as_ref().is_some_and(|text| text.to_lowercase().contains(&lowercase))
}

async fn find(client: &Client, query: &str, all: bool) -> Result<()> {
    let (_, root) = get_ui(client).await?;
    let located = located_tree(&root);

    let mut shown = 0;
    for item in &located {
        if !matches_loosely(item.view, query) {
            continue;
        }
        if !all && item.status() != "visible" {
            continue;
        }
        shown += 1;
        println!("{}", item.line());
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
        let located = located_tree(&root);

        if let Some(item) = located
            .iter()
            .find(|item| matches_loosely(item.view, query) && item.status() == "visible")
        {
            println!("{}", item.line());
            return Ok(());
        }

        if Instant::now() > deadline {
            bail!("No visible view matched {query} within {wait_seconds}s");
        }
        tokio::time::sleep(Duration::from_millis(300)).await;
    }
}

async fn drag(client: &Client, from: (f32, f32), to: (f32, f32), steps: usize) -> Result<()> {
    send(client, UIRequest::Drag { from, to, steps }.into()).await?;
    println!("ok");
    Ok(())
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

/// Repeats scroll steps until the view's center is inside the area it can
/// show in, aimed at the scroll view that clips it, or at the window
/// center when nothing clips it. Fuzzy matching, the target is often known only
/// by text.
async fn scroll_to(client: &Client, query: &str) -> Result<()> {
    for _ in 0..16 {
        let (_, root) = get_ui(client).await?;
        let located = located_tree(&root);
        // Loose matching and the first hit, an ambiguous query is fine
        // here, any of the matches leads the scroll to the same place.
        let Some(item) = located.iter().find(|item| !item.hidden && matches_loosely(item.view, query)) else {
            bail!("No view matches: {query}");
        };

        let center = item.y + item.view.frame.size.height / 2.0;
        let clip = item.clip;
        if center > clip.top && center < clip.bottom {
            println!("{}", item.line());
            return Ok(());
        }

        send(
            client,
            UIRequest::Scroll {
                view_id: clip.scroller.map(str::to_string),
                dx:      0.0,
                dy:      clip.center_y() - center,
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
    let located = located_tree(root);
    let lowercase = anchor.to_lowercase();

    // An exact id works as the anchor too, it is the way out when several
    // views carry the same text.
    let anchors: Vec<&Located> = located
        .iter()
        .filter(|item| {
            item.view.id == anchor
                || (item.status() == "visible"
                    && item.view.text.as_ref().is_some_and(|text| text.to_lowercase() == lowercase))
        })
        .collect();

    let anchor_item = match anchors.as_slice() {
        [] => bail!("No visible view has the exact text: {anchor}"),
        [only] => only,
        candidates => {
            let listed: Vec<String> = candidates.iter().map(|item| item.line()).collect();
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
            item.status() == "visible"
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

async fn hold(client: &Client, names: &[String], ms: u32) -> Result<()> {
    let keys = names.iter().map(|name| parse_key_code(name)).collect::<Result<Vec<_>>>()?;
    send(client, UIRequest::Hold { keys, ms }.into()).await?;
    println!("ok");
    Ok(())
}

/// `w` is `KeyW` and `1` is `Digit1`, the rest go through the serde
/// form of `KeyCode`, the plain variant name.
fn parse_key_code(name: &str) -> Result<KeyCode> {
    let mut chars = name.chars();
    let full = match (chars.next(), chars.next()) {
        (Some(ch), None) if ch.is_ascii_alphabetic() => format!("Key{}", ch.to_ascii_uppercase()),
        (Some(ch), None) if ch.is_ascii_digit() => format!("Digit{ch}"),
        _ => name.to_string(),
    };
    match from_value(Value::String(full)) {
        Ok(code) => Ok(code),
        Err(_) => {
            bail!("Unknown key: {name}. Use a letter, a digit or a winit KeyCode name like Space or ArrowUp")
        }
    }
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
    use hilen::{
        inspect::ViewRepr,
        refs::{Own, hreads::set_current_thread_as_main},
        window::KeyCode,
    };

    use super::{Freshness, freshness, located_tree, parse_key_code};

    fn view(
        id: &str,
        frame: (f32, f32, f32, f32),
        clips: bool,
        subviews: Vec<Own<ViewRepr>>,
    ) -> Own<ViewRepr> {
        Own::new(ViewRepr {
            id: id.into(),
            frame: frame.into(),
            clips,
            subviews,
            ..ViewRepr::default()
        })
    }

    /// The sidebar Dev item sat below the end of its scroll view and was
    /// reported visible, so a tap on it pressed the view drawn there.
    #[test]
    fn a_row_cut_off_by_its_scroll_view_is_offscreen() {
        set_current_thread_as_main();
        let rows = view(
            "rows",
            (0.0, 0.0, 200.0, 600.0),
            false,
            vec![
                view("shown", (0.0, 100.0, 200.0, 40.0), false, vec![]),
                view("cut", (0.0, 480.0, 200.0, 40.0), false, vec![]),
            ],
        );
        let scroll = view("scroll", (0.0, 70.0, 200.0, 390.0), true, vec![rows]);
        let root = view("root", (0.0, 0.0, 800.0, 600.0), false, vec![scroll]);

        let located = located_tree(&root);
        let status = |id: &str| located.iter().find(|item| item.view.id == id).unwrap().status();
        assert_eq!(status("shown"), "visible");
        assert_eq!(status("cut"), "offscreen");
        let cut = located.iter().find(|item| item.view.id == "cut").unwrap();
        assert_eq!(cut.clip.scroller, Some("scroll"));
    }

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

    #[test]
    fn key_names_map_to_physical_keys() {
        assert_eq!(parse_key_code("w").unwrap(), KeyCode::KeyW);
        assert_eq!(parse_key_code("W").unwrap(), KeyCode::KeyW);
        assert_eq!(parse_key_code("1").unwrap(), KeyCode::Digit1);
        assert_eq!(parse_key_code("Space").unwrap(), KeyCode::Space);
        assert_eq!(parse_key_code("ArrowUp").unwrap(), KeyCode::ArrowUp);
        assert!(parse_key_code("Foo").is_err());
    }
}
