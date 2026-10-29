//! A system with no speech to text yet.

use anyhow::Result;
use log::warn;

use super::{SpeechError, dictation::Job};

pub(super) fn check() -> Result<()> {
    Err(SpeechError::Unsupported.into())
}

pub(super) fn run(job: Job) -> Result<()> {
    warn!(
        "dictation: this system has no speech to text, {} stays unused",
        job.models.dir().display()
    );
    Err(SpeechError::Unsupported.into())
}
