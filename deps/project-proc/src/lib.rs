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

/// The `version` of the `hilen.toml` above the crate that calls it, as a
/// string literal, or the version of that crate's `Cargo.toml` when the file
/// has none. The iOS, tvOS and Android projects of an app are stamped with
/// the version of `hilen.toml`, `register_app!` reports it from there.
#[proc_macro]
pub fn app_version(input: TokenStream) -> TokenStream {
    let found = if input.is_empty() {
        var("CARGO_MANIFEST_DIR")
            .context("CARGO_MANIFEST_DIR is not set")
            .and_then(|dir| find_version(Path::new(&dir)))
    } else {
        Err(anyhow!("app_version!() takes no arguments"))
    };

    match found {
        Ok((path, version)) => {
            let path = path.to_string_lossy().into_owned();
            let version = version.map_or_else(
                || quote! { env!("CARGO_PKG_VERSION") },
                |version| quote! { #version },
            );
            quote! {{
                const _: &[u8] = include_bytes!(#path);
                #version
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

/// The url scheme of the `[links]` table of the `hilen.toml` above the crate
/// that calls it, as `Some("shopping")`, or `None` for an app with no such
/// table. The scheme is `scheme` of the table, else the `project_name` in
/// lower case with `-` for `_`. A desktop app reads it to find the link in
/// the arguments of its start, see `system::Links` of the engine.
#[proc_macro]
pub fn link_scheme(input: TokenStream) -> TokenStream {
    let found = if input.is_empty() {
        var("CARGO_MANIFEST_DIR")
            .context("CARGO_MANIFEST_DIR is not set")
            .and_then(|dir| find_link_scheme(Path::new(&dir)))
    } else {
        Err(anyhow!("link_scheme!() takes no arguments"))
    };

    match found {
        Ok((path, scheme)) => {
            let path = path.to_string_lossy().into_owned();
            let scheme = scheme.map_or_else(|| quote! { None }, |scheme| quote! { Some(#scheme) });
            quote! {{
                const _: &[u8] = include_bytes!(#path);
                #scheme
            }}
        }
        Err(err) => {
            let message = format!("{err:#}");
            quote! { compile_error!(#message) }
        }
    }
    .into()
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

#[derive(Deserialize)]
struct VersionFile {
    version: Option<String>,
}

/// The nearest `hilen.toml` and its `version`, none when it has no such
/// key or an empty one.
fn find_version(start: &Path) -> Result<(PathBuf, Option<String>)> {
    let (path, _) = find(start)?;
    let text = read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let file: VersionFile =
        toml::from_str(&text).with_context(|| format!("a wrong version in {}", path.display()))?;
    let version = file.version.filter(|version| !version.trim().is_empty());
    Ok((path, version))
}

#[derive(Deserialize)]
struct LinksFile {
    links: Option<LinksTable>,
}

/// Only the scheme is read here, the domains are for the mobile projects.
#[derive(Deserialize)]
struct LinksTable {
    scheme: Option<String>,
}

/// The nearest `hilen.toml` and the url scheme of its `[links]` table.
fn find_link_scheme(start: &Path) -> Result<(PathBuf, Option<String>)> {
    let (path, name) = find(start)?;
    let text = read_to_string(&path).with_context(|| format!("cannot read {}", path.display()))?;
    let file: LinksFile =
        toml::from_str(&text).with_context(|| format!("a wrong [links] table in {}", path.display()))?;
    let Some(links) = file.links else {
        return Ok((path, None));
    };
    let scheme = links.scheme.unwrap_or_else(|| name.to_lowercase().replace('_', "-"));
    let mut chars = scheme.chars();
    ensure!(
        chars.next().is_some_and(|c| c.is_ascii_lowercase())
            && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '+' | '-' | '.')),
        "the link scheme {scheme:?} in {} must start with a small letter and have only small letters, \
         digits, +, - and .",
        path.display()
    );
    ensure!(
        !matches!(scheme.as_str(), "http" | "https" | "file"),
        "the link scheme {scheme:?} in {} belongs to the system",
        path.display()
    );
    Ok((path, Some(scheme)))
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

    use super::{FILE, find, find_link_scheme, find_version};

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
    fn the_version_comes_from_the_file() -> Result<()> {
        let repo = Repo::new("version")?;
        write(
            repo.0.join(FILE),
            "project_name = \"skaityk\"\nversion = \"3.0.14\"\n",
        )?;

        assert_eq!(
            find_version(&repo.0.join("crates/app"))?.1.as_deref(),
            Some("3.0.14")
        );
        Ok(())
    }

    #[test]
    fn a_file_with_no_version_leaves_the_cargo_version() -> Result<()> {
        let repo = Repo::new("no-version")?;
        write(repo.0.join(FILE), "project_name = \"skaityk\"\nversion = \"\"\n")?;
        assert_eq!(find_version(&repo.0)?.1, None);

        write(repo.0.join(FILE), "project_name = \"skaityk\"\n")?;
        assert_eq!(find_version(&repo.0)?.1, None);
        Ok(())
    }

    #[test]
    fn an_app_with_no_links_table_has_no_scheme() -> Result<()> {
        let repo = Repo::new("no-links")?;
        write(repo.0.join(FILE), "project_name = \"shopping\"\n")?;

        assert_eq!(find_link_scheme(&repo.0)?.1, None);
        Ok(())
    }

    #[test]
    fn the_scheme_is_the_project_name_unless_the_table_names_one() -> Result<()> {
        let repo = Repo::new("links")?;
        write(
            repo.0.join(FILE),
            "project_name = \"My_Shop\"\n[links]\ndomains = [\"shop.example.com\"]\n",
        )?;
        assert_eq!(find_link_scheme(&repo.0)?.1.as_deref(), Some("my-shop"));

        write(
            repo.0.join(FILE),
            "project_name = \"shop\"\n[links]\nscheme = \"shop-app\"\n",
        )?;
        assert_eq!(find_link_scheme(&repo.0)?.1.as_deref(), Some("shop-app"));
        Ok(())
    }

    #[test]
    fn a_wrong_scheme_is_an_error() -> Result<()> {
        let repo = Repo::new("bad-scheme")?;
        for scheme in ["Shop", "1shop", "shop app", "https", "shop/x", ""] {
            write(
                repo.0.join(FILE),
                format!("project_name = \"shop\"\n[links]\nscheme = \"{scheme}\"\n"),
            )?;
            assert!(find_link_scheme(&repo.0).is_err(), "{scheme:?}");
        }
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
