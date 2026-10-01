//! The disk cache of downloaded images. A file is named by the hash of its
//! url and holds the bytes the server sent. Next to it a `.meta` file keeps
//! the `ETag` and the time the server was last asked. A size bound deletes
//! the files used longest ago, and a recheck age asks the server about an
//! old file again with a conditional request.

use std::{
    fs::{File, create_dir_all, read, read_dir, read_to_string, remove_file, write},
    io,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result};
use log::{debug, warn};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::deps::refs::manage::{Fetched, fetch_bytes_with, fetch_if_changed};

const META: &str = "meta";

#[derive(Clone, Default)]
pub(super) struct DiskCache {
    /// None for no disk cache.
    pub dir:         Option<PathBuf>,
    /// Bytes the folder may hold, 0 for no bound.
    pub limit:       u64,
    /// How old the last answer of the server may be before a file is asked
    /// about again, none for never.
    pub recheck_age: Option<Duration>,
}

#[derive(Default, Serialize, Deserialize)]
struct Meta {
    etag:    Option<String>,
    /// Unix seconds of the last answer of the server about this file.
    checked: u64,
}

impl DiskCache {
    /// The bytes of a url, from the cache folder when it has them and they
    /// are not due for a recheck.
    pub(super) async fn bytes_of(&self, url: &str, headers: &[(&str, &str)]) -> Result<Vec<u8>> {
        let Some(dir) = &self.dir else {
            return fetch_bytes_with(url, headers).await;
        };
        let file = dir.join(hex::encode(Sha256::digest(url.as_bytes())));
        let cached = read(&file).ok();
        let meta = read_meta(&file);

        if let Some(bytes) = cached.as_ref().filter(|_| !self.is_due(&meta, now())) {
            touch(&file);
            return Ok(bytes.clone());
        }

        // An etag with no bytes on disk must not be sent, a 304 would leave
        // nothing to show.
        let etag = meta.etag.as_deref().filter(|_| cached.is_some());
        match fetch_if_changed(url, headers, etag).await {
            Ok(Fetched::Fresh { bytes, etag }) => {
                self.store(dir, &file, &bytes, etag);
                Ok(bytes)
            }
            Ok(Fetched::NotModified) => {
                let bytes = cached.context("the server answered not modified to a request with no etag")?;
                write_meta(
                    &file,
                    &Meta {
                        etag:    meta.etag,
                        checked: now(),
                    },
                );
                touch(&file);
                Ok(bytes)
            }
            Err(err) => {
                // The server cannot be asked, the old picture is better
                // than none.
                let bytes = cached.ok_or(err.context("not in the image cache either"))?;
                warn!(
                    "image cache: {} not rechecked, the old file is used",
                    file.display()
                );
                touch(&file);
                Ok(bytes)
            }
        }
    }

    fn is_due(&self, meta: &Meta, now: u64) -> bool {
        self.recheck_age
            .is_some_and(|age| now.saturating_sub(meta.checked) >= age.as_secs())
    }

    fn store(&self, dir: &Path, file: &Path, bytes: &[u8], etag: Option<String>) {
        let stored = create_dir_all(dir).and_then(|()| write(file, bytes));
        if let Err(err) = stored {
            warn!("image cache: {} not written, {err}", file.display());
            return;
        }
        write_meta(file, &Meta { etag, checked: now() });
        self.prune(file);
    }

    /// Deletes the files used longest ago until the folder fits the bound.
    /// `keep` is the file in use right now, it stays whatever its age.
    pub(super) fn prune(&self, keep: &Path) {
        let Some(dir) = &self.dir else {
            return;
        };
        if self.limit == 0 {
            return;
        }
        match prune(dir, self.limit, keep) {
            Ok(0) => {}
            Ok(removed) => debug!("image cache: {removed} files removed to fit {} bytes", self.limit),
            Err(err) => warn!("image cache: {} not trimmed, {err}", dir.display()),
        }
    }
}

fn prune(dir: &Path, limit: u64, keep: &Path) -> io::Result<usize> {
    let mut files = Vec::new();
    for entry in read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let data = entry.metadata()?;
        if !data.is_file() || path.extension().is_some_and(|extension| extension == META) {
            continue;
        }
        let meta_size = meta_path(&path).metadata().map_or(0, |meta| meta.len());
        files.push((data.modified()?, data.len() + meta_size, path));
    }

    let mut total: u64 = files.iter().map(|(_, size, _)| size).sum();
    files.sort_by_key(|(used, _, _)| *used);

    let mut removed = 0;
    for (_, size, path) in files {
        if total <= limit {
            break;
        }
        if path == keep {
            continue;
        }
        remove_file(&path)?;
        let meta = meta_path(&path);
        if meta.exists() {
            remove_file(meta)?;
        }
        total -= size;
        removed += 1;
    }
    Ok(removed)
}

