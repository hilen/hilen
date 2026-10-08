//! One file read picture by picture, with no player, no queue and no clock.
//! The pictures for an app, the pieces of a list and the export read through
//! it. A seek lands on the exact frame: decoding starts at the keyframe
//! before the place and the frames up to it are decoded and skipped, by the
//! same rule a seek of the player uses.

use ffmpeg_next::{Error, Packet, frame, util::error::EAGAIN};

use crate::{
    gm::LossyConvert,
    video::{
        VideoSource,
        decoder::{Decoding, Matrix, Transfer, VideoFrame, color_info, open},
        hw,
        source::Interrupt,
    },
};

/// One decoded picture as the decoder made it, on the hardware device or in
/// system memory.
pub(crate) struct Picture {
    /// Seconds from the start of the stream.
    pub seconds: f64,
    pub frame:   frame::Video,
}

/// How the samples of a picture turn into colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PictureColor {
    pub full_range: bool,
    pub matrix:     Matrix,
    pub transfer:   Transfer,
}

impl Picture {
    pub(crate) fn color(&self) -> PictureColor {
        let (full_range, matrix, transfer) = color_info(&self.frame);
        PictureColor {
            full_range,
            matrix,
            transfer,
        }
    }

    /// The picture in system memory. A frame of the hardware device is copied
    /// down, any other one is handed back as it is.
    pub(crate) fn into_software(self) -> Result<frame::Video, Error> {
        if self.frame.format() != hw::pixel() {
            return Ok(self.frame);
        }
        let mut copy = frame::Video::empty();
        hw::transfer(&self.frame, &mut copy)?;
        Ok(copy)
    }
}

/// What a file is, known once it opened.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FileInfo {
    pub duration:   f64,
    pub width:      u32,
    pub height:     u32,
    pub frame_rate: f64,
    /// The ffmpeg codec name of the picture.
    pub decoder:    String,
    pub has_sound:  bool,
}

pub(crate) struct Reader {
    decoding: Decoding,
    reads:    Interrupt,
    info:     FileInfo,
    /// The demuxer is at the end and the decoder was told.
    drained:  bool,
    /// The last frame a seek skipped. A place past the last frame gives it.
    skipped:  Option<Picture>,
}

impl Reader {
    pub(crate) fn open(source: &VideoSource, reads: &Interrupt) -> Result<Self, Error> {
        let (decoding, info) = open(source, reads)?;
        let info = FileInfo {
            duration:   info.duration,
            width:      info.width,
            height:     info.height,
            frame_rate: info.frame_rate,
            decoder:    info.decoder,
            has_sound:  !info.tracks.audio.is_empty(),
        };
        Ok(Self {
            decoding,
            reads: reads.clone(),
            info,
            drained: false,
            skipped: None,
        })
    }

    pub(crate) fn info(&self) -> &FileInfo {
        &self.info
    }

    /// Half the time one frame is on screen. A frame is the one for a place
    /// from this much before its timestamp.
    pub(crate) fn half_frame(&self) -> f64 {
        0.5 / self.info.frame_rate
    }

    /// The next `next` gives the frame at these seconds. Decoding starts
    /// again at the keyframe before them.
    pub(crate) fn seek(&mut self, seconds: f64) -> Result<(), Error> {
        let target = seconds.max(0.0);
        // A whole number of microseconds, far below what an i64 holds.
        let micros: i64 = ((target + self.decoding.start) * 1_000_000.0).round().lossy_convert();
        {
            let _reading = self.reads.reading();
            self.decoding.input.seek(micros, ..micros)?;
        }
        self.decoding.decoder.flush();
        self.decoding.skip_until = Some(target);
        self.drained = false;
        self.skipped = None;
        Ok(())
    }

    /// The next `next` gives the frame at these seconds with no seek, the
    /// frames up to it are decoded and skipped. For a place a little ahead
    /// of the last frame, where a seek would go back to a keyframe.
    pub(crate) fn skip_to(&mut self, seconds: f64) {
        self.decoding.skip_until = Some(seconds.max(0.0));
        self.skipped = None;
    }

    /// The next picture in the order of the stream, none at its end. After a
    /// seek to a place past the last frame the last frame comes, once.
    pub(crate) fn next(&mut self) -> Result<Option<Picture>, Error> {
        loop {
            let mut decoded = frame::Video::empty();
            match self.decoding.decoder.receive_frame(&mut decoded) {
                Ok(()) => {
                    let seconds = self.decoding.seconds(&decoded);
                    self.decoding.sent += 1;
                    let picture = Picture {
                        seconds,
                        frame: decoded,
                    };
                    let half = self.half_frame();
                    if self.decoding.skip_until.is_some_and(|until| seconds + half < until) {
                        self.skipped = Some(picture);
                        continue;
                    }
                    self.decoding.skip_until = None;
                    self.skipped = None;
                    return Ok(Some(picture));
                }
                Err(Error::Other { errno: EAGAIN }) => {}
                Err(Error::Eof) => {
                    self.decoding.skip_until = None;
                    return Ok(self.skipped.take());
                }
                Err(err) => return Err(err),
            }
            self.feed()?;
        }
    }

