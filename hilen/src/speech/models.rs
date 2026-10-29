//! The model files of speech, downloaded once into the data folder of the
//! app. They are hundreds of MB, too big to build into a binary.

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use log::info;

use crate::filesystem::Paths;

/// 1 file of a model, with the size it has at the pinned commit, so a file
/// that is cut short is not taken for a whole one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelFile {
    pub name: &'static str,
    pub size: u64,
}

/// A transducer model of sherpa-onnx on Hugging Face, pinned to 1 commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeechModel {
    /// The folder of the model inside the speech folder.
    pub folder:  &'static str,
    repo:        &'static str,
    commit:      &'static str,
    pub encoder: ModelFile,
    pub decoder: ModelFile,
    pub joiner:  ModelFile,
    pub tokens:  ModelFile,
}

impl SpeechModel {
    /// English, streaming, the words show while the user talks, 190 MB. It
    /// writes in capitals with no punctuation.
    pub const STREAMING: Self = Self {
        folder:  "zipformer-en-2023-06-21",
        repo:    "csukuangfj/sherpa-onnx-streaming-zipformer-en-2023-06-21",
        commit:  "9a65b6ea94c311ca770c2bf895b30f456a22d703",
        encoder: ModelFile {
            name: "encoder-epoch-99-avg-1.int8.onnx",
            size: 187_823_992,
        },
        decoder: ModelFile {
            name: "decoder-epoch-99-avg-1.onnx",
            size: 2_092_566,
        },
        joiner:  ModelFile {
            name: "joiner-epoch-99-avg-1.int8.onnx",
            size: 259_335,
        },
        tokens:  ModelFile {
            name: "tokens.txt",
            size: 5_048,
        },
    };

    /// English, Parakeet TDT 0.6b v2 of NVIDIA, reads a whole sentence again
    /// after a pause, with capitals and punctuation, 660 MB.
    pub const REFINER: Self = Self {
        folder:  "parakeet-tdt-0.6b-v2",
        repo:    "csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8",
        commit:  "1ab9323565ddb038682214b292f588070a538ce2",
        encoder: ModelFile {
            name: "encoder.int8.onnx",
            size: 652_184_296,
        },
        decoder: ModelFile {
            name: "decoder.int8.onnx",
            size: 7_257_753,
        },
        joiner:  ModelFile {
            name: "joiner.int8.onnx",
            size: 1_739_080,
        },
        tokens:  ModelFile {
            name: "tokens.txt",
            size: 9_384,
        },
    };

    pub fn files(&self) -> [ModelFile; 4] {
        [self.encoder, self.decoder, self.joiner, self.tokens]
    }

    /// The bytes of all its files.
    pub fn size(&self) -> u64 {
        self.files().iter().map(|file| file.size).sum()
    }

    fn url(&self, file: ModelFile) -> String {
        format!(
            "https://huggingface.co/{}/resolve/{}/{}",
            self.repo, self.commit, file.name
        )
    }
}

/// The folder that holds the models, `speech` in the data folder of the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeechModels {
    dir: PathBuf,
}

impl Default for SpeechModels {
    fn default() -> Self {
        Self::at(Paths::storage().join("speech"))
    }
}

impl SpeechModels {
    pub fn at(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path(&self, model: &SpeechModel, file: ModelFile) -> PathBuf {
        self.dir.join(model.folder).join(file.name)
    }

    /// Every file of the model is there at its full size.
    pub fn has(&self, model: &SpeechModel) -> bool {
        model
            .files()
            .into_iter()
            .all(|file| self.path(model, file).metadata().is_ok_and(|meta| meta.len() == file.size))
    }

    /// Downloads the files of the model that are missing or cut short, and
    /// reports the bytes so far of the whole model and its size.
    #[cfg(not_wasm)]
    pub async fn download(&self, model: &SpeechModel, mut on_progress: impl FnMut(u64, u64)) -> Result<()> {
        use crate::deps::netrun::rest::download_to_file;

        let total = model.size();
        let mut before = 0;
        tokio::fs::create_dir_all(self.dir.join(model.folder)).await?;
        for file in model.files() {
            let path = self.path(model, file);
            if !path.metadata().is_ok_and(|meta| meta.len() == file.size) {
                let url = model.url(file);
                info!("speech: downloading {url}");
                download_to_file(&url, &path, |done, _| on_progress(before + done, total)).await?;
                let size = path.metadata()?.len();
                if size != file.size {
                    bail!("{url} has {size} bytes, the pinned file has {}", file.size);
                }
            }
            before += file.size;
            on_progress(before, total);
        }
        info!("speech: {} is ready in {}", model.folder, self.dir.display());
        Ok(())
    }

    #[cfg(wasm)]
    pub async fn download(&self, model: &SpeechModel, _on_progress: impl FnMut(u64, u64)) -> Result<()> {
        bail!(
            "a browser has no speech models yet, {} is not downloaded",
            model.folder
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, remove_dir_all, write},
        process::id,
    };

    use super::{SpeechModel, SpeechModels};

    #[test]
    fn a_file_of_the_wrong_size_is_missing() {
        let dir = temp_dir().join(format!("hilen-speech-{}", id()));
        let models = SpeechModels::at(&dir);
        let model = SpeechModel::STREAMING;
        assert!(!models.has(&model));

        create_dir_all(dir.join(model.folder)).expect("make the folder");
        for file in model.files() {
            write(models.path(&model, file), b"cut").expect("write a file");
        }
        assert!(!models.has(&model));
        remove_dir_all(&dir).expect("remove the folder");
    }

    #[test]
    fn a_url_names_the_pinned_commit() {
        let model = SpeechModel::REFINER;
        assert_eq!(
            model.url(model.tokens),
            "https://huggingface.co/csukuangfj/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/resolve/1ab9323565ddb038682214b292f588070a538ce2/tokens.txt"
        );
        assert_eq!(model.size(), 661_190_513);
    }
}
