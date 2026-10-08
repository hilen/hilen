//! The receiving side of a hot swap on a phone, see `docs/hot-reload.md`.
//!
//! In the simulator the script writes the next library into the hot folder,
//! which is a folder of the Mac. A phone cannot read the Mac, so the app
//! that runs takes the next library and its assets over the inspect
//! connection, writes them into the same folder and points the loader at
//! the library. The loader then swaps as it does in the simulator.

use std::{
    env::var_os,
    fs::{File, OpenOptions, create_dir_all, read, read_dir, read_to_string, remove_file, rename, write},
    io::{Seek, SeekFrom, Write},
    path::{Component, Path, PathBuf},
    thread::{sleep, spawn},
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use log::{error, info};

use crate::inspect::protocol::{AppCommand, HotFileRepr, file_hash};

/// The loader sets it, the folder with the libraries and the pointer file.
const HOT_DIR: &str = "HILEN_HOT_DIR";
const POINTER: &str = "current";
/// The loader writes the name of the library it started into this file.
const STARTED: &str = "started";
/// The swap ends this build and its connections. It waits this long, so
/// the answer to the swap command is on its way first.
const SWAP_DELAY: Duration = Duration::from_millis(200);

fn dir() -> Result<PathBuf> {
    Ok(PathBuf::from(
        var_os(HOT_DIR).context("the loader did not set the hot folder")?,
    ))
}

/// A path below the hot folder. A sender must not reach any other file.
fn below(dir: &Path, file: &str) -> Result<PathBuf> {
    let path = Path::new(file);
    ensure!(
        !file.is_empty() && path.components().all(|part| matches!(part, Component::Normal(_))),
        "`{file}` is not a path below the hot folder"
    );
    Ok(dir.join(path))
}

pub(crate) fn info() -> Result<AppCommand> {
    let started = read_to_string(dir()?.join(STARTED)).unwrap_or_default();
    let started = started.trim();
    Ok(AppCommand::Hot {
        library: (!started.is_empty()).then(|| started.to_string()),
    })
}

pub(crate) fn files(root: &str) -> Result<AppCommand> {
    let root = below(&dir()?, root)?;
    let mut found = vec![];
    if root.is_dir() {
        list(&root, &root, &mut found)?;
    }
    Ok(AppCommand::HotFiles(found))
}

fn list(root: &Path, folder: &Path, found: &mut Vec<HotFileRepr>) -> Result<()> {
    for entry in read_dir(folder)? {
        let path = entry?.path();
        if path.is_dir() {
            list(root, &path, found)?;
            continue;
        }
        let data = read(&path)?;
        let parts: Vec<_> = path.strip_prefix(root)?.iter().map(|part| part.to_string_lossy()).collect();
        found.push(HotFileRepr {
            path: parts.join("/"),
            len:  u64::try_from(data.len())?,
            hash: file_hash(&data),
        });
    }
    Ok(())
}

pub(crate) fn chunk(file: &str, offset: u64, data_base64: &str) -> Result<AppCommand> {
    let path = below(&dir()?, file)?;
    let data = STANDARD.decode(data_base64)?;

    let mut target = if offset == 0 {
        create_dir_all(path.parent().context("the file has no folder")?)?;
        File::create(&path)?
    } else {
        OpenOptions::new().write(true).open(&path)?
    };
    ensure!(
        target.metadata()?.len() == offset,
        "`{file}` got a piece out of order"
    );
    target.seek(SeekFrom::Start(offset))?;
    target.write_all(&data)?;
    Ok(AppCommand::Ok)
}

pub(crate) fn swap(library: &str, root: Option<&str>) -> Result<AppCommand> {
    let dir = dir()?;
    let path = below(&dir, library)?;
    if !path.is_file() {
        bail!("`{library}` was not sent");
    }
    let pointer = match root {
        Some(root) => format!("{library}\n{}", below(&dir, root)?.display()),
        None => library.to_string(),
    };

    drop_old_libraries(&dir, library)?;
    info!("Hot swap to {library}");

    // The pointer changes with 1 rename, the loader never reads half of it.
    spawn(move || {
        sleep(SWAP_DELAY);
        let pending = dir.join("current.new");
        if let Err(err) = write(&pending, pointer).and_then(|()| rename(&pending, dir.join(POINTER))) {
            error!("The hot swap pointer is not written: {err}");
        }
    });
    Ok(AppCommand::Ok)
}

/// A phone has little room, only the library that runs and the next one
/// stay.
fn drop_old_libraries(dir: &Path, next: &str) -> Result<()> {
    let running = read_to_string(dir.join(STARTED)).unwrap_or_default();
    for entry in read_dir(dir)? {
        let path = entry?.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();
        if path.extension().is_some_and(|extension| extension == "dylib")
            && name != next
            && name != running.trim()
        {
            remove_file(&path)?;
        }
    }
    Ok(())
}
