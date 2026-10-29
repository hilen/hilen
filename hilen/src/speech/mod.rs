//! Speech to text from the microphone. The words show while the user talks,
//! and on each pause a bigger model reads the sentence again.
//!
//! ```ignore
//! use hilen::{dispatch::spawn, speech::{Dictation, DictationField, SpeechModel, SpeechModels}};
//!
//! spawn(async move {
//!     let models = SpeechModels::default();
//!     models.download(&SpeechModel::STREAMING, |done, total| {}).await?;
//!     models.download(&SpeechModel::REFINER, |done, total| {}).await?;
//!     anyhow::Ok(())
//! });
//!
//! let mut target = DictationField::new(view.field);
//! let dictation = Dictation::start(&SpeechModels::default(), move |event| target.apply(&event))?;
//! // later
//! dictation.stop();
//! ```
//!
//! The first start asks the user for the microphone. On iOS and on a
//! packaged Mac app the system shows the `[speech] usage` text of
//! `hilen.toml`, which the build writes into the app.

mod dictation;
mod error;
mod field;
mod models;
mod transcript;

#[cfg(not(any(wasm, target_os = "android", target_os = "tvos")))]
mod listen;
#[cfg(not(any(wasm, target_os = "android", target_os = "tvos")))]
use listen as system;

#[cfg(any(wasm, target_os = "android", target_os = "tvos"))]
mod none;
pub use dictation::{Dictation, DictationEvent};
pub use error::SpeechError;
pub use field::DictationField;
pub use models::{ModelFile, SpeechModel, SpeechModels};
#[cfg(any(wasm, target_os = "android", target_os = "tvos"))]
use none as system;
pub use transcript::Transcript;
