//! `project_name!()`, the `project_name` of the `hilen.toml` above the
//! crate that calls it, as a string literal.
//!
//! An installed app has no `hilen.toml`, so the name goes into the binary
//! at build time. The engine is built as a dependency and cannot tell
//! where the app repo is. The crate that calls the macro always sits
//! inside that repo, so the search starts at its own folder and walks up.

mod assets;

use std::{
    env::var,
    fs::read_to_string,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, anyhow, ensure};
use proc_macro::TokenStream;
use quote::quote;
use serde::Deserialize;

const FILE: &str = "hilen.toml";

#[derive(Deserialize)]
struct Project {
    project_name: String,
}

#[proc_macro]
pub fn project_name(input: TokenStream) -> TokenStream {
    let found = if input.is_empty() {
        var("CARGO_MANIFEST_DIR")
            .context("CARGO_MANIFEST_DIR is not set")
            .and_then(|dir| find(Path::new(&dir)))
    } else {
        Err(anyhow!("project_name!() takes no arguments"))
    };

    match found {
        Ok((path, name)) => {
            let path = path.to_string_lossy().into_owned();
            // The include makes cargo build the crate again when the file
            // changes, a proc macro alone leaves the old name in place.
            quote! {{
                const _: &[u8] = include_bytes!(#path);
                #name
            }}
        }
        Err(err) => {
            let message = format!("{err:#}");
            quote! { compile_error!(#message) }
        }
    }
    .into()
}

/// The moment the crate that calls it is compiled, in unix seconds, as a
/// `u64` number. `register_app!` calls it in the final crate of an app.
/// Cargo compiles that crate again after any change below it, so the number
/// is the time the code of the app was last built. A build script can not
/// give this, an app has none that the engine could write.
#[proc_macro]
pub fn build_time(input: TokenStream) -> TokenStream {
    if !input.is_empty() {
        return quote! { compile_error!("build_time!() takes no arguments") }.into();
    }
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_secs());
    quote! { #seconds }.into()
}

/// The files of the app's `assets` folder as a list of name and bytes, for
/// `hilen::embed_assets!`. The folder is the nearest one at or above the
/// crate that calls it.
#[proc_macro]
pub fn embedded_assets(input: TokenStream) -> TokenStream {
    let packed = if input.is_empty() {
        var("CARGO_MANIFEST_DIR")
            .context("CARGO_MANIFEST_DIR is not set")
            .and_then(|dir| assets::find(Path::new(&dir)))
            .and_then(|dir| assets::collect(&dir))
    } else {
        Err(anyhow!("embedded_assets!() takes no arguments"))
    };

    match packed {
        Ok(packed) => {
            let names = packed.iter().map(|(name, _)| name);
            // The include also makes cargo build the crate again when a
            // packed file changes. A file added later is seen only by a
            // build that compiles the crate anyway, a release build does.
            let paths = packed.iter().map(|(_, path)| path.to_string_lossy().into_owned());
            quote! {
                &[#((#names, include_bytes!(#paths) as &[u8])),*]
            }
        }
        Err(err) => {
            let message = format!("{err:#}");
            quote! { compile_error!(#message) }
        }
    }
    .into()
}

/// The nearest `hilen.toml` at `start` or above it, and its `project_name`.
fn find(start: &Path) -> Result<(PathBuf, String)> {
    let path = start
        .ancestors()
        .map(|dir| dir.join(FILE))
        .find(|path| path.is_file())
        .with_context(|| {
            format!(
                "no {FILE} in {} or in any folder above it, the name of the app is its project_name",
                start.display()
            )
        })?;

    let text = read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let project: Project =
        toml::from_str(&text).with_context(|| format!("no project_name in {}", path.display()))?;
    let name = project.project_name;

    // It becomes a folder name, `~/.config/<project_name>`.
    ensure!(
        !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "project_name {name:?} in {} must be letters, digits, - or _",
        path.display()
    );

    Ok((path, name))
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

    use super::{FILE, find};

    /// A fresh folder per test, removed when the test ends.
    struct Repo(PathBuf);

    impl Repo {
        fn new(test: &str) -> Result<Self> {
            let root = temp_dir().join(format!("project-proc-{test}-{}", id()));
            create_dir_all(root.join("crates/app/src"))?;
            Ok(Self(root))
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
    fn the_name_comes_from_the_file_above_the_crate() -> Result<()> {
        let repo = Repo::new("above")?;
        write(
            repo.0.join(FILE),
            "bundle_id = \"x.y\"\nproject_name = \"flixen\"\n\n[release]\ntarget_subdir = \"a\"\n",
        )?;

        let (path, name) = find(&repo.0.join("crates/app"))?;

        assert_eq!(name, "flixen");
        assert_eq!(path, repo.0.join(FILE));
        Ok(())
    }

    #[test]
    fn the_nearest_file_wins() -> Result<()> {
        let repo = Repo::new("nearest")?;
        write(repo.0.join(FILE), "project_name = \"outer\"\n")?;
        write(repo.0.join("crates/app").join(FILE), "project_name = \"inner\"\n")?;

        assert_eq!(find(&repo.0.join("crates/app/src"))?.1, "inner");
        Ok(())
    }

    #[test]
    fn a_file_without_the_name_is_an_error() -> Result<()> {
        let repo = Repo::new("missing")?;
        write(repo.0.join(FILE), "bundle_id = \"x.y\"\n")?;

        let err = find(&repo.0).expect_err("no project_name in the file");

        assert!(format!("{err:#}").contains("no project_name"), "{err:#}");
        Ok(())
    }

    #[test]
    fn a_name_that_is_not_a_folder_name_is_an_error() -> Result<()> {
        let repo = Repo::new("escape")?;
        write(repo.0.join(FILE), "project_name = \"../other\"\n")?;

        let err = find(&repo.0).expect_err("the name leaves the config folder");

        assert!(format!("{err:#}").contains("must be letters"), "{err:#}");
        Ok(())
    }
}
