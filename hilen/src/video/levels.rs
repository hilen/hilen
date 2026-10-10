//! How loud the sound of a video file is along its length, for the waveform
//! of a clip in an editor. A part of the file is cut into equal steps and
//! every step gives its loudest sample.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::Builder,
};

use anyhow::{Result, anyhow, bail};
use log::error;
use parking_lot::Mutex;

use crate::{
    deps::hreads::on_main,
    gm::LossyConvert,
    video::{
        VideoSource,
        audio::pieces::{FileSound, samples},
        count_to_f64,
        source::Interrupt,
    },
};

/// Frames asked of the file in one read.
const CHUNK: usize = 16_384;

/// What `VideoLevels::load` reports, once and on the main thread.
pub enum VideoLevelsEvent {
    /// A level per step, in the order of time, see `VideoLevels::read`.
    Loaded(Vec<f32>),
    /// The file did not open or decode, with the reason.
    Failed(String),
}

/// A `VideoLevels::load` that runs. Dropping it changes nothing, the load
/// goes on.
pub struct VideoLevelsLoad {
    cancelled: Arc<AtomicBool>,
}

impl VideoLevelsLoad {
    /// Ends the load. No event comes after this call.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

/// The loudness of the sound of a video file.
pub struct VideoLevels;

impl VideoLevels {
    /// The part of a file from `start` up to `end`, in seconds, cut into
    /// `count` equal steps. Every step gives its loudest sample of both
    /// channels as a level from 0, silence, to 1, full scale. A file with no
    /// sound track gives zeros, and so does the part past the end of the
    /// sound. It decodes the sound of the whole part and blocks, so it lives
    /// on a thread of the app, never on the main thread. `load` is the same
    /// work with the thread included.
    pub fn read(source: impl Into<VideoSource>, start: f64, end: f64, count: usize) -> Result<Vec<f32>> {
        let stop = Arc::new(AtomicBool::new(false));
        read(&source.into(), start, end, count, &Interrupt::new(&stop))
    }

    /// Reads the levels on a thread of its own and hands them over on the
    /// main thread, see `VideoLevelsEvent`.
    pub fn load(
        source: impl Into<VideoSource>,
        start: f64,
        end: f64,
        count: usize,
        each: impl FnOnce(VideoLevelsEvent) + Send + 'static,
    ) -> VideoLevelsLoad {
        let source = source.into();
        let cancelled = Arc::new(AtomicBool::new(false));
        let report = Reporter {
            each:      Arc::new(Mutex::new(Some(each))),
            cancelled: Arc::clone(&cancelled),
        };
        let location = source.location().to_string();
        let failed = report.clone();
        let spawned = Builder::new().name("hilen-video-levels".into()).spawn(move || {
            let reads = Interrupt::new(&report.cancelled);
            match read(&source, start, end, count, &reads) {
                Ok(levels) => report.send(VideoLevelsEvent::Loaded(levels)),
                // A cancel ends a read with an error, that is not a failure.
                Err(_) if reads.stopped() => {}
                Err(err) => {
                    error!("video {}: {err:#}", source.location());
                    report.send(VideoLevelsEvent::Failed(format!("{err:#}")));
                }
            }
        });
        if let Err(err) = spawned {
            error!("video {location}: no thread to read the sound levels, {err}");
            failed.send(VideoLevelsEvent::Failed(err.to_string()));
        }
        VideoLevelsLoad { cancelled }
    }
}

/// Hands the event of a load to the app, on the main thread.
struct Reporter<F> {
    each:      Arc<Mutex<Option<F>>>,
    cancelled: Arc<AtomicBool>,
}

impl<F> Clone for Reporter<F> {
    fn clone(&self) -> Self {
        Self {
            each:      Arc::clone(&self.each),
            cancelled: Arc::clone(&self.cancelled),
        }
    }
}

impl<F: FnOnce(VideoLevelsEvent) + Send + 'static> Reporter<F> {
    fn send(&self, event: VideoLevelsEvent) {
        let this = self.clone();
        on_main(move || {
            if this.cancelled.load(Ordering::Relaxed) {
                return;
            }
            if let Some(each) = this.each.lock().take() {
                each(event);
            }
        });
    }
}

