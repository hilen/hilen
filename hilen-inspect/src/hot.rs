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
use hilen::inspect::protocol::{AppCommand, Client, HotFileRepr, InspectorCommand, file_hash};
use tokio::time::{Instant, sleep, timeout};

use super::{reach::apps, send};

/// A request may be 64 MB, a piece stays well below it as base64.
const PIECE: usize = 8 * 1024 * 1024;
const CONNECT: Duration = Duration::from_secs(2);
const SWAP_WAIT: Duration = Duration::from_secs(90);
const POLL: Duration = Duration::from_millis(500);

/// A hot build on the network.
struct Loader {
    id:      String,
    addr:    SocketAddr,
    client:  Client,
    /// The library it runs, none for the one packed in the loader app.
    library: Option<String>,
}

/// The library an app runs, when the app is a hot build that answers.
async fn ask(addr: SocketAddr) -> Option<(Client, Option<String>)> {
    let client = timeout(CONNECT, Client::connect(addr)).await.ok()?.ok()?;
    match timeout(CONNECT, client.send(InspectorCommand::HotInfo)).await.ok()?.ok()? {
        AppCommand::Hot { library } => Some((client, library)),
        _ => None,
    }
}

async fn loaders(apps: HashMap<String, SocketAddr>) -> Vec<Loader> {
    let mut found = vec![];
    for (id, addr) in apps {
        if let Some((client, library)) = ask(addr).await {
            found.push(Loader {
                id,
                addr,
                client,
                library,
            });
        }
    }
    found
}

/// The hot build to talk to: the one at the address `addr`, the app with
/// the id `app`, or the only hot build on the network.
async fn loader(app: Option<String>, addr: Option<SocketAddr>) -> Result<Loader> {
    let mut found = loaders(apps(addr).await?).await;
    if addr.is_none()
        && let Some(app) = app
    {
        found.retain(|loader| loader.id == app);
    }
    let ids: Vec<String> = found.iter().map(|loader| format!("{} at {}", loader.id, loader.addr)).collect();
    match (found.len(), addr) {
        (0, Some(addr)) => {
            bail!("No hot build answers at {addr}. Start the loader app on the phone and keep it in front.")
        }
        (0, None) => bail!("No hot build found on the network. Start the loader app on the phone."),
        (1, _) => Ok(found.remove(0)),
        _ => bail!("Several hot builds run, pass --app. Found: {}", ids.join(", ")),
    }
}

pub(super) async fn status(app: Option<String>, addr: Option<SocketAddr>) -> Result<()> {
    let loader = loader(app, addr).await?;
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
    addr: Option<SocketAddr>,
    library: &Path,
    assets: Option<&Path>,
    name: &str,
) -> Result<()> {
    let loader = loader(app, addr).await?;
    println!("loader {} at {}", loader.id, loader.addr);

    let root = match assets {
        Some(assets) => {
            let root = format!("apps/{name}");
            send_assets(&loader.client, assets, &root).await?;
            Some(root)
        }
        None => None,
    };

    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let sent_name = format!("{stamp}.dylib");
    let data = read(library).with_context(|| format!("no library at {}", library.display()))?;
    println!("sending the library, {} MB", data.len() / 1024 / 1024);
    send_file(&loader.client, &sent_name, &data).await?;

    send(
        &loader.client,
        InspectorCommand::HotSwap {
            library: sent_name.clone(),
            root,
        },
    )
    .await?;

    wait_for(loader.addr, addr, &sent_name).await
}

async fn send_assets(client: &Client, assets: &Path, root: &str) -> Result<()> {
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
        send_file(client, &format!("{remote_root}/{path}"), &data).await?;
        sent += 1;
    }
    println!(
        "assets: {sent} of {} files sent, the rest is there already",
        files.len()
    );
    Ok(())
}

async fn send_file(client: &Client, file: &str, data: &[u8]) -> Result<()> {
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

/// The swap starts a new build, with a new id. Waits until a hot build at
/// the address of the phone runs `library`. A search finds the new build on
/// whatever port it took. With a given address, `given`, there is no
/// search, the new build is asked at the same port again.
async fn wait_for(phone: SocketAddr, given: Option<SocketAddr>, library: &str) -> Result<()> {
    let deadline = Instant::now() + SWAP_WAIT;
    while Instant::now() < deadline {
        sleep(POLL).await;
        let mut found = apps(given).await?;
        found.retain(|_, addr| addr.ip() == phone.ip());
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
