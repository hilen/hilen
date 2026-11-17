//! The microphone through cpal into the streaming recognizer of sherpa-onnx,
//! on a thread of its own. The recognizers and the cpal stream are made and
//! dropped on that thread, the streaming recognizer is not `Send`.

use std::{
    mem::take,
    sync::{
        Arc,
        atomic::Ordering,
        mpsc::{RecvTimeoutError, Sender, channel},
    },
    thread,
    time::Duration,
};

use anyhow::{Context, Result, bail};
use cpal::{
    SampleFormat, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use log::{debug, info, warn};
use sherpa_onnx::{
    OfflineRecognizer, OfflineRecognizerConfig, OnlineRecognizer, OnlineRecognizerConfig, OnlineStream,
};

use super::{
    DictationEvent, ModelFile, SpeechError, SpeechModel, SpeechModels,
    dictation::{Job, Sink},
    transcript::streamed,
};
use crate::gm::ToF32;

/// How long a wait for samples lasts before the stop flag is read again.
const LOOK: Duration = Duration::from_millis(100);

pub(super) fn check() -> Result<()> {
    if usage_text_missing() {
        return Err(SpeechError::NoUsageText.into());
    }
    Ok(())
}

#[cfg(target_vendor = "apple")]
fn usage_text_missing() -> bool {
    apple::needs_usage_text() && !apple::has_usage_text()
}

/// Only iOS and a packaged Mac app ask for the microphone with a text.
#[cfg(not(target_vendor = "apple"))]
fn usage_text_missing() -> bool {
    false
}

pub(super) fn run(job: Job) -> Result<()> {
    thread::Builder::new().name("dictation".into()).spawn(move || {
        if let Err(err) = listen(&job) {
            warn!("dictation: {err:#}");
            job.sink.send(DictationEvent::Failed(format!("{err:#}")));
        }
        info!("dictation: stopped");
        job.sink.send(DictationEvent::Stopped);
    })?;
    Ok(())
}

fn listen(job: &Job) -> Result<()> {
    #[cfg(target_os = "ios")]
    apple::record()?;

    let online = OnlineRecognizer::create(&online_config(&job.models))
        .context("the streaming speech model did not load")?;
    let refiner = if job.refine {
        let refiner = OfflineRecognizer::create(&offline_config(&job.models))
            .context("the refining speech model did not load")?;
        Some(Arc::new(refiner))
    } else {
        None
    };

    let (samples, heard_samples) = channel();
    let (microphone, rate) = microphone(samples)?;
    microphone.play().context("the microphone did not start")?;
    info!("dictation: listens at {rate} Hz");

    let speech = online.create_stream();
    let mut sentence = 0;
    let mut audio: Vec<f32> = vec![];
    let mut shown = String::new();
    while !job.stop.load(Ordering::Acquire) {
        let chunk = match heard_samples.recv_timeout(LOOK) {
            Ok(chunk) => chunk,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => bail!("the microphone stopped"),
        };
        speech.accept_waveform(rate, &chunk);
        audio.extend_from_slice(&chunk);
        while online.is_ready(&speech) {
            online.decode(&speech);
        }
        let text = heard(&online, &speech);
        if online.is_endpoint(&speech) {
            if !text.is_empty() {
                finish(
                    sentence,
                    text,
                    take(&mut audio),
                    rate,
                    refiner.as_ref(),
                    &job.sink,
                );
                sentence += 1;
            }
            audio.clear();
            shown.clear();
            online.reset(&speech);
        } else if text != shown {
            job.sink.send(DictationEvent::Live {
                sentence,
                text: text.clone(),
            });
            shown = text;
        }
    }
    drop(microphone);

    while online.is_ready(&speech) {
        online.decode(&speech);
    }
    let text = heard(&online, &speech);
    if !text.is_empty() {
        finish(sentence, text, audio, rate, refiner.as_ref(), &job.sink);
    }
    Ok(())
}

/// Sends the sentence as `Final`, and reads it again with the refiner on a
/// thread of its own, so the next sentence is heard meanwhile.
fn finish(
    sentence: usize,
    text: String,
    audio: Vec<f32>,
    rate: i32,
    refiner: Option<&Arc<OfflineRecognizer>>,
    sink: &Sink,
) {
    debug!("dictation: sentence {sentence} done, {} samples", audio.len());
    sink.send(DictationEvent::Final { sentence, text });
    let Some(refiner) = refiner else {
        return;
    };
    let refiner = refiner.clone();
    let sink = sink.clone();
    let spawned = thread::Builder::new().name("dictation refine".into()).spawn(move || {
        let stream = refiner.create_stream();
        stream.accept_waveform(rate, &audio);
        refiner.decode(&stream);
        let text = stream
            .get_result()
            .map_or_else(String::new, |result| result.text.trim().to_string());
        debug!("dictation: sentence {sentence} refined");
        sink.send(DictationEvent::Refined { sentence, text });
    });
    if let Err(err) = spawned {
        warn!("dictation: sentence {sentence} is not refined, no thread: {err}");
    }
}

/// What the streaming model heard of the sentence so far.
fn heard(online: &OnlineRecognizer, speech: &OnlineStream) -> String {
    online
        .get_result(speech)
        .map_or_else(String::new, |result| streamed(&result.text))
}

fn path(models: &SpeechModels, model: &SpeechModel, file: ModelFile) -> Option<String> {
    let path = models.path(model, file);
    path.exists().then(|| path.to_string_lossy().into_owned())
}

fn online_config(models: &SpeechModels) -> OnlineRecognizerConfig {
    let model = SpeechModel::STREAMING;
    let mut config = OnlineRecognizerConfig::default();
    config.model_config.transducer.encoder = path(models, &model, model.encoder);
    config.model_config.transducer.decoder = path(models, &model, model.decoder);
    config.model_config.transducer.joiner = path(models, &model, model.joiner);
    config.model_config.tokens = path(models, &model, model.tokens);
    config.enable_endpoint = true;
    config
}

fn offline_config(models: &SpeechModels) -> OfflineRecognizerConfig {
    let model = SpeechModel::REFINER;
    let mut config = OfflineRecognizerConfig::default();
    config.model_config.transducer.encoder = path(models, &model, model.encoder);
    config.model_config.transducer.decoder = path(models, &model, model.decoder);
    config.model_config.transducer.joiner = path(models, &model, model.joiner);
    config.model_config.tokens = path(models, &model, model.tokens);
    config.model_config.model_type = Some("nemo_transducer".into());
    config
}

/// The default microphone, its samples as mono f32 into `samples`, and its
/// sample rate. The recognizers take any rate and resample themselves.
fn microphone(samples: Sender<Vec<f32>>) -> Result<(Stream, i32)> {
    let device = cpal::default_host()
        .default_input_device()
        .context("this device has no microphone")?;
    let supported = device.default_input_config().context("the microphone has no input format")?;
    let rate = i32::try_from(supported.sample_rate())?;
    let channels = usize::from(supported.channels());
    let failed = |err| warn!("dictation: the microphone failed: {err}");
    let stream = match supported.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            supported.config(),
            move |data: &[f32], _: &_| send(&samples, mono(data.iter().copied(), channels)),
            failed,
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            supported.config(),
            move |data: &[i16], _: &_| {
                send(
                    &samples,
                    mono(data.iter().map(|sample| f32::from(*sample) / 32_768.0), channels),
                );
            },
            failed,
            None,
        )?,
        other => bail!("the microphone gives {other:?} samples, speech takes f32 and i16"),
    };
    Ok((stream, rate))
}

