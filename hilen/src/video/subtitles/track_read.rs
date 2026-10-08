//! Every line of one subtitle track, read once on a thread of its own with
//! a demuxer of its own. A seek then needs no packet from before its
//! keyframe: a line that began long before the target is already in hand,
//! also one that lies under a shorter line.

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use ffmpeg_next::{
    Error, Packet,
    ffi::{
        AVDiscard, AVFormatContext, AVSEEK_FLAG_BACKWARD, AVStream, av_seek_frame,
        avformat_index_get_entries_count, avformat_index_get_entry, avio_seek,
    },
    format::context::Input,
    media,
};
use log::{debug, error, info, warn};

use crate::{
    gm::LossyConvert,
    video::{
        VideoSource,
        decoder::first_timestamp,
        source::{Interrupt, byte_position, transport_error},
        subtitles::{Cue, CueDecoder},
    },
};

/// The wait before a track whose stream broke is opened again.
const AGAIN_AFTER: Duration = Duration::from_secs(1);

/// `whence` of `avio_seek` that moves nothing and reports the position.
const SEEK_CUR: i32 = 1;

/// Seconds a packet of another stream may lie past the last line of a group
/// before the walk gives the group up. Pictures are stored out of order.
const PAST_GROUP: f64 = 2.0;

/// How the track was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum How {
    /// From packet to packet through the container's index of the track, a
    /// few small reads.
    Index,
    /// The whole source read through once, the container has no index of
    /// the subtitle packets.
    Through,
}

/// One packet of the track in the index: where its read starts in the file
/// and its timestamp in the stream's time base.
#[derive(Debug, Clone, Copy)]
struct Entry {
    pos:   i64,
    ticks: i64,
}

struct TrackRead<'a> {
    input:      Input,
    decoder:    CueDecoder,
    /// Seconds the first timestamp of the picture sits at.
    start:      f64,
    time_bases: Vec<f64>,
    reads:      &'a Interrupt,
    source:     &'a VideoSource,
    /// Seconds of the stream the last packet read sits at.
    at:         f64,
}

/// How far the read of a track got, so a read that opens the source again
/// after a cut goes on where it stood and hands no line over twice.
#[derive(Default)]
struct Progress {
    /// Groups of the index that are done.
    groups:  usize,
    /// Packets of the track taken, of the group in work with an index, of
    /// the whole source without one.
    packets: usize,
    /// Every packet of the track taken so far, to tell that a read got
    /// further since the last cut.
    total:   usize,
}

/// A cut the read has not got past yet: when it began and how far the read
/// was.
struct Cut {
    since: Instant,
    total: usize,
}

/// Reads every line of subtitle stream `index` and hands each to `line`,
/// which answers false when nobody wants them any more. `Error::Exit` when
/// the read was broken or given up. A network stream that breaks is opened
/// again and the read goes on after the last packet it took, until it got
/// no further for the reconnect limit of the source.
pub(crate) fn read_track(
    source: &VideoSource,
    reads: &Interrupt,
    index: usize,
    line: &mut impl FnMut(Cue) -> bool,
) -> Result<How, Error> {
    let mut progress = Progress::default();
    let mut cut: Option<Cut> = None;
    loop {
        let err = match read_on(source, reads, index, line, &mut progress) {
            Err(err) if err != Error::Exit && source.is_network() => err,
            done => return done,
        };
        let since = match &cut {
            Some(cut) if cut.total == progress.total => cut.since,
            _ => Instant::now(),
        };
        if since.elapsed() >= source.reconnect_limit() {
            error!(
                "video {}: gave up on the subtitle stream of track {index} after {:.1} s, {err}",
                source.location(),
                since.elapsed().as_secs_f64()
            );
            return Err(err);
        }
        cut = Some(Cut {
            since,
            total: progress.total,
        });
        let until = Instant::now() + AGAIN_AFTER;
        while Instant::now() < until {
            if reads.stopped() || reads.is_broken() {
                return Err(Error::Exit);
            }
            sleep(Duration::from_millis(20));
        }
        info!(
            "video {}: the subtitle stream of track {index} opens again, {} packets of it are read",
            source.location(),
            progress.total
        );
    }
}

