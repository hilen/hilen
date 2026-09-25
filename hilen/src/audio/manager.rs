use std::sync::{
    OnceLock,
    atomic::{AtomicU32, Ordering},
};

use kira::{
    AudioManager, AudioManagerSettings, Tween,
    track::{TrackBuilder, TrackHandle},
};
use parking_lot::{Mutex, MutexGuard};

use crate::audio::sound::decibels;

/// Sound effects start 20 dB down. A video track plays at its own volume
/// next to them.
const DEFAULT_EFFECTS_VOLUME: f32 = 0.1;

static AUDIO_MANAGER: OnceLock<Mutex<AudioManager>> = OnceLock::new();
/// Every `Sound` plays on this track.
static EFFECTS: OnceLock<Mutex<TrackHandle>> = OnceLock::new();
static EFFECTS_VOLUME: AtomicU32 = AtomicU32::new(DEFAULT_EFFECTS_VOLUME.to_bits());

pub(crate) fn audio_manager() -> MutexGuard<'static, AudioManager> {
    AUDIO_MANAGER
        .get_or_init(|| {
            Mutex::new(
                AudioManager::new(AudioManagerSettings::default()).expect("Failed to get audio manager"),
            )
        })
        .lock()
}

pub(crate) fn effects() -> MutexGuard<'static, TrackHandle> {
    EFFECTS
        .get_or_init(|| {
            let track = audio_manager()
                .add_sub_track(TrackBuilder::new().volume(decibels(effects_volume())))
                .expect("Failed to add the sound effects track");
            Mutex::new(track)
        })
        .lock()
}

pub(crate) fn effects_volume() -> f32 {
    f32::from_bits(EFFECTS_VOLUME.load(Ordering::Relaxed))
}

/// Before the first sound plays only the value is stored, the audio device
/// opens on the first play.
pub(crate) fn set_effects_volume(volume: f32) {
    EFFECTS_VOLUME.store(volume.to_bits(), Ordering::Relaxed);
    if let Some(effects) = EFFECTS.get() {
        effects.lock().set_volume(decibels(volume), Tween::default());
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
        assert!(EFFECTS.get().is_none());
        assert!(AUDIO_MANAGER.get().is_none());
    }
}