    /// One more packet of the picture stream into the decoder, or the end of
    /// the stream.
    fn feed(&mut self) -> Result<(), Error> {
        if self.drained {
            return Ok(());
        }
        let mut packet = Packet::empty();
        let read = {
            let _reading = self.reads.reading();
            packet.read(&mut self.decoding.input)
        };
        match read {
            Ok(()) => {
                if packet.stream() == self.decoding.stream {
                    self.decoding.decoder.send_packet(&packet)?;
                }
                Ok(())
            }
            Err(Error::Eof) => {
                self.drained = true;
                self.decoding.decoder.send_eof()
            }
            Err(err) => Err(err),
        }
    }

    /// The picture as the planes the player uploads, with the timestamp and
    /// the seek generation the player is told.
    pub(crate) fn video_frame(
        &mut self,
        picture: &Picture,
        generation: u32,
        pts: f64,
    ) -> Result<VideoFrame, Error> {
        self.decoding.generation = generation;
        self.decoding.convert(&picture.frame, pts)
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc, atomic::AtomicBool};

    use crate::video::{
        decoder::reader::{Picture, Reader},
        source::Interrupt,
        test_fixture,
    };

    /// The fixture has 60 frames at 30 a second, frame `n` is a flat gray of
    /// luma `16 + 3 n`. One keyframe a second and B frames, so a place in the
    /// middle is far from where decoding can start.
    pub(crate) fn ramp() -> Reader {
        let stop = Arc::new(AtomicBool::new(false));
        Reader::open(&test_fixture("ramp.mp4"), &Interrupt::new(&stop)).expect("the fixture opens")
    }

    /// Which frame of the ramp fixture a picture is, from its luma.
    pub(crate) fn ramp_index(picture: Picture) -> i32 {
        let frame = picture.into_software().expect("the frame copies down");
        let stride = frame.stride(0);
        let middle = frame.data(0)[stride * 36 + 64];
        (i32::from(middle) - 16 + 1) / 3
    }

    #[test]
    fn a_file_reads_in_order_from_its_start() {
        let mut reader = ramp();
        let info = reader.info().clone();
        assert_eq!((info.width, info.height), (128, 72));
        assert!((info.duration - 2.0).abs() < 0.01, "{}", info.duration);
        assert!((info.frame_rate - 30.0).abs() < 0.01, "{}", info.frame_rate);
        assert!(info.has_sound);

        let mut seen = Vec::new();
        while let Some(picture) = reader.next().expect("the fixture decodes") {
            let at = (picture.seconds * 30.0).round();
            assert!((picture.seconds * 30.0 - at).abs() < 0.01, "{}", picture.seconds);
            seen.push(ramp_index(picture));
        }
        assert_eq!(seen, (0..60).collect::<Vec<_>>());
    }

    /// A seek gives the frame at the place, also between 2 keyframes, and
    /// the frames after it follow in order. A seek back works after the end
    /// of the file was reached.
    #[test]
    fn a_seek_lands_on_the_exact_frame() {
        let mut reader = ramp();
        for frame in [47, 3, 29, 30, 31, 59, 0] {
            let seconds = f64::from(frame) / 30.0;
            reader.seek(seconds).expect("the fixture seeks");
            let picture = reader.next().expect("the fixture decodes").expect("a frame");
            assert!((picture.seconds - seconds).abs() < 0.001);
            assert_eq!(ramp_index(picture), frame, "the frame at {seconds} s");
            if frame < 59 {
                let after = reader.next().expect("the fixture decodes").expect("a frame");
                assert_eq!(ramp_index(after), frame + 1, "the frame after {seconds} s");
            }
        }
    }

    /// A frame is the one for a place from half a frame before its
    /// timestamp, the rule a seek of the player has.
    #[test]
    fn a_place_between_two_frames_gives_the_nearest_one() {
        let mut reader = ramp();
        let frame = 1.0 / 30.0;
        for (seconds, expected) in [(10.4 * frame, 10), (10.6 * frame, 11)] {
            reader.seek(seconds).expect("the fixture seeks");
            let picture = reader.next().expect("the fixture decodes").expect("a frame");
            assert_eq!(ramp_index(picture), expected, "the frame at {seconds} s");
        }
    }

    #[test]
    fn a_place_past_the_end_gives_the_last_frame_once() {
        let mut reader = ramp();
        reader.seek(5.0).expect("the fixture seeks");
        let picture = reader.next().expect("the fixture decodes").expect("the last frame");
        assert_eq!(ramp_index(picture), 59);
        assert!(reader.next().expect("the fixture decodes").is_none());
    }

    #[test]
    fn a_skip_ahead_needs_no_seek() {
        let mut reader = ramp();
        reader.seek(0.2).expect("the fixture seeks");
        let picture = reader.next().expect("the fixture decodes").expect("a frame");
        assert_eq!(ramp_index(picture), 6);
        reader.skip_to(0.5);
        let picture = reader.next().expect("the fixture decodes").expect("a frame");
        assert_eq!(ramp_index(picture), 15);
    }
}
