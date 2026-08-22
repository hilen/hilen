use std::sync::OnceLock;

use kira::{AudioManager, AudioManagerSettings, track::MainTrackBuilder};
use parking_lot::{Mutex, MutexGuard};

static AUDIO_MANAGER: OnceLock<Mutex<AudioManager>> = OnceLock::new();

pub(crate) fn audio_manager() -> MutexGuard<'static, AudioManager> {
    AUDIO_MANAGER
        .get_or_init(|| {
            Mutex::new(
                AudioManager::new(AudioManagerSettings {
                    main_track_builder: MainTrackBuilder::default().volume(-20.0),
                    ..Default::default()
                })
                .expect("Failed to get audio manager"),
            )
        })
        .lock()
}
