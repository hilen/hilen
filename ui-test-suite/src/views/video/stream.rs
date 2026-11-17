use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{sleep, spawn},
    time::{Duration, Instant},
};

use anyhow::{Result, bail, ensure};
use hilen::{
    dispatch::from_main,
    refs::Weak,
    ui::{ImageMode, Label, Setup, VideoView, ViewFrame, ViewTest, view},
    ui_test::checkpoint,
    video::{VideoSource, VideoState},
};
use log::{debug, error};

/// Six seconds of the ffmpeg `testsrc2` pattern at 30 frames per second, with
/// moving shapes and a running timer, h264 in an mp4 with the index at the
/// front. No sound.
const VIDEO: &[u8] = include_bytes!("stream.mp4");

/// The byte the frame at 3 seconds starts on. The server holds everything
/// from here on until the gate opens, so the stream stalls before that frame.
const STALL_AT: usize = 53966;

const HEADER: &str = "X-Hilen-Token";
const TOKEN: &str = "secret-of-the-test";

/// Lets the held part of the file through.
static GATE: AtomicBool = AtomicBool::new(false);
/// Every request the server got, head lines joined, lower case.
static REQUESTS: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Every state `on_state` reported, in order.
static STATES: Mutex<Vec<VideoState>> = Mutex::new(Vec::new());

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
                            // The player closes a connection it no longer
                            // needs, like the one that looked for a sound
                            // track. A write after that fails and is fine.
                            debug!("the test video server stopped a response: {err}");
                        }
                    });
                }
                Err(err) => error!("the test video server got no connection: {err}"),
            }
        }
    });
    format!("http://127.0.0.1:{port}/stream.mp4")
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
    REQUESTS.lock().expect("the request log is not poisoned").push(head);

    let total = VIDEO.len();
    write!(
        stream,
        "HTTP/1.1 206 Partial Content\r\nContent-Type: video/mp4\r\nAccept-Ranges: bytes\r\nContent-Range: \
         bytes {start}-{}/{total}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
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

/// Plays the fixture from a local http server that asks for a token header
/// and stalls the stream at 3 seconds. Proves the request carries the header
/// of the `VideoSource`, the state goes loading, paused, playing, buffering
/// when the stream stalls, the position holds while it buffers, and playback
/// goes on to the end once the stream comes back. Real time, the stall is a
/// real network read that waits.
#[view]
struct VideoStream {
    #[init]
    title: Label,
    video: VideoView,
    state: Label,
}

impl Setup for VideoStream {
    fn setup(self: Weak<Self>) {
        GATE.store(false, Ordering::Relaxed);
        REQUESTS.lock().expect("the request log is not poisoned").clear();
        STATES.lock().expect("the state log is not poisoned").clear();

        self.title.set_frame((20, 40, 560, 40));
        self.title.set_text("video over http, the server stalls at 3 s");
        self.state.set_frame((20, 470, 560, 40));
        self.state.set_text("state: Empty");
        self.video.set_frame((20, 100, 560, 350));

        self.video.on_state.val(move |state| {
            self.state.set_text(format!("state: {state:?}"));
            STATES.lock().expect("the state log is not poisoned").push(state);
        });
        self.video
            .set_mode(ImageMode::Fill)
            .set_source(VideoSource::new(serve()).header(HEADER, TOKEN));
    }
}

impl ViewTest for VideoStream {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_until("the first frame", move || {
            view.video.state() == VideoState::Paused
        })?;
        checkpoint("the first frame shows, state Paused")?;

        from_main(move || {
            view.video.play();
        });
        wait_until("the stream to stall", move || {
            view.video.state() == VideoState::Buffering
        })?;
        let held = from_main(move || view.video.position());
        ensure!(
            held > 1.0 && held < 3.5,
            "the stall is inside the part the server let through, got {held}"
        );
        sleep(Duration::from_millis(300));
        let later = from_main(move || view.video.position());
        ensure!(
            (later - held).abs() < f64::EPSILON,
            "the clock holds while buffering, it went from {held} to {later}"
        );
        ensure!(
            from_main(move || view.video.is_playing()),
            "a buffering video still counts as playing"
        );
        checkpoint("the stream stalled, state Buffering, the picture holds")?;

        GATE.store(true, Ordering::Relaxed);
        wait_until("the end of the video", move || {
            view.video.state() == VideoState::Finished
        })?;
        checkpoint("the stream came back and played to the end, state Finished")?;

        let states = STATES.lock().expect("the state log is not poisoned").clone();
        ensure!(
            states
                == [
                    VideoState::Loading,
                    VideoState::Paused,
                    VideoState::Playing,
                    VideoState::Buffering,
                    VideoState::Playing,
                    VideoState::Finished,
                ],
            "on_state reported {states:?}"
        );

        let requests = REQUESTS.lock().expect("the request log is not poisoned").clone();
        ensure!(!requests.is_empty(), "the server got no request");
        let wanted = format!("{}: {TOKEN}", HEADER.to_lowercase());
        for request in &requests {
            ensure!(
                request.lines().any(|line| line.trim() == wanted),
                "a request came without the token header: {request}"
            );
        }
        Ok(())
    }
}
