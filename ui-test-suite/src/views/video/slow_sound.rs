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

/// The picture and the 2 sound tracks of `tracks.mkv`, four seconds, the
/// picture small, in an mkv with its index at the front. A 440 Hz tone at
/// stream 1 and a 1760 Hz tone at stream 2. The sound is the clock.
const VIDEO: &[u8] = include_bytes!("slow_sound.mkv");

const SECOND_SOUND: usize = 2;

/// The byte the packets at 2.5 seconds start on. While the gate is shut
/// the server sends nothing from here on.
const STALL_AT: usize = 50000;

/// Lets the part of the file after `STALL_AT` through.
static GATE: AtomicBool = AtomicBool::new(false);

/// While set, a new request gets no answer at all, so opening the source
/// waits on the network.
static HOLD: AtomicBool = AtomicBool::new(false);

/// How long the server keeps a call waiting before the test gives up on
/// it and lets the network go on.
const GIVE_UP: Duration = Duration::from_secs(2);

/// The longest a call may keep the main thread. Each of them is a few
/// lines of bookkeeping once the network work is on another thread.
const HELD_FRAME: Duration = Duration::from_millis(300);

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
                            // The player drops a connection it no longer
                            // needs, a write after that fails and is fine.
                            debug!("the test video server stopped a response: {err}");
                        }
                    });
                }
                Err(err) => error!("the test video server got no connection: {err}"),
            }
        }
    });
    format!("http://127.0.0.1:{port}/slow_sound.mkv")
}

fn wait_while(flag: &AtomicBool, set: bool) {
    while flag.load(Ordering::Relaxed) == set {
        sleep(Duration::from_millis(5));
    }
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

    wait_while(&HOLD, true);

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
    wait_while(&GATE, false);
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

/// Runs `call` on the main thread while the network is shut by `flag`, and
/// returns how long the main thread was kept. The network opens again
/// after the call, or after `GIVE_UP` when the call itself waits for it.
fn held(flag: &'static AtomicBool, shut: bool, call: impl FnOnce() + Send + 'static) -> Duration {
    flag.store(shut, Ordering::Relaxed);
    let give_up = spawn(move || {
        let deadline = Instant::now() + GIVE_UP;
        while flag.load(Ordering::Relaxed) == shut && Instant::now() < deadline {
            sleep(Duration::from_millis(5));
        }
        flag.store(!shut, Ordering::Relaxed);
    });
    let started = Instant::now();
    from_main(call);
    let kept = started.elapsed();
    flag.store(!shut, Ordering::Relaxed);
    if give_up.join().is_err() {
        error!("the thread that opens the network again died");
    }
    kept
}

/// Plays the fixture from a local http server that can stop answering, and
/// measures how long the calls that open or start the sound keep the main
/// thread while it does. Proves none of them waits for the network: the
/// first play after a seek, a speed change, a sound track switch and a seek
/// after the end. Each used to read the source on the main thread, and a
/// slow network held the frame loop that long. Real time.
#[view]
struct VideoSlowSound {
    #[init]
    title: Label,
    video: VideoView,
    state: Label,
}

impl Setup for VideoSlowSound {
    fn setup(self: Weak<Self>) {
        GATE.store(false, Ordering::Relaxed);
        HOLD.store(false, Ordering::Relaxed);

        self.title.set_frame((20, 40, 560, 40)).set_text_size(24);
        self.title.set_text("the sound opens while the network is shut");
        self.state.set_frame((20, 470, 560, 40)).set_text_size(24);
        self.state.set_text("state: Empty");
        self.video.set_frame((20, 100, 560, 350));

        self.video.on_state.val(move |state| {
            self.state.set_text(format!("state: {state:?}"));
        });
        self.video.set_mode(ImageMode::Fill).set_volume(0.0).set_source(serve());
    }
}

impl ViewTest for VideoSlowSound {
    fn perform_test(view: Weak<Self>) -> Result<()> {
        wait_until("the first frame", move || {
            view.video.state() == VideoState::Paused
        })?;

        // The first play starts the sound at 3 seconds, behind the shut
        // gate, so the first read of the sound waits.
        from_main(move || {
            view.video.seek_to(3.0);
        });
        let play = held(&GATE, false, move || {
            view.video.play();
        });
        wait_until("the video to play from 3 s", move || {
            view.video.state() == VideoState::Playing && view.video.position() > 3.1
        })?;
        checkpoint("playing from 3 s, the play did not wait for the network")?;

        // A new speed opens a fresh sound decoder, a new request.
        from_main(move || {
            view.video.pause();
            view.video.seek_to(0.5);
        });
        let speed = held(&HOLD, true, move || {
            view.video.set_speed(2.0);
        });

        // So does another sound track.
        let track = held(&HOLD, true, move || {
            view.video.set_audio_track(SECOND_SOUND);
        });
        // The old track stays the named one until the new one has opened on
        // its thread, and the network opened again only now.
        wait_until("the second sound track to be the chosen one", move || {
            view.video.audio_track() == Some(SECOND_SOUND)
        })?;

        from_main(move || {
            view.video.set_speed(1.0).play();
        });
        wait_until("the end of the video", move || {
            view.video.state() == VideoState::Finished
        })?;

        // The sound of a finished video is gone, a seek opens a new one.
        let seek = held(&HOLD, true, move || {
            view.video.seek_to(0.5);
        });
        from_main(move || {
            view.video.play();
        });
        wait_until("the video to play again after the end", move || {
            view.video.state() == VideoState::Playing && (0.7..3.5).contains(&view.video.position())
        })?;
        checkpoint("playing again from 0.5 s on the second sound track")?;
        wait_until("the second end of the video", move || {
            view.video.state() == VideoState::Finished
        })?;

        let kept = [
            ("the first play", play),
            ("a speed change", speed),
            ("a sound track switch", track),
            ("a seek after the end", seek),
        ];
        let slow: Vec<String> = kept
            .iter()
            .filter(|(_, kept)| *kept > HELD_FRAME)
            .map(|(what, kept)| format!("{what} kept the main thread {} ms", kept.as_millis()))
            .collect();
        ensure!(slow.is_empty(), "{}", slow.join(", "));
        Ok(())
    }
}
