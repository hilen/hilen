use std::{
    fmt::{Debug, Formatter},
    io::Cursor,
    path::{Path, PathBuf},
};

use kira::sound::static_sound::StaticSoundData;
use log::error;
use refs::manage::ResourceLoader;

use crate::{audio::manager::audio_manager, filesystem::read_bytes as read};

pub struct Sound {
    path: PathBuf,
    data: StaticSoundData,
}

impl Sound {
    pub fn play(&mut self) {
        audio_manager().play(self.data.clone()).expect("Failed to play sound");
    }
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
