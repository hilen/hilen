//! The one place that opens a source through ffmpeg. The picture and the
//! sound each keep their own demuxer, both come through here so they ask
//! the server the same way.

use std::{
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use ffmpeg_next::{
    Dictionary, Error,
    format::{self, context::Input},
};

use crate::video::VideoSource;

/// Read times count from here, in milliseconds. 0 means not in a read.
static EPOCH: LazyLock<Instant> = LazyLock::new(Instant::now);

fn now_ms() -> u64 {
    u64::try_from(EPOCH.elapsed().as_millis()).unwrap_or(u64::MAX).max(1)
}

/// What ends a read of one demuxer that waits on the network. Every demuxer
/// has its own, with the stop flag of the player shared between them.
#[derive(Clone)]
pub(crate) struct Interrupt {
    /// Set when the player drops.
    stop:          Arc<AtomicBool>,
    /// Set by the player when a seek must not wait behind a stalled read.
    broken:        Arc<AtomicBool>,
    /// When the demuxer went into the read it is in, 0 outside a read.
    reading_since: Arc<AtomicU64>,
}

/// Marks a read of a demuxer, from `Interrupt::reading` until it drops.
pub(crate) struct Reading(Arc<AtomicU64>);

impl Drop for Reading {
    fn drop(&mut self) {
        self.0.store(0, Ordering::Relaxed);
    }
}

impl Interrupt {
    pub(crate) fn new(stop: &Arc<AtomicBool>) -> Self {
        Self {
            stop:          Arc::clone(stop),
            broken:        Arc::default(),
            reading_since: Arc::default(),
        }
    }

    /// One for another demuxer of the same player.
    pub(crate) fn fresh(&self) -> Self {
        Self::new(&self.stop)
    }

    pub(crate) fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    /// Ends the read the demuxer is in. The read comes back with an error,
    /// which demuxer decides, some report the end of the file.
    pub(crate) fn break_read(&self) {
        self.broken.store(true, Ordering::Relaxed);
    }

    pub(crate) fn is_broken(&self) -> bool {
        self.broken.load(Ordering::Relaxed)
    }

    /// The demuxer thread took the break, reads go through again.
    pub(crate) fn clear(&self) {
        self.broken.store(false, Ordering::Relaxed);
    }

    /// Call it right before a read and drop it right after.
    pub(crate) fn reading(&self) -> Reading {
        self.reading_since.store(now_ms(), Ordering::Relaxed);
        Reading(Arc::clone(&self.reading_since))
    }

    /// How long the demuxer has been inside the read it is in, zero when
    /// it is not reading.
    pub(crate) fn read_time(&self) -> Duration {
        match self.reading_since.load(Ordering::Relaxed) {
            0 => Duration::ZERO,
            since => Duration::from_millis(now_ms().saturating_sub(since)),
        }
    }
}

impl VideoSource {
    /// Opens a demuxer over the source. A read gives up once the player is
    /// gone or a seek breaks it, so a stalled network stream holds neither
    /// its thread nor a seek.
    pub(crate) fn open(&self, interrupt: &Interrupt) -> Result<Input, Error> {
        crate::video::init();

        let interrupt = interrupt.clone();
        let interrupt = move || interrupt.stopped() || interrupt.is_broken();
        if self.headers.is_empty() {
            return format::input_with_interrupt(&self.location, interrupt);
        }

        let mut lines = String::new();
        for (name, value) in &self.headers {
            lines.push_str(name);
            lines.push_str(": ");
            lines.push_str(value);
            lines.push_str("\r\n");
        }
        let mut options = Dictionary::new();
        options.set("headers", &lines);
        format::input_with_interrupt_and_dictionary(&self.location, interrupt, options)
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, atomic::AtomicBool};

    use ffmpeg_next::{Error, decoder::find_by_name};

    use crate::video::{VideoSource, source::Interrupt};

    /// The prebuilt archive must carry the software AV1 decoder and zlib.
    /// The png decoder is the witness for zlib, ffmpeg builds it only with
    /// zlib in.
    #[test]
    fn dav1d_and_zlib_are_linked() {
        crate::video::init();
        assert!(
            find_by_name("libdav1d").is_some(),
            "the ffmpeg archive has no dav1d"
        );
        assert!(find_by_name("png").is_some(), "the ffmpeg archive has no zlib");
    }

    /// The prebuilt archive must carry TLS. Nothing listens on the port, so
    /// the open fails either way, with a connect error when the https protocol
    /// is linked and with `ProtocolNotFound` when it is not.
    #[test]
    fn https_protocol_is_linked() {
        let stop = Arc::new(AtomicBool::new(false));
        let Err(err) = VideoSource::new("https://127.0.0.1:9/video.mp4").open(&Interrupt::new(&stop)) else {
            panic!("nothing serves this url");
        };
        assert!(
            !matches!(err, Error::ProtocolNotFound),
            "the ffmpeg archive has no https protocol"
        );
    }
}
