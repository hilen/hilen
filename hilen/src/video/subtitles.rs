//! Text subtitles. A track inside the source is decoded on the video thread
//! as its packets pass by, a file from outside is read whole on its own
//! thread. Both end as cues, and the player shows the one the clock is in.

use std::{
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{Receiver, RecvTimeoutError, TryRecvError, channel},
    },
    thread::Builder,
    time::Duration,
};

use ffmpeg_next::{
    Error, Packet,
    codec::{context::Context, decoder, subtitle},
    format::context::Input,
    media,
};
use log::{debug, warn};

use crate::{
    gm::{Clock, LossyConvert},
    video::{VideoSource, decoder::first_timestamp, source::Interrupt},
};

/// Seconds before a seek target the read back starts at. A line on screen
/// at the target began at most this long before it.
const READ_BACK: f64 = 10.0;

/// How long a stepped test waits for the read back before it goes on.
const STEPPED_WAIT: Duration = Duration::from_secs(5);

/// One line of subtitles and the seconds of the stream it shows for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Cue {
    /// The seek generation it was decoded in, so cues from before a seek are
    /// told apart, like frames.
    pub generation: u32,
    pub start:      f64,
    pub end:        f64,
    pub text:       String,
}

/// Decodes the packets of one subtitle stream into cues.
pub(crate) struct CueDecoder {
    stream:    usize,
    time_base: f64,
    decoder:   decoder::Subtitle,
}

impl CueDecoder {
    pub(crate) fn open(input: &Input, index: usize) -> Result<Self, Error> {
        let stream = input
            .streams()
            .find(|stream| stream.index() == index && stream.parameters().medium() == media::Type::Subtitle)
            .ok_or(Error::StreamNotFound)?;
        let decoder = Context::from_parameters(stream.parameters())?.decoder().subtitle()?;
        Ok(Self {
            stream: index,
            time_base: stream.time_base().into(),
            decoder,
        })
    }

    pub(crate) fn stream(&self) -> usize {
        self.stream
    }

    /// The cue of a packet of this stream, none for a packet with no text.
    /// `start` is the seconds the stream's first timestamp sits at.
    pub(crate) fn decode(
        &mut self,
        packet: &Packet,
        start: f64,
        generation: u32,
    ) -> Result<Option<Cue>, Error> {
        let mut decoded = subtitle::Subtitle::new();
        if !self.decoder.decode(packet, &mut decoded)? {
            return Ok(None);
        }

        let lines: Vec<String> = decoded
            .rects()
            .filter_map(|rect| match rect {
                subtitle::Rect::Text(text) => Some(text.get().to_string()),
                subtitle::Rect::Ass(ass) => Some(plain_text(ass.get())),
                subtitle::Rect::Bitmap(_) | subtitle::Rect::None(_) => None,
            })
            .filter(|line| !line.is_empty())
            .collect();
        if lines.is_empty() {
            return Ok(None);
        }

        let ticks: f64 = packet.pts().unwrap_or(0).lossy_convert();
        let at = ticks * self.time_base - start;
        let shown_from = f64::from(decoded.start()) / 1000.0;
        // The decoder gives the display time in milliseconds when the format
        // has one, else the packet's own length is all there is.
        let shown_until = if decoded.end() > decoded.start() && decoded.end() != u32::MAX {
            f64::from(decoded.end()) / 1000.0
        } else {
            let length: f64 = packet.duration().lossy_convert();
            length * self.time_base
        };

        Ok(Some(Cue {
            generation,
            start: at + shown_from,
            end: at + shown_until,
            text: lines.join("\n"),
        }))
    }
}

/// The text of an ASS event with its styling dropped. ffmpeg hands every text
/// format over as an ASS line, `ReadOrder,Layer,Style,Name,MarginL,MarginR,
/// MarginV,Effect,Text`. The override blocks in braces go, `\N` is a line
/// break and `\h` a space.
pub(crate) fn plain_text(ass: &str) -> String {
    let text = ass.splitn(9, ',').nth(8).unwrap_or(ass);

    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    let mut in_override = false;
    while let Some(c) = chars.next() {
        match c {
            '{' => in_override = true,
            '}' => in_override = false,
            _ if in_override => {}
            '\\' => match chars.peek() {
                Some('N' | 'n') => {
                    chars.next();
                    out.push('\n');
                }
                Some('h') => {
                    chars.next();
                    out.push(' ');
                }
                _ => out.push(c),
            },
            _ => out.push(c),
        }
    }
    out.trim().to_string()
}

