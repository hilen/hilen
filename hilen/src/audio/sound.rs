use std::{
    fmt::{Debug, Formatter},
    io::Cursor,
    path::{Path, PathBuf},
};

use kira::{
    Decibels, Tween,
    sound::{
        FromFileError, PlaybackState,
        static_sound::{StaticSoundData, StaticSoundHandle},
    },
};
use log::error;

use crate::{
    audio::manager::{effects, effects_volume, set_effects_volume},
    deps::refs::manage::ResourceLoader,
    filesystem::read_bytes as read,
};

pub struct Sound {
    path: PathBuf,
    data: StaticSoundData,
}

impl Sound {
    /// The volume of every `Sound` at once, 1 as recorded and 0 silent. It
    /// starts at 0.1, 20 dB down. A game that mixes its own levels sets 1.
    /// A video has its own volume and does not follow this one.
    pub fn set_volume(volume: f32) {
        set_effects_volume(volume);
    }

    pub fn volume() -> f32 {
        effects_volume()
    }

    /// Whether the machine has an output device. Without one every sound is
    /// silent and every `Playing` reports it is not playing. The first call
    /// opens the device.
    pub fn has_output() -> bool {
        effects().is_some()
    }

    pub fn play(&mut self) {
        self.play_with_volume(1.0);
    }

    /// Plays once at `volume`, 1 as recorded and 0 silent, a linear
    /// amplitude like a mixer fader. Nothing can stop it, `play_once` can.
    pub fn play_with_volume(&mut self, volume: f32) {
        self.start(self.data.volume(decibels(volume)));
    }

    /// Plays once at `volume` like `play_with_volume`, and the returned
    /// `Playing` stops it early. Dropping it stops the sound too, so a spoken
    /// line kept in a field is cut off when the next line replaces it.
    pub fn play_once(&mut self, volume: f32) -> Playing {
        Playing {
            handle: self.start(self.data.volume(decibels(volume))),
        }
    }

    /// Plays over and over at `volume` until the returned `Playing` is
    /// stopped or dropped, the way a campfire or a river sounds.
    pub fn play_looped(&mut self, volume: f32) -> Playing {
        Playing {
            handle: self.start(self.data.volume(decibels(volume)).loop_region(..)),
        }
    }

    pub(super) fn at_volume(&self, volume: f32) -> StaticSoundData {
        self.data.volume(decibels(volume))
    }

    #[cfg(test)]
    pub(super) fn from_frames(sample_rate: u32, frames: Vec<kira::Frame>) -> Self {
        Self {
            path: "test".into(),
            data: StaticSoundData {
                sample_rate,
                frames: frames.into(),
                settings: kira::sound::static_sound::StaticSoundSettings::default(),
                slice: None,
            },
        }
    }

    fn start(&self, data: StaticSoundData) -> Option<StaticSoundHandle> {
        match effects()?.play(data) {
            Ok(handle) => Some(handle),
            Err(err) => {
                error!("Failed to play sound {}: {err}", self.path.display());
                None
            }
        }
    }
}

/// A sound started by `Sound::play_once` or `Sound::play_looped`. Dropping
/// it stops the sound.
pub struct Playing {
    /// None when the sound never started, with no output device.
    handle: Option<StaticSoundHandle>,
}

impl Playing {
    /// Changes the volume at once, 1 as recorded and 0 silent. A game
    /// fades a sound with distance this way.
    pub fn set_volume(&mut self, volume: f32) {
        if let Some(handle) = &mut self.handle {
            handle.set_volume(decibels(volume), Tween::default());
        }
    }

    pub fn stop(&mut self) {
        if let Some(handle) = &mut self.handle {
            handle.stop(Tween::default());
        }
    }

    /// False once the sound played to its end or was stopped, and always
    /// false without an output device. A stop fades out over a few
    /// milliseconds, so it reads true right after `stop`.
    pub fn is_playing(&self) -> bool {
        self.handle
            .as_ref()
            .is_some_and(|handle| handle.state() != PlaybackState::Stopped)
    }
}

impl Drop for Playing {
    fn drop(&mut self) {
        self.stop();
    }
}

/// A linear volume as kira's decibels. Kira treats -60 as silence.
pub(crate) fn decibels(volume: f32) -> Decibels {
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

    /// A file kira cannot decode, like quad audio, gets the default sound, the
    /// same as a missing file. A panic here stopped a game the first time such
    /// a sound played.
    fn load_data(data: &[u8], name: impl ToString) -> Self {
        let name = name.to_string();
        let data = decode(data).unwrap_or_else(|err| {
            error!("Failed to decode sound {name}: {err}. Returning default sound");
            decode(DEFAULT_SOUND_DATA).expect("the default sound decodes")
        });

        Self {
            path: name.into(),
            data,
        }
    }
}

fn decode(data: &[u8]) -> Result<StaticSoundData, FromFileError> {
    StaticSoundData::from_media_source(Cursor::new(data.to_vec()))
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

    #[test]
    fn undecodable_data_gets_the_default_sound() {
        let sound = Sound::load_data(b"not a sound", "broken.ogg");
        let default = decode(DEFAULT_SOUND_DATA).expect("the default sound decodes");
        assert_eq!(sound.path, PathBuf::from("broken.ogg"));
        assert_eq!(sound.data.frames.len(), default.frames.len());
    }
}
