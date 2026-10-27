//! A list of pieces read as 1 video. `Pieces` walks piece after piece and
//! gives every frame its time in the whole list. The decode thread of a list
//! is built on it, `run` below, and so is the export. The step from a piece
//! to the next loses no frame, and on the decode thread it also costs no
//! time: the next piece is opened and decoded up to its first frame while
//! the queue is full and the thread would only wait.

use std::{
    mem::take,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, RecvError, SyncSender, TryRecvError, TrySendError},
    },
};

use ffmpeg_next::{Error, media};
use log::{debug, warn};

use crate::video::{
    decoder::{
        Command, MediaInfo, Message, Tracks, VideoFrame,
        reader::{FileInfo, Picture, Reader, Step},
    },
    source::{Interrupt, PieceList},
};

/// Seconds a piece may start after the place its file is read at and still
/// be reached by decoding on. A place further ahead, or behind, is a seek.
const READ_ON: f64 = 1.0;

/// One piece and the open file its frames come from.
struct Playing {
    index:  usize,
    reader: Reader,
    /// A frame read ahead of its turn, the next one to hand out.
    first:  Option<Picture>,
    /// A frame of this piece was handed out.
    shown:  bool,
}

impl Playing {
    /// Decodes up to the frame the reader was sent to and keeps it as the
    /// first one. `give_up` is asked after every step on the way, true ends
    /// the work there. False when it ended that way. The piece is whole
    /// then too, its next read goes on to the same frame.
    fn reach(&mut self, give_up: &mut dyn FnMut() -> bool) -> Result<bool, Error> {
        loop {
            match self.reader.step()? {
                Step::Done(picture) => {
                    self.first = picture;
                    return Ok(true);
                }
                Step::More if give_up() => return Ok(false),
                Step::More => {}
            }
        }
    }

    /// `reach` for a scrub. When a newer seek waits and a frame on the way
    /// is there, it stops and that frame is the first one, the nearest
    /// place it got to. A stop ends it at once. False when it did not reach
    /// its own frame.
    fn reach_near(&mut self, news: &mut dyn FnMut() -> News) -> Result<bool, Error> {
        loop {
            match self.reader.step()? {
                Step::Done(picture) => {
                    self.first = picture;
                    return Ok(true);
                }
                Step::More => match news() {
                    News::Stop => return Ok(false),
                    News::Seek if self.reader.has_skipped() => {
                        self.first = self.reader.take_skipped();
                        return Ok(false);
                    }
                    News::Seek | News::None => {}
                },
            }
        }
    }
}

/// What came for the decode thread while it works.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum News {
    None,
    /// A newer seek waits.
    Seek,
    /// A stop came or the player is gone.
    Stop,
}

/// The answer to `give_up` of work that always runs to its end.
fn never() -> bool {
    false
}

/// One picture of a list.
pub(crate) struct ListPicture {
    /// Seconds from the start of the list.
    pub seconds: f64,
    pub picture: Picture,
}

pub(crate) struct Pieces<'a> {
    list:    &'a PieceList,
    reads:   &'a Interrupt,
    /// The piece frames come from now, none at the end of the list.
    current: Option<Playing>,
    /// The piece after it, at its first frame before its turn.
    next:    Option<Playing>,
}

impl<'a> Pieces<'a> {
    pub(crate) fn new(list: &'a PieceList, reads: &'a Interrupt) -> Self {
        Self {
            list,
            reads,
            current: None,
            next: None,
        }
    }

