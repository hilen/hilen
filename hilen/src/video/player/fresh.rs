//! A connection that went dead with no error. A server, or a proxy in front
//! of it, drops a connection nobody reads from, and it can do so without a
//! close the reader sees. A read on it then gives no byte and no error for
//! as long as the system waits, half a minute in the run that found it. So
//! a play after a long pause opens fresh connections at once, and a read
//! that hangs while the video buffers is broken, which opens the stream
//! again.

use std::time::Duration;

use log::{info, warn};
use web_time::Instant;

use crate::video::{player::Player, source::Interrupt};

/// How long a read may give no byte before it is broken. A read of one
/// packet takes milliseconds on a network that works, and seconds only on
/// one too slow to play the video at all.
const HUNG_READ: Duration = Duration::from_secs(3);

/// A pause longer than this makes the next play open fresh connections.
/// nginx closes a connection nobody reads from after 60 seconds.
const STALE_AFTER: Duration = Duration::from_secs(60);

/// How long the player has not played.
pub(super) struct Idle {
    /// When the player opened or was paused, none once it played.
    since:            Option<Instant>,
    /// The pause after which the connections count as old.
    pub(super) limit: Duration,
}

impl Default for Idle {
    fn default() -> Self {
        Self {
            since: Some(Instant::now()),
            limit: STALE_AFTER,
        }
    }
}

impl Idle {
    /// The player was paused just now.
    pub(super) fn rest(&mut self) {
        self.since = Some(Instant::now());
    }
}

impl Player {
    /// Called by a play. After a long pause the connections of the picture
    /// and of the sound are not trusted: both open again, and the pictures
    /// decoded before the pause play in the meantime.
    pub(super) fn open_stale_streams(&mut self) {
        let Some(since) = self.idle.since.take() else {
            return;
        };
        let rested = since.elapsed();
        if rested < self.idle.limit || !self.source.is_network() || self.info.is_none() || self.plays_pieces()
        {
            return;
        }
        info!(
            "video {}: a play at {:.1} s after {:.0} s without one, the picture and the sound open fresh connections",
            self.source.location(),
            self.base,
            rested.as_secs_f64()
        );
        // A picture that is read to its end needs no connection.
        if !self.queue.eof {
            self.reads.break_stale();
        }
        if self.audio_track.is_some() {
            if let Some(reads) = &self.sound_reads.playing {
                // Its thread at kira ends with this.
                reads.break_read();
            }
            self.reopen_sound();
        }
    }

    /// Breaks a read that gives no byte while the video plays. The picture
    /// thread then opens the stream again, the sound gets a fresh decoder.
    pub(super) fn break_hung_reads(&mut self) {
        if !self.playing || self.failed || !self.source.is_network() {
            return;
        }
        if self.flow.buffering && !self.reads.is_broken() && self.reads.read_time() > HUNG_READ {
            warn!(
                "video {}: the read of the picture gave no byte for {:.1} s at {:.1} s, it is broken and the stream opens again",
                self.source.location(),
                self.reads.read_time().as_secs_f64(),
                self.base
            );
            self.reads.break_hung();
        }

        let hung = self
            .sound_reads
            .playing
            .as_ref()
            .map(Interrupt::read_time)
            .filter(|time| *time > HUNG_READ);
        if let Some(time) = hung {
            warn!(
                "video {}: the read of the sound gave no byte for {:.1} s at {:.1} s, the sound opens again",
                self.source.location(),
                time.as_secs_f64(),
                self.position()
            );
            self.replace_stalled_sound();
        }
    }
}
