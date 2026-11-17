use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use anyhow::Result;
use log::info;
use parking_lot::Mutex;

use super::{SpeechError, SpeechModel, SpeechModels, system};
use crate::deps::hreads::on_main;

/// What a dictation heard. Each sentence has its number, a later event of
/// the same sentence replaces the text of an earlier one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictationEvent {
    /// The running guess of the sentence the user says now. Its last words
    /// may still change.
    Live { sentence: usize, text: String },
    /// The user paused, the streaming model is done with the sentence.
    Final { sentence: usize, text: String },
    /// The refiner read the sentence again, with capitals and punctuation.
    Refined { sentence: usize, text: String },
    /// The microphone or a model failed, the dictation ends.
    Failed(String),
    /// The dictation ended, nothing comes after this.
    Stopped,
}

type Handler = Box<dyn FnMut(DictationEvent) + Send>;

/// Hands the events to the handler of the app on the main thread, in the
/// order they were sent.
#[derive(Clone)]
pub(super) struct Sink {
    handler: Arc<Mutex<Handler>>,
}

impl Sink {
    pub(super) fn send(&self, event: DictationEvent) {
        let handler = self.handler.clone();
        on_main(move || (handler.lock())(event));
    }
}

/// What a system needs to listen.
pub(super) struct Job {
    pub(super) models: SpeechModels,
    pub(super) refine: bool,
    pub(super) stop:   Arc<AtomicBool>,
    pub(super) sink:   Sink,
}

/// The microphone heard and turned into text. It runs until `stop` or until
/// it is dropped, the events come on the main thread.
pub struct Dictation {
    stop: Arc<AtomicBool>,
}

impl Dictation {
    /// Starts to listen. It needs `SpeechModel::STREAMING` downloaded, and
    /// uses `SpeechModel::REFINER` when that is downloaded too.
    pub fn start(
        models: &SpeechModels,
        on_event: impl FnMut(DictationEvent) + Send + 'static,
    ) -> Result<Self> {
        system::check()?;
        if !models.has(&SpeechModel::STREAMING) {
            return Err(SpeechError::NoModel(SpeechModel::STREAMING.folder).into());
        }
        let refine = models.has(&SpeechModel::REFINER);
        info!("dictation: starts, refine {refine}");
        let stop = Arc::new(AtomicBool::new(false));
        let sink = Sink {
            handler: Arc::new(Mutex::new(Box::new(on_event))),
        };
        system::run(Job {
            models: models.clone(),
            refine,
            stop: stop.clone(),
            sink,
        })?;
        Ok(Self { stop })
    }

    /// Stops listening. The sentence that was said last still comes as
    /// `Final`, and `Refined` after it, then `Stopped`.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Release);
    }
}

impl Drop for Dictation {
    fn drop(&mut self) {
        self.stop();
    }
}