    /// Opens the first piece, at the start of the list, and looks at every
    /// file once for a sound track. Gives what the first file is, none for a
    /// list with no pieces.
    pub(crate) fn start(&mut self) -> Result<Option<FileInfo>, Error> {
        let Some(first) = self.list.pieces().first() else {
            return Ok(None);
        };
        let mut playing = self.open(0, first.start)?;
        playing.reach(&mut never)?;
        let file = playing.reader.info().clone();
        self.current = Some(playing);

        let mut has_sound = file.has_sound;
        let mut seen = vec![&first.source];
        for piece in &self.list.pieces()[1..] {
            if has_sound {
                break;
            }
            if seen.iter().any(|source| source.same_as(&piece.source)) {
                continue;
            }
            seen.push(&piece.source);
            // A file that does not open fails the list at its turn.
            match piece.source.open(self.reads) {
                Ok(input) => has_sound = input.streams().best(media::Type::Audio).is_some(),
                Err(err) => warn!("video {}: does not open, {err}", piece.source.location()),
            }
        }
        self.list.set_has_sound(has_sound);
        Ok(Some(file))
    }

    /// Opens the file of a piece and sends its reader to the frame at
    /// `seconds` of that file. `reach` then decodes up to that frame.
    fn open(&self, index: usize, seconds: f64) -> Result<Playing, Error> {
        let piece = &self.list.pieces()[index];
        let mut reader = Reader::open(&piece.source, self.reads)?;
        reader.seek(seconds)?;
        Ok(Playing {
            index,
            reader,
            first: None,
            shown: false,
        })
    }

    /// The next picture is the one at these seconds of the list. `give_up`
    /// is asked after every step of the decode up to that frame, true ends
    /// the seek there. False when it ended that way, the caller then has a
    /// newer seek to run. The decode thread scrubs, the tests seek exact.
    #[cfg(test)]
    pub(crate) fn seek(&mut self, seconds: f64, give_up: &mut dyn FnMut() -> bool) -> Result<bool, Error> {
        let reached = self.seek_by(seconds, |playing| playing.reach(give_up))?;
        if !reached {
            debug!("video: seek to {seconds:.2} s of the list given up, a newer one waits");
        }
        Ok(reached)
    }

    /// A seek of a drag. With no newer seek it lands on its frame like
    /// `seek`. When a newer one comes on the way, the next picture is the
    /// frame it got to, so the drag shows a picture near the place in the
    /// time the decoder needs for its first frame, not for the exact one.
    /// False when it ended that way or on a stop.
    pub(crate) fn scrub(&mut self, seconds: f64, news: &mut dyn FnMut() -> News) -> Result<bool, Error> {
        let reached = self.seek_by(seconds, |playing| playing.reach_near(news))?;
        if !reached {
            debug!("video: seek to {seconds:.2} s of the list shows a frame on the way, a newer one waits");
        }
        Ok(reached)
    }

    fn seek_by(
        &mut self,
        seconds: f64,
        reach: impl FnOnce(&mut Playing) -> Result<bool, Error>,
    ) -> Result<bool, Error> {
        let Some((index, place)) = self.list.locate(seconds) else {
            return Ok(true);
        };
        let open = match (self.current.take(), self.next.take()) {
            (Some(playing), next) if playing.index == index => {
                self.next = next;
                Some(playing)
            }
            (_, Some(playing)) if playing.index == index => Some(playing),
            _ => None,
        };
        let mut playing = match open {
            Some(mut playing) => {
                playing.reader.seek(place)?;
                playing.first = None;
                playing.shown = false;
                playing
            }
            None => self.open(index, place)?,
        };
        let reached = reach(&mut playing)?;
        self.current = Some(playing);
        if reached {
            debug!("video: seek to {seconds:.2} s of the list, piece {index} at {place:.2} s");
        }
        Ok(reached)
    }

    /// Frames that came out of the decoders of the open pieces so far.
    #[cfg(test)]
    fn decoded(&self) -> u64 {
        [&self.current, &self.next]
            .into_iter()
            .flatten()
            .map(|playing| playing.reader.decoded())
            .sum()
    }

