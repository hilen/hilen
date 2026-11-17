//! A local http server over a video fixture for the unit tests, with
//! `Range` support. It can break a connection the 2 ways the real case
//! did: an answer that promises the whole file and closes in the middle of
//! the body, and one that also sits still before it closes, like a
//! connection of a paused player.

use std::{
    fs::read,
    io::{BufRead, BufReader, Result, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{sleep, spawn},
    time::Duration,
};

use log::{debug, error};
use parking_lot::Mutex;

use crate::video::{VideoSource, test_fixture};

/// How the server treats a request.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    /// The answer promises the whole file, stops at the byte of the cut
    /// and closes the connection. This is what nginx did to a connection
    /// the paused player had not read from for a minute.
    Cut,
    /// Every connection is closed with no answer.
    Dead,
    /// The whole file.
    Whole,
}

/// Where a cut answer ends.
#[derive(Clone, Copy)]
pub(crate) struct Cut {
    /// The answer closes before this byte.
    pub at:   usize,
    /// The answer stops before this byte and waits for `Server::release`
    /// before it sends the rest up to `at`. The reader then holds only the
    /// bytes after this one when the connection closes. A demuxer that
    /// goes back to the start of the block it was in has to ask the server
    /// again, and it gets a new connection the engine knows nothing of.
    pub hold: Option<usize>,
    /// This many of the first answers that would be cut are whole, for a
    /// reader that opens the source more than once.
    pub skip: usize,
}

pub(crate) struct Server {
    pub url:  String,
    mode:     Arc<Mutex<Mode>>,
    released: Arc<AtomicBool>,
    /// Answers that were up for a cut.
    pub cuts: Arc<AtomicUsize>,
}

impl Server {
    pub(crate) fn start(fixture: &str, mode: Mode, cut: Cut) -> Self {
        let video = Arc::new(read(test_fixture(fixture).location()).expect("the fixture is there"));
        let listener = TcpListener::bind("127.0.0.1:0").expect("a local port is free");
        let port = listener.local_addr().expect("a bound listener has an address").port();
        let mode = Arc::new(Mutex::new(mode));
        let released = Arc::new(AtomicBool::new(false));
        let cuts = Arc::new(AtomicUsize::new(0));

        let (served, gate, counted) = (Arc::clone(&mode), Arc::clone(&released), Arc::clone(&cuts));
        spawn(move || {
            for stream in listener.incoming() {
                let stream = match stream {
                    Ok(stream) => stream,
                    Err(err) => {
                        error!("the test video server got no connection: {err}");
                        continue;
                    }
                };
                let mode = *served.lock();
                let (video, gate, counted) = (Arc::clone(&video), Arc::clone(&gate), Arc::clone(&counted));
                spawn(move || {
                    if let Err(err) = respond(stream, &video, mode, cut, &gate, &counted) {
                        // A player that opens the source again drops the
                        // old connection, a write to it fails.
                        debug!("the test video server stopped an answer: {err}");
                    }
                });
            }
        });

        Self {
            url: format!("http://127.0.0.1:{port}/{fixture}"),
            mode,
            released,
            cuts,
        }
    }

    pub(crate) fn source(&self) -> VideoSource {
        VideoSource::new(&self.url)
    }

    /// For the connections that come from now on.
    pub(crate) fn set(&self, mode: Mode) {
        *self.mode.lock() = mode;
    }

    /// Lets the held answers send their rest and close.
    pub(crate) fn release(&self) {
        self.released.store(true, Ordering::Relaxed);
    }
}

fn respond(
    mut stream: TcpStream,
    video: &[u8],
    mode: Mode,
    cut: Cut,
    released: &AtomicBool,
    cuts: &AtomicUsize,
) -> Result<()> {
    if mode == Mode::Dead {
        return Ok(());
    }
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" {
            break;
        }
        head.push_str(&line.to_lowercase());
    }
    let total = video.len();
    let start = head
        .lines()
        .find_map(|line| line.strip_prefix("range: bytes="))
        .and_then(|range| range.split('-').next())
        .and_then(|from| from.trim().parse::<usize>().ok())
        .unwrap_or(0)
        .min(total);

    write!(
        stream,
        "HTTP/1.1 206 Partial Content\r\nContent-Type: video/x-matroska\r\nAccept-Ranges: \
         bytes\r\nContent-Range: bytes {start}-{}/{total}\r\nContent-Length: {}\r\n\r\n",
        total - 1,
        total - start
    )?;

    // `cuts` counts the answers that were up for a cut, spared or not.
    let spared = mode == Mode::Cut && start < cut.at && cuts.fetch_add(1, Ordering::Relaxed) < cut.skip;
    if mode == Mode::Whole || start >= cut.at || spared {
        stream.write_all(&video[start..])?;
        return stream.flush();
    }

    let mut sent = start;
    if let Some(hold) = cut.hold.filter(|hold| *hold > start) {
        stream.write_all(&video[start..hold])?;
        stream.flush()?;
        sent = hold;
        while !released.load(Ordering::Relaxed) {
            sleep(Duration::from_millis(5));
        }
    }
    stream.write_all(&video[sent..cut.at])?;
    stream.flush()?;
    // The stream drops here and the connection closes in the middle of the
    // body it promised.
    Ok(())
}