/// Every cue of a subtitle file, read to its end.
fn read_file(source: &VideoSource, stop: &Arc<AtomicBool>) -> Result<Vec<Cue>, Error> {
    let mut input = source.open(&Interrupt::new(stop))?;
    let index = input
        .streams()
        .best(media::Type::Subtitle)
        .ok_or(Error::StreamNotFound)?
        .index();
    let mut decoder = CueDecoder::open(&input, index)?;

    let mut cues = Vec::new();
    loop {
        let mut packet = Packet::empty();
        match packet.read(&mut input) {
            Ok(()) => {
                if packet.stream() == index
                    && let Some(cue) = decoder.decode(&packet, 0.0, 0)?
                {
                    cues.push(cue);
                }
            }
            Err(Error::Eof) => break,
            Err(err) => return Err(err),
        }
    }
    cues.sort_by(|a, b| a.start.total_cmp(&b.start));
    Ok(cues)
}

/// The cues of subtitle stream `index` that are on screen at `target`, read
/// from a demuxer of their own. A seek restarts the picture at the keyframe
/// before the target, and the packet of a line that began before that
/// keyframe lies behind it, so the picture thread never sees it.
fn read_back(
    source: &VideoSource,
    stop: &Arc<AtomicBool>,
    index: usize,
    target: f64,
) -> Result<Vec<Cue>, Error> {
    let mut input = source.open(&Interrupt::new(stop))?;
    let video = input.streams().best(media::Type::Video).ok_or(Error::StreamNotFound)?;
    let start = first_timestamp(&video);
    let time_bases: Vec<f64> = input.streams().map(|stream| stream.time_base().into()).collect();
    let mut decoder = CueDecoder::open(&input, index)?;

    let from = (target - READ_BACK).max(0.0) + start;
    let micros: i64 = (from * 1_000_000.0).lossy_convert();
    input.seek(micros, ..micros)?;

    let mut cues = Vec::new();
    loop {
        let mut packet = Packet::empty();
        match packet.read(&mut input) {
            Ok(()) => {}
            Err(Error::Eof) => break,
            Err(err) => return Err(err),
        }
        let at = packet.pts().zip(time_bases.get(packet.stream())).map(|(pts, time_base)| {
            let ticks: f64 = pts.lossy_convert();
            ticks * time_base - start
        });
        // Pictures are stored out of order, so a packet a little past the
        // target can come before a subtitle packet from before it.
        if at.is_some_and(|at| at > target + 1.0) {
            break;
        }
        if packet.stream() == index
            && let Some(cue) = decoder.decode(&packet, start, 0)?
            && cue.start <= target
            && target < cue.end
        {
            cues.push(cue);
        }
    }
    Ok(cues)
}

/// The same line decoded twice. Both times come from the same packet
/// through the same math, so the times match to the bit.
fn same_line(a: &Cue, b: &Cue) -> bool {
    a.start.to_bits() == b.start.to_bits() && a.end.to_bits() == b.end.to_bits() && a.text == b.text
}

/// What the player knows about subtitles: the cues in hand and the text on
/// screen.
#[derive(Default)]
pub(crate) struct Subtitles {
    cues:         Vec<Cue>,
    /// A file from outside the source is loading on its thread.
    loading:      Option<Receiver<Result<Vec<Cue>, String>>>,
    /// The cues came from a file, so a seek keeps them.
    file:         bool,
    /// The subtitle stream of the source the cues come from.
    track:        Option<usize>,
    /// The line that is on screen at a seek target is read on its thread.
    reading_back: Option<Receiver<Vec<Cue>>>,
    shown:        Option<String>,
    /// The choice changed, the text on screen has to be looked at again even
    /// while paused.
    dirty:        bool,
}

impl Subtitles {
    /// Drops every cue, for a new track or none.
    pub(crate) fn reset(&mut self) {
        self.cues.clear();
        self.loading = None;
        self.reading_back = None;
        self.file = false;
        self.track = None;
        self.dirty = true;
    }

    /// The cues come from this subtitle stream of the source from now on.
    pub(crate) fn set_track(&mut self, index: Option<usize>) {
        self.track = index;
    }

