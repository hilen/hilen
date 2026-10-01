//! The data folder of a backend that runs outside Docker,
//! `~/.config/<project_name>`. It is the folder the app of the same project
//! stores in, a twin of `Paths::storage` in `hilen`, since the server crate
//! never links the UI crate.

use std::{
    fs::create_dir_all,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

/// `~/.config/<project_name>` as an `anyhow::Result<PathBuf>`, created when
/// it is not there. The name is the `project_name` of the `hilen.toml` above
/// the backend crate, read at build time, so the app and its backend can
/// never end up in 2 folders. A Docker build has to copy `hilen.toml` into
/// the image build, the macro reads it there too.
#[macro_export]
macro_rules! data_dir {
    () => {
        $crate::data_dir::of_project($crate::project_name!())
    };
}

/// What `data_dir!` calls with the name it read.
pub fn of_project(name: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().context("no home dir")?;
    in_home(&home, name)
}

fn in_home(home: &Path, name: &str) -> Result<PathBuf> {
    let dir = home.join(".config").join(name);
    create_dir_all(&dir).with_context(|| format!("create {}", dir.display()))?;
    Ok(dir)
}

#[cfg(test)]
mod test {
    use std::{env::temp_dir, fs::remove_dir_all, process::id};

    use anyhow::Result;

    use super::in_home;

    #[test]
    fn the_folder_is_config_and_the_name_under_home_and_gets_created() -> Result<()> {
        let home = temp_dir().join(format!("hilen-server-data-dir-{}", id()));

        let dir = in_home(&home, "flixen")?;

        assert_eq!(dir, home.join(".config").join("flixen"));
        assert!(dir.is_dir());
        remove_dir_all(&home)?;
        Ok(())
    }

    /// This crate sits in the hilen repo, whose `hilen.toml` names the
    /// project `demo`. A backend gets the name of its own repo the same way.
    #[test]
    fn the_name_is_read_from_hilen_toml_at_build_time() {
        assert_eq!(crate::project_name!(), "demo");
    }
}