fn meta_path(file: &Path) -> PathBuf {
    file.with_extension(META)
}

/// The meta of a file. A file from before the meta existed, or with a
/// broken one, reads as never checked and with no etag.
fn read_meta(file: &Path) -> Meta {
    read_to_string(meta_path(file))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn write_meta(file: &Path, meta: &Meta) {
    let written = serde_json::to_string(meta)
        .map_err(io::Error::other)
        .and_then(|text| write(meta_path(file), text));
    if let Err(err) = written {
        warn!("image cache: the meta of {} not written, {err}", file.display());
    }
}

/// Marks the file as used now, the size bound deletes by this time.
fn touch(file: &Path) {
    let touched = File::options()
        .write(true)
        .open(file)
        .and_then(|file| file.set_modified(SystemTime::now()));
    if let Err(err) = touched {
        warn!("image cache: the use of {} not recorded, {err}", file.display());
    }
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs())
}

#[cfg(test)]
mod test {
    use std::{
        env::temp_dir,
        fs::{File, create_dir_all, read_dir, remove_dir_all, write},
        io::{BufRead, BufReader, Write},
        net::{TcpListener, TcpStream},
        path::{Path, PathBuf},
        process::id,
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        thread::spawn,
        time::{Duration, SystemTime},
    };

    use anyhow::Result;
    use sha2::{Digest, Sha256};

    use super::{DiskCache, prune};

    /// A picture server with an etag. It counts the full answers and the
    /// not modified answers, and `changed` swaps the picture for a new one.
    #[derive(Default)]
    struct Server {
        full:         AtomicUsize,
        not_modified: AtomicUsize,
        changed:      AtomicBool,
    }

    impl Server {
        fn start() -> (Arc<Self>, String) {
            let server = Arc::new(Self::default());
            let listener = TcpListener::bind("127.0.0.1:0").expect("a local port is free");
            let port = listener.local_addr().expect("a bound listener has an address").port();
            let for_thread = Arc::clone(&server);
            spawn(move || {
                for stream in listener.incoming().flatten() {
                    if let Err(err) = for_thread.respond(stream) {
                        eprintln!("the test server stopped a response: {err}");
                    }
                }
            });
            (server, format!("http://127.0.0.1:{port}/poster"))
        }

        fn respond(&self, mut stream: TcpStream) -> Result<()> {
            let mut reader = BufReader::new(stream.try_clone()?);
            let mut head = Vec::new();
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line)? == 0 || line == "\r\n" {
                    break;
                }
                head.push(line.trim().to_lowercase());
            }

            let (etag, body) = if self.changed.load(Ordering::Relaxed) {
                ("\"v2\"", "second picture")
            } else {
                ("\"v1\"", "first picture")
            };

