use std::{
    fmt::{Debug, Formatter},
    io::Cursor,
    path::{Path, PathBuf},
};

use kira::{
    Decibels, Tween,
    sound::static_sound::{StaticSoundData, StaticSoundHandle},
};
use log::error;

use crate::{
    audio::manager::audio_manager, deps::refs::manage::ResourceLoader, filesystem::read_bytes as read,
};

pub struct Sound {
    path: PathBuf,
    data: StaticSoundData,
}

impl Sound {
    pub fn play(&mut self) {
        self.play_with_volume(1.0);
    }

    /// Plays once at `volume`, 1 as recorded and 0 silent, a linear
    /// amplitude like a mixer fader.
    pub fn play_with_volume(&mut self, volume: f32) {
        audio_manager()
            .play(self.data.volume(decibels(volume)))
            .expect("Failed to play sound");
    }

    /// Plays over and over at `volume` until the returned `Playing` is
    /// stopped or dropped, the way a campfire or a river sounds.
    pub fn play_looped(&mut self, volume: f32) -> Playing {
        let data = self.data.volume(decibels(volume)).loop_region(..);
        let handle = audio_manager().play(data).expect("Failed to play sound");
        Playing { handle }
    }
}

/// A sound that plays until it is stopped, see `Sound::play_looped`.
/// Dropping it stops it.
pub struct Playing {
    handle: StaticSoundHandle,
}

impl Playing {
    /// Changes the volume at once, 1 as recorded and 0 silent. A game
    /// fades a sound with distance this way.
    pub fn set_volume(&mut self, volume: f32) {
        self.handle.set_volume(decibels(volume), Tween::default());
    }

    pub fn stop(&mut self) {
        self.handle.stop(Tween::default());
    }
}

impl Drop for Playing {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A linear volume as kira's decibels. Kira treats -60 as silence.
fn decibels(volume: f32) -> Decibels {
    if volume <= 0.001 {
        return Decibels::SILENCE;
    }
    Decibels((20.0 * volume.log10()).max(Decibels::SILENCE.0))
}
static DEFAULT_SOUND_DATA: &[u8] = include_bytes!("pek.wav");

impl ResourceLoader for Sound {
    fn load_path(path: &Path) -> Self {
        let data = match read(path) {
            Ok(data) => data,
            Err(err) => {
                error!(
                    "Failed to read sound file: {}. Error: {err} Returning default sound",
                    path.display()
                );
                DEFAULT_SOUND_DATA.into()
            }
        };

        Self::load_data(&data, path.display())
    }

    fn load_data(data: &[u8], name: impl ToString) -> Self {
        let buffer = data.to_vec();

        let data = StaticSoundData::from_media_source(Cursor::new(buffer))
            .expect("StaticSoundData::from_media_source(Cursor::new(buffer))");

        Self {
            path: name.to_string().into(),
            data,
        }
    }
}

impl Debug for Sound {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.path.fmt(f)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn volume_maps_to_decibels() {
        assert_eq!(decibels(1.0), Decibels::IDENTITY);
        assert!((decibels(0.5).0 + 6.02).abs() < 0.01);
        assert_eq!(decibels(0.0), Decibels::SILENCE);
        assert_eq!(decibels(0.000_01), Decibels::SILENCE);
        assert!((decibels(0.5).as_amplitude() - 0.5).abs() < 1e-4);
    }
}
