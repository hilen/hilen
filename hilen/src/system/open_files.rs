//! The files the system asks the app to open, from Open With or a double
//! click. Each platform hands a path to `receive`, and `OpenFiles::on_open`
//! gives it to the app on the main thread. The file types are in the
//! `[open_files]` table of `hilen.toml`, which the packaging writes.
//!
//! A file that comes before anybody listens is kept, the file of a cold start
//! most of all. It goes to the first view that subscribes.

use std::path::PathBuf;
#[cfg(desktop)]
use std::path::{Path, absolute};

#[cfg(desktop)]
use log::info;
use parking_lot::Mutex;

use crate::{
    deps::{hreads::on_main_later, refs::Weak},
    ui::UIEvent,
};

static ON_OPEN: UIEvent<PathBuf> = UIEvent::const_new();

/// Paths that came while nobody listened, oldest first.
static PENDING: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

pub struct OpenFiles;

impl OpenFiles {
    /// Calls `action` with every path the system asks the app to open, while
    /// `subscriber` lives, on the main thread and in a turn of the loop of its
    /// own. A path that came before goes to the first subscriber.
    pub fn on_open<U: ?Sized>(subscriber: Weak<U>, action: impl FnMut(PathBuf) + Send + 'static) {
        ON_OPEN.val(subscriber, action);
        if !PENDING.lock().is_empty() {
            on_main_later(deliver_pending);
        }
    }
}

/// Takes a path from a platform, from any thread.
#[cfg(desktop)]
pub(crate) fn receive(path: PathBuf, source: &str) {
    info!("file to open from {source}: {}", path.display());
    PENDING.lock().push(path);
    on_main_later(deliver_pending);
}

/// Windows and Linux start the app with the file as an argument, and a run
/// from a terminal does the same on a Mac.
#[cfg(desktop)]
pub(crate) fn receive_from_args() {
    for path in existing_paths(std::env::args().skip(1)) {
        receive(path, "the start arguments");
    }
}

#[cfg(desktop)]
fn existing_paths(args: impl Iterator<Item = String>) -> Vec<PathBuf> {
    args.filter(|arg| !arg.starts_with('-') && Path::new(arg).exists())
        .filter_map(|arg| absolute(arg).ok())
        .collect()
}

fn deliver_pending() {
    if !ON_OPEN.has_subscribers() {
        return;
    }
    let paths: Vec<PathBuf> = PENDING.lock().drain(..).collect();
    for path in paths {
        ON_OPEN.trigger(path);
    }
}

#[cfg(all(test, desktop))]
mod test {
    use std::env::temp_dir;

    use super::existing_paths;

    #[test]
    fn only_an_existing_path_is_a_file_to_open() {
        let folder = temp_dir().display().to_string();
        let args = ["--flag", "no/such/file.mp4", "kutreel://invite", folder.as_str()];

        let paths = existing_paths(args.iter().map(ToString::to_string));

        assert_eq!(paths.len(), 1);
        assert!(paths[0].is_absolute());
    }
}
