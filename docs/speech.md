# Speech

The `speech` feature, off by default, gives `hilen::speech`: speech to text from
the microphone. The words show while the user talks, and on each pause a bigger
model reads the sentence again and replaces it. Found by banda, a mic button by
its compose box, since the dictation of iOS was too poor.

```rust
let models = SpeechModels::default();
spawn(async move {
    models.download(&SpeechModel::STREAMING, |done, total| {}).await?;
    models.download(&SpeechModel::REFINER, |done, total| {}).await?;
    anyhow::Ok(())
});

let mut target = DictationField::new(view.field);
let dictation = Dictation::start(&SpeechModels::default(), move |event| target.apply(&event))?;
dictation.stop();
```

- `SpeechModels::default()` is the folder `speech` in the data folder of the app.
  `download` fetches the files of a model from Hugging Face, pinned to 1 commit,
  each into a `.part` file that gets its name only when it is whole, and checks
  its size. `has` says whether every file is there at its size.
- `SpeechModel::STREAMING` is the English streaming Zipformer of sherpa-onnx,
  190 MB. It writes in capitals with no punctuation, the engine turns its words
  to lower case and a lone `i` to I. `SpeechModel::REFINER` is Parakeet TDT 0.6b
  v2, 660 MB, with capitals and punctuation. Both are English only.
- `Dictation::start` needs the streaming model and uses the refiner when it is
  there. It fails at once with `SpeechError::NoModel`, `NoUsageText` or
  `Unsupported`. A failure later, like a microphone that stopped, comes as
  `DictationEvent::Failed`.
- The events come on the main thread in order: `Live` while a sentence is said,
  `Final` on a pause, `Refined` when the refiner is done with it, `Stopped` at
  the end. Each carries the number of its sentence, a later one replaces the
  text of an earlier one. `stop`, or dropping the `Dictation`, still sends the
  last sentence as `Final` and `Refined`.
- `DictationField` puts the text into a `TextField` at the caret it had at the
  start, with a space in front after a word, through `TextField::replace_range`.
  Keys typed into the field during a dictation move the text under it, the
  dictated part is then replaced at the old place.
- `Transcript` is the text logic alone, for an app that shows the words in its
  own view.

The usage text is in `hilen.toml`, the build writes it where each system wants
it, `NSMicrophoneUsageDescription` on iOS and in a packaged Mac app, and the
`audio-input` entitlement of a signed Mac release:

```toml
[speech]
usage = "Turns what you say into text."
```

## Inside

- `listen.rs` runs on a thread of its own: the default microphone through
  [cpal](https://github.com/RustAudio/cpal), mono f32 at the rate of the device,
  into the `OnlineRecognizer` of
  [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx), which resamples by itself.
  Its endpoint rule ends a sentence on a pause. The streaming recognizer is not
  `Send`, so it is made and dropped on that thread. Each sentence goes to the
  `OfflineRecognizer` on a thread of its own, so the next one is heard meanwhile.
- On iOS the audio session is set to play and record before the microphone opens.
- sherpa-onnx links its prebuilt static libraries, the crate downloads them for
  the target at build time.

## Per platform

- macOS, Windows, Linux, iOS: as above. A bare binary on a Mac asks for the
  microphone in the name of its terminal and needs no usage text.
- Android, a browser and tvOS: `Dictation::start` answers
  `SpeechError::Unsupported`, see the roadmap.

## Proven

Unit tests of the transcript, the capitals, the mono mix, the model sizes and
urls, a file of the wrong size, and the download into a file. The UI test `Text
field replace range` for `TextField::caret` and `replace_range`. Clippy is clean
with `speech` for macOS, iOS and Linux. Not proven yet: a real microphone on any
system.
