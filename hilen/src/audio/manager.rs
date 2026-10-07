use std::sync::atomic::{AtomicU32, Ordering};

use kira::{
    AudioManager, AudioManagerSettings, Tween,
    track::{TrackBuilder, TrackHandle},
};
use log::error;
use parking_lot::{MappedMutexGuard, Mutex, MutexGuard};

use crate::audio::sound::decibels;

/// Sound effects start 20 dB down. A video track plays at its own volume
/// next to them.
const DEFAULT_EFFECTS_VOLUME: f32 = 0.1;

/// A part of the audio output that opens on its first use.
enum Slot<T> {
    NotOpened,
    Open(T),
    /// A machine with no output device, like a CI runner or a desktop with
    /// nothing plugged in. Every sound is silent then.
    Silent,
}

impl<T> Slot<T> {
    fn open(&mut self) -> Option<&mut T> {
        match self {
            Self::Open(open) => Some(open),
            Self::NotOpened | Self::Silent => None,
        }
    }
}

static AUDIO_MANAGER: Mutex<Slot<AudioManager>> = Mutex::new(Slot::NotOpened);
/// Every `Sound` plays on this track.
static EFFECTS: Mutex<Slot<TrackHandle>> = Mutex::new(Slot::NotOpened);
static EFFECTS_VOLUME: AtomicU32 = AtomicU32::new(DEFAULT_EFFECTS_VOLUME.to_bits());

/// Opens the device on the first call. A missing device used to panic, so
/// the first sound on such a machine took the app down.
pub(crate) fn audio_manager() -> Option<MappedMutexGuard<'static, AudioManager>> {
    let mut manager = AUDIO_MANAGER.lock();
    if matches!(*manager, Slot::NotOpened) {
        *manager = match AudioManager::new(AudioManagerSettings::default()) {
            Ok(manager) => Slot::Open(manager),
            Err(err) => {
                error!("No audio output, every sound is silent: {err}");
                Slot::Silent
            }
        };
    }
    MutexGuard::try_map(manager, Slot::open).ok()
}

pub(crate) fn effects() -> Option<MappedMutexGuard<'static, TrackHandle>> {
    let mut effects = EFFECTS.lock();
    if matches!(*effects, Slot::NotOpened) {
        *effects = open_effects();
    }
    MutexGuard::try_map(effects, Slot::open).ok()
}

fn open_effects() -> Slot<TrackHandle> {
    let Some(mut manager) = audio_manager() else {
        return Slot::Silent;
    };
    match manager.add_sub_track(TrackBuilder::new().volume(decibels(effects_volume()))) {
        Ok(track) => Slot::Open(track),
        Err(err) => {
            error!("No sound effects track, every sound is silent: {err}");
            Slot::Silent
        }
    }
}

/// Closes the output device, which ends the audio threads. Both stay closed,
/// a hot build that is stopped must not open them again, see
/// `docs/hot-reload.md`.
#[cfg(hot)]
pub(crate) fn stop() {
    *EFFECTS.lock() = Slot::Silent;
    *AUDIO_MANAGER.lock() = Slot::Silent;
}

pub(crate) fn effects_volume() -> f32 {
    f32::from_bits(EFFECTS_VOLUME.load(Ordering::Relaxed))
}

/// Before the first sound plays only the value is stored, the audio device
/// opens on the first play.
pub(crate) fn set_effects_volume(volume: f32) {
    EFFECTS_VOLUME.store(volume.to_bits(), Ordering::Relaxed);
    if let Some(effects) = EFFECTS.lock().open() {
        effects.set_volume(decibels(volume), Tween::default());
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn the_effects_volume_is_stored_without_opening_the_device() {
        assert!((effects_volume() - DEFAULT_EFFECTS_VOLUME).abs() < f32::EPSILON);
        assert!((decibels(DEFAULT_EFFECTS_VOLUME).0 + 20.0).abs() < 0.01);
        set_effects_volume(1.0);
        assert!((effects_volume() - 1.0).abs() < f32::EPSILON);
        assert!(matches!(*EFFECTS.lock(), Slot::NotOpened));
        assert!(matches!(*AUDIO_MANAGER.lock(), Slot::NotOpened));
    }
}
