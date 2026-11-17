//! Self update for desktop apps, on top of the `hilen-updater` crate. The
//! app opts in by returning an `UpdateSource` from `App::update_source`.
//! `check` reads that source on main and fetches the manifest, `install`
//! downloads, verifies and swaps the executable, and `relaunch` starts the
//! new binary and stops the app. Mobile updates go through the stores and a
//! wasm app updates by rehosting, so outside the desktop every call is a
//! no-op, matching `system::Router`.

use anyhow::Result;
pub use hilen_updater::{DEFAULT_UPDATE_HOST, UpdateArtifact, UpdateInfo, UpdateManifest, UpdateSource};

/// The update source of an app that gives its key in `App::update_key`,
/// `None` for an app with no key and outside the desktop.
pub(crate) fn app_update_source(key: Option<&str>, url: Option<String>) -> Option<UpdateSource> {
    #[cfg(desktop)]
    {
        use crate::app::{hilen_app_version, hilen_project_name};

        let name = hilen_project_name();
        let env = std::env::var(update_url_env(name)).ok();
        Some(source_for(name, hilen_app_version(), key?, url, env))
    }
    #[cfg(not(desktop))]
    {
        drop((key, url));
        None
    }
}

/// The env var that points the updater of app `name` at another manifest,
/// like a local file server for a test without a release.
#[cfg(desktop)]
fn update_url_env(name: &str) -> String {
    format!("{}_UPDATE_URL", name.to_uppercase().replace('-', "_"))
}

/// The env var wins over the address of the app, and that one over the
/// default server.
#[cfg(desktop)]
fn source_for(
    name: &str,
    version: &str,
    key: &str,
    url: Option<String>,
    env: Option<String>,
) -> UpdateSource {
    let manifest_url = env
        .filter(|url| !url.trim().is_empty())
        .or(url)
        .unwrap_or_else(|| format!("{DEFAULT_UPDATE_HOST}/{name}/updater.json"));
    UpdateSource::new(manifest_url, version, key)
}

pub struct Updater;

impl Updater {
    /// `Ok(None)` means up to date, or no update source, or a platform
    /// with no self update.
    pub async fn check() -> Result<Option<UpdateInfo>> {
        #[cfg(desktop)]
        {
            use crate::deps::hreads::from_main;

            let Some(source) = from_main(|| crate::app::app().update_source()).await? else {
                return Ok(None);
            };

            hilen_updater::check(&source).await
        }
        // The API is async on every platform, here the answer is immediate.
        #[cfg(not(desktop))]
        {
            log::trace!("Updater::check outside the desktop is a no-op");
            std::future::ready(Ok(None)).await
        }
    }

    /// Downloads, verifies and swaps the executable. The new binary runs
    /// on the next start, call `relaunch` to switch now.
    pub async fn install(info: UpdateInfo) -> Result<()> {
        Self::install_with_progress(info, |_, _| {}).await
    }

    /// `install` with the download reported as bytes so far and the
    /// total, `None` while the server sent no Content-Length.
    pub async fn install_with_progress(
        info: UpdateInfo,
        on_progress: impl FnMut(u64, Option<u64>) + Send,
    ) -> Result<()> {
        #[cfg(desktop)]
        {
            hilen_updater::install_with_progress(info, on_progress).await
        }
        // The API is async on every platform, here the answer is immediate.
        #[cfg(not(desktop))]
        {
            drop(on_progress);
            std::future::ready(Err(anyhow::anyhow!(
                "Self update is desktop only, cannot install version {}",
                info.version
            )))
            .await
        }
    }

    /// Spawns the freshly installed binary and closes this one.
    pub fn relaunch() -> Result<()> {
        #[cfg(desktop)]
        {
            use crate::{AppRunner, deps::hreads::on_main};

            hilen_updater::start_new_binary()?;

            on_main(AppRunner::stop);

            Ok(())
        }
        #[cfg(not(desktop))]
        {
            anyhow::bail!("Self update is desktop only, cannot relaunch")
        }
    }
}

#[cfg(all(test, desktop))]
mod tests {
    use super::{source_for, update_url_env};

    #[test]
    fn the_source_of_an_app_points_at_the_default_server_under_its_name() {
        let source = source_for("flixen", "0.1.0", "abc\n", None, None);

        assert_eq!(source.manifest_url, "https://get.vladas.xyz/flixen/updater.json");
        assert_eq!(source.current_version, "0.1.0");
        assert_eq!(source.verify_key, "abc");
    }

    #[test]
    fn the_address_of_the_app_wins_over_the_default_and_the_env_var_over_both() {
        let own = Some("https://example.com/u.json".to_string());
        let env = Some("http://localhost:8000/updater.json".to_string());

        assert_eq!(
            source_for("flixen", "0.1.0", "abc", own.clone(), None).manifest_url,
            "https://example.com/u.json"
        );
        assert_eq!(
            source_for("flixen", "0.1.0", "abc", own.clone(), env).manifest_url,
            "http://localhost:8000/updater.json"
        );
        // An env var that is set and empty changes nothing.
        assert_eq!(
            source_for("flixen", "0.1.0", "abc", own, Some(" ".to_string())).manifest_url,
            "https://example.com/u.json"
        );
    }

    #[test]
    fn the_env_var_is_named_after_the_app() {
        assert_eq!(update_url_env("flixen"), "FLIXEN_UPDATE_URL");
        assert_eq!(update_url_env("my-app"), "MY_APP_UPDATE_URL");
    }
}