            if head.contains(&format!("if-none-match: {etag}")) {
                self.not_modified.fetch_add(1, Ordering::Relaxed);
                write!(
                    stream,
                    "HTTP/1.1 304 Not Modified\r\nETag: {etag}\r\nConnection: close\r\n\r\n"
                )?;
            } else {
                self.full.fetch_add(1, Ordering::Relaxed);
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nETag: {etag}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )?;
            }
            Ok(())
        }

        fn counts(&self) -> (usize, usize) {
            (
                self.full.load(Ordering::Relaxed),
                self.not_modified.load(Ordering::Relaxed),
            )
        }
    }

    /// A fresh cache folder per test, removed when the test ends.
    struct Folder(PathBuf);

    impl Folder {
        fn new(test: &str) -> Self {
            let dir = temp_dir().join(format!("hilen-disk-cache-{test}-{}", id()));
            create_dir_all(&dir).expect("the temp folder takes a folder");
            Self(dir)
        }

        fn cache(&self, recheck_age: Option<Duration>) -> DiskCache {
            DiskCache {
                dir: Some(self.0.clone()),
                limit: 0,
                recheck_age,
            }
        }
    }

    impl Drop for Folder {
        fn drop(&mut self) {
            if let Err(err) = remove_dir_all(&self.0) {
                eprintln!("{} not removed: {err}", self.0.display());
            }
        }
    }

    #[tokio::test]
    async fn with_no_recheck_age_a_cached_file_makes_no_request() -> Result<()> {
        let (server, url) = Server::start();
        let folder = Folder::new("no-age");
        let cache = folder.cache(None);

        assert_eq!(cache.bytes_of(&url, &[]).await?, b"first picture");
        server.changed.store(true, Ordering::Relaxed);
        assert_eq!(cache.bytes_of(&url, &[]).await?, b"first picture");

        assert_eq!(server.counts(), (1, 0), "the second read never asked the server");
        Ok(())
    }

    #[tokio::test]
    async fn an_old_file_is_asked_about_and_kept_when_the_picture_is_the_same() -> Result<()> {
        let (server, url) = Server::start();
        let folder = Folder::new("same");
        let cache = folder.cache(Some(Duration::ZERO));

        assert_eq!(cache.bytes_of(&url, &[]).await?, b"first picture");
        assert_eq!(cache.bytes_of(&url, &[]).await?, b"first picture");

        assert_eq!(
            server.counts(),
            (1, 1),
            "the recheck sent the etag and got no body"
        );
        Ok(())
    }

    #[tokio::test]
    async fn a_file_younger_than_the_age_is_not_asked_about() -> Result<()> {
        let (server, url) = Server::start();
        let folder = Folder::new("young");
        let cache = folder.cache(Some(Duration::from_secs(3600)));

        cache.bytes_of(&url, &[]).await?;
        cache.bytes_of(&url, &[]).await?;

        assert_eq!(server.counts(), (1, 0));
        Ok(())
    }

    #[tokio::test]
    async fn a_changed_picture_replaces_the_file() -> Result<()> {
        let (server, url) = Server::start();
        let folder = Folder::new("changed");
        let cache = folder.cache(Some(Duration::ZERO));

        assert_eq!(cache.bytes_of(&url, &[]).await?, b"first picture");
        server.changed.store(true, Ordering::Relaxed);
        assert_eq!(cache.bytes_of(&url, &[]).await?, b"second picture");
        // The new file carries the new etag, so the next recheck is cheap.
        assert_eq!(cache.bytes_of(&url, &[]).await?, b"second picture");

        assert_eq!(server.counts(), (2, 1));
        Ok(())
    }

    #[tokio::test]
    async fn a_server_that_is_gone_leaves_the_cached_file_in_use() -> Result<()> {
        let folder = Folder::new("gone");
        let cache = folder.cache(Some(Duration::ZERO));

        // Nobody listens on this url. The hash of the url names the file,
        // so the cache is filled by hand.
        let dead = "http://127.0.0.1:1/poster";
        write(
            folder.0.join(hex::encode(Sha256::digest(dead.as_bytes()))),
            b"kept picture",
        )?;

        assert_eq!(cache.bytes_of(dead, &[]).await?, b"kept picture");
        Ok(())
    }

    #[tokio::test]
    async fn a_server_that_is_gone_and_no_cached_file_is_an_error() {
        let folder = Folder::new("nothing");
        let cache = folder.cache(None);

        assert!(cache.bytes_of("http://127.0.0.1:1/poster", &[]).await.is_err());
    }

    fn file(dir: &Path, name: &str, size: usize, age_secs: u64) -> Result<PathBuf> {
        let path = dir.join(name);
        write(&path, vec![0; size])?;
        File::options()
            .write(true)
            .open(&path)?
            .set_modified(SystemTime::now() - Duration::from_secs(age_secs))?;
        Ok(path)
    }

    fn names(dir: &Path) -> Result<Vec<String>> {
        let mut names: Vec<String> = read_dir(dir)?
            .map(|entry| Ok(entry?.file_name().to_string_lossy().into_owned()))
            .collect::<Result<_>>()?;
        names.sort();
        Ok(names)
    }

    #[test]
    fn the_bound_deletes_the_files_used_longest_ago_with_their_meta() -> Result<()> {
        let folder = Folder::new("bound");
        let old = file(&folder.0, "old", 100, 300)?;
        write(old.with_extension("meta"), "{}")?;
        file(&folder.0, "middle", 100, 200)?;
        let new = file(&folder.0, "new", 100, 100)?;

        // Room for 2 files of 100 bytes.
        let removed = prune(&folder.0, 250, &new)?;

        assert_eq!(removed, 1);
        assert_eq!(names(&folder.0)?, ["middle", "new"]);
        Ok(())
    }

    #[test]
    fn the_file_in_use_stays_even_when_it_is_the_oldest() -> Result<()> {
        let folder = Folder::new("keep");
        let in_use = file(&folder.0, "in-use", 100, 300)?;
        file(&folder.0, "other", 100, 100)?;

        prune(&folder.0, 150, &in_use)?;

        assert_eq!(names(&folder.0)?, ["in-use"]);
        Ok(())
    }
}
