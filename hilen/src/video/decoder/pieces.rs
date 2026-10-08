//! A list of pieces read as 1 video. `Pieces` walks piece after piece and
//! gives every frame its time in the whole list. The decode thread of a list
//! is built on it, `run` below, and so is the export. The step from a piece
//! to the next loses no frame, and on the decode thread it also costs no
//! time: the next piece is opened and decoded up to its first frame while
//! the queue is full and the thread would only wait.

use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::{Receiver, RecvError, SyncSender, TryRecvError, TrySendError},
};

use ffmpeg_next::{Error, media};
use log::{debug, warn};

use crate::video::{
    decoder::{
        Command, MediaInfo, Message, Tracks, VideoFrame,
        reader::{FileInfo, Picture, Reader},
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
        let playing = self.open(0, first.start)?;
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

    /// Opens the file of a piece and decodes up to the frame at `seconds`
    /// of that file.
    fn open(&self, index: usize, seconds: f64) -> Result<Playing, Error> {
        let piece = &self.list.pieces()[index];
        let mut reader = Reader::open(&piece.source, self.reads)?;
        reader.seek(seconds)?;
        let first = reader.next()?;
        Ok(Playing {
            index,
            reader,
            first,
            shown: false,
        })
    }

    /// The next picture is the one at these seconds of the list.
    pub(crate) fn seek(&mut self, seconds: f64) -> Result<(), Error> {
        let Some((index, place)) = self.list.locate(seconds) else {
            return Ok(());
        };
        let open = match (self.current.take(), self.next.take()) {
            (Some(playing), next) if playing.index == index => {
                self.next = next;
                Some(playing)
            }
            (_, Some(playing)) if playing.index == index => Some(playing),
            _ => None,
        };
        self.current = Some(match open {
            Some(mut playing) => {
                playing.reader.seek(place)?;
                playing.first = playing.reader.next()?;
                playing.shown = false;
                playing
            }
            None => self.open(index, place)?,
        });
        debug!("video: seek to {seconds:.2} s of the list, piece {index} at {place:.2} s");
        Ok(())
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
        self.current = Some(self.open(index, piece.start)?);
        Ok(())
    }

    /// Opens the piece after the current one before its turn. A file that
    /// fails to open fails again at its turn, and the list fails then.
    pub(crate) fn prepare_next(&mut self) {
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
        match self.open(index, piece.start) {
            Ok(next) => self.next = Some(next),
            Err(err) => warn!(
                "video: piece {index}, {}, does not open ahead of its turn, {err}",
                piece.source.location()
            ),
        }
    }
}

/// Takes a seek, true when the command was one. The other commands mean
/// nothing to a list, it has no subtitle tracks.
fn seek(pieces: &mut Pieces, command: &Command, generation: &mut u32) -> Result<bool, Error> {
    let Command::Seek {
        generation: new,
        seconds,
    } = *command
    else {
        return Ok(false);
    };
    *generation = new;
    pieces.seek(seconds)?;
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
    loop {
        // Every queued command, the latest seek wins.
        loop {
            match commands.try_recv() {
                Ok(Command::Stop) | Err(TryRecvError::Disconnected) => return Ok(()),
                Ok(command) => eof &= !seek(&mut pieces, &command, &mut generation)?,
                Err(TryRecvError::Empty) => break,
            }
        }
        if eof {
            // Nothing to decode until a seek, so block instead of spinning.
            match commands.recv() {
                Ok(Command::Stop) | Err(RecvError) => return Ok(()),
                Ok(command) => eof &= !seek(&mut pieces, &command, &mut generation)?,
            }
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
                pieces.prepare_next();
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
        decoder::{self, Command, Message, VideoFrame},
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