    /// The next picture of the list, none at its end.
    pub(crate) fn next_picture(&mut self) -> Result<Option<ListPicture>, Error> {
        let list = self.list;
        loop {
            let Some(playing) = &mut self.current else {
                return Ok(None);
            };
            let piece = &list.pieces()[playing.index];
            let picture = match playing.first.take() {
                Some(picture) => Some(picture),
                None => playing.reader.next()?,
            };
            let half = playing.reader.half_frame();
            match picture {
                // The frame at the end of a piece belongs to what follows
                // it. The first frame of a piece always shows, a piece can
                // be shorter than the time 1 frame of its file is on screen.
                Some(picture) if !playing.shown || picture.seconds + half < piece.end => {
                    playing.shown = true;
                    let seconds = list.start_of(playing.index) + (picture.seconds - piece.start).max(0.0);
                    return Ok(Some(ListPicture { seconds, picture }));
                }
                over => self.advance(over)?,
            }
        }
    }

    /// The picture `next_picture` gave last as the planes the player
    /// uploads.
    pub(crate) fn video_frame(
        &mut self,
        picture: &ListPicture,
        generation: u32,
    ) -> Result<VideoFrame, Error> {
        let playing = self.current.as_mut().ok_or(Error::Bug)?;
        playing.reader.video_frame(&picture.picture, generation, picture.seconds)
    }

    /// A piece that goes on in the file of the piece before it, a little
    /// ahead of its end. It is reached by decoding on, with no seek.
    fn reads_on(&self, index: usize) -> bool {
        let pieces = self.list.pieces();
        let (Some(before), Some(piece)) = (
            index.checked_sub(1).and_then(|at| pieces.get(at)),
            pieces.get(index),
        ) else {
            return false;
        };
        before.source.same_as(&piece.source) && (0.0..=READ_ON).contains(&(piece.start - before.end))
    }

    /// The current piece is over, `over` is the frame of its file that came
    /// after its end. Makes the next piece the current one.
    fn advance(&mut self, over: Option<Picture>) -> Result<(), Error> {
        let Some(mut playing) = self.current.take() else {
            return Ok(());
        };
        let index = playing.index + 1;
        let Some(piece) = self.list.pieces().get(index) else {
            return Ok(());
        };
        if let Some(next) = self.next.take().filter(|next| next.index == index) {
            self.current = Some(next);
            return Ok(());
        }
        if self.reads_on(index) {
            // The frame in hand is the first one of the piece when it
            // starts right at the cut.
            let half = playing.reader.half_frame();
            playing.index = index;
            playing.shown = false;
            playing.first = over.filter(|picture| picture.seconds + half >= piece.start);
            if playing.first.is_none() {
                playing.reader.skip_to(piece.start);
            }
            self.current = Some(playing);
            return Ok(());
        }
        let mut next = self.open(index, piece.start)?;
        next.reach(&mut never)?;
        self.current = Some(next);
        Ok(())
    }

    /// Opens the piece after the current one before its turn. A file that
    /// fails to open fails again at its turn, and the list fails then.
    /// `give_up` ends the decode up to the first frame of that piece, as in
    /// `seek`. The piece is kept and decodes on at its turn.
    pub(crate) fn prepare_next(&mut self, give_up: &mut dyn FnMut() -> bool) {
        let Some(playing) = &self.current else {
            return;
        };
        let index = playing.index + 1;
        let Some(piece) = self.list.pieces().get(index) else {
            return;
        };
        if self.next.as_ref().is_some_and(|next| next.index == index) || self.reads_on(index) {
            return;
        }
        let opened = self.open(index, piece.start).and_then(|mut next| {
            next.reach(give_up)?;
            Ok(next)
        });
        match opened {
            Ok(next) => self.next = Some(next),
            Err(err) => warn!(
                "video: piece {index}, {}, does not open ahead of its turn, {err}",
                piece.source.location()
            ),
        }
    }
}

/// The commands that wait for the decode thread of a list. Of the seeks
/// only the newest is kept. A seek of a list decodes up to its frame, so
/// a seek that waits behind a newer one would only hold the newer one back.
struct Waiting<'a> {
    commands: &'a Receiver<Command>,
    /// The generation and the seconds of the newest seek not run yet.
    seek:     Option<(u32, f64)>,
    /// A stop came or the player is gone.
    stop:     bool,
    /// Seeks that made way for a newer one since the last seek that ran.
    dropped:  u32,
}