/// One open of the source and the read from where `progress` stands.
fn read_on(
    source: &VideoSource,
    reads: &Interrupt,
    index: usize,
    line: &mut impl FnMut(Cue) -> bool,
    progress: &mut Progress,
) -> Result<How, Error> {
    let mut read = TrackRead::open(source, reads, index)?;
    let name = read.input.format().name().to_string();
    let entries = read.index(&name)?;
    debug!(
        "subtitles {}: track {index} of {name}, {}",
        source.location(),
        entries
            .as_ref()
            .map_or("no index, reading through".to_string(), |entries| format!(
                "{} lines in the index",
                entries.len()
            ))
    );
    if let Some(entries) = entries {
        read.walk(&entries, line, progress)?;
        return Ok(How::Index);
    }
    // The look at the index moved the demuxer, and with no index nothing
    // brings it back to the start for sure. A fresh one stands there.
    drop(read);
    TrackRead::open(source, reads, index)?.through(line, progress)?;
    Ok(How::Through)
}

impl<'a> TrackRead<'a> {
    fn open(source: &'a VideoSource, reads: &'a Interrupt, index: usize) -> Result<Self, Error> {
        let input = source.open(reads)?;
        let start = input
            .streams()
            .best(media::Type::Video)
            .map_or(0.0, |video| first_timestamp(&video));
        let time_bases: Vec<f64> = input.streams().map(|stream| stream.time_base().into()).collect();
        let decoder = CueDecoder::open(&input, index)?;
        Ok(Self {
            input,
            decoder,
            start,
            time_bases,
            reads,
            source,
            at: 0.0,
        })
    }

    fn context(&mut self) -> *mut AVFormatContext {
        // SAFETY: the pointer of the open demuxer this struct owns.
        unsafe { self.input.as_mut_ptr() }
    }

    fn stream(&mut self) -> Result<(*mut AVStream, i32), Error> {
        let index = self.decoder.stream();
        let number = i32::try_from(index).map_err(|_| Error::StreamNotFound)?;
        let context = self.context();
        // SAFETY: the decoder was opened on stream `index` of this demuxer,
        // so the stream array has that many entries.
        let stream = unsafe { *(*context).streams.add(index) };
        Ok((stream, number))
    }

    fn entries(&mut self) -> Result<Vec<Entry>, Error> {
        let (stream, _) = self.stream()?;
        // SAFETY: the stream belongs to the open demuxer, and an entry
        // pointer is read before anything else touches the index.
        unsafe {
            let count = avformat_index_get_entries_count(stream);
            Ok((0..count)
                .filter_map(|at| avformat_index_get_entry(stream, at).as_ref())
                .map(|entry| Entry {
                    pos:   entry.pos,
                    ticks: entry.timestamp,
                })
                .collect())
        }
    }

    /// Puts the demuxer on the packet of the track at `ticks`.
    fn seek(&mut self, ticks: i64) -> Result<(), Error> {
        let (_, number) = self.stream()?;
        let context = self.context();
        let code = {
            let _reading = self.reads.reading();
            // SAFETY: the open demuxer and the number of one of its streams.
            unsafe { av_seek_frame(context, number, ticks, AVSEEK_FLAG_BACKWARD) }
        };
        if self.reads.is_broken() || self.reads.stopped() {
            return Err(Error::Exit);
        }
        if code < 0 {
            return Err(Error::from(code));
        }
        Ok(())
    }

    /// The index of the track when it lists every line, none when the
    /// source has to be read through.
    ///
    /// mp4 lists every sample of every track. Matroska lists what its
    /// muxer wrote into the cues, and its demuxer also adds every packet
    /// it happens to read, so an index there counts only when the cues
    /// gave it lines: more than the open had seen, or one that lies past
    /// the place the open read to.
    fn index(&mut self, name: &str) -> Result<Option<Vec<Entry>>, Error> {
        let seen = self.entries()?.len();
        let context = self.context();
        // SAFETY: the io context of the open demuxer, checked for null. A
        // seek by 0 from the current place only reports the position.
        let read_to = unsafe {
            let io = (*context).pb;
            if io.is_null() {
                i64::MAX
            } else {
                avio_seek(io, 0, SEEK_CUR)
            }
        };
        // Matroska reads its cues on the first seek. Without one the
        // source is not seekable and is read from where it stands.
        if let Err(err) = self.seek(i64::MIN) {
            if matches!(err, Error::Exit) {
                return Err(err);
            }
            return Ok(None);
        }
        let entries = self.entries()?;

        let listed = if name.contains("mp4") {
            !entries.is_empty()
        } else if name.contains("matroska") {
            entries.len() > seen || entries.iter().any(|entry| entry.pos > read_to)
        } else {
            false
        };
        Ok(listed.then_some(entries))
    }

