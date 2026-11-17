use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SpeechError {
    #[error("the speech model {0} is not downloaded")]
    NoModel(&'static str),
    #[error("this system has no speech to text yet")]
    Unsupported,
    #[error("the app has no microphone usage text, add a [speech] usage line to hilen.toml")]
    NoUsageText,
}
