//! A network stream that breaks while it plays. The connection of a demuxer
//! is of no use after a read that failed, so the source is opened again and
//! decoding goes on at the place it stood at. The player sees no frames in
//! that time and holds in its buffering state.

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use ffmpeg_next::Error;
use log::{error, info, warn};

use crate::video::{
    VideoSource,
    decoder::Decoding,
    source::{Interrupt, byte_position},
};

/// The wait before the second try. The first one runs at once, most breaks
/// are one closed connection and the next one works.
const FIRST_PAUSE: Duration = Duration::from_millis(250);

/// The wait doubles up to this, a server that is down is not asked more
/// often.
const LONGEST_PAUSE: Duration = Duration::from_secs(2);

/// A wait is slept in pieces of this, so a player that drops ends it soon.
const NAP: Duration = Duration::from_millis(20);

/// A break of the stream that is not mended yet.
pub(super) struct Fault {
    since: Instant,
    tries: u32,
    /// The wait before the next try.
    pause: Duration,
    /// What the last read or open failed with.
    error: Error,
}

impl Fault {
    fn new(error: Error) -> Self {
        Self {
            since: Instant::now(),
            tries: 0,
            pause: Duration::ZERO,
            error,
        }
    }

    /// Waits before the next try, no longer than `left`. False once the
    /// player is gone.
    fn wait(&mut self, reads: &Interrupt, left: Duration) -> bool {
        let until = Instant::now() + self.pause.min(left);
        self.pause = (self.pause * 2).clamp(FIRST_PAUSE, LONGEST_PAUSE);
        while Instant::now() < until {
            if reads.stopped() {
                return false;
            }
            sleep(NAP);
        }
        !reads.stopped()
    }
}

/// A packet came through, a break before it is mended.
pub(super) fn mended(fault: &mut Option<Fault>, source: &VideoSource, at: f64) {
    if let Some(fault) = fault.take() {
        info!(
            "video {}: the picture stream plays on at {at:.2} s, after {} tries in {:.1} s",
            source.location(),
            fault.tries,
            fault.since.elapsed().as_secs_f64()
        );
    }
}

impl Decoding {
    /// Opens the source again after a read, a seek or an open that failed
    /// with `error`, try after try until one opens or the limit of the
    /// source is over. `fault` lives until a packet comes through, so a
    /// stream that opens and breaks at once again does not get a fresh
    /// limit. False once the player is gone, the last error when it gives
    /// up.
    pub(super) fn reconnect(
        &mut self,
        source: &VideoSource,
        reads: &Interrupt,
        fault: &mut Option<Fault>,
        error: Error,
    ) -> Result<bool, Error> {
        let limit = source.reconnect_limit();
        let fault = if let Some(fault) = fault {
            warn!(
                "video {}: the picture stream broke again after try {}, {error}",
                source.location(),
                fault.tries
            );
            fault.error = error;
            fault
        } else {
            warn!(
                "video {}: the picture stream broke at {:.2} s, seen at byte {}, {error}, it opens again for up to {:.0} s",
                source.location(),
                self.at,
                byte_position(&self.input),
                limit.as_secs_f64()
            );
            fault.insert(Fault::new(error))
        };

        loop {
            let spent = fault.since.elapsed();
            if spent >= limit {
                error!(
                    "video {}: gave up on the picture stream at {:.2} s after {} tries in {:.1} s, {}",
                    source.location(),
                    self.at,
                    fault.tries,
                    spent.as_secs_f64(),
                    fault.error
                );
                return Err(fault.error);
            }
            if !fault.wait(reads, limit.saturating_sub(spent)) {
                return Ok(false);
            }
            let left = limit.saturating_sub(fault.since.elapsed());
            if left.is_zero() {
                continue;
            }
            fault.tries += 1;

            // A connect to a server that does not answer waits for as long
            // as the system lets it, so the open ends at the limit too.
            reads.give_up_in(left);
            let opened = self.reopen(source, reads);
            reads.never_give_up();

            match opened {
                Ok(()) => {
                    info!(
                        "video {}: try {} opened the picture stream again, decoding goes on at {:.2} s",
                        source.location(),
                        fault.tries,
                        self.at
                    );
                    return Ok(true);
                }
                Err(_) if reads.stopped() => return Ok(false),
                // The limit ended the open. The error of the try before
                // says more about the stream than this one.
                Err(Error::Exit) => warn!(
                    "video {}: try {} to open the picture stream again ran out of time",
                    source.location(),
                    fault.tries
                ),
                Err(err) => {
                    warn!(
                        "video {}: try {} to open the picture stream again failed, {err}",
                        source.location(),
                        fault.tries
                    );
                    fault.error = err;
                }
            }
        }
    }
}