    /// Starts reading the line that is on screen at `target`. The picture
    /// thread decodes again from the keyframe before the target and sends
    /// the lines from there on, a line that began earlier comes from here.
    pub(crate) fn read_back(&mut self, source: &VideoSource, stop: &Arc<AtomicBool>, target: f64) {
        let Some(index) = self.track else {
            return;
        };
        let (send, receive) = channel();
        // A newer seek drops the receiver of the older one, so a late
        // answer for an old target never reaches the cues.
        self.reading_back = Some(receive);
        let source = source.clone();
        let stop = Arc::clone(stop);
        Builder::new()
            .name("hilen-subtitles".into())
            .spawn(move || {
                let cues = match read_back(&source, &stop, index, target) {
                    Ok(cues) => cues,
                    Err(err) => {
                        warn!("video {}: no subtitle line read back, {err}", source.location());
                        Vec::new()
                    }
                };
                if send.send(cues).is_err() {
                    debug!("subtitles {}: a newer seek took over", source.location());
                }
            })
            .expect("failed to spawn the subtitle thread");
    }

    /// Starts reading a subtitle file from outside the source.
    pub(crate) fn load_file(&mut self, source: VideoSource, stop: &Arc<AtomicBool>) {
        self.reset();
        self.file = true;
        let (send, receive) = channel();
        self.loading = Some(receive);
        let stop = Arc::clone(stop);
        Builder::new()
            .name("hilen-subtitles".into())
            .spawn(move || {
                let cues = read_file(&source, &stop).map_err(|err| err.to_string());
                if send.send(cues).is_err() {
                    // The player picked something else in the meantime.
                    warn!("subtitles {}: nobody waits for them", source.location());
                }
            })
            .expect("failed to spawn the subtitle thread");
    }

    /// A line both the picture thread and the read back found is kept once.
    pub(crate) fn push(&mut self, cue: Cue) {
        let known = self.cues.iter().any(|have| same_line(have, &cue));
        if !self.file && !known {
            self.cues.push(cue);
        }
    }

    /// A seek leaves the cues of a track inside the source behind, the
    /// decoder sends the ones after the target again and the read back the
    /// one that is on screen at the target.
    pub(crate) fn seek(&mut self, source: &VideoSource, stop: &Arc<AtomicBool>, target: f64) {
        if !self.file {
            self.cues.clear();
        }
        self.dirty = true;
        self.read_back(source, stop, target);
    }

    /// The player has to keep updating until the text on screen is settled.
    pub(crate) fn pending(&self) -> bool {
        self.dirty || self.loading.is_some() || self.reading_back.is_some()
    }

    /// Takes the lines the read back found. Under stepped time it waits for
    /// them, a test must see the line on the frame after the seek.
    fn take_read_back(&mut self) {
        let Some(reading) = &self.reading_back else {
            return;
        };
        let cues = if Clock::is_stepped() {
            match reading.recv_timeout(STEPPED_WAIT) {
                Ok(cues) => Some(cues),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => Some(Vec::new()),
            }
        } else {
            match reading.try_recv() {
                Ok(cues) => Some(cues),
                Err(TryRecvError::Disconnected) => Some(Vec::new()),
                Err(TryRecvError::Empty) => None,
            }
        };
        if let Some(cues) = cues {
            self.reading_back = None;
            for cue in cues {
                self.push(cue);
            }
        }
    }

    pub(crate) fn shown(&self) -> Option<&str> {
        self.shown.as_deref()
    }

    /// Picks the line for `position`. True when it differs from what was on
    /// screen, `shown` has the new one.
    pub(crate) fn update(&mut self, position: f64, source: &VideoSource) -> bool {
        if let Some(loading) = &self.loading {
            match loading.try_recv() {
                Ok(Ok(cues)) => {
                    self.cues = cues;
                    self.loading = None;
                }
                Ok(Err(err)) => {
                    warn!("video {}: no subtitles from the file, {err}", source.location());
                    self.loading = None;
                }
                Err(TryRecvError::Disconnected) => self.loading = None,
                Err(TryRecvError::Empty) => {}
            }
        }
        self.take_read_back();
        self.dirty = false;

        let text = self
            .cues
            .iter()
            .rev()
            .find(|cue| cue.start <= position && position < cue.end)
            .map(|cue| cue.text.clone());
        if text == self.shown {
            return false;
        }
        self.shown = text;
        true
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, atomic::AtomicBool};

    use ffmpeg_next::{Error, Packet};

    use crate::{
        gm::LossyConvert,
        video::{
            source::Interrupt,
            subtitles::{Cue, CueDecoder, plain_text, read_back, read_file},
            test_fixture,
        },
    };

    #[test]
    fn ass_event_becomes_plain_text() {
        assert_eq!(plain_text("0,0,Default,,0,0,0,,Hello, world"), "Hello, world");
        assert_eq!(
            plain_text(r"1,0,Default,,0,0,0,,{\i1}first{\i0}\Nsecond\hline"),
            "first\nsecond line"
        );
        assert_eq!(plain_text(r"2,0,Default,,0,0,0,,{\an8\pos(10,20)}top"), "top");
    }