fn to_f64(count: usize) -> f64 {
    count_to_f64(u64::try_from(count).expect("a sample count fits u64"))
}

fn read(source: &VideoSource, start: f64, end: f64, count: usize, reads: &Interrupt) -> Result<Vec<f32>> {
    let mut levels = vec![0.0; count];
    let first = samples(start);
    let total = samples(end).saturating_sub(first);
    if count == 0 || total == 0 {
        return Ok(levels);
    }
    let file = FileSound::open(source, reads)
        .map_err(|err| anyhow!("video {} does not open: {err}", source.location()))?;
    let Some(mut file) = file else {
        return Ok(levels);
    };
    file.seek(first)?;

    // The sample of the part the step after `step` starts at.
    let edge =
        |step: usize| -> usize { (to_f64(total) * to_f64(step + 1) / to_f64(count)).round().lossy_convert() };
    let (mut at, mut step, mut next) = (0, 0, edge(0));
    while at < total {
        if reads.stopped() {
            bail!("the read of the sound levels was cancelled");
        }
        for frame in file.read((total - at).min(CHUNK))? {
            while at >= next && step + 1 < count {
                step += 1;
                next = edge(step);
            }
            let level = frame.left.abs().max(frame.right.abs()).min(1.0);
            if level > levels[step] {
                levels[step] = level;
            }
            at += 1;
        }
    }
    Ok(levels)
}

#[cfg(test)]
mod test {
    use crate::video::{levels::VideoLevels, test_fixture};

    fn assert_levels(levels: &[f32], expected: &[f32], what: &str) {
        assert_eq!(levels.len(), expected.len(), "{what}");
        for (step, (level, expected)) in levels.iter().zip(expected).enumerate() {
            assert!(
                (level - expected).abs() < 0.001,
                "{what}: step {step} is {level}, not {expected}"
            );
        }
    }

    /// The sound of the ramp fixture rises in a line from -0.8 to 0.8 over
    /// its 2 seconds, so the loudest sample of a step is at the end of it
    /// that is further from the middle of the file.
    #[test]
    fn a_step_gives_its_loudest_sample() {
        let levels = VideoLevels::read(test_fixture("ramp.mp4"), 0.0, 2.0, 4).expect("the fixture decodes");
        assert_levels(&levels, &[0.8, 0.4, 0.4, 0.8], "the whole file");

        let levels = VideoLevels::read(test_fixture("ramp.mp4"), 0.0, 2.0, 8).expect("the fixture decodes");
        assert_levels(&levels, &[0.8, 0.6, 0.4, 0.2, 0.2, 0.4, 0.6, 0.8], "finer steps");
    }

    /// The part of a trimmed clip, not the whole file.
    #[test]
    fn a_part_of_the_file_gives_only_its_sound() {
        let levels = VideoLevels::read(test_fixture("ramp.mp4"), 0.25, 0.75, 2).expect("the fixture decodes");
        assert_levels(&levels, &[0.6, 0.4], "a part before the middle");

        let levels = VideoLevels::read(test_fixture("ramp.mp4"), 1.5, 1.75, 1).expect("the fixture decodes");
        assert_levels(&levels, &[0.6], "a part after the middle");
    }

    #[test]
    fn no_sound_is_silence() {
        let levels =
            VideoLevels::read(test_fixture("ramp_silent.mp4"), 0.0, 2.0, 4).expect("the fixture opens");
        assert_levels(&levels, &[0.0; 4], "a file with no sound track");

        // The fixture is 2 seconds long.
        let levels = VideoLevels::read(test_fixture("ramp.mp4"), 3.0, 4.0, 2).expect("the fixture decodes");
        assert_levels(&levels, &[0.0; 2], "past the end of the sound");

        let levels = VideoLevels::read(test_fixture("ramp.mp4"), 1.0, 1.0, 3).expect("nothing is read");
        assert_levels(&levels, &[0.0; 3], "a part with no length");
        let levels = VideoLevels::read(test_fixture("ramp.mp4"), 0.0, 2.0, 0).expect("nothing is read");
        assert_levels(&levels, &[], "no steps");
    }

    #[test]
    fn a_missing_file_fails() {
        assert!(VideoLevels::read(test_fixture("no_such_file.mp4"), 0.0, 1.0, 4).is_err());
    }
}
