//! The sending side of a hot swap on a phone, see `docs/hot-reload.md` in
//! hilen: find the loader on the network, send it a library and the assets
//! of the app, and wait until it runs the library.

use std::{
    collections::HashMap,
    fs::{read, read_dir},
    net::SocketAddr,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use hilen::inspect::protocol::{
    AppCommand, Client, FIXED_PORT, HotFileRepr, InspectorCommand, discover, file_hash, megabytes,
};
use tokio::time::{Instant, sleep, timeout};

use super::{device::Device, reach::GIVEN, send};

/// A request may be 64 MB, a piece stays well below it as base64.
const PIECE: usize = 8 * 1024 * 1024;
const CONNECT: Duration = Duration::from_secs(2);
const SWAP_WAIT: Duration = Duration::from_secs(90);
const POLL: Duration = Duration::from_millis(500);

const NO_TARGET: &str = r"A send needs the device it goes to. Pass --device with a word of its name or kind, like `--device tv`, or --addr.
`hilen-inspect devices` lists the paired devices. A send to the only hot build a search finds once went to the wrong device.";

/// What a hot command is aimed at.
pub(super) enum Target {
    /// A paired device of this Mac, `--device`.
    Device(Device),
    /// A given address, `--addr`.
    Addr(SocketAddr),
    /// Whatever hot build a search of the network finds.
    Search,
}

impl Target {
    /// Every address a hot build of an aimed target may answer at, the
    /// likely ones first. A swap starts a new build, and the new build
    /// takes the fixed port when the old one let it go, else a new port,
    /// which only a search tells. `search` adds those.
    async fn places(&self, search: bool) -> Result<Vec<SocketAddr>> {
        match self {
            Self::Device(device) => device.places(search).await,
            Self::Addr(addr) => {
                let mut places = vec![*addr];
                let mut fixed = *addr;
                fixed.set_port(FIXED_PORT);
                if !places.contains(&fixed) {
                    places.push(fixed);
                }
                if search {
                    for found in discover().await?.into_values() {
                        if found.ip() == addr.ip() && !places.contains(&found) {
                            places.push(found);
                        }
                    }
                }
                Ok(places)
            }
            Self::Search => Ok(vec![]),
        }
    }

    fn name(&self) -> String {
        match self {
            Self::Device(device) => device.line(),
            Self::Addr(addr) => addr.to_string(),
            Self::Search => "the network".to_string(),
        }
    }
}

/// A send with no device, no address and no app id is refused.
fn check_aimed(app: Option<&str>, target: &Target) -> Result<()> {
    if app.is_none() && matches!(target, Target::Search) {
        bail!(NO_TARGET);
    }
    Ok(())
}

/// A hot build on the network.
struct Loader {
    id:      String,
    addr:    SocketAddr,
    client:  Client,
    /// The library it runs, none for the one packed in the loader app.
    library: Option<String>,
    /// It takes a file in raw pieces.
    raw:     bool,
}

/// The library an app runs, when the app is a hot build that answers.
async fn ask(addr: SocketAddr) -> Option<(Client, Option<String>, bool)> {
    let client = timeout(CONNECT, Client::connect(addr)).await.ok()?.ok()?;
    match timeout(CONNECT, client.send(InspectorCommand::HotInfo)).await.ok()?.ok()? {
        AppCommand::Hot { library, raw } => Some((client, library, raw)),
        _ => None,
    }
}

async fn loaders(apps: HashMap<String, SocketAddr>) -> Vec<Loader> {
    let mut found = vec![];
    for (id, addr) in apps {
        if let Some((client, library, raw)) = ask(addr).await {
            found.push(Loader {
                id,
                addr,
                client,
                library,
                raw,
            });
        }
    }
    found
}

/// The first hot build that answers at one of the `places`.
async fn first_at(places: Vec<SocketAddr>) -> Option<Loader> {
    for addr in places {
        if let Some((client, library, raw)) = ask(addr).await {
            return Some(Loader {
                id: GIVEN.to_string(),
                addr,
                client,
                library,
                raw,
            });
        }
    }
    None
}

/// The hot build to talk to: the one on the aimed device or address, the
/// app with the id `app`, or the only hot build on the network.
async fn loader(app: Option<String>, target: &Target) -> Result<Loader> {
    if !matches!(target, Target::Search) {
        for search in [false, true] {
            if let Some(loader) = first_at(target.places(search).await?).await {
                return Ok(loader);
            }
        }
        bail!(
            "No hot build answers on {}. Start the loader app on the device and keep it in front.",
            target.name()
        );
    }

    let mut found = loaders(discover().await?).await;
    if let Some(app) = app {
        found.retain(|loader| loader.id == app);
    }
    let ids: Vec<String> = found.iter().map(|loader| format!("{} at {}", loader.id, loader.addr)).collect();
    match found.len() {
        0 => bail!("No hot build found on the network. Start the loader app on the device."),
        1 => Ok(found.remove(0)),
        _ => bail!("Several hot builds run, pass --device. Found: {}", ids.join(", ")),
    }
}

pub(super) async fn status(app: Option<String>, target: &Target) -> Result<()> {
    let loader = loader(app, target).await?;
    println!("loader   {} at {}", loader.id, loader.addr);
    println!(
        "library  {}",
        loader.library.as_deref().unwrap_or("the one packed in the loader")
    );
    Ok(())
}

/// Sends `library` and the `assets` folder of the app `name` to the loader
/// and waits until the loader runs the library.
pub(super) async fn send_app(
    app: Option<String>,
    target: &Target,
    library: &Path,
    assets: Option<&Path>,
    name: &str,
) -> Result<()> {
    check_aimed(app.as_deref(), target)?;
    let loader = loader(app, target).await?;
    println!("loader {} at {}", loader.id, loader.addr);

    // The time of each step, a slow send says which one it was.
    let started = Instant::now();
    let root = match assets {
        Some(assets) => {
            let root = format!("apps/{name}");
            send_assets(&loader, assets, &root).await?;
            Some(root)
        }
        None => None,
    };
    let assets_done = Instant::now();

    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let sent_name = format!("{stamp}.dylib");
    let data = read(library).with_context(|| format!("no library at {}", library.display()))?;
    println!("sending the library, {} MB", data.len() / 1024 / 1024);
    send_file(&loader, &sent_name, &data).await?;
    let library_done = Instant::now();

    send(
        &loader.client,
        InspectorCommand::HotSwap {
            library: sent_name.clone(),
            root,
        },
    )
    .await?;

    wait_for(loader.addr, target, &sent_name).await?;
    let library_time = library_done - assets_done;
    println!(
        "times: assets {:.1} s, library {:.1} s at {:.1} MB/s, start of the app {:.1} s",
        (assets_done - started).as_secs_f64(),
        library_time.as_secs_f64(),
        megabytes(u64::try_from(data.len())?) / library_time.as_secs_f64(),
        library_done.elapsed().as_secs_f64(),
    );
    Ok(())
}

async fn send_assets(loader: &Loader, assets: &Path, root: &str) -> Result<()> {
    let client = &loader.client;
    let remote_root = format!("{root}/assets");
    let AppCommand::HotFiles(there) = send(
        client,
        InspectorCommand::HotFiles {
            root: remote_root.clone(),
        },
    )
    .await?
    else {
        bail!("Unexpected response to the list of files");
    };

    let mut files = vec![];
    walk(assets, assets, &mut files)?;

    let mut sent = 0;
    for (path, file) in &files {
        let data = read(file)?;
        let here = HotFileRepr {
            path: path.clone(),
            len:  u64::try_from(data.len())?,
            hash: file_hash(&data),
        };
        if there.contains(&here) {
            continue;
        }
        send_file(loader, &format!("{remote_root}/{path}"), &data).await?;
        sent += 1;
    }
    println!(
        "assets: {sent} of {} files sent, the rest is there already",
        files.len()
    );
    Ok(())
}

async fn send_file(loader: &Loader, file: &str, data: &[u8]) -> Result<()> {
    let client = &loader.client;
    if loader.raw {
        return client.send_file(file, data).await;
    }
    // A loader from before 2026-10-13 takes a piece only as base64.
    // An empty file has no piece, it is still made.
    let pieces: Vec<&[u8]> = if data.is_empty() {
        vec![data]
    } else {
        data.chunks(PIECE).collect()
    };
    let mut offset = 0;
    for piece in pieces {
        send(
            client,
            InspectorCommand::HotChunk {
                file: file.to_string(),
                offset,
                data_base64: STANDARD.encode(piece),
            },
        )
        .await?;
        offset += u64::try_from(piece.len())?;
    }
    Ok(())
}

/// Every file below `folder`, its path from `root` with `/` between the
/// parts. Hidden files stay out.
fn walk(root: &Path, folder: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<()> {
    for entry in read_dir(folder).with_context(|| format!("no folder {}", folder.display()))? {
        let path = entry?.path();
        if path.file_name().is_some_and(|name| name.to_string_lossy().starts_with('.')) {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, files)?;
            continue;
        }
        let parts: Vec<_> = path.strip_prefix(root)?.iter().map(|part| part.to_string_lossy()).collect();
        files.push((parts.join("/"), path));
    }
    Ok(())
}

/// The swap starts a new build, with a new id and maybe a new port. Waits
/// until a hot build of the target runs `library`. An aimed target is asked
/// at every port a build may have taken there. Asking only the old port
/// again reported a swap that worked as failed, when the new build took
/// another port. With no aim, a search finds the new build at the address
/// of the `old` one.
async fn wait_for(old: SocketAddr, target: &Target, library: &str) -> Result<()> {
    let deadline = Instant::now() + SWAP_WAIT;
    while Instant::now() < deadline {
        sleep(POLL).await;
        let found = if matches!(target, Target::Search) {
            let mut found = discover().await?;
            found.retain(|_, addr| addr.ip() == old.ip());
            found
        } else {
            let places = target.places(true).await?;
            places.into_iter().map(|addr| (addr.to_string(), addr)).collect()
        };
        for loader in loaders(found).await {
            if loader.library.as_deref() == Some(library) {
                println!("swapped, {} at {} runs {library}", loader.id, loader.addr);
                return Ok(());
            }
        }
    }
    bail!(
        "The loader did not start {library} in {} seconds",
        SWAP_WAIT.as_secs()
    )
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use hilen::inspect::protocol::FIXED_PORT;
    use tokio::runtime::Builder;

    use super::{Target, check_aimed};

    #[test]
    fn a_send_with_no_aim_is_refused() -> Result<()> {
        let refused = check_aimed(None, &Target::Search).unwrap_err().to_string();
        assert!(refused.contains("--device"), "{refused}");

        check_aimed(Some("ABCDEF"), &Target::Search)?;
        check_aimed(None, &Target::Addr("192.168.0.129:7435".parse()?))?;
        Ok(())
    }

    #[test]
    fn a_given_address_is_also_asked_at_the_fixed_port() -> Result<()> {
        Builder::new_current_thread().enable_all().build()?.block_on(async {
            let places = Target::Addr("192.168.0.129:62484".parse()?).places(false).await?;
            assert_eq!(
                places,
                [
                    "192.168.0.129:62484".parse()?,
                    format!("192.168.0.129:{FIXED_PORT}").parse()?
                ]
            );

            let fixed = Target::Addr(format!("192.168.0.129:{FIXED_PORT}").parse()?);
            assert_eq!(fixed.places(false).await?.len(), 1);
            Ok(())
        })
    }
}
