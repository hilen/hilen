use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::atomic::{AtomicBool, Ordering},
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{ImageMode, Label, Setup, VideoView, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
    video::VideoState,
};
use log::{debug, error};

/// The picture and the first sound track of `tracks.mkv`, four seconds,
/// copied into an mkv with its index at the front. With the index at the
/// end a seek reads the end of the file first, and that read would wait in
/// the stall of this test too. The sound is the clock.
const VIDEO: &[u8] = include_bytes!("stalled.mkv");

/// The byte the frame at 2.5 seconds starts on. The server holds
/// everything from here on until the gate opens, so the picture and the
/// sound both stall inside a network read.
const STALL_AT: usize = 61490;

/// Lets the held part of the file through.
static GATE: AtomicBool = AtomicBool::new(false);

/// Serves the fixture over http on a free local port and returns its url.
fn serve() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a local port is free");
    let port = listener.local_addr().expect("a bound listener has an address").port();
    spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    spawn(move || {
                        if let Err(err) = respond(stream) {
                            // The player drops a connection it broke out
                            // of, a write after that fails and is fine.
                            debug!("the test video server stopped a response: {err}");
                        }
                    });
                }
                Err(err) => error!("the test video server got no connection: {err}"),
            }
        }
    });
    format!("http://127.0.0.1:{port}/stalled.mkv")
}

fn respond(mut stream: TcpStream) -> Result<()> {
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut head = String::new();
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 || line == "\r\n" {
            break;
        }
        head.push_str(&line.to_lowercase());
    }

    let start = head
        .lines()
        .find_map(|line| line.strip_prefix("range: bytes="))
        .and_then(|range| range.split('-').next())
        .and_then(|from| from.trim().parse::<usize>().ok())
        .unwrap_or(0)
        .min(VIDEO.len());

    debug!("the test video server got a request from byte {start}");

    let total = VIDEO.len();
    write!(
        stream,
        "HTTP/1.1 206 Partial Content\r\nContent-Type: video/x-matroska\r\nAccept-Ranges: \
         bytes\r\nContent-Range: bytes {start}-{}/{total}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        total - 1,
        total - start
    )?;

    let open = start.max(STALL_AT);
    if start < open {
        stream.write_all(&VIDEO[start..open])?;
        stream.flush()?;
    }
    while !GATE.load(Ordering::Relaxed) {
        sleep(Duration::from_millis(10));
    }
    stream.write_all(&VIDEO[open..])?;
    Ok(())
}

/// Polls the main thread until `done` holds.
fn wait_until(what: &str, done: impl Fn() -> bool + Send + Copy + 'static) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !from_main(done) {
        if Instant::now() > deadline {
            bail!("timed out waiting for {what}");
        }
        sleep(Duration::from_millis(10));
    }
    Ok(())
}

/// Plays the fixture from a local http server that stops sending at 2.5
/// seconds, then seeks back while the picture and the sound both wait in a
/// network read. Proves the seek does not wait for the stalled reads: the
/// video plays again from the target, and the sound, which is the clock,
/// moves on from there. Real time, the stall is a real read that waits.
#[view]
struct VideoStalledSeek {
    #[init]
    title: Label,
    video: VideoView,
    state: Label,
}

impl Setup for VideoStalledSeek {
    fn setup(self: Weak<Self>) {
        GATE.store(false, Ordering::Relaxed);

        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("a seek while the stream is stalled at 2.5 s");
        self.state.set_frame((20, 470, 560, 40));
        self.state.set_text("state: Empty");
        self.video.set_frame((20, 100, 560, 350));

        self.video.on_state.val(move |state| {
            self.state.set_text(format!("state: {state:?}"));
        });
        self.video.set_mode(ImageMode::Fill).set_volume(0.0).set_source(serve());
    }
}

impl ViewTest for VideoStalledSeek {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_until("the first frame", move || {
            view.video.state() == VideoState::Paused
        })?;

        from_main(move || {
            view.video.play();
        });
        let stalled = wait_until("the stream to stall", move || {
            view.video.state() == VideoState::Buffering
        });
        if stalled.is_err() {
            let (state, position) = from_main(move || (view.video.state(), view.video.position()));
            bail!("the stream did not stall, state {state:?} at {position:.2} s");
        }
        let held = from_main(move || view.video.position());
        ensure!(
            held > 1.0 && held < 3.0,
            "the stall is inside the part the server let through, got {held}"
        );
        checkpoint("the stream stalled, state Buffering")?;

        // The server never sends the rest on the stalled connections. The
        // seek has to break out of their reads to get anywhere.
        from_main(move || {
            view.video.seek_to(0.5);
        });
        let resumed = wait_until("the seek to get past the stalled read", move || {
            view.video.state() == VideoState::Playing && (0.7..2.4).contains(&view.video.position())
        });
        if resumed.is_err() {
            let (state, position) = from_main(move || (view.video.state(), view.video.position()));
            bail!("the seek did not get past the stalled read, state {state:?} at {position:.2} s");
        }
        checkpoint("playing again from 0.5 s, the seek did not wait for the stall")?;

        GATE.store(true, Ordering::Relaxed);
        wait_until("the end of the video", move || {
            view.video.state() == VideoState::Finished
        })?;
        checkpoint("the stream came back and played to the end, state Finished")?;
        Ok(())
    }
}