fn send(samples: &Sender<Vec<f32>>, chunk: Vec<f32>) {
    if let Err(err) = samples.send(chunk) {
        debug!("dictation: a chunk after the stop: {err}");
    }
}

/// The mean of the channels of each frame.
fn mono(samples: impl Iterator<Item = f32>, channels: usize) -> Vec<f32> {
    let channels = channels.max(1);
    let mut mono = vec![];
    let mut sum = 0.0;
    for (index, sample) in samples.enumerate() {
        sum += sample;
        if index % channels == channels - 1 {
            mono.push(sum / channels.to_f32());
            sum = 0.0;
        }
    }
    mono
}

#[cfg(target_vendor = "apple")]
mod apple {
    #[cfg(target_os = "macos")]
    use std::path::Path;

    use objc2_foundation::{NSBundle, NSString};

    const USAGE_KEY: &str = "NSMicrophoneUsageDescription";

    /// iOS stops an app that asks without the text, and so does a packaged
    /// Mac app. A bare binary on a Mac asks in the name of its terminal.
    pub(super) fn needs_usage_text() -> bool {
        #[cfg(target_os = "ios")]
        {
            true
        }
        #[cfg(target_os = "macos")]
        {
            Path::new(&NSBundle::mainBundle().bundlePath().to_string())
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("app"))
        }
    }

    pub(super) fn has_usage_text() -> bool {
        NSBundle::mainBundle()
            .objectForInfoDictionaryKey(&NSString::from_str(USAGE_KEY))
            .is_some()
    }

    /// The audio session of iOS plays only until it is told to record too.
    #[cfg(target_os = "ios")]
    pub(super) fn record() -> anyhow::Result<()> {
        use anyhow::{Context, anyhow};
        use objc2_avf_audio::{AVAudioSession, AVAudioSessionCategoryPlayAndRecord};

        // SAFETY: the shared session and a constant of the framework, which
        // is linked on every iOS.
        let (session, category) = unsafe {
            (
                AVAudioSession::sharedInstance(),
                AVAudioSessionCategoryPlayAndRecord,
            )
        };
        let category = category.context("the audio session has no play and record category")?;
        // SAFETY: a category constant of the framework.
        unsafe { session.setCategory_error(category) }
            .map_err(|err| anyhow!("the audio session did not record: {err}"))?;
        // SAFETY: the shared session of the app.
        unsafe { session.setActive_error(true) }
            .map_err(|err| anyhow!("the audio session did not start: {err}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::mono;

    #[test]
    fn mono_is_the_mean_of_each_frame() {
        assert_eq!(mono([1.0, 0.0, 0.5, 0.5].into_iter(), 2), vec![0.5, 0.5]);
        assert_eq!(mono([0.25, 0.75].into_iter(), 1), vec![0.25, 0.75]);
    }
}