    /// The next packet, none at the end of the source.
    fn next_packet(&mut self) -> Result<Option<Packet>, Error> {
        let mut packet = Packet::empty();
        let read = {
            let _reading = self.reads.reading();
            packet.read(&mut self.input)
        };
        if self.reads.is_broken() || self.reads.stopped() {
            return Err(Error::Exit);
        }
        // A packet that came is no proof, see `transport_error`.
        let cut = transport_error(&self.input);
        match (read, cut) {
            (Ok(()), None) => {
                if let Some(at) = self.seconds(&packet) {
                    self.at = at;
                }
                Ok(Some(packet))
            }
            (Err(Error::Eof), None) => Ok(None),
            (Err(err), None) | (_, Some(err)) => Err(self.cut_by(err)),
        }
    }

    /// The log line of a read or a seek that failed, with the place the
    /// read stood at.
    fn cut_by(&self, err: Error) -> Error {
        warn!(
            "video {}: the subtitle stream of track {} broke at {:.2} s, seen at byte {}, {err}",
            self.source.location(),
            self.decoder.stream(),
            self.at,
            byte_position(&self.input)
        );
        err
    }

    fn seconds(&self, packet: &Packet) -> Option<f64> {
        packet.pts().zip(self.time_bases.get(packet.stream())).map(|(pts, time_base)| {
            let ticks: f64 = pts.lossy_convert();
            ticks * time_base - self.start
        })
    }

    fn decode(&mut self, packet: &Packet, line: &mut impl FnMut(Cue) -> bool) -> Result<(), Error> {
        match self.decoder.decode(packet, self.start, 0) {
            Ok(Some(cue)) => {
                if !line(cue) {
                    return Err(Error::Exit);
                }
            }
            Ok(None) => {}
            // The film goes on without a line that fails to decode.
            Err(err) => debug!("subtitles: a line did not decode, {err}"),
        }
        Ok(())
    }

    /// From line to line through the index. Lines whose reads start at the
    /// same place in the file, a cluster of matroska, share one seek.
    fn walk(
        &mut self,
        entries: &[Entry],
        line: &mut impl FnMut(Cue) -> bool,
        progress: &mut Progress,
    ) -> Result<(), Error> {
        let index = self.decoder.stream();
        let time_base = self.time_bases.get(index).copied().ok_or(Error::StreamNotFound)?;
        // A read that opened the source again leaves out the groups that
        // are done.
        for group in entries.chunk_by(|a, b| a.pos == b.pos).skip(progress.groups) {
            let (Some(first), Some(last)) = (group.first(), group.last()) else {
                progress.groups += 1;
                continue;
            };
            if let Err(err) = self.seek(first.ticks) {
                return Err(if err == Error::Exit { err } else { self.cut_by(err) });
            }
            let last_ticks: f64 = last.ticks.lossy_convert();
            let until = last_ticks * time_base - self.start + PAST_GROUP;
            let mut left = group.len();
            // And the packets of this group it took before the cut.
            let mut known = progress.packets;
            while left > 0 {
                let Some(packet) = self.next_packet()? else {
                    break;
                };
                if packet.stream() != index {
                    if self.seconds(&packet).is_some_and(|at| at > until) {
                        break;
                    }
                    continue;
                }
                match packet.pts() {
                    Some(pts) if pts < first.ticks => continue,
                    Some(pts) if pts > last.ticks => break,
                    _ => {}
                }
                left -= 1;
                if known > 0 {
                    known -= 1;
                    continue;
                }
                progress.packets += 1;
                progress.total += 1;
                self.decode(&packet, line)?;
            }
            progress.groups += 1;
            progress.packets = 0;
        }
        Ok(())
    }