    fn close(cue: &Cue, start: f64, end: f64, text: &str) -> bool {
        (cue.start - start).abs() < 0.01 && (cue.end - end).abs() < 0.01 && cue.text == text
    }

    #[test]
    fn a_subtitle_file_gives_its_cues() {
        let stop = Arc::new(AtomicBool::new(false));
        let cues = read_file(&test_fixture("tracks.srt"), &stop).expect("the srt file reads");
        assert_eq!(cues.len(), 1, "{cues:?}");
        assert!(close(&cues[0], 0.5, 3.5, "line from a file"), "{cues:?}");
    }

    /// The second subtitle track of the fixture, stream 4, decoded packet by
    /// packet the way the video thread does it. Styling is dropped.
    #[test]
    fn a_track_inside_the_source_gives_its_cues() {
        let stop = Arc::new(AtomicBool::new(false));
        let mut input = test_fixture("tracks.mkv")
            .open(&Interrupt::new(&stop))
            .expect("the fixture opens");
        let mut decoder = CueDecoder::open(&input, 4).expect("stream 4 is a subtitle track");

        let mut cues = Vec::new();
        loop {
            let mut packet = Packet::empty();
            match packet.read(&mut input) {
                Ok(()) => {
                    if packet.stream() == decoder.stream()
                        && let Some(cue) = decoder.decode(&packet, 0.0, 7).expect("the line decodes")
                    {
                        cues.push(cue);
                    }
                }
                Err(Error::Eof) => break,
                Err(err) => panic!("reading the fixture: {err}"),
            }
        }
        assert_eq!(cues.len(), 2, "{cues:?}");
        assert!(close(&cues[0], 0.5, 1.5, "erste Zeile"), "{cues:?}");
        assert!(close(&cues[1], 2.0, 3.0, "zweite Zeile"), "{cues:?}");
        assert_eq!(cues[0].generation, 7);
    }

    /// The lines the picture thread gets after a seek: the demuxer lands on
    /// the keyframe before the target and reads on from there.
    fn seen_after_seek(target: f64) -> Vec<String> {
        let stop = Arc::new(AtomicBool::new(false));
        let mut input = test_fixture("tracks.mkv")
            .open(&Interrupt::new(&stop))
            .expect("the fixture opens");
        let mut decoder = CueDecoder::open(&input, 4).expect("stream 4 is a subtitle track");
        let micros: i64 = (target * 1_000_000.0).lossy_convert();
        input.seek(micros, ..micros).expect("the fixture seeks");

        let mut lines = Vec::new();
        loop {
            let mut packet = Packet::empty();
            match packet.read(&mut input) {
                Ok(()) => {
                    if packet.stream() == decoder.stream()
                        && let Some(cue) = decoder.decode(&packet, 0.0, 0).expect("the line decodes")
                    {
                        lines.push(cue.text);
                    }
                }
                Err(Error::Eof) => break,
                Err(err) => panic!("reading the fixture: {err}"),
            }
        }
        lines
    }

    /// The fixture has a keyframe every second and the line `erste Zeile`
    /// from 0.5 to 1.5. A seek to 1.2 lands on the keyframe at 1, behind
    /// the packet of that line, so the picture thread never sends it. The
    /// read back finds it.
    #[test]
    fn a_line_that_began_before_the_seek_keyframe_is_read_back() {
        assert_eq!(
            seen_after_seek(1.2),
            ["zweite Zeile"],
            "the picture thread misses the first line"
        );

        let stop = Arc::new(AtomicBool::new(false));
        let cues = read_back(&test_fixture("tracks.mkv"), &stop, 4, 1.2).expect("the fixture reads");
        assert_eq!(cues.len(), 1, "{cues:?}");
        assert!(close(&cues[0], 0.5, 1.5, "erste Zeile"), "{cues:?}");
    }

    /// Between the 2 lines nothing is on screen, and a line that starts
    /// after the target is left to the picture thread.
    #[test]
    fn the_read_back_gives_only_the_line_on_screen_at_the_target() {
        let stop = Arc::new(AtomicBool::new(false));
        let fixture = test_fixture("tracks.mkv");
        let between = read_back(&fixture, &stop, 4, 1.7).expect("the fixture reads");
        assert_eq!(between, Vec::new());
        let cues = read_back(&fixture, &stop, 4, 2.5).expect("the fixture reads");
        assert_eq!(cues.len(), 1, "{cues:?}");
        assert!(close(&cues[0], 2.0, 3.0, "zweite Zeile"), "{cues:?}");
    }
}
