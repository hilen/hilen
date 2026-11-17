//! The files of the `assets` folder that an app packs into its binary with
//! `embedded_assets!()`. An installed desktop app has no `assets` folder
//! next to it, and its updater swaps only the executable.

use std::{
    fs::read_dir,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

const ASSETS: &str = "assets";

/// The folders of `assets` the engine loads from at run time. What else
/// sits there, like the app icon or a key file, is not packed.
const FOLDERS: [&str; 4] = ["fonts", "images", "models", "sounds"];

/// One file to pack: its name under `assets`, with `/` between the parts on
/// every system, and its full path.
pub(crate) type Packed = (String, PathBuf);

/// The nearest `assets` folder at `start` or above it. The engine finds the
/// folder of a dev run the same way.
pub(crate) fn find(start: &Path) -> Result<PathBuf> {
    start
        .ancestors()
        .map(|dir| dir.join(ASSETS))
        .find(|path| path.is_dir())
        .with_context(|| {
            format!(
                "no {ASSETS} folder in {} or in any folder above it, there is nothing to embed",
                start.display()
            )
        })
}

/// Every file of the loaded folders, sorted by name. A file or folder whose
/// name starts with a dot is left out, like `.gitkeep` and `.DS_Store`.
pub(crate) fn collect(assets: &Path) -> Result<Vec<Packed>> {
    let mut packed = Vec::new();
    for folder in FOLDERS {
        let dir = assets.join(folder);
        if dir.is_dir() {
            walk(&dir, folder, &mut packed)?;
        }
    }
    packed.sort();
    Ok(packed)
}

fn walk(dir: &Path, name: &str, packed: &mut Vec<Packed>) -> Result<()> {
    let entries = read_dir(dir).with_context(|| format!("cannot read {}", dir.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("cannot read {}", dir.display()))?;
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        if file_name.starts_with('.') {
            continue;
        }
        let path = entry.path();
        let name = format!("{name}/{file_name}");
        if path.is_dir() {
            walk(&path, &name, packed)?;
        } else {
            packed.push((name, path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, remove_dir_all, write},
        path::PathBuf,
        process::id,
    };

    use anyhow::Result;

    use super::{collect, find};

    /// A fresh folder per test, removed when the test ends.
    struct Repo(PathBuf);

    impl Repo {
        fn new(test: &str) -> Result<Self> {
            let root = temp_dir().join(format!("project-proc-assets-{test}-{}", id()));
            create_dir_all(root.join("crates/app/src"))?;
            Ok(Self(root))
        }

        fn file(&self, path: &str) -> Result<()> {
            let path = self.0.join(path);
            if let Some(dir) = path.parent() {
                create_dir_all(dir)?;
            }
            write(path, "x")?;
            Ok(())
        }
    }

    impl Drop for Repo {
        fn drop(&mut self) {
            if let Err(err) = remove_dir_all(&self.0) {
                eprintln!("{} not removed: {err}", self.0.display());
            }
        }
    }

    #[test]
    fn the_assets_folder_above_the_crate_is_found() -> Result<()> {
        let repo = Repo::new("above")?;
        repo.file("assets/images/play.svg")?;

        assert_eq!(find(&repo.0.join("crates/app"))?, repo.0.join("assets"));
        Ok(())
    }

    #[test]
    fn a_repo_with_no_assets_folder_is_an_error_that_names_the_start() -> Result<()> {
        let repo = Repo::new("none")?;
        let start = repo.0.join("crates/app");

        let Err(err) = find(&start) else {
            panic!("the temp folder has no assets folder above it");
        };

        assert!(format!("{err:#}").contains(&start.display().to_string()));
        Ok(())
    }

    #[test]
    fn the_files_of_the_loaded_folders_are_packed_with_slash_names() -> Result<()> {
        let repo = Repo::new("packed")?;
        repo.file("assets/images/play.svg")?;
        repo.file("assets/images/prince/01a.png")?;
        repo.file("assets/fonts/Inter.ttf")?;
        repo.file("assets/sounds/tap.wav")?;
        repo.file("assets/models/cube.glb")?;

        let names: Vec<String> = collect(&repo.0.join("assets"))?.into_iter().map(|(name, _)| name).collect();

        assert_eq!(
            names,
            [
                "fonts/Inter.ttf",
                "images/play.svg",
                "images/prince/01a.png",
                "models/cube.glb",
                "sounds/tap.wav"
            ]
        );
        Ok(())
    }

    /// The app icon, the update key and the iOS icon set sit in `assets` too.
    /// The engine never loads them by name, so they stay out of the binary.
    #[test]
    fn files_outside_the_loaded_folders_and_dot_files_are_left_out() -> Result<()> {
        let repo = Repo::new("left-out")?;
        repo.file("assets/icon.icns")?;
        repo.file("assets/update-key.pub")?;
        repo.file("assets/AppIcon.appiconset/16.png")?;
        repo.file("assets/images/.gitkeep")?;
        repo.file("assets/images/.cache/old.png")?;
        repo.file("assets/images/play.svg")?;

        let packed = collect(&repo.0.join("assets"))?;

        assert_eq!(packed.len(), 1);
        assert_eq!(packed[0].0, "images/play.svg");
        assert_eq!(packed[0].1, repo.0.join("assets/images/play.svg"));
        Ok(())
    }

    #[test]
    fn an_assets_folder_with_none_of_the_loaded_folders_packs_nothing() -> Result<()> {
        let repo = Repo::new("empty")?;
        repo.file("assets/icon.png")?;

        assert_eq!(collect(&repo.0.join("assets"))?.len(), 0);
        Ok(())
    }
}