    /// The whole source once, every other stream thrown away by the
    /// demuxer.
    fn through(&mut self, line: &mut impl FnMut(Cue) -> bool, progress: &mut Progress) -> Result<(), Error> {
        let index = self.decoder.stream();
        let context = self.context();
        // SAFETY: the stream array of the open demuxer has `nb_streams`
        // entries, and `discard` is a plain field the demuxer reads.
        unsafe {
            let count = usize::try_from((*context).nb_streams).map_err(|_| Error::StreamNotFound)?;
            for at in (0..count).filter(|at| *at != index) {
                (**(*context).streams.add(at)).discard = AVDiscard::AVDISCARD_ALL;
            }
        }
        // The packets a read before a cut took come by again, they are
        // not handed over twice.
        let mut known = progress.packets;
        while let Some(packet) = self.next_packet()? {
            if packet.stream() != index {
                continue;
            }
            if known > 0 {
                known -= 1;
                continue;
            }
            progress.packets += 1;
            progress.total += 1;
            self.decode(&packet, line)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread::{scope, sleep},
        time::{Duration, Instant},
    };

    use crate::video::{
        source::Interrupt,
        subtitles::{
            Cue,
            track_read::{How, read_track},
        },
        test_fixture,
        test_server::{Cut, Mode, Server},
    };

    fn lines(fixture: &str, index: usize) -> (How, Vec<(f64, f64, String)>) {
        let stop = Arc::new(AtomicBool::new(false));
        let mut cues = Vec::new();
        let how = read_track(
            &test_fixture(fixture),
            &Interrupt::new(&stop),
            index,
            &mut |cue: Cue| {
                cues.push((cue.start, cue.end, cue.text));
                true
            },
        )
        .expect("the fixture reads");
        (how, cues)
    }

    fn overlap() -> Vec<(f64, f64, String)> {
        vec![
            (2.0, 26.0, "a long line".to_string()),
            (18.0, 21.0, "a short line over it".to_string()),
            (27.0, 28.0, "a last line".to_string()),
        ]
    }

    /// `overlap.mkv` has cues, its 3 lines come through the index.
    #[test]
    fn a_track_with_an_index_is_walked() {
        let (how, cues) = lines("overlap.mkv", 1);
        assert_eq!(how, How::Index);
        assert_eq!(cues, overlap());
    }

    /// `no_index.mkv` is the same film written to a pipe, so it has no
    /// cues. The same 3 lines come from a read through the file.
    #[test]
    fn a_track_with_no_index_is_read_through() {
        let (how, cues) = lines("no_index.mkv", 1);
        assert_eq!(how, How::Through);
        assert_eq!(cues, overlap());
    }

    /// The second subtitle track of a file with 2 sound and 2 subtitle
    /// tracks, only its own lines come.
    #[test]
    fn only_the_chosen_track_is_read() {
        let (_, cues) = lines("tracks.mkv", 4);
        let texts: Vec<&str> = cues.iter().map(|(_, _, text)| text.as_str()).collect();
        assert_eq!(texts, ["erste Zeile", "zweite Zeile"]);
    }

    /// A reader that answers false ends the read.
    #[test]
    fn the_read_ends_when_nobody_wants_the_lines() {
        let stop = Arc::new(AtomicBool::new(false));
        let mut seen = 0;
        let read = read_track(
            &test_fixture("overlap.mkv"),
            &Interrupt::new(&stop),
            1,
            &mut |_| {
                seen += 1;
                false
            },
        );
        assert!(read.is_err(), "the read went on");
        assert_eq!(seen, 1);
    }

    /// The server closes the connection of the read through `no_index.mkv`
    /// in the middle of the picture block at 18.0 seconds, bytes 5606 to
    /// 5641. The line at 18 seconds lies after it in the same cluster. The
    /// matroska demuxer skips to the next cluster on its own and reports
    /// nothing, so that line was lost with no word in the log. Now the read
    /// sees the cut, opens the source again and hands over every line once.
    #[test]
    fn a_cut_read_through_opens_again_and_gives_every_line() {
        // The first open only looks for an index, the second one reads.
        let cut = Cut {
            at:   5630,
            hold: Some(5620),
            skip: 1,
        };
        let server = Server::start("no_index.mkv", Mode::Cut, cut);
        let stop = Arc::new(AtomicBool::new(false));
        let reads = Interrupt::new(&stop);
        let mut cues = Vec::new();

        let how = scope(|scope| {
            scope.spawn(|| {
                // The read waits in the held answer.
                let deadline = Instant::now() + Duration::from_secs(10);
                while reads.read_time() < Duration::from_millis(200) && Instant::now() < deadline {
                    sleep(Duration::from_millis(5));
                }
                server.set(Mode::Whole);
                server.release();
            });
            read_track(&server.source(), &reads, 1, &mut |cue: Cue| {
                cues.push((cue.start, cue.end, cue.text));
                true
            })
        })
        .expect("the track reads over the cut");

        assert_eq!(how, How::Through);
        assert_eq!(cues, overlap());
        assert_eq!(
            server.cuts.load(Ordering::Relaxed),
            2,
            "the open for the index and the read that was cut"
        );
    }
}
