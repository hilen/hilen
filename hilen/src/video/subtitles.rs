//! Text subtitles. A track inside the source is decoded on the video thread
//! as its packets pass by, a file from outside is read whole on its own
//! thread. Both end as cues, and the player shows the one the clock is in.

use std::{
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{Receiver, TryRecvError, channel},
    },
    thread::Builder,
};

use ffmpeg_next::{
    Error, Packet,
    codec::{context::Context, decoder, subtitle},
    format::context::Input,
    media,
};
use log::warn;

use crate::{gm::LossyConvert, video::VideoSource};

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
    let mut input = source.open(stop)?;
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

/// What the player knows about subtitles: the cues in hand and the text on
/// screen.
#[derive(Default)]
pub(crate) struct Subtitles {
    cues:    Vec<Cue>,
    /// A file from outside the source is loading on its thread.
    loading: Option<Receiver<Result<Vec<Cue>, String>>>,
    /// The cues came from a file, so a seek keeps them.
    file:    bool,
    shown:   Option<String>,
    /// The choice changed, the text on screen has to be looked at again even
    /// while paused.
    dirty:   bool,
}

impl Subtitles {
    /// Drops every cue, for a new track or none.
    pub(crate) fn reset(&mut self) {
        self.cues.clear();
        self.loading = None;
        self.file = false;
        self.dirty = true;
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

    pub(crate) fn push(&mut self, cue: Cue) {
        if !self.file {
            self.cues.push(cue);
        }
    }

    /// A seek leaves the cues of a track inside the source behind, the
    /// decoder sends the ones after the target again.
    pub(crate) fn seek(&mut self) {
        if !self.file {
            self.cues.clear();
        }
        self.dirty = true;
    }

    /// The player has to keep updating until the text on screen is settled.
    pub(crate) fn pending(&self) -> bool {
        self.dirty || self.loading.is_some()
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

    use crate::video::{
        subtitles::{Cue, CueDecoder, plain_text, read_file},
        test_fixture,
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
        let mut input = test_fixture("tracks.mkv").open(&stop).expect("the fixture opens");
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
}