impl<'a> Waiting<'a> {
    fn new(commands: &'a Receiver<Command>) -> Self {
        Self {
            commands,
            seek: None,
            stop: false,
            dropped: 0,
        }
    }

    fn keep(&mut self, command: Command) {
        match command {
            Command::Seek { generation, seconds } => {
                if self.seek.replace((generation, seconds)).is_some() {
                    self.dropped += 1;
                }
            }
            Command::Stop => self.stop = true,
            // A list has no subtitle tracks.
            Command::Subtitle(_) => {}
        }
    }

    /// Takes every command that is there and waits for none.
    fn drain(&mut self) {
        loop {
            match self.commands.try_recv() {
                Ok(command) => self.keep(command),
                Err(TryRecvError::Disconnected) => {
                    self.stop = true;
                    return;
                }
                Err(TryRecvError::Empty) => return,
            }
        }
    }

    fn news_now(&mut self) -> News {
        self.drain();
        if self.stop {
            News::Stop
        } else if self.seek.is_some() {
            News::Seek
        } else {
            News::None
        }
    }

    /// A seek or a stop is there, so the work in hand is of no use.
    fn news(&mut self) -> bool {
        self.drain();
        self.stop || self.seek.is_some()
    }

    /// Blocks until a seek or a stop is there.
    fn wait(&mut self) {
        while !self.news() {
            match self.commands.recv() {
                Ok(command) => self.keep(command),
                Err(RecvError) => self.stop = true,
            }
        }
    }
}

/// Runs the newest seek that waits, and only that one. When a newer seek
/// comes while it decodes, it ends at the first frame it has on the way,
/// and that frame goes out before the newer seek starts. The player shows
/// it while it waits for the newer one. So a drag that seeks on every move
/// shows a picture near the pointer all along, and the last seek of the
/// drag lands on its exact frame. A seek given up for each newer one with
/// no frame showed nothing until the drag ended. True when the list was
/// moved to another place.
fn seek_newest(pieces: &mut Pieces, waiting: &mut Waiting, generation: &mut u32) -> Result<bool, Error> {
    if waiting.stop {
        return Ok(false);
    }
    let Some((new, seconds)) = waiting.seek.take() else {
        return Ok(false);
    };
    let dropped = take(&mut waiting.dropped);
    if dropped > 0 {
        debug!("video: seek {new} of the list is the newest, {dropped} older ones were dropped");
    }
    *generation = new;
    pieces.scrub(seconds, &mut || waiting.news_now())?;
    Ok(true)
}

