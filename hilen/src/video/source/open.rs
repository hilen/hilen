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

/// How long a network stream that broke is opened again and again before
/// the video fails.
const RECONNECT_LIMIT: Duration = Duration::from_secs(30);

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
    /// The time after which every read gives up, 0 for none.
    give_up_at:    Arc<AtomicU64>,
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
            give_up_at:    Arc::default(),
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

    /// Every read and every open ends with an error once this much time has
    /// passed. A connect to a server that does not answer waits for as long
    /// as the system lets it, far past the limit of a reconnect.
    pub(crate) fn give_up_in(&self, time: Duration) {
        let time = u64::try_from(time.as_millis()).unwrap_or(u64::MAX);
        self.give_up_at.store(now_ms().saturating_add(time), Ordering::Relaxed);
    }

    /// Reads wait for as long as they need again.
    pub(crate) fn never_give_up(&self) {
        self.give_up_at.store(0, Ordering::Relaxed);
    }

    fn gave_up(&self) -> bool {
        match self.give_up_at.load(Ordering::Relaxed) {
            0 => false,
            at => now_ms() >= at,
        }
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
        let interrupt = move || interrupt.stopped() || interrupt.is_broken() || interrupt.gave_up();
        if self.headers.is_empty() {
            return format::input_with_interrupt(&self.location, interrupt).map(fresh);
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
        format::input_with_interrupt_and_dictionary(&self.location, interrupt, options).map(fresh)
    }
}

impl VideoSource {
    /// Read over the network, where a connection can break and be made
    /// again. A path or a `file:` url is not.
    pub(crate) fn is_network(&self) -> bool {
        self.location.contains("://") && !self.location.starts_with("file:")
    }

    pub(crate) fn reconnect_limit(&self) -> Duration {
        self.reconnect_limit.unwrap_or(RECONNECT_LIMIT)
    }

    #[cfg(test)]
    pub(crate) fn with_reconnect_limit(mut self, limit: Duration) -> Self {
        self.reconnect_limit = Some(limit);
        self
    }
}

/// An input that just opened, with no error of its connection on record.
/// The record is never cleared by ffmpeg, and a demuxer can open fine after
/// a read that failed, like one for an index it can do without. Such an old
/// error must not look like a cut later.
fn fresh(mut input: Input) -> Input {
    // SAFETY: the context of an open input is valid, `pb` is checked
    // before it is used, and `error` is a plain field.
    unsafe {
        let io = (*input.as_mut_ptr()).pb;
        if !io.is_null() {
            (*io).error = 0;
        }
    }
    input
}

/// The error that broke the connection of a demuxer since it opened, none
/// while it is fine. A reader asks after every packet, a packet that came
/// through is no proof: the matroska demuxer answers a read that failed by
/// going back to the start of the block it was in, which makes a new
/// connection, and by skipping to the next cluster. It reports nothing and
/// the packets in between are lost. Other demuxers report the failed read
/// as the end of the file.
pub(crate) fn transport_error(input: &Input) -> Option<Error> {
    // SAFETY: the context of an open input is valid, and `pb` is checked
    // before it is read.
    let code = unsafe {
        let io = (*input.as_ptr()).pb;
        if io.is_null() {
            return None;
        }
        (*io).error
    };
    (code < 0).then(|| Error::from(code))
}

/// The byte of the source the demuxer has read up to, for a log line. After
/// a cut the demuxer hid it lies past the cut, at most a cluster.
pub(crate) fn byte_position(input: &Input) -> i64 {
    // SAFETY: as in `transport_error`.
    unsafe {
        let io = (*input.as_ptr()).pb;
        if io.is_null() { 0 } else { (*io).pos }
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
