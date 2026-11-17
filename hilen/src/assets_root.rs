//! Where the `assets` folder of a desktop run is. Only the file system is
//! asked. An installed app sits on a machine with no repo and no dev tools,
//! so nothing here may start a program or fail.

use std::path::{Path, PathBuf};

/// The nearest folder above `cwd` that holds `assets` wins, so an app nested
/// in a bigger repo gets its own. The exe is the second start, a dev build
/// sits in `<repo>/target/<profile>/` and an IDE may launch it from any
/// folder. `None` is the normal case of an installed app that embeds its
/// assets.
pub(crate) fn find(cwd: Option<&Path>, exe: Option<&Path>) -> Option<PathBuf> {
    cwd.and_then(nearest_with_assets)
        .or_else(|| exe.and_then(Path::parent).and_then(nearest_with_assets))
}

fn nearest_with_assets(start: &Path) -> Option<PathBuf> {
    start.ancestors().find(|dir| dir.join("assets").is_dir()).map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use std::{
        env::temp_dir,
        fs::{create_dir_all, remove_dir_all},
        path::PathBuf,
        process::id,
    };

    use anyhow::Result;

    use super::find;

    fn fixture(name: &str) -> Result<PathBuf> {
        let dir = temp_dir().join(format!("hilen-assets-root-{name}-{}", id()));
        create_dir_all(&dir)?;
        Ok(dir)
    }

    #[test]
    fn nearest_folder_above_cwd_wins() -> Result<()> {
        let repo = fixture("nearest")?;
        let app = repo.join("apps/game");
        let cwd = app.join("src/ui");
        create_dir_all(repo.join("assets"))?;
        create_dir_all(app.join("assets"))?;
        create_dir_all(&cwd)?;

        let found = find(Some(&cwd), None);
        remove_dir_all(&repo)?;

        assert_eq!(found, Some(app));
        Ok(())
    }

    #[test]
    fn exe_is_used_when_cwd_has_nothing() -> Result<()> {
        let repo = fixture("exe")?;
        let elsewhere = fixture("exe-cwd")?;
        let exe = repo.join("target/release/game");
        create_dir_all(repo.join("assets"))?;
        create_dir_all(repo.join("target/release"))?;

        let found = find(Some(&elsewhere), Some(&exe));
        remove_dir_all(&repo)?;
        remove_dir_all(&elsewhere)?;

        assert_eq!(found, Some(repo));
        Ok(())
    }

    #[test]
    fn installed_app_finds_nothing() -> Result<()> {
        let install = fixture("installed")?;
        let exe = install.join("game.exe");

        let found = find(Some(&install), Some(&exe));
        remove_dir_all(&install)?;

        assert_eq!(found, None);
        Ok(())
    }
}