/// The decode thread of a list, the twin of `run` of a file.
pub(super) fn run(
    list: &PieceList,
    commands: &Receiver<Command>,
    messages: &SyncSender<Message>,
    counter: &AtomicU64,
    reads: &Interrupt,
) -> Result<(), Error> {
    let mut pieces = Pieces::new(list, reads);
    let Some(file) = pieces.start()? else {
        return Err(Error::StreamNotFound);
    };
    let info = MediaInfo {
        duration:   list.duration(),
        width:      file.width,
        height:     file.height,
        frame_rate: file.frame_rate,
        decoder:    file.decoder,
        // The sound of a list is made by the player, see `player/pieces.rs`.
        audio:      None,
        tracks:     Tracks::default(),
    };
    if messages.send(Message::Info(Box::new(info))).is_err() {
        return Ok(());
    }

    // The newest seek and whether the list was read to its end since.
    let mut generation = 0;
    let mut eof = false;
    let mut waiting = Waiting::new(commands);
    loop {
        if eof {
            // Nothing to decode until a seek, so block instead of spinning.
            waiting.wait();
        } else {
            waiting.drain();
        }
        eof &= !seek_newest(&mut pieces, &mut waiting, &mut generation)?;
        if waiting.stop {
            return Ok(());
        }
        if eof {
            continue;
        }

        let Some(picture) = pieces.next_picture()? else {
            eof = true;
            if messages.send(Message::Eof { generation }).is_err() {
                return Ok(());
            }
            continue;
        };
        let frame = pieces.video_frame(&picture, generation)?;
        counter.fetch_add(1, Ordering::Relaxed);
        match messages.try_send(Message::Frame(frame)) {
            Ok(()) => {}
            Err(TrySendError::Disconnected(_)) => return Ok(()),
            // The player has frames enough. This is the time to get the
            // next piece ready, then wait for room.
            Err(TrySendError::Full(message)) => {
                pieces.prepare_next(&mut || waiting.news());
                if messages.send(message).is_err() {
                    return Ok(());
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, AtomicU64, Ordering},
            mpsc::{Receiver, Sender, channel, sync_channel},
        },
        time::Duration,
    };

    use crate::video::{
        VideoPiece, VideoSource,
        decoder::{
            self, Command, Message, VideoFrame,
            pieces::{Pieces, Waiting, never, seek_newest},
        },
        source::Interrupt,
        test_fixture,
    };

    struct Thread {
        commands: Sender<Command>,
        messages: Receiver<Message>,
        stop:     Arc<AtomicBool>,
    }

    impl Drop for Thread {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }

    /// The decode thread of a player over these pieces.
    fn thread(pieces: impl IntoIterator<Item = VideoPiece>) -> Thread {
        let (commands, command_receiver) = channel();
        let (message_sender, messages) = sync_channel(decoder::QUEUE);
        let stop = Arc::new(AtomicBool::new(false));
        decoder::spawn(
            VideoSource::from_pieces(pieces),
            command_receiver,
            message_sender,
            Arc::new(AtomicU64::new(0)),
            Interrupt::new(&stop),
            true,
        );
        Thread {
            commands,
            messages,
            stop,
        }
    }

    fn ramp(first: u32, end: u32) -> VideoPiece {
        VideoPiece::new(
            test_fixture("ramp.mp4"),
            f64::from(first) / 30.0,
            f64::from(end) / 30.0,
        )
    }

    /// Which frame of the ramp fixture this is, from its luma, see
    /// `reader.rs`.
    fn ramp_index(frame: &VideoFrame) -> i32 {
        let middle = frame.y[usize::try_from(frame.y_stride).expect("a stride fits usize") * 36 + 64];
        (i32::from(middle) - 16 + 1) / 3
    }

    impl Thread {
        fn next(&self) -> Message {
            self.messages
                .recv_timeout(Duration::from_secs(10))
                .expect("the decode thread answers")
        }

        /// The length the thread reports for the list.
        fn duration(&self) -> f64 {
            match self.next() {
                Message::Info(info) => info.duration,
                Message::Error(err) => panic!("the list does not open: {err}"),
                _ => panic!("the first message is the info"),
            }
        }

        /// The frames of a generation up to the end of the list.
        fn frames(&self, generation: u32) -> Vec<VideoFrame> {
            let mut frames = Vec::new();
            loop {
                match self.next() {
                    Message::Frame(frame) if frame.generation == generation => frames.push(frame),
                    Message::Eof { generation: ended } if ended == generation => return frames,
                    Message::Error(err) => panic!("the list fails: {err}"),
                    _ => {}
                }
            }
        }
    }

    /// Checks that the frames follow one another a thirtieth of a second
    /// apart from `first` on, with none missing and none twice.
    fn assert_no_frame_missing(frames: &[VideoFrame], first: f64) {
        for (count, frame) in frames.iter().enumerate() {
            let expected = first + f64::from(u32::try_from(count).expect("a small count")) / 30.0;
            assert!(
                (frame.pts - expected).abs() < 0.001,
                "frame {count} of the list is at {}, not at {expected}",
                frame.pts
            );
        }
    }

    /// 2 pieces of 1 file with a jump between them, both starting and
    /// ending between 2 keyframes. The frame before the cut is the last
    /// one of the first piece, the frame after it the first one of the
    /// second, and the times run on with no hole.
    #[test]
    fn a_cut_inside_one_file_loses_no_frame() {
        let thread = thread([ramp(10, 20), ramp(41, 50)]);
        let duration = thread.duration();
        assert!((duration - 19.0 / 30.0).abs() < 1e-6, "{duration}");

        let frames = thread.frames(0);
        let seen: Vec<i32> = frames.iter().map(ramp_index).collect();
        let expected: Vec<i32> = (10..20).chain(41..50).collect();
        assert_eq!(seen, expected);
        assert_no_frame_missing(&frames, 0.0);
    }

    /// A piece that goes on where the one before it ended, the cut of a
    /// split with no trim, and one a few frames further on. Both are read
    /// on in the open file.
    #[test]
    fn a_piece_that_goes_on_in_the_same_file_is_read_on() {
        let thread = thread([ramp(6, 15), ramp(15, 24), ramp(27, 30)]);
        thread.duration();
        let frames = thread.frames(0);
        let seen: Vec<i32> = frames.iter().map(ramp_index).collect();
        let expected: Vec<i32> = (6..24).chain(27..30).collect();
        assert_eq!(seen, expected);
        assert_no_frame_missing(&frames, 0.0);
    }

    /// A piece of another file, with another size and frame rate, between 2
    /// pieces of the first file, and a piece that goes back in the file.
    #[test]
    fn pieces_of_two_files_follow_each_other() {
        // The color fixture has 1 frame a second. The piece is half of the
        // time its frame is on screen, and still shows that frame. By the
        // rule for the end of a piece alone it had no frame at all.
        let colors = VideoPiece::new(test_fixture("colors.mp4"), 2.0, 2.5);
        let thread = thread([ramp(50, 53), colors, ramp(3, 6)]);
        let duration = thread.duration();
        assert!((duration - 0.7).abs() < 1e-6, "{duration}");

        let frames = thread.frames(0);
        let sizes: Vec<(u32, u32)> = frames.iter().map(|frame| (frame.width, frame.height)).collect();
        let ramp_size = (128, 72);
        assert_eq!(
            sizes,
            [
                ramp_size,
                ramp_size,
                ramp_size,
                (128, 128),
                ramp_size,
                ramp_size,
                ramp_size
            ]
        );
        let indexes: Vec<i32> = frames.iter().filter(|frame| frame.height == 72).map(ramp_index).collect();
        assert_eq!(indexes, [50, 51, 52, 3, 4, 5]);
        assert_no_frame_missing(&frames[..3], 0.0);
        assert!((frames[3].pts - 0.1).abs() < 0.001, "{}", frames[3].pts);
        assert_no_frame_missing(&frames[4..], 0.6);
    }

    /// A seek goes to a place of the whole list: the piece that plays then,
    /// at the right frame of its file. A seek to the place of a cut gives
    /// the first frame after the cut.
    #[test]
    fn a_seek_goes_to_a_place_in_the_whole_list() {
        let thread = thread([ramp(10, 20), ramp(41, 50)]);
        thread.duration();
        let seek = |generation: u32, frame: u32| {
            thread
                .commands
                .send(Command::Seek {
                    generation,
                    seconds: f64::from(frame) / 30.0,
                })
                .expect("the decode thread takes commands");
            thread.frames(generation)
        };

        // 12 frames into the list is 2 frames into the second piece.
        let frames = seek(1, 12);
        let ramps: Vec<i32> = frames.iter().map(ramp_index).collect();
        assert_eq!(ramps, (43..50).collect::<Vec<_>>());
        assert_no_frame_missing(&frames, 12.0 / 30.0);

        // Back into the first piece, the second one follows again.
        let frames = seek(2, 7);
        let ramps: Vec<i32> = frames.iter().map(ramp_index).collect();
        assert_eq!(ramps, (17..20).chain(41..50).collect::<Vec<_>>());
        assert_no_frame_missing(&frames, 7.0 / 30.0);

        let frames = seek(3, 10);
        assert_eq!(ramp_index(&frames[0]), 41);
        assert_no_frame_missing(&frames, 10.0 / 30.0);
    }

    fn seek_to_frame(generation: u32, frame: u32) -> Command {
        Command::Seek {
            generation,
            seconds: f64::from(frame) / 30.0,
        }
    }

    /// Runs a test on the whole ramp fixture as a list of 1 piece, opened
    /// with no thread. The number is the frames decoded by then.
    fn with_ramp_list(test: impl FnOnce(&mut Pieces, u64)) {
        let source = VideoSource::from_pieces([ramp(0, 60)]);
        let list = source.piece_list().expect("a list of pieces");
        let stop = Arc::new(AtomicBool::new(false));
        let reads = Interrupt::new(&stop);
        let mut pieces = Pieces::new(list, &reads);
        pieces.start().expect("the list opens");
        let before = pieces.decoded();
        test(&mut pieces, before);
    }

    /// Which frame of the ramp fixture the next picture of the list is.
    fn next_ramp_index(pieces: &mut Pieces) -> i32 {
        let picture = pieces.next_picture().expect("the list decodes").expect("a picture");
        ramp_index(&pieces.video_frame(&picture, 0).expect("the frame converts"))
    }

    /// 20 seeks wait at once, the drag of a slider while a slow seek ran.
    /// Each is 22 frames or more after its keyframe. Only the newest one is
    /// decoded, every seek used to be, 20 times the work for 1 picture.
    #[test]
    fn of_the_seeks_that_wait_only_the_newest_is_decoded() {
        with_ramp_list(|pieces, before| {
            let (commands, receiver) = channel();
            for generation in 1..20 {
                let frame = if generation % 2 == 0 { 55 } else { 25 };
                commands
                    .send(seek_to_frame(generation, frame))
                    .expect("the queue takes commands");
            }
            commands.send(seek_to_frame(20, 52)).expect("the queue takes commands");

            let mut waiting = Waiting::new(&receiver);
            waiting.drain();
            let mut generation = 0;
            assert!(seek_newest(pieces, &mut waiting, &mut generation).expect("the list seeks"));
            assert_eq!(generation, 20);

            // The fixture has a keyframe a second, so 1 seek is 30 frames
            // at most. The 20 of them were 517.
            let decoded = pieces.decoded() - before;
            assert!(decoded <= 30, "{decoded} frames were decoded for 1 picture");
            assert_eq!(next_ramp_index(pieces), 52);
            assert_eq!(next_ramp_index(pieces), 53);
        });
    }

    /// The drag of a slider: a newer seek comes while a seek decodes. The
    /// running seek ends at the first frame it has on the way, from its
    /// keyframe up to its place, and that picture goes out before the
    /// newer seek runs. A seek given up for every newer one showed no
    /// picture until the drag ended. The last seek, with none after it,
    /// lands on its exact frame.
    #[test]
    fn a_running_seek_shows_a_frame_on_the_way_before_a_newer_one() {
        with_ramp_list(|pieces, _| {
            let (commands, receiver) = channel();
            commands.send(seek_to_frame(1, 55)).expect("the queue takes commands");
            let mut waiting = Waiting::new(&receiver);
            waiting.drain();
            // It waits in the queue while seek 1 decodes.
            commands.send(seek_to_frame(2, 25)).expect("the queue takes commands");

            let mut generation = 0;
            assert!(seek_newest(pieces, &mut waiting, &mut generation).expect("the list seeks"));
            assert_eq!(generation, 1);
            // The fixture has a keyframe a second, frame 30 is the one
            // before frame 55.
            let on_the_way = next_ramp_index(pieces);
            assert!(
                (30..55).contains(&on_the_way),
                "frame {on_the_way} is not on the way"
            );
            assert!(waiting.seek.is_some(), "the newer seek still waits");

            waiting.drain();
            assert!(seek_newest(pieces, &mut waiting, &mut generation).expect("the list seeks"));
            assert_eq!(generation, 2);
            assert_eq!(next_ramp_index(pieces), 25);
        });
    }

    /// A stop that waits behind seeks ends the thread with no decode.
    #[test]
    fn a_stop_that_waits_runs_no_seek() {
        with_ramp_list(|pieces, before| {
            let (commands, receiver) = channel();
            commands.send(seek_to_frame(1, 55)).expect("the queue takes commands");
            commands.send(Command::Stop).expect("the queue takes commands");

            let mut waiting = Waiting::new(&receiver);
            waiting.drain();
            let mut generation = 0;
            assert!(!seek_newest(pieces, &mut waiting, &mut generation).expect("nothing runs"));
            assert!(waiting.stop);
            assert_eq!(pieces.decoded(), before);
        });
    }

    /// A seek that runs ends when a newer one comes, long before its frame.
    /// The newer seek then lands on its exact frame. A seek that was given
    /// up with no seek after it still leads to its own frame.
    #[test]
    fn a_seek_gives_up_for_a_newer_one() {
        with_ramp_list(|pieces, before| {
            // Frame 55 is 25 frames after its keyframe. The newer seek
            // comes after 4 steps of the decode.
            let mut asked = 0;
            let mut newer_came = || {
                asked += 1;
                asked == 4
            };
            assert!(!pieces.seek(55.0 / 30.0, &mut newer_came).expect("the list seeks"));
            let decoded = pieces.decoded() - before;
            assert!(
                decoded <= 4,
                "{decoded} frames were decoded before the seek gave up"
            );

            assert!(pieces.seek(40.0 / 30.0, &mut never).expect("the list seeks"));
            assert_eq!(next_ramp_index(pieces), 40);
            assert_eq!(next_ramp_index(pieces), 41);

            assert!(!pieces.seek(17.0 / 30.0, &mut || true).expect("the list seeks"));
            assert_eq!(next_ramp_index(pieces), 17);
        });
    }

    /// Many seeks sent to a running thread one right after another, the
    /// drag of a slider. The frames of the last seek are the ones of its
    /// place, and every frame after the first of them belongs to it.
    #[test]
    fn the_frames_after_many_seeks_are_those_of_the_last_one() {
        let thread = thread([ramp(10, 20), ramp(41, 50)]);
        thread.duration();
        for generation in 1..40 {
            let frame = if generation % 2 == 0 { 3 } else { 17 };
            thread
                .commands
                .send(seek_to_frame(generation, frame))
                .expect("the decode thread takes commands");
        }
        // 12 frames into the list is 2 frames into the second piece.
        thread
            .commands
            .send(seek_to_frame(40, 12))
            .expect("the decode thread takes commands");

        let mut frames = Vec::new();
        loop {
            match thread.next() {
                Message::Frame(frame) if frame.generation == 40 => frames.push(frame),
                Message::Frame(_) => assert!(frames.is_empty(), "an old frame came after a newer one"),
                Message::Eof { generation: 40 } => break,
                Message::Error(err) => panic!("the list fails: {err}"),
                _ => {}
            }
        }
        let ramps: Vec<i32> = frames.iter().map(ramp_index).collect();
        assert_eq!(ramps, (43..50).collect::<Vec<_>>());
        assert_no_frame_missing(&frames, 12.0 / 30.0);
    }

    #[test]
    fn a_list_with_a_file_that_is_missing_fails() {
        let missing = VideoPiece::new(test_fixture("no_such_file.mp4"), 0.0, 1.0);
        let thread = thread([ramp(0, 3), missing]);
        thread.duration();
        loop {
            match thread.next() {
                Message::Error(_) => break,
                Message::Eof { .. } => panic!("the list ended with no error"),
                _ => {}
            }
        }
    }
}
